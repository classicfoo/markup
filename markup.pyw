import tkinter as tk
from tkinter import ttk
from PIL import Image, ImageTk, ImageDraw, ImageFilter, ImageOps, ImageGrab, ImageFont
import os
import sys
import shutil
import subprocess
import tempfile
import json
from io import BytesIO
from tkinter import filedialog, messagebox, colorchooser
import pyperclip  # You'll need to pip install pyperclip


class ColorInfoDialog(tk.Toplevel):
    def __init__(self, parent, color_rgb, x, y):
        super().__init__(parent)
        self.title("Color Information")
        
        # Make dialog modal
        self.transient(parent)
        
        # Convert RGB to hex
        rgb_hex = '#{:02x}{:02x}{:02x}'.format(*color_rgb)
        
        # Create main frame with padding
        main_frame = ttk.Frame(self, padding="10")
        main_frame.pack(fill="both", expand=True)
        
        # Color preview at the top
        preview_frame = ttk.Frame(main_frame)
        preview_frame.pack(fill="x", pady=(0, 10))
        preview = tk.Canvas(preview_frame, width=50, height=50, bg=rgb_hex)
        preview.pack()

        # Create grid frame
        grid_frame = ttk.Frame(main_frame)
        grid_frame.pack(fill="x", pady=(0, 10))
        
        # Position information
        ttk.Label(grid_frame, text="Position:").grid(row=0, column=0, sticky="e", padx=5, pady=2)
        pos_var = tk.StringVar(value=f"{x}, {y}")
        pos_entry = ttk.Entry(grid_frame, textvariable=pos_var, width=20, state="readonly")
        pos_entry.grid(row=0, column=1, sticky="w", padx=5, pady=2)
        ttk.Button(grid_frame, text="Copy", 
                  command=lambda: pyperclip.copy(pos_var.get())
        ).grid(row=0, column=2, padx=5, pady=2)
        
        # RGB information
        ttk.Label(grid_frame, text="RGB:").grid(row=1, column=0, sticky="e", padx=5, pady=2)
        rgb_var = tk.StringVar(value=f"{color_rgb[0]}, {color_rgb[1]}, {color_rgb[2]}")
        rgb_entry = ttk.Entry(grid_frame, textvariable=rgb_var, width=20, state="readonly")
        rgb_entry.grid(row=1, column=1, sticky="w", padx=5, pady=2)
        ttk.Button(grid_frame, text="Copy",
                  command=lambda: pyperclip.copy(rgb_var.get())
        ).grid(row=1, column=2, padx=5, pady=2)
        
        # Hex information
        ttk.Label(grid_frame, text="Hex:").grid(row=2, column=0, sticky="e", padx=5, pady=2)
        hex_var = tk.StringVar(value=rgb_hex)
        hex_entry = ttk.Entry(grid_frame, textvariable=hex_var, width=20, state="readonly")
        hex_entry.grid(row=2, column=1, sticky="w", padx=5, pady=2)
        ttk.Button(grid_frame, text="Copy",
                  command=lambda: pyperclip.copy(hex_var.get())
        ).grid(row=2, column=2, padx=5, pady=2)
        
        # Close button at the bottom
        ttk.Button(main_frame, text="Close", command=self.destroy).pack(pady=(0, 5))
        
        # Configure grid weights
        grid_frame.columnconfigure(1, weight=1)
        
        # Center dialog on parent window
        self.geometry(f"+{parent.winfo_rootx() + 50}+{parent.winfo_rooty() + 50}")
        
        # Make dialog non-resizable
        self.resizable(False, False)
        self.wait_visibility()
        self.grab_set()

