//! `accent`, `louder`, `softer`.

use super::{ACCENT_BOOST, AccentRule, clamp_velocity};
use crate::expand::Track;

pub fn accent(mut track: Track, rule: AccentRule) -> Track {
    match rule {
        AccentRule::Every(n) => {
            let mut onsets: Vec<_> = track.notes.iter().map(|n| n.start).collect();
            onsets.sort_unstable();
            onsets.dedup();
            for note in &mut track.notes {
                let k = onsets.binary_search(&note.start).unwrap_or(0);
                if k % n as usize == 0 {
                    note.velocity = clamp_velocity(i32::from(note.velocity) + ACCENT_BOOST);
                }
            }
        }
        AccentRule::PitchClass(pc) => {
            for note in track.notes.iter_mut().filter(|n| n.pitch.class() == pc) {
                note.velocity = clamp_velocity(i32::from(note.velocity) + ACCENT_BOOST);
            }
        }
    }
    track
}

pub fn volume(mut track: Track, delta: i32) -> Track {
    for note in &mut track.notes {
        note.velocity = clamp_velocity(i32::from(note.velocity) + delta);
    }
    track
}
