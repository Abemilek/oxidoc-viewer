//! UI (egui): file browser and document viewer with virtualized scrolling.
use crate::doc::*;
use crate::load;
use crate::platform::{self, AppHandle};
use eframe::egui::{
    self, text::LayoutJob, vec2, Align, Color32, FontFamily, FontId, Stroke, TextFormat,
    TextureHandle, TextureOptions,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const TEXT: Color32 = Color32::from_rgb(25, 25, 25);
/// Document typographic points -> egui logical points.
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
    /// Measured height of every block (0.0 = not measured yet).
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
    /// Status bar height in physical pixels (the window is edge-to-edge).
    top_inset_px: f32,
    /// Floating top bar of the viewer (shown with a tap).
    show_bar: bool,
    bar_time: f64,
    bar_rect: Option<egui::Rect>,
}

/// Pure-black ("AMOLED") dark theme for the app chrome. The document stays white.
fn amoled_visuals() -> egui::Visuals {
    let g = Color32::from_gray;
    let mut v = egui::Visuals::dark();
    v.panel_fill = Color32::BLACK;
    v.window_fill = Color32::BLACK;
    v.extreme_bg_color = Color32::BLACK;
    v.faint_bg_color = g(10);
    v.widgets.noninteractive.bg_fill = Color32::BLACK;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, g(48));
    v.widgets.inactive.bg_fill = g(26);
    v.widgets.inactive.weak_bg_fill = g(26);
    v.widgets.hovered.bg_fill = g(44);
    v.widgets.hovered.weak_bg_fill = g(44);
    v.widgets.active.bg_fill = g(64);
    v.widgets.active.weak_bg_fill = g(64);
    v
}

/// Looks for a system bold font under `/system/fonts`.
fn setup_fonts(ctx: &egui::Context) -> bool {
    let candidates = [
        "/system/fonts/NotoSerif-Bold.ttf",
        "/system/fonts/NotoSans-Bold.ttf",
        "/system/fonts/DroidSans-Bold.ttf",
        "/system/fonts/Roboto-Bold.ttf",
    ];
    for p in candidates {
        if let Ok(bytes) = std::fs::read(p) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("bold".to_owned(), egui::FontData::from_owned(bytes).into());
            let mut list = vec!["bold".to_owned()];
            list.extend(
                fonts
                    .families
                    .get(&FontFamily::Proportional)
                    .cloned()
                    .unwrap_or_default(),
            );
            fonts.families.insert(FontFamily::Name("bold".into()), list);
            ctx.set_fonts(fonts);
            return true;
        }
    }
    false
}

