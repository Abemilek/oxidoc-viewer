//! Internal document model, independent of the source format.
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, Default)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub color: Option<[u8; 3]>,
    pub size_pt: f32,
    pub highlight: Option<[u8; 3]>,
    pub sup: bool,
    pub sub: bool,
}

#[derive(Clone, Debug)]
pub enum Inline {
    Text(Run),
    Image { key: String, w_pt: f32, h_pt: f32 },
    PageBreak,
}

#[derive(Clone, Debug, Default)]
pub struct Para {
    pub inlines: Vec<Inline>,
    pub align: Alignment,
    pub heading: u8,
    pub label: Option<String>,
    pub indent_pt: f32,
    pub default_pt: f32,
    /// A new page starts at this paragraph.
    pub break_before: bool,
    /// A new page starts after this paragraph.
    pub break_after: bool,
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub blocks: Vec<Block>,
    /// Number of grid columns this cell spans (at least 1).
    pub span: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Table {
    pub rows: Vec<Vec<Cell>>,
    /// Grid column widths in points (empty when unknown).
    pub col_w: Vec<f32>,
}

#[derive(Clone, Debug)]
pub enum Block {
    Para(Para),
    Table(Table),
}

/// Physical page geometry in points.
#[derive(Clone, Copy, Debug)]
pub struct PageInfo {
    pub w_pt: f32,
    pub h_pt: f32,
    pub margin_l: f32,
    pub margin_r: f32,
    pub margin_t: f32,
    pub margin_b: f32,
}

impl Default for PageInfo {
    fn default() -> Self {
        // A4 with 2.54 cm margins.
        PageInfo {
            w_pt: 595.0,
            h_pt: 842.0,
            margin_l: 72.0,
            margin_r: 72.0,
            margin_t: 72.0,
            margin_b: 72.0,
        }
    }
}

#[derive(Default)]
pub struct Document {
    pub blocks: Vec<Block>,
    /// ZIP-internal path -> image bytes.
    pub images: HashMap<String, Vec<u8>>,
    pub title: String,
    pub page: PageInfo,
    /// The file carries the page breaks of its last layout (Word `lastRenderedPageBreak`).
    pub layout_pages: bool,
}
