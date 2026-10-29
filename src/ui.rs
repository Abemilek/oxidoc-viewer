//! Interfaz (egui): explorador de archivos + visor de documentos con scroll virtualizado.
use crate::doc::*;
use crate::load;
use crate::platform::{self, AppHandle};
use eframe::egui::{
    self, text::LayoutJob, vec2, Align, Color32, FontFamily, FontId, Stroke, TextFormat, TextureHandle,
    TextureOptions,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const TEXT: Color32 = Color32::from_rgb(25, 25, 25);
/// puntos tipográficos del documento -> puntos lógicos de egui
const PT_SCALE: f32 = 1.25;

struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

struct Open {
    doc: Document,
    name: String,
    textures: RefCell<HashMap<String, Option<TextureHandle>>>,
    heights: Vec<f32>,
    layout_key: (i32, i32),
}

pub struct ViewerApp {
    handle: AppHandle,
    dir: PathBuf,
    entries: Vec<Entry>,
    dir_error: Option<String>,
    quick: Vec<(String, PathBuf)>,
    open: Option<Open>,
    message: Option<String>,
    zoom: f32,
    has_bold: bool,
    access_ok: bool,
    next_check: f64,
}

/// Busca una fuente "negrita" del sistema (Android: /system/fonts, Linux: DejaVu/Liberation/Noto).
fn setup_fonts(ctx: &egui::Context) -> bool {
    let candidates = [
        "/system/fonts/NotoSerif-Bold.ttf",
        "/system/fonts/DroidSans-Bold.ttf",
        "/system/fonts/Roboto-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
    ];
    for p in candidates {
        if let Ok(bytes) = std::fs::read(p) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("bold".to_owned(), egui::FontData::from_owned(bytes).into());
            let mut list = vec!["bold".to_owned()];
            list.extend(fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
            fonts.families.insert(FontFamily::Name("bold".into()), list);
            ctx.set_fonts(fonts);
            return true;
        }
    }
    false
}

impl ViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, handle: AppHandle, start: Option<PathBuf>) -> Self {
        let ctx = &cc.egui_ctx;
        ctx.set_visuals(egui::Visuals::light());
        ctx.style_mut(|s| {
            s.spacing.button_padding = vec2(12.0, 10.0);
            s.spacing.item_spacing = vec2(8.0, 8.0);
            s.interaction.selectable_labels = false; // para que arrastrar = scroll en pantallas táctiles
        });
        let has_bold = setup_fonts(ctx);
        let quick = platform::quick_dirs(&handle);
        let dir = quick.first().map(|q| q.1.clone()).unwrap_or_else(|| PathBuf::from("/"));
        let access_ok = platform::has_access(&handle);
        let mut app = ViewerApp {
            handle,
            dir,
            entries: Vec::new(),
            dir_error: None,
            quick,
            open: None,
            message: None,
            zoom: 1.0,
            has_bold,
            access_ok,
            next_check: 0.0,
        };
        if let Some(p) = start {
            if p.is_dir() {
                app.dir = p;
            } else {
                if let Some(parent) = p.parent() {
                    if !parent.as_os_str().is_empty() {
                        app.dir = parent.to_path_buf();
                    }
                }
                app.open_file(&p);
            }
        }
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        self.entries.clear();
        self.dir_error = None;
        match std::fs::read_dir(&self.dir) {
            Ok(rd) => {
                for e in rd.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') {
                        continue;
                    }
                    let path = e.path();
                    let is_dir = std::fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false);
                    if !is_dir && !load::is_supported_name(&name) {
                        continue;
                    }
                    self.entries.push(Entry { name, path, is_dir });
                }
                self.entries.sort_by(|a, b| {
                    b.is_dir
                        .cmp(&a.is_dir)
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
            }
            Err(e) => {
                self.dir_error = Some(format!("No se puede abrir la carpeta: {e}"));
            }
        }
    }

    fn open_file(&mut self, path: &Path) {
        match load::load(path) {
            Ok(doc) => {
                let n = doc.blocks.len();
                let name = if doc.title.is_empty() { path.display().to_string() } else { doc.title.clone() };
                self.open = Some(Open {
                    doc,
                    name,
                    textures: RefCell::new(HashMap::new()),
                    heights: vec![0.0; n],
                    layout_key: (0, 0),
                });
                self.message = None;
            }
            Err(e) => self.message = Some(e),
        }
    }

    fn browser_ui(&mut self, ctx: &egui::Context) {
        let mut nav: Option<PathBuf> = None;
        let mut open: Option<PathBuf> = None;
        let mut up = false;
        let mut ask = false;
        let mut recheck = false;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading("Visor de documentos");
            ui.label(egui::RichText::new(self.dir.display().to_string()).small());
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for (label, path) in &self.quick {
                    if ui.button(label.as_str()).clicked() {
                        nav = Some(path.clone());
                    }
                }
            });
            if !self.access_ok {
                ui.colored_label(
                    Color32::from_rgb(180, 90, 0),
                    "Para ver tus archivos hace falta el permiso «Acceso a todos los archivos».",
                );
                ui.horizontal(|ui| {
                    if ui.button("Conceder acceso").clicked() {
                        ask = true;
                    }
                    if ui.button("Ya lo concedí").clicked() {
                        recheck = true;
                    }
                });
            }
            if let Some(m) = &self.message {
                ui.colored_label(Color32::from_rgb(190, 40, 40), m.as_str());
            }
            if ui.button("⬆ Carpeta superior").clicked() {
                up = true;
            }
            ui.separator();
            if let Some(e) = &self.dir_error {
                ui.label(e.as_str());
            }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for e in &self.entries {
                    let icon = if e.is_dir { "📁" } else { "📄" };
                    let w = ui.available_width();
                    let b = egui::Button::new(format!("{icon}  {}", e.name)).min_size(vec2(w, 44.0));
                    if ui.add(b).clicked() {
                        if e.is_dir {
                            nav = Some(e.path.clone());
                        } else {
                            open = Some(e.path.clone());
                        }
                    }
                }
                if self.entries.is_empty() && self.dir_error.is_none() {
                    ui.label("(carpeta vacía o sin documentos compatibles)");
                }
            });
        });

        if ask {
            platform::request_access(&self.handle);
        }
        if recheck {
            self.access_ok = platform::has_access(&self.handle);
            self.refresh();
        }
        if up {
            if let Some(p) = self.dir.parent() {
                nav = Some(p.to_path_buf());
            }
        }
        if let Some(p) = nav {
            self.dir = p;
            self.message = None;
            self.refresh();
        }
        if let Some(p) = open {
            self.open_file(&p);
        }
    }

    fn viewer_ui(&mut self, ctx: &egui::Context) {
        let mut close = false;
        let mut dz = ctx.input(|i| i.zoom_delta());
        let name = self.open.as_ref().map(|o| o.name.clone()).unwrap_or_default();
        let zoom_now = self.zoom;

        egui::TopBottomPanel::top("bar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("◀ Volver").clicked() {
                    close = true;
                }
                if ui.button("−").clicked() {
                    dz /= 1.15;
                }
                if ui.button("+").clicked() {
                    dz *= 1.15;
                }
                ui.label(format!("{}%", (zoom_now * 100.0).round() as i32));
            });
            ui.label(egui::RichText::new(&name).small());
            ui.add_space(4.0);
        });

        if (dz - 1.0).abs() > 0.001 {
            self.zoom = (self.zoom * dz).clamp(0.4, 4.0);
        }
        let zoom = self.zoom;
        let has_bold = self.has_bold;

        let frame = egui::Frame::default().fill(Color32::WHITE).inner_margin(10.0);
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            if let Some(o) = &mut self.open {
                let Open { doc, textures, heights, layout_key, .. } = o;
                let w = ui.available_width();
                let key = (w as i32, (zoom * 100.0) as i32);
                if *layout_key != key || heights.len() != doc.blocks.len() {
                    heights.clear();
                    heights.resize(doc.blocks.len(), 0.0);
                    *layout_key = key;
                }
                let rc = RCtx { images: &doc.images, textures: &*textures, scale: zoom * PT_SCALE, has_bold };
                egui::ScrollArea::vertical().id_salt(("visor", name.as_str())).auto_shrink([false, false]).show_viewport(ui, |ui, viewport| {
                    ui.spacing_mut().item_spacing.y = (6.0 * zoom).clamp(3.0, 18.0);
                    let sp = ui.spacing().item_spacing.y;
                    let avail = ui.available_width();
                    let top = viewport.min.y - 800.0;
                    let bot = viewport.max.y + 800.0;
                    let mut y = 0.0f32;
                    for (i, b) in doc.blocks.iter().enumerate() {
                        let h = heights[i];
                        let known = h > 0.0;
                        let render = if known { y + h >= top && y <= bot } else { y <= bot };
                        if render {
                            let r = ui.scope(|ui| render_block(ui, b, &rc));
                            let hh = r.response.rect.height().max(0.5);
                            heights[i] = hh;
                            y += hh + sp;
                        } else {
                            let hh = if known { h } else { estimate(b, &rc) };
                            ui.allocate_space(vec2(avail, hh));
                            y += hh + sp;
                        }
                    }
                });
            }
        });

        if close {
            self.open = None;
        }
    }
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = ctx.input(|i| i.time);
        if now >= self.next_check {
            let was = self.access_ok;
            self.access_ok = platform::has_access(&self.handle);
            self.next_check = now + 1.5;
            if !was && self.access_ok {
                self.refresh();
            }
        }
        if !self.access_ok {
            ctx.request_repaint_after(std::time::Duration::from_millis(1500));
        }
        if self.open.is_some() {
            self.viewer_ui(ctx);
        } else {
            self.browser_ui(ctx);
        }
    }
}

