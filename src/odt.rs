//! Basic OpenDocument Text (.odt) parser.
use crate::doc::*;
use crate::load::{read_bytes, read_str};
use crate::xml::*;
use std::collections::HashMap;
use std::io::Cursor;

#[derive(Clone, Default)]
struct OStyle {
    parent: Option<String>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    strike: Option<bool>,
    color: Option<[u8; 3]>,
    size: Option<f32>,
    bg: Option<[u8; 3]>,
    align: Option<Alignment>,
    break_before: Option<bool>,
    break_after: Option<bool>,
}

impl OStyle {
    fn merge(&mut self, o: &OStyle) {
        if o.bold.is_some() {
            self.bold = o.bold;
        }
        if o.italic.is_some() {
            self.italic = o.italic;
        }
        if o.underline.is_some() {
            self.underline = o.underline;
        }
        if o.strike.is_some() {
            self.strike = o.strike;
        }
        if o.color.is_some() {
            self.color = o.color;
        }
        if o.size.is_some() {
            self.size = o.size;
        }
        if o.bg.is_some() {
            self.bg = o.bg;
        }
        if o.align.is_some() {
            self.align = o.align;
        }
        if o.break_before.is_some() {
            self.break_before = o.break_before;
        }
        if o.break_after.is_some() {
            self.break_after = o.break_after;
        }
    }
}

#[derive(Default)]
struct Styles {
    map: HashMap<String, OStyle>,
    ordered: HashMap<String, bool>,
}

fn hex_color(s: &str) -> Option<[u8; 3]> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 || !s.is_ascii() {
        return None;
    }
    Some([
        u8::from_str_radix(&s[0..2], 16).ok()?,
        u8::from_str_radix(&s[2..4], 16).ok()?,
        u8::from_str_radix(&s[4..6], 16).ok()?,
    ])
}

/// Converts "12pt", "2.5cm", "1in", "10mm", or "16px" to points.
fn len_pt(s: &str) -> Option<f32> {
    let s = s.trim();
    let units: [(&str, f32); 5] = [
        ("pt", 1.0),
        ("cm", 28.3465),
        ("mm", 2.83465),
        ("in", 72.0),
        ("px", 0.75),
    ];
    for (u, k) in units {
        if let Some(n) = s.strip_suffix(u) {
            return n.trim().parse::<f32>().ok().map(|v| v * k);
        }
    }
    None
}

