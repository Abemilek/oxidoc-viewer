# Visor DOCX

[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)](https://www.rust-lang.org)
[![Platforms](https://img.shields.io/badge/platforms-Android%20%7C%20Linux-blue)](#requirements)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

A lightweight, native document viewer for Android and desktop, written entirely in Rust.

Visor DOCX opens Microsoft Word, OpenDocument, and plain-text files without requiring Microsoft
Office, LibreOffice, Java, or Kotlin. The DOCX and ODT parsers are implemented from scratch on top
of a small XML tokenizer and the `zip` crate, and the interface is rendered with
[egui](https://github.com/emilk/egui).

## Why Visor DOCX?

- **No office suite required** — documents are parsed and rendered by the application itself.
- **Native Android app** — runs as a `NativeActivity` that loads `libvisor_docx.so`; there is no
  JVM or Kotlin UI layer.
- **One codebase** — the same viewer runs on Android and Linux desktop.
- **Offline and private** — documents are processed locally and are never uploaded.
- **Read-only by design** — focused on viewing documents, with a simple file browser and zoomable
  rendering.

## Supported formats

| Format | Extensions | Support |
|---|---|---|
| Microsoft Word | `.docx` `.docm` `.dotx` `.dotm` | Paragraphs, bold/italic/underline/strikethrough, color, size, highlight, super/subscript, headings, bulleted and numbered lists, tables (including nested), images (PNG/JPEG/GIF/BMP), alignment, hyperlink styling, and page breaks |
| OpenDocument Text | `.odt` `.ott` | Basic styles, lists, tables, and images |
| Plain text | `.txt` `.md` | Full text; Markdown files also detect `#` headings |
| Legacy formats | `.doc`, `.rtf` | Not supported; the viewer displays a conversion hint |

## Features

- Rich text rendering with common inline formatting and colors.
- Heading detection and paragraph alignment.
- Bulleted and numbered lists.
- Tables, including nested tables.
- Embedded images with automatic scaling.
- File browser with quick-access folders.
- Zoom with on-screen buttons or pinch gestures.
- Touch-friendly scrolling with virtualized rendering for large documents.

## Requirements

### Desktop

- Rust stable, installed with [rustup](https://rustup.rs).
- Linux build dependencies:

  ```bash
  sudo apt update
  sudo apt install build-essential pkg-config libx11-dev libxkbcommon-dev \
                   libwayland-dev libegl1-mesa-dev libgl1-mesa-dev
  ```

- An OpenGL-capable graphics driver (the default renderer uses `glow`).

### Android

- Rust stable with the `aarch64-linux-android` target.
- JDK 17 or newer.
- Android SDK with platform 34, build-tools 34.0.0, and platform-tools.
- Android NDK 27.2.12479018 (or a newer NDK with the paths adjusted).
- [`cargo-apk`](https://github.com/rust-mobile/cargo-apk) for APK packaging.
- `adb` for installing on a device.
- Android 11 (API 30) or newer.

## Build and run

### Desktop

Clone the repository and run the sample document:

```bash
git clone https://github.com/Abemilek/visor-docx-rs.git
cd visor-docx-rs
cargo run --release -- samples/ejemplo.docx
```

You can also open a directory or another file:

```bash
cargo run --release -- /path/to/documents
cargo run --release -- /path/to/file.docx
```

With no argument, the viewer opens the home directory. To build the release binary:

```bash
cargo build --release
./target/release/visor-docx-desktop
```

### Android

Install the Rust target and packaging tool:

```bash
rustup target add aarch64-linux-android
cargo install cargo-apk
```

Configure the Android SDK and NDK. If you use the command-line tools, install the required
packages first:

```bash
sdkmanager "platforms;android-34" "build-tools;34.0.0" "platform-tools" "ndk;27.2.12479018"

export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_NDK_ROOT="$ANDROID_HOME/ndk/27.2.12479018"
```

Build and install a debug APK:

```bash
./build-android.sh
adb install -r "$(find target -name '*.apk' | head -n 1)"
```

Alternatively, build and launch directly on a connected device:

```bash
cargo apk run --lib
```

For a signed release build, create a keystore and provide its credentials:

```bash
keytool -genkeypair -v -keystore "$HOME/visor.keystore" \
        -alias visor -keyalg RSA -keysize 2048 -validity 10000

export CARGO_APK_RELEASE_KEYSTORE="$HOME/visor.keystore"
export CARGO_APK_RELEASE_KEYSTORE_PASSWORD="your-password"
./build-android.sh release
```

> `cargo-apk` is used for `NativeActivity` packaging. If it is unavailable in your environment,
> `xbuild` can be used as an alternative (`x build --platform android --arch arm64`).

## Usage

### Android

1. Launch **Visor DOCX**.
2. Grant **All files access** when prompted, then return to the app.
3. Browse the quick-access folders (for example, *Descargas* or *Documentos*), then tap a document.
4. Use the **−** and **+** buttons or pinch to zoom, and drag to scroll.

You can copy files to the device with `adb`:

```bash
adb push document.docx /sdcard/Download/
```

### Desktop

The desktop build uses the same file browser and viewer. Pass a file or directory on the command
line, or navigate with the built-in browser.

> The in-app interface is currently in Spanish.

## Project structure

```text
src/lib.rs       Entry points for Android (`android_main`) and desktop (`run_desktop`)
src/xml.rs       Minimal XML tokenizer
src/docx.rs      DOCX parser (styles, numbering, tables, images)
src/odt.rs       ODT parser
src/text.rs      Plain text and Markdown
src/load.rs      Format detection and parser dispatch
src/ui.rs        egui file browser and virtualized document viewer
src/platform.rs  Android JNI helpers for storage permission
samples/         Sample documents
build-android.sh Android APK build script
```

## Limitations

- Read-only: there is no editing, search, or printing.
- Headers, footers, footnotes, text boxes, complex graphics, and equations are not rendered.
  Floating images are displayed inline.
- Table columns have equal width, and merged cells are treated as a single cell.
- Justified text is displayed left-aligned; line and paragraph spacing are fixed.
- Font support is limited to Latin alphabets. Real bold text requires a system `*-Bold.ttf` font;
  otherwise a higher-contrast fallback is used.
- Android's "Open with" flow is not supported, since it requires reading `content://` URIs.
- `MANAGE_EXTERNAL_STORAGE` is intended for sideloaded APKs and is not accepted by Google Play for
  this type of viewer.

## Troubleshooting

- **Black screen on some devices:** switch the renderer from `glow` to `wgpu` in `Cargo.toml` and
  add `renderer: eframe::Renderer::Wgpu` to `NativeOptions` in `src/lib.rs`.
- **Storage permission not granted:** open Android Settings → Apps → Visor DOCX → Special access →
  All files access, enable it, and return to the app.

## Contributing

Issues and pull requests are welcome. Before submitting a change, run:

```bash
cargo fmt --check
cargo test
cargo check --all-targets
cargo check --target aarch64-linux-android --lib
```

Commit messages follow the [Conventional Commits](https://www.conventionalcommits.org/) format.

## License

Released under the MIT License. See [LICENSE](LICENSE) for details.
