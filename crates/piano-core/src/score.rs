//! The compiled, playable form of a piece: time-ordered note and pedal events.

use crate::alert::SrcSpan;
use crate::pitch::Pitch;
use crate::time::{TICKS_PER_QUARTER, Tick, TimeSig};

/// Default tempo in quarter notes per minute.
pub const DEFAULT_TEMPO: u16 = 100;

/// Dynamic marks from `ppp` to `fff`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Dynamic {
    Ppp,
    Pp,
    P,
    Mp,
    #[default]
    Mf,
    F,
    Ff,
    Fff,
}

impl Dynamic {
    pub const ALL: [Dynamic; 8] = [
        Dynamic::Ppp,
        Dynamic::Pp,
        Dynamic::P,
        Dynamic::Mp,
        Dynamic::Mf,
        Dynamic::F,
        Dynamic::Ff,
        Dynamic::Fff,
    ];

    /// MIDI velocity for this mark.
    pub fn velocity(self) -> u8 {
        match self {
            Dynamic::Ppp => 16,
            Dynamic::Pp => 33,
            Dynamic::P => 49,
            Dynamic::Mp => 64,
            Dynamic::Mf => 80,
            Dynamic::F => 96,
            Dynamic::Ff => 112,
            Dynamic::Fff => 127,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Dynamic::Ppp => "ppp",
            Dynamic::Pp => "pp",
            Dynamic::P => "p",
            Dynamic::Mp => "mp",
            Dynamic::Mf => "mf",
            Dynamic::F => "f",
            Dynamic::Ff => "ff",
            Dynamic::Fff => "fff",
        }
    }

    pub fn parse(s: &str) -> Option<Dynamic> {
        Dynamic::ALL
            .into_iter()
            .find(|d| d.name().eq_ignore_ascii_case(s))
    }
}

impl std::fmt::Display for Dynamic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// The three piano pedals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PedalKind {
    /// Right pedal: sustains every note.
    Damper,
    /// Middle pedal: sustains only notes sounding when engaged.
    Sostenuto,
    /// Left pedal (una corda): softer sound.
    Soft,
}

impl PedalKind {
    pub const ALL: [PedalKind; 3] = [PedalKind::Damper, PedalKind::Sostenuto, PedalKind::Soft];

