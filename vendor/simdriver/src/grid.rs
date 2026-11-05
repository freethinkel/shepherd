//! A named grid over the screen, for targeting what accessibility does not expose
//! (maps, canvases, games). Rows are letters down the screen — A, B, C, D, F… as many as
//! the height needs for roughly square cells (…Z, AA, AB past 25) — and columns are
//! numbers across: "C4" is the third row, fourth column. Rows skip E, so no cell name
//! reads like an element ref (`e4`) in any capitalisation. Coordinates are 0..1 of the
//! screen, the same space as [`crate::Driver::pointer`].

/// Row letters: the alphabet without E.
const LETTERS: &[u8; 25] = b"ABCDFGHIJKLMNOPQRSTUVWXYZ";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub rows: u32,
    pub cols: u32,
}

impl Grid {
    pub const DEFAULT_COLS: u32 = 6;

    /// A grid with `cols` columns (at least 1) over a screen of `screen_pt` points,
    /// with as many rows as keep the cells closest to square.
    pub fn new(screen_pt: (f64, f64), cols: u32) -> Grid {
        let cols = cols.max(1);
        let (w, h) = screen_pt;
        let rows = if w > 0.0 && h > 0.0 {
            (h / (w / f64::from(cols))).round().max(1.0) as u32
        } else {
            1
        };
        Grid { rows, cols }
    }

    /// The name of the cell at `row`, `col` (both from 0), e.g. (2, 3) → "C4".
    pub fn name(&self, row: u32, col: u32) -> String {
        format!("{}{}", row_letters(row), col + 1)
    }

    /// The centre of a named cell in 0..1 of the screen; `None` for a name outside the
    /// grid. Case-insensitive.
    pub fn center(&self, name: &str) -> Option<(f64, f64)> {
        let [x, y, w, h] = self.rect(name)?;
        Some((x + w / 2.0, y + h / 2.0))
    }

    /// A named cell as `[x, y, width, height]` in 0..1 of the screen (for drawing it).
    pub fn rect(&self, name: &str) -> Option<[f64; 4]> {
        let (row, col) = self.parse(name)?;
        let (cw, ch) = (1.0 / f64::from(self.cols), 1.0 / f64::from(self.rows));
        Some([f64::from(col) * cw, f64::from(row) * ch, cw, ch])
    }

    /// The name of the cell containing (`x`, `y`) in 0..1 of the screen (clamped).
    pub fn at(&self, x: f64, y: f64) -> String {
        let pick = |v: f64, n: u32| ((v.clamp(0.0, 1.0) * f64::from(n)) as u32).min(n - 1);
        self.name(pick(y, self.rows), pick(x, self.cols))
    }

    fn parse(&self, name: &str) -> Option<(u32, u32)> {
        let name = name.trim();
        let split = name.find(|c: char| c.is_ascii_digit())?;
        let (letters, digits) = name.split_at(split);
        if letters.is_empty() || !letters.chars().all(|c| c.is_ascii_alphabetic()) {
            return None;
        }
        // Bijective base 25 over LETTERS: A=1 … Z=25, AA=26.
        let mut row: u64 = 0;
        for c in letters.bytes() {
            let digit = LETTERS.iter().position(|&l| l == c.to_ascii_uppercase())?;
            row = row * 25 + digit as u64 + 1;
            if row > u64::from(u32::MAX) {
                return None;
            }
        }
        let col: u32 = digits.parse().ok()?;
        let (row, col) = (u32::try_from(row - 1).ok()?, col.checked_sub(1)?);
        (row < self.rows && col < self.cols).then_some((row, col))
    }
}

fn row_letters(mut row: u32) -> String {
    let mut out = Vec::new();
    loop {
        out.push(LETTERS[(row % 25) as usize]);
        if row < 25 {
            break;
        }
        row = row / 25 - 1;
    }
    out.reverse();
    String::from_utf8(out).expect("ascii")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iphone_grid_is_six_by_thirteen() {
        let g = Grid::new((402.0, 874.0), Grid::DEFAULT_COLS);
        assert_eq!(g, Grid { rows: 13, cols: 6 });
        assert_eq!(g.name(0, 0), "A1");
        assert_eq!(g.name(4, 0), "F1", "no row E");
        assert_eq!(g.name(12, 5), "N6");
        assert_eq!(g.center("a1"), Some((1.0 / 12.0, 1.0 / 26.0)));
        assert_eq!(g.center("N6"), Some((11.0 / 12.0, 25.0 / 26.0)));
        assert_eq!(g.center("E1"), None, "E is never a row");
        assert_eq!(g.center("e4"), None);
        assert_eq!(g.center("O1"), None, "row past the grid");
        assert_eq!(g.center("A7"), None, "column past the grid");
        assert_eq!(g.center("A0"), None);
        assert_eq!(g.center("1A"), None);
        assert_eq!(g.center(""), None);
        assert_eq!(g.at(0.0, 0.0), "A1");
        assert_eq!(g.at(1.0, 1.0), "N6");
        assert_eq!(g.at(0.5, 0.5), "H4");
    }

    #[test]
    fn rows_past_z_go_on_as_aa() {
        let g = Grid::new((100.0, 3000.0), 3); // 90 rows
        assert_eq!(g.rows, 90);
        assert_eq!(g.name(24, 0), "Z1");
        assert_eq!(g.name(25, 0), "AA1");
        assert_eq!(g.name(29, 0), "AF1", "no E in later places either");
        assert_eq!(g.name(89, 2), "CP3");
        assert!((0..g.rows).all(|r| !g.name(r, 0).contains('E')));
        for (r, c) in [(0, 0), (24, 1), (25, 2), (49, 0), (50, 1), (89, 2)] {
            let n = g.name(r, c);
            let (x, y) = g.center(&n).unwrap();
            assert_eq!(g.at(x, y), n);
        }
    }

    #[test]
    fn degenerate_screens_still_make_a_grid() {
        assert_eq!(Grid::new((0.0, 0.0), 0), Grid { rows: 1, cols: 1 });
        assert_eq!(Grid::new((400.0, 10.0), 6).rows, 1);
    }
}
