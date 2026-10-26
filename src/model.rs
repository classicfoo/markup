use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::{drawing::draw_text_mut, filter::gaussian_blur_f32};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::OnceLock};

const SHADOW_BORDER: u32 = 20;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tool {
    #[default]
    Highlighter,
    Redaction,
    ColorPicker,
    Text,
    Ocr,
}

impl Tool {
    pub const ALL: [Tool; 5] = [
        Tool::Highlighter,
        Tool::Redaction,
        Tool::ColorPicker,
        Tool::Text,
        Tool::Ocr,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tool::Highlighter => "Highlighter",
            Tool::Redaction => "Redaction",
            Tool::ColorPicker => "Color Picker",
            Tool::Text => "Text",
            Tool::Ocr => "OCR",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextOverlay {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub font_family: String,
    pub font_size: u32,
    pub color: [u8; 4],
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub image: DynamicImage,
    pub text_overlays: Vec<TextOverlay>,
    pub next_text_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub last_output_format: String,
    pub show_shadow: bool,
    pub shadow_offset_x: i32,
    pub shadow_offset_y: i32,
    pub shadow_blur: f32,
    pub shadow_padding: u32,
    pub shadow_color: [u8; 4],
    pub background_color: [u8; 4],
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            last_output_format: "png".to_owned(),
            show_shadow: true,
            shadow_offset_x: 13,
            shadow_offset_y: 13,
            shadow_blur: 8.0,
            shadow_padding: SHADOW_BORDER,
            shadow_color: [128, 128, 128, 255],
            background_color: [255, 255, 255, 255],
        }
    }
}

impl Preferences {
    pub fn path() -> PathBuf {
        let base = dirs::config_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(std::env::temp_dir);
        base.join("screenshot_markup").join("config.json")
    }

    pub fn load() -> Self {
        let contents = std::fs::read_to_string(Self::path()).ok().or_else(|| {
            dirs::home_dir()
                .map(|home| home.join(".screenshot_markup_config.json"))
                .and_then(|legacy| std::fs::read_to_string(legacy).ok())
        });
        let Some(contents) = contents else {
            return Self::default();
        };
        let mut prefs: Self = serde_json::from_str(&contents).unwrap_or_default();
        prefs.last_output_format = normalize_format(&prefs.last_output_format).to_owned();
        prefs.shadow_offset_x = prefs.shadow_offset_x.clamp(-256, 256);
        prefs.shadow_offset_y = prefs.shadow_offset_y.clamp(-256, 256);
        prefs.shadow_blur = prefs.shadow_blur.clamp(0.0, 32.0);
        prefs.shadow_padding = prefs.shadow_padding.min(256);
        prefs
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(contents) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, contents);
        }
    }
}

pub fn normalize_format(file_format: &str) -> &'static str {
    if matches!(file_format.to_ascii_lowercase().as_str(), "jpg" | "jpeg") {
        "jpg"
    } else {
        "png"
    }
}

pub fn load_image(path: &std::path::Path) -> image::ImageResult<DynamicImage> {
    image::ImageReader::open(path)?
        .decode()
        .map(|image| image.to_rgba8().into())
}

pub fn draw_markup(image: &mut DynamicImage, rect: [i32; 4], tool: Tool) {
    let (width, height) = (image.width() as i32, image.height() as i32);
    if width == 0 || height == 0 {
        return;
    }
    let left = rect[0].min(rect[2]).clamp(0, width - 1);
    let right = rect[0].max(rect[2]).clamp(0, width - 1);
    let top = rect[1].min(rect[3]).clamp(0, height - 1);
    let bottom = rect[1].max(rect[3]).clamp(0, height - 1);
    let mut pixels = image.to_rgba8();
    for y in top..=bottom {
        for x in left..=right {
            let pixel = pixels.get_pixel_mut(x as u32, y as u32);
            match tool {
                Tool::Highlighter => *pixel = blend_pixel(*pixel, Rgba([255, 255, 0, 128])),
                Tool::Redaction => *pixel = Rgba([0, 0, 0, 255]),
                _ => return,
            }
        }
    }
    *image = DynamicImage::ImageRgba8(pixels);
}