class MultilineTextDialog(tk.Toplevel):
    def __init__(self, parent, title, prompt, initial_text=""):
        super().__init__(parent)
        self.result = None
        self.title(title)
        self.transient(parent)
        self.grab_set()

        main_frame = ttk.Frame(self, padding="10")
        main_frame.pack(fill="both", expand=True)

        ttk.Label(main_frame, text=prompt).pack(anchor="w", pady=(0, 6))

        self.text_input = tk.Text(main_frame, width=48, height=6, wrap="word")
        self.text_input.pack(fill="both", expand=True)
        self.text_input.insert("1.0", initial_text)
        self.text_input.focus_set()
        self.text_input.bind("<Tab>", self.focus_next_widget)
        self.text_input.bind("<Shift-Tab>", self.focus_previous_widget)

        button_frame = ttk.Frame(main_frame)
        button_frame.pack(fill="x", pady=(10, 0))
        self.ok_button = ttk.Button(button_frame, text="OK", command=self.on_ok)
        self.ok_button.pack(side="right")
        self.cancel_button = ttk.Button(button_frame, text="Cancel", command=self.on_cancel)
        self.cancel_button.pack(side="right", padx=(0, 8))
        self.ok_button.bind("<Return>", lambda event: self.invoke_focused_button(event))
        self.ok_button.bind("<KP_Enter>", lambda event: self.invoke_focused_button(event))
        self.cancel_button.bind("<Return>", lambda event: self.invoke_focused_button(event))
        self.cancel_button.bind("<KP_Enter>", lambda event: self.invoke_focused_button(event))

        self.bind("<Escape>", lambda event: self.on_cancel())
        self.bind("<Control-Return>", lambda event: self.on_ok())
        self.protocol("WM_DELETE_WINDOW", self.on_cancel)

        self.geometry(f"+{parent.winfo_rootx() + 80}+{parent.winfo_rooty() + 80}")

    def on_ok(self):
        self.result = self.text_input.get("1.0", "end-1c")
        self.destroy()

    def on_cancel(self):
        self.result = None
        self.destroy()

    def focus_next_widget(self, event):
        if event.widget == self.text_input:
            self.ok_button.focus_set()
            return "break"
        next_widget = event.widget.tk_focusNext()
        if next_widget:
            next_widget.focus_set()
        return "break"

    def focus_previous_widget(self, event):
        previous_widget = event.widget.tk_focusPrev()
        if previous_widget:
            previous_widget.focus_set()
        return "break"

    def invoke_focused_button(self, event):
        event.widget.invoke()
        return "break"

