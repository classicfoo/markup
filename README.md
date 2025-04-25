# Screenshot Markup Tool

A Windows desktop application for quickly marking up screenshots with highlighting, redaction, and color picking capabilities. Perfect for documentation, tutorials, or redacting sensitive information.

## Features

### Drawing Tools
- **Highlighter Tool**: Create semi-transparent yellow rectangles (50% opacity)
- **Redaction Tool**: Draw solid black rectangles (100% opacity)
- **Color Picker**: Get RGB and Hex values from any pixel

### Image Enhancement
- Optional drop shadow effect (toggleable)
- Customizable shadow properties
- Automatic border padding

### Edit Operations
- Undo/Redo support (up to 20 states)
- Multiple tool selection via context menu
- Precise cursor for accurate marking

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+V | Paste image from clipboard |
| Ctrl+C | Copy modified image to clipboard |
| Ctrl+S | Save image as JPEG |
| Ctrl+Z | Undo last action |
| Ctrl+Y | Redo last action |

## Usage

1. Copy a screenshot to your clipboard (e.g., using Windows+Shift+S)
2. Launch the application
3. Paste your image (Ctrl+V)
4. Right-click to select your desired tool
5. Click and drag to draw
6. Save (Ctrl+S) or copy (Ctrl+C) the result

## Requirements
- Windows 10 or later
- Python 3.8 or higher
- Required packages (automatically installed during setup):
  - Pillow >= 10.0.0 (Image processing)
  - pywin32 >= 306 (Windows clipboard operations)
  - pyperclip >= 1.8.2 (Cross-platform clipboard support)
  - tkinter (Included with Python)

## Installation

1. Make sure you have Python 3.8 or higher installed on your Windows computer
   - Download Python from [python.org](https://www.python.org/downloads/)
   - During installation, make sure to check "Add Python to PATH"

2. Download this tool:
   - Download the ZIP file of this project
   - Extract it to a folder of your choice

3. Open Command Prompt in the extracted folder:
   - Hold Shift + Right-click in the folder
   - Select "Open PowerShell window here" or "Open Command window here"

4. Install the required packages by typing:
   ```
   pip install -r requirements.txt
   ```

5. Double-click `markup.pyw` to run the application
   - You can also create a shortcut to `markup.pyw` on your desktop
   - To always run with Python, right-click → Open with → Python

## Development

### Project Structure
- `markup.pyw` - Main application file
- `requirements.txt` - Python dependencies
- `screenshot.jpg` - Sample screenshot showing the application in use

### Contributing
1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request

## License

This project is licensed under the GPLv3 License - see the LICENSE file for details.

## Acknowledgments

- Built with Python and Tkinter
- Uses Pillow for image processing
- Special thanks to all contributors
