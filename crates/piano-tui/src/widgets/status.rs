//! Status bar.

use piano_app::App;
use piano_core::{PedalKind, Pitch};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme;

pub fn line(app: &App) -> Line<'static> {
    let state = app.state_at_cursor();
    let lo = Pitch::fold((app.view_octave() + 1) * 12).0;
    let hi = Pitch::fold((app.view_octave() + 1) * 12 + 23).0;
    let pedals: Vec<&str> = PedalKind::ALL
        .iter()
        .filter(|k| state.pedal(**k))
        .map(|k| k.keyword())
        .collect();
    let b = app.buf();
    let loudness = app.input.dynamic.unwrap_or_default();
    let mut spans = vec![
        Span::styled(
            format!(" [{}] ", app.focus.label()),
            Style::new()
                .fg(ratatui::style::Color::Black)
                .bg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " keys {lo}-{hi} ({loudness}) · pedal {} · {}{}{} · {} · {}",
            if pedals.is_empty() {
                "off".to_string()
            } else {
                pedals.join("+")
            },
            b.name,
            if b.dirty { "*" } else { "" },
            if b.readonly { " (read-only)" } else { "" },
            app.project.name(),
            app.lang.code(),
        )),
    ];
    if app.playing.is_some() {
        spans.push(Span::styled(
            "  ▶ playing (F6 stops)",
            Style::new().fg(theme::ACCENT),
        ));
    }
    Line::from(spans)
}
