//! SoundFont piano with all three pedals.
//!
//! rustysynth handles the damper pedal (CC 64) itself; sostenuto and soft are
//! implemented here because the synthesizer does not support them.

use std::collections::HashSet;
use std::sync::Arc;

use piano_core::PedalKind;
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

use crate::AudioError;

const CHANNEL: i32 = 0;
const CC: i32 = 0xB0;
const CC_DAMPER: i32 = 0x40;
/// Velocity factor while the soft pedal is down.
const SOFT_FACTOR: f32 = 0.8;
/// rustysynth defaults to 0.5, which leaves a single note around -24 dBFS.
/// Louder output is kept within range by `soft_clip`.
const MASTER_VOLUME: f32 = 1.6;

pub struct PianoSynth {
    synth: Synthesizer,
    /// Keys currently down (note-on without note-off).
    down: HashSet<u8>,
    sostenuto: bool,
    /// Keys captured when the sostenuto pedal was pressed.
    captured: HashSet<u8>,
    /// Captured keys released while sostenuto is down; released with the pedal.
    deferred: HashSet<u8>,
    soft: bool,
}

impl PianoSynth {
    pub fn new(sound_font: &Arc<SoundFont>, sample_rate: u32) -> Result<PianoSynth, AudioError> {
        let settings = SynthesizerSettings::new(sample_rate as i32);
        let mut synth = Synthesizer::new(sound_font, &settings)
            .map_err(|e| AudioError::SoundFont(e.to_string()))?;
        synth.set_master_volume(MASTER_VOLUME);
        Ok(PianoSynth {
            synth,
            down: HashSet::new(),
            sostenuto: false,
            captured: HashSet::new(),
            deferred: HashSet::new(),
            soft: false,
        })
    }

    pub fn note_on(&mut self, key: u8, velocity: u8) {
        let mut v = f32::from(velocity);
        if self.soft {
            v *= SOFT_FACTOR;
        }
        self.deferred.remove(&key);
        self.down.insert(key);
        self.synth
            .note_on(CHANNEL, i32::from(key), (v.round() as i32).clamp(1, 127));
    }

    pub fn note_off(&mut self, key: u8) {
        self.down.remove(&key);
        if self.sostenuto && self.captured.contains(&key) {
            self.deferred.insert(key);
        } else {
            self.synth.note_off(CHANNEL, i32::from(key));
        }
    }

    pub fn pedal(&mut self, kind: PedalKind, on: bool) {
        match kind {
            PedalKind::Damper => {
                self.synth
                    .process_midi_message(CHANNEL, CC, CC_DAMPER, if on { 127 } else { 0 });
            }
            PedalKind::Sostenuto => {
                if on && !self.sostenuto {
                    self.captured = self.down.clone();
                } else if !on && self.sostenuto {
                    for key in std::mem::take(&mut self.deferred) {
                        if !self.down.contains(&key) {
                            self.synth.note_off(CHANNEL, i32::from(key));
                        }
                    }
                    self.captured.clear();
                }
                self.sostenuto = on;
            }
            PedalKind::Soft => self.soft = on,
        }
    }

    /// Releases every note and pedal.
    pub fn reset(&mut self) {
        self.synth.note_off_all(false);
        self.synth.process_midi_message(CHANNEL, CC, CC_DAMPER, 0);
        self.down.clear();
        self.captured.clear();
        self.deferred.clear();
        self.sostenuto = false;
        self.soft = false;
    }

    /// Hard stop: silences immediately.
    pub fn silence(&mut self) {
        self.reset();
        self.synth.note_off_all(true);
    }

    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.synth.render(left, right);
        for s in left.iter_mut().chain(right.iter_mut()) {
            *s = soft_clip(*s);
        }
    }
}

/// Linear below 0.6, then bends smoothly toward ±1 so loud chords never clip.
pub fn soft_clip(x: f32) -> f32 {
    const KNEE: f32 = 0.6;
    let a = x.abs();
    if a <= KNEE {
        x
    } else {
        let over = (a - KNEE) / (1.0 - KNEE);
        (KNEE + (1.0 - KNEE) * over.tanh()).copysign(x)
    }
}

#[cfg(test)]
mod tests {
    use super::soft_clip;

    #[test]
    fn soft_clip_is_linear_then_bounded() {
        assert_eq!(soft_clip(0.3), 0.3);
        assert_eq!(soft_clip(-0.6), -0.6);
        assert!(soft_clip(5.0) <= 1.0 && soft_clip(5.0) > 0.99);
        assert!(soft_clip(0.8) > 0.6 && soft_clip(0.8) < 0.8);
        assert_eq!(soft_clip(-5.0), -soft_clip(5.0));
    }
}
