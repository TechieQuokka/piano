//! Pitches on the 88-key piano (A0..=C8).

/// A key on the piano, stored as a MIDI note number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pitch(u8);

/// Note letter names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Letter {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl Letter {
    /// Semitones above C.
    pub fn semitone(self) -> i32 {
        match self {
            Letter::C => 0,
            Letter::D => 2,
            Letter::E => 4,
            Letter::F => 5,
            Letter::G => 7,
            Letter::A => 9,
            Letter::B => 11,
        }
    }

    pub fn from_char(c: char) -> Option<Letter> {
        Some(match c.to_ascii_uppercase() {
            'C' => Letter::C,
            'D' => Letter::D,
            'E' => Letter::E,
            'F' => Letter::F,
            'G' => Letter::G,
            'A' => Letter::A,
            'B' => Letter::B,
            _ => return None,
        })
    }
}

/// Sharp, flat, or none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Accidental {
    #[default]
    Natural,
    Sharp,
    Flat,
}

impl Accidental {
    pub fn offset(self) -> i32 {
        match self {
            Accidental::Natural => 0,
            Accidental::Sharp => 1,
            Accidental::Flat => -1,
        }
    }
}

const NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const SOLFEGE: [&str; 12] = [
    "도", "도#", "레", "레#", "미", "파", "파#", "솔", "솔#", "라", "라#", "시",
];

impl Pitch {
    pub const LOWEST: Pitch = Pitch(21);
    pub const HIGHEST: Pitch = Pitch(108);
    pub const MIDDLE_C: Pitch = Pitch(60);

    /// Returns `None` outside the 88-key range.
    pub fn new(midi: i32) -> Option<Pitch> {
        (i32::from(Self::LOWEST.0)..=i32::from(Self::HIGHEST.0))
            .contains(&midi)
            .then_some(Pitch(midi as u8))
    }

    /// MIDI number of a spelled note; may lie outside the piano range.
    pub fn spell(letter: Letter, accidental: Accidental, octave: i32) -> i32 {
        (octave + 1) * 12 + letter.semitone() + accidental.offset()
    }

    /// Moves `midi` by octaves until it lies within A0..=C8 (note name preserved).
    /// The flag is `true` if folding happened.
    pub fn fold(midi: i32) -> (Pitch, bool) {
        let (lo, hi) = (i32::from(Self::LOWEST.0), i32::from(Self::HIGHEST.0));
        let mut n = midi;
        while n > hi {
            n -= 12;
        }
        while n < lo {
            n += 12;
        }
        (Pitch(n as u8), n != midi)
    }

    pub fn midi(self) -> u8 {
        self.0
    }

    /// Scientific octave number (`C4` = middle C).
    pub fn octave(self) -> i32 {
        i32::from(self.0) / 12 - 1
    }

    /// Pitch class, 0 = C .. 11 = B.
    pub fn class(self) -> usize {
        usize::from(self.0 % 12)
    }

    pub fn is_black(self) -> bool {
        matches!(self.class(), 1 | 3 | 6 | 8 | 10)
    }

    /// Canonical name using sharps, without octave (`C#`).
    pub fn letter_name(self) -> &'static str {
        NAMES[self.class()]
    }

    /// Korean solfège name (`도`, `도#`, ...).
    pub fn solfege(self) -> &'static str {
        SOLFEGE[self.class()]
    }

    /// Zero-based index on the keyboard (A0 = 0 .. C8 = 87).
    pub fn key_index(self) -> usize {
        usize::from(self.0 - Self::LOWEST.0)
    }

    /// Transposes by `semitones`, folding back into range.
    pub fn transpose(self, semitones: i32) -> (Pitch, bool) {
        Pitch::fold(i32::from(self.0) + semitones)
    }
}

impl std::fmt::Display for Pitch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.letter_name(), self.octave())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spelling() {
        assert_eq!(Pitch::spell(Letter::C, Accidental::Natural, 4), 60);
        assert_eq!(Pitch::spell(Letter::A, Accidental::Natural, 0), 21);
        assert_eq!(Pitch::spell(Letter::C, Accidental::Natural, 8), 108);
        assert_eq!(Pitch::spell(Letter::B, Accidental::Flat, 3), 58);
        assert_eq!(Pitch::spell(Letter::C, Accidental::Sharp, 4), 61);
    }

    #[test]
    fn range() {
        assert!(Pitch::new(20).is_none());
        assert!(Pitch::new(109).is_none());
        assert_eq!(Pitch::new(60), Some(Pitch::MIDDLE_C));
    }

    #[test]
    fn folding_keeps_name() {
        let (p, folded) = Pitch::fold(Pitch::spell(Letter::D, Accidental::Natural, 8));
        assert!(folded);
        assert_eq!(p.to_string(), "D7");
        let (p, folded) = Pitch::fold(Pitch::spell(Letter::G, Accidental::Natural, 0));
        assert!(folded);
        assert_eq!(p.to_string(), "G1");
        let (_, folded) = Pitch::fold(60);
        assert!(!folded);
    }

    #[test]
    fn names() {
        let p = Pitch::new(61).unwrap();
        assert_eq!(p.to_string(), "C#4");
        assert_eq!(p.solfege(), "도#");
        assert!(p.is_black());
        assert_eq!(Pitch::LOWEST.key_index(), 0);
        assert_eq!(Pitch::HIGHEST.key_index(), 87);
    }
}
