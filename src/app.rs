use crate::model::{
    build_export_image, draw_markup, image_origin, load_image, overlay_bounds, transform_case,
    Preferences, Snapshot, TextOverlay, Tool,
};
use arboard::{Clipboard, ImageData};
use eframe::egui::{
    self, Color32, CursorIcon, Key, Modifiers, PointerButton, Pos2, Rect, Sense, Vec2,
};
use image::{DynamicImage, GenericImageView, RgbaImage};
use std::{borrow::Cow, path::PathBuf, process::Command, time::SystemTime};

const MAX_UNDO_STATES: usize = 20;
const CASE_OPTIONS: [&str; 6] = [
    "No Change",
    "Sentence case",
    "lowercase",
    "UPPERCASE",
    "Capitalize Each Word",
    "tOGGLE cASE",
];

#[derive(Clone)]
struct DragRectangle {
    tool: Tool,
    start: Pos2,
    current: Pos2,
}

#[derive(Clone)]
struct TextDialog {
    overlay_id: Option<u64>,
    x: f32,
    y: f32,
    text: String,
}

struct OcrDialog {
    text: String,
    case_action: String,
    selection: (usize, usize),
    selected_count: usize,
}

struct ColorDialog {
    x: i32,
    y: i32,
    color: [u8; 3],
}

struct MessageDialog {
    title: String,
    text: String,
}

enum Dialog {
    Text(TextDialog),
    Ocr(OcrDialog),
    Color(ColorDialog),
    Message(MessageDialog),
    ShadowSettings,
}

pub struct MarkupApp {
    image: Option<DynamicImage>,
    text_overlays: Vec<TextOverlay>,
    next_text_id: u64,
    undo_stack: Vec<Snapshot>,
    redo_stack: Vec<Snapshot>,
    tool: Tool,
    prefs: Preferences,
    texture: Option<egui::TextureHandle>,
    texture_dirty: bool,
    dialog: Option<Dialog>,
    drawing: Option<DragRectangle>,
    dragging_text: Option<u64>,
    drag_text_offset: Pos2,
    first_image_size: bool,
}

impl MarkupApp {
    pub fn new(cc: &eframe::CreationContext<'_>, image_path: Option<PathBuf>) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Light);
        let mut app = Self {
            image: None,
            text_overlays: Vec::new(),
            next_text_id: 1,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            tool: Tool::Highlighter,
            prefs: Preferences::load(),
            texture: None,
            texture_dirty: true,
            dialog: None,
            drawing: None,
            dragging_text: None,
            drag_text_offset: Pos2::ZERO,
            first_image_size: true,
        };
        if let Some(path) = image_path {
            app.open_path(path);
        }
        app
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        if !self.texture_dirty {
            return;
        }
        self.texture_dirty = false;
        let Some(image) = self.image.as_ref() else {
            self.texture = None;
            return;
        };
        let rendered = build_export_image(image, &self.text_overlays, &self.prefs).to_rgba8();
        let size = [rendered.width() as usize, rendered.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rendered.as_raw());
        self.texture =
            Some(ctx.load_texture("markup-canvas", color_image, egui::TextureOptions::LINEAR));
    }