class ImageViewer(tk.Tk):
    def __init__(self, image_path=None):
        super().__init__()
        self.title("Screenshot Markup")

        # Add undo/redo stacks
        self.undo_stack = []
        self.redo_stack = []
        self.max_undos = 20  # Limit stack size to prevent memory issues

        # Remove toolbar-related code
        self.drawing_mode = tk.StringVar(value="highlighter")
        self.show_shadow = tk.BooleanVar(value=True)

        # Image handling
        self.start_x = None
        self.start_y = None
        self.rect = None
        self.original_image = None
        self.final_image = None
        self.text_overlays = []
        self.next_text_id = 1
        self.dragging_text_id = None
        self.drag_text_dx = 0
        self.drag_text_dy = 0
        self.drag_state_saved = False
        self.default_text_font = "Arial"
        self.default_text_size = 14
        self.default_text_color = "red"
        self.config_path = os.path.join(os.path.expanduser("~"), ".screenshot_markup_config.json")
        self.default_output_format = "png"
        self.load_preferences()

        # Setting up the canvas
        self.canvas = tk.Canvas(self, cursor="cross")
        self.canvas.pack(fill="both", expand=True)

        # Create context menu
        self.context_menu = tk.Menu(self, tearoff=0)
        self.create_context_menu()

        # Bind mouse events
        self.canvas.bind("<ButtonPress-1>", self.on_button_press)
        self.canvas.bind("<B1-Motion>", self.on_move_press)
        self.canvas.bind("<ButtonRelease-1>", self.on_button_release)
        self.canvas.bind("<Motion>", self.on_mouse_move)
        self.canvas.bind("<Double-Button-1>", self.on_double_click)
        self.canvas.bind("<Button-3>", self.show_context_menu)  # Right-click

        # Keyboard shortcuts
        self.bind("<Control-v>", lambda event: self.load_image_from_clipboard())
        self.bind("<Control-c>", self.copy_image)
        self.bind("<Control-s>", lambda event: self.save_image())
        self.bind("<Control-z>", self.undo)
        self.bind("<Control-y>", self.redo)
        self.bind("<Control-l>", lambda event: self.load_image_from_file())
        self.bind("<Control-n>", lambda event: self.open_new_window())

        if image_path:
            self.load_image(image_path)
        self.update_image()

    def create_context_menu(self):
        # Tools submenu (changed from Drawing mode)
        self.tools_submenu = tk.Menu(self.context_menu, tearoff=0)
        self.tools_submenu.add_radiobutton(
            label="Highlighter", 
            variable=self.drawing_mode,
            value="highlighter"
        )
        self.tools_submenu.add_radiobutton(
            label="Redaction", 
            variable=self.drawing_mode,
            value="redaction"
        )
        self.tools_submenu.add_radiobutton(
            label="Color Picker",
            variable=self.drawing_mode,
            value="color_picker"
        )
        self.tools_submenu.add_radiobutton(
            label="Text",
            variable=self.drawing_mode,
            value="text"
        )
        
        self.context_menu.add_cascade(
            label="Tools",  # Changed from "Drawing Mode"
            menu=self.tools_submenu
        )

        self.context_menu.add_command(
            label="New Window",
            command=self.open_new_window
        )
        # Add separator
        self.context_menu.add_separator()
        
        # Shadow toggle
        self.context_menu.add_checkbutton(
            label="Shadow",
            variable=self.show_shadow,
            command=self.update_image
        )

    def open_new_window(self):
        image_path = None
        if self.final_image is not None:
            temp_file = tempfile.NamedTemporaryFile(delete=False, suffix=".png")
            temp_file.close()
            self.final_image.save(temp_file.name, "PNG")
            image_path = temp_file.name

        script_path = os.path.abspath(sys.argv[0])
        try:
            if image_path:
                subprocess.Popen([sys.executable, script_path, image_path])
            else:
                subprocess.Popen([sys.executable, script_path])
        except Exception as exc:
            messagebox.showerror(
                "Screenshot Markup",
                f"Failed to open a new window: {exc}",
            )

    def show_context_menu(self, event):
        try:
            self.context_menu.tk_popup(event.x_root, event.y_root)
        finally:
            self.context_menu.grab_release()

    def normalize_output_format(self, file_format):
        if isinstance(file_format, str) and file_format.lower() in ("jpg", "jpeg"):
            return "jpg"
        return "png"

    def load_preferences(self):
        if not os.path.exists(self.config_path):
            return
        try:
            with open(self.config_path, "r", encoding="utf-8") as config_file:
                config = json.load(config_file)
            self.default_output_format = self.normalize_output_format(config.get("last_output_format", "png"))
        except Exception:
            self.default_output_format = "png"

    def save_preferences(self):
        config = {"last_output_format": self.default_output_format}
        try:
            with open(self.config_path, "w", encoding="utf-8") as config_file:
                json.dump(config, config_file, indent=2)
        except Exception:
            pass
    def update_image(self):
        if self.original_image is not None:
            self.final_image = self.original_image.copy()
            
            if self.show_shadow.get():
                self.final_image = add_shadow(self.final_image)
            
            self.display_image = ImageTk.PhotoImage(self.final_image)

            # Update canvas with the new image
            self.canvas.delete("all")
            self.canvas.create_image(0, 0, anchor="nw", image=self.display_image)
            self.render_text_overlays()
            self.canvas.config(scrollregion=self.canvas.bbox(tk.ALL))

            # Resize the window to fit the final image
            window_width = self.final_image.width
            window_height = self.final_image.height
            self.geometry(f"{window_width}x{window_height}")

    def get_shadow_offset(self):
        return 20 if self.show_shadow.get() else 0

    def get_image_coordinates(self, canvas_x, canvas_y):
        offset = self.get_shadow_offset()
        return canvas_x - offset, canvas_y - offset

    def to_canvas_coordinates(self, image_x, image_y):
        offset = self.get_shadow_offset()
        return image_x + offset, image_y + offset

    def draw_text_on_image(self, target_image):
        draw = ImageDraw.Draw(target_image)
        offset = self.get_shadow_offset()
        for overlay in self.text_overlays:
            font = load_default_text_font(overlay["font_size"])
            draw.text(
                (overlay["x"] + offset, overlay["y"] + offset),
                overlay["text"],
                fill=overlay["color"],
                font=font
            )

    def build_export_image(self):
        if self.original_image is None:
            return None

        export_image = self.original_image.copy()
        if self.show_shadow.get():
            export_image = add_shadow(export_image)
        self.draw_text_on_image(export_image)
        return export_image

    def render_text_overlays(self):
        for overlay in self.text_overlays:
            canvas_x, canvas_y = self.to_canvas_coordinates(overlay["x"], overlay["y"])
            overlay["canvas_item"] = self.canvas.create_text(
                canvas_x,
                canvas_y,
                anchor="nw",
                text=overlay["text"],
                fill=overlay["color"],
                font=(overlay["font_family"], overlay["font_size"]),
                tags=("text_overlay", f"text_overlay_{overlay['id']}")
            )

    def get_overlay_by_canvas_item(self, canvas_item):
        for overlay in self.text_overlays:
            if overlay.get("canvas_item") == canvas_item:
                return overlay
        return None

    def get_overlay_at_event(self, event):
        canvas_x = self.canvas.canvasx(event.x)
        canvas_y = self.canvas.canvasy(event.y)
        for canvas_item in reversed(self.canvas.find_overlapping(canvas_x, canvas_y, canvas_x, canvas_y)):
            if "text_overlay" in self.canvas.gettags(canvas_item):
                return self.get_overlay_by_canvas_item(canvas_item)
        return None

    def add_text_overlay(self, image_x, image_y):
        text_value = self.show_multiline_text_dialog("Add Text", "Enter text:")
        if text_value is None:
            return

        if not text_value.strip():
            return

        self.save_state()
        self.text_overlays.append({
            "id": self.next_text_id,
            "x": image_x,
            "y": image_y,
            "text": text_value,
            "font_family": self.default_text_font,
            "font_size": self.default_text_size,
            "color": self.default_text_color
        })
        self.next_text_id += 1
        self.update_image()

    def edit_text_overlay(self, overlay):
        text_value = self.show_multiline_text_dialog("Edit Text", "Update text:", overlay["text"])
        if text_value is None:
            return

        if not text_value.strip() or text_value == overlay["text"]:
            return

        self.save_state()
        overlay["text"] = text_value
        self.update_image()

    def show_multiline_text_dialog(self, title, prompt, initial_text=""):
        dialog = MultilineTextDialog(self, title, prompt, initial_text)
        self.wait_window(dialog)
        return dialog.result

    def on_double_click(self, event):
        if self.original_image is None:
            return

        overlay = self.get_overlay_at_event(event)
        if overlay is not None:
            self.edit_text_overlay(overlay)

    def on_mouse_move(self, event):
        if self.original_image is None:
            self.canvas.configure(cursor="cross")
            return

        if self.dragging_text_id is not None:
            self.canvas.configure(cursor="fleur")
            return

        overlay = self.get_overlay_at_event(event)
        self.canvas.configure(cursor="fleur" if overlay is not None else "cross")

    def on_button_press(self, event):
        if self.original_image is None:
            return

        selected_overlay = self.get_overlay_at_event(event)
        if selected_overlay is not None:
            self.dragging_text_id = selected_overlay["id"]
            display_x, display_y = self.to_canvas_coordinates(selected_overlay["x"], selected_overlay["y"])
            self.drag_text_dx = self.canvas.canvasx(event.x) - display_x
            self.drag_text_dy = self.canvas.canvasy(event.y) - display_y
            self.drag_state_saved = False
            return

        if self.drawing_mode.get() == "text":
            image_x, image_y = self.get_image_coordinates(self.canvas.canvasx(event.x), self.canvas.canvasy(event.y))
            self.add_text_overlay(image_x, image_y)
            return

        if self.drawing_mode.get() == "color_picker":
            # Get coordinates relative to the image
            x = int(self.canvas.canvasx(event.x))
            y = int(self.canvas.canvasy(event.y))
            x, y = self.get_image_coordinates(x, y)
            x, y = int(x), int(y)
            
            # Get color at clicked position
            try:
                color = self.original_image.getpixel((x, y))
                if len(color) > 3:  # If RGBA, convert to RGB
                    color = color[:3]
                ColorInfoDialog(self, color, x, y)
            except IndexError:
                # Clicked outside image bounds
                pass
            return
            
        self.start_x = self.canvas.canvasx(event.x)
        self.start_y = self.canvas.canvasy(event.y)
        
        # Set colors based on drawing mode
        if self.drawing_mode.get() == "highlighter":
            outline_color = "yellow"
            fill_color = "yellow"
        else:  # redaction mode
            outline_color = "black"
            fill_color = "black"
            
        self.rect = self.canvas.create_rectangle(
            self.start_x, self.start_y, 
            self.start_x + 1, self.start_y + 1,
            outline=outline_color, fill=fill_color
        )

    def on_move_press(self, event):
        if self.dragging_text_id is not None:
            overlay = next((item for item in self.text_overlays if item["id"] == self.dragging_text_id), None)
            if overlay is None:
                return

            if not self.drag_state_saved:
                self.save_state()
                self.drag_state_saved = True

            curX, curY = self.canvas.canvasx(event.x), self.canvas.canvasy(event.y)
            new_canvas_x = curX - self.drag_text_dx
            new_canvas_y = curY - self.drag_text_dy
            image_x, image_y = self.get_image_coordinates(new_canvas_x, new_canvas_y)
            overlay["x"] = image_x
            overlay["y"] = image_y
            if overlay.get("canvas_item"):
                self.canvas.coords(overlay["canvas_item"], new_canvas_x, new_canvas_y)
            return

        curX, curY = self.canvas.canvasx(event.x), self.canvas.canvasy(event.y)
        if self.rect:
            x0, y0 = min(self.start_x, curX), min(self.start_y, curY)
            x1, y1 = max(self.start_x, curX), max(self.start_y, curY)
            self.canvas.coords(self.rect, x0, y0, x1, y1)
            
            if self.drawing_mode.get() == "highlighter":
                outline_color = "yellow"
                fill_color = "yellow"
            else:  # redaction mode
                outline_color = "black"
                fill_color = "black"
                
            self.canvas.itemconfig(self.rect, outline=outline_color, fill=fill_color, stipple="gray50")

    def on_button_release(self, event):
        if self.dragging_text_id is not None:
            self.dragging_text_id = None
            self.drag_state_saved = False
            self.update_image()
            return

        if self.rect and self.original_image is not None:
            # Save current state before making changes
            self.save_state()
            
            end_x, end_y = self.canvas.canvasx(event.x), self.canvas.canvasy(event.y)
            
            # Only subtract border offset if shadow is enabled
            offset = 20 if self.show_shadow.get() else 0
            
            # Determine the smallest and largest x and y coordinates
            x0, y0 = min(self.start_x, end_x) - offset, min(self.start_y, end_y) - offset
            x1, y1 = max(self.start_x, end_x) - offset, max(self.start_y, end_y) - offset

            overlay = Image.new('RGBA', self.original_image.size, (0, 0, 0, 0))
            draw = ImageDraw.Draw(overlay)

            if self.drawing_mode.get() == "highlighter":
                color = (255, 255, 0, 128)  # Yellow, 50% opacity
            else:  # redaction mode
                color = (0, 0, 0, 255)  # Black, 100% opacity
                
            draw.rectangle([x0, y0, x1, y1], fill=color)
            self.original_image = Image.alpha_composite(self.original_image.convert('RGBA'), overlay)
            self.update_image()

    def save_image(self):
        export_image = self.build_export_image()
        if export_image is not None:
            default_extension = ".png" if self.default_output_format == "png" else ".jpg"
            if self.default_output_format == "png":
                file_types = [("PNG files", "*.png"), ("JPEG files", "*.jpg *.jpeg")]
            else:
                file_types = [("JPEG files", "*.jpg *.jpeg"), ("PNG files", "*.png")]

            file_path = filedialog.asksaveasfilename(
                defaultextension=default_extension,
                filetypes=file_types
            )
            if file_path:
                extension = os.path.splitext(file_path)[1].lower()
                if extension in (".jpg", ".jpeg"):
                    selected_format = "jpg"
                elif extension == ".png":
                    selected_format = "png"
                else:
                    selected_format = self.default_output_format

                if selected_format == "jpg":
                    export_image.convert('RGB').save(file_path, "JPEG")
                else:
                    export_image.save(file_path, "PNG")

                self.default_output_format = selected_format
                self.save_preferences()

    def copy_image(self, event):
        export_image = self.build_export_image()
        if export_image is not None:
            copy_to_clipboard(export_image)

    def load_image(self, image_path):
        img = None
        if image_path is not None:
            img = Image.open(image_path)
        if img is None:
            img = get_image_from_clipboard()
        self.original_image = img
        self.text_overlays.clear()
        self.next_text_id = 1
    
    def load_image_from_clipboard(self):
        img = get_image_from_clipboard()
        if img is not None:
            # Clear undo/redo stacks when loading new image
            self.undo_stack.clear()
            self.redo_stack.clear()
            
            self.original_image = img
            self.text_overlays.clear()
            self.next_text_id = 1
            self.update_image()
        else:
            print("No image found in clipboard.")
            messagebox.showinfo("Screenshot Markup", "No image found in clipboard.")

    def save_state(self):
        """Save current state for undo"""
        if self.original_image:
            # Clear redo stack when new action is performed
            self.redo_stack.clear()
            
            # Save a copy of the current state
            self.undo_stack.append({
                "image": self.original_image.copy(),
                "text_overlays": [self.clone_text_overlay(item) for item in self.text_overlays],
                "next_text_id": self.next_text_id
            })
            
            # Limit stack size
            if len(self.undo_stack) > self.max_undos:
                self.undo_stack.pop(0)

    def undo(self, event):
        """Restore previous state"""
        if self.undo_stack and self.original_image:
            # Save current state to redo stack
            self.redo_stack.append({
                "image": self.original_image.copy(),
                "text_overlays": [self.clone_text_overlay(item) for item in self.text_overlays],
                "next_text_id": self.next_text_id
            })
            
            # Restore previous state
            previous_state = self.undo_stack.pop()
            self.original_image = previous_state["image"]
            self.text_overlays = [self.clone_text_overlay(item) for item in previous_state["text_overlays"]]
            self.next_text_id = previous_state["next_text_id"]
            self.update_image()

    def redo(self, event):
        """Restore previously undone state"""
        if self.redo_stack and self.original_image:
            # Save current state to undo stack
            self.undo_stack.append({
                "image": self.original_image.copy(),
                "text_overlays": [self.clone_text_overlay(item) for item in self.text_overlays],
                "next_text_id": self.next_text_id
            })
            
            # Restore previously undone state
            next_state = self.redo_stack.pop()
            self.original_image = next_state["image"]
            self.text_overlays = [self.clone_text_overlay(item) for item in next_state["text_overlays"]]
            self.next_text_id = next_state["next_text_id"]
            self.update_image()

    def clone_text_overlay(self, overlay):
        return {
            "id": overlay["id"],
            "x": overlay["x"],
            "y": overlay["y"],
            "text": overlay["text"],
            "font_family": overlay["font_family"],
            "font_size": overlay["font_size"],
            "color": overlay["color"]
        }

    def load_image_from_file(self):
        """Load an image from a file using a file dialog"""
        file_path = filedialog.askopenfilename(
            filetypes=[
                ("Image files", "*.png *.jpg *.jpeg *.gif *.bmp"),
                ("All files", "*.*")
            ]
        )
        if file_path:
            try:
                # Clear undo/redo stacks when loading new image
                self.undo_stack.clear()
                self.redo_stack.clear()
                
                # Load and display the image
                self.original_image = Image.open(file_path)
                self.text_overlays.clear()
                self.next_text_id = 1
                self.update_image()
            except Exception as e:
                messagebox.showerror("Error", f"Failed to load image: {str(e)}")

