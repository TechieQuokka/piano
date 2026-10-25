//! Score → 16-bit stereo WAV, rendered with the SoundFont.

use std::io::Cursor;
use std::sync::Arc;

use piano_audio::SoundFont;
use piano_core::Score;

use crate::ExportError;

pub const SAMPLE_RATE: u32 = 44_100;

pub fn wav_bytes(score: &Score, sound_font: &Arc<SoundFont>) -> Result<Vec<u8>, ExportError> {
    let (left, right) = piano_audio::render(score, sound_font, SAMPLE_RATE)?;
    // Normalize only if the mix clips.
    let peak = left
        .iter()
        .chain(&right)
        .fold(0.0f32, |m, s| m.max(s.abs()));
    let gain = if peak > 1.0 { 1.0 / peak } else { 1.0 };
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut out = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut out, spec)?;
    for (l, r) in left.iter().zip(&right) {
        for s in [l, r] {
            writer.write_sample((s * gain * f32::from(i16::MAX)) as i16)?;
        }
    }
    writer.finalize()?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_readable_wav() {
        let score = piano_lang::compile_piece("C4 E G").score;
        let bytes = wav_bytes(&score, &piano_audio::testing::tiny_soundfont()).unwrap();
        let reader = hound::WavReader::new(Cursor::new(bytes)).unwrap();
        assert_eq!(reader.spec().channels, 2);
        // 3 quarters at 100 bpm = 1.8 s plus a 2 s tail.
        let frames = reader.duration() as f64 / f64::from(SAMPLE_RATE);
        assert!((frames - 3.8).abs() < 0.01, "{frames}");
    }
}
