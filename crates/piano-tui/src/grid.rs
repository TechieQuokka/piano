//! A fixed-width cell grid that knows about wide (CJK) characters.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

#[derive(Clone)]
enum Cell {
    Char(char, Style),
    /// Second column of a wide character.
    Cont,
}

pub struct Grid {
    cells: Vec<Cell>,
}

impl Grid {
    pub fn new(width: usize, style: Style) -> Grid {
        Grid {
            cells: vec![Cell::Char(' ', style); width],
        }
    }

    /// Writes `text` from column `x`; returns the column after it.
    pub fn put(&mut self, x: usize, text: &str, style: Style) -> usize {
        let mut x = x;
        for c in text.chars() {
            let w = c.width().unwrap_or(1).max(1);
            if x + w > self.cells.len() {
                break;
            }
            self.cells[x] = Cell::Char(c, style);
            if w == 2 {
                self.cells[x + 1] = Cell::Cont;
            }
            x += w;
        }
        x
    }

    pub fn into_line(self) -> Line<'static> {
        let mut spans: Vec<Span> = Vec::new();
        let mut cur = String::new();
        let mut cur_style = None;
        for cell in self.cells {
            let Cell::Char(c, s) = cell else { continue };
            if cur_style != Some(s) {
                if let Some(st) = cur_style {
                    spans.push(Span::styled(std::mem::take(&mut cur), st));
                }
                cur_style = Some(s);
            }
            cur.push(c);
        }
        if let Some(st) = cur_style {
            spans.push(Span::styled(cur, st));
        }
        Line::from(spans)
    }
}

/// Display width of a string.
pub fn width(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}

/// Truncates `s` to at most `max` columns.
pub fn truncate(s: &str, max: usize) -> String {
    let mut out = String::new();
    let mut w = 0;
    for c in s.chars() {
        let cw = c.width().unwrap_or(1);
        if w + cw > max {
            break;
        }
        out.push(c);
        w += cw;
    }
    out
}

/// Column where continuation lines start: after `  Field   ` when the line has a
/// field name followed by two or more spaces, else its leading spaces.
fn hanging_indent(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let lead = chars.iter().take_while(|c| **c == ' ').count();
    let word_end = lead + chars[lead..].iter().take_while(|c| **c != ' ').count();
    let gap = chars[word_end..].iter().take_while(|c| **c == ' ').count();
    if word_end > lead && gap >= 2 && word_end + gap < chars.len() {
        chars[..word_end + gap]
            .iter()
            .map(|c| c.width().unwrap_or(1))
            .sum()
    } else {
        lead
    }
}

/// Wraps `text` to `width` columns, preferring spaces as break points.
/// Continuation lines keep the original indentation (so `Field  value` blocks stay
/// aligned). Always returns at least one line.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(4);
    if self::width(text) <= width {
        return vec![text.to_string()];
    }
    let indent = " ".repeat(hanging_indent(text));
    let indent = if indent.len() + 8 < width {
        indent
    } else {
        String::new()
    };
    let mut lines = Vec::new();
    let mut rest: Vec<char> = text.chars().collect();
    let mut first = true;
    while !rest.is_empty() {
        let prefix = if first { String::new() } else { indent.clone() };
        let room = width - self::width(&prefix);
        // Longest prefix of `rest` that fits.
        let mut w = 0;
        let mut fit = 0;
        for c in &rest {
            let cw = c.width().unwrap_or(1);
            if w + cw > room {
                break;
            }
            w += cw;
            fit += 1;
        }
        let cut = if fit == rest.len() || rest[fit] == ' ' {
            // Everything fits, or the row ends exactly at a word boundary.
            fit
        } else {
            match rest[..fit].iter().rposition(|c| *c == ' ') {
                Some(sp) if sp > 0 => sp,
                _ => fit.max(1),
            }
        };
        let piece: String = rest[..cut].iter().collect();
        lines.push(format!(
            "{prefix}{}",
            if first {
                piece.as_str()
            } else {
                piece.trim_start()
            }
        ));
        rest.drain(..cut);
        while rest.first() == Some(&' ') {
            rest.remove(0);
        }
        first = false;
    }
    lines
}

#[cfg(test)]
mod tests {
    #[test]
    fn wrapping_keeps_indent_and_words() {
        let lines = super::wrap("  Omitted  nearest to the previous note", 22);
        assert_eq!(
            lines,
            [
                "  Omitted  nearest to",
                "           the",
                "           previous",
                "           note"
            ]
        );
        let plain = super::wrap("  plain text that is long", 12);
        assert_eq!(plain, ["  plain text", "  that is", "  long"]);
        assert_eq!(super::wrap("short", 20), ["short"]);
        assert!(
            super::wrap("가나다라마바사", 6)
                .iter()
                .all(|l| super::width(l) <= 6)
        );
    }

    use super::*;

    #[test]
    fn wide_chars_take_two_columns() {
        let mut g = Grid::new(6, Style::new());
        let end = g.put(1, "도x", Style::new());
        assert_eq!(end, 4);
        assert_eq!(g.into_line().to_string(), " 도x  ");
    }

    #[test]
    fn truncation() {
        assert_eq!(truncate("가나다", 5), "가나");
        assert_eq!(width("도 C"), 4);
    }
}
