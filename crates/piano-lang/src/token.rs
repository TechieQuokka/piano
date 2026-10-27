//! Tokens produced by the lexer.

use piano_core::{Accidental, Letter, SrcSpan};

/// A spelled note as written, before octave resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteName {
    pub letter: Letter,
    pub accidental: Accidental,
    pub octave: Option<i32>,
}

/// A mark written after a note: how it is played.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// `'`
    Staccato,
    /// `''`
    Staccatissimo,
    /// `^`
    Accent,
    /// `_`
    Tenuto,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Note(NoteName),
    Rest,
    /// `/8.` — raw denominator, validated by the parser.
    Len {
        denom: u32,
        dots: u8,
    },
    Tie,
    Mark(Mark),
    Word(String),
    Number(i64),
    Fraction(i64, i64),
    Bar,
    RepeatStart,
    RepeatEnd,
    Volta(u32),
    Percent,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Comma,
    Arrow,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: SrcSpan,
    /// The source text of the token.
    pub text: String,
}
