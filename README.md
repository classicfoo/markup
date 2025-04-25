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
- Windows OS
- Python 3.x
- Required packages:
  - Pillow
  - win32clipboard
  - [other dependencies...]

## Installation
