//! Plain text and basic Markdown (only line-leading # headings).
use crate::doc::*;

pub fn parse(bytes: &[u8], markdown: bool) -> Document {
    let s = String::from_utf8_lossy(bytes);
    let mut blocks = Vec::new();
    for line in s.lines() {
        let mut level = 0u8;
        let mut text = line.trim_end();
        if markdown {
            let hashes = text.chars().take_while(|c| *c == '#').count();
            if (1..=6).contains(&hashes) && text[hashes..].starts_with(' ') {
                level = hashes as u8;
                text = text[hashes..].trim();
            }
        }
        let size = match level {
            0 => 11.0,
            1 => 20.0,
            2 => 16.0,
            3 => 14.0,
            _ => 12.0,
        };
        let mut p = Para {
            heading: level,
            default_pt: 11.0,
            ..Default::default()
        };
        if !text.is_empty() {
            p.inlines.push(Inline::Text(Run {
                text: text.replace('\t', "    "),
                bold: level > 0,
                size_pt: size,
                ..Default::default()
            }));
        }
        blocks.push(Block::Para(p));
    }
    Document {
        blocks,
        ..Default::default()
    }
}