// ------------------------------------------------------------------ render

struct RCtx<'a> {
    images: &'a HashMap<String, Vec<u8>>,
    textures: &'a RefCell<HashMap<String, Option<TextureHandle>>>,
    scale: f32,
    has_bold: bool,
}

fn estimate(b: &Block, rc: &RCtx) -> f32 {
    match b {
        Block::Para(p) => {
            let chars: usize = p
                .inlines
                .iter()
                .map(|i| match i {
                    Inline::Text(r) => r.text.chars().count(),
                    Inline::Image { .. } => 600,
                    Inline::PageBreak => 0,
                })
                .sum();
            let pt = if p.default_pt > 0.0 { p.default_pt } else { 11.0 };
            let line = pt * rc.scale * 1.35;
            ((chars / 55) + 1) as f32 * line
        }
        Block::Table(t) => t.rows.len().max(1) as f32 * 50.0 * rc.scale,
    }
}

fn render_block(ui: &mut egui::Ui, b: &Block, rc: &RCtx) {
    match b {
        Block::Para(p) => render_para(ui, p, rc),
        Block::Table(t) => render_table(ui, t, rc),
    }
}

fn render_table(ui: &mut egui::Ui, t: &Table, rc: &RCtx) {
    for row in &t.rows {
        let n = row.len();
        if n == 0 {
            continue;
        }
        ui.columns(n, |cols| {
            for (i, cell) in row.iter().enumerate() {
                let col = &mut cols[i];
                egui::Frame::group(col.style()).show(col, |ui| {
                    ui.set_width(ui.available_width());
                    for b in &cell.blocks {
                        render_block(ui, b, rc);
                    }
                });
            }
        });
    }
}

