//! Syntax tree as written, before inheritance is resolved.

use piano_core::{Duration, Dynamic, PedalKind, SrcSpan, TimeSig};

use crate::token::{Mark, NoteName};

/// Marks written after a note or chord: `'` `''` `^` `_` `@`. They are not inherited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Marks {
    /// 0: none, 1: staccato `'`, 2: staccatissimo `''`.
    pub staccato: u8,
    pub accent: bool,
    pub tenuto: bool,
    pub fermata: bool,
}

impl Marks {
    pub fn add(&mut self, mark: Mark) {
        match mark {
            Mark::Staccato => self.staccato = self.staccato.max(1),
            Mark::Staccatissimo => self.staccato = 2,
            Mark::Accent => self.accent = true,
            Mark::Tenuto => self.tenuto = true,
            Mark::Fermata => self.fermata = true,
        }
    }

    pub fn is_empty(self) -> bool {
        self == Marks::default()
    }

    /// Percent of the written length that sounds: `'` 50, `''` 25, `'_` 75.
    pub fn gate(self) -> u64 {
        match (self.staccato, self.tenuto) {
            (2, _) => 25,
            (1, true) => 75,
            (1, false) => 50,
            _ => 100,
        }
    }

    /// Velocity added: `^` 20 (as `accent`), `_` 8.
    pub fn boost(self) -> i32 {
        20 * i32::from(self.accent) + 8 * i32::from(self.tenuto)
    }
}

/// A mark for the next note only: `sf` and its kin stress it; `fp` and its kin
/// also make the notes after it soft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sudden {
    /// `sf` `sfz` `sffz` `fz` `rf` `rfz`: two levels above the current one, at least f.
    Sf,
    /// `fp`: f, then p.
    Fp,
    /// `sfp` `sfzp`: as `sf`, then p.
    Sfp,
    /// `ffp`: ff, then p.
    Ffp,
}

impl Sudden {
    pub fn parse(word: &str) -> Option<Sudden> {
        Some(match word.to_ascii_lowercase().as_str() {
            "sf" | "sfz" | "sffz" | "fz" | "rf" | "rfz" => Sudden::Sf,
            "fp" => Sudden::Fp,
            "sfp" | "sfzp" => Sudden::Sfp,
            "ffp" => Sudden::Ffp,
            _ => return None,
        })
    }

    /// Velocity of the marked note, from the velocity it would have had.
    pub fn velocity(self, current: u8) -> u8 {
        let level = |d: Dynamic| Dynamic::ALL.iter().position(|x| *x == d).unwrap_or(0);
        let sf = || {
            let now = Dynamic::ALL
                .iter()
                .enumerate()
                .min_by_key(|(_, d)| d.velocity().abs_diff(current))
                .map_or(0, |(i, _)| i);
            Dynamic::ALL[(now + 2).max(level(Dynamic::F)).min(Dynamic::ALL.len() - 1)]
        };
        match self {
            Sudden::Sf | Sudden::Sfp => sf(),
            Sudden::Fp => Dynamic::F,
            Sudden::Ffp => Dynamic::Ff,
        }
        .velocity()
    }

    /// The word written for it.
    pub fn keyword(self) -> &'static str {
        match self {
            Sudden::Sf => "sf",
            Sudden::Fp => "fp",
            Sudden::Sfp => "sfp",
            Sudden::Ffp => "ffp",
        }
    }

    /// Whether the notes after it are soft (`p`).
    pub fn then_soft(self) -> bool {
        !matches!(self, Sudden::Sf)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Int(i64),
    Fraction(i64, i64),
    Note(NoteName),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Note {
        name: NoteName,
        len: Option<Duration>,
        marks: Marks,
        tie: bool,
        span: SrcSpan,
    },
    Rest {
        len: Option<Duration>,
        /// `@`: a fermata over the rest.
        hold: bool,
        span: SrcSpan,
    },
    Chord {
        /// (name, span, tied by its own `~`)
        notes: Vec<(NoteName, SrcSpan, bool)>,
        len: Option<Duration>,
        marks: Marks,
        tie: bool,
        span: SrcSpan,
    },
    Dynamic(Dynamic, SrcSpan),
    Sudden(Sudden, SrcSpan),
    Cresc(SrcSpan),
    Dim(SrcSpan),
    Pedal {
        kind: PedalKind,
        on: bool,
        span: SrcSpan,
    },
    Tempo(u16, SrcSpan),
    Time(TimeSig, SrcSpan),
    Bar(SrcSpan),
    BarRepeat(SrcSpan),
    Repeat {
        body: Vec<Node>,
        endings: Vec<Vec<Node>>,
        times: u32,
        span: SrcSpan,
    },
    Func {
        name: String,
        args: Vec<Arg>,
        body: Vec<Node>,
        span: SrcSpan,
    },
    /// `voices{ ... }{ ... }`: sections that start together and sound at once.
    Voices {
        voices: Vec<Vec<Node>>,
        span: SrcSpan,
    },
    /// `free{ ... }`: an unmetered stretch; the bar that holds it is not checked.
    Free {
        body: Vec<Node>,
        span: SrcSpan,
    },
    /// `grace{ ... }`: quick notes played on the beat of the next note, in its time.
    Grace {
        body: Vec<Node>,
        span: SrcSpan,
    },
    /// Nodes kept for their content only (e.g. the body of an unknown function).
    Group(Vec<Node>),
}

impl Node {
    /// Position of the node in the source, if it has one.
    pub fn span(&self) -> Option<SrcSpan> {
        match self {
            Node::Note { span, .. }
            | Node::Rest { span, .. }
            | Node::Chord { span, .. }
            | Node::Dynamic(_, span)
            | Node::Sudden(_, span)
            | Node::Cresc(span)
            | Node::Dim(span)
            | Node::Pedal { span, .. }
            | Node::Tempo(_, span)
            | Node::Time(_, span)
            | Node::Bar(span)
            | Node::BarRepeat(span)
            | Node::Repeat { span, .. }
            | Node::Func { span, .. }
            | Node::Voices { span, .. }
            | Node::Free { span, .. }
            | Node::Grace { span, .. } => Some(*span),
            Node::Group(nodes) => nodes.first().and_then(Node::span),
        }
    }
}
