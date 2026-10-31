//! OxiDoc Viewer: native document viewer for Android written in Rust. No Java or Kotlin: the app is a NativeActivity + egui.

// The UI is Android-only; host builds use the parser modules for tests.
#![cfg_attr(not(target_os = "android"), allow(dead_code))]

mod doc;
mod docx;
mod load;
mod odt;
mod text;
mod xml;

#[cfg(test)]
mod smoke;

#[cfg(target_os = "android")]
mod platform;
#[cfg(target_os = "android")]
mod ui;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    let options = eframe::NativeOptions {
        android_app: Some(app.clone()),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "OxiDoc Viewer",
        options,
        Box::new(move |cc| Ok(Box::new(ui::ViewerApp::new(cc, app.clone())))),
    );
    // The event loop cannot be created twice in one process, so make sure the next launch (for
    // example "Open with" from a file manager) starts from a clean process.
    std::process::exit(0);
}
