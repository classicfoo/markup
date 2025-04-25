import tkinter as tk
from tkinter import ttk
from PIL import Image, ImageTk, ImageDraw, ImageFilter, ImageOps
import sys
import win32clipboard
from io import BytesIO
from tkinter import filedialog, messagebox, colorchooser
import pyperclip  # You'll need to pip install pyperclip

class ColorInfoDialog(tk.Toplevel):
    def __init__(self, parent, color_rgb, x, y):
        super().__init__(parent)
        self.title("Color Information")
        
        # Make dialog modal
        self.transient(parent)
        self.grab_set()
        
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
        self.canvas.bind("<Button-3>", self.show_context_menu)  # Right-click

        # Keyboard shortcuts
        self.bind("<Control-v>", lambda event: self.load_image_from_clipboard())
        self.bind("<Control-c>", self.copy_image)
        self.bind("<Control-s>", lambda event: self.save_image())
        self.bind("<Control-z>", self.undo)
        self.bind("<Control-y>", self.redo)

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
        
        self.context_menu.add_cascade(
            label="Tools",  # Changed from "Drawing Mode"
            menu=self.tools_submenu
        )
        
        # Add separator
        self.context_menu.add_separator()
        
        # Shadow toggle
        self.context_menu.add_checkbutton(
            label="Shadow",
            variable=self.show_shadow,
            command=self.update_image
        )

    def show_context_menu(self, event):
        try:
            self.context_menu.tk_popup(event.x_root, event.y_root)
        finally:
            self.context_menu.grab_release()

    def update_image(self):
        if self.original_image is not None:
            self.final_image = self.original_image.copy()
            
            if self.show_shadow.get():
                self.final_image = add_shadow(self.final_image)
            
            self.display_image = ImageTk.PhotoImage(self.final_image)

            # Update canvas with the new image
            self.canvas.delete("all")
            self.canvas.create_image(0, 0, anchor="nw", image=self.display_image)
            self.canvas.config(scrollregion=self.canvas.bbox(tk.ALL))

            # Resize the window to fit the final image
            window_width = self.final_image.width
            window_height = self.final_image.height
            self.geometry(f"{window_width}x{window_height}")

    def on_button_press(self, event):
        if self.drawing_mode.get() == "color_picker":
            if self.original_image is not None:
                # Get coordinates relative to the image
                x = int(self.canvas.canvasx(event.x))
                y = int(self.canvas.canvasy(event.y))
                
                # Adjust coordinates if shadow is enabled
                offset = 20 if self.show_shadow.get() else 0
                x -= offset
                y -= offset
                
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
        if self.final_image is not None:
            file_path = filedialog.asksaveasfilename(defaultextension=".jpg", filetypes=[("JPEG files", "*.jpg")])
            if file_path:
                # Convert the image to RGB mode before saving
                self.final_image.convert('RGB').save(file_path, "JPEG")

    def copy_image(self, event):
        if self.final_image is not None:
            copy_to_clipboard(self.final_image)

    def load_image(self, image_path):
        # Try to get image from clipboard
        img = get_image_from_clipboard()
        if img is None and image_path is not None:
            # Load the image from file
            img = Image.open(image_path)
        self.original_image = img
    
    def load_image_from_clipboard(self):
        img = get_image_from_clipboard()
        if img is not None:
            # Clear undo/redo stacks when loading new image
            self.undo_stack.clear()
            self.redo_stack.clear()
            
            self.original_image = img
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
            self.undo_stack.append(self.original_image.copy())
            
            # Limit stack size
            if len(self.undo_stack) > self.max_undos:
                self.undo_stack.pop(0)

    def undo(self, event):
        """Restore previous state"""
        if self.undo_stack and self.original_image:
            # Save current state to redo stack
            self.redo_stack.append(self.original_image.copy())
            
            # Restore previous state
            self.original_image = self.undo_stack.pop()
            self.update_image()

    def redo(self, event):
        """Restore previously undone state"""
        if self.redo_stack and self.original_image:
            # Save current state to undo stack
            self.undo_stack.append(self.original_image.copy())
            
            # Restore previously undone state
            self.original_image = self.redo_stack.pop()
            self.update_image()

# Existing functions
def get_image_from_clipboard():
    win32clipboard.OpenClipboard()
    try:
        # Check if the clipboard contains an image format
        if win32clipboard.IsClipboardFormatAvailable(win32clipboard.CF_DIB):
            data = win32clipboard.GetClipboardData(win32clipboard.CF_DIB)
            image = Image.open(BytesIO(data))
            return image
        else:
            print("No image in clipboard")
            return None
    finally:
        win32clipboard.CloseClipboard()


def add_shadow(image, offset=(13, 13), background_color='white', shadow_color='grey', border=20, blur_radius=8):
    # Ensure the image has an alpha channel
    if image.mode != 'RGBA':
        image = image.convert('RGBA')

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
    output = BytesIO()
    image.convert('RGB').save(output, 'BMP')
    data = output.getvalue()[14:]  # Remove the 14-byte BMP header
    output.close()

    win32clipboard.OpenClipboard()
    win32clipboard.EmptyClipboard()
    win32clipboard.SetClipboardData(win32clipboard.CF_DIB, data)
    win32clipboard.CloseClipboard()

def main(image_path=None):
    app = ImageViewer(image_path)
    app.mainloop()

if __name__ == "__main__":
    if len(sys.argv) > 1:
        main(sys.argv[1])
    else:
        main()