fn blend_pixel(background: Rgba<u8>, foreground: Rgba<u8>) -> Rgba<u8> {
    let alpha = u32::from(foreground[3]);
    let inverse = 255 - alpha;
    let out_alpha = alpha + (u32::from(background[3]) * inverse + 127) / 255;
    if out_alpha == 0 {
        return Rgba([0, 0, 0, 0]);
    }
    let mut out = [0; 4];
    for channel in 0..3 {
        let fg = u32::from(foreground[channel]) * alpha;
        let bg = u32::from(background[channel]) * u32::from(background[3]) * inverse / 255;
        out[channel] = ((fg + bg + out_alpha / 2) / out_alpha).min(255) as u8;
    }
    out[3] = out_alpha.min(255) as u8;
    Rgba(out)
}

pub fn build_export_image(
    image: &DynamicImage,
    text_overlays: &[TextOverlay],
    prefs: &Preferences,
) -> DynamicImage {
    let mut output = if prefs.show_shadow {
        add_shadow(image, prefs)
    } else {
        image.to_rgba8()
    };
    let (text_offset_x, text_offset_y) = image_origin(prefs);
    let font = load_markup_font();
    for overlay in text_overlays {
        let scale = PxScale::from(overlay.font_size.max(1) as f32);
        let line_height =
            (font.as_scaled(scale).height() + font.as_scaled(scale).line_gap()).ceil() as i32;
        for (line, value) in overlay.text.split('\n').enumerate() {
            draw_text_mut(
                &mut output,
                Rgba(overlay.color),
                overlay.x.round() as i32 + text_offset_x,
                overlay.y.round() as i32 + text_offset_y + line as i32 * line_height,
                scale,
                &font,
                value,
            );
        }
    }
    DynamicImage::ImageRgba8(output)
}

pub fn image_origin(prefs: &Preferences) -> (i32, i32) {
    if prefs.show_shadow {
        (
            prefs.shadow_padding as i32 - prefs.shadow_offset_x.min(0),
            prefs.shadow_padding as i32 - prefs.shadow_offset_y.min(0),
        )
    } else {
        (0, 0)
    }
}

fn add_shadow(source: &DynamicImage, prefs: &Preferences) -> RgbaImage {
    let source = source.to_rgba8();
    let (width, height) = source.dimensions();
    let bordered_width = width + 2;
    let bordered_height = height + 2;
    let offset_x = prefs.shadow_offset_x;
    let offset_y = prefs.shadow_offset_y;
    let total_width = bordered_width + offset_x.unsigned_abs() + prefs.shadow_padding * 2;
    let total_height = bordered_height + offset_y.unsigned_abs() + prefs.shadow_padding * 2;
    let mut canvas =
        ImageBuffer::from_pixel(total_width, total_height, Rgba(prefs.background_color));

    let shadow_left = prefs.shadow_padding as i32 + offset_x.max(0);
    let shadow_top = prefs.shadow_padding as i32 + offset_y.max(0);
    let left = shadow_left.clamp(0, total_width as i32) as u32;
    let top = shadow_top.clamp(0, total_height as i32) as u32;
    let right = (shadow_left + bordered_width as i32).clamp(0, total_width as i32) as u32;
    let bottom = (shadow_top + bordered_height as i32).clamp(0, total_height as i32) as u32;
    for y in top..bottom {
        for x in left..right {
            canvas.put_pixel(x, y, Rgba(prefs.shadow_color));
        }
    }
    if prefs.shadow_blur > 0.0 {
        canvas = gaussian_blur_f32(&canvas, prefs.shadow_blur);
    }

    let image_left = (prefs.shadow_padding as i32 - offset_x.min(0)).max(0) as u32;
    let image_top = (prefs.shadow_padding as i32 - offset_y.min(0)).max(0) as u32;
    for y in 0..bordered_height {
        for x in 0..bordered_width {
            let pixel = if x == 0 || y == 0 || x == bordered_width - 1 || y == bordered_height - 1 {
                Rgba([211, 211, 211, 255])
            } else {
                source.get_pixel(x - 1, y - 1).to_owned()
            };
            if image_left + x < total_width && image_top + y < total_height {
                canvas.put_pixel(
                    image_left + x,
                    image_top + y,
                    blend_pixel(
                        canvas.get_pixel(image_left + x, image_top + y).to_owned(),
                        pixel,
                    ),
                );
            }
        }
    }
    canvas
}