fn render_para(ui: &mut egui::Ui, p: &Para, rc: &RCtx) {
    let indent = p.indent_pt * rc.scale;
    if indent > 1.0 {
        ui.horizontal_top(|ui| {
            ui.add_space(indent);
            ui.vertical(|ui| para_inner(ui, p, rc));
        });
    } else {
        para_inner(ui, p, rc);
    }
}

fn para_inner(ui: &mut egui::Ui, p: &Para, rc: &RCtx) {
    let base_pt = if p.default_pt > 0.0 { p.default_pt } else { 11.0 };
    let mut job = LayoutJob::default();
    let mut any = false;
    if let Some(l) = &p.label {
        job.append(
            &format!("{l}  "),
            0.0,
            TextFormat {
                font_id: FontId::proportional(base_pt * rc.scale),
                color: TEXT,
                ..Default::default()
            },
        );
        any = true;
    }
    for inl in &p.inlines {
        match inl {
            Inline::Text(r) => {
                job.append(&r.text, 0.0, run_format(r, rc));
                any = true;
            }
            Inline::Image { key, w_pt, h_pt } => {
                flush(ui, &mut job, p.align);
                draw_image(ui, rc, key, *w_pt, *h_pt);
                any = true;
            }
            Inline::PageBreak => {
                flush(ui, &mut job, p.align);
                ui.separator();
                any = true;
            }
        }
    }
    flush(ui, &mut job, p.align);
    if !any {
        ui.add_space(base_pt * rc.scale);
    }
}