    fn fit_window_to_image(&mut self, ctx: &egui::Context) {
        if !self.first_image_size {
            return;
        }
        let Some(image) = self.image.as_ref() else {
            return;
        };
        self.first_image_size = false;
        let shadow_width = if self.prefs.show_shadow {
            2 + self.prefs.shadow_offset_x.unsigned_abs() + self.prefs.shadow_padding * 2
        } else {
            0
        } as f32;
        let shadow_height = if self.prefs.show_shadow {
            2 + self.prefs.shadow_offset_y.unsigned_abs() + self.prefs.shadow_padding * 2
        } else {
            0
        } as f32;
        let desired = Vec2::new(
            (image.width() as f32 + shadow_width + 42.0).clamp(700.0, 1500.0),
            (image.height() as f32 + shadow_height + 116.0).clamp(500.0, 1000.0),
        );
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(desired));
    }

    fn snapshot(&self) -> Option<Snapshot> {
        Some(Snapshot {
            image: self.image.as_ref()?.clone(),
            text_overlays: self.text_overlays.clone(),
            next_text_id: self.next_text_id,
        })
    }

    fn save_state(&mut self) {
        let Some(snapshot) = self.snapshot() else {
            return;
        };
        self.redo_stack.clear();
        self.undo_stack.push(snapshot);
        if self.undo_stack.len() > MAX_UNDO_STATES {
            self.undo_stack.remove(0);
        }
    }

    fn undo(&mut self) {
        if let (Some(current), Some(previous)) = (self.snapshot(), self.undo_stack.pop()) {
            self.redo_stack.push(current);
            self.image = Some(previous.image);
            self.text_overlays = previous.text_overlays;
            self.next_text_id = previous.next_text_id;
            self.texture_dirty = true;
        }
    }

    fn redo(&mut self) {
        if let (Some(current), Some(next)) = (self.snapshot(), self.redo_stack.pop()) {
            self.undo_stack.push(current);
            self.image = Some(next.image);
            self.text_overlays = next.text_overlays;
            self.next_text_id = next.next_text_id;
            self.texture_dirty = true;
        }
    }

    fn replace_image(&mut self, image: DynamicImage) {
        self.image = Some(image.to_rgba8().into());
        self.text_overlays.clear();
        self.next_text_id = 1;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.drawing = None;
        self.dragging_text = None;
        self.texture_dirty = true;
        self.first_image_size = true;
    }

    fn open_path(&mut self, path: PathBuf) {
        match load_image(&path) {
            Ok(image) => self.replace_image(image),
            Err(error) => self.show_message("Could not open image", error.to_string()),
        }
    }

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Image files",
                &["png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff", "webp"],
            )
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.open_path(path);
        }
    }

    fn save_file_dialog(&mut self) {
        let Some(image) = self.image.as_ref() else {
            self.show_message("Nothing to save", "Load an image before saving.");
            return;
        };
        let mut path = rfd::FileDialog::new()
            .add_filter("PNG image", &["png"])
            .add_filter("JPEG image", &["jpg", "jpeg"])
            .set_file_name(if self.prefs.last_output_format == "jpg" {
                "markup.jpg"
            } else {
                "markup.png"
            })
            .save_file();
        let Some(mut path) = path.take() else { return };
        let mut format = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        if format.is_empty() {
            format = self.prefs.last_output_format.clone();
            path.set_extension(&format);
        }
        let selected_format = crate::model::normalize_format(&format);
        let output = build_export_image(image, &self.text_overlays, &self.prefs);
        let result = if selected_format == "jpg" {
            output
                .to_rgb8()
                .save_with_format(&path, image::ImageFormat::Jpeg)
        } else {
            output.save_with_format(&path, image::ImageFormat::Png)
        };
        match result {
            Ok(()) => {
                self.prefs.last_output_format = selected_format.to_owned();
                self.prefs.save();
            }
            Err(error) => self.show_message("Could not save image", error.to_string()),
        }
    }

    fn copy_image(&mut self) {
        let Some(image) = self.image.as_ref() else {
            self.show_message("Nothing to copy", "Load an image before copying it.");
            return;
        };
        let image = build_export_image(image, &self.text_overlays, &self.prefs).to_rgba8();
        let data = ImageData {
            width: image.width() as usize,
            height: image.height() as usize,
            bytes: Cow::Owned(image.into_raw()),
        };
        match Clipboard::new().and_then(|mut clipboard| clipboard.set_image(data)) {
            Ok(()) => {}
            Err(error) => self.show_message("Clipboard error", error.to_string()),
        }
    }

    fn paste_image(&mut self) {
        match Clipboard::new().and_then(|mut clipboard| clipboard.get_image()) {
            Ok(data) => {
                let width = data.width as u32;
                let height = data.height as u32;
                match RgbaImage::from_raw(width, height, data.bytes.into_owned()) {
                    Some(image) => self.replace_image(DynamicImage::ImageRgba8(image)),
                    None => self.show_message(
                        "Clipboard error",
                        "The clipboard image has invalid pixel data.",
                    ),
                }
            }
            Err(_) => {
                #[cfg(windows)]
                if let Some(image) = read_windows_clipboard_image() {
                    self.replace_image(image);
                    return;
                }
                self.show_message(
                    "No image in clipboard",
                    "Copy a screenshot or another image, then try Ctrl+V.",
                );
            }
        }
    }

    fn copy_text(&mut self, text: &str) {
        if let Err(error) =
            Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text.to_owned()))
        {
            self.show_message("Clipboard error", error.to_string());
        }
    }

    fn new_window(&mut self) {
        match std::env::current_exe() {
            Ok(executable) => {
                if let Err(error) = Command::new(executable).spawn() {
                    self.show_message("Could not open a new window", error.to_string());
                }
            }
            Err(error) => self.show_message("Could not open a new window", error.to_string()),
        }
    }

    fn run_ocr(&mut self, rect: [i32; 4]) {
        let Some(source) = self.image.as_ref() else {
            return;
        };
        let left = rect[0].min(rect[2]).clamp(0, source.width() as i32) as u32;
        let right = rect[0].max(rect[2]).clamp(0, source.width() as i32) as u32;
        let top = rect[1].min(rect[3]).clamp(0, source.height() as i32) as u32;
        let bottom = rect[1].max(rect[3]).clamp(0, source.height() as i32) as u32;
        if right.saturating_sub(left) < 2 || bottom.saturating_sub(top) < 2 {
            return;
        }
        let crop = source
            .crop_imm(left, top, right - left, bottom - top)
            .to_rgb8();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "screenshot-markup-{}-{now}.png",
            std::process::id()
        ));
        if let Err(error) = crop.save(&path) {
            self.show_message("OCR failed", error.to_string());
            return;
        }
        let command = std::env::var_os("TESSERACT_CMD").unwrap_or_else(|| "tesseract".into());
        let result = Command::new(command).arg(&path).arg("stdout").output();
        let _ = std::fs::remove_file(path);
        match result {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                self.dialog = Some(Dialog::Ocr(OcrDialog {
                    text,
                    case_action: CASE_OPTIONS[0].to_owned(),
                    selection: (0, 0),
                    selected_count: 0,
                }));
            }
            Ok(output) => {
                let error = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                self.show_message(
                    "OCR failed",
                    if error.is_empty() {
                        "Tesseract could not read the selected image region. Verify that Tesseract is installed.".to_owned()
                    } else {
                        format!("{error}\n\nInstall Tesseract OCR and add it to PATH, or set TESSERACT_CMD.")
                    },
                );
            }
            Err(error) => self.show_message(
                "OCR unavailable",
                format!("Could not start Tesseract OCR: {error}\n\nInstall Tesseract and add it to PATH, or set TESSERACT_CMD."),
            ),
        }
    }

    fn show_message(&mut self, title: impl Into<String>, text: impl Into<String>) {
        self.dialog = Some(Dialog::Message(MessageDialog {
            title: title.into(),
            text: text.into(),
        }));
    }

    fn apply_text_dialog(&mut self, dialog: TextDialog) {
        if dialog.text.trim().is_empty() {
            return;
        }
        self.save_state();
        if let Some(id) = dialog.overlay_id {
            if let Some(overlay) = self.text_overlays.iter_mut().find(|item| item.id == id) {
                overlay.text = dialog.text;
            }
        } else {
            self.text_overlays.push(TextOverlay {
                id: self.next_text_id,
                x: dialog.x,
                y: dialog.y,
                text: dialog.text,
                font_family: "Arial".to_owned(),
                font_size: 14,
                color: [255, 0, 0, 255],
            });
            self.next_text_id += 1;
        }
        self.texture_dirty = true;
    }

    fn overlay_at(&self, image_point: Pos2) -> Option<u64> {
        self.text_overlays.iter().rev().find_map(|overlay| {
            let bounds = overlay_bounds(overlay);
            (image_point.x >= bounds[0] - 3.0
                && image_point.x <= bounds[2] + 3.0
                && image_point.y >= bounds[1] - 3.0
                && image_point.y <= bounds[3] + 3.0)
                .then_some(overlay.id)
        })
    }

    fn image_point(pointer: Pos2, canvas: Rect, origin: (i32, i32)) -> Pos2 {
        Pos2::new(
            pointer.x - canvas.left() - origin.0 as f32,
            pointer.y - canvas.top() - origin.1 as f32,
        )
    }

    fn handle_canvas_response(
        &mut self,
        ctx: &egui::Context,
        response: &egui::Response,
        rect: Rect,
    ) {
        let origin = image_origin(&self.prefs);
        let pointer = response.interact_pointer_pos();
        let image_point = pointer.map(|pointer| Self::image_point(pointer, rect, origin));
        let hovered_overlay = image_point.and_then(|point| self.overlay_at(point));
        response
            .clone()
            .on_hover_cursor(if self.dragging_text.is_some() {
                CursorIcon::Grabbing
            } else if hovered_overlay.is_some() {
                CursorIcon::Grab
            } else {
                CursorIcon::Crosshair
            });

        let mut open_new_window = false;
        let mut shadow_changed = false;
        response.context_menu(|ui| {
            ui.label("Tools");
            for tool in Tool::ALL {
                ui.selectable_value(&mut self.tool, tool, tool.label());
            }
            ui.separator();
            open_new_window = ui.button("Open New Window").clicked();
            let old_shadow = self.prefs.show_shadow;
            ui.checkbox(&mut self.prefs.show_shadow, "Shadow");
            shadow_changed |= old_shadow != self.prefs.show_shadow;
            if ui.button("Shadow Settings…").clicked() {
                self.dialog = Some(Dialog::ShadowSettings);
            }
        });
        if open_new_window {
            self.new_window();
        }
        if shadow_changed {
            self.prefs.save();
            self.texture_dirty = true;
        }

        if response.double_clicked() {
            if let Some(id) = hovered_overlay {
                if let Some(overlay) = self.text_overlays.iter().find(|item| item.id == id) {
                    self.dialog = Some(Dialog::Text(TextDialog {
                        overlay_id: Some(id),
                        x: overlay.x,
                        y: overlay.y,
                        text: overlay.text.clone(),
                    }));
                }
            }
        }

        if response.drag_started_by(PointerButton::Primary) {
            let start = ctx
                .input(|input| input.pointer.press_origin())
                .or(pointer)
                .map(|point| Self::image_point(point, rect, origin));
            if let (Some(start), Some(id)) = (start, start.and_then(|point| self.overlay_at(point)))
            {
                if let Some(overlay) = self.text_overlays.iter().find(|item| item.id == id) {
                    self.drag_text_offset = Pos2::new(start.x - overlay.x, start.y - overlay.y);
                    self.dragging_text = Some(id);
                    self.save_state();
                }
            } else if let Some(start) = start {
                if matches!(self.tool, Tool::Highlighter | Tool::Redaction | Tool::Ocr) {
                    self.drawing = Some(DragRectangle {
                        tool: self.tool,
                        start,
                        current: start,
                    });
                }
            }
        }

        if response.dragged_by(PointerButton::Primary) {
            if let Some(point) = image_point {
                if let Some(id) = self.dragging_text {
                    if let Some(overlay) = self.text_overlays.iter_mut().find(|item| item.id == id)
                    {
                        overlay.x = point.x - self.drag_text_offset.x;
                        overlay.y = point.y - self.drag_text_offset.y;
                        self.texture_dirty = true;
                    }
                } else if let Some(drawing) = self.drawing.as_mut() {
                    drawing.current = point;
                }
            }
        }

        if response.drag_stopped_by(PointerButton::Primary) {
            if self.dragging_text.take().is_some() {
                self.texture_dirty = true;
            } else if let Some(drawing) = self.drawing.take() {
                let coords = [
                    drawing.start.x.floor() as i32,
                    drawing.start.y.floor() as i32,
                    drawing.current.x.floor() as i32,
                    drawing.current.y.floor() as i32,
                ];
                match drawing.tool {
                    Tool::Ocr => self.run_ocr(coords),
                    Tool::Highlighter | Tool::Redaction => {
                        self.save_state();
                        if let Some(image) = self.image.as_mut() {
                            draw_markup(image, coords, drawing.tool);
                        }
                        self.texture_dirty = true;
                    }
                    _ => {}
                }
            }
        }

        if response.clicked_by(PointerButton::Primary) && self.dragging_text.is_none() {
            if let Some(point) = image_point {
                let overlay = self.overlay_at(point);
                if overlay.is_none() {
                    match self.tool {
                        Tool::Text => {
                            self.dialog = Some(Dialog::Text(TextDialog {
                                overlay_id: None,
                                x: point.x,
                                y: point.y,
                                text: String::new(),
                            }));
                        }
                        Tool::ColorPicker => self.show_color_info(point),
                        _ => {}
                    }
                }
            }
        }

        if let Some(drawing) = &self.drawing {
            let pointer_screen = Pos2::new(
                rect.left() + drawing.current.x + origin.0 as f32,
                rect.top() + drawing.current.y + origin.1 as f32,
            );
            let start_screen = Pos2::new(
                rect.left() + drawing.start.x + origin.0 as f32,
                rect.top() + drawing.start.y + origin.1 as f32,
            );
            let preview = Rect::from_two_pos(start_screen, pointer_screen);
            let (color, fill) = match drawing.tool {
                Tool::Highlighter => (
                    Color32::YELLOW,
                    Color32::from_rgba_unmultiplied(255, 255, 0, 128),
                ),
                Tool::Redaction => (Color32::BLACK, Color32::BLACK),
                Tool::Ocr => (Color32::CYAN, Color32::TRANSPARENT),
                _ => (Color32::TRANSPARENT, Color32::TRANSPARENT),
            };
            ui_paint_rect(ctx, preview, color, fill);
        }
    }

    fn show_color_info(&mut self, point: Pos2) {
        let Some(image) = self.image.as_ref() else {
            return;
        };
        let x = point.x.floor() as i32;
        let y = point.y.floor() as i32;
        if x < 0 || y < 0 || x >= image.width() as i32 || y >= image.height() as i32 {
            return;
        }
        let pixel = image.get_pixel(x as u32, y as u32).0;
        self.dialog = Some(Dialog::Color(ColorDialog {
            x,
            y,
            color: [pixel[0], pixel[1], pixel[2]],
        }));
    }

    fn show_top_bar(&mut self, ui: &mut egui::Ui) {
        let shadow_before = self.prefs.show_shadow;
        egui::Panel::top("top-menu").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open…  Ctrl+L").clicked() {
                        self.open_file_dialog();
                        ui.close();
                    }
                    if ui.button("Save As…  Ctrl+S").clicked() {
                        self.save_file_dialog();
                        ui.close();
                    }
                    if ui.button("Copy Image  Ctrl+C").clicked() {
                        self.copy_image();
                        ui.close();
                    }
                    if ui.button("Paste Image  Ctrl+V").clicked() {
                        self.paste_image();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Open New Window  Ctrl+N").clicked() {
                        self.new_window();
                        ui.close();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui
                        .add_enabled(
                            !self.undo_stack.is_empty(),
                            egui::Button::new("Undo  Ctrl+Z"),
                        )
                        .clicked()
                    {
                        self.undo();
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            !self.redo_stack.is_empty(),
                            egui::Button::new("Redo  Ctrl+Y"),
                        )
                        .clicked()
                    {
                        self.redo();
                        ui.close();
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.prefs.show_shadow, "Shadow");
                    if ui.button("Shadow Settings…").clicked() {
                        self.dialog = Some(Dialog::ShadowSettings);
                        ui.close();
                    }
                });
                ui.menu_button("Tools", |ui| {
                    for tool in Tool::ALL {
                        ui.selectable_value(&mut self.tool, tool, tool.label());
                    }
                });
            });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.label("Tool:");
                for tool in Tool::ALL {
                    ui.selectable_value(&mut self.tool, tool, tool.label());
                }
                ui.separator();
                ui.checkbox(&mut self.prefs.show_shadow, "Shadow");
                if ui.button("Open").clicked() {
                    self.open_file_dialog();
                }
                if ui.button("Save").clicked() {
                    self.save_file_dialog();
                }
                if ui.button("Copy").clicked() {
                    self.copy_image();
                }
            });
        });
        if shadow_before != self.prefs.show_shadow {
            self.prefs.save();
            self.texture_dirty = true;
        }
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ui, |ui| {
            if self.image.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Screenshot Markup");
                        ui.label("Paste a screenshot with Ctrl+V, open an image with Ctrl+L, or drop a file here.");
                        if ui.button("Open Image…").clicked() { self.open_file_dialog(); }
                    });
                });
                return;
            }
            self.update_texture(ctx);
            let Some(texture) = self.texture.as_ref().cloned() else { return };
            let size = Vec2::new(texture.size()[0] as f32, texture.size()[1] as f32);
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
                ui.painter().image(texture.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                self.handle_canvas_response(ctx, &response, rect);
            });
        });
    }

    fn show_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        if ctx.input(|input| input.key_pressed(Key::Escape)) {
            return;
        }
        let mut keep_dialog = true;
        match &mut dialog {
            Dialog::Text(text) => {
                let title = if text.overlay_id.is_some() {
                    "Edit Text"
                } else {
                    "Add Text"
                };
                let mut apply = false;
                let mut cancel = false;
                egui::Window::new(title)
                    .collapsible(false)
                    .resizable(true)
                    .show(ctx, |ui| {
                        ui.label(if text.overlay_id.is_some() {
                            "Update text:"
                        } else {
                            "Enter text:"
                        });
                        ui.add(
                            egui::TextEdit::multiline(&mut text.text)
                                .desired_rows(6)
                                .desired_width(360.0),
                        );
                        ui.horizontal(|ui| {
                            apply = ui.button("OK").clicked();
                            cancel = ui.button("Cancel").clicked();
                        });
                    });
                if apply {
                    self.apply_text_dialog(text.clone());
                    keep_dialog = false;
                }
                if cancel {
                    keep_dialog = false;
                }
            }
            Dialog::Ocr(ocr) => {
                let mut copy = false;
                let mut close = false;
                let mut apply_case = false;
                egui::Window::new("OCR Result")
                    .collapsible(false)
                    .resizable(true)
                    .default_size([520.0, 360.0])
                    .show(ctx, |ui| {
                        let output = egui::TextEdit::multiline(&mut ocr.text)
                            .desired_rows(12)
                            .desired_width(f32::INFINITY)
                            .show(ui);
                        if let Some(range) = output.cursor_range {
                            let start: usize = range.primary.index.into();
                            let end: usize = range.secondary.index.into();
                            ocr.selection = (start.min(end), start.max(end));
                            ocr.selected_count = ocr.selection.1.saturating_sub(ocr.selection.0);
                        }
                        ui.horizontal(|ui| {
                            copy = ui.button("Copy").clicked();
                            egui::ComboBox::from_id_salt("ocr-case")
                                .selected_text(&ocr.case_action)
                                .show_ui(ui, |ui| {
                                    for option in CASE_OPTIONS {
                                        ui.selectable_value(
                                            &mut ocr.case_action,
                                            option.to_owned(),
                                            option,
                                        );
                                    }
                                });
                            apply_case = ui.button("Apply to selection").clicked();
                            ui.label(format!("Selected: {}", ocr.selected_count));
                            close = ui.button("Close").clicked();
                        });
                    });
                if copy {
                    self.copy_text(&ocr.text);
                }
                if apply_case {
                    apply_case_to_selection(ocr);
                }
                if close {
                    keep_dialog = false;
                }
            }
            Dialog::Color(color) => {
                let mut copy_position = false;
                let mut copy_rgb = false;
                let mut copy_hex = false;
                let mut close = false;
                let rgb = format!("{}, {}, {}", color.color[0], color.color[1], color.color[2]);
                let hex = format!(
                    "#{:02x}{:02x}{:02x}",
                    color.color[0], color.color[1], color.color[2]
                );
                egui::Window::new("Color Information")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        let (swatch, _) = ui.allocate_exact_size(Vec2::splat(52.0), Sense::hover());
                        ui.painter().rect_filled(
                            swatch,
                            3.0,
                            Color32::from_rgb(color.color[0], color.color[1], color.color[2]),
                        );
                        egui::Grid::new("color-info-grid").show(ui, |ui| {
                            ui.label("Position:");
                            ui.monospace(format!("{}, {}", color.x, color.y));
                            copy_position = ui.button("Copy").clicked();
                            ui.end_row();
                            ui.label("RGB:");
                            ui.monospace(&rgb);
                            copy_rgb = ui.button("Copy").clicked();
                            ui.end_row();
                            ui.label("Hex:");
                            ui.monospace(&hex);
                            copy_hex = ui.button("Copy").clicked();
                            ui.end_row();
                        });
                        close = ui.button("Close").clicked();
                    });
                if copy_position {
                    self.copy_text(&format!("{}, {}", color.x, color.y));
                }
                if copy_rgb {
                    self.copy_text(&rgb);
                }
                if copy_hex {
                    self.copy_text(&hex);
                }
                if close {
                    keep_dialog = false;
                }
            }
            Dialog::Message(message) => {
                let mut close = false;
                egui::Window::new(&message.title)
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(&message.text);
                        close = ui.button("OK").clicked();
                    });
                if close {
                    keep_dialog = false;
                }
            }
            Dialog::ShadowSettings => {
                let mut close = false;
                let mut changed = false;
                egui::Window::new("Shadow Settings")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut self.prefs.shadow_offset_x, -64..=64)
                                    .text("Horizontal offset"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut self.prefs.shadow_offset_y, -64..=64)
                                    .text("Vertical offset"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut self.prefs.shadow_blur, 0.0..=32.0)
                                    .text("Blur radius"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut self.prefs.shadow_padding, 0..=128)
                                    .text("Border padding"),
                            )
                            .changed();
                        let mut shadow = Color32::from_rgba_unmultiplied(
                            self.prefs.shadow_color[0],
                            self.prefs.shadow_color[1],
                            self.prefs.shadow_color[2],
                            self.prefs.shadow_color[3],
                        );
                        ui.horizontal(|ui| {
                            ui.label("Shadow color");
                            changed |= ui.color_edit_button_srgba(&mut shadow).changed();
                        });
                        self.prefs.shadow_color = [shadow.r(), shadow.g(), shadow.b(), shadow.a()];
                        let mut background = Color32::from_rgba_unmultiplied(
                            self.prefs.background_color[0],
                            self.prefs.background_color[1],
                            self.prefs.background_color[2],
                            self.prefs.background_color[3],
                        );
                        ui.horizontal(|ui| {
                            ui.label("Background color");
                            changed |= ui.color_edit_button_srgba(&mut background).changed();
                        });
                        self.prefs.background_color = [
                            background.r(),
                            background.g(),
                            background.b(),
                            background.a(),
                        ];
                        close = ui.button("Done").clicked();
                    });
                if changed {
                    self.prefs.save();
                    self.texture_dirty = true;
                }
                if close {
                    keep_dialog = false;
                }
            }
        }
        if keep_dialog {
            self.dialog = Some(dialog);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if matches!(self.dialog, Some(Dialog::Text(_) | Dialog::Ocr(_))) {
            self.handle_text_clipboard_shortcuts(ctx);
            return;
        }
        let ctrl = Modifiers::CTRL;
        if ctx.input_mut(|input| input.consume_key(ctrl, Key::V)) {
            self.paste_image();
        }
        if ctx.input_mut(|input| input.consume_key(ctrl, Key::C)) {
            self.copy_image();
        }
        if ctx.input_mut(|input| input.consume_key(ctrl, Key::S)) {
            self.save_file_dialog();
        }
        if ctx.input_mut(|input| input.consume_key(ctrl, Key::L)) {
            self.open_file_dialog();
        }
        if ctx.input_mut(|input| input.consume_key(ctrl, Key::N)) {
            self.new_window();
        }
        if ctx.input_mut(|input| input.consume_key(ctrl, Key::Z)) {
            self.undo();
        }
        if ctx.input_mut(|input| input.consume_key(ctrl, Key::Y)) {
            self.redo();
        }
    }

    fn handle_text_clipboard_shortcuts(&mut self, ctx: &egui::Context) {
        let ctrl = Modifiers::CTRL;
        for (key, event) in [(Key::C, egui::Event::Copy), (Key::X, egui::Event::Cut)] {
            if ctx.input_mut(|input| input.consume_key(ctrl, key)) {
                ctx.input_mut(|input| input.events.push(event));
            }
        }
        if ctx.input_mut(|input| input.consume_key(ctrl, Key::V)) {
            if let Ok(text) = Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
                ctx.input_mut(|input| {
                    input.events.push(egui::Event::Paste(text.replace("\r\n", "\n")));
                });
            }
        }
    }
}

