//! Carga de archivos: detecta el formato y delega al parser correspondiente.
use crate::doc::Document;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

const MAX_FILE: u64 = 150 * 1024 * 1024;
const MAX_ENTRY: u64 = 200 * 1024 * 1024;

pub fn read_bytes<R: Read + Seek>(z: &mut zip::ZipArchive<R>, name: &str) -> Option<Vec<u8>> {
    let f = z.by_name(name).ok()?;
    let mut buf = Vec::new();
    f.take(MAX_ENTRY).read_to_end(&mut buf).ok()?;
    Some(buf)
}

pub fn read_str<R: Read + Seek>(z: &mut zip::ZipArchive<R>, name: &str) -> Option<String> {
    let b = read_bytes(z, name)?;
    Some(String::from_utf8_lossy(&b).into_owned())
}

pub fn is_supported_name(name: &str) -> bool {
    let ext = ext_of(name);
    matches!(
        ext.as_str(),
        "docx" | "docm" | "dotx" | "dotm" | "odt" | "ott" | "txt" | "md" | "doc" | "rtf"
    )
}

fn ext_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

pub fn load(path: &Path) -> Result<Document, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("No se puede leer el archivo: {e}"))?;
    if meta.len() > MAX_FILE {
        return Err("El archivo es demasiado grande (máx. 150 MB).".into());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("No se puede leer el archivo: {e}"))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = ext_of(&name);

    if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
        return Err("Este es un .doc binario antiguo (OLE). Aún no está soportado: guárdalo como .docx (p. ej. con LibreOffice o Word) y vuelve a abrirlo.".into());
    }
    if bytes.starts_with(b"{\\rtf") {
        return Err("El formato RTF todavía no está soportado. Guárdalo como .docx u .odt.".into());
    }

    let mut doc = if bytes.starts_with(b"PK") {
        let mut z = zip::ZipArchive::new(Cursor::new(&bytes[..]))
            .map_err(|e| format!("Archivo ZIP/Office inválido: {e}"))?;
        let is_docx = z.by_name("word/document.xml").is_ok();
        let is_odt = z.by_name("content.xml").is_ok();
        drop(z);
        if is_docx {
            crate::docx::parse(&bytes)?
        } else if is_odt {
            crate::odt::parse(&bytes)?
        } else {
            return Err("El ZIP no contiene un documento Word (.docx) ni OpenDocument (.odt).".into());
        }
    } else if matches!(ext.as_str(), "txt" | "md" | "log" | "csv") || std::str::from_utf8(&bytes).is_ok() {
        crate::text::parse(&bytes, ext == "md")
    } else {
        return Err("Formato no reconocido.".into());
    };
    doc.title = name;
    Ok(doc)
}
