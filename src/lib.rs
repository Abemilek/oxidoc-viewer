//! Android document viewer written in Rust. No Java or Kotlin: the app is a NativeActivity + egui.

// The UI is Android-only; host builds use the parser modules for tests.
#![cfg_attr(not(target_os = "android"), allow(dead_code))]

mod doc;
mod docx;
mod load;
mod odt;
mod text;
mod xml;

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
        "DOCX Viewer",
        options,
        Box::new(move |cc| Ok(Box::new(ui::ViewerApp::new(cc, app.clone())))),
    );
}

#[cfg(test)]
mod smoke {
    use crate::doc::Block;

    #[test]
    fn load_sample() {
        let d = crate::load::load(std::path::Path::new("samples/ejemplo.docx")).unwrap();
        assert!(!d.blocks.is_empty());
        assert!(!d.images.is_empty());
        for b in &d.blocks {
            if let Block::Para(p) = b {
                let text: String = p
                    .inlines
                    .iter()
                    .filter_map(|i| match i {
                        crate::doc::Inline::Text(r) => Some(r.text.as_str()),
                        _ => None,
                    })
                    .collect();
                eprintln!(
                    "heading={} label={:?} align={:?} :: {}",
                    p.heading,
                    p.label,
                    p.align,
                    text.chars().take(50).collect::<String>()
                );
            }
        }
    }
}
