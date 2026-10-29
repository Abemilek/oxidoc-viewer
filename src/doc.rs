//! Modelo interno de documento (independiente del formato de origen).
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
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, Default)]
pub struct Table {
    pub rows: Vec<Vec<Cell>>,
}

#[derive(Clone, Debug)]
pub enum Block {
    Para(Para),
    Table(Table),
}

#[derive(Default)]
pub struct Document {
    pub blocks: Vec<Block>,
    /// ruta interna del zip -> bytes de la imagen
    pub images: HashMap<String, Vec<u8>>,
    pub title: String,
}