fn flush(ui: &mut egui::Ui, job: &mut LayoutJob, align: Alignment) {
    if job.text.is_empty() {
        return;
    }
    let mut j = std::mem::take(job);
    j.wrap.max_width = ui.available_width();
    match align {
        Alignment::Left => {
            ui.label(j);
        }
        Alignment::Center => {
            ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
                ui.label(j);
            });
        }
        Alignment::Right => {
            ui.with_layout(egui::Layout::top_down(Align::Max), |ui| {
                ui.label(j);
            });
        }
    }
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn run_format(r: &Run, rc: &RCtx) -> TextFormat {
    let shrink = if r.sup || r.sub { 0.7 } else { 1.0 };
    let size = (r.size_pt * rc.scale * shrink).clamp(5.0, 200.0);
    let family = if r.bold && rc.has_bold {
        FontFamily::Name("bold".into())
    } else {
        FontFamily::Proportional
    };
    let mut color = r.color.map(rgb).unwrap_or(TEXT);
    if r.bold && !rc.has_bold && color == TEXT {
        color = Color32::BLACK; // sin fuente negrita del sistema: al menos más contraste
    }
    let mut f = TextFormat {
        font_id: FontId::new(size, family),
        color,
        background: r.highlight.map(rgb).unwrap_or(Color32::TRANSPARENT),
        italics: r.italic,
        underline: if r.underline { Stroke::new(1.0_f32, color) } else { Stroke::NONE },
        strikethrough: if r.strike { Stroke::new(1.0_f32, color) } else { Stroke::NONE },
        ..Default::default()
    };
    if r.sup {
        f.valign = Align::TOP;
    }
    f
}

fn decode_image(bytes: &[u8]) -> Option<egui::ColorImage> {
    let mut img = image::load_from_memory(bytes).ok()?;
    if img.width() > 2048 || img.height() > 2048 {
        img = img.thumbnail(2048, 2048);
    }
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

fn texture(ui: &egui::Ui, rc: &RCtx, key: &str) -> Option<TextureHandle> {
    if let Some(v) = rc.textures.borrow().get(key) {
        return v.clone();
    }
    let tex = rc
        .images
        .get(key)
        .and_then(|b| decode_image(b))
        .map(|ci| ui.ctx().load_texture(format!("doc:{key}"), ci, TextureOptions::LINEAR));
    rc.textures.borrow_mut().insert(key.to_string(), tex.clone());
    tex
}

fn draw_image(ui: &mut egui::Ui, rc: &RCtx, key: &str, w_pt: f32, h_pt: f32) {
    match texture(ui, rc, key) {
        Some(tex) => {
            let natural = tex.size_vec2();
            let (mut w, mut h) = if w_pt > 1.0 && h_pt > 1.0 {
                (w_pt * rc.scale, h_pt * rc.scale)
            } else {
                (natural.x * 0.75 * rc.scale, natural.y * 0.75 * rc.scale)
            };
            let maxw = ui.available_width();
            if w > maxw && w > 0.0 {
                h *= maxw / w;
                w = maxw;
            }
            ui.add(egui::Image::new((tex.id(), vec2(w, h))));
        }
        None => {
            ui.label("[imagen no soportada]");
        }
    }
}