impl eframe::App for MarkupApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        let dropped = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .collect::<Vec<_>>()
        });
        if let Some(path) = dropped.into_iter().next() {
            self.open_path(path);
        }
        self.fit_window_to_image(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.show_top_bar(ui);
        self.show_canvas(ui, &ctx);
        self.show_dialog(&ctx);
    }
}

fn ui_paint_rect(ctx: &egui::Context, rect: Rect, outline: Color32, fill: Color32) {
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("markup-preview"),
    ))
    .rect_filled(rect, 0.0, fill);
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("markup-preview-outline"),
    ))
    .rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.5, outline),
        egui::StrokeKind::Middle,
    );
}

fn apply_case_to_selection(dialog: &mut OcrDialog) {
    if dialog.case_action == "No Change" || dialog.selection.0 == dialog.selection.1 {
        return;
    }
    let chars: Vec<char> = dialog.text.chars().collect();
    let start = dialog.selection.0.min(chars.len());
    let end = dialog.selection.1.min(chars.len());
    if start >= end {
        return;
    }
    let selected: String = chars[start..end].iter().collect();
    let transformed = transform_case(&selected, &dialog.case_action);
    let prefix: String = chars[..start].iter().collect();
    let suffix: String = chars[end..].iter().collect();
    let transformed_len = transformed.chars().count();
    dialog.text = format!("{prefix}{transformed}{suffix}");
    dialog.selection = (start, start + transformed_len);
    dialog.selected_count = transformed_len;
}

