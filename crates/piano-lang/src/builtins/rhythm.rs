//! `tuplet`.

use piano_core::Tick;

use crate::expand::Track;

/// Plays `n` notes in the time of the nearest lower power of two.
pub fn tuplet(mut track: Track, n: u32) -> Track {
    let m = 1u32 << (31 - (n - 1).leading_zeros());
    let scale = |t: Tick| t * Tick::from(m) / Tick::from(n);
    for note in &mut track.notes {
        note.start = scale(note.start);
        note.len = scale(note.len);
        note.cut = scale(note.cut);
    }
    for p in &mut track.pedals {
        p.at = scale(p.at);
    }
    for t in &mut track.tempos {
        t.at = scale(t.at);
    }
    for t in &mut track.time_sigs {
        t.at = scale(t.at);
    }
    track.len = scale(track.len);
    track
}

#[cfg(test)]
mod tests {
    #[test]
    fn lower_power_of_two() {
        let m = |n: u32| 1u32 << (31 - (n - 1).leading_zeros());
        assert_eq!(m(3), 2);
        assert_eq!(m(5), 4);
        assert_eq!(m(7), 4);
        assert_eq!(m(9), 8);
    }
}
