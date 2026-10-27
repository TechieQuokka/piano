//! Syntax tree as written, before inheritance is resolved.

use piano_core::{Duration, Dynamic, PedalKind, SrcSpan, TimeSig};

use crate::token::{Mark, NoteName};

/// Marks written after a note or chord: `'` `''` `^` `_`. They are not inherited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Marks {
    /// 0: none, 1: staccato `'`, 2: staccatissimo `''`.
    pub staccato: u8,
    pub accent: bool,
    pub tenuto: bool,
}

impl Marks {
    pub fn add(&mut self, mark: Mark) {
        match mark {
            Mark::Staccato => self.staccato = self.staccato.max(1),
            Mark::Staccatissimo => self.staccato = 2,
            Mark::Accent => self.accent = true,
            Mark::Tenuto => self.tenuto = true,
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
            | Node::Free { span, .. } => Some(*span),
            Node::Group(nodes) => nodes.first().and_then(Node::span),
        }
    }
}
