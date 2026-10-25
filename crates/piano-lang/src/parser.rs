//! Recursive-descent parser with error recovery: problems become alerts, never failures.

use piano_core::time::{DENOMINATORS, MAX_DOTS};
use piano_core::{Alert, Duration, Dynamic, PedalKind, SrcSpan, TimeSig};

use crate::ast::{Arg, Node};
use crate::builtins;
use crate::token::{Tok, Token};

/// Maximum nesting depth of builtin functions.
pub const MAX_DEPTH: usize = 32;

/// Highest repeat count accepted by `x n`.
const MAX_TIMES: i64 = 99;

pub fn parse(tokens: &[Token], alerts: &mut Vec<Alert>) -> Vec<Node> {
    let mut p = Parser {
        tokens,
        pos: 0,
        depth: 0,
        alerts,
    };
    let (nodes, _) = p.seq(Ctx::Top);
    nodes
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Top,
    Func,
    RepeatBody,
    Ending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)] // `RepeatEnd` names the `:|` token.
enum End {
    Eof,
    RBrace,
    RepeatEnd,
    Volta(u32),
    /// An ending stopped before a bar line or a new repeat (not consumed).
    Stop,
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    depth: usize,
    alerts: &'a mut Vec<Alert>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_tok(&self) -> Option<&Tok> {
        self.peek().map(|t| &t.tok)
    }

    fn bump(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        self.pos += 1;
        t
    }

    fn error(&mut self, span: SrcSpan, msg: impl Into<String>) {
        self.alerts.push(Alert::error(span, msg));
    }

    fn warning(&mut self, span: SrcSpan, msg: impl Into<String>) {
        self.alerts.push(Alert::warning(span, msg));
    }

    fn seq(&mut self, ctx: Ctx) -> (Vec<Node>, End) {
        let mut nodes = Vec::new();
        // Index after the last repeat block: an unmatched `:|` repeats from here.
        let mut repeat_mark = 0;
        loop {
            let Some(token) = self.peek().cloned() else {
                return (nodes, End::Eof);
            };
            match token.tok {
                Tok::RBrace => match ctx {
                    Ctx::Func => {
                        self.bump();
                        return (nodes, End::RBrace);
                    }
                    Ctx::RepeatBody | Ctx::Ending if self.depth > 0 => {
                        return (nodes, End::RBrace);
                    }
                    _ => {
                        self.bump();
                        self.error(token.span, "unmatched `}`");
                    }
                },
                Tok::RepeatEnd => match ctx {
                    Ctx::RepeatBody | Ctx::Ending => {
                        self.bump();
                        return (nodes, End::RepeatEnd);
                    }
                    Ctx::Top | Ctx::Func => {
                        self.bump();
                        let body: Vec<Node> = nodes.drain(repeat_mark..).collect();
                        self.alerts.push(Alert::hint(
                            token.span,
                            "`:|` without `|:` repeats from the start",
                        ));
                        let times = self.times();
                        nodes.push(Node::Repeat {
                            body,
                            endings: Vec::new(),
                            times,
                            span: token.span,
                        });
                        repeat_mark = nodes.len();
                    }
                },
                Tok::Volta(n) => match ctx {
                    Ctx::RepeatBody | Ctx::Ending => {
                        self.bump();
                        return (nodes, End::Volta(n));
                    }
                    _ => {
                        self.bump();
                        self.error(token.span, format!("`[{n}` outside a repeat"));
                    }
                },
                Tok::Bar | Tok::RepeatStart if ctx == Ctx::Ending => {
                    return (nodes, End::Stop);
                }
                Tok::RepeatStart => {
                    self.bump();
                    let node = self.repeat(token.span);
                    nodes.push(node);
                    repeat_mark = nodes.len();
                }
                _ => {
                    if let Some(node) = self.atom() {
                        nodes.push(node);
                    }
                }
            }
        }
    }

    fn repeat(&mut self, span: SrcSpan) -> Node {
        let (body, end) = self.seq(Ctx::RepeatBody);
        match end {
            End::RepeatEnd => {
                let times = self.times();
                Node::Repeat {
                    body,
                    endings: Vec::new(),
                    times,
                    span,
                }
            }
            End::Volta(first) => {
                if first != 1 {
                    self.warning(span, format!("first ending is `[{first}`, expected `[1`"));
                }
                let mut endings = Vec::new();
                loop {
                    let (ending, end) = self.seq(Ctx::Ending);
                    endings.push(ending);
                    match end {
                        End::RepeatEnd => match self.peek_tok() {
                            Some(Tok::Volta(_)) => {
                                self.bump();
                            }
                            _ => break,
                        },
                        End::Volta(_) => {}
                        End::Stop | End::Eof | End::RBrace => break,
                    }
                }
                let times = endings.len() as u32;
                Node::Repeat {
                    body,
                    endings,
                    times,
                    span,
                }
            }
            End::Eof | End::RBrace | End::Stop => {
                self.error(span, "`|:` has no matching `:|`");
                Node::Group(body)
            }
        }
    }

    /// Optional `x n` / `xn` after `:|`. Defaults to 2.
    fn times(&mut self) -> u32 {
        let Some(token) = self.peek().cloned() else {
            return 2;
        };
        let n = match &token.tok {
            Tok::Word(w) if w.eq_ignore_ascii_case("x") => {
                self.bump();
                match self.peek().cloned() {
                    Some(Token {
                        tok: Tok::Number(n),
                        ..
                    }) => {
                        self.bump();
                        n
                    }
                    _ => {
                        self.error(token.span, "`x` needs a number, e.g. `x3`");
                        return 2;
                    }
                }
            }
            Tok::Word(w) => match repeat_count(w) {
                Some(n) => {
                    self.bump();
                    n
                }
                None => return 2,
            },
            _ => return 2,
        };
        if !(1..=MAX_TIMES).contains(&n) {
            self.warning(
                token.span,
                format!("repeat count must be 1-{MAX_TIMES}, using 2"),
            );
            return 2;
        }
        n as u32
    }

    fn atom(&mut self) -> Option<Node> {
        let token = self.bump()?.clone();
        let span = token.span;
        match token.tok {
            Tok::Note(name) => {
                let (len, tie) = self.len_tie();
                Some(Node::Note {
                    name,
                    len,
                    tie,
                    span,
                })
            }
            Tok::Rest => {
                let (len, tie) = self.len_tie();
                if tie {
                    self.warning(span, "rests cannot be tied");
                }
                Some(Node::Rest { len, span })
            }
            Tok::LBrace => Some(self.chord(span)),
            Tok::Word(w) => self.word(&w, span),
            Tok::Bar => Some(Node::Bar(span)),
            Tok::Percent => Some(Node::BarRepeat(span)),
            Tok::Len { .. } => {
                self.error(span, "length without a note");
                None
            }
            Tok::Tie => {
                self.warning(span, "`~` must follow a note or chord");
                None
            }
            Tok::Arrow => {
                self.error(span, "`>` is only used in song files");
                None
            }
            Tok::Number(_) | Tok::Fraction(..) => {
                self.error(span, format!("unexpected number `{}`", token.text));
                None
            }
            _ => {
                self.error(span, format!("unknown symbol `{}`", token.text));
                None
            }
        }
    }

    fn len_tie(&mut self) -> (Option<Duration>, bool) {
        let mut len = None;
        if let Some(Token {
            tok: Tok::Len { denom, dots },
            span,
            ..
        }) = self.peek().cloned()
        {
            self.bump();
            len = self.duration(denom, dots, span);
        }
        let tie = matches!(self.peek_tok(), Some(Tok::Tie));
        if tie {
            self.bump();
        }
        (len, tie)
    }

    fn duration(&mut self, denom: u32, dots: u8, span: SrcSpan) -> Option<Duration> {
        let valid = u8::try_from(denom)
            .ok()
            .filter(|d| DENOMINATORS.contains(d));
        let Some(denom) = valid else {
            self.error(
                span,
                format!("invalid length `/{denom}`, use /1 /2 /4 /8 /16 /32 /64"),
            );
            return None;
        };
        if dots > MAX_DOTS {
            self.warning(span, format!("at most {MAX_DOTS} dots, using {MAX_DOTS}"));
        }
        Duration::new(denom, dots.min(MAX_DOTS))
    }

    fn chord(&mut self, span: SrcSpan) -> Node {
        let mut notes = Vec::new();
        loop {
            match self.peek().cloned() {
                None => {
                    self.error(span, "chord `{` has no matching `}`");
                    break;
                }
                Some(Token {
                    tok: Tok::RBrace, ..
                }) => {
                    self.bump();
                    break;
                }
                Some(Token {
                    tok: Tok::Note(name),
                    span: s,
                    ..
                }) => {
                    self.bump();
                    notes.push((name, s));
                    if let Some(Tok::Len { .. }) = self.peek_tok() {
                        let s = self.bump().map(|t| t.span).unwrap_or(s);
                        self.warning(s, "put the chord length after `}`");
                    }
                }
                Some(t) => {
                    self.bump();
                    self.error(
                        t.span,
                        format!("only notes are allowed in a chord, found `{}`", t.text),
                    );
                }
            }
        }
        if notes.is_empty() {
            self.warning(span, "empty chord");
        }
        let (len, tie) = self.len_tie();
        Node::Chord {
            notes,
            len,
            tie,
            span,
        }
    }

    fn word(&mut self, w: &str, span: SrcSpan) -> Option<Node> {
        if let Some(d) = Dynamic::parse(w) {
            return Some(Node::Dynamic(d, span));
        }
        let lower = w.to_ascii_lowercase();
        match lower.as_str() {
            "cresc" => return Some(Node::Cresc(span)),
            "dim" => return Some(Node::Dim(span)),
            "tempo" => return self.tempo(span),
            "time" => return self.time(span),
            _ => {}
        }
        if let Some(kind) = PedalKind::parse(&lower) {
            return self.pedal(kind, span);
        }
        if lower == "x" || repeat_count(&lower).is_some() {
            self.warning(span, "repeat count without a repeat");
            if lower == "x" && matches!(self.peek_tok(), Some(Tok::Number(_))) {
                self.bump();
            }
            return None;
        }
        if lower == "voices" && matches!(self.peek_tok(), Some(Tok::LBrace)) {
            return Some(self.voices(span));
        }
        if matches!(self.peek_tok(), Some(Tok::LParen)) {
            return self.func(w, span);
        }
        self.error(span, format!("unknown word `{w}`"));
        None
    }

    fn tempo(&mut self, span: SrcSpan) -> Option<Node> {
        match self.peek().cloned() {
            Some(Token {
                tok: Tok::Number(n),
                span: s,
                ..
            }) => {
                self.bump();
                if (20..=400).contains(&n) {
                    Some(Node::Tempo(n as u16, span))
                } else {
                    self.error(s, "tempo must be 20-400");
                    None
                }
            }
            _ => {
                self.error(span, "`tempo` needs a number, e.g. `tempo 100`");
                None
            }
        }
    }

    fn time(&mut self, span: SrcSpan) -> Option<Node> {
        match self.peek().cloned() {
            Some(Token {
                tok: Tok::Fraction(beats, unit),
                span: s,
                ..
            }) => {
                self.bump();
                let unit_ok = u8::try_from(unit).ok().filter(|u| DENOMINATORS.contains(u));
                match (
                    u8::try_from(beats).ok().filter(|b| (1..=32).contains(b)),
                    unit_ok,
                ) {
                    (Some(beats), Some(unit)) => Some(Node::Time(TimeSig { beats, unit }, span)),
                    _ => {
                        self.error(s, "invalid time signature, e.g. `time 3/4`");
                        None
                    }
                }
            }
            _ => {
                self.error(span, "`time` needs a signature, e.g. `time 4/4`");
                None
            }
        }
    }

    fn pedal(&mut self, kind: PedalKind, span: SrcSpan) -> Option<Node> {
        let on = match self.peek_tok() {
            Some(Tok::Word(w)) if w.eq_ignore_ascii_case("on") => true,
            Some(Tok::Word(w)) if w.eq_ignore_ascii_case("off") => false,
            _ => {
                self.error(span, format!("`{}` needs `on` or `off`", kind.keyword()));
                return None;
            }
        };
        self.bump();
        Some(Node::Pedal { kind, on, span })
    }

    fn func(&mut self, name: &str, span: SrcSpan) -> Option<Node> {
        self.bump(); // (
        let mut args = Vec::new();
        loop {
            let Some(t) = self.bump().cloned() else {
                self.error(span, "`(` has no matching `)`");
                return None;
            };
            match t.tok {
                Tok::RParen => break,
                Tok::Comma => {}
                Tok::Number(n) => args.push(Arg::Int(n)),
                Tok::Fraction(a, b) => args.push(Arg::Fraction(a, b)),
                Tok::Note(n) => args.push(Arg::Note(n)),
                _ => self.error(t.span, format!("invalid argument `{}`", t.text)),
            }
        }
        if !matches!(self.peek_tok(), Some(Tok::LBrace)) {
            self.error(span, format!("`{name}(...)` needs a body `{{ ... }}`"));
            return None;
        }
        self.bump();
        self.depth += 1;
        let too_deep = self.depth > MAX_DEPTH;
        let (body, end) = self.seq(Ctx::Func);
        self.depth -= 1;
        if end != End::RBrace {
            self.error(span, format!("`{name}{{` has no matching `}}`"));
        }
        if too_deep {
            self.warning(
                span,
                format!("functions nested deeper than {MAX_DEPTH}, `{name}` ignored"),
            );
            return Some(Node::Group(body));
        }
        if builtins::info(name).is_none() {
            self.error(span, format!("unknown function `{name}`, see `man fn`"));
            return Some(Node::Group(body));
        }
        Some(Node::Func {
            name: name.to_ascii_lowercase(),
            args,
            body,
            span,
        })
    }

    /// `voices{ ... }{ ... }`. A `{` right after a body starts another voice, unless
    /// its `}` is followed by a length or `~`: then it is a chord.
    fn voices(&mut self, span: SrcSpan) -> Node {
        let mut voices = Vec::new();
        while matches!(self.peek_tok(), Some(Tok::LBrace))
            && (voices.is_empty() || !self.chord_ahead())
        {
            self.bump();
            self.depth += 1;
            let (body, end) = self.seq(Ctx::Func);
            self.depth -= 1;
            if end != End::RBrace {
                self.error(span, "`voices{` has no matching `}`");
            }
            voices.push(body);
        }
        if self.depth >= MAX_DEPTH {
            self.warning(
                span,
                format!("blocks nested deeper than {MAX_DEPTH}, `voices` ignored"),
            );
            return Node::Group(voices.into_iter().flatten().collect());
        }
        if voices.len() == 1 {
            self.alerts.push(Alert::hint(
                span,
                "`voices` has one voice, add another `{ ... }`",
            ));
        }
        Node::Voices { voices, span }
    }

    /// Whether the `{` at the cursor closes with a `}` followed by a length or `~`.
    fn chord_ahead(&self) -> bool {
        let mut depth = 0;
        for (i, t) in self.tokens.iter().enumerate().skip(self.pos) {
            match t.tok {
                Tok::LBrace => depth += 1,
                Tok::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return matches!(
                            self.tokens.get(i + 1).map(|t| &t.tok),
                            Some(Tok::Len { .. } | Tok::Tie)
                        );
                    }
                }
                _ => {}
            }
        }
        false
    }
}

