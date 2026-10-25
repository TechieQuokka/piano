//! `up`, `down`, `seq`.

use piano_core::{Alert, Pitch};

use super::midi_name;
use crate::expand::Track;

pub fn transpose(mut track: Track, semitones: i32, alerts: &mut Vec<Alert>) -> Track {
    for note in &mut track.notes {
        let target = i32::from(note.pitch.midi()) + semitones;
        let (pitch, folded) = Pitch::fold(target);
        if folded {
            alerts.push(
                Alert::warning(note.src, format!("{} folded to {pitch}", midi_name(target)))
                    .with_fix("(outside A0-C8)"),
            );
        }
        note.pitch = pitch;
    }
    track
}

pub fn seq(track: Track, times: u32, step: i32, alerts: &mut Vec<Alert>) -> Track {
    let mut out = Track::default();
    for k in 0..times {
        let shifted = transpose(track.clone(), step * k as i32, alerts);
        let at = out.len;
        out.append(shifted, at);
    }
    out
}
