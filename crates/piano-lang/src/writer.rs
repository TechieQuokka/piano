//! Canonical text for notes inserted from the keyboard.
//!
//! The octave and length are written only when inheritance would not produce them.

use piano_core::{Accidental, Duration, Letter, Pitch};

use crate::resolve::State;
use crate::token::NoteName;

fn spelled(pitch: Pitch) -> (Letter, Accidental) {
    use Accidental::{Natural as N, Sharp as S};
    const TABLE: [(Letter, Accidental); 12] = [
        (Letter::C, N),
        (Letter::C, S),
        (Letter::D, N),
        (Letter::D, S),
        (Letter::E, N),
        (Letter::F, N),
        (Letter::F, S),
        (Letter::G, N),
        (Letter::G, S),
        (Letter::A, N),
        (Letter::A, S),
        (Letter::B, N),
    ];
    TABLE[pitch.class()]
}

/// Note name text relative to `reference`; returns the text and the new reference.
fn pitch_text(pitch: Pitch, reference: i32) -> String {
    let (letter, accidental) = spelled(pitch);
    let name = NoteName {
        letter,
        accidental,
        octave: None,
    };
    let probe = State {
        reference,
        ..State::new(&Default::default())
    };
    let (n, tie) = probe.resolve_note(&name);
    if n == i32::from(pitch.midi()) && !tie {
        pitch.letter_name().to_string()
    } else {
        pitch.to_string()
    }
}

fn len_text(len: Duration, state: &State) -> String {
    if len == state.len {
        String::new()
    } else {
        len.to_string()
    }
}

/// Text for a single note, e.g. `E`, `C5`, `G/8`.
pub fn note_text(pitch: Pitch, len: Duration, state: &State) -> String {
    format!(
        "{}{}",
        pitch_text(pitch, state.reference),
        len_text(len, state)
    )
}

/// Text for a chord, e.g. `{C E G}/2`.
pub fn chord_text(pitches: &[Pitch], len: Duration, state: &State) -> String {
    let mut reference = state.reference;
    let parts: Vec<String> = pitches
        .iter()
        .map(|&p| {
            let t = pitch_text(p, reference);
            reference = i32::from(p.midi());
            t
        })
        .collect();
    format!("{{{}}}{}", parts.join(" "), len_text(len, state))
}

/// Text for a rest, e.g. `r` or `r/8`.
pub fn rest_text(len: Duration, state: &State) -> String {
    format!("r{}", len_text(len, state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CompileOptions, compile_piece};

    fn state() -> State {
        State::new(&CompileOptions::default())
    }

    fn p(s: &str) -> Pitch {
        compile_piece(s).score.notes[0].pitch
    }

    #[test]
    fn omits_inherited_values() {
        assert_eq!(note_text(p("E4"), Duration::QUARTER, &state()), "E");
        assert_eq!(note_text(p("E5"), Duration::QUARTER, &state()), "E5");
        assert_eq!(
            note_text(p("C#4"), Duration::new(8, 0).unwrap(), &state()),
            "C#/8"
        );
    }

    #[test]
    fn ambiguous_writes_octave() {
        assert_eq!(note_text(p("F#4"), Duration::QUARTER, &state()), "F#4");
    }

    #[test]
    fn chord_and_rest() {
        let pitches = [p("C4"), p("E4"), p("G4")];
        let half = Duration::new(2, 0).unwrap();
        assert_eq!(chord_text(&pitches, half, &state()), "{C E G}/2");
        assert_eq!(rest_text(Duration::QUARTER, &state()), "r");
    }

    #[test]
    fn round_trip() {
        let s = state();
        for midi in 21..=108 {
            let pitch = Pitch::new(midi).unwrap();
            let text = note_text(pitch, Duration::QUARTER, &s);
            assert_eq!(compile_piece(&text).score.notes[0].pitch, pitch, "{text}");
        }
    }
}
