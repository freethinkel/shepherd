//! `#{window_layout}`, tmux's split tree: `<checksum>,<w>x<h>,<x>,<y>` then `,<pane>`, or `{…}` for panes side by side, or
//! `[…]` for stacked ones. A split there has any number of sides, and a split of Shepherd's has two, so `{a,b,c}` reads
//! as `a` beside `(b beside c)`.
use crate::model::{LayoutPane, LayoutSplit, Rect};

#[derive(Debug, PartialEq)]
pub(super) enum Node {
    Pane(String, Rect),
    Split { across: bool, rect: Rect, sides: Vec<Node> },
}

/// The tree of a window's layout.
pub(super) fn parse(layout: &str) -> Option<Node> {
    let (_checksum, mut rest) = layout.split_once(',')?;
    let node = node(&mut rest)?;
    rest.is_empty().then_some(node)
}

/// The number at the front of `text`.
fn number(text: &mut &str) -> Option<u16> {
    let end = text.find(|c: char| !c.is_ascii_digit()).unwrap_or(text.len());
    let (digits, rest) = text.split_at(end);
    *text = rest;
    digits.parse().ok()
}

/// The `expected` character at the front of `text`.
fn eat(text: &mut &str, expected: char) -> Option<()> {
    *text = text.strip_prefix(expected)?;
    Some(())
}

fn node(text: &mut &str) -> Option<Node> {
    let width = number(text)?;
    eat(text, 'x')?;
    let height = number(text)?;
    eat(text, ',')?;
    let x = number(text)?;
    eat(text, ',')?;
    let y = number(text)?;
    let rect = Rect { x, y, width, height };
    let (across, close) = match text.chars().next()? {
        ',' => {
            eat(text, ',')?;
            return Some(Node::Pane(format!("%{}", number(text)?), rect));
        }
        '{' => (true, '}'),
        '[' => (false, ']'),
        _ => return None,
    };
    *text = &text[1..];
    let mut sides = vec![node(text)?];
    while eat(text, ',').is_some() {
        sides.push(node(text)?);
    }
    eat(text, close)?;
    Some(Node::Split { across, rect, sides })
}

impl Node {
    fn rect(&self) -> Rect {
        match self {
            Node::Pane(_, rect) | Node::Split { rect, .. } => *rect,
        }
    }

    /// Every pane under this, in order.
    pub(super) fn panes(&self) -> Vec<(String, Rect)> {
        match self {
            Node::Pane(pane, rect) => vec![(pane.clone(), *rect)],
            Node::Split { sides, .. } => sides.iter().flat_map(Node::panes).collect(),
        }
    }

    /// As the window lays panes out, `focused` marked.
    pub(super) fn laid(&self, focused: &str) -> Vec<LayoutPane> {
        self.panes().into_iter().map(|(pane_id, rect)| LayoutPane { focused: pane_id == focused, pane_id, rect }).collect()
    }

    /// Its splits, two sides each. A split's id is its path, which [`crate::model::split_path`] reads back.
    pub(super) fn splits(&self) -> Vec<LayoutSplit> {
        let mut out = Vec::new();
        self.walk("", &mut out);
        out
    }

    fn walk(&self, path: &str, out: &mut Vec<LayoutSplit>) {
        if let Node::Split { across, sides, .. } = self {
            two_sides(*across, sides, path, out);
        }
    }

    /// The split at `path`, as [`crate::commands::Command::SetSplitRatio`] names it.
    pub(super) fn border(&self, path: &[bool]) -> Option<Border> {
        let Node::Split { across, sides, .. } = self else { return None };
        border(*across, sides, path)
    }
}