#[cfg(windows)]
fn read_windows_clipboard_image() -> Option<DynamicImage> {
    use clipboard_win::{formats, get_clipboard};

    let dib_image = [formats::CF_DIB, formats::CF_DIBV5]
        .into_iter()
        .find_map(|format| {
            let dib = get_clipboard(formats::RawData(format)).ok()?;
            let bmp = dib_to_bmp(&dib).ok()?;
            image::load_from_memory_with_format(&bmp, image::ImageFormat::Bmp).ok()
        });

    dib_image.or_else(|| {
        let bmp = get_clipboard(formats::Bitmap).ok()?;
        image::load_from_memory_with_format(&bmp, image::ImageFormat::Bmp).ok()
    })
}

fn dib_to_bmp(dib: &[u8]) -> Result<Vec<u8>, &'static str> {
    let header_size = read_u32(dib, 0).ok_or("missing DIB header")? as usize;
    let pixel_offset = match header_size {
        12 => {
            if dib.len() < 12 {
                return Err("truncated bitmap core header");
            }
            let bit_count = read_u16(dib, 10).ok_or("missing bit depth")? as usize;
            let palette_entries = if bit_count <= 8 {
                1usize << bit_count
            } else {
                0
            };
            12usize
                .checked_add(palette_entries.checked_mul(3).ok_or("palette too large")?)
                .ok_or("bitmap offset overflow")?
        }
        size if size >= 40 => {
            if dib.len() < size {
                return Err("truncated bitmap info header");
            }
            let bit_count = read_u16(dib, 14).ok_or("missing bit depth")? as usize;
            let compression = read_u32(dib, 16).ok_or("missing compression mode")?;
            let colors_used = read_u32(dib, 32).ok_or("missing color table size")? as usize;
            let external_masks = if size == 40 {
                match compression {
                    3 => 12usize,
                    6 => 16usize,
                    _ => 0,
                }
            } else {
                0
            };
            let default_palette = if bit_count <= 8 {
                1usize << bit_count
            } else {
                0
            };
            let palette_entries = colors_used.max(default_palette);
            size.checked_add(external_masks)
                .and_then(|offset| offset.checked_add(palette_entries.checked_mul(4)?))
                .ok_or("bitmap offset overflow")?
        }
        _ => return Err("unsupported bitmap header"),
    };

    if pixel_offset > dib.len() {
        return Err("bitmap pixels are missing");
    }
    let file_size = dib
        .len()
        .checked_add(14)
        .and_then(|size| u32::try_from(size).ok())
        .ok_or("bitmap is too large")?;
    let file_pixel_offset = pixel_offset
        .checked_add(14)
        .and_then(|offset| u32::try_from(offset).ok())
        .ok_or("bitmap offset is too large")?;

    let mut bmp = Vec::with_capacity(file_size as usize);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&file_size.to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&file_pixel_offset.to_le_bytes());
    bmp.extend_from_slice(dib);
    Ok(bmp)
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_transform_applies_only_to_selected_unicode_text() {
        let mut dialog = OcrDialog {
            text: "one hELLO two".to_owned(),
            case_action: "UPPERCASE".to_owned(),
            selection: (4, 9),
            selected_count: 5,
        };
        apply_case_to_selection(&mut dialog);
        assert_eq!(dialog.text, "one HELLO two");
        assert_eq!(dialog.selection, (4, 9));
        let mut unicode = OcrDialog {
            text: "a café z".to_owned(),
            case_action: "UPPERCASE".to_owned(),
            selection: (2, 6),
            selected_count: 4,
        };
        apply_case_to_selection(&mut unicode);
        assert_eq!(unicode.text, "a CAFÉ z");
    }

    #[test]
    fn clipboard_dib_is_wrapped_as_a_decodable_bmp() {
        let mut dib = vec![0; 40];
        dib[0..4].copy_from_slice(&40u32.to_le_bytes());
        dib[4..8].copy_from_slice(&1i32.to_le_bytes());
        dib[8..12].copy_from_slice(&1i32.to_le_bytes());
        dib[12..14].copy_from_slice(&1u16.to_le_bytes());
        dib[14..16].copy_from_slice(&24u16.to_le_bytes());
        dib.extend_from_slice(&[0, 0, 255, 0]);

        let bmp = dib_to_bmp(&dib).expect("valid DIB should be wrapped");
        let image = image::load_from_memory_with_format(&bmp, image::ImageFormat::Bmp)
            .expect("wrapped DIB should decode");
        assert_eq!(image.to_rgb8().get_pixel(0, 0).0, [255, 0, 0]);
    }

    #[test]
    fn clipboard_dib_rejects_truncated_headers() {
        assert!(dib_to_bmp(&[40, 0, 0, 0]).is_err());
    }
}
