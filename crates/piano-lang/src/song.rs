//! `song.piano`: an arrangement of independent pieces.
//!
//! ```text
//! p1 > bridge1 > p4 > fade > p7 > r/2 > up(2){ p1 } > ped > ending
//! ```

use piano_core::{
    Alert, Duration, PedalEvent, PedalKind, SrcSpan, TICKS_PER_QUARTER, TICKS_PER_WHOLE,
    TempoEvent, Tick, TimeSigEvent,
};

use crate::ast::Arg;
use crate::builtins::{Builtin, clamp_velocity};
use crate::expand::Track;
use crate::lexer::lex;
use crate::resolve::CompileOptions;
use crate::token::{Tok, Token};
use crate::{Compiled, compile_piece_with, finish};

/// Velocity factor reached at the end of a `fade`.
const FADE_FLOOR: f64 = 0.3;

#[derive(Debug, Clone)]
enum El {
    Piece(String, SrcSpan),
    Fade(SrcSpan),
    Ped(SrcSpan),
    Rest(Tick),
    Func(Builtin, Vec<El>),
}

/// Compiles a song file. `load(name)` returns the source of `name.piano`.
pub fn compile_song(
    src: &str,
    load: &dyn Fn(&str) -> Option<String>,
    opts: &CompileOptions,
) -> Compiled {
    let mut alerts = Vec::new();
    let tokens = lex(src);
    let mut pos = 0;
    let els = parse(&tokens, &mut pos, false, opts, &mut alerts);
    let track = lay(&els, load, opts, &mut alerts);
    finish(track, opts, alerts)
}

fn parse(
    tokens: &[Token],
    pos: &mut usize,
    in_func: bool,
    opts: &CompileOptions,
    alerts: &mut Vec<Alert>,
) -> Vec<El> {
    let mut els = Vec::new();
    while let Some(t) = tokens.get(*pos) {
        *pos += 1;
        match &t.tok {
            Tok::Arrow => {}
            Tok::RBrace if in_func => return els,
            Tok::Rest => {
                let mut len = opts.len;
                if let Some(Token {
                    tok: Tok::Len { denom, dots },
                    ..
                }) = tokens.get(*pos)
                {
                    *pos += 1;
                    match u8::try_from(*denom)
                        .ok()
                        .and_then(|d| Duration::new(d, *dots))
                    {
                        Some(d) => len = d,
                        None => alerts.push(Alert::error(t.span, "invalid rest length")),
                    }
                }
                els.push(El::Rest(len.ticks()));
            }
            Tok::Word(w) if w.eq_ignore_ascii_case("fade") => els.push(El::Fade(t.span)),
            Tok::Word(w) if w.eq_ignore_ascii_case("ped") => els.push(El::Ped(t.span)),
            Tok::Word(w)
                if matches!(
                    tokens.get(*pos),
                    Some(Token {
                        tok: Tok::LParen,
                        ..
                    })
                ) =>
            {
                *pos += 1;
                let args = parse_args(tokens, pos, alerts);
                if !matches!(
                    tokens.get(*pos),
                    Some(Token {
                        tok: Tok::LBrace,
                        ..
                    })
                ) {
                    alerts.push(Alert::error(
                        t.span,
                        format!("`{w}(...)` needs a body `{{ ... }}`"),
                    ));
                    continue;
                }
                *pos += 1;
                let body = parse(tokens, pos, true, opts, alerts);
                match Builtin::from_args(&w.to_ascii_lowercase(), &args, t.span, alerts) {
                    Some(b) => els.push(El::Func(b, body)),
                    None => els.extend(body),
                }
            }
            Tok::Word(_) | Tok::Note(_) => els.push(El::Piece(t.text.clone(), t.span)),
            _ => alerts.push(Alert::error(
                t.span,
                format!("unexpected `{}` in song, expected a piece name", t.text),
            )),
        }
    }
    if in_func {
        alerts.push(Alert::error(
            tokens.last().map_or_else(SrcSpan::default, |t| t.span),
            "missing `}` in song",
        ));
    }
    els
}

fn parse_args(tokens: &[Token], pos: &mut usize, alerts: &mut Vec<Alert>) -> Vec<Arg> {
    let mut args = Vec::new();
    while let Some(t) = tokens.get(*pos) {
        *pos += 1;
        match &t.tok {
            Tok::RParen => break,
            Tok::Comma => {}
            Tok::Number(n) => args.push(Arg::Int(*n)),
            Tok::Fraction(a, b) => args.push(Arg::Fraction(*a, *b)),
            Tok::Note(n) => args.push(Arg::Note(*n)),
            _ => alerts.push(Alert::error(
                t.span,
                format!("invalid argument `{}`", t.text),
            )),
        }
    }
    args
}

