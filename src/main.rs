//! Binario de escritorio: sirve para probar el visor en la laptop con `cargo run`.
//! Uso: cargo run --release -- [ruta/al/archivo.docx | carpeta]
fn main() {
    #[cfg(not(target_os = "android"))]
    {
        let arg = std::env::args().nth(1);
        if let Err(e) = visor_docx::run_desktop(arg) {
            eprintln!("Error: {e}");
        }
    }
}
