//! Display views: Ref, Output, Help / Man / Tutorial pages, Song.
//! The Display is a tall, narrow column, so long lines are wrapped.

use piano_app::{App, Page};
use piano_core::PedalKind;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::grid::{truncate, wrap, wrap_flow};
use crate::theme;

fn dim() -> Style {
    Style::new().fg(theme::DIM)
}

/// Ref, top to bottom: builtin list, details of the function under the cursor,
/// and the inherited state at the cursor.
pub fn reference(app: &App, width: usize) -> Vec<Line<'static>> {
    let current = app.function_at_cursor();
    let mut out = vec![Line::styled("Functions", theme::heading())];
    for b in piano_lang::BUILTINS {
        let on = current.is_some_and(|c| c.name == b.name);
        let style = if on {
            Style::new().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::new()
        };
        let text = format!("{}{:<8}{}", if on { "▶" } else { " " }, b.name, b.summary);
        out.push(Line::styled(truncate(&text, width), style));
    }
    out.push(Line::default());
    match current {
        Some(f) => {
            out.push(Line::styled(f.signature, theme::heading()));
            for l in wrap(f.detail, width) {
                out.push(Line::raw(l));
            }
            for l in wrap(&format!("e.g. {}", f.example), width) {
                out.push(Line::styled(l, dim()));
            }
        }
        None => {
            for l in wrap(
                "Move the cursor onto a function to see its details (man fn).",
                width,
            ) {
                out.push(Line::styled(l, dim()));
            }
        }
    }
    let s = app.state_at_cursor();
    let (line, _) = app.buf().pos1();
    out.push(Line::default());
    out.push(Line::styled(
        format!("State @ line {line}"),
        theme::heading(),
    ));
    let pedals: Vec<String> = PedalKind::ALL
        .iter()
        .map(|k| format!("{} {}", k.keyword(), if s.pedal(*k) { "on" } else { "off" }))
        .collect();
    for text in [
        format!(
            "ref {} · len {} · {}",
            s.reference_pitch(),
            s.len,
            s.dynamic
        ),
        format!("tempo {} · time {}", s.tempo, s.time),
        pedals.join(" · "),
    ] {
        for l in wrap(&text, width) {
            out.push(Line::raw(l));
        }
    }
    out
}

/// Notes further apart than this start a new row in the Played view.
const PHRASE_GAP_SECS: f64 = 1.2;

/// Played: notes struck in Keys focus, newest last, grouped into rows by pauses,
/// so they can be copied into the code.
pub fn played(app: &App, height: usize, width: usize) -> Vec<Line<'static>> {
    let mut phrases: Vec<Vec<String>> = Vec::new();
    let mut last = None;
    for (pitch, at) in &app.played {
        let new_phrase = last.is_none_or(|t: std::time::Instant| {
            at.duration_since(t).as_secs_f64() > PHRASE_GAP_SECS
        });
        if new_phrase {
            phrases.push(Vec::new());
        }
        if let Some(p) = phrases.last_mut() {
            p.push(format!("{pitch} {}", app.solfege(*pitch)));
        }
        last = Some(*at);
    }
    let mut rows: Vec<Line<'static>> = Vec::new();
    for phrase in &phrases {
        for l in wrap(&phrase.join("  "), width) {
            rows.push(Line::raw(l));
        }
    }
    let mut out = Vec::new();
    if rows.is_empty() {
        for l in wrap(
            "Press piano keys (Z X C …, Q W E …) to listen. Played notes appear here, \
             newest last, so you can write them in the code (F3).",
            width,
        ) {
            out.push(Line::styled(l, dim()));
        }
        return out;
    }
    let body = height.saturating_sub(1);
    let start = rows.len().saturating_sub(body + app.display_scroll);
    out.extend(rows.into_iter().skip(start).take(body));
    out.push(Line::styled("newest last · clear played", dim()));
    out
}

