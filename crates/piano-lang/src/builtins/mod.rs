//! Builtin transform functions: `name(args){ ... }`.

mod dynamics;
mod pitch;
mod rhythm;

use piano_core::{Alert, Pitch, SrcSpan};

use crate::ast::Arg;
use crate::expand::Track;
use crate::token::NoteName;

/// Reference information shown in the Ref view and the manual.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinInfo {
    pub name: &'static str,
    pub signature: &'static str,
    pub summary: &'static str,
    pub detail: &'static str,
    pub example: &'static str,
}

pub const BUILTINS: &[BuiltinInfo] = &[
    BuiltinInfo {
        name: "up",
        signature: "up(n){ ... }",
        summary: "raise n semitones",
        detail: "Raise every note in the section by n semitones (12 = one octave).",
        example: "up(12){ C E G }",
    },
    BuiltinInfo {
        name: "down",
        signature: "down(n){ ... }",
        summary: "lower n semitones",
        detail: "Lower every note in the section by n semitones.",
        example: "down(2){ G A B }",
    },
    BuiltinInfo {
        name: "accent",
        signature: "accent(rule){ ... }",
        summary: "emphasize notes",
        detail: "Make matching notes louder (+20). rule 1/n = every n-th onset, \
                 starting with the first; rule G = every G.",
        example: "accent(1/3){ C D E F G A }",
    },
    BuiltinInfo {
        name: "louder",
        signature: "louder(n){ ... }",
        summary: "volume +n",
        detail: "Add n to the velocity of every note (clamped to 1-127).",
        example: "louder(20){ C D E }",
    },
    BuiltinInfo {
        name: "softer",
        signature: "softer(n){ ... }",
        summary: "volume -n",
        detail: "Subtract n from the velocity of every note (clamped to 1-127).",
        example: "softer(20){ C D E }",
    },
    BuiltinInfo {
        name: "tuplet",
        signature: "tuplet(n){ ... }",
        summary: "n notes in time of fewer",
        detail: "Play the section in the time of the nearest lower power of two \
                 (3 in 2, 5 in 4, 7 in 4). n >= 3.",
        example: "tuplet(3){ C/8 D E }",
    },
    BuiltinInfo {
        name: "seq",
        signature: "seq(times, step){ ... }",
        summary: "repeat shifted by step",
        detail: "Play the section `times` times, each time shifted by `step` more semitones.",
        example: "seq(3, 1){ C D E }",
    },
];

pub fn info(name: &str) -> Option<&'static BuiltinInfo> {
    BUILTINS.iter().find(|b| b.name.eq_ignore_ascii_case(name))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccentRule {
    /// `1/n`: every n-th onset, starting with the first.
    Every(u32),
    /// A note name: every note of that pitch class.
    PitchClass(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    Transpose(i32),
    Accent(AccentRule),
    Volume(i32),
    Tuplet(u32),
    Seq { times: u32, step: i32 },
}

/// Velocity added by `accent`.
pub const ACCENT_BOOST: i32 = 20;

impl Builtin {
    /// Validates arguments; reports an Error and returns `None` if they are wrong.
    pub fn from_args(
        name: &str,
        args: &[Arg],
        span: SrcSpan,
        alerts: &mut Vec<Alert>,
    ) -> Option<Builtin> {
        let usage = || info(name).map_or("", |i| i.signature);
        let int = |i: usize| match args.get(i) {
            Some(Arg::Int(n)) => Some(*n),
            _ => None,
        };
        let result = match (name, args.len()) {
            ("up", 1) => int(0).map(|n| Builtin::Transpose(n as i32)),
            ("down", 1) => int(0).map(|n| Builtin::Transpose(-n as i32)),
            ("louder", 1) => int(0).map(|n| Builtin::Volume(n as i32)),
            ("softer", 1) => int(0).map(|n| Builtin::Volume(-n as i32)),
            ("tuplet", 1) => int(0)
                .filter(|n| (3..=64).contains(n))
                .map(|n| Builtin::Tuplet(n as u32)),
            ("seq", 2) => match (int(0), int(1)) {
                (Some(t), Some(s)) if (1..=64).contains(&t) => Some(Builtin::Seq {
                    times: t as u32,
                    step: s as i32,
                }),
                _ => None,
            },
            ("accent", 1) => match &args[0] {
                Arg::Fraction(1, n) if (1..=64).contains(n) => {
                    Some(Builtin::Accent(AccentRule::Every(*n as u32)))
                }
                Arg::Note(NoteName {
                    letter, accidental, ..
                }) => {
                    let pc = (letter.semitone() + accidental.offset()).rem_euclid(12);
                    Some(Builtin::Accent(AccentRule::PitchClass(pc as usize)))
                }
                _ => None,
            },
            _ => None,
        };
        let result = result.filter(|b| match b {
            Builtin::Transpose(n) => n.abs() <= 88,
            _ => true,
        });
        if result.is_none() {
            alerts.push(
                Alert::error(span, format!("invalid arguments for `{name}`"))
                    .with_fix(format!("Usage: {}", usage())),
            );
        }
        result
    }

    pub fn apply(self, track: Track, alerts: &mut Vec<Alert>) -> Track {
        match self {
            Builtin::Transpose(n) => pitch::transpose(track, n, alerts),
            Builtin::Seq { times, step } => pitch::seq(track, times, step, alerts),
            Builtin::Accent(rule) => dynamics::accent(track, rule),
            Builtin::Volume(n) => dynamics::volume(track, n),
            Builtin::Tuplet(n) => rhythm::tuplet(track, n),
        }
    }
}

/// Name of a MIDI number that may lie outside the piano (`G9`).
pub(crate) fn midi_name(midi: i32) -> String {
    let (p, _) = Pitch::fold(midi);
    format!("{}{}", p.letter_name(), midi.div_euclid(12) - 1)
}

/// Clamps a velocity to 1..=127.
pub(crate) fn clamp_velocity(v: i32) -> u8 {
    v.clamp(1, 127) as u8
}
