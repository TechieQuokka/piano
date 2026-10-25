//! Offline rendering (for WAV export).

use std::sync::Arc;

use piano_core::Score;
use rustysynth::SoundFont;

use crate::AudioError;
use crate::scheduler::{Msg, MsgKind, schedule};
use crate::synth::PianoSynth;

/// Seconds rendered after the last event so notes can ring out.
const TAIL_SECS: u32 = 2;

/// Renders the whole score to stereo samples.
pub fn render(
    score: &Score,
    sound_font: &Arc<SoundFont>,
    sample_rate: u32,
) -> Result<(Vec<f32>, Vec<f32>), AudioError> {
    let mut synth = PianoSynth::new(sound_font, sample_rate)?;
    let (msgs, end) = schedule(score, 0, None, sample_rate);
    let total = (end + u64::from(sample_rate * TAIL_SECS)) as usize;
    let mut left = vec![0.0; total];
    let mut right = vec![0.0; total];
    let mut pos = 0usize;
    let mut i = 0;
    while pos < total {
        while let Some(m) = msgs.get(i).filter(|m| m.at as usize <= pos) {
            apply(&mut synth, m);
            i += 1;
        }
        let next = msgs.get(i).map_or(total, |m| (m.at as usize).min(total));
        synth.render(&mut left[pos..next], &mut right[pos..next]);
        pos = next;
    }
    Ok((left, right))
}

pub(crate) fn apply(synth: &mut PianoSynth, m: &Msg) {
    match m.kind {
        MsgKind::On { key, velocity } => synth.note_on(key, velocity),
        MsgKind::Off { key } => synth.note_off(key),
        MsgKind::Pedal { kind, on } => synth.pedal(kind, on),
    }
}
