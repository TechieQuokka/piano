//! Exports a `Score` to MIDI and WAV. Functions return bytes; the caller writes
//! files (so the project sandbox decides where writing is allowed).

pub mod midi;
pub mod wav;

pub use midi::midi_bytes;
pub use wav::wav_bytes;

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error(transparent)]
    Audio(#[from] piano_audio::AudioError),
    #[error("wav: {0}")]
    Wav(#[from] hound::Error),
}
