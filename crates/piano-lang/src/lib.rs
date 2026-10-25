//! The piano language: turns source text into a `Score` plus alerts.
//!
//! ```text
//! text ─▶ lexer ─▶ parser ─▶ resolve ─▶ expand (+ builtins) ─▶ Score
//! ```
//!
//! Compilation never fails: problems become alerts and the rest still plays.

pub mod ast;
pub mod builtins;
pub mod expand;
pub mod lexer;
pub mod linemap;
pub mod parser;
pub mod resolve;
pub mod song;
pub mod token;
pub mod writer;

use piano_core::{Alert, Score, TempoEvent, TimeSigEvent};

pub use builtins::{BUILTINS, BuiltinInfo};
pub use linemap::{line_at, line_range, tick_at};
pub use resolve::{CompileOptions, State};
pub use song::compile_song;
pub use writer::{chord_text, note_text, rest_text};

use expand::Track;

/// Result of compiling a piece or a song.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Compiled {
    pub score: Score,
    /// Sorted by position; duplicates removed.
    pub alerts: Vec<Alert>,
}

pub fn compile_piece(src: &str) -> Compiled {
    compile_piece_with(src, &CompileOptions::default())
}

pub fn compile_piece_with(src: &str, opts: &CompileOptions) -> Compiled {
    let mut alerts = Vec::new();
    let tokens = lexer::lex(src);
    let nodes = parser::parse(&tokens, &mut alerts);
    let (resolved, _) = resolve::resolve(&nodes, opts, None, &mut alerts);
    let track = expand::layout(&resolved, opts.time, &mut alerts);
    finish(track, opts, alerts)
}

/// Inherited state just before position (`line`, `col`), for the Ref view and for
/// writing new notes at the cursor.
pub fn state_at(src: &str, line: u32, col: u32, opts: &CompileOptions) -> State {
    let mut alerts = Vec::new();
    let nodes = parser::parse(&lexer::lex(src), &mut alerts);
    resolve::resolve(&nodes, opts, Some((line, col)), &mut alerts).1
}

fn finish(track: Track, opts: &CompileOptions, mut alerts: Vec<Alert>) -> Compiled {
    let mut score = Score {
        notes: track.notes,
        pedals: track.pedals,
        tempos: track.tempos,
        time_sigs: track.time_sigs,
        end: track.len,
    };
    if !score.tempos.iter().any(|t| t.at == 0) {
        score.tempos.insert(
            0,
            TempoEvent {
                at: 0,
                bpm: opts.tempo,
            },
        );
    }
    if !score.time_sigs.iter().any(|t| t.at == 0) {
        score.time_sigs.insert(
            0,
            TimeSigEvent {
                at: 0,
                sig: opts.time,
            },
        );
    }
    score.finish();
    // Keep the last tempo / time signature written at the same tick.
    dedup_last(&mut score.tempos, |t| t.at);
    dedup_last(&mut score.time_sigs, |t| t.at);
    alerts.sort_by_key(|a| (a.span.line, a.span.col));
    alerts.dedup();
    Compiled { score, alerts }
}

fn dedup_last<T, K: PartialEq>(v: &mut Vec<T>, key: impl Fn(&T) -> K) {
    let mut i = 0;
    while i + 1 < v.len() {
        if key(&v[i]) == key(&v[i + 1]) {
            v.remove(i);
        } else {
            i += 1;
        }
    }
}
