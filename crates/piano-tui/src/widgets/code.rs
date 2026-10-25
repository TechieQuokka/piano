//! Code area: line numbers, alert gutter, playing-line highlight, and soft wrap.
//!
//! Long lines wrap on screen only; the text and its line numbers are unchanged
//! (playback ranges use line numbers). Continuation rows show `↪`.

use piano_app::App;
use piano_core::Severity;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

use crate::theme;

/// Width of `  12 ⚠ │ `.
pub const GUTTER: usize = 9;

/// Char ranges `[start, end)` of each screen row of `text` at `width` columns.
fn segments(text: &str, width: usize) -> Vec<(usize, usize)> {
    let width = width.max(1);
    let mut segs = Vec::new();
    let (mut start, mut w) = (0, 0);
    for (i, c) in text.chars().enumerate() {
        let cw = c.width().unwrap_or(1);
        if w + cw > width && i > start {
            segs.push((start, i));
            start = i;
            w = 0;
        }
        w += cw;
    }
    segs.push((start, text.chars().count()));
    segs
}

/// Screen (row within the line, column) of char column `col`.
fn locate(text: &str, segs: &[(usize, usize)], col: usize, width: usize) -> (usize, usize) {
    let row = segs
        .iter()
        .position(|&(s, e)| col >= s && col < e)
        .unwrap_or(segs.len() - 1);
    let (s, _) = segs[row];
    let x: usize = text
        .chars()
        .skip(s)
        .take(col - s)
        .map(|c| c.width().unwrap_or(1))
        .sum();
    if x >= width.max(1) {
        (row + 1, 0)
    } else {
        (row, x)
    }
}

/// Adjusts the buffer's first visible line so the cursor row is on screen.
pub fn scroll_to_cursor(app: &mut App, width: usize, height: usize) {
    let height = height.max(1);
    let b = app.buf_mut();
    if b.cursor.line < b.scroll {
        b.scroll = b.cursor.line;
        return;
    }
    let cursor_rows = |b: &piano_app::buffer::Buffer, from: usize| -> usize {
        let before: usize = (from..b.cursor.line)
            .map(|i| segments(&b.lines[i], width).len())
            .sum();
        let text = &b.lines[b.cursor.line];
        let (row, _) = locate(text, &segments(text, width), b.cursor.col, width);
        before + row + 1
    };
    while b.scroll < b.cursor.line && cursor_rows(b, b.scroll) > height {
        b.scroll += 1;
    }
}

/// Visible rows, and the cursor's (row, column) on screen if visible.
pub fn lines(
    app: &App,
    width: usize,
    height: usize,
) -> (Vec<Line<'static>>, Option<(usize, usize)>) {
    let b = app.buf();
    let playing = app.playing_line();
    let mut worst: Vec<Option<Severity>> = vec![None; b.lines.len()];
    for a in &b.compiled.alerts {
        if let Some(w) = worst.get_mut(a.span.line as usize - 1) {
            *w = (*w).max(Some(a.severity));
        }
    }
    let mut rows = Vec::new();
    let mut cursor = None;
    for (i, text) in b.lines.iter().enumerate().skip(b.scroll) {
        if rows.len() >= height {
            break;
        }
        let segs = segments(text, width);
        if i == b.cursor.line {
            let (r, x) = locate(text, &segs, b.cursor.col, width);
            cursor = Some((rows.len() + r, x));
        }
        let chars: Vec<char> = text.chars().collect();
        for (k, &(s, e)) in segs.iter().enumerate() {
            if rows.len() >= height {
                break;
            }
            let gutter = if k == 0 {
                let marker = worst[i].map_or(Span::raw(" "), |s| {
                    Span::styled(s.marker().to_string(), theme::severity(s))
                });
                let num = if i == b.cursor.line {
                    Style::new().fg(theme::ACCENT)
                } else {
                    Style::new().fg(theme::DIM)
                };
                vec![Span::styled(format!("{:>4} ", i + 1), num), marker]
            } else {
                vec![Span::styled("   ↪  ", Style::new().fg(theme::DIM))]
            };
            let mut spans = gutter;
            spans.push(Span::styled(" │ ", Style::new().fg(theme::DIM)));
            spans.push(Span::raw(chars[s..e].iter().collect::<String>()));
            let mut line = Line::from(spans);
            if playing == Some(i as u32 + 1) {
                line = line.style(theme::playing_line());
            }
            rows.push(line);
        }
    }
    let cursor = cursor.filter(|(r, _)| *r < height);
    (rows, cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_by_width() {
        assert_eq!(segments("", 4), [(0, 0)]);
        assert_eq!(segments("abcdefghij", 4), [(0, 4), (4, 8), (8, 10)]);
        // Wide characters take two columns.
        assert_eq!(segments("도레미", 4), [(0, 2), (2, 3)]);
    }

    #[test]
    fn cursor_location() {
        let t = "abcdefghij";
        let s = segments(t, 4);
        assert_eq!(locate(t, &s, 0, 4), (0, 0));
        assert_eq!(locate(t, &s, 5, 4), (1, 1));
        assert_eq!(locate(t, &s, 10, 4), (2, 2));
        let t = "abcd";
        let s = segments(t, 4);
        // At the end of a full row the cursor goes to the next row.
        assert_eq!(locate(t, &s, 4, 4), (1, 0));
    }
}
