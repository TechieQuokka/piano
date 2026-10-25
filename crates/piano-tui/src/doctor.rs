//! Interactive key check for `piano doctor`.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::terminal;

type Matcher = fn(KeyCode, KeyModifiers) -> bool;

/// Keys the editor relies on, as (label, matcher).
const KEYS: &[(&str, Matcher)] = &[
    ("F2", |c, _| c == KeyCode::F(2)),
    ("F3", |c, _| c == KeyCode::F(3)),
    ("F4", |c, _| c == KeyCode::F(4)),
    ("F5", |c, _| c == KeyCode::F(5)),
    ("F6", |c, _| c == KeyCode::F(6)),
    ("F12", |c, m| {
        c == KeyCode::F(12) && !m.contains(KeyModifiers::SHIFT)
    }),
    ("Shift+F12", |c, m| {
        (c == KeyCode::F(12) && m.contains(KeyModifiers::SHIFT)) || c == KeyCode::F(24)
    }),
];

/// Asks the user to press each key; returns (label, received). Esc skips a key;
/// a key not pressed within 10 s counts as not received.
pub fn key_test() -> io::Result<Vec<(&'static str, bool)>> {
    let mut results = Vec::new();
    terminal::enable_raw_mode()?;
    let run = (|| -> io::Result<()> {
        for (label, matches) in KEYS {
            print!("  press {label:<10} (Esc = skip) … ");
            io::stdout().flush()?;
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut ok = false;
            while Instant::now() < deadline {
                if !event::poll(Duration::from_millis(100))? {
                    continue;
                }
                if let Event::Key(k) = event::read()?
                    && k.kind == KeyEventKind::Press
                {
                    if k.code == KeyCode::Esc {
                        break;
                    }
                    if matches(k.code, k.modifiers) {
                        ok = true;
                        break;
                    }
                }
            }
            print!("{}\r\n", if ok { "ok" } else { "not received" });
            results.push((*label, ok));
        }
        Ok(())
    })();
    terminal::disable_raw_mode()?;
    run?;
    Ok(results)
}

/// Whether the terminal supports disambiguated keys (better Shift+F12 / Esc).
pub fn keyboard_enhancement() -> bool {
    terminal::supports_keyboard_enhancement().unwrap_or(false)
}
