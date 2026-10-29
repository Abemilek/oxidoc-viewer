//! Desktop binary: used to test the viewer locally with `cargo run`.
//! Usage: cargo run --release -- [path/to/file.docx | directory]
fn main() {
    #[cfg(not(target_os = "android"))]
    {
        let arg = std::env::args().nth(1);
        if let Err(e) = visor_docx::run_desktop(arg) {
            eprintln!("Error: {e}");
        }
    }
}
