//! Compiles the written pieces and compares their notes with what the score asks for.

use std::collections::BTreeMap;

use piano_core::{Severity, TICKS_PER_QUARTER, Tick};
use piano_lang::{CompileOptions, compile_song};

use crate::rational::Q;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Check {
    /// Notes the score asks for.
    pub expected: usize,
    pub matched: usize,
    /// (start in quarters, MIDI) of notes missing from the compiled song.
    pub missing: Vec<(f64, i32)>,
    /// Compiled notes the score does not have.
    pub extra: usize,
    /// Errors and warnings of the compiled song.
    pub alerts: Vec<String>,
}

impl Check {
    pub fn ok(&self) -> bool {
        self.missing.is_empty() && self.extra == 0 && self.alerts.is_empty()
    }
}

fn ticks(q: Q) -> Option<Tick> {
    let t = q * Q::int(TICKS_PER_QUARTER as i64);
    (t.is_integer() && t.numer() >= 0).then(|| t.numer() as Tick)
}

pub fn check(pieces: &[(String, String)], song: &str, expected: &[(Q, i32, Q)]) -> Check {
    let load = |name: &str| {
        pieces
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| t.clone())
    };
    let compiled = compile_song(song, &load, &CompileOptions::default());
    let mut want: BTreeMap<(Tick, i32, Tick), usize> = BTreeMap::new();
    let mut missing = Vec::new();
    for &(start, midi, len) in expected {
        match (ticks(start), ticks(len)) {
            (Some(s), Some(l)) => *want.entry((s, midi, l)).or_default() += 1,
            _ => missing.push((start.to_f64(), midi)),
        }
    }
    let mut extra = 0;
    for n in &compiled.score.notes {
        let key = (n.start, i32::from(n.pitch.midi()), n.len);
        match want.get_mut(&key) {
            Some(c) if *c > 0 => *c -= 1,
            _ => extra += 1,
        }
    }
    for (&(s, midi, _), &c) in &want {
        for _ in 0..c {
            missing.push((s as f64 / TICKS_PER_QUARTER as f64, midi));
        }
    }
    missing.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Check {
        expected: expected.len(),
        matched: expected.len() - missing.len(),
        missing,
        extra,
        alerts: compiled
            .alerts
            .iter()
            .filter(|a| a.severity != Severity::Hint)
            .map(|a| a.located("song"))
            .collect(),
    }
}