impl ViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, handle: AppHandle) -> Self {
        let ctx = &cc.egui_ctx;
        // Always dark, whatever the system theme is.
        ctx.set_theme(egui::ThemePreference::Dark);
        ctx.set_visuals_of(egui::Theme::Dark, amoled_visuals());
        ctx.style_mut_of(egui::Theme::Dark, |s| {
            s.spacing.button_padding = vec2(12.0, 10.0);
            s.spacing.item_spacing = vec2(8.0, 8.0);
            s.interaction.selectable_labels = false; // so dragging scrolls on touch screens
        });
        let has_bold = setup_fonts(ctx);
        let quick = platform::quick_dirs(&handle);
        let dir = quick
            .first()
            .map(|q| q.1.clone())
            .unwrap_or_else(|| PathBuf::from("/"));
        let access_ok = platform::has_access(&handle);
        let top_inset_px = platform::status_bar_height_px(&handle);
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
            top_inset_px,
            show_bar: false,
            bar_time: 0.0,
            bar_rect: None,
        };
        app.refresh();
        // Started from "Open with" in a file manager or another app?
        if let Some((name, bytes)) = platform::launch_document(&app.handle) {
            match load::load_bytes(&name, &bytes) {
                Ok(doc) => app.show_document(doc),
                Err(e) => app.message = Some(e),
            }
        }
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
                    let is_dir = std::fs::metadata(&path)
                        .map(|m| m.is_dir())
                        .unwrap_or(false);
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
                self.dir_error = Some(format!("Could not open folder: {e}"));
            }
        }
    }

    fn open_file(&mut self, path: &Path) {
        match load::load(path) {
            Ok(doc) => self.show_document(doc),
            Err(e) => self.message = Some(e),
        }
    }

    fn show_document(&mut self, doc: Document) {
        let n = doc.blocks.len();
        let name = if doc.title.is_empty() {
            "document".to_string()
        } else {
            doc.title.clone()
        };
        self.open = Some(Open {
            doc,
            name,
            textures: RefCell::new(HashMap::new()),
            heights: vec![0.0; n],
            layout_key: (0, 0),
        });
        self.message = None;
        self.show_bar = false;
    }

    /// Back gesture / key: viewer -> browser, subfolder -> parent folder, root -> close the app.
    fn go_back(&mut self) {
        if self.open.is_some() {
            self.open = None;
            return;
        }
        let root = Path::new("/storage/emulated/0");
        let parent = self.dir.parent().map(|p| p.to_path_buf());
        match parent {
            Some(p) if self.dir.as_path() != root => {
                self.dir = p;
                self.message = None;
                self.refresh();
            }
            _ => platform::finish_activity(&self.handle),
        }
    }

    fn browser_ui(&mut self, ctx: &egui::Context) {
        let mut nav: Option<PathBuf> = None;
        let mut open: Option<PathBuf> = None;
        let mut up = false;
        let mut ask = false;
        let mut recheck = false;
        let inset = self.top_inset_px / ctx.pixels_per_point();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0 + inset);
            ui.heading("OxiDoc Viewer");
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
                    "To browse your files, the app needs the \"All files access\" permission.",
                );
                ui.horizontal(|ui| {
                    if ui.button("Grant access").clicked() {
                        ask = true;
                    }
                    if ui.button("Already granted").clicked() {
                        recheck = true;
                    }
                });
            }
            if let Some(m) = &self.message {
                ui.colored_label(Color32::from_rgb(190, 40, 40), m.as_str());
            }
            if ui.button("⬆ Parent folder").clicked() {
                up = true;
            }
            ui.separator();
            if let Some(e) = &self.dir_error {
                ui.label(e.as_str());
            }
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for e in &self.entries {
                        let icon = if e.is_dir { "📁" } else { "📄" };
                        let w = ui.available_width();
                        let b = egui::Button::new(format!("{icon}  {}", e.name))
                            .min_size(vec2(w, 44.0));
                        if ui.add(b).clicked() {
                            if e.is_dir {
                                nav = Some(e.path.clone());
                            } else {
                                open = Some(e.path.clone());
                            }
                        }
                    }
                    if self.entries.is_empty() && self.dir_error.is_none() {
                        ui.label("(empty folder or no supported documents)");
                    }
                    // Keep the last entries reachable above the navigation bar.
                    ui.add_space(64.0);
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
        let now = ctx.input(|i| i.time);
        let inset = self.top_inset_px / ctx.pixels_per_point();
        let name = self
            .open
            .as_ref()
            .map(|o| o.name.clone())
            .unwrap_or_default();
        let mut close = false;
        let mut reset_zoom = false;

        // Pinch to zoom.
        let dz = ctx.input(|i| i.zoom_delta());
        if (dz - 1.0).abs() > 0.001 {
            self.zoom = (self.zoom * dz).clamp(0.4, 4.0);
        }

        // A tap anywhere (outside the bar) shows or hides the floating bar.
        if ctx.input(|i| i.pointer.primary_clicked()) {
            let pos = ctx.input(|i| i.pointer.interact_pos());
            let on_bar = match (self.show_bar, self.bar_rect, pos) {
                (true, Some(r), Some(p)) => r.contains(p),
                _ => false,
            };
            if !on_bar {
                self.show_bar = !self.show_bar;
                self.bar_time = now;
            }
        }
        if self.show_bar && now - self.bar_time > 4.0 {
            self.show_bar = false;
        }

        // Black strip under the status bar so its icons never sit on top of a white page.
        if inset > 0.5 {
            egui::TopBottomPanel::top("status_strip")
                .frame(egui::Frame::default().fill(Color32::BLACK))
                .exact_height(inset)
                .show(ctx, |_ui| {});
        }

        let zoom = self.zoom;
        let has_bold = self.has_bold;

        let frame = egui::Frame::default()
            .fill(Color32::WHITE)
            .inner_margin(10.0);
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            if let Some(o) = &mut self.open {
                let Open {
                    doc,
                    textures,
                    heights,
                    layout_key,
                    ..
                } = o;
                let w = ui.available_width();
                let key = (w as i32, (zoom * 100.0) as i32);
                if *layout_key != key || heights.len() != doc.blocks.len() {
                    heights.clear();
                    heights.resize(doc.blocks.len(), 0.0);
                    *layout_key = key;
                }
                let rc = RCtx {
                    images: &doc.images,
                    textures: &*textures,
                    scale: zoom * PT_SCALE,
                    has_bold,
                };
                egui::ScrollArea::vertical()
                    .id_salt(("document", name.as_str()))
                    .auto_shrink([false, false])
                    .show_viewport(ui, |ui, viewport| {
                        ui.spacing_mut().item_spacing.y = (6.0 * zoom).clamp(3.0, 18.0);
                        let sp = ui.spacing().item_spacing.y;
                        let avail = ui.available_width();
                        let top = viewport.min.y - 800.0;
                        let bot = viewport.max.y + 800.0;
                        let mut y = 0.0f32;
                        for (i, b) in doc.blocks.iter().enumerate() {
                            let h = heights[i];
                            let known = h > 0.0;
                            let render = if known {
                                y + h >= top && y <= bot
                            } else {
                                y <= bot
                            };
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
                        // Keep the end of the document reachable above the navigation bar.
                        ui.add_space(64.0);
                    });
            }
        });

        // Floating bar: back, document name and zoom (tap the percentage to reset it).
        if self.show_bar {
            let zoom_pct = (self.zoom * 100.0).round() as i32;
            let screen_w = ctx.screen_rect().width();
            let resp = egui::Area::new(egui::Id::new("viewer_bar"))
                .anchor(egui::Align2::LEFT_TOP, vec2(0.0, inset))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::default()
                        .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 225))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_min_width(screen_w - 16.0);
                            ui.horizontal(|ui| {
                                if ui.button("◀ Back").clicked() {
                                    close = true;
                                }
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    if ui.button(format!("{zoom_pct}%")).clicked() {
                                        reset_zoom = true;
                                        self.bar_time = now;
                                    }
                                    ui.add(
                                        egui::Label::new(egui::RichText::new(&name).small())
                                            .truncate(),
                                    );
                                });
                            });
                        });
                });
            self.bar_rect = Some(resp.response.rect);
            ctx.request_repaint_after(std::time::Duration::from_millis(4100));
        } else {
            self.bar_rect = None;
        }

        if reset_zoom {
            self.zoom = 1.0;
        }
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
        let back = ctx.input(|i| {
            i.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::Key { pressed: true, key, .. }
                        if *key == egui::Key::Escape || format!("{key:?}") == "BrowserBack"
                )
            })
        });
        if back {
            self.go_back();
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
            let pt = if p.default_pt > 0.0 {
                p.default_pt
            } else {
                11.0
            };
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
    let ncols = t
        .rows
        .iter()
        .map(|r| r.iter().map(|c| c.span.max(1)).sum::<usize>())
        .max()
        .unwrap_or(0);
    if ncols == 0 {
        return;
    }
    let total_w = ui.available_width();
    let known = t.col_w.len() == ncols && t.col_w.iter().all(|w| *w > 0.0);
    let mut grid: Vec<f32> = if known {
        t.col_w.clone()
    } else {
        vec![1.0; ncols]
    };
    let sum: f32 = grid.iter().sum();
    for w in grid.iter_mut() {
        *w = *w / sum * total_w;
    }
    let pad = 4.0 * rc.scale.min(2.0);
    let stroke = Stroke::new(1.0_f32, Color32::from_gray(140));

    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        for row in &t.rows {
            if row.is_empty() {
                continue;
            }
            let origin = ui.available_rect_before_wrap().min;
            let mut x = 0.0f32;
            let mut col = 0usize;
            let mut geo: Vec<(f32, f32)> = Vec::new();
            let mut max_h = 0.0f32;
            for cell in row {
                if col >= ncols {
                    break;
                }
                let end = (col + cell.span.max(1)).min(ncols);
                let w: f32 = grid[col..end].iter().sum();
                // Each cell is drawn into its own child Ui with an exact width, so text wraps
                // inside the cell. A plain left-aligned layout avoids egui's justified text.
                let inner = egui::Rect::from_min_size(
                    origin + vec2(x + pad, pad),
                    vec2((w - 2.0 * pad).max(8.0), 100_000.0),
                );
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(inner)
                        .layout(egui::Layout::top_down(Align::Min)),
                );
                child.spacing_mut().item_spacing.y = (3.0 * rc.scale).min(8.0);
                for b in &cell.blocks {
                    render_block(&mut child, b, rc);
                }
                max_h = max_h.max(child.min_rect().height());
                geo.push((x, w));
                x += w;
                col = end;
            }
            let row_h = max_h + 2.0 * pad;
            for (cell_x, w) in geo {
                let r = egui::Rect::from_min_size(origin + vec2(cell_x, 0.0), vec2(w, row_h));
                let p = ui.painter();
                p.line_segment([r.left_top(), r.right_top()], stroke);
                p.line_segment([r.left_bottom(), r.right_bottom()], stroke);
                p.line_segment([r.left_top(), r.left_bottom()], stroke);
                p.line_segment([r.right_top(), r.right_bottom()], stroke);
            }
            let row_rect = egui::Rect::from_min_size(origin, vec2(total_w, row_h));
            ui.allocate_rect(row_rect, egui::Sense::hover());
        }
    });
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
    let base_pt = if p.default_pt > 0.0 {
        p.default_pt
    } else {
        11.0
    };
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
        color = Color32::BLACK; // no system bold font: at least increase contrast
    }
    let mut f = TextFormat {
        font_id: FontId::new(size, family),
        color,
        background: r.highlight.map(rgb).unwrap_or(Color32::TRANSPARENT),
        italics: r.italic,
        underline: if r.underline {
            Stroke::new(1.0_f32, color)
        } else {
            Stroke::NONE
        },
        strikethrough: if r.strike {
            Stroke::new(1.0_f32, color)
        } else {
            Stroke::NONE
        },
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
    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        rgba.as_raw(),
    ))
}

fn texture(ui: &egui::Ui, rc: &RCtx, key: &str) -> Option<TextureHandle> {
    if let Some(v) = rc.textures.borrow().get(key) {
        return v.clone();
    }
    let tex = rc.images.get(key).and_then(|b| decode_image(b)).map(|ci| {
        ui.ctx()
            .load_texture(format!("doc:{key}"), ci, TextureOptions::LINEAR)
    });
    rc.textures
        .borrow_mut()
        .insert(key.to_string(), tex.clone());
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
            ui.label(egui::RichText::new("[unsupported image]").color(TEXT));
        }
    }
}