/// `x3` → 3.
fn repeat_count(w: &str) -> Option<i64> {
    let rest = w.strip_prefix('x').or_else(|| w.strip_prefix('X'))?;
    rest.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn parse_src(src: &str) -> (Vec<Node>, Vec<Alert>) {
        let mut alerts = Vec::new();
        let nodes = parse(&lex(src), &mut alerts);
        (nodes, alerts)
    }

    #[test]
    fn repeat_with_count() {
        let (nodes, alerts) = parse_src("|: C D :| x3");
        assert!(alerts.is_empty(), "{alerts:?}");
        assert!(matches!(&nodes[0], Node::Repeat { body, times: 3, .. } if body.len() == 2));
    }

    #[test]
    fn voltas() {
        let (nodes, alerts) = parse_src("|: C D [1 E :| [2 G | A");
        assert!(alerts.is_empty(), "{alerts:?}");
        match &nodes[0] {
            Node::Repeat {
                body,
                endings,
                times,
                ..
            } => {
                assert_eq!(body.len(), 2);
                assert_eq!(endings.len(), 2);
                assert_eq!(*times, 2);
            }
            n => panic!("{n:?}"),
        }
        assert!(matches!(nodes[1], Node::Bar(_)));
    }

    #[test]
    fn nested_functions() {
        let (nodes, alerts) = parse_src("up(12){ accent(1/3){ C D E } }");
        assert!(alerts.is_empty(), "{alerts:?}");
        match &nodes[0] {
            Node::Func { name, body, .. } => {
                assert_eq!(name, "up");
                assert!(matches!(&body[0], Node::Func { name, .. } if name == "accent"));
            }
            n => panic!("{n:?}"),
        }
    }

    #[test]
    fn errors_do_not_stop_parsing() {
        let (nodes, alerts) = parse_src("C /3 $ D foo E");
        assert_eq!(alerts.len(), 3, "{alerts:?}");
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn chord_and_pedal() {
        let (nodes, alerts) = parse_src("ped on {C E G}/2~ ped off");
        assert!(alerts.is_empty(), "{alerts:?}");
        assert!(matches!(&nodes[1], Node::Chord { notes, tie: true, .. } if notes.len() == 3));
    }

    #[test]
    fn unmatched_repeat_start() {
        let (_, alerts) = parse_src("|: C D");
        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn voices_bodies() {
        let (nodes, alerts) = parse_src("voices{ C D }\n{ E/2 } {C E G}/2 F");
        assert!(alerts.is_empty(), "{alerts:?}");
        assert!(matches!(&nodes[0], Node::Voices { voices, .. } if voices.len() == 2));
        assert!(matches!(&nodes[1], Node::Chord { .. }));
        assert!(matches!(&nodes[2], Node::Note { .. }));
    }

    #[test]
    fn voices_with_one_voice() {
        let (nodes, alerts) = parse_src("voices{ C D }");
        assert_eq!(alerts.len(), 1);
        assert!(matches!(&nodes[0], Node::Voices { voices, .. } if voices.len() == 1));
    }

    #[test]
    fn unknown_function() {
        let (nodes, alerts) = parse_src("foo(1){ C }");
        assert_eq!(alerts.len(), 1);
        assert!(matches!(nodes[0], Node::Group(_)));
    }
}
