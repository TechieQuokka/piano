//! Keys focus: audition only (nothing is written to the code).
//!
//! A held key keeps sounding and fades when let go. Terminals that report key
//! release events (kitty keyboard protocol) give exact presses and releases.
//! Others, like GNOME Terminal, only send auto-repeats while a key is held, so
//! holding and letting go are inferred from the repeat stream.

use std::time::{Duration, Instant};

use piano_core::{Dynamic, Pitch};

use crate::app::App;
use crate::input;

/// Events of the same key closer than this are auto-repeats.
pub const REPEAT_GAP: Duration = Duration::from_millis(120);
/// A repeating key with no event for this long has been let go.
pub const RELEASE_GAP: Duration = Duration::from_millis(150);
/// Longest auto-repeat delay we wait for before deciding a key was only tapped.
pub const FIRST_REPEAT_WAIT: Duration = Duration::from_millis(700);
/// Extra ring after letting go, before the SoundFont's own release fades the note.
pub const RELEASE_TAIL_SECS: f64 = 0.25;
/// A short tap still sounds at least this long.
pub const MIN_RING_SECS: f64 = 0.4;
/// Played notes kept for the Played view.
pub const MAX_PLAYED: usize = 200;

/// A key currently sounding from the keyboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    key: char,
    pitch: Pitch,
    started: Instant,
    /// Last event for this key (press or repeat).
    last: Instant,
    /// Auto-repeats have been seen (inferred mode).
    repeating: bool,
    /// A later event that may be a second tap; struck if no repeats follow.
    maybe_tap: bool,
}

impl App {
    fn key_velocity(&self) -> u8 {
        self.input.dynamic.unwrap_or(Dynamic::Mf).velocity()
    }

    fn strike(&mut self, pitch: Pitch, now: Instant) {
        self.player.strike(pitch, self.key_velocity());
        self.pressed.push((pitch, now));
        self.played.push((pitch, now));
        if self.played.len() > MAX_PLAYED {
            let extra = self.played.len() - MAX_PLAYED;
            self.played.drain(..extra);
        }
    }

    fn let_go(&mut self, i: usize, now: Instant) {
        let h = self.held.remove(i);
        let rung = now.duration_since(h.started).as_secs_f64();
        let after = RELEASE_TAIL_SECS.max(MIN_RING_SECS - rung);
        self.player.release(h.pitch, after);
    }

    /// Pitch for a PC key in the current octave; warns outside the piano.
    fn key_pitch(&mut self, c: char) -> Option<Option<Pitch>> {
        let midi = input::key_pitch(c, self.input.octave)?;
        let pitch = Pitch::new(midi);
        if pitch.is_none() {
            self.output.warn("outside the 88 keys (A0-C8)");
        }
        Some(pitch)
    }

    /// A key event from a terminal without release events (press or auto-repeat
    /// look the same). Returns `false` if `c` is not a piano key.
    pub fn key_down(&mut self, c: char) -> bool {
        self.key_down_at(c, Instant::now())
    }

    pub fn key_down_at(&mut self, c: char, now: Instant) -> bool {
        let c = c.to_ascii_lowercase();
        let Some(pitch) = self.key_pitch(c) else {
            return false;
        };
        let Some(pitch) = pitch else {
            return true;
        };
        if let Some(h) = self.held.iter_mut().find(|h| h.key == c) {
            let gap = now.duration_since(h.last);
            h.last = now;
            if gap <= REPEAT_GAP {
                // Auto-repeat: keep sounding.
                h.repeating = true;
                h.maybe_tap = false;
            } else if !h.repeating {
                // The first auto-repeat or a second tap: decided in `tick`.
                h.maybe_tap = true;
            } else {
                h.started = now;
                self.strike(pitch, now);
            }
            return true;
        }
        self.strike(pitch, now);
        self.held.push(Held {
            key: c,
            pitch,
            started: now,
            last: now,
            repeating: false,
            maybe_tap: false,
        });
        true
    }

    /// Exact press (terminals with release events).
    pub fn key_press(&mut self, c: char) -> bool {
        let now = Instant::now();
        let c = c.to_ascii_lowercase();
        let Some(pitch) = self.key_pitch(c) else {
            return false;
        };
        let Some(pitch) = pitch else {
            return true;
        };
        // Still held (no release yet): an auto-repeat. Kitty-protocol terminals send
        // repeats of text keys as plain presses unless every key is reported as an
        // escape code, so a held key would otherwise be struck again and again.
        if self.held.iter().any(|h| h.key == c) {
            return true;
        }
        self.strike(pitch, now);
        self.held.push(Held {
            key: c,
            pitch,
            started: now,
            last: now,
            repeating: false,
            maybe_tap: false,
        });
        true
    }

