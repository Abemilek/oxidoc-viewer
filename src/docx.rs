//! Hand-written .docx (WordprocessingML) parser on top of `zip` and a custom tokenizer.
use crate::doc::*;
use crate::load::{read_bytes, read_str};
use crate::xml::*;
use std::collections::HashMap;
use std::io::Cursor;

// ---------------------------------------------------------------- properties

#[derive(Clone, Default)]
struct RPr {
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    strike: Option<bool>,
    color: Option<[u8; 3]>,
    size: Option<f32>,
    highlight: Option<[u8; 3]>,
    vert: Option<i8>,
    rstyle: Option<String>,
}

impl RPr {
    fn merge(&mut self, o: &RPr) {
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
        if o.highlight.is_some() {
            self.highlight = o.highlight;
        }
        if o.vert.is_some() {
            self.vert = o.vert;
        }
    }
}

#[derive(Clone, Default)]
struct PPr {
    style: Option<String>,
    jc: Option<Alignment>,
    num_id: Option<u32>,
    ilvl: Option<u8>,
    ind_left: Option<f32>,
    outline: Option<u8>,
}

impl PPr {
    fn merge(&mut self, o: &PPr) {
        if o.jc.is_some() {
            self.jc = o.jc;
        }
        if o.num_id.is_some() {
            self.num_id = o.num_id;
        }
        if o.ilvl.is_some() {
            self.ilvl = o.ilvl;
        }
        if o.ind_left.is_some() {
            self.ind_left = o.ind_left;
        }
        if o.outline.is_some() {
            self.outline = o.outline;
        }
    }
}

fn on(attrs: &[(String, String)]) -> bool {
    match attr(attrs, "val") {
        None => true,
        Some(v) => !matches!(v, "0" | "false" | "off" | "none"),
    }
}

fn hex(s: &str) -> Option<[u8; 3]> {
    if s.len() != 6 || !s.is_ascii() {
        return None;
    }
    Some([
        u8::from_str_radix(&s[0..2], 16).ok()?,
        u8::from_str_radix(&s[2..4], 16).ok()?,
        u8::from_str_radix(&s[4..6], 16).ok()?,
    ])
}

fn highlight_color(v: &str) -> Option<[u8; 3]> {
    Some(match v {
        "yellow" => [255, 255, 0],
        "green" => [0, 255, 0],
        "cyan" => [0, 255, 255],
        "magenta" => [255, 0, 255],
        "blue" => [0, 0, 255],
        "red" => [255, 0, 0],
        "darkBlue" => [0, 0, 139],
        "darkCyan" => [0, 139, 139],
        "darkGreen" => [0, 100, 0],
        "darkMagenta" => [139, 0, 139],
        "darkRed" => [139, 0, 0],
        "darkYellow" => [128, 128, 0],
        "darkGray" => [128, 128, 128],
        "lightGray" => [192, 192, 192],
        "black" => [0, 0, 0],
        "white" => [255, 255, 255],
        _ => return None,
    })
}

/// Called after the Start of <w:rPr> has been consumed.
fn parse_rpr(cur: &mut Cur) -> RPr {
    let mut r = RPr::default();
    while let Some(t) = cur.next() {
        match t {
            Tok::End(n) => {
                if n == "rPr" {
                    break;
                }
            }
            Tok::Start { name, attrs } => match name.as_str() {
                "b" => r.bold = Some(on(&attrs)),
                "i" => r.italic = Some(on(&attrs)),
                "u" => r.underline = Some(on(&attrs)),
                "strike" | "dstrike" => r.strike = Some(on(&attrs)),
                "color" => r.color = attr(&attrs, "val").and_then(hex),
                "sz" => {
                    r.size = attr(&attrs, "val")
                        .and_then(|v| v.parse::<f32>().ok())
                        .map(|v| v / 2.0)
                }
                "highlight" => r.highlight = attr(&attrs, "val").and_then(highlight_color),
                "vertAlign" => {
                    r.vert = match attr(&attrs, "val") {
                        Some("superscript") => Some(1),
                        Some("subscript") => Some(-1),
                        _ => Some(0),
                    }
                }
                "rStyle" => r.rstyle = attr(&attrs, "val").map(String::from),
                "rPrChange" => cur.skip("rPrChange"),
                _ => {}
            },
            _ => {}
        }
    }
    r
}

