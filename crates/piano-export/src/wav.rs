//! Score → stereo PCM rendered with the SoundFont, written as WAV or FLAC.
//!
//! By default the sample rate is the SoundFont's own (no resampling of its
//! samples) and samples are 24-bit, so quiet tails keep their detail.

use std::io::Cursor;
use std::sync::Arc;

use piano_audio::SoundFont;
use piano_core::Score;

use crate::ExportError;

/// Sample rate and bit depth of an audio file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub rate: u32,
    /// 16 or 24.
    pub bits: u16,
}

impl AudioFormat {
    /// CD format, 44.1 kHz / 16-bit.
    pub const CD: AudioFormat = AudioFormat {
        rate: 44_100,
        bits: 16,
    };

    /// 24-bit at the SoundFont's sample rate.
    pub fn native(sound_font: &SoundFont) -> AudioFormat {
        AudioFormat {
            rate: native_rate(sound_font),
            bits: 24,
        }
    }
}

/// The sample rate most of the SoundFont's samples use (48 kHz if it has none).
pub fn native_rate(sound_font: &SoundFont) -> u32 {
    let mut counts: Vec<(u32, usize)> = Vec::new();
    for h in sound_font.get_sample_headers() {
        let r = u32::try_from(h.get_sample_rate()).unwrap_or(0);
        if !(8_000..=192_000).contains(&r) {
            continue;
        }
        match counts.iter_mut().find(|c| c.0 == r) {
            Some(c) => c.1 += 1,
            None => counts.push((r, 1)),
        }
    }
    counts
        .into_iter()
        .max_by_key(|c| (c.1, c.0))
        .map_or(48_000, |c| c.0)
}

/// Interleaved stereo samples at `format`, normalized only if the mix clips.
fn pcm(
    score: &Score,
    sound_font: &Arc<SoundFont>,
    format: AudioFormat,
) -> Result<Vec<i32>, ExportError> {
    if !matches!(format.bits, 16 | 24) {
        return Err(ExportError::Format(format!(
            "{} bits: use 16 or 24",
            format.bits
        )));
    }
    if !(8_000..=192_000).contains(&format.rate) {
        return Err(ExportError::Format(format!(
            "{} Hz: use 8000-192000",
            format.rate
        )));
    }
    let (left, right) = piano_audio::render(score, sound_font, format.rate)?;
    let peak = left
        .iter()
        .chain(&right)
        .fold(0.0f32, |m, s| m.max(s.abs()));
    let gain = if peak > 1.0 { 1.0 / peak } else { 1.0 };
    let full = ((1i64 << (format.bits - 1)) - 1) as f32;
    let to_int = |s: f32| (s * gain * full).round().clamp(-full - 1.0, full) as i32;
    Ok(left
        .iter()
        .zip(&right)
        .flat_map(|(l, r)| [to_int(*l), to_int(*r)])
        .collect())
}

pub fn wav_bytes(
    score: &Score,
    sound_font: &Arc<SoundFont>,
    format: AudioFormat,
) -> Result<Vec<u8>, ExportError> {
    let samples = pcm(score, sound_font, format)?;
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: format.rate,
        bits_per_sample: format.bits,
        sample_format: hound::SampleFormat::Int,
    };
    let mut out = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut out, spec)?;
    for s in samples {
        if format.bits == 16 {
            writer.write_sample(s as i16)?;
        } else {
            writer.write_sample(s)?;
        }
    }
    writer.finalize()?;
    Ok(out.into_inner())
}

/// Lossless FLAC: the same samples as [`wav_bytes`], about half the size.
pub fn flac_bytes(
    score: &Score,
    sound_font: &Arc<SoundFont>,
    format: AudioFormat,
) -> Result<Vec<u8>, ExportError> {
    use flacenc::component::BitRepr;
    use flacenc::error::Verify;

    let samples = pcm(score, sound_font, format)?;
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|(_, e)| ExportError::Format(format!("flac config: {e:?}")))?;
    let source = flacenc::source::MemSource::from_samples(
        &samples,
        2,
        usize::from(format.bits),
        format.rate as usize,
    );
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|e| ExportError::Format(format!("flac: {e:?}")))?;
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|e| ExportError::Format(format!("flac: {e:?}")))?;
    Ok(sink.as_slice().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score() -> Score {
        piano_lang::compile_piece("C4 E G").score
    }

    #[test]
    fn writes_readable_wav() {
        let sf = piano_audio::testing::tiny_soundfont();
        for format in [AudioFormat::CD, AudioFormat::native(&sf)] {
            let bytes = wav_bytes(&score(), &sf, format).unwrap();
            let reader = hound::WavReader::new(Cursor::new(bytes)).unwrap();
            assert_eq!(reader.spec().channels, 2);
            assert_eq!(reader.spec().bits_per_sample, format.bits);
            assert_eq!(reader.spec().sample_rate, format.rate);
            // 3 quarters at 100 bpm = 1.8 s plus a 2 s tail.
            let secs = f64::from(reader.duration()) / f64::from(format.rate);
            assert!((secs - 3.8).abs() < 0.01, "{secs}");
        }
    }

    #[test]
    fn flac_holds_the_same_samples_as_wav() {
        let sf = piano_audio::testing::tiny_soundfont();
        let format = AudioFormat::native(&sf);
        let flac = flac_bytes(&score(), &sf, format).unwrap();
        let mut reader = claxon::FlacReader::new(Cursor::new(&flac)).unwrap();
        let info = reader.streaminfo();
        assert_eq!(
            (info.channels, info.bits_per_sample, info.sample_rate),
            (2, 24, format.rate)
        );
        let decoded: Vec<i32> = reader.samples().map(Result::unwrap).collect();
        assert_eq!(decoded, pcm(&score(), &sf, format).unwrap());
        let wav = wav_bytes(&score(), &sf, format).unwrap();
        assert!(flac.len() < wav.len(), "{} vs {}", flac.len(), wav.len());
    }

    #[test]
    fn bad_formats_are_refused() {
        let sf = piano_audio::testing::tiny_soundfont();
        let f = AudioFormat {
            rate: 48_000,
            bits: 20,
        };
        assert!(wav_bytes(&score(), &sf, f).is_err());
    }
}
