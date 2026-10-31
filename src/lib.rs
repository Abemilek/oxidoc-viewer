//! OxiDoc Viewer: native document viewer for Android written in Rust. No Java or Kotlin: the app is a NativeActivity + egui.

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
        "OxiDoc Viewer",
        options,
        Box::new(move |cc| Ok(Box::new(ui::ViewerApp::new(cc, app.clone())))),
    );
}

#[cfg(test)]
mod smoke {
    use crate::doc::Block;

    #[test]
    fn load_sample() {
        let d = crate::load::load(std::path::Path::new("samples/sample.docx")).unwrap();
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

    #[test]
    fn offline_no_network_or_telemetry() {
        // Offline-only rule: the app must not declare the INTERNET permission and must not
        // use any network or telemetry/analytics API.
        let manifest = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        )
        .expect("failed to read Cargo.toml");
        assert!(
            !manifest.contains("android.permission.INTERNET"),
            "android.permission.INTERNET must not be declared"
        );

        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let entries = std::fs::read_dir(&src_dir).expect("failed to read src directory");
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // Skip the test module itself, which references the forbidden API names.
            if path.extension().map(|e| e == "rs").unwrap_or(false) && file_name != "lib.rs" {
                let src = std::fs::read_to_string(&path).expect("failed to read source file");
                for forbidden in [
                    "TcpStream",
                    "UdpSocket",
                    "reqwest",
                    "socket",
                    "fetch",
                    "telemetry",
                    "analytics",
                    "firebase",
                    "crashlytics",
                ] {
                    assert!(
                        !src.contains(forbidden),
                        "forbidden API '{}' found in {}",
                        forbidden,
                        path.display()
                    );
                }
            }
        }
    }
}