/// Called after the Start of <w:pPr> has been consumed.
fn parse_ppr(cur: &mut Cur) -> PPr {
    let mut p = PPr::default();
    while let Some(t) = cur.next() {
        match t {
            Tok::End(n) => {
                if n == "pPr" {
                    break;
                }
            }
            Tok::Start { name, attrs } => match name.as_str() {
                "pStyle" => p.style = attr(&attrs, "val").map(String::from),
                "jc" => {
                    p.jc = match attr(&attrs, "val") {
                        Some("center") => Some(Alignment::Center),
                        Some("right") | Some("end") => Some(Alignment::Right),
                        Some(_) => Some(Alignment::Left),
                        None => None,
                    }
                }
                "numId" => p.num_id = attr(&attrs, "val").and_then(|v| v.parse().ok()),
                "ilvl" => p.ilvl = attr(&attrs, "val").and_then(|v| v.parse().ok()),
                "ind" => {
                    let v = attr(&attrs, "left").or(attr(&attrs, "start"));
                    if let Some(v) = v.and_then(|v| v.parse::<f32>().ok()) {
                        p.ind_left = Some(v / 20.0);
                    }
                }
                "outlineLvl" => {
                    p.outline = attr(&attrs, "val")
                        .and_then(|v| v.parse::<u8>().ok())
                        .map(|v| v.saturating_add(1));
                }
                "pPrChange" | "rPr" | "sectPr" => cur.skip(&name),
                _ => {}
            },
            _ => {}
        }
    }
    p
}

// ---------------------------------------------------------------- styles

#[derive(Default)]
struct Style {
    name: String,
    based: Option<String>,
    rpr: RPr,
    ppr: PPr,
}

#[derive(Default)]
struct Styles {
    map: HashMap<String, Style>,
    default_rpr: RPr,
    default_para: Option<String>,
}

impl Styles {
    fn resolve(&self, id: &str) -> (RPr, PPr) {
        let mut chain: Vec<&Style> = Vec::new();
        let mut cur = Some(id);
        let mut n = 0;
        while let Some(c) = cur {
            if n > 10 {
                break;
            }
            match self.map.get(c) {
                Some(s) => {
                    chain.push(s);
                    cur = s.based.as_deref();
                }
                None => break,
            }
            n += 1;
        }
        let mut r = RPr::default();
        let mut p = PPr::default();
        for s in chain.iter().rev() {
            r.merge(&s.rpr);
            p.merge(&s.ppr);
        }
        (r, p)
    }

    fn name(&self, id: &str) -> &str {
        self.map.get(id).map(|s| s.name.as_str()).unwrap_or("")
    }
}

fn parse_style(cur: &mut Cur) -> Style {
    let mut s = Style::default();
    while let Some(t) = cur.next() {
        match t {
            Tok::End(n) => {
                if n == "style" {
                    break;
                }
            }
            Tok::Start { name, attrs } => match name.as_str() {
                "name" => s.name = attr(&attrs, "val").unwrap_or("").to_lowercase(),
                "basedOn" => s.based = attr(&attrs, "val").map(String::from),
                "pPr" => s.ppr = parse_ppr(cur),
                "rPr" => s.rpr = parse_rpr(cur),
                "tblStylePr" => cur.skip("tblStylePr"),
                _ => {}
            },
            _ => {}
        }
    }
    s
}

fn parse_styles(xml: &str) -> Styles {
    let mut cur = Cur::new(xml);
    let mut st = Styles::default();
    while let Some(t) = cur.next() {
        if let Tok::Start { name, attrs } = t {
            match name.as_str() {
                "rPr" => st.default_rpr = parse_rpr(&mut cur),
                "style" => {
                    let id = attr(&attrs, "styleId").unwrap_or("").to_string();
                    let is_default_para = attr(&attrs, "type") == Some("paragraph")
                        && matches!(attr(&attrs, "default"), Some("1") | Some("true"));
                    let s = parse_style(&mut cur);
                    if is_default_para {
                        st.default_para = Some(id.clone());
                    }
                    st.map.insert(id, s);
                }
                "latentStyles" => cur.skip("latentStyles"),
                _ => {}
            }
        }
    }
    st
}

fn heading_level(name: &str) -> u8 {
    if name == "title" {
        return 1;
    }
    if let Some(r) = name.strip_prefix("heading ") {
        return r.trim().parse::<u8>().unwrap_or(1).clamp(1, 6);
    }
    0
}

fn heading_pt(level: u8) -> f32 {
    match level {
        1 => 20.0,
        2 => 16.0,
        3 => 14.0,
        4 => 13.0,
        5 => 12.0,
        _ => 11.0,
    }
}

