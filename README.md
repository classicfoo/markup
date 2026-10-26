# Screenshot Markup

A Rust and egui desktop app for annotating screenshots and other images. Windows downloads are published on the [GitHub Releases page](https://github.com/classicfoo/markup/releases).

## Features

- Paste screenshots from the clipboard, open files, or pass an image path when launching.
- Draw translucent yellow highlights and solid black redactions.
- Pick a pixel to view and copy its position, RGB value, and hex value.
- Add text overlays, move them by dragging, and edit them by double-clicking.
- Select a region for OCR with Tesseract, edit the result, transform selected text's case, and copy it.
- Undo and redo up to 20 edits.
- Save as PNG or JPEG, or copy the composed image to the clipboard.
- Toggle the drop shadow and adjust its offsets, blur, padding, shadow color, and background color.
- Open additional windows from the image's right-click menu.
- Remember the last image output format and shadow settings in the user's config directory.
- Drop an image file onto the app to open it.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| Ctrl+V | Paste an image from the clipboard |
| Ctrl+C | Copy the rendered image |
| Ctrl+S | Save the image as PNG or JPEG |
| Ctrl+L | Open an image file |
| Ctrl+N | Open another app window |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |

Right-click the image to switch tools, open another window, or change shadow settings. Scroll when the image is larger than the window.

## OCR setup

OCR uses the Tesseract executable installed on the computer. Install Tesseract and make `tesseract` available on `PATH`, or set `TESSERACT_CMD` to its full path. The app remains usable when Tesseract is not installed; only OCR is unavailable.

## Build from source

Install the stable Rust toolchain, then run:

```sh
cargo run --release
```

Run the automated checks with:

```sh
cargo test --all-targets
```

To open an image at startup:

```sh
cargo run --release -- path/to/screenshot.png
```

## License

This application is licensed under GPL-3.0; see [LICENSE](LICENSE). A DejaVu Sans font is bundled as a fallback for exported text and retains the license in [assets/DejaVu-LICENSE.txt](assets/DejaVu-LICENSE.txt).