def load_default_text_font(size):
    for font_name in ("arial.ttf", "segoeui.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(font_name, size)
        except OSError:
            continue
    return ImageFont.load_default()

# Existing functions
def get_image_from_clipboard():
    data = ImageGrab.grabclipboard()
    if isinstance(data, Image.Image):
        return data
    if isinstance(data, list) and data:
        try:
            return Image.open(data[0])
        except (OSError, FileNotFoundError):
            return None
    return None


def add_shadow(image, offset=(13, 13), background_color='white', shadow_color='grey', border=20, blur_radius=8):
    # Ensure the image has an alpha channel
    if image.mode != 'RGBA':
        image = image.convert('RGBA')

    # Add 1px light grey border to the original image
    image = add_border(image, border=1, color='lightgrey')

    # Create an image for the shadow
    total_width = image.width + abs(offset[0]) + 2*border
    total_height = image.height + abs(offset[1]) + 2*border
    shadow = Image.new('RGBA', (total_width, total_height), background_color)

    # Place the shadow, with blur
    shadow_left = border + max(offset[0], 0)
    shadow_top = border + max(offset[1], 0)
    shadow.paste(shadow_color, [shadow_left, shadow_top, shadow_left + image.width, shadow_top + image.height])
    shadow = shadow.filter(ImageFilter.GaussianBlur(radius=blur_radius))

    # Paste the original image on top of the shadow
    image_left = border - min(offset[0], 0)
    image_top = border - min(offset[1], 0)
    shadow.paste(image, (image_left, image_top), image)

    return shadow

def add_border(image, border=1, color='lightgrey'):
    # Add a border of the given size and color
    image_with_border = ImageOps.expand(image, border=border, fill=color)
    return image_with_border

def copy_to_clipboard(image):
    if sys.platform.startswith("linux"):
        output = BytesIO()
        image.convert("RGB").save(output, "PNG")
        data = output.getvalue()
        output.close()

        if shutil.which("wl-copy"):
            command = ["wl-copy", "--type", "image/png"]
        elif shutil.which("xclip"):
            command = ["xclip", "-selection", "clipboard", "-t", "image/png"]
        else:
            messagebox.showinfo(
                "Screenshot Markup",
                "Install wl-copy or xclip to copy images to the clipboard.",
            )
            return

        try:
            subprocess.run(command, input=data, check=True)
        except subprocess.CalledProcessError as exc:
            messagebox.showerror(
                "Screenshot Markup",
                f"Failed to copy image to the clipboard: {exc}",
            )
        return

    messagebox.showinfo(
        "Screenshot Markup",
        "Copying images to the clipboard is only supported on Linux.",
    )

def main(image_path=None):
    app = ImageViewer(image_path)
    app.mainloop()

if __name__ == "__main__":
    if len(sys.argv) > 1:
        main(sys.argv[1])
    else:
        main()