    /// Exact release (terminals with release events).
    pub fn key_release(&mut self, c: char) {
        let c = c.to_ascii_lowercase();
        if let Some(i) = self.held.iter().position(|h| h.key == c) {
            self.let_go(i, Instant::now());
        }
    }

    /// Lets every held key go (focus change, octave shift).
    pub fn release_all_keys(&mut self) {
        let now = Instant::now();
        while !self.held.is_empty() {
            self.let_go(0, now);
        }
    }

    /// Inferred mode: resolves second taps and lets go of keys whose repeats stopped.
    pub(crate) fn tick_keys(&mut self, now: Instant) {
        if self.exact_keys {
            return;
        }
        let mut i = 0;
        while i < self.held.len() {
            let h = &self.held[i];
            let quiet = now.duration_since(h.last);
            if h.maybe_tap && quiet > REPEAT_GAP {
                // No repeats followed: it was a second tap.
                let pitch = h.pitch;
                let h = &mut self.held[i];
                h.maybe_tap = false;
                h.started = now;
                self.strike(pitch, now);
                i += 1;
                continue;
            }
            let released = if h.repeating {
                quiet > RELEASE_GAP
            } else {
                !h.maybe_tap && quiet > FIRST_REPEAT_WAIT
            };
            if released {
                self.let_go(i, now);
            } else {
                i += 1;
            }
        }
    }

    /// Pitches currently held (for highlighting).
    pub fn held_pitches(&self) -> Vec<Pitch> {
        self.held.iter().map(|h| h.pitch).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::App;

    fn app(name: &str) -> (App, std::path::PathBuf) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-tmp")
            .join(format!("keys-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let project = piano_project::Project::create(&dir).unwrap();
        let mut player = piano_audio::Player::silent();
        player
            .set_soundfont(piano_audio::testing::tiny_soundfont())
            .unwrap();
        (App::headless_with(project, player), dir)
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn holding_strikes_once_and_lets_go_after_repeats_stop() {
        let (mut a, dir) = app("hold");
        let t0 = Instant::now();
        a.key_down_at('z', t0);
        // OS delay, then auto-repeat every 33 ms for a second.
        let mut t = t0 + ms(500);
        a.key_down_at('z', t);
        a.tick_keys(t + ms(20));
        for _ in 0..30 {
            t += ms(33);
            a.key_down_at('z', t);
            a.tick_keys(t);
        }
        assert_eq!(a.played.len(), 1, "held key must strike once");
        assert_eq!(a.held.len(), 1);
        a.tick_keys(t + ms(100));
        assert_eq!(a.held.len(), 1, "still within the repeat gap");
        a.tick_keys(t + ms(200));
        assert!(a.held.is_empty(), "let go once repeats stop");
        assert!(a.buf().text().trim() == "// p1", "nothing written to the code");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_tap_rings_then_is_let_go() {
        let (mut a, dir) = app("tap");
        let t0 = Instant::now();
        a.key_down_at('x', t0);
        a.tick_keys(t0 + ms(300));
        assert_eq!(a.held.len(), 1, "may still be the OS repeat delay");
        a.tick_keys(t0 + ms(800));
        assert!(a.held.is_empty());
        assert_eq!(a.played.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn second_tap_of_the_same_key_strikes_again() {
        let (mut a, dir) = app("retap");
        let t0 = Instant::now();
        a.key_down_at('c', t0);
        a.key_down_at('c', t0 + ms(300));
        assert_eq!(a.played.len(), 1, "undecided until no repeats follow");
        a.tick_keys(t0 + ms(450));
        assert_eq!(a.played.len(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn different_keys_each_strike() {
        let (mut a, dir) = app("many");
        let t0 = Instant::now();
        for (i, c) in "zxc".chars().enumerate() {
            a.key_down_at(c, t0 + ms(40 * i as u64));
        }
        let names: Vec<String> = a.played.iter().map(|(p, _)| p.to_string()).collect();
        assert_eq!(names, ["C4", "D4", "E4"]);
        assert!(!a.key_down_at('a', t0), "A is not a piano key");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn exact_repeats_sent_as_presses_strike_once() {
        let (mut a, dir) = app("exact");
        a.exact_keys = true;
        // Kitty protocol: repeats of a held text key arrive as plain presses.
        for _ in 0..20 {
            a.key_press('z');
        }
        assert_eq!(a.played.len(), 1, "held key must strike once");
        a.key_release('z');
        assert!(a.held.is_empty());
        a.key_press('z');
        assert_eq!(a.played.len(), 2, "a new press after release strikes again");
        let _ = std::fs::remove_dir_all(dir);
    }
}