/// Output: the newest (wrapped) rows that fit, scrolled up by `display_scroll` lines.
pub fn output(app: &App, height: usize, width: usize) -> Vec<Line<'static>> {
    let lines = &app.output.lines;
    let end = lines.len().saturating_sub(app.display_scroll);
    // Wrap from the end backwards until the area is full.
    let mut rows: Vec<Line<'static>> = Vec::new();
    for l in lines[..end].iter().rev() {
        let style = theme::out(l.kind);
        let wrapped = if l.list {
            wrap_flow(&l.text, width)
        } else {
            wrap(&l.text, width)
        };
        for w in wrapped.into_iter().rev() {
            rows.push(Line::styled(w, style));
        }
        if rows.len() >= height {
            break;
        }
    }
    rows.truncate(height);
    rows.reverse();
    rows
}

fn page_style(l: &str) -> Style {
    let t = l.trim_start();
    if ["Try", "Example", "Type ", "Line ", "Piece "]
        .iter()
        .any(|p| t.starts_with(p))
    {
        Style::new().fg(theme::ACCENT)
    } else if t.starts_with("Check") {
        Style::new().fg(if t.contains('✔') {
            Color::Green
        } else {
            Color::Yellow
        })
    } else if !t.is_empty()
        && t == l
        && t.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && t.chars().all(|c| !c.is_lowercase())
    {
        theme::heading()
    } else {
        Style::new()
    }
}

/// A text page (Help, Man, Tutorial), wrapped and scrolled by `display_scroll` rows.
pub fn page(app: &App, page: &Page, height: usize, width: usize) -> Vec<Line<'static>> {
    let rows: Vec<Line<'static>> = page
        .lines
        .iter()
        .flat_map(|l| {
            let style = page_style(l);
            wrap(l, width)
                .into_iter()
                .map(move |w| Line::styled(w, style))
        })
        .collect();
    let body = height.saturating_sub(1);
    let max_scroll = rows.len().saturating_sub(body);
    let scroll = app.display_scroll.min(max_scroll);
    let mut out = vec![Line::styled(truncate(&page.title, width), theme::heading())];
    let more = scroll < max_scroll;
    let take = if more { body.saturating_sub(1) } else { body };
    out.extend(rows.into_iter().skip(scroll).take(take));
    if more {
        out.push(Line::styled("… Shift+PgDn for more", dim()));
    }
    out
}

/// Song: pieces in the project and the arrangement.
pub fn song(app: &App, width: usize) -> Vec<Line<'static>> {
    let mut out = vec![Line::styled("Pieces", theme::heading())];
    let mut row: Vec<Span<'static>> = Vec::new();
    let mut used = 0;
    for p in app.project.pieces() {
        let b = app.buffers.iter().find(|b| b.name == p);
        let mark = match b {
            Some(b) if b.dirty => "*●",
            Some(_) => "*",
            None => "",
        };
        let text = format!("{p}{mark}");
        let w = crate::grid::width(&text) + 2;
        if used + w > width && !row.is_empty() {
            out.push(Line::from(std::mem::take(&mut row)));
            used = 0;
        }
        let style = if app.buf().name == p {
            Style::new().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::new()
        };
        row.push(Span::styled(text, style));
        row.push(Span::raw("  "));
        used += w;
    }
    out.push(Line::from(row));
    for l in wrap("* open  ● unsaved · open <piece> · new <name>", width) {
        out.push(Line::styled(l, dim()));
    }
    out.push(Line::default());
    out.push(Line::styled("song.piano", theme::heading()));
    match app.piece_source(piano_project::SONG) {
        Some(src) => {
            for l in src.lines().take(8) {
                for w in wrap(&format!("  {l}"), width) {
                    out.push(Line::raw(w));
                }
            }
            if let Some((score, compiled)) = app.score_of(piano_project::SONG) {
                out.push(Line::styled(
                    format!(
                        "  {} notes · {} · {} alerts",
                        score.notes.len(),
                        piano_app::app::clock(score.duration_secs()),
                        compiled.alerts.len()
                    ),
                    dim(),
                ));
            }
        }
        None => out.push(Line::styled("  (none) — new song", dim())),
    }
    out
}