fn load_markup_font() -> FontArc {
    static FONT: OnceLock<FontArc> = OnceLock::new();
    FONT.get_or_init(|| {
        let candidates = if cfg!(target_os = "windows") {
            vec![
                std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_owned())
                    + "\\Fonts\\arial.ttf",
                std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_owned())
                    + "\\Fonts\\segoeui.ttf",
            ]
        } else if cfg!(target_os = "macos") {
            vec![
                "/System/Library/Fonts/Supplemental/Arial.ttf".to_owned(),
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf".to_owned(),
            ]
        } else {
            vec![
                "/usr/share/fonts/truetype/msttcorefonts/Arial.ttf".to_owned(),
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_owned(),
                "/usr/share/fonts/dejavu/DejaVuSans.ttf".to_owned(),
            ]
        };
        candidates
            .into_iter()
            .find_map(|path| {
                std::fs::read(path)
                    .ok()
                    .and_then(|bytes| FontArc::try_from_vec(bytes).ok())
            })
            .unwrap_or_else(|| {
                FontArc::try_from_vec(include_bytes!("../assets/DejaVuSans.ttf").to_vec())
                    .expect("bundled fallback font is valid")
            })
    })
    .clone()
}

pub fn transform_case(text: &str, mode: &str) -> String {
    match mode {
        "Sentence case" => {
            let mut capitalize_next = true;
            text.to_lowercase()
                .chars()
                .map(|character| {
                    let result = if capitalize_next && character.is_alphabetic() {
                        capitalize_next = false;
                        character.to_uppercase().collect::<String>()
                    } else {
                        character.to_string()
                    };
                    if matches!(character, '.' | '!' | '?') {
                        capitalize_next = true;
                    }
                    result
                })
                .collect()
        }
        "lowercase" => text.to_lowercase(),
        "UPPERCASE" => text.to_uppercase(),
        "Capitalize Each Word" => {
            let mut capitalize_next = true;
            text.to_lowercase()
                .chars()
                .map(|character| {
                    if character.is_alphanumeric() && capitalize_next {
                        capitalize_next = false;
                        character.to_uppercase().collect::<String>()
                    } else if character.is_alphanumeric() {
                        character.to_string()
                    } else {
                        capitalize_next = true;
                        character.to_string()
                    }
                })
                .collect()
        }
        "tOGGLE cASE" => text
            .chars()
            .flat_map(|character| {
                if character.is_lowercase() {
                    character.to_uppercase().collect::<Vec<_>>()
                } else {
                    character.to_lowercase().collect::<Vec<_>>()
                }
            })
            .collect(),
        _ => text.to_owned(),
    }
}

