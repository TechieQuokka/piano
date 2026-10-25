//! Colors and styles in one place.

use piano_app::OutKind;
use piano_core::Severity;
use ratatui::style::{Color, Modifier, Style};

pub const ACCENT: Color = Color::Cyan;
pub const DIM: Color = Color::DarkGray;

pub fn title(focused: bool) -> Style {
    if focused {
        Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(Color::Gray)
    }
}

pub fn border(focused: bool) -> Style {
    if focused {
        Style::new().fg(ACCENT)
    } else {
        Style::new().fg(DIM)
    }
}

pub fn severity(s: Severity) -> Style {
    match s {
        Severity::Error => Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        Severity::Warning => Style::new().fg(Color::Yellow),
        Severity::Hint => Style::new().fg(DIM),
    }
}

pub fn out(kind: OutKind) -> Style {
    match kind {
        OutKind::Cmd => Style::new().fg(ACCENT),
        OutKind::Info => Style::new(),
        OutKind::Ok => Style::new().fg(Color::Green),
        OutKind::Hint => Style::new().fg(DIM),
        OutKind::Warn => Style::new().fg(Color::Yellow),
        OutKind::Error => Style::new().fg(Color::Red),
    }
}

pub fn heading() -> Style {
    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn white_key() -> Style {
    Style::new().fg(Color::Black).bg(Color::Gray)
}

pub fn black_key() -> Style {
    Style::new().fg(Color::White).bg(Color::Black)
}

/// Keys outside the playable range.
pub fn white_key_dim() -> Style {
    Style::new().fg(Color::Black).bg(Color::Rgb(95, 95, 95))
}

pub fn black_key_dim() -> Style {
    Style::new().fg(Color::Gray).bg(Color::Rgb(35, 35, 35))
}

pub fn pressed() -> Style {
    Style::new()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

pub fn playing_line() -> Style {
    Style::new().bg(Color::Rgb(40, 40, 70))
}
