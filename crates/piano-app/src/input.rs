//! Keyboard note entry: PC key → piano key, and token-level editing at the cursor.

use piano_core::{Dynamic, Pitch};

use crate::buffer::Buffer;

/// Keyboard (Keys focus) settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputState {
    /// Loudness of the keys; `None` = mf.
    pub dynamic: Option<Dynamic>,
    /// Octave of the lower row (`Z` = C of this octave).
    pub octave: i32,
}

impl Default for InputState {
    fn default() -> Self {
        InputState {
            dynamic: None,
            octave: 4,
        }
    }
}

pub const MIN_OCTAVE: i32 = 0;
pub const MAX_OCTAVE: i32 = 7;

/// Lower row (`Z`…`M`, black `S D G H J`) and upper row (`Q`…`U`, black `2 3 5 6 7`):
/// (row, semitone above C).
pub fn piano_key(c: char) -> Option<(i32, i32)> {
    let c = c.to_ascii_lowercase();
    const LOWER: [(char, i32); 12] = [
        ('z', 0),
        ('s', 1),
        ('x', 2),
        ('d', 3),
        ('c', 4),
        ('v', 5),
        ('g', 6),
        ('b', 7),
        ('h', 8),
        ('n', 9),
        ('j', 10),
        ('m', 11),
    ];
    const UPPER: [(char, i32); 12] = [
        ('q', 0),
        ('2', 1),
        ('w', 2),
        ('3', 3),
        ('e', 4),
        ('r', 5),
        ('5', 6),
        ('t', 7),
        ('6', 8),
        ('y', 9),
        ('7', 10),
        ('u', 11),
    ];
    if let Some((_, s)) = LOWER.iter().find(|(k, _)| *k == c) {
        return Some((0, *s));
    }
    UPPER.iter().find(|(k, _)| *k == c).map(|(_, s)| (1, *s))
}

/// The PC key label for a pitch in the current range (for the Keys view).
pub fn key_label(pitch: Pitch, octave: i32) -> Option<char> {
    let base = (octave + 1) * 12;
    let rel = i32::from(pitch.midi()) - base;
    const LOWER: [char; 12] = ['Z', 'S', 'X', 'D', 'C', 'V', 'G', 'B', 'H', 'N', 'J', 'M'];
    const UPPER: [char; 12] = ['Q', '2', 'W', '3', 'E', 'R', '5', 'T', '6', 'Y', '7', 'U'];
    match rel {
        0..=11 => Some(LOWER[rel as usize]),
        12..=23 => Some(UPPER[rel as usize - 12]),
        _ => None,
    }
}

/// MIDI number for a key press, or `None` if not a piano key.
pub fn key_pitch(c: char, octave: i32) -> Option<i32> {
    let (row, semi) = piano_key(c)?;
    Some((octave + 1 + row) * 12 + semi)
}

/// Inserts `token` at the cursor, separated by spaces from its neighbours.
/// A cursor inside a `//` comment moves to a new line first, so the note is not
/// swallowed by the comment.
pub fn insert_token(buf: &mut Buffer, token: &str) {
    let head: String = buf.line().chars().take(buf.cursor.col).collect();
    if head.contains("//") {
        buf.end();
        buf.insert_str("\n");
    }
    let before = buf.char_before().is_some_and(|c| !c.is_whitespace());
    let after = buf.char_at();
    let mut s = String::new();
    if before {
        s.push(' ');
    }
    s.push_str(token);
    if !after.is_some_and(char::is_whitespace) {
        s.push(' ');
        buf.insert_str(&s);
    } else {
        buf.insert_str(&s);
        buf.move_right();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn buf(text: &str) -> Buffer {
        let mut b = Buffer::new("p1", Path::new("p1.piano"), text, false);
        b.end();
        b
    }

    #[test]
    fn key_mapping() {
        assert_eq!(key_pitch('z', 4), Some(60));
        assert_eq!(key_pitch('S', 4), Some(61));
        assert_eq!(key_pitch('m', 4), Some(71));
        assert_eq!(key_pitch('q', 4), Some(72));
        assert_eq!(key_pitch('7', 4), Some(82));
        assert_eq!(key_pitch('a', 4), None);
        assert_eq!(key_label(Pitch::new(64).unwrap(), 4), Some('C'));
    }

    #[test]
    fn insert_spacing() {
        let mut b = buf("C");
        insert_token(&mut b, "D");
        assert_eq!(b.line(), "C D ");
        insert_token(&mut b, "E/8");
        assert_eq!(b.line(), "C D E/8 ");
    }

    #[test]
    fn insert_after_comment_starts_new_line() {
        let mut b = buf("C // intro");
        insert_token(&mut b, "D");
        assert_eq!(b.lines, ["C // intro", "D "]);
    }

    #[test]
    fn insert_in_middle() {
        let mut b = buf("C E");
        b.cursor.col = 1;
        insert_token(&mut b, "D");
        assert_eq!(b.line(), "C D E");
        assert_eq!(b.cursor.col, 4);
    }
}
