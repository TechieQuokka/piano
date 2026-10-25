//! The always-visible keyboard: the 88-key map and a zoomed keyboard that grows
//! with the terminal width. Only the playable two octaves carry PC key labels;
//! the octaves around them are drawn dimmed for orientation.

use piano_app::App;
use piano_app::input::key_label;
use piano_core::Pitch;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;

use crate::grid::Grid;
use crate::theme;

/// Rows of the full keyboard: map, gap, and 5 key rows.
pub const FULL_ROWS: u16 = 7;
const MIN_CELL: usize = 4;
const MAX_CELL: usize = 6;
/// Width of the map line: `A0 ` + 88 keys + ` C8`.
const MAP_WIDTH: usize = 88 + 6;

fn is_pressed(app: &App, p: Pitch) -> bool {
    app.pressed.iter().any(|(q, _)| *q == p)
        || app.held_pitches().contains(&p)
}

/// MIDI range playable with the PC keys (two octaves from C of the input octave).
fn playable(app: &App) -> (i32, i32) {
    let lo = (app.input.octave + 1) * 12;
    (lo, lo + 23)
}

fn is_black(midi: i32) -> bool {
    matches!(midi.rem_euclid(12), 1 | 3 | 6 | 8 | 10)
}

pub fn lines(app: &App, width: usize, compact: bool) -> Vec<Line<'static>> {
    let mut out = vec![map_line(app, width)];
    if !compact {
        out.push(Line::default());
        out.extend(zoomed(app, width));
    }
    out
}

/// `A0 ████▄█▄██ … C8` with the playable range highlighted.
fn map_line(app: &App, width: usize) -> Line<'static> {
    let (lo, hi) = playable(app);
    let label = |p: i32| Pitch::new(p).map_or_else(|| "--".to_string(), |p| p.to_string());
    if width < MAP_WIDTH {
        return Line::styled(
            format!("keys {} - {}  (of A0 - C8)", label(lo), label(hi)),
            Style::new().fg(theme::ACCENT),
        );
    }
    let mut g = Grid::new(MAP_WIDTH, Style::new());
    g.put(0, "A0 ", Style::new().fg(theme::DIM));
    for midi in 21..=108 {
        let p = Pitch::new(midi).expect("piano range");
        let in_range = (lo..=hi).contains(&midi);
        let (ch, mut style) = if p.is_black() {
            (
                '▄',
                Style::new().fg(if in_range {
                    Color::Cyan
                } else {
                    Color::DarkGray
                }),
            )
        } else {
            (
                '█',
                Style::new().fg(if in_range { Color::White } else { Color::Gray }),
            )
        };
        if in_range {
            style = style.add_modifier(Modifier::BOLD);
        }
        if is_pressed(app, p) {
            style = Style::new().fg(Color::Yellow);
        }
        g.put(3 + p.key_index(), &ch.to_string(), style);
    }
    g.put(3 + 88, " C8", Style::new().fg(theme::DIM));
    g.into_line()
}

/// White keys (MIDI numbers) to draw: the playable ones, widened by whole octaves
/// on both sides while they fit, never beyond A0..C8.
fn visible_whites(app: &App, width: usize) -> Vec<i32> {
    let whites: Vec<i32> = (21..=108).filter(|m| !is_black(*m)).collect();
    let (lo, hi) = playable(app);
    let (mut a, mut b) = match (
        whites.iter().position(|m| *m >= lo),
        whites.iter().rposition(|m| *m <= hi),
    ) {
        (Some(a), Some(b)) if a <= b => (a, b),
        _ => return whites[whites.len() - 7..].to_vec(),
    };
    let max = (width.saturating_sub(1) / MIN_CELL).max(1);
    loop {
        let mut grew = false;
        if a > 0 && b - a + 1 + 7.min(a) <= max {
            a -= 7.min(a);
            grew = true;
        }
        if b + 1 < whites.len() && b - a + 1 + 7.min(whites.len() - 1 - b) <= max {
            b += 7.min(whites.len() - 1 - b);
            grew = true;
        }
        if !grew {
            break;
        }
    }
    whites[a..=b].to_vec()
}

