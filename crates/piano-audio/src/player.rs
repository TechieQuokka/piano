//! Real-time playback on the audio thread. The UI only sends commands.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

#[cfg(feature = "device")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(feature = "device")]
use cpal::{FromSample, SampleFormat, SizedSample};
use piano_core::{Pitch, Score, Tick};
use rustysynth::SoundFont;

use crate::AudioError;
use crate::render::apply;
use crate::scheduler::{Msg, MsgKind, schedule};
use crate::synth::PianoSynth;

/// How long a previewed note sounds.
const PREVIEW_SECS: f64 = 0.9;
/// Render in slices of at most this many frames so message timing stays tight.
#[cfg_attr(not(feature = "device"), allow(dead_code))]
const MAX_SLICE: usize = 256;

#[cfg_attr(not(feature = "device"), allow(dead_code))]
struct Playback {
    score: Arc<Score>,
    from: Tick,
    to: Option<Tick>,
    msgs: Vec<Msg>,
    next: usize,
    /// Samples since the start of this pass.
    pos: u64,
    end: u64,
    looping: bool,
}

#[cfg_attr(not(feature = "device"), allow(dead_code))]
struct Engine {
    synth: Option<PianoSynth>,
    sample_rate: u32,
    playback: Option<Playback>,
    /// Absolute-time preview messages (sample clock = `clock`).
    preview: Vec<Msg>,
    clock: u64,
    left: Vec<f32>,
    right: Vec<f32>,
    /// Peak level sent to the device since last read (diagnostics).
    peak: f32,
}

#[cfg_attr(not(feature = "device"), allow(dead_code))]
impl Engine {
    fn fill(&mut self, out: &mut [f32], channels: usize) {
        let frames = out.len() / channels.max(1);
        if self.left.len() < frames {
            self.left.resize(frames, 0.0);
            self.right.resize(frames, 0.0);
        }
        let mut done = 0;
        while done < frames {
            let slice = (frames - done).min(MAX_SLICE);
            let slice = self.dispatch(slice);
            if let Some(synth) = &mut self.synth {
                synth.render(
                    &mut self.left[done..done + slice],
                    &mut self.right[done..done + slice],
                );
            } else {
                self.left[done..done + slice].fill(0.0);
                self.right[done..done + slice].fill(0.0);
            }
            self.clock += slice as u64;
            if let Some(p) = &mut self.playback {
                p.pos += slice as u64;
            }
            done += slice;
        }
        for (i, frame) in out.chunks_mut(channels).enumerate() {
            let (l, r) = (self.left[i], self.right[i]);
            self.peak = self.peak.max(l.abs()).max(r.abs());
            for (c, s) in frame.iter_mut().enumerate() {
                *s = if c % 2 == 0 { l } else { r };
            }
        }
    }

    /// Applies due messages; returns how many frames can be rendered before the next one.
    fn dispatch(&mut self, max: usize) -> usize {
        let mut limit = max as u64;
        if let Some(synth) = &mut self.synth {
            self.preview.retain(|m| {
                if m.at <= self.clock {
                    apply(synth, m);
                    false
                } else {
                    true
                }
            });
            if let Some(next) = self.preview.iter().map(|m| m.at - self.clock).min() {
                limit = limit.min(next);
            }
        }
        if let Some(p) = &mut self.playback {
            if let Some(synth) = &mut self.synth {
                while let Some(m) = p.msgs.get(p.next).filter(|m| m.at <= p.pos) {
                    apply(synth, m);
                    p.next += 1;
                }
            } else {
                p.next = p.msgs.partition_point(|m| m.at <= p.pos);
            }
            if p.pos >= p.end && p.next >= p.msgs.len() {
                if p.looping {
                    p.pos = 0;
                    p.next = 0;
                    if let Some(synth) = &mut self.synth {
                        synth.reset();
                    }
                    return self.dispatch(max);
                }
                self.playback = None;
            } else {
                let next = p.msgs.get(p.next).map_or(p.end, |m| m.at);
                limit = limit.min(next.saturating_sub(p.pos).max(1));
            }
        }
        (limit.max(1) as usize).min(max)
    }
}

/// Plays scores and previews notes. Works without a SoundFont or audio device
/// (silently), so editing never depends on audio.
#[cfg(feature = "device")]
type Stream = cpal::Stream;
#[cfg(not(feature = "device"))]
type Stream = ();

