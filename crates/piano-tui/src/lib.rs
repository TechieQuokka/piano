//! Terminal UI: event loop, key mapping, layout, and views.

pub mod doctor;
mod grid;
pub mod keymap;
pub mod layout;
mod theme;
mod views;
mod widgets;

use std::io;
use std::time::Duration;

use piano_app::App;
use ratatui::crossterm::event::{
    self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::{execute, terminal};

/// Redraw / tick interval.
const TICK: Duration = Duration::from_millis(50);

/// Runs the editor until `quit`. The terminal is restored even on error.
pub fn run(app: &mut App) -> io::Result<()> {
    let mut term = ratatui::init();
    // Disambiguated escape codes let terminals report Shift+F12 and Esc reliably.
    let enhanced = terminal::supports_keyboard_enhancement().unwrap_or(false)
        && execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            )
        )
        .is_ok();
    // With event types reported, held piano keys get exact presses and releases.
    app.exact_keys = enhanced;
    let result = (|| -> io::Result<()> {
        while !app.quit {
            app.tick();
            term.draw(|f| layout::draw(f, app))?;
            if event::poll(TICK)? {
                match event::read()? {
                    Event::Key(k) => keymap::handle(app, k),
                    Event::Paste(text) => paste(app, &text),
                    _ => {}
                }
            }
        }
        Ok(())
    })();
    if enhanced {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    app.stop();
    ratatui::restore();
    result
}

fn paste(app: &mut App, text: &str) {
    match app.focus {
        piano_app::Focus::Code => {
            for c in text.chars() {
                if c == '\n' {
                    app.code_newline();
                } else if c != '\r' {
                    app.code_char(c);
                }
            }
        }
        piano_app::Focus::Cmd => {
            for c in text.chars().filter(|c| !c.is_control()) {
                app.cmdline.insert(c);
            }
        }
        piano_app::Focus::Keys => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use piano_project::Project;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn app(name: &str) -> (App, std::path::PathBuf) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-tmp")
            .join(format!("tui-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let project = Project::create(&dir).unwrap();
        (App::headless(project), dir)
    }

    fn screen(app: &mut App, w: u16, h: u16) -> String {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| layout::draw(f, app)).unwrap();
        let buf = t.backend().buffer().clone();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn key(app: &mut App, code: KeyCode) {
        keymap::handle(app, KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn renders_every_view_at_several_sizes() {
        let (mut a, dir) = app("views");
        for c in ['z', 'x', 'c'] {
            key(&mut a, KeyCode::Char(c));
        }
        for view in [
            "played", "ref", "output", "help", "man", "guide", "song", "tutorial",
        ] {
            a.execute(view);
            for (w, h) in [(120, 40), (80, 24), (40, 12), (20, 6)] {
                let s = screen(&mut a, w, h);
                assert!(!s.is_empty());
            }
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn keys_view_shows_keyboard_and_typed_notes() {
        let (mut a, dir) = app("keys");
        // Startup shows Output (startup messages); `played` switches the Display.
        a.execute("played");
        for c in ['z', 'x', 'c'] {
            key(&mut a, KeyCode::Char(c));
        }
        a.lang = piano_project::Lang::Ko;
        let s = screen(&mut a, 120, 40);
        assert!(!s.contains("A0 "), "no 88-key map line: {s}");
        assert!(s.contains("C4"), "{s}");
        assert!(s.contains("도"), "{s}");
        assert!(s.contains("E4 미"), "played notes are listed: {s}");
        assert!(s.contains("[Keys]"), "{s}");
        a.lang = piano_project::Lang::En;
        let s = screen(&mut a, 120, 40);
        assert!(
            s.contains("E4 Mi") && !s.contains('도'),
            "solfege follows lang: {s}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn small_terminal_shows_the_range_line() {
        let (mut a, dir) = app("compact");
        let s = screen(&mut a, 100, 20);
        assert!(s.contains("keys C4 - B5"), "{s}");
        assert!(!s.contains("│ Z │"), "no zoomed keyboard when short: {s}");
        let s = screen(&mut a, 100, 30);
        assert!(s.contains(" Z "), "{s}");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// `cargo test -p piano-tui dump -- --ignored --nocapture` prints sample screens.
    #[test]
    #[ignore]
    fn dump() {
        let (mut a, dir) = app("dump");
        for c in "zxcvb".chars() {
            key(&mut a, KeyCode::Char(c));
        }
        key(&mut a, KeyCode::Enter);
        for c in "zcb".chars() {
            key(&mut a, KeyCode::Char(c));
        }
        key(&mut a, KeyCode::Enter);
        a.execute("ped on");
        println!("{}", screen(&mut a, 110, 34));
        println!("{}", screen(&mut a, 200, 30));
        key(&mut a, KeyCode::F(3));
        for c in
            "\nup(12){ accent(1/3){ C4/8 D E F G A } } r/4 ped off C D E F G A B C D E F G".chars()
        {
            if c == '\n' {
                key(&mut a, KeyCode::Enter);
            } else {
                key(&mut a, KeyCode::Char(c));
            }
        }
        key(&mut a, KeyCode::Left);
        key(&mut a, KeyCode::Left);
        println!("{}", screen(&mut a, 110, 34));
        a.execute("man octave");
        println!("{}", screen(&mut a, 110, 34));
        a.execute("guide octave");
        println!("{}", screen(&mut a, 160, 45));
        a.execute("lang ko");
        println!("{}", screen(&mut a, 160, 45));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn focus_keys_and_cmd_typing() {
        let (mut a, dir) = app("focus");
        key(&mut a, KeyCode::F(12));
        assert_eq!(a.focus, piano_app::Focus::Code);
        keymap::handle(&mut a, KeyEvent::new(KeyCode::F(12), KeyModifiers::SHIFT));
        assert_eq!(a.focus, piano_app::Focus::Keys);
        key(&mut a, KeyCode::F(4));
        for c in "help".chars() {
            key(&mut a, KeyCode::Char(c));
        }
        key(&mut a, KeyCode::Enter);
        assert_eq!(a.view, piano_app::View::Help);
        let s = screen(&mut a, 100, 30);
        assert!(s.contains("PLAYBACK"), "{s}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
