# Changelog

## v3

### Added

- **Open with.** The app is offered in the Android "Open with" dialog (Just once / Always) for
  `.docx`, `.dotx`, `.docm`, `.odt`, `.ott`, `.txt` and `.md` files coming from file managers, chat
  apps and other apps. The file is read through the `ContentResolver`, so it works even without the
  "All files access" permission.
- Back gesture: viewer -> file browser -> parent folder -> close the app (when the system
  delivers it to the app).

### Changed

- **Always pure-black (AMOLED) dark UI** in the file browser, independent of the system theme.
  The document stays white.
- **Viewer without a fixed toolbar.** The Back / - / + buttons are gone: zoom is done by pinching,
  and a tap shows a floating bar (Back, file name, zoom %, tap the % to reset) that hides itself
  after a few seconds. The whole screen is now used by the document.
- The status bar is always readable: the app uses a dark system theme (white icons) and keeps a
  black strip under the status bar.
- The process now exits when the activity closes, so a later "Open with" always starts cleanly.

### Notes

- If the app is already open in the background, Android hands a new "Open with" file to the running
  instance, and a pure-Rust `NativeActivity` cannot receive it (`onNewIntent` needs a Java
  subclass). Close the app first (Back from the file browser) and open the file again.
- File managers that report `application/octet-stream` for `.docx` files will not list the app;
  open the file from the app's own browser in that case.
- Run `cargo fmt` once after pulling this version, because CI runs `cargo fmt --check`.

## v2 - OxiDoc Viewer

### Added

- New launcher icon for the app: adaptive icon plus legacy PNGs in `res/`, with a 512 px copy in
  `assets/icon.png`.

### Fixed

- Tables now render as real tables: shared borders, column widths taken from the document
  (`w:tblGrid`), merged cells (`gridSpan` / `number-columns-spanned`) and equal row heights. The
  stretched text inside narrow cells is fixed (egui was justifying the text).
- Status bar overlap fixed: the top bar and the file browser now leave room for the system status
  bar (height queried through JNI).
- Extra space at the bottom of lists/documents so the last items are not hidden by the navigation
  bar.

### Changed

- Renamed the app to **OxiDoc Viewer** (`dev.oxidoc.viewer`, `liboxidoc_viewer.so`) and the in-app
  title no longer says "Document Viewer".

### Notes

- After pulling this version run `cargo fmt` once, because CI runs `cargo fmt --check`.
