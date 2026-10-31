//! Splits a document into pages.
//!
//! Word files store the page breaks of their last layout (`lastRenderedPageBreak`); when they are
//! present they are used as they are. Otherwise explicit page breaks are honoured and the rest of
//! the content is split with a rough height estimate, so long documents still show as pages.
use crate::doc::*;

/// Returns the index of the first block of every page (strictly increasing, starts with 0).
pub fn split(doc: &Document) -> Vec<usize> {
    let n = doc.blocks.len();
    let estimate = !doc.layout_pages;
    let pg = doc.page;
    let content_h = (pg.h_pt - pg.margin_t - pg.margin_b).max(200.0);
    let text_w = (pg.w_pt - pg.margin_l - pg.margin_r).max(150.0);
    let mut starts: Vec<usize> = vec![0];
    let mut acc = 0.0f32;
    for (i, b) in doc.blocks.iter().enumerate() {
        let (before, after) = match b {
            Block::Para(p) => (p.break_before, p.break_after),
            Block::Table(_) => (false, false),
        };
        if before && i > 0 {
            push_start(&mut starts, i);
            acc = 0.0;
        }
        let h = if estimate { est_height(b, text_w) } else { 0.0 };
        if estimate && acc > 0.0 && acc + h > content_h {
            push_start(&mut starts, i);
            acc = 0.0;
        }
        acc += h;
        if after && i + 1 < n {
            push_start(&mut starts, i + 1);
            acc = 0.0;
        }
    }
    starts
}

fn push_start(starts: &mut Vec<usize>, idx: usize) {
    if starts.last().map_or(true, |l| *l < idx) {
        starts.push(idx);
    }
}

fn est_height(b: &Block, text_w: f32) -> f32 {
    match b {
        Block::Para(p) => {
            let mut chars = 0usize;
            let mut size = if p.default_pt > 0.0 {
                p.default_pt
            } else {
                11.0
            };
            let mut img_h = 0.0f32;
            for i in &p.inlines {
                match i {
                    Inline::Text(r) => {
                        chars += r.text.chars().count();
                        size = size.max(r.size_pt);
                    }
                    Inline::Image { h_pt, .. } => {
                        img_h += if *h_pt > 1.0 { *h_pt } else { 150.0 };
                    }
                    Inline::PageBreak => {}
                }
            }
            let per_line = (text_w / (size * 0.5)).max(8.0);
            let lines = (chars as f32 / per_line).ceil().max(1.0);
            lines * size * 1.25 + img_h + 6.0
        }
        Block::Table(t) => {
            let ncols = t.rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1) as f32;
            let cell_w = text_w / ncols;
            t.rows
                .iter()
                .map(|r| {
                    let tallest = r
                        .iter()
                        .map(|c| c.blocks.iter().map(|b| est_height(b, cell_w)).sum::<f32>())
                        .fold(0.0f32, f32::max);
                    tallest + 8.0
                })
                .sum()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn para(text: &str, before: bool) -> Block {
        Block::Para(Para {
            inlines: vec![Inline::Text(Run {
                text: text.to_string(),
                size_pt: 11.0,
                ..Default::default()
            })],
            default_pt: 11.0,
            break_before: before,
            ..Default::default()
        })
    }

    #[test]
    fn explicit_breaks_are_used_when_layout_is_stored() {
        let doc = Document {
            blocks: vec![
                para("a", false),
                para("b", true),
                para("c", false),
                para("d", true),
            ],
            layout_pages: true,
            ..Default::default()
        };
        assert_eq!(split(&doc), vec![0, 1, 3]);
    }

    #[test]
    fn long_documents_are_split_by_estimate() {
        let blocks: Vec<Block> = (0..400).map(|_| para("Some text", false)).collect();
        let doc = Document {
            blocks,
            ..Default::default()
        };
        let starts = split(&doc);
        assert!(starts.len() > 3);
        assert!(starts.windows(2).all(|w| w[0] < w[1]));
    }
}
