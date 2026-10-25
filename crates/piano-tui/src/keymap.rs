//! Key handling per focus.

use piano_app::{App, Focus};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Lines moved by PgUp / PgDn.
const PAGE: usize = 10;

/// Handles any key event. Release and repeat events only arrive from terminals
/// with the keyboard enhancement enabled; elsewhere every event is a press.
pub fn handle(app: &mut App, key: KeyEvent) {
    match key.kind {
        KeyEventKind::Release => {
            if app.focus == Focus::Keys
                && let KeyCode::Char(c) = key.code
            {
                app.key_release(c);
            }
            return;
        }
        // A held piano key keeps sounding; elsewhere a repeat types again.
        KeyEventKind::Repeat if app.focus == Focus::Keys => return,
        KeyEventKind::Repeat if matches!(key.code, KeyCode::F(_) | KeyCode::Esc) => return,
        _ => {}
    }
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let scroll_mod = key
        .modifiers
        .intersects(KeyModifiers::SHIFT | KeyModifiers::ALT);
    match key.code {
        // Some terminals report Shift+F12 as F24.
        KeyCode::F(12) if shift => app.set_focus(app.focus.prev()),
        KeyCode::F(24) => app.set_focus(app.focus.prev()),
        KeyCode::F(12) => app.set_focus(app.focus.next()),
        KeyCode::F(2) => app.set_focus(Focus::Keys),
        KeyCode::F(3) => app.set_focus(Focus::Code),
        KeyCode::F(4) => app.set_focus(Focus::Cmd),
        KeyCode::F(5) => app.play_here(),
        KeyCode::F(6) => {
            app.stop();
            app.output.info("■ stopped");
        }
        KeyCode::Char('c') if ctrl => app.execute("quit"),
        KeyCode::Esc => {
            if app.focus == Focus::Cmd {
                app.cmdline.take();
            }
            app.escape();
        }
        // Shift (or Alt) + PgUp/PgDn scroll the Display from any focus.
        KeyCode::PageUp if scroll_mod || app.focus != Focus::Code => {
            app.display_scroll = app.display_scroll.saturating_add(PAGE);
        }
        KeyCode::PageDown if scroll_mod || app.focus != Focus::Code => {
            app.display_scroll = app.display_scroll.saturating_sub(PAGE);
        }
        _ => match app.focus {
            Focus::Keys => keys(app, key),
            Focus::Code => code(app, key),
            Focus::Cmd => cmd(app, key),
        },
    }
}

fn keys(app: &mut App, key: KeyEvent) {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return;
    }
    match key.code {
        KeyCode::Char(c) if app.exact_keys => {
            app.key_press(c);
        }
        KeyCode::Char(c) => {
            app.key_down(c);
        }
        KeyCode::Left => app.shift_octave(-1),
        KeyCode::Right => app.shift_octave(1),
        _ => {}
    }
}

fn code(app: &mut App, key: KeyEvent) {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return;
    }
    match key.code {
        KeyCode::Char(c) => app.code_char(c),
        KeyCode::Enter => app.code_newline(),
        KeyCode::Backspace => app.code_backspace(),
        KeyCode::Delete => app.code_delete(),
        KeyCode::Tab => app.code_indent(),
        KeyCode::Left => app.buf_mut().move_left(),
        KeyCode::Right => app.buf_mut().move_right(),
        KeyCode::Up => app.buf_mut().move_up(1),
        KeyCode::Down => app.buf_mut().move_down(1),
        KeyCode::PageUp => app.buf_mut().move_up(PAGE),
        KeyCode::PageDown => app.buf_mut().move_down(PAGE),
        KeyCode::Home => app.buf_mut().home(),
        KeyCode::End => app.buf_mut().end(),
        _ => {}
    }
}

fn cmd(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => app.cmdline.insert(c),
        KeyCode::Enter => {
            app.submit_cmd();
            app.display_scroll = 0;
        }
        KeyCode::Backspace => app.cmdline.backspace(),
        KeyCode::Delete => app.cmdline.delete(),
        KeyCode::Left => app.cmdline.left(),
        KeyCode::Right => app.cmdline.right(),
        KeyCode::Home => app.cmdline.home(),
        KeyCode::End => app.cmdline.end(),
        KeyCode::Up => app.history_prev(),
        KeyCode::Down => app.history_next(),
        KeyCode::Tab => app.complete(),
        _ => {}
    }
}