pub struct Player {
    engine: Arc<Mutex<Engine>>,
    _stream: Option<Stream>,
    sound_font: Option<Arc<SoundFont>>,
    device_error: Option<String>,
    /// e.g. "Default ALSA Output · 48000 Hz · 2 ch · f32"
    device_info: Option<String>,
    /// Errors reported by the audio stream since the last `take_errors`.
    errors: Arc<Mutex<Vec<String>>>,
}

/// Errors kept at most between two `take_errors` calls.
#[cfg_attr(not(feature = "device"), allow(dead_code))]
const MAX_ERRORS: usize = 20;

impl Player {
    /// Opens the default output device. Failures are remembered, not returned.
    pub fn new() -> Player {
        Player::create(true)
    }

    /// A player that never opens an audio device (tests, headless tools).
    pub fn silent() -> Player {
        Player::create(false)
    }

    fn create(open_device: bool) -> Player {
        let engine = Arc::new(Mutex::new(Engine {
            synth: None,
            sample_rate: 44_100,
            playback: None,
            preview: Vec::new(),
            clock: 0,
            left: Vec::new(),
            right: Vec::new(),
            peak: 0.0,
        }));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let (stream, device_info, device_error) = if open_device {
            match open_stream(&engine, &errors) {
                Ok((s, info)) => (Some(s), Some(info), None),
                Err(e) => (None, None, Some(e.to_string())),
            }
        } else {
            (None, None, Some("silent player".to_string()))
        };
        Player {
            engine,
            _stream: stream,
            sound_font: None,
            device_error,
            device_info,
            errors,
        }
    }

    fn lock(&self) -> MutexGuard<'_, Engine> {
        self.engine.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn load_soundfont(&mut self, path: &Path) -> Result<(), AudioError> {
        let sf = crate::load_soundfont(path)?;
        self.set_soundfont(sf)
    }

    pub fn set_soundfont(&mut self, sf: Arc<SoundFont>) -> Result<(), AudioError> {
        // Built outside the lock: the audio thread waits for it.
        let synth = PianoSynth::new(&sf, self.lock().sample_rate)?;
        let old = self.lock().synth.replace(synth);
        drop(old);
        self.sound_font = Some(sf);
        Ok(())
    }

    pub fn sound_font(&self) -> Option<&Arc<SoundFont>> {
        self.sound_font.as_ref()
    }

    pub fn has_sound(&self) -> bool {
        self.sound_font.is_some() && self.device_error.is_none()
    }

    /// Why there is no audio device, if so.
    pub fn device_error(&self) -> Option<&str> {
        self.device_error.as_deref()
    }

    /// The opened device and its format.
    pub fn device_info(&self) -> Option<&str> {
        self.device_info.as_deref()
    }

    /// Stream errors since the last call (underruns, device changes, ...).
    pub fn take_errors(&self) -> Vec<String> {
        std::mem::take(&mut *self.errors.lock().unwrap_or_else(|e| e.into_inner()))
    }

    pub fn play(&self, score: Arc<Score>, from: Tick, to: Option<Tick>) {
        self.start(score, from, to, false);
    }

    pub fn play_loop(&self, score: Arc<Score>, from: Tick, to: Tick) {
        self.start(score, from, Some(to), true);
    }

    fn start(&self, score: Arc<Score>, from: Tick, to: Option<Tick>, looping: bool) {
        // Scheduled outside the lock: the audio thread waits for it.
        let (msgs, end) = schedule(&score, from, to, self.lock().sample_rate);
        let playback = Playback {
            score,
            from,
            to,
            msgs,
            next: 0,
            pos: 0,
            end,
            looping,
        };
        let mut engine = self.lock();
        if let Some(synth) = &mut engine.synth {
            synth.reset();
        }
        let old = engine.playback.replace(playback);
        drop(engine);
        drop(old);
    }

    pub fn stop(&self) {
        let mut engine = self.lock();
        let old = engine.playback.take();
        engine.preview.clear();
        if let Some(synth) = &mut engine.synth {
            synth.silence();
        }
        drop(engine);
        drop(old);
    }

