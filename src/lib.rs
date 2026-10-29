//! Document viewer written in Rust. No Java or Kotlin: the app is a NativeActivity + egui.

mod doc;
mod docx;
mod load;
mod odt;
mod platform;
mod text;
mod ui;
mod xml;

#[cfg(not(target_os = "android"))]
pub fn run_desktop(arg: Option<String>) -> eframe::Result<()> {
    let start = arg.map(std::path::PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([480.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Visor DOCX",
        options,
        Box::new(move |cc| Ok(Box::new(ui::ViewerApp::new(cc, (), start.clone())))),
    )
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    let options = eframe::NativeOptions {
        android_app: Some(app.clone()),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Visor DOCX",
        options,
        Box::new(move |cc| Ok(Box::new(ui::ViewerApp::new(cc, app.clone(), None)))),
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
