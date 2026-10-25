//! Converts a score (or a part of it) into time-ordered synth messages.

use piano_core::{PedalKind, Score, Tick};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgKind {
    On { key: u8, velocity: u8 },
    Off { key: u8 },
    Pedal { kind: PedalKind, on: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Msg {
    /// Sample offset from the start of playback.
    pub at: u64,
    pub kind: MsgKind,
}

impl MsgKind {
    /// Order at the same instant: releases first, then presses; a pedal pressed
    /// together with notes catches them (e.g. `sos on C2`).
    fn order(self) -> u8 {
        match self {
            MsgKind::Off { .. } => 0,
            MsgKind::Pedal { on: false, .. } => 1,
            MsgKind::On { .. } => 2,
            MsgKind::Pedal { on: true, .. } => 3,
        }
    }
}

/// Messages for playing `score` from `from` to `to` (exclusive, `None` = end).
///
/// Starting in the middle sounds the same as a full playback at that point:
/// notes still sounding at `from` are struck, and pedals down at `from` are pressed.
pub fn schedule(score: &Score, from: Tick, to: Option<Tick>, sample_rate: u32) -> (Vec<Msg>, u64) {
    let to = to.unwrap_or(score.end).min(score.end).max(from);
    let origin = score.seconds_at(from);
    let sample = |t: Tick| ((score.seconds_at(t) - origin) * f64::from(sample_rate)).round() as u64;
    let mut msgs = Vec::new();

    let mut pedal_state = [false; 3];
    for p in score.pedals.iter().filter(|p| p.at < from) {
        pedal_state[p.kind.index()] = p.on;
    }
    for kind in PedalKind::ALL {
        if pedal_state[kind.index()] {
            msgs.push(Msg {
                at: 0,
                kind: MsgKind::Pedal { kind, on: true },
            });
        }
    }
    for p in score.pedals.iter().filter(|p| p.at >= from && p.at < to) {
        msgs.push(Msg {
            at: sample(p.at),
            kind: MsgKind::Pedal {
                kind: p.kind,
                on: p.on,
            },
        });
    }
    for n in score
        .notes
        .iter()
        .filter(|n| n.start < to && n.end() > from)
    {
        let key = n.pitch.midi();
        msgs.push(Msg {
            at: sample(n.start.max(from)),
            kind: MsgKind::On {
                key,
                velocity: n.velocity,
            },
        });
        msgs.push(Msg {
            at: sample(n.end().min(to)),
            kind: MsgKind::Off { key },
        });
    }
    // Release every pedal at the end so a loop or the next play starts clean.
    let end = sample(to);
    for kind in PedalKind::ALL {
        msgs.push(Msg {
            at: end,
            kind: MsgKind::Pedal { kind, on: false },
        });
    }
    msgs.sort_by_key(|m| (m.at, m.kind.order()));
    (msgs, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use piano_core::{NoteEvent, PedalEvent, Pitch, SrcSpan, TICKS_PER_QUARTER as Q, TempoEvent};

    fn score() -> Score {
        let note = |midi, start, len| NoteEvent {
            pitch: Pitch::new(midi).unwrap(),
            start,
            len,
            velocity: 80,
            src: SrcSpan::default(),
        };
        let mut s = Score {
            notes: vec![note(48, 0, 4 * Q), note(60, Q, Q), note(62, 2 * Q, Q)],
            pedals: vec![PedalEvent {
                kind: PedalKind::Damper,
                on: true,
                at: 0,
                src: SrcSpan::default(),
            }],
            tempos: vec![TempoEvent { at: 0, bpm: 60 }],
            ..Default::default()
        };
        s.finish();
        s
    }

    #[test]
    fn start_in_the_middle_restores_state() {
        // 60 bpm: one quarter = 1 s = 100 samples at 100 Hz.
        let (msgs, end) = schedule(&score(), 2 * Q, None, 100);
        assert_eq!(end, 200);
        let firsts: Vec<_> = msgs.iter().take(3).map(|m| (m.at, m.kind)).collect();
        assert_eq!(
            firsts,
            [
                (
                    0,
                    MsgKind::On {
                        key: 48,
                        velocity: 80
                    }
                ),
                (
                    0,
                    MsgKind::On {
                        key: 62,
                        velocity: 80
                    }
                ),
                (
                    0,
                    MsgKind::Pedal {
                        kind: PedalKind::Damper,
                        on: true
                    }
                ),
            ]
        );
        assert!(!msgs.iter().any(|m| m.kind
            == MsgKind::On {
                key: 60,
                velocity: 80
            }));
    }

    #[test]
    fn range_cuts_notes() {
        let (msgs, end) = schedule(&score(), 0, Some(Q), 100);
        assert_eq!(end, 100);
        assert!(
            msgs.iter()
                .any(|m| m.kind == MsgKind::Off { key: 48 } && m.at == 100)
        );
        assert!(
            !msgs
                .iter()
                .any(|m| matches!(m.kind, MsgKind::On { key: 60, .. }))
        );
    }
}