    /// Strikes `pitches` now for a short time.
    pub fn preview(&self, pitches: &[Pitch], velocity: u8) {
        for p in pitches {
            self.strike(*p, velocity);
            self.release(*p, PREVIEW_SECS);
        }
    }

    /// Presses a key now; it sounds until [`Player::release`].
    /// A pending release of the same key is cancelled.
    pub fn strike(&self, pitch: Pitch, velocity: u8) {
        let mut engine = self.lock();
        let (now, key) = (engine.clock, pitch.midi());
        engine.preview.retain(|m| m.kind != MsgKind::Off { key });
        engine.preview.push(Msg {
            at: now,
            kind: MsgKind::On { key, velocity },
        });
    }

    /// Lets a key go `after_secs` from now; the SoundFont's release then fades it.
    pub fn release(&self, pitch: Pitch, after_secs: f64) {
        let mut engine = self.lock();
        let (now, sr, key) = (engine.clock, engine.sample_rate, pitch.midi());
        engine.preview.retain(|m| m.kind != MsgKind::Off { key });
        engine.preview.push(Msg {
            at: now + (after_secs.max(0.0) * f64::from(sr)) as u64,
            kind: MsgKind::Off { key },
        });
    }

    /// Pulls `secs` of stereo audio through the engine as the device callback
    /// would, and returns the peak level. For tests and diagnostics only.
    #[doc(hidden)]
    pub fn pull_peak(&self, secs: f64) -> f32 {
        let frames = (secs * f64::from(self.lock().sample_rate)) as usize;
        let mut out = vec![0.0; 512 * 2];
        let mut peak = 0.0f32;
        for _ in 0..frames.div_ceil(512) {
            self.lock().fill(&mut out, 2);
            peak = out.iter().fold(peak, |m, s| m.max(s.abs()));
        }
        peak
    }

    /// Peak level sent to the device since the last call, and the sample clock.
    /// Lets diagnostics tell "no signal" apart from "signal but not heard".
    pub fn take_peak(&self) -> (f32, u64) {
        let mut e = self.lock();
        (std::mem::take(&mut e.peak), e.clock)
    }

    pub fn is_playing(&self) -> bool {
        self.lock().playback.is_some()
    }

    /// Current score position while playing.
    pub fn position(&self) -> Option<Tick> {
        let engine = self.lock();
        let p = engine.playback.as_ref()?;
        let secs = p.score.seconds_at(p.from) + p.pos as f64 / f64::from(engine.sample_rate);
        let tick = p.score.tick_at_seconds(secs);
        Some(p.to.map_or(tick, |to| tick.min(to)))
    }
}

impl Default for Player {
    fn default() -> Self {
        Player::new()
    }
}

type Errors = Arc<Mutex<Vec<String>>>;

#[cfg(not(feature = "device"))]
fn open_stream(
    _engine: &Arc<Mutex<Engine>>,
    _errors: &Errors,
) -> Result<(Stream, String), AudioError> {
    Err(AudioError::Device(
        "built without audio output (enable the `device` feature)".into(),
    ))
}

#[cfg(feature = "device")]
fn open_stream(
    engine: &Arc<Mutex<Engine>>,
    errors: &Errors,
) -> Result<(Stream, String), AudioError> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| AudioError::Device("no audio output device".into()))?;
    let supported = device
        .default_output_config()
        .map_err(|e| AudioError::Device(e.to_string()))?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let name = device
        .description()
        .map(|d| d.to_string())
        .or_else(|_| device.id().map(|i| i.to_string()))
        .unwrap_or_else(|_| "unknown device".into());
    let info = format!(
        "{name} · {} Hz · {} ch · {format}",
        config.sample_rate, config.channels
    );
    engine.lock().unwrap_or_else(|e| e.into_inner()).sample_rate = config.sample_rate;
    let stream = match format {
        SampleFormat::F32 => build::<f32>(&device, config, engine, errors),
        SampleFormat::I16 => build::<i16>(&device, config, engine, errors),
        SampleFormat::I32 => build::<i32>(&device, config, engine, errors),
        SampleFormat::U16 => build::<u16>(&device, config, engine, errors),
        other => {
            return Err(AudioError::Device(format!(
                "unsupported sample format {other}"
            )));
        }
    }?;
    stream
        .play()
        .map_err(|e| AudioError::Device(e.to_string()))?;
    Ok((stream, info))
}