// ---------------------------------------------------------------- numbering

struct LvlDef {
    fmt: String,
    text: String,
    start: u32,
    ind: Option<f32>,
}

#[derive(Default)]
struct Numbering {
    abs: HashMap<u32, HashMap<usize, LvlDef>>,
    nums: HashMap<u32, u32>,
}

fn parse_numbering(xml: &str) -> Numbering {
    let mut cur = Cur::new(xml);
    let mut n = Numbering::default();
    while let Some(t) = cur.next() {
        let (name, attrs) = match t {
            Tok::Start { name, attrs } => (name, attrs),
            _ => continue,
        };
        match name.as_str() {
            "abstractNum" => {
                let id: u32 = attr(&attrs, "abstractNumId")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let mut lvls: HashMap<usize, LvlDef> = HashMap::new();
                while let Some(t2) = cur.next() {
                    match t2 {
                        Tok::End(e) => {
                            if e == "abstractNum" {
                                break;
                            }
                        }
                        Tok::Start {
                            name: n2,
                            attrs: a2,
                        } => {
                            if n2 == "lvl" {
                                let ilvl: usize =
                                    attr(&a2, "ilvl").and_then(|v| v.parse().ok()).unwrap_or(0);
                                let mut def = LvlDef {
                                    fmt: "decimal".into(),
                                    text: String::new(),
                                    start: 1,
                                    ind: None,
                                };
                                while let Some(t3) = cur.next() {
                                    match t3 {
                                        Tok::End(e) => {
                                            if e == "lvl" {
                                                break;
                                            }
                                        }
                                        Tok::Start {
                                            name: n3,
                                            attrs: a3,
                                        } => match n3.as_str() {
                                            "start" => {
                                                def.start = attr(&a3, "val")
                                                    .and_then(|v| v.parse().ok())
                                                    .unwrap_or(1)
                                            }
                                            "numFmt" => {
                                                def.fmt = attr(&a3, "val")
                                                    .unwrap_or("decimal")
                                                    .to_string()
                                            }
                                            "lvlText" => {
                                                def.text =
                                                    attr(&a3, "val").unwrap_or("").to_string()
                                            }
                                            "ind" => {
                                                let v = attr(&a3, "left").or(attr(&a3, "start"));
                                                def.ind = v
                                                    .and_then(|v| v.parse::<f32>().ok())
                                                    .map(|v| v / 20.0);
                                            }
                                            _ => {}
                                        },
                                        _ => {}
                                    }
                                }
                                lvls.insert(ilvl.min(8), def);
                            }
                        }
                        _ => {}
                    }
                }
                n.abs.insert(id, lvls);
            }
            "num" => {
                let id: u32 = attr(&attrs, "numId")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                while let Some(t2) = cur.next() {
                    match t2 {
                        Tok::End(e) => {
                            if e == "num" {
                                break;
                            }
                        }
                        Tok::Start {
                            name: n2,
                            attrs: a2,
                        } => match n2.as_str() {
                            "abstractNumId" => {
                                if let Some(a) = attr(&a2, "val").and_then(|v| v.parse().ok()) {
                                    n.nums.insert(id, a);
                                }
                            }
                            "lvlOverride" => cur.skip("lvlOverride"),
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    n
}

fn roman(mut n: u32) -> String {
    let t = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut s = String::new();
    if n == 0 || n > 3999 {
        return n.to_string();
    }
    for (v, r) in t {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    s
}

fn letters(n: u32, upper: bool) -> String {
    if n == 0 {
        return "0".into();
    }
    let idx = (n - 1) % 26;
    let times = (n - 1) / 26 + 1;
    let base = if upper { b'A' } else { b'a' };
    let c = (base + idx as u8) as char;
    std::iter::repeat(c).take(times as usize).collect()
}

fn fmt_num(n: u32, fmt: &str) -> String {
    match fmt {
        "lowerLetter" => letters(n, false),
        "upperLetter" => letters(n, true),
        "lowerRoman" => roman(n),
        "upperRoman" => roman(n).to_uppercase(),
        "decimalZero" => format!("{:02}", n),
        _ => n.to_string(),
    }
}

// ---------------------------------------------------------------- relationships

fn parse_rels(xml: &str) -> HashMap<String, String> {
    let mut cur = Cur::new(xml);
    let mut m = HashMap::new();
    while let Some(t) = cur.next() {
        if let Tok::Start { name, attrs } = t {
            if name == "Relationship" && attr(&attrs, "TargetMode") != Some("External") {
                if let (Some(id), Some(target)) = (attr(&attrs, "Id"), attr(&attrs, "Target")) {
                    m.insert(id.to_string(), target.to_string());
                }
            }
        }
    }
    m
}

// ---------------------------------------------------------------- context

struct Ctx<'a> {
    styles: &'a Styles,
    num: &'a Numbering,
    rels: &'a HashMap<String, String>,
    counters: HashMap<u32, [Option<u32>; 9]>,
    media: Vec<String>,
    default_pt: f32,
}

impl<'a> Ctx<'a> {
    fn rel(&mut self, id: &str) -> Option<String> {
        let t = self.rels.get(id)?;
        let path = if let Some(s) = t.strip_prefix('/') {
            s.to_string()
        } else if let Some(s) = t.strip_prefix("../") {
            s.to_string()
        } else {
            format!("word/{}", t)
        };
        if !self.media.contains(&path) {
            self.media.push(path.clone());
        }
        Some(path)
    }

    /// Returns (label, left indent in pt when defined by the level).
    fn list_label(&mut self, num_id: u32, ilvl: usize) -> Option<(String, Option<f32>)> {
        let ilvl = ilvl.min(8);
        let num = self.num;
        let abs_id = *num.nums.get(&num_id)?;
        let lv = num.abs.get(&abs_id)?;
        let def = lv.get(&ilvl)?;
        let ind = def.ind;
        if def.fmt == "bullet" {
            let b = ["•", "–", "·"][ilvl % 3];
            return Some((b.to_string(), ind));
        }
        if def.fmt == "none" {
            return Some((String::new(), ind));
        }
        let c = self.counters.entry(num_id).or_insert([None; 9]);
        match c[ilvl] {
            None => c[ilvl] = Some(def.start),
            Some(n) => c[ilvl] = Some(n + 1),
        }
        for j in (ilvl + 1)..9 {
            c[j] = None;
        }
        let snapshot = *c;
        let chars: Vec<char> = def.text.chars().collect();
        let mut out = String::new();
        let mut k = 0;
        while k < chars.len() {
            if chars[k] == '%' && k + 1 < chars.len() && chars[k + 1].is_ascii_digit() {
                let li = (chars[k + 1] as usize).saturating_sub('1' as usize).min(8);
                let (fmt, start) = match lv.get(&li) {
                    Some(d) => (d.fmt.as_str(), d.start),
                    None => ("decimal", 1),
                };
                let n = snapshot[li].unwrap_or(start);
                out.push_str(&fmt_num(n, fmt));
                k += 2;
            } else {
                out.push(chars[k]);
                k += 1;
            }
        }
        Some((out, ind))
    }
}

// ---------------------------------------------------------------- blocks

fn apply_ppr(ppr: &PPr, p: &mut Para, base: &mut RPr, cx: &mut Ctx) {
    let style_id: Option<String> = ppr.style.clone().or_else(|| cx.styles.default_para.clone());
    let (srpr, sppr) = match &style_id {
        Some(id) => cx.styles.resolve(id),
        None => (RPr::default(), PPr::default()),
    };
    base.merge(&srpr);
    let sname = style_id
        .as_deref()
        .map(|id| cx.styles.name(id))
        .unwrap_or("");
    let mut level = heading_level(sname);
    if level == 0 {
        let o = ppr.outline.or(sppr.outline).unwrap_or(0);
        if (1..=6).contains(&o) {
            level = o;
        }
    }
    if level > 0 {
        if srpr.size.is_none() {
            base.size = Some(heading_pt(level));
        }
        if srpr.bold.is_none() {
            base.bold = Some(true);
        }
    }
    p.heading = level;
    p.align = ppr.jc.or(sppr.jc).unwrap_or(Alignment::Left);
    let ind = ppr.ind_left.or(sppr.ind_left);
    p.indent_pt = ind.unwrap_or(0.0);
    p.label = None;
    let num_id = ppr.num_id.or(sppr.num_id);
    let ilvl = ppr.ilvl.or(sppr.ilvl).unwrap_or(0) as usize;
    if let Some(n) = num_id {
        if n != 0 {
            if let Some((label, lind)) = cx.list_label(n, ilvl) {
                p.label = if label.is_empty() { None } else { Some(label) };
                if ind.is_none() {
                    p.indent_pt = lind.unwrap_or(18.0 * (ilvl as f32 + 1.0));
                }
            }
        }
    }
}

enum Item {
    Text(String),
    Img { key: String, w: f32, h: f32 },
    Page,
}

/// Called after the Start of <w:r> has been consumed.
fn parse_run(cur: &mut Cur, cx: &mut Ctx, base: &RPr, link: bool, out: &mut Vec<Inline>) {
    let mut direct = RPr::default();
    let mut items: Vec<Item> = Vec::new();
    let mut text = String::new();
    while let Some(t) = cur.next() {
        match t {
            Tok::End(n) => {
                if n == "r" {
                    break;
                }
            }
            Tok::Start { name, attrs } => match name.as_str() {
                "rPr" => direct = parse_rpr(cur),
                "t" => {
                    while let Some(x) = cur.next() {
                        match x {
                            Tok::Text(s) => text.push_str(&s),
                            Tok::End(n2) => {
                                if n2 == "t" {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                "tab" => text.push_str("    "),
                "br" => {
                    if attr(&attrs, "type") == Some("page") {
                        if !text.is_empty() {
                            items.push(Item::Text(std::mem::take(&mut text)));
                        }
                        items.push(Item::Page);
                    } else {
                        text.push('\n');
                    }
                }
                "cr" => text.push('\n'),
                "noBreakHyphen" => text.push('-'),
                "Fallback" => cur.skip("Fallback"),
                "drawing" | "pict" | "object" => {
                    let end = name.clone();
                    let mut depth = 1;
                    let mut key: Option<String> = None;
                    let (mut w, mut h) = (0.0f32, 0.0f32);
                    while let Some(x) = cur.next() {
                        match x {
                            Tok::Start { name: n, attrs: a } => {
                                if n == end {
                                    depth += 1;
                                }
                                match n.as_str() {
                                    "blip" => {
                                        if key.is_none() {
                                            if let Some(id) = attr(&a, "embed") {
                                                key = cx.rel(id);
                                            }
                                        }
                                    }
                                    "imagedata" => {
                                        if key.is_none() {
                                            if let Some(id) = attr(&a, "id") {
                                                key = cx.rel(id);
                                            }
                                        }
                                    }
                                    "extent" => {
                                        if w == 0.0 {
                                            w = attr(&a, "cx")
                                                .and_then(|v| v.parse::<f32>().ok())
                                                .unwrap_or(0.0)
                                                / 12700.0;
                                            h = attr(&a, "cy")
                                                .and_then(|v| v.parse::<f32>().ok())
                                                .unwrap_or(0.0)
                                                / 12700.0;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            Tok::End(n) => {
                                if n == end {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(k) = key {
                        if !text.is_empty() {
                            items.push(Item::Text(std::mem::take(&mut text)));
                        }
                        items.push(Item::Img { key: k, w, h });
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    if !text.is_empty() {
        items.push(Item::Text(text));
    }

    let mut props = base.clone();
    if let Some(id) = &direct.rstyle {
        props.merge(&cx.styles.resolve(id).0);
    }
    props.merge(&direct);
    let size = props.size.unwrap_or(cx.default_pt);
    let mut color = props.color;
    let mut underline = props.underline.unwrap_or(false);
    if link {
        if color.is_none() {
            color = Some([5, 99, 193]);
        }
        if props.underline.is_none() {
            underline = true;
        }
    }
    for it in items {
        match it {
            Item::Text(s) => out.push(Inline::Text(Run {
                text: s,
                bold: props.bold.unwrap_or(false),
                italic: props.italic.unwrap_or(false),
                underline,
                strike: props.strike.unwrap_or(false),
                color,
                size_pt: size,
                highlight: props.highlight,
                sup: props.vert == Some(1),
                sub: props.vert == Some(-1),
            })),
            Item::Img { key, w, h } => out.push(Inline::Image {
                key,
                w_pt: w,
                h_pt: h,
            }),
            Item::Page => out.push(Inline::PageBreak),
        }
    }
}

/// Called after the Start of <w:p> has been consumed.
fn parse_para(cur: &mut Cur, cx: &mut Ctx) -> Para {
    let mut p = Para {
        default_pt: cx.default_pt,
        ..Default::default()
    };
    let mut base = cx.styles.default_rpr.clone();
    apply_ppr(&PPr::default(), &mut p, &mut base, cx);
    let mut link: i32 = 0;
    while let Some(t) = cur.next() {
        match t {
            Tok::End(n) => {
                if n == "p" {
                    break;
                }
                if n == "hyperlink" {
                    link -= 1;
                }
            }
            Tok::Start { name, .. } => match name.as_str() {
                "pPr" => {
                    let ppr = parse_ppr(cur);
                    apply_ppr(&ppr, &mut p, &mut base, cx);
                }
                "r" => parse_run(cur, cx, &base, link > 0, &mut p.inlines),
                "hyperlink" => link += 1,
                "Fallback" | "txbxContent" | "del" | "moveFrom" => cur.skip(&name),
                _ => {}
            },
            _ => {}
        }
    }
    p
}

/// Called after the Start of <w:tbl> has been consumed.
fn parse_table(cur: &mut Cur, cx: &mut Ctx) -> Table {
    let mut rows = Vec::new();
    let mut col_w: Vec<f32> = Vec::new();
    while let Some(t) = cur.next() {
        match t {
            Tok::End(n) => {
                if n == "tbl" {
                    break;
                }
            }
            Tok::Start { name, attrs } => match name.as_str() {
                "tr" => rows.push(parse_row(cur, cx)),
                "gridCol" => {
                    let w = attr(&attrs, "w").and_then(|v| v.parse::<f32>().ok());
                    col_w.push(w.unwrap_or(0.0) / 20.0);
                }
                _ => {}
            },
            _ => {}
        }
    }
    Table { rows, col_w }
}

fn parse_row(cur: &mut Cur, cx: &mut Ctx) -> Vec<Cell> {
    let mut cells = Vec::new();
    while let Some(t) = cur.next() {
        match t {
            Tok::End(n) => {
                if n == "tr" {
                    break;
                }
            }
            Tok::Start { name, .. } => {
                if name == "tc" {
                    cells.push(parse_cell(cur, cx));
                }
            }
            _ => {}
        }
    }
    cells
}

fn parse_cell(cur: &mut Cur, cx: &mut Ctx) -> Cell {
    let mut blocks = Vec::new();
    let mut span = 1usize;
    while let Some(t) = cur.next() {
        match t {
            Tok::End(n) => {
                if n == "tc" {
                    break;
                }
            }
            Tok::Start { name, attrs } => match name.as_str() {
                "p" => blocks.push(Block::Para(parse_para(cur, cx))),
                "tbl" => blocks.push(Block::Table(parse_table(cur, cx))),
                "gridSpan" => {
                    let s = attr(&attrs, "val").and_then(|v| v.parse::<usize>().ok());
                    span = s.unwrap_or(1).max(1);
                }
                _ => {}
            },
            _ => {}
        }
    }
    Cell { blocks, span }
}

// ---------------------------------------------------------------- entrypoint

pub fn parse(bytes: &[u8]) -> Result<Document, String> {
    let mut zip =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("Invalid ZIP: {e}"))?;
    let doc_xml = read_str(&mut zip, "word/document.xml")
        .ok_or_else(|| "word/document.xml not found (is this a valid .docx file?)".to_string())?;
    let styles = read_str(&mut zip, "word/styles.xml")
        .map(|s| parse_styles(&s))
        .unwrap_or_default();
    let numbering = read_str(&mut zip, "word/numbering.xml")
        .map(|s| parse_numbering(&s))
        .unwrap_or_default();
    let rels = read_str(&mut zip, "word/_rels/document.xml.rels")
        .map(|s| parse_rels(&s))
        .unwrap_or_default();

    let mut b = styles.default_rpr.clone();
    if let Some(id) = &styles.default_para {
        b.merge(&styles.resolve(id).0);
    }
    let default_pt = b.size.unwrap_or(11.0);

    let mut cx = Ctx {
        styles: &styles,
        num: &numbering,
        rels: &rels,
        counters: HashMap::new(),
        media: Vec::new(),
        default_pt,
    };

    let mut cur = Cur::new(&doc_xml);
    let mut blocks: Vec<Block> = Vec::new();
    while let Some(t) = cur.next() {
        if let Tok::Start { name, .. } = t {
            match name.as_str() {
                "p" => blocks.push(Block::Para(parse_para(&mut cur, &mut cx))),
                "tbl" => blocks.push(Block::Table(parse_table(&mut cur, &mut cx))),
                "Fallback" => cur.skip("Fallback"),
                _ => {}
            }
        }
    }

    let media = std::mem::take(&mut cx.media);
    let mut images = HashMap::new();
    for k in media {
        if let Some(bytes) = read_bytes(&mut zip, &k) {
            images.insert(k, bytes);
        }
    }
    Ok(Document {
        blocks,
        images,
        title: String::new(),
    })
}
