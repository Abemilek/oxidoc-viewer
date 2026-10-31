# OxiDoc Viewer

[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Android-blue)](#requirements)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

A lightweight, native document viewer for Android, written entirely in Rust.

OxiDoc Viewer opens Microsoft Word, OpenDocument, and plain-text files without requiring Microsoft
Office, LibreOffice, Java, or Kotlin. The DOCX and ODT parsers are implemented from scratch on top
of a small XML tokenizer and the `zip` crate, and the interface is rendered with
[egui](https://github.com/emilk/egui).

## Why OxiDoc Viewer?

- **Native Android app** — runs as a `NativeActivity` that loads `liboxidoc_viewer.so`; there is no
  JVM or Kotlin UI layer.
- **No office suite required** — documents are parsed and rendered by the application itself.
- **Offline and private** — documents are processed locally and are never uploaded.
- **Read-only by design** — focused on viewing documents, with a simple file browser and zoomable
  rendering.
- **Touch-first interface** — pinch-to-zoom, drag-to-scroll, and large tap targets.

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

- Rust stable, installed with [rustup](https://rustup.rs), plus the
  `aarch64-linux-android` target.
- JDK 17 or newer.
- Android SDK with platform 34, build-tools 34.0.0, and platform-tools.
- Android NDK 27.2.12479018 (or a newer NDK with the paths adjusted).
- [`cargo-apk`](https://github.com/rust-mobile/cargo-apk) for APK packaging.
- `adb` for installing on a device.
- Android 11 (API 30) or newer.

## Build and run

Install the Rust target and packaging tool:

```bash
rustup target add aarch64-linux-android
cargo install cargo-apk
```

Configure the Android SDK and NDK. If you use the command-line tools, install the required
packages first (adjust the paths to your SDK location, for example
`C:\Users\<user>\Android\Sdk` on Windows or `~/Library/Android/sdk` on macOS):

```bash
sdkmanager "platforms;android-34" "build-tools;34.0.0" "platform-tools" "ndk;27.2.12479018"

export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_NDK_ROOT="$HOME/Android/Sdk/ndk/27.2.12479018"
```

The build steps are the same on Windows, Linux and macOS (on Windows run the shell script from
WSL or Git Bash and point `ANDROID_HOME` to your SDK path). Only the SDK paths change per
operating system.

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
keytool -genkeypair -v -keystore "$HOME/oxidoc-viewer-release.keystore" \
        -alias oxidoc -keyalg RSA -keysize 2048 -validity 10000

export CARGO_APK_RELEASE_KEYSTORE="$HOME/oxidoc-viewer-release.keystore"
export CARGO_APK_RELEASE_KEYSTORE_PASSWORD="oxidoc"
./build-android.sh release
```

This produces signed APKs in `target/release/apk/` (for example `OxiDocViewer.apk`).

The release APK for the current setup is kept in the `apks/` folder of the repository: a single
universal APK that embeds both the `arm64-v8a` and the `armeabi-v7a` native libraries, so it runs on
both new and old phones:

- `apks/OxiDocViewer-universal-release.apk` — for new and old phones (arm64-v8a and armeabi-v7a).

> `cargo-apk` is used for `NativeActivity` packaging. If it is unavailable in your environment,
> `xbuild` can be used as an alternative (`x build --platform android --arch arm64`).

## Usage

1. Launch **OxiDoc Viewer**.
2. Grant **All files access** when prompted, then return to the app.
3. Browse the quick-access folders (*Downloads*, *Documents*, or *Storage*) and tap a document.
4. Use the **−** and **+** buttons or pinch to zoom, and drag to scroll.

You can copy files to the device with `adb`:

```bash
adb push document.docx /sdcard/Download/
```

## Project structure

```text
src/lib.rs       Android entry point (`android_main`) and module wiring
src/xml.rs       Minimal XML tokenizer
src/docx.rs      DOCX parser (styles, numbering, tables, images)
src/odt.rs       ODT parser
src/text.rs      Plain text and Markdown
src/load.rs      Format detection and parser dispatch
src/ui.rs        egui file browser and virtualized document viewer
src/platform.rs  Android JNI helpers for storage permission
samples/         Sample document used by the parser smoke test
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
- **Storage permission not granted:** open Android Settings → Apps → OxiDoc Viewer → Special access →
  All files access, enable it, and return to the app.

## Contributing

Issues and pull requests are welcome. Before submitting a change, run:

```bash
cargo fmt --check
cargo test
cargo check --target aarch64-linux-android --lib
```

`cargo test` runs the parser smoke test on the host; the UI and JNI layers are compiled for Android.

Commit messages follow the [Conventional Commits](https://www.conventionalcommits.org/) format.

## License

Released under the MIT License. See [LICENSE](LICENSE) for details.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release notes.