impl Styles {
    fn scan(&mut self, xml: &str) {
        let mut cur = Cur::new(xml);
        let mut cs: Option<String> = None;
        let mut cl: Option<String> = None;
        while let Some(t) = cur.next() {
            match t {
                Tok::Start { name, attrs } => match name.as_str() {
                    "style" => {
                        cs = attr(&attrs, "name").map(String::from);
                        if let Some(n) = &cs {
                            let e = self.map.entry(n.clone()).or_default();
                            e.parent = attr(&attrs, "parent-style-name").map(String::from);
                        }
                    }
                    "text-properties" => {
                        if let Some(n) = &cs {
                            let e = self.map.entry(n.clone()).or_default();
                            if let Some(v) = attr(&attrs, "font-weight") {
                                e.bold = Some(v == "bold" || v == "700");
                            }
                            if let Some(v) = attr(&attrs, "font-style") {
                                e.italic = Some(v == "italic" || v == "oblique");
                            }
                            if let Some(v) = attr(&attrs, "text-underline-style") {
                                e.underline = Some(v != "none");
                            }
                            if let Some(v) = attr(&attrs, "text-line-through-style") {
                                e.strike = Some(v != "none");
                            }
                            if let Some(c) = attr(&attrs, "color").and_then(hex_color) {
                                e.color = Some(c);
                            }
                            if let Some(sz) = attr(&attrs, "font-size").and_then(len_pt) {
                                e.size = Some(sz);
                            }
                            if let Some(c) = attr(&attrs, "background-color").and_then(hex_color) {
                                e.bg = Some(c);
                            }
                        }
                    }
                    "paragraph-properties" => {
                        if let Some(n) = &cs {
                            let e = self.map.entry(n.clone()).or_default();
                            if let Some(v) = attr(&attrs, "break-before") {
                                e.break_before = Some(v == "page");
                            }
                            if let Some(v) = attr(&attrs, "break-after") {
                                e.break_after = Some(v == "page");
                            }
                            match attr(&attrs, "text-align") {
                                Some("center") => e.align = Some(Alignment::Center),
                                Some("end") | Some("right") => e.align = Some(Alignment::Right),
                                Some(_) => e.align = Some(Alignment::Left),
                                None => {}
                            }
                        }
                    }
                    "list-style" => cl = attr(&attrs, "name").map(String::from),
                    "list-level-style-number" => {
                        if let Some(n) = &cl {
                            self.ordered.entry(n.clone()).or_insert(true);
                        }
                    }
                    "list-level-style-bullet" | "list-level-style-image" => {
                        if let Some(n) = &cl {
                            self.ordered.entry(n.clone()).or_insert(false);
                        }
                    }
                    _ => {}
                },
                Tok::End(n) => match n.as_str() {
                    "style" => cs = None,
                    "list-style" => cl = None,
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn resolved(&self, name: &str) -> OStyle {
        let mut chain: Vec<&OStyle> = Vec::new();
        let mut cur = Some(name);
        let mut n = 0;
        while let Some(c) = cur {
            if n > 8 {
                break;
            }
            match self.map.get(c) {
                Some(s) => {
                    chain.push(s);
                    cur = s.parent.as_deref();
                }
                None => break,
            }
            n += 1;
        }
        let mut r = OStyle::default();
        for s in chain.iter().rev() {
            r.merge(s);
        }
        r
    }
}

struct Ctx<'a> {
    st: &'a Styles,
    media: Vec<String>,
}

fn heading_pt(level: u8) -> f32 {
    match level {
        1 => 20.0,
        2 => 16.0,
        3 => 14.0,
        4 => 13.0,
        _ => 12.0,
    }
}

fn push_text(buf: &mut String, s: &str) {
    // ODF collapses whitespace; real repeated spaces are encoded as <text:s/>.
    for ch in s.chars() {
        let ch = if ch == '\n' || ch == '\t' || ch == '\r' {
            ' '
        } else {
            ch
        };
        if ch == ' ' && buf.ends_with(' ') {
            continue;
        }
        buf.push(ch);
    }
}

fn flush(buf: &mut String, props: &OStyle, out: &mut Vec<Inline>) {
    if buf.is_empty() {
        return;
    }
    out.push(Inline::Text(Run {
        text: std::mem::take(buf),
        bold: props.bold.unwrap_or(false),
        italic: props.italic.unwrap_or(false),
        underline: props.underline.unwrap_or(false),
        strike: props.strike.unwrap_or(false),
        color: props.color,
        size_pt: props.size.unwrap_or(12.0),
        highlight: props.bg,
        sup: false,
        sub: false,
    }));
}

/// Called after the Start of <text:p> or <text:h> has been consumed; `end` is "p" or "h".
fn parse_para(cur: &mut Cur, cx: &mut Ctx, end: &str, attrs: &[(String, String)]) -> Para {
    let heading: u8 = if end == "h" {
        attr(attrs, "outline-level")
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(1)
            .clamp(1, 6)
    } else {
        0
    };
    let mut base = attr(attrs, "style-name")
        .map(|n| cx.st.resolved(n))
        .unwrap_or_default();
    if heading > 0 {
        if base.size.is_none() {
            base.size = Some(heading_pt(heading));
        }
        if base.bold.is_none() {
            base.bold = Some(true);
        }
    }
    let mut p = Para {
        heading,
        align: base.align.unwrap_or(Alignment::Left),
        default_pt: 12.0,
        ..Default::default()
    };
    p.break_before = base.break_before.unwrap_or(false);
    p.break_after = base.break_after.unwrap_or(false);
    let mut stack: Vec<OStyle> = vec![base];
    let mut buf = String::new();
    let (mut fw, mut fh) = (0.0f32, 0.0f32);
    while let Some(t) = cur.next() {
        match t {
            Tok::Text(s) => push_text(&mut buf, &s),
            Tok::End(n) => {
                if n == end {
                    break;
                }
                if n == "span" || n == "a" {
                    flush(&mut buf, stack.last().unwrap(), &mut p.inlines);
                    if stack.len() > 1 {
                        stack.pop();
                    }
                }
            }
            Tok::Start { name, attrs: a } => match name.as_str() {
                "span" => {
                    flush(&mut buf, stack.last().unwrap(), &mut p.inlines);
                    let mut cur_style = stack.last().unwrap().clone();
                    if let Some(sn) = attr(&a, "style-name") {
                        cur_style.merge(&cx.st.resolved(sn));
                    }
                    stack.push(cur_style);
                }
                "a" => {
                    flush(&mut buf, stack.last().unwrap(), &mut p.inlines);
                    let mut cur_style = stack.last().unwrap().clone();
                    if cur_style.color.is_none() {
                        cur_style.color = Some([5, 99, 193]);
                    }
                    cur_style.underline = Some(true);
                    stack.push(cur_style);
                }
                "s" => {
                    let n: usize = attr(&a, "c")
                        .and_then(|v| v.parse::<usize>().ok())
                        .unwrap_or(1)
                        .min(200);
                    for _ in 0..n {
                        buf.push(' ');
                    }
                }
                "soft-page-break" => {
                    if p.inlines.is_empty() && buf.is_empty() {
                        p.break_before = true;
                    } else {
                        p.break_after = true;
                    }
                }
                "tab" => buf.push_str("    "),
                "line-break" => buf.push('\n'),
                "frame" => {
                    fw = attr(&a, "width").and_then(len_pt).unwrap_or(0.0);
                    fh = attr(&a, "height").and_then(len_pt).unwrap_or(0.0);
                }
                "image" => {
                    if let Some(href) = attr(&a, "href") {
                        flush(&mut buf, stack.last().unwrap(), &mut p.inlines);
                        let key = href.to_string();
                        if !cx.media.contains(&key) {
                            cx.media.push(key.clone());
                        }
                        p.inlines.push(Inline::Image {
                            key,
                            w_pt: fw,
                            h_pt: fh,
                        });
                    }
                }
                "text-box" | "annotation" | "note" => cur.skip(&name),
                _ => {}
            },
        }
    }
    flush(&mut buf, stack.last().unwrap(), &mut p.inlines);
    p
}

fn parse_list(
    cur: &mut Cur,
    cx: &mut Ctx,
    depth: usize,
    inherited: Option<String>,
    out: &mut Vec<Block>,
    attrs: &[(String, String)],
) {
    let name: Option<String> = attr(attrs, "style-name").map(String::from).or(inherited);
    let ordered = name
        .as_ref()
        .and_then(|n| cx.st.ordered.get(n))
        .copied()
        .unwrap_or(false);
    let mut counter = 0u32;
    while let Some(t) = cur.next() {
        match t {
            Tok::End(n) => {
                if n == "list" {
                    break;
                }
            }
            Tok::Start { name: n, .. } => {
                if n == "list-item" || n == "list-header" {
                    counter += 1;
                    let mut first = true;
                    while let Some(t2) = cur.next() {
                        match t2 {
                            Tok::End(e) => {
                                if e == "list-item" || e == "list-header" {
                                    break;
                                }
                            }
                            Tok::Start {
                                name: n2,
                                attrs: a2,
                            } => match n2.as_str() {
                                "p" | "h" => {
                                    let mut p = parse_para(cur, cx, &n2, &a2);
                                    p.indent_pt = 18.0 * (depth as f32 + 1.0);
                                    if first {
                                        p.label = Some(if ordered {
                                            format!("{}.", counter)
                                        } else {
                                            ["•", "–", "·"][depth % 3].to_string()
                                        });
                                        first = false;
                                    }
                                    out.push(Block::Para(p));
                                }
                                "list" => parse_list(cur, cx, depth + 1, name.clone(), out, &a2),
                                "table" => out.push(Block::Table(parse_table(cur, cx))),
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn parse_cell_blocks(cur: &mut Cur, cx: &mut Ctx) -> Vec<Block> {
    let mut blocks = Vec::new();
    while let Some(t) = cur.next() {
        match t {
            Tok::End(n) => {
                if n == "table-cell" {
                    break;
                }
            }
            Tok::Start { name, attrs } => match name.as_str() {
                "p" | "h" => blocks.push(Block::Para(parse_para(cur, cx, &name, &attrs))),
                "list" => parse_list(cur, cx, 0, None, &mut blocks, &attrs),
                "table" => blocks.push(Block::Table(parse_table(cur, cx))),
                _ => {}
            },
            _ => {}
        }
    }
    blocks
}

/// Called after the Start of <table:table> has been consumed.
fn parse_table(cur: &mut Cur, cx: &mut Ctx) -> Table {
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    while let Some(t) = cur.next() {
        match t {
            Tok::End(n) => {
                if n == "table" {
                    break;
                }
            }
            Tok::Start { name, .. } => {
                if name == "table-row" {
                    let mut cells = Vec::new();
                    while let Some(t2) = cur.next() {
                        match t2 {
                            Tok::End(e) => {
                                if e == "table-row" {
                                    break;
                                }
                            }
                            Tok::Start {
                                name: n2,
                                attrs: a2,
                            } => {
                                if n2 == "table-cell" {
                                    let s = attr(&a2, "number-columns-spanned")
                                        .and_then(|v| v.parse::<usize>().ok());
                                    cells.push(Cell {
                                        blocks: parse_cell_blocks(cur, cx),
                                        span: s.unwrap_or(1).max(1),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                    rows.push(cells);
                }
            }
            _ => {}
        }
    }
    Table {
        rows,
        col_w: Vec::new(),
    }
}

pub fn parse(bytes: &[u8]) -> Result<Document, String> {
    let mut zip =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("Invalid ZIP: {e}"))?;
    let content =
        read_str(&mut zip, "content.xml").ok_or_else(|| "content.xml not found".to_string())?;
    let mut st = Styles::default();
    if let Some(s) = read_str(&mut zip, "styles.xml") {
        st.scan(&s);
    }
    st.scan(&content);

    let mut cx = Ctx {
        st: &st,
        media: Vec::new(),
    };
    let mut cur = Cur::new(&content);
    let mut blocks: Vec<Block> = Vec::new();
    let mut pending_break = false;
    while let Some(t) = cur.next() {
        if let Tok::Start { name, attrs } = t {
            match name.as_str() {
                "soft-page-break" => pending_break = true,
                "p" | "h" => {
                    let mut p = parse_para(&mut cur, &mut cx, &name, &attrs);
                    if pending_break {
                        p.break_before = true;
                        pending_break = false;
                    }
                    blocks.push(Block::Para(p));
                }
                "list" => parse_list(&mut cur, &mut cx, 0, None, &mut blocks, &attrs),
                "table" => blocks.push(Block::Table(parse_table(&mut cur, &mut cx))),
                _ => {}
            }
        }
    }

    let media = std::mem::take(&mut cx.media);
    let mut images = HashMap::new();
    for k in media {
        if let Some(b) = read_bytes(&mut zip, &k) {
            images.insert(k, b);
        }
    }
    Ok(Document {
        blocks,
        images,
        ..Default::default()
    })
}