fn lay(
    els: &[El],
    load: &dyn Fn(&str) -> Option<String>,
    opts: &CompileOptions,
    alerts: &mut Vec<Alert>,
) -> Track {
    let mut track = Track::default();
    let mut last: Option<(Tick, Tick)> = None;
    for el in els {
        let now = track.len;
        match el {
            El::Piece(name, span) => {
                if name.eq_ignore_ascii_case("song") {
                    alerts.push(Alert::error(*span, "a song cannot contain itself"));
                    continue;
                }
                let Some(src) = load(name) else {
                    alerts.push(Alert::error(
                        *span,
                        format!("piece `{name}` not found ({name}.piano)"),
                    ));
                    continue;
                };
                let compiled = compile_piece_with(&src, opts);
                alerts.extend(compiled.alerts.into_iter().map(|a| Alert {
                    message: format!("[{name} line {}] {}", a.span.line, a.message),
                    span: *span,
                    piece: Some((name.clone(), a.span.line)),
                    ..a
                }));
                let piece = piece_track(compiled.score);
                last = Some((now, now + piece.len));
                track.append(piece, now);
            }
            El::Fade(span) => match last {
                Some((start, end)) => {
                    let window = (end - start).clamp(1, TICKS_PER_WHOLE);
                    let from = end - window;
                    for n in track
                        .notes
                        .iter_mut()
                        .filter(|n| (from..end).contains(&n.start))
                    {
                        let t = (n.start - from) as f64 / window as f64;
                        let factor = 1.0 - (1.0 - FADE_FLOOR) * t;
                        n.velocity =
                            clamp_velocity((f64::from(n.velocity) * factor).round() as i32);
                    }
                }
                None => alerts.push(Alert::warning(*span, "`fade` needs a piece before it")),
            },
            El::Ped(span) => match last {
                Some((start, end)) => {
                    let on = end.saturating_sub(TICKS_PER_QUARTER).max(start);
                    track.pedals.push(PedalEvent {
                        kind: PedalKind::Damper,
                        on: true,
                        at: on,
                        src: *span,
                    });
                    track.pedals.push(PedalEvent {
                        kind: PedalKind::Damper,
                        on: false,
                        at: end + TICKS_PER_QUARTER,
                        src: *span,
                    });
                }
                None => alerts.push(Alert::warning(*span, "`ped` needs a piece before it")),
            },
            El::Rest(len) => {
                track.len = now + len;
                last = None;
            }
            El::Func(builtin, body) => {
                let sub = lay(body, load, opts, alerts);
                let out = builtin.apply(sub, alerts);
                last = Some((now, now + out.len));
                track.append(out, now);
            }
        }
    }
    track
}

/// Converts a piece score into a track that is independent of what came before:
/// its starting tempo and time signature are explicit, and pedals end with it.
fn piece_track(score: piano_core::Score) -> Track {
    let mut track = Track {
        tempos: score.tempos.clone(),
        time_sigs: score.time_sigs.clone(),
        len: score.end,
        ..Default::default()
    };
    if !track.tempos.iter().any(|t| t.at == 0) {
        track.tempos.insert(
            0,
            TempoEvent {
                at: 0,
                bpm: score.tempo_at(0),
            },
        );
    }
    if !track.time_sigs.iter().any(|t| t.at == 0) {
        track.time_sigs.insert(
            0,
            TimeSigEvent {
                at: 0,
                sig: Default::default(),
            },
        );
    }
    let mut on = [None; 3];
    for p in &score.pedals {
        on[p.kind.index()] = p.on.then_some(p.src);
    }
    track.pedals = score.pedals;
    for kind in PedalKind::ALL {
        if let Some(src) = on[kind.index()] {
            track.pedals.push(PedalEvent {
                kind,
                on: false,
                at: score.end,
                src,
            });
        }
    }
    track.notes = score.notes;
    track.holds = score.holds;
    track
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn song(src: &str, pieces: &[(&str, &str)]) -> Compiled {
        let map: HashMap<String, String> = pieces
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        compile_song(src, &|n| map.get(n).cloned(), &CompileOptions::default())
    }

    #[test]
    fn concatenates_pieces() {
        let c = song("p1 > p2 > p1", &[("p1", "C D"), ("p2", "tempo 60\nE")]);
        assert!(c.alerts.is_empty(), "{:?}", c.alerts);
        let names: Vec<String> = c.score.notes.iter().map(|n| n.pitch.to_string()).collect();
        assert_eq!(names, ["C4", "D4", "E4", "C4", "D4"]);
        // p1 after p2 is back at the default tempo.
        assert_eq!(c.score.tempo_at(3 * TICKS_PER_QUARTER), 100);
        assert_eq!(c.score.tempo_at(2 * TICKS_PER_QUARTER), 60);
    }

    #[test]
    fn transforms_and_rests() {
        let c = song("p1 > r/2 > up(12){ p1 }", &[("p1", "C")]);
        assert!(c.alerts.is_empty(), "{:?}", c.alerts);
        let n = &c.score.notes;
        assert_eq!(n[1].pitch.to_string(), "C5");
        assert_eq!(n[1].start, 3 * TICKS_PER_QUARTER);
    }

    #[test]
    fn missing_piece() {
        let c = song("p1 > nope", &[("p1", "C")]);
        assert_eq!(c.alerts.len(), 1);
        assert_eq!(c.alerts[0].span.col, 6);
    }

    #[test]
    fn fade_lowers_end() {
        let c = song("p1 > fade", &[("p1", "C D E F")]);
        let v: Vec<u8> = c.score.notes.iter().map(|n| n.velocity).collect();
        assert_eq!(v[0], 80);
        assert!(v[3] < v[2] && v[2] < v[1]);
    }
}