fn zoomed(app: &App, width: usize) -> Vec<Line<'static>> {
    let (lo, hi) = playable(app);
    let whites = visible_whites(app, width);
    let cell = (width.saturating_sub(1) / whites.len().max(1)).clamp(MIN_CELL, MAX_CELL);
    let inner = cell - 1;
    let total = whites.len() * cell + 1;
    let left = width.saturating_sub(total) / 2;
    let mut rows: Vec<Grid> = (0..5)
        .map(|_| Grid::new(width.max(left + total), Style::new()))
        .collect();
    let octave = app.input.octave;
    let border = Style::new().fg(theme::DIM);
    let center = |s: &str| format!("{s:^inner$}");

    for (i, &w) in whites.iter().enumerate() {
        let x = left + i * cell;
        let p = Pitch::new(w).expect("white key in range");
        let live = (lo..=hi).contains(&w);
        let style = if is_pressed(app, p) {
            theme::pressed()
        } else if live {
            theme::white_key()
        } else {
            theme::white_key_dim()
        };
        for row in rows.iter_mut().skip(2) {
            row.put(x, "│", border);
            row.put(x + 1, &" ".repeat(inner), style);
        }
        if live {
            let key = key_label(p, octave).map_or(String::new(), |c| c.to_string());
            rows[2].put(x + 1, &center(&key), style.add_modifier(Modifier::BOLD));
            // `도` is two columns wide: center by display width, not chars.
            rows[3].put(x + 1 + inner.saturating_sub(2) / 2, p.solfege(), style);
        }
        let name = if p.class() == 0 {
            p.to_string()
        } else if live {
            p.letter_name().to_string()
        } else {
            String::new()
        };
        rows[4].put(x + 1, &center(&name), style);

        // Black key after this white key, centered on the border to its right.
        let black = w + 1;
        if i + 1 < whites.len() && is_black(black) {
            let bx = x + cell - 1;
            let bp = Pitch::new(black).expect("black key in range");
            let blive = (lo..=hi).contains(&black);
            let bstyle = if is_pressed(app, bp) {
                theme::pressed()
            } else if blive {
                theme::black_key()
            } else {
                theme::black_key_dim()
            };
            let key = if blive {
                key_label(bp, octave).unwrap_or(' ')
            } else {
                ' '
            };
            rows[0].put(bx, &format!(" {key} "), bstyle.add_modifier(Modifier::BOLD));
            let label = if blive { bp.letter_name() } else { "" };
            rows[1].put(bx, &format!("{label:<3}"), bstyle);
        }
    }
    for row in rows.iter_mut().skip(2) {
        row.put(left + whites.len() * cell, "│", border);
    }
    rows.into_iter().map(Grid::into_line).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> (App, std::path::PathBuf) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-tmp")
            .join(format!("kbd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let project = piano_project::Project::create(&dir).unwrap();
        (App::headless(project), dir)
    }

    #[test]
    fn more_octaves_when_wider() {
        let (mut a, dir) = app();
        let narrow = visible_whites(&a, 60);
        let wide = visible_whites(&a, 200);
        let full = visible_whites(&a, 1000);
        assert_eq!(narrow.len(), 14);
        assert!(wide.len() > narrow.len());
        // Wide enough: the whole piano (52 white keys), from A0 to C8.
        assert_eq!(full.len(), 52);
        assert_eq!((full[0], full[51]), (21, 108));
        // The playable range is always inside.
        a.shift_octave(3);
        let w = visible_whites(&a, 120);
        assert!(w.contains(&((a.input.octave + 1) * 12)));
        let _ = std::fs::remove_dir_all(dir);
    }
}
