//! Screen layout: Keys (always) / Code | Display / Cmd / Status.

use piano_app::{App, Focus, View};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::grid::truncate;
use crate::theme;
use crate::views;
use crate::widgets::{code, keyboard, status};

/// Rows of the full keyboard (5 key rows) plus its border.
const KEYS_FULL: u16 = keyboard::FULL_ROWS + 2;
/// Rows of the compact keyboard (one line of text) plus its border.
const KEYS_COMPACT: u16 = 3;
/// Below this terminal height the keyboard shrinks to one line.
const COMPACT_BELOW: u16 = 24;

pub struct Areas {
    pub keys: Rect,
    pub code: Rect,
    pub display: Rect,
    pub cmd: Rect,
    pub status: Rect,
}

pub fn areas(area: Rect) -> Areas {
    let keys_rows = if area.height < COMPACT_BELOW {
        KEYS_COMPACT
    } else {
        KEYS_FULL
    };
    let [keys, middle, cmd, status] = Layout::vertical([
        Constraint::Length(keys_rows),
        Constraint::Min(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(area);
    let [code, display] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(middle);
    Areas {
        keys,
        code,
        display,
        cmd,
        status,
    }
}

fn block(title: String, focused: bool) -> Block<'static> {
    Block::bordered()
        .title(Span::styled(format!(" {title} "), theme::title(focused)))
        .border_style(theme::border(focused))
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let a = areas(frame.area());
    draw_keys(frame, app, a.keys);
    draw_code(frame, app, a.code);
    draw_display(frame, app, a.display);
    draw_cmd(frame, app, a.cmd);
    frame.render_widget(Paragraph::new(status::line(app)), a.status);
}

fn draw_keys(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Keys;
    let width = area.width.saturating_sub(2) as usize;
    let compact = area.height < KEYS_FULL;
    let lines = keyboard::lines(app, width, compact);
    let title = if focused {
        "Keys · listen (← → octave)".to_string()
    } else {
        "Keys · F2 to listen".to_string()
    };
    frame.render_widget(Paragraph::new(lines).block(block(title, focused)), area);
}

fn draw_display(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width.saturating_sub(2) as usize;
    let height = area.height.saturating_sub(2) as usize;
    let (title, lines): (&str, Vec<Line>) = match app.view {
        View::Played => ("Played", views::played(app, height, width)),
        View::Ref => ("Ref", views::reference(app, width)),
        View::Output => ("Output", views::output(app, height, width)),
        View::Help | View::Man => ("Help / Man", views::page(app, &app.page, height, width)),
        View::Tutorial => (
            "Tutorial",
            views::page(app, &app.tutorial_page(), height, width),
        ),
        View::Song => ("Song", views::song(app, width)),
    };
    frame.render_widget(
        Paragraph::new(lines).block(block(format!("Display · {title}"), false)),
        area,
    );
}

fn draw_code(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Code;
    let width = (area.width.saturating_sub(2) as usize).saturating_sub(code::GUTTER);
    let height = area.height.saturating_sub(2) as usize;
    code::scroll_to_cursor(app, width, height);
    let b = app.buf();
    let title = format!(
        "Code · {}{}{}",
        b.name,
        if b.dirty { " *" } else { "" },
        if b.readonly { " (read-only)" } else { "" }
    );
    let (lines, cursor) = code::lines(app, width, height);
    frame.render_widget(Paragraph::new(lines).block(block(title, focused)), area);
    if focused && let Some((row, col)) = cursor {
        let x = area.x + 1 + (code::GUTTER + col) as u16;
        let y = area.y + 1 + row as u16;
        if x < area.right() && y < area.bottom() {
            frame.set_cursor_position(Position::new(x, y));
        }
    }
}

fn draw_cmd(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Cmd;
    let width = area.width.saturating_sub(4) as usize;
    let mut b = block("Cmd".into(), focused);
    if let Some(last) = app.output.last_result() {
        let text = truncate(&last.text, width.saturating_sub(12));
        b = b.title_bottom(
            Line::from(Span::styled(format!(" {text} "), theme::out(last.kind))).right_aligned(),
        );
    }
    let prompt = if app.pending.is_some() { "y/n> " } else { "> " };
    let line = Line::from(vec![
        Span::styled(prompt, Style::new().fg(theme::ACCENT)),
        Span::raw(app.cmdline.text.clone()),
    ]);
    frame.render_widget(Paragraph::new(line).block(b), area);
    if focused {
        let head: String = app.cmdline.text.chars().take(app.cmdline.cursor).collect();
        let x = area.x + 1 + prompt.len() as u16 + crate::grid::width(&head) as u16;
        if x < area.right() {
            frame.set_cursor_position(Position::new(x, area.y + 1));
        }
    }
}
