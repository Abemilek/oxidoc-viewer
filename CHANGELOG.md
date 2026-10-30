# Changelog

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