/// The split between the first of `sides` and the rest, then the ones within each.
fn two_sides(across: bool, sides: &[Node], path: &str, out: &mut Vec<LayoutSplit>) {
    let [first, rest @ ..] = sides else { return };
    if rest.is_empty() {
        return first.walk(path, out);
    }
    let (start, whole) = (first.rect(), reach(sides));
    let size = |r: Rect| if across { r.width } else { r.height };
    out.push(LayoutSplit {
        id: format!("split_0_{}", if path.is_empty() { "root" } else { path }),
        direction: if across { "right" } else { "down" }.into(),
        ratio: f32::from(size(start)) / f32::from(size(whole)),
        rect: whole,
    });
    first.walk(&format!("{path}0"), out);
    two_sides(across, rest, &format!("{path}1"), out);
}

/// The rectangle `sides` fill.
fn reach(sides: &[Node]) -> Rect {
    let (first, last) = (sides[0].rect(), sides[sides.len() - 1].rect());
    Rect { x: first.x, y: first.y, width: last.x + last.width - first.x, height: last.y + last.height - first.y }
}

/// A split's border as `resize-pane` moves it.
#[derive(Debug, PartialEq)]
pub(super) struct Border {
    /// The pane that ends the first side: its right or bottom edge is the border.
    pub(super) pane: String,
    pub(super) across: bool,
    /// The size of the first side, and of both.
    pub(super) first: u16,
    pub(super) whole: u16,
}

fn border(across: bool, sides: &[Node], path: &[bool]) -> Option<Border> {
    let [first, rest @ ..] = sides else { return None };
    if rest.is_empty() {
        return first.border(path);
    }
    let size = |r: Rect| if across { r.width } else { r.height };
    match path.split_first() {
        None => Some(Border { pane: first.panes().pop()?.0, across, first: size(first.rect()), whole: size(reach(sides)) }),
        Some((false, inner)) => first.border(inner),
        Some((true, inner)) => border(across, rest, inner),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const THREE: &str = "abcd,120x40,0,0{40x40,0,0,1,39x40,41,0[39x20,41,0,2,39x19,41,21,3],39x40,81,0,4}";

    #[test]
    fn a_layout_reads_as_its_tree() {
        assert_eq!(parse("a87d,100x30,0,0,7"), Some(Node::Pane("%7".into(), Rect { x: 0, y: 0, width: 100, height: 30 })));
        let root = parse(THREE).unwrap();
        let panes: Vec<_> = root.panes().into_iter().map(|(p, r)| (p, r.x, r.y, r.width, r.height)).collect();
        assert_eq!(panes, [("%1".into(), 0, 0, 40, 40), ("%2".into(), 41, 0, 39, 20), ("%3".into(), 41, 21, 39, 19), ("%4".into(), 81, 0, 39, 40)]);
        assert_eq!(parse("abcd,100x30,0,0{"), None);
        assert_eq!(parse("abcd,100x30,0,0,7 trailing"), None);
    }

    #[test]
    fn a_split_of_three_reads_as_two_of_two() {
        let splits = parse(THREE).unwrap().splits();
        let read: Vec<_> = splits.iter().map(|s| (s.id.as_str(), s.direction.as_str(), s.rect.x, s.rect.width)).collect();
        assert_eq!(read, [("split_0_root", "right", 0, 120), ("split_0_1", "right", 41, 79), ("split_0_10", "down", 41, 39)]);
        assert_eq!(splits[0].ratio, 40. / 120.);
        assert!(splits.iter().all(|s| crate::model::split_path(&s.id).is_some()));
    }

    #[test]
    fn a_border_is_found_by_its_path() {
        let root = parse(THREE).unwrap();
        assert_eq!(root.border(&[]), Some(Border { pane: "%1".into(), across: true, first: 40, whole: 120 }));
        assert_eq!(root.border(&[true]), Some(Border { pane: "%3".into(), across: true, first: 39, whole: 79 }), "the pane at the first side's end");
        assert_eq!(root.border(&[true, false]), Some(Border { pane: "%2".into(), across: false, first: 20, whole: 40 }));
        assert_eq!(root.border(&[false]), None, "a pane has no border");
    }
}
