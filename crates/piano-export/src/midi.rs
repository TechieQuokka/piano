//! Score → Standard MIDI File (format 0, one track).

use midly::num::{u4, u7, u15, u24, u28};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use piano_core::{PedalKind, Score, TICKS_PER_QUARTER, Tick};

const CHANNEL: u8 = 0;

fn controller(kind: PedalKind) -> u8 {
    match kind {
        PedalKind::Damper => 64,
        PedalKind::Sostenuto => 66,
        PedalKind::Soft => 67,
    }
}

/// Encodes `score` as a `.mid` file. Ticks map 1:1 (PPQ = `TICKS_PER_QUARTER`).
pub fn midi_bytes(score: &Score) -> Vec<u8> {
    // (tick, order, kind): order mirrors playback: releases, then presses; the soft
    // pedal goes down before the notes it softens, damper and sostenuto after them.
    let mut events: Vec<(Tick, u8, TrackEventKind<'static>)> = Vec::new();
    let midi = |message| TrackEventKind::Midi {
        channel: u4::new(CHANNEL),
        message,
    };
    events.push((0, 0, TrackEventKind::Meta(MetaMessage::TrackName(b"piano"))));
    events.push((
        0,
        0,
        midi(MidiMessage::ProgramChange {
            program: u7::new(0),
        }),
    ));
    for t in &score.tempos {
        let micros = 60_000_000 / u32::from(t.bpm.max(1));
        events.push((
            t.at,
            0,
            TrackEventKind::Meta(MetaMessage::Tempo(u24::new(micros))),
        ));
    }
    for t in &score.time_sigs {
        let denom_pow = t.sig.unit.trailing_zeros() as u8;
        events.push((
            t.at,
            0,
            TrackEventKind::Meta(MetaMessage::TimeSignature(t.sig.beats, denom_pow, 24, 8)),
        ));
    }
    for n in &score.notes {
        let key = u7::new(n.pitch.midi());
        events.push((
            n.start,
            4,
            midi(MidiMessage::NoteOn {
                key,
                vel: u7::new(n.velocity.min(127)),
            }),
        ));
        events.push((
            n.end(),
            1,
            midi(MidiMessage::NoteOff {
                key,
                vel: u7::new(0),
            }),
        ));
    }
    for p in &score.pedals {
        events.push((
            p.at,
            match (p.on, p.kind) {
                (false, _) => 2,
                (true, PedalKind::Soft) => 3,
                (true, _) => 5,
            },
            midi(MidiMessage::Controller {
                controller: u7::new(controller(p.kind)),
                value: u7::new(if p.on { 127 } else { 0 }),
            }),
        ));
    }
    events.sort_by_key(|(t, order, _)| (*t, *order));

    let mut track = Vec::with_capacity(events.len() + 1);
    let mut last = 0;
    for (t, _, kind) in events {
        track.push(TrackEvent {
            delta: u28::new((t - last) as u32),
            kind,
        });
        last = t;
    }
    let end = score.end.max(last);
    track.push(TrackEvent {
        delta: u28::new((end - last) as u32),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });

    let mut smf = Smf::new(Header::new(
        Format::SingleTrack,
        Timing::Metrical(u15::new(TICKS_PER_QUARTER as u16)),
    ));
    smf.tracks.push(track);
    let mut out = Vec::new();
    smf.write_std(&mut out)
        .expect("writing to a Vec cannot fail");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use piano_lang::compile_piece;

    #[test]
    fn round_trip_through_midly() {
        let score = compile_piece("tempo 120 ped on C4 E ped off {G C5}/2").score;
        let bytes = midi_bytes(&score);
        let smf = Smf::parse(&bytes).unwrap();
        let Timing::Metrical(ppq) = smf.header.timing else {
            panic!("timing");
        };
        assert_eq!(u32::from(ppq.as_int()), TICKS_PER_QUARTER as u32);
        let track = &smf.tracks[0];
        let ons = track
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    TrackEventKind::Midi {
                        message: MidiMessage::NoteOn { .. },
                        ..
                    }
                )
            })
            .count();
        assert_eq!(ons, 4);
        assert!(track.iter().any(|e| matches!(
            e.kind,
            TrackEventKind::Meta(MetaMessage::Tempo(t)) if t.as_int() == 500_000
        )));
        let total: u64 = track.iter().map(|e| u64::from(e.delta.as_int())).sum();
        assert_eq!(total, score.end);
    }

    #[test]
    fn soft_pedal_before_the_notes_damper_after() {
        let score = compile_piece("soft on ped on C4").score;
        let bytes = midi_bytes(&score);
        let smf = Smf::parse(&bytes).unwrap();
        let order: Vec<String> = smf.tracks[0]
            .iter()
            .filter_map(|e| match e.kind {
                TrackEventKind::Midi {
                    message: MidiMessage::NoteOn { .. },
                    ..
                } => Some("on".to_string()),
                TrackEventKind::Midi {
                    message: MidiMessage::Controller { controller, value },
                    ..
                } if value.as_int() > 0 => Some(format!("cc{controller}")),
                _ => None,
            })
            .collect();
        assert_eq!(order, ["cc67", "on", "cc64"]);
    }
}