pub fn overlay_bounds(overlay: &TextOverlay) -> [f32; 4] {
    let font = load_markup_font();
    let scale = PxScale::from(overlay.font_size.max(1) as f32);
    let scaled = font.as_scaled(scale);
    let line_height = scaled.height() + scaled.line_gap();
    let mut x = 0.0_f32;
    let mut max_x = 0.0_f32;
    let mut lines = 1_u32;
    for character in overlay.text.chars() {
        if character == '\n' {
            max_x = max_x.max(x);
            x = 0.0;
            lines += 1;
        } else {
            x += scaled.h_advance(font.glyph_id(character));
        }
    }
    max_x = max_x.max(x);
    [
        overlay.x,
        overlay.y,
        overlay.x + max_x.max(8.0),
        overlay.y + line_height * lines as f32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;

    #[test]
    fn format_normalization_defaults_to_png() {
        assert_eq!(normalize_format("JPEG"), "jpg");
        assert_eq!(normalize_format("jpg"), "jpg");
        assert_eq!(normalize_format("anything"), "png");
    }

    #[test]
    fn markup_clips_to_image_and_uses_expected_colors() {
        let mut image =
            DynamicImage::ImageRgba8(ImageBuffer::from_pixel(4, 4, Rgba([20, 40, 60, 255])));
        draw_markup(&mut image, [-2, -2, 1, 1], Tool::Highlighter);
        let pixels = image.to_rgba8();
        assert_eq!(pixels.get_pixel(0, 0), &Rgba([138, 148, 30, 255]));
        assert_eq!(pixels.get_pixel(2, 0), &Rgba([20, 40, 60, 255]));
        draw_markup(&mut image, [2, 2, 7, 7], Tool::Redaction);
        assert_eq!(image.to_rgba8().get_pixel(3, 3), &Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn export_shadow_and_text_sizes_match_configured_canvas() {
        let source =
            DynamicImage::ImageRgba8(ImageBuffer::from_pixel(10, 8, Rgba([255, 255, 255, 255])));
        let prefs = Preferences::default();
        let result = build_export_image(&source, &[], &prefs);
        assert_eq!(result.dimensions(), (65, 63));
        let no_shadow = Preferences {
            show_shadow: false,
            ..prefs
        };
        assert_eq!(
            build_export_image(&source, &[], &no_shadow).dimensions(),
            (10, 8)
        );
    }

    #[test]
    fn canvas_origin_tracks_shadow_padding_and_negative_offsets() {
        let prefs = Preferences::default();
        assert_eq!(image_origin(&prefs), (20, 20));
        let shifted = Preferences {
            shadow_offset_x: -7,
            shadow_offset_y: 5,
            shadow_padding: 11,
            ..prefs.clone()
        };
        assert_eq!(image_origin(&shifted), (18, 11));
        assert_eq!(
            image_origin(&Preferences {
                show_shadow: false,
                ..shifted
            }),
            (0, 0)
        );
    }

    #[test]
    fn case_transform_modes_match_ocr_options() {
        assert_eq!(
            transform_case("hELLO wORLD", "Sentence case"),
            "Hello world"
        );
        assert_eq!(transform_case("hELLO wORLD", "lowercase"), "hello world");
        assert_eq!(transform_case("hello", "UPPERCASE"), "HELLO");
        assert_eq!(
            transform_case("hello world", "Capitalize Each Word"),
            "Hello World"
        );
        assert_eq!(
            transform_case("rock'n'roll", "Capitalize Each Word"),
            "Rock'N'Roll"
        );
        assert_eq!(transform_case("Hello", "tOGGLE cASE"), "hELLO");
    }

    #[test]
    fn text_export_preserves_multiline_overlays() {
        let source =
            DynamicImage::ImageRgba8(ImageBuffer::from_pixel(120, 60, Rgba([255, 255, 255, 255])));
        let prefs = Preferences {
            show_shadow: false,
            ..Preferences::default()
        };
        let overlay = TextOverlay {
            id: 1,
            x: 5.0,
            y: 5.0,
            text: "First line\nSecond line".to_owned(),
            font_family: "Arial".to_owned(),
            font_size: 16,
            color: [255, 0, 0, 255],
        };
        let result = build_export_image(&source, &[overlay], &prefs).to_rgba8();
        let red_rows: Vec<u32> = (0..result.height())
            .filter(|y| {
                (0..result.width()).any(|x| {
                    let pixel = result.get_pixel(x, *y);
                    pixel[0] > 240 && pixel[1] < 120 && pixel[2] < 120
                })
            })
            .collect();
        assert!(
            red_rows.last().unwrap() - red_rows.first().unwrap() >= 20,
            "the second line must be rendered below the first: {red_rows:?}"
        );
    }
}