#[cfg(feature = "device")]
fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    engine: &Arc<Mutex<Engine>>,
    errors: &Errors,
) -> Result<cpal::Stream, AudioError>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels);
    let engine = Arc::clone(engine);
    let mut buf: Vec<f32> = Vec::new();
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                buf.resize(data.len(), 0.0);
                // A panic elsewhere must not silence playback for good.
                engine
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .fill(&mut buf, channels);
                for (d, s) in data.iter_mut().zip(&buf) {
                    *d = T::from_sample(*s);
                }
            },
            {
                let errors = Arc::clone(errors);
                move |err| {
                    let mut e = errors.lock().unwrap_or_else(|e| e.into_inner());
                    if e.len() < MAX_ERRORS {
                        e.push(err.to_string());
                    }
                }
            },
            None,
        )
        .map_err(|e| AudioError::Device(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pulls `secs` of audio through the engine as the device callback would.
    fn pull(p: &Player, secs: f64) -> f32 {
        let frames = (secs * 44_100.0) as usize;
        let mut out = vec![0.0; 512 * 2];
        let mut peak = 0.0f32;
        for _ in 0..frames / 512 {
            p.lock().fill(&mut out, 2);
            peak = out.iter().fold(peak, |m, s| m.max(s.abs()));
        }
        peak
    }

    fn player() -> Player {
        let mut p = Player::silent();
        p.set_soundfont(crate::testing::tiny_soundfont()).unwrap();
        p
    }

    #[test]
    fn preview_sounds_without_playback() {
        let p = player();
        assert_eq!(pull(&p, 0.1), 0.0);
        p.preview(&[Pitch::MIDDLE_C], 80);
        assert!(pull(&p, 0.2) > 0.01, "preview should be audible");
    }

    /// Real device + installed SoundFont: `cargo test -p piano-audio --features device -- --ignored`
    #[cfg(feature = "device")]
    #[test]
    #[ignore]
    fn real_device_receives_preview() {
        let Some(sf) = crate::find_soundfont() else {
            eprintln!("no soundfont installed");
            return;
        };
        let mut p = Player::new();
        assert_eq!(p.device_error(), None);
        eprintln!("device: {:?}", p.device_info());
        p.load_soundfont(&sf).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let (quiet, clock0) = p.take_peak();
        p.preview(&[Pitch::MIDDLE_C], 80);
        std::thread::sleep(std::time::Duration::from_millis(600));
        let (peak, clock1) = p.take_peak();
        eprintln!(
            "before {quiet} after {peak}, clock {clock0} -> {clock1}, errors {:?}",
            p.take_errors()
        );
        assert!(clock1 > clock0, "audio callback is not running");
        assert!(peak > 0.01, "preview produced no signal");
    }

    #[test]
    fn strike_sounds_until_release() {
        let p = player();
        p.strike(Pitch::MIDDLE_C, 90);
        let on = pull(&p, 0.2);
        assert!(on > 0.01);
        // Still sounding long after the preview time would have ended.
        assert!(pull(&p, 1.5) > on * 0.5);
        p.release(Pitch::MIDDLE_C, 0.0);
        pull(&p, 1.0);
        assert!(pull(&p, 0.2) < on * 0.1);
    }

    #[test]
    fn restrike_cancels_pending_release() {
        let p = player();
        p.strike(Pitch::MIDDLE_C, 90);
        p.release(Pitch::MIDDLE_C, 0.1);
        p.strike(Pitch::MIDDLE_C, 90);
        pull(&p, 0.5);
        assert!(pull(&p, 0.2) > 0.01);
    }

    #[test]
    fn preview_after_playback() {
        let p = player();
        let score = Arc::new(piano_core::Score {
            notes: vec![piano_core::NoteEvent {
                pitch: Pitch::MIDDLE_C,
                start: 0,
                len: piano_core::TICKS_PER_QUARTER,
                velocity: 80,
                src: Default::default(),
            }],
            end: piano_core::TICKS_PER_QUARTER,
            ..Default::default()
        });
        p.play(score, 0, None);
        assert!(pull(&p, 0.3) > 0.01);
        pull(&p, 2.0);
        assert!(!p.is_playing());
        p.preview(&[Pitch::MIDDLE_C], 80);
        assert!(pull(&p, 0.2) > 0.01);
    }
}
