//! A tiny generated SoundFont (looped sine wave) for tests without real `.sf2` files.

use std::sync::Arc;

use rustysynth::SoundFont;

const PERIOD: usize = 100;
const CYCLES: usize = 20;

fn chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = id.to_vec();
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
    out
}

fn list(kind: &[u8; 4], chunks: &[Vec<u8>]) -> Vec<u8> {
    let mut data = kind.to_vec();
    for c in chunks {
        data.extend(c);
    }
    chunk(b"LIST", &data)
}

fn name20(name: &str) -> Vec<u8> {
    let mut b = name.as_bytes().to_vec();
    b.resize(20, 0);
    b
}

fn u16s(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// Bytes of a valid SF2 file with one preset (bank 0, patch 0).
pub fn tiny_soundfont_bytes() -> Vec<u8> {
    let len = PERIOD * CYCLES;
    let mut samples: Vec<i16> = (0..len)
        .map(|i| {
            let x = i as f64 / PERIOD as f64 * std::f64::consts::TAU;
            (x.sin() * 12_000.0) as i16
        })
        .collect();
    samples.extend(std::iter::repeat_n(0, 46));
    let smpl: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();

    let info = list(
        b"INFO",
        &[
            chunk(b"ifil", &u16s(&[2, 1])),
            chunk(b"isng", b"EMU8000\0"),
            chunk(b"INAM", b"Test\0\0"),
        ],
    );
    let sdta = list(b"sdta", &[chunk(b"smpl", &smpl)]);

    let mut phdr = Vec::new();
    for (name, bag) in [("Piano", 0u16), ("EOP", 1)] {
        phdr.extend(name20(name));
        phdr.extend(u16s(&[0, 0, bag]));
        phdr.extend([0u8; 12]);
    }
    let mut inst = Vec::new();
    for (name, bag) in [("Sine", 0u16), ("EOI", 1)] {
        inst.extend(name20(name));
        inst.extend(u16s(&[bag]));
    }
    let mut shdr = Vec::new();
    for (name, start, end) in [("sine", 0u32, len as u32), ("EOS", 0, 0)] {
        shdr.extend(name20(name));
        for v in [start, end, start, end, 44_100] {
            shdr.extend(v.to_le_bytes());
        }
        let (pitch, kind) = if name == "EOS" { (0, 0) } else { (69u8, 1u16) };
        shdr.push(pitch); // original pitch (A4 = 441 Hz here)
        shdr.push(0); // pitch correction
        shdr.extend(u16s(&[0, kind]));
    }
    let pdta = list(
        b"pdta",
        &[
            chunk(b"phdr", &phdr),
            chunk(b"pbag", &u16s(&[0, 0, 1, 0])),
            chunk(b"pmod", &[0u8; 10]),
            // instrument (41) = 0, then terminal
            chunk(b"pgen", &u16s(&[41, 0, 0, 0])),
            chunk(b"inst", &inst),
            chunk(b"ibag", &u16s(&[0, 0, 2, 0])),
            chunk(b"imod", &[0u8; 10]),
            // sampleModes (54) = loop, sampleID (53) = 0, then terminal
            chunk(b"igen", &u16s(&[54, 1, 53, 0, 0, 0])),
            chunk(b"shdr", &shdr),
        ],
    );

    let mut body = b"sfbk".to_vec();
    body.extend(info);
    body.extend(sdta);
    body.extend(pdta);
    chunk(b"RIFF", &body)
}

pub fn tiny_soundfont() -> Arc<SoundFont> {
    let bytes = tiny_soundfont_bytes();
    Arc::new(SoundFont::new(&mut bytes.as_slice()).expect("valid generated soundfont"))
}

#[cfg(test)]
mod tests {
    use piano_core::PedalKind;

    use super::*;
    use crate::synth::PianoSynth;

    const SR: u32 = 22_050;

    fn rms(synth: &mut PianoSynth, secs: f32) -> f32 {
        let n = (SR as f32 * secs) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        synth.render(&mut l, &mut r);
        (l.iter().map(|s| s * s).sum::<f32>() / n as f32).sqrt()
    }

    #[test]
    fn loads_and_sounds() {
        let mut s = PianoSynth::new(&tiny_soundfont(), SR).unwrap();
        assert_eq!(rms(&mut s, 0.05), 0.0);
        s.note_on(60, 100);
        let on = rms(&mut s, 0.1);
        assert!(on > 0.01);
        s.note_off(60);
        rms(&mut s, 0.5);
        // Only the reverb tail is left.
        assert!(rms(&mut s, 0.1) < on * 0.05);
    }

    #[test]
    fn sostenuto_holds_only_captured_notes() {
        let mut s = PianoSynth::new(&tiny_soundfont(), SR).unwrap();
        s.note_on(48, 100);
        s.pedal(PedalKind::Sostenuto, true);
        s.note_off(48);
        s.note_on(72, 100);
        s.note_off(72);
        rms(&mut s, 0.5);
        // 48 still rings (held by sostenuto); 72 was released normally.
        let held = rms(&mut s, 0.1);
        assert!(held > 0.01);
        s.pedal(PedalKind::Sostenuto, false);
        rms(&mut s, 0.5);
        assert!(rms(&mut s, 0.1) < held * 0.05);
    }

    #[test]
    fn damper_holds_everything() {
        let mut s = PianoSynth::new(&tiny_soundfont(), SR).unwrap();
        s.pedal(PedalKind::Damper, true);
        s.note_on(60, 100);
        s.note_off(60);
        rms(&mut s, 0.5);
        let held = rms(&mut s, 0.1);
        assert!(held > 0.01);
        s.pedal(PedalKind::Damper, false);
        rms(&mut s, 0.5);
        assert!(rms(&mut s, 0.1) < held * 0.05);
    }

    #[test]
    fn soft_pedal_is_quieter() {
        let loud = {
            let mut s = PianoSynth::new(&tiny_soundfont(), SR).unwrap();
            s.note_on(60, 100);
            rms(&mut s, 0.1)
        };
        let soft = {
            let mut s = PianoSynth::new(&tiny_soundfont(), SR).unwrap();
            s.pedal(PedalKind::Soft, true);
            s.note_on(60, 100);
            rms(&mut s, 0.1)
        };
        assert!(soft < loud, "{soft} < {loud}");
    }

    #[test]
    fn offline_render() {
        let score = piano_core::Score {
            notes: vec![piano_core::NoteEvent {
                pitch: piano_core::Pitch::MIDDLE_C,
                start: 0,
                len: piano_core::TICKS_PER_QUARTER,
                velocity: 90,
                cut: 0,
                src: Default::default(),
            }],
            end: piano_core::TICKS_PER_QUARTER,
            ..Default::default()
        };
        let (l, _) = crate::render(&score, &tiny_soundfont(), SR).unwrap();
        // 1 quarter at 100 bpm = 0.6 s, plus a 2 s tail.
        assert_eq!(l.len(), (SR as f64 * 2.6).round() as usize);
        assert!(l.iter().any(|s| s.abs() > 0.01));
    }

    #[test]
    fn soft_pedal_softens_the_note_struck_with_it() {
        // `C4` vs `soft on C4`: the pedal and the note start at the same instant.
        let first_note = |soft: bool| {
            let score = piano_core::Score {
                notes: vec![piano_core::NoteEvent {
                    pitch: piano_core::Pitch::MIDDLE_C,
                    start: 0,
                    len: piano_core::TICKS_PER_QUARTER,
                    velocity: 100,
                    cut: 0,
                    src: Default::default(),
                }],
                pedals: if soft {
                    vec![piano_core::PedalEvent {
                        kind: PedalKind::Soft,
                        on: true,
                        at: 0,
                        src: Default::default(),
                    }]
                } else {
                    Vec::new()
                },
                end: piano_core::TICKS_PER_QUARTER,
                ..Default::default()
            };
            let (l, _) = crate::render(&score, &tiny_soundfont(), SR).unwrap();
            let w = &l[..(SR / 10) as usize];
            (w.iter().map(|s| s * s).sum::<f32>() / w.len() as f32).sqrt()
        };
        let (loud, soft) = (first_note(false), first_note(true));
        assert!(soft < loud * 0.95, "{soft} < {loud}");
    }
}
