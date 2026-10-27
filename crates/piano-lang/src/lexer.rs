//! Lenient lexer: never fails, unknown characters become `Tok::Unknown`.

use piano_core::{Accidental, Letter, SrcSpan};

use crate::token::{Mark, NoteName, Tok, Token};

pub fn lex(src: &str) -> Vec<Token> {
    let mut out = Vec::new();
    for (li, line) in src.lines().enumerate() {
        lex_line(li as u32 + 1, line, &mut out);
    }
    out
}

fn lex_line(line_no: u32, line: &str, out: &mut Vec<Token>) {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let at = |i: usize| chars.get(i).copied();
    let mut i = 0;
    while i < n {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        let tok = match c {
            '/' if at(i + 1) == Some('/') => break,
            '/' => {
                i += 1;
                let digits = take_while(&chars, &mut i, |c| c.is_ascii_digit());
                if digits.is_empty() {
                    Tok::Unknown
                } else {
                    let dots = take_while(&chars, &mut i, |c| c == '.').len();
                    Tok::Len {
                        denom: digits.parse().unwrap_or(u32::MAX),
                        dots: dots.min(u8::MAX as usize) as u8,
                    }
                }
            }
            '|' if at(i + 1) == Some(':') => {
                i += 2;
                Tok::RepeatStart
            }
            '|' => {
                i += 1;
                Tok::Bar
            }
            ':' if at(i + 1) == Some('|') => {
                i += 2;
                Tok::RepeatEnd
            }
            '[' => {
                i += 1;
                let digits = take_while(&chars, &mut i, |c| c.is_ascii_digit());
                match digits.parse() {
                    Ok(n) => Tok::Volta(n),
                    Err(_) => Tok::Unknown,
                }
            }
            c if c.is_ascii_digit()
                || ((c == '-' || c == '+') && at(i + 1).is_some_and(|d| d.is_ascii_digit())) =>
            {
                i += 1;
                take_while(&chars, &mut i, |c| c.is_ascii_digit());
                let num: i64 = chars[start..i]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .unwrap_or(i64::MAX);
                if at(i) == Some('/') && at(i + 1).is_some_and(|d| d.is_ascii_digit()) {
                    i += 1;
                    let den = take_while(&chars, &mut i, |c| c.is_ascii_digit());
                    Tok::Fraction(num, den.parse().unwrap_or(i64::MAX))
                } else {
                    Tok::Number(num)
                }
            }
            '\'' if at(i + 1) == Some('\'') => {
                i += 2;
                Tok::Mark(Mark::Staccatissimo)
            }
            // `_` alone is a tenuto mark; `_intro` is a word (a piece name).
            c if c.is_alphabetic()
                || (c == '_' && at(i + 1).is_some_and(|d| d.is_alphanumeric())) =>
            {
                let mut word = take_while(&chars, &mut i, |c| {
                    c.is_alphanumeric() || c == '_' || c == '#'
                });
                // `C_` is a note with a tenuto mark, not a word.
                let bare = word.trim_end_matches('_');
                if bare.len() < word.len() && (parse_note(bare).is_some() || bare == "r") {
                    i -= word.len() - bare.len();
                    word.truncate(bare.len());
                }
                classify(&word)
            }
            _ => {
                i += 1;
                match c {
                    '%' => Tok::Percent,
                    '{' => Tok::LBrace,
                    '}' => Tok::RBrace,
                    '(' => Tok::LParen,
                    ')' => Tok::RParen,
                    ',' => Tok::Comma,
                    '~' => Tok::Tie,
                    '>' => Tok::Arrow,
                    '\'' => Tok::Mark(Mark::Staccato),
                    '^' => Tok::Mark(Mark::Accent),
                    '_' => Tok::Mark(Mark::Tenuto),
                    _ => Tok::Unknown,
                }
            }
        };
        let text: String = chars[start..i].iter().collect();
        out.push(Token {
            tok,
            span: SrcSpan::new(line_no, start as u32 + 1, (i - start) as u32),
            text,
        });
    }
}

fn take_while(chars: &[char], i: &mut usize, f: impl Fn(char) -> bool) -> String {
    let start = *i;
    while *i < chars.len() && f(chars[*i]) {
        *i += 1;
    }
    chars[start..*i].iter().collect()
}