    /// Keyword used in the language and in Cmd.
    pub fn keyword(self) -> &'static str {
        match self {
            PedalKind::Damper => "ped",
            PedalKind::Sostenuto => "sos",
            PedalKind::Soft => "soft",
        }
    }

    pub fn index(self) -> usize {
        match self {
            PedalKind::Damper => 0,
            PedalKind::Sostenuto => 1,
            PedalKind::Soft => 2,
        }
    }

    pub fn parse(s: &str) -> Option<PedalKind> {
        PedalKind::ALL
            .into_iter()
            .find(|k| k.keyword().eq_ignore_ascii_case(s))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEvent {
    pub pitch: Pitch,
    pub start: Tick,
    /// Written length (ties joined).
    pub len: Tick,
    pub velocity: u8,
    /// How much earlier than the written end the key is let go (staccato).
    pub cut: Tick,
    /// How many milliseconds after its start the key goes down (arpeggio).
    pub roll_ms: u16,
    pub src: SrcSpan,
}

impl NoteEvent {
    /// Written end.
    pub fn end(&self) -> Tick {
        self.start + self.len
    }

    /// When the key is let go: the written end, earlier for a staccato.
    pub fn release(&self) -> Tick {
        self.start + self.len.saturating_sub(self.cut).max(1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PedalEvent {
    pub kind: PedalKind,
    pub on: bool,
    pub at: Tick,
    pub src: SrcSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TempoEvent {
    pub at: Tick,
    pub bpm: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSigEvent {
    pub at: Tick,
    pub sig: TimeSig,
}

/// A fully expanded piece, ready to play or export.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Score {
    pub notes: Vec<NoteEvent>,
    pub pedals: Vec<PedalEvent>,
    pub tempos: Vec<TempoEvent>,
    pub time_sigs: Vec<TimeSigEvent>,
    /// Fermatas: (from, to) stretches played at half speed.
    pub holds: Vec<(Tick, Tick)>,
    pub end: Tick,
}

/// How long a fermata holds, against the written time.
pub const HOLD: f64 = 2.0;

/// Wall-clock time of a score: the tempo in effect from each tick on, slowed under
/// a fermata.
#[derive(Debug, Clone, PartialEq)]
pub struct Clock {
    /// (from tick, quarter notes per minute), in order, starting at tick 0.
    pace: Vec<(Tick, f64)>,
}

impl Clock {
    pub fn pace(&self) -> &[(Tick, f64)] {
        &self.pace
    }

    /// Seconds from tick 0 to `tick`.
    pub fn seconds_at(&self, tick: Tick) -> f64 {
        let mut secs = 0.0;
        for (i, &(at, bpm)) in self.pace.iter().enumerate() {
            if at >= tick {
                break;
            }
            let end = self.pace.get(i + 1).map_or(tick, |n| n.0.min(tick));
            secs += ticks_to_secs(end - at, bpm);
        }
        secs
    }

    /// Inverse of [`Clock::seconds_at`].
    pub fn tick_at_seconds(&self, secs: f64) -> Tick {
        let mut elapsed = 0.0;
        for (i, &(at, bpm)) in self.pace.iter().enumerate() {
            match self.pace.get(i + 1) {
                Some(&(next, _)) if elapsed + ticks_to_secs(next - at, bpm) <= secs => {
                    elapsed += ticks_to_secs(next - at, bpm);
                }
                _ => return at + secs_to_ticks(secs - elapsed, bpm),
            }
        }
        0
    }
}

impl Score {
    /// The score's clock; build it once to time many events.
    pub fn clock(&self) -> Clock {
        let mut cuts: Vec<Tick> = std::iter::once(0)
            .chain(self.tempos.iter().map(|t| t.at))
            .chain(self.holds.iter().flat_map(|&(a, b)| [a, b]))
            .collect();
        cuts.sort_unstable();
        cuts.dedup();
        let mut pace: Vec<(Tick, f64)> = Vec::new();
        let mut tempos = self.tempos.iter().peekable();
        let mut bpm = DEFAULT_TEMPO;
        for t in cuts {
            while let Some(e) = tempos.next_if(|e| e.at <= t) {
                bpm = e.bpm;
            }
            let held = self.holds.iter().any(|&(a, b)| a <= t && t < b);
            let v = f64::from(bpm.max(1)) / if held { HOLD } else { 1.0 };
            if pace.last().is_none_or(|l| l.1 != v) {
                pace.push((t, v));
            }
        }
        Clock { pace }
    }

    /// Sorts events by time and recomputes `end`.
    pub fn finish(&mut self) {
        self.notes.sort_by_key(|n| (n.start, n.pitch));
        self.pedals.sort_by_key(|p| p.at);
        self.tempos.sort_by_key(|t| t.at);
        self.time_sigs.sort_by_key(|t| t.at);
        let notes_end = self.notes.iter().map(NoteEvent::end).max().unwrap_or(0);
        let pedals_end = self.pedals.iter().map(|p| p.at).max().unwrap_or(0);
        self.end = self.end.max(notes_end).max(pedals_end);
    }

    /// Tempo in effect at `tick`.
    pub fn tempo_at(&self, tick: Tick) -> u16 {
        self.tempos
            .iter()
            .take_while(|t| t.at <= tick)
            .last()
            .map_or(DEFAULT_TEMPO, |t| t.bpm)
    }

    /// Wall-clock seconds from tick 0 to `tick`, honoring tempo changes and fermatas.
    pub fn seconds_at(&self, tick: Tick) -> f64 {
        self.clock().seconds_at(tick)
    }

    /// Inverse of [`Score::seconds_at`].
    pub fn tick_at_seconds(&self, secs: f64) -> Tick {
        self.clock().tick_at_seconds(secs)
    }

    /// Total length in seconds.
    pub fn duration_secs(&self) -> f64 {
        self.seconds_at(self.end)
    }
}

fn ticks_to_secs(ticks: Tick, bpm: f64) -> f64 {
    ticks as f64 / TICKS_PER_QUARTER as f64 * 60.0 / bpm
}

fn secs_to_ticks(secs: f64, bpm: f64) -> Tick {
    (secs.max(0.0) * bpm / 60.0 * TICKS_PER_QUARTER as f64).round() as Tick
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_table() {
        let v: Vec<u8> = Dynamic::ALL.iter().map(|d| d.velocity()).collect();
        assert_eq!(v, [16, 33, 49, 64, 80, 96, 112, 127]);
        assert_eq!(Dynamic::parse("MF"), Some(Dynamic::Mf));
        assert_eq!(Dynamic::parse("x"), None);
    }

    #[test]
    fn timing_with_tempo_change() {
        let mut s = Score::default();
        s.tempos.push(TempoEvent { at: 0, bpm: 60 });
        s.tempos.push(TempoEvent {
            at: TICKS_PER_QUARTER * 2,
            bpm: 120,
        });
        // 2 quarters at 60 bpm = 2 s, then 2 quarters at 120 bpm = 1 s.
        let t = TICKS_PER_QUARTER * 4;
        assert!((s.seconds_at(t) - 3.0).abs() < 1e-9);
        assert_eq!(s.tick_at_seconds(3.0), t);
        assert_eq!(s.tick_at_seconds(1.0), TICKS_PER_QUARTER);
        assert_eq!(s.tempo_at(t), 120);
    }

    #[test]
    fn a_fermata_holds_twice_as_long() {
        let mut s = Score::default();
        s.tempos.push(TempoEvent { at: 0, bpm: 60 });
        s.holds.push((TICKS_PER_QUARTER, TICKS_PER_QUARTER * 2));
        // 1 s, then the held quarter 2 s, then 1 s.
        let t = TICKS_PER_QUARTER * 3;
        assert!((s.seconds_at(t) - 4.0).abs() < 1e-9);
        assert_eq!(s.tick_at_seconds(2.0), TICKS_PER_QUARTER * 3 / 2);
        assert_eq!(s.tick_at_seconds(4.0), t);
    }

    #[test]
    fn pedal_keywords() {
        assert_eq!(PedalKind::parse("sos"), Some(PedalKind::Sostenuto));
        assert_eq!(PedalKind::Soft.keyword(), "soft");
    }
}