/// Notes are `[A-G][#|b]?[0-9]?`, case-insensitive, except that a lone lowercase
/// `f` is the dynamic mark (forte). `r` is a rest; everything else is a word.
fn classify(word: &str) -> Tok {
    if word.eq_ignore_ascii_case("r") {
        return Tok::Rest;
    }
    if word == "f" {
        return Tok::Word(word.to_string());
    }
    parse_note(word).map_or_else(|| Tok::Word(word.to_string()), Tok::Note)
}

fn parse_note(word: &str) -> Option<NoteName> {
    let mut chars = word.chars();
    let letter = Letter::from_char(chars.next()?)?;
    let rest: Vec<char> = chars.collect();
    let (accidental, rest) = match rest.first() {
        Some('#') => (Accidental::Sharp, &rest[1..]),
        Some('b') => (Accidental::Flat, &rest[1..]),
        _ => (Accidental::Natural, &rest[..]),
    };
    let octave = match rest {
        [] => None,
        [d] if d.is_ascii_digit() => Some(d.to_digit(10)? as i32),
        _ => return None,
    };
    Some(NoteName {
        letter,
        accidental,
        octave,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Tok> {
        lex(src).into_iter().map(|t| t.tok).collect()
    }

    fn note(letter: Letter, accidental: Accidental, octave: Option<i32>) -> Tok {
        Tok::Note(NoteName {
            letter,
            accidental,
            octave,
        })
    }

    #[test]
    fn notes_and_lengths() {
        assert_eq!(
            toks("C#4/8. bb e"),
            vec![
                note(Letter::C, Accidental::Sharp, Some(4)),
                Tok::Len { denom: 8, dots: 1 },
                note(Letter::B, Accidental::Flat, None),
                note(Letter::E, Accidental::Natural, None),
            ]
        );
    }

    #[test]
    fn lowercase_f_is_forte() {
        assert_eq!(
            toks("f F f4"),
            vec![
                Tok::Word("f".into()),
                note(Letter::F, Accidental::Natural, None),
                note(Letter::F, Accidental::Natural, Some(4)),
            ]
        );
    }

    #[test]
    fn structure_and_comments() {
        assert_eq!(
            toks("|: C :| x3 [1 % // comment G"),
            vec![
                Tok::RepeatStart,
                note(Letter::C, Accidental::Natural, None),
                Tok::RepeatEnd,
                Tok::Word("x3".into()),
                Tok::Volta(1),
                Tok::Percent,
            ]
        );
    }

    #[test]
    fn numbers_and_fractions() {
        assert_eq!(
            toks("time 3/4 down(-2) accent(1/3)"),
            vec![
                Tok::Word("time".into()),
                Tok::Fraction(3, 4),
                Tok::Word("down".into()),
                Tok::LParen,
                Tok::Number(-2),
                Tok::RParen,
                Tok::Word("accent".into()),
                Tok::LParen,
                Tok::Fraction(1, 3),
                Tok::RParen,
            ]
        );
    }

    #[test]
    fn spans_are_one_based() {
        let t = lex("  C4\n G");
        assert_eq!(t[0].span, SrcSpan::new(1, 3, 2));
        assert_eq!(t[1].span, SrcSpan::new(2, 2, 1));
    }

    #[test]
    fn marks() {
        let c = note(Letter::C, Accidental::Natural, None);
        let c4 = note(Letter::C, Accidental::Natural, Some(4));
        let m = Tok::Mark;
        assert_eq!(
            toks("C' C4/8'' C^ C_ C4_ r_ _intro"),
            vec![
                c.clone(),
                m(Mark::Staccato),
                c4.clone(),
                Tok::Len { denom: 8, dots: 0 },
                m(Mark::Staccatissimo),
                c.clone(),
                m(Mark::Accent),
                c,
                m(Mark::Tenuto),
                c4,
                m(Mark::Tenuto),
                Tok::Rest,
                m(Mark::Tenuto),
                Tok::Word("_intro".into()),
            ]
        );
    }

    #[test]
    fn unknown_chars() {
        assert_eq!(toks("C $ D"), {
            vec![
                note(Letter::C, Accidental::Natural, None),
                Tok::Unknown,
                note(Letter::D, Accidental::Natural, None),
            ]
        });
    }
}
