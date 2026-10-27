//! Inheritance: turns written notes into absolute pitches, lengths, and velocities.
//!
//! The walk follows the written order, so octaves inside functions are resolved on
//! the text before any transform, and the flow around a function is not disturbed.

use piano_core::{
    Alert, Duration, Dynamic, PedalKind, Pitch, SrcSpan, Tick, TimeSig, score::DEFAULT_TEMPO,
};

use crate::ast::{Marks, Node, Sudden};
use crate::builtins::Builtin;
use crate::token::NoteName;

/// Defaults a piece starts with (from `piano.toml` or builtin).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompileOptions {
    pub tempo: u16,
    pub time: TimeSig,
    pub len: Duration,
    pub dynamic: Dynamic,
    /// Octave of the starting reference note C.
    pub octave: i32,
}

impl Default for CompileOptions {
    fn default() -> Self {
        CompileOptions {
            tempo: DEFAULT_TEMPO,
            time: TimeSig::COMMON,
            len: Duration::QUARTER,
            dynamic: Dynamic::Mf,
            octave: 4,
        }
    }
}

/// Inherited state at a point in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    /// MIDI number of the octave reference (the previous note).
    pub reference: i32,
    pub len: Duration,
    pub dynamic: Dynamic,
    pub pedals: [bool; 3],
    pub tempo: u16,
    pub time: TimeSig,
    /// `sf`, `fp`, ... waiting for its note.
    pub sudden: Option<(Sudden, SrcSpan)>,
}

impl State {
    pub fn new(opts: &CompileOptions) -> State {
        State {
            reference: (opts.octave + 1) * 12,
            len: opts.len,
            dynamic: opts.dynamic,
            pedals: [false; 3],
            tempo: opts.tempo,
            time: opts.time,
            sudden: None,
        }
    }

    /// The reference as a piano key (folded into range for display).
    pub fn reference_pitch(&self) -> Pitch {
        Pitch::fold(self.reference).0
    }

    pub fn pedal(&self, kind: PedalKind) -> bool {
        self.pedals[kind.index()]
    }

    /// Resolves a written note against this state's reference.
    /// Returns the MIDI number and whether the octave choice was a tie.
    pub fn resolve_note(&self, name: &NoteName) -> (i32, bool) {
        if let Some(octave) = name.octave {
            return (Pitch::spell(name.letter, name.accidental, octave), false);
        }
        nearest(self.reference, name)
    }
}

/// Picks the octave closest to `reference`; a 6-semitone tie goes up.
fn nearest(reference: i32, name: &NoteName) -> (i32, bool) {
    let base_octave = reference.div_euclid(12) - 1;
    let mut best: Option<(i32, i32)> = None;
    let mut tie = false;
    for octave in base_octave - 1..=base_octave + 1 {
        let n = Pitch::spell(name.letter, name.accidental, octave);
        let d = (n - reference).abs();
        match best {
            None => best = Some((n, d)),
            Some((_, bd)) if d < bd => {
                best = Some((n, d));
                tie = false;
            }
            Some((bn, bd)) if d == bd => {
                tie = true;
                best = Some((n.max(bn), d));
            }
            _ => {}
        }
    }
    (best.map_or(reference, |(n, _)| n), tie)
}

/// A note or chord with resolved values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
    pub pitches: Vec<Pitch>,
    pub len: Tick,
    pub velocity: u8,
    pub marks: Marks,
    /// `sf`, `fp`, ... written just before it.
    pub sudden: Option<Sudden>,
    /// Per pitch: tied to the next sound.
    pub ties: Vec<bool>,
    pub span: SrcSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// Index into [`Resolved::sounds`].
    Sound(usize),
    Rest {
        len: Tick,
        span: SrcSpan,
    },
    Pedal {
        kind: PedalKind,
        on: bool,
        span: SrcSpan,
    },
    Tempo(u16),
    Time(TimeSig),
    Bar(SrcSpan),
    BarRepeat(SrcSpan),
    Repeat {
        body: Vec<Item>,
        endings: Vec<Vec<Item>>,
        times: u32,
        span: SrcSpan,
    },
    Func {
        builtin: Builtin,
        body: Vec<Item>,
    },
    /// Sections that start together.
    Voices {
        voices: Vec<Vec<Item>>,
        span: SrcSpan,
    },
    /// An unmetered stretch (`free`).
    Free(Vec<Item>),
    /// Grace notes (indexes into [`Resolved::sounds`]), for the next sound.
    Grace {
        sounds: Vec<usize>,
        span: SrcSpan,
    },
    Group(Vec<Item>),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Resolved {
    pub items: Vec<Item>,
    pub sounds: Vec<Sound>,
}

/// Resolves `nodes`. If `probe` is set, also returns the state just before the first
/// node at or after that (line, col) position (or the final state).
pub fn resolve(
    nodes: &[Node],
    opts: &CompileOptions,
    probe: Option<(u32, u32)>,
    alerts: &mut Vec<Alert>,
) -> (Resolved, State) {
    let mut r = Resolver {
        state: State::new(opts),
        sounds: Vec::new(),
        cresc: None,
        probe,
        probed: None,
        alerts,
    };
    let items = r.seq(nodes);
    r.end_cresc();
    r.unused_sudden();
    let state = r.probed.unwrap_or(r.state);
    (
        Resolved {
            items,
            sounds: r.sounds,
        },
        state,
    )
}

struct Cresc {
    start: u8,
    rising: bool,
    sounds: Vec<usize>,
    span: SrcSpan,
}

struct Resolver<'a> {
    state: State,
    sounds: Vec<Sound>,
    cresc: Option<Cresc>,
    probe: Option<(u32, u32)>,
    probed: Option<State>,
    alerts: &'a mut Vec<Alert>,
}

impl Resolver<'_> {
    fn seq(&mut self, nodes: &[Node]) -> Vec<Item> {
        nodes.iter().filter_map(|n| self.node(n)).collect()
    }

    fn check_probe(&mut self, span: Option<SrcSpan>) {
        if let (None, Some((line, col)), Some(s)) = (self.probed, self.probe, span)
            && (s.line, s.col) >= (line, col)
        {
            self.probed = Some(self.state);
        }
    }

    fn node(&mut self, node: &Node) -> Option<Item> {
        if !matches!(node, Node::Group(_)) {
            self.check_probe(node.span());
        }
        match node {
            Node::Note {
                name,
                len,
                marks,
                tie,
                span,
            } => {
                let pitch = self.pitch(name, *span);
                let first = pitch.map_or(self.state.reference, |(_, n)| n);
                self.state.reference = first;
                let pitches: Vec<Pitch> = pitch.map(|(p, _)| p).into_iter().collect();
                let ties = vec![*tie; pitches.len()];
                Some(self.sound(pitches, *len, *marks, ties, *span))
            }
            Node::Chord {
                notes,
                len,
                marks,
                tie,
                span,
            } => {
                let before = self.state.reference;
                let mut pitches = Vec::new();
                let mut ties = Vec::new();
                let mut first = None;
                for (name, s, t) in notes {
                    if let Some((p, n)) = self.pitch(name, *s) {
                        first.get_or_insert(n);
                        self.state.reference = n;
                        // `~` after the chord ties every note.
                        match pitches.iter().position(|q| *q == p) {
                            Some(k) => ties[k] |= *t || *tie,
                            None => {
                                pitches.push(p);
                                ties.push(*t || *tie);
                            }
                        }
                    }
                }
                self.state.reference = first.unwrap_or(before);
                Some(self.sound(pitches, *len, *marks, ties, *span))
            }
            Node::Rest { len, span } => {
                if let Some(l) = len {
                    self.state.len = *l;
                }
                Some(Item::Rest {
                    len: self.state.len.ticks(),
                    span: *span,
                })
            }
            Node::Dynamic(d, span) => {
                self.dynamic(*d, *span);
                None
            }
            Node::Sudden(s, span) => {
                self.unused_sudden();
                self.state.sudden = Some((*s, *span));
                None
            }
            Node::Cresc(span) | Node::Dim(span) => {
                self.cresc = Some(Cresc {
                    start: self.state.dynamic.velocity(),
                    rising: matches!(node, Node::Cresc(_)),
                    sounds: Vec::new(),
                    span: *span,
                });
                None
            }
            Node::Pedal { kind, on, span } => {
                let was = self.state.pedals[kind.index()];
                if was == *on {
                    self.alerts.push(Alert::hint(
                        *span,
                        format!(
                            "`{} {}` while already {}",
                            kind.keyword(),
                            if *on { "on" } else { "off" },
                            if was { "on" } else { "off" }
                        ),
                    ));
                }
                self.state.pedals[kind.index()] = *on;
                Some(Item::Pedal {
                    kind: *kind,
                    on: *on,
                    span: *span,
                })
            }
            Node::Tempo(bpm, _) => {
                self.state.tempo = *bpm;
                Some(Item::Tempo(*bpm))
            }
            Node::Time(sig, _) => {
                self.state.time = *sig;
                Some(Item::Time(*sig))
            }
            Node::Bar(span) => Some(Item::Bar(*span)),
            Node::BarRepeat(span) => Some(Item::BarRepeat(*span)),
            Node::Repeat {
                body,
                endings,
                times,
                span,
            } => {
                let body = self.seq(body);
                let after_body = self.state;
                let endings = endings
                    .iter()
                    .map(|e| {
                        self.state = after_body;
                        self.seq(e)
                    })
                    .collect();
                Some(Item::Repeat {
                    body,
                    endings,
                    times: *times,
                    span: *span,
                })
            }
            Node::Func {
                name,
                args,
                body,
                span,
            } => {
                let builtin = Builtin::from_args(name, args, *span, self.alerts);
                let body = self.seq(body);
                Some(match builtin {
                    Some(builtin) => Item::Func { builtin, body },
                    None => Item::Group(body),
                })
            }
            Node::Voices { voices, span } => Some(self.voices(voices, *span)),
            Node::Free { body, .. } => Some(Item::Free(self.seq(body))),
            Node::Grace { body, span } => Some(self.grace(body, *span)),
            Node::Group(nodes) => Some(Item::Group(self.seq(nodes))),
        }
    }

    /// Each voice starts from the state before the block and keeps its own octave,
    /// length, dynamic, and cresc. Pedals, tempo, and time belong to the whole piano:
    /// one changed in any voice stays changed after the block (the first voice that
    /// changes it wins). Otherwise the first voice continues.
    fn voices(&mut self, voices: &[Vec<Node>], span: SrcSpan) -> Item {
        let before = self.state;
        let outer = self.cresc.take();
        let mut pedals = before.pedals;
        let mut first_end = None;
        let (mut tempo, mut time) = (None, None);
        let voices = voices
            .iter()
            .map(|v| {
                self.state = before;
                let items = self.seq(v);
                self.end_cresc();
                self.unused_sudden();
                for (p, (now, was)) in pedals
                    .iter_mut()
                    .zip(self.state.pedals.iter().zip(before.pedals))
                {
                    if *now != was {
                        *p = *now;
                    }
                }
                if self.state.tempo != before.tempo {
                    tempo.get_or_insert(self.state.tempo);
                }
                if self.state.time != before.time {
                    time.get_or_insert(self.state.time);
                }
                first_end.get_or_insert(self.state);
                items
            })
            .collect();
        self.state = first_end.unwrap_or(before);
        self.state.pedals = pedals;
        self.state.tempo = tempo.unwrap_or(before.tempo);
        self.state.time = time.unwrap_or(before.time);
        self.cresc = outer;
        Item::Voices { voices, span }
    }

    /// Grace notes take the octave flow but not the length; they stay out of a cresc.
    fn grace(&mut self, body: &[Node], span: SrcSpan) -> Item {
        let len = self.state.len;
        let cresc = self.cresc.take();
        let sudden = self.state.sudden.take();
        let mut sounds = Vec::new();
        for node in body {
            if let Some(Item::Sound(i)) = self.node(node) {
                self.sounds[i].ties.iter_mut().for_each(|t| *t = false);
                sounds.push(i);
            }
        }
        self.state.len = len;
        self.cresc = cresc;
        self.state.sudden = sudden;
        Item::Grace { sounds, span }
    }

    /// Warns about an `sf` (or its kin) with no note after it.
    fn unused_sudden(&mut self) {
        if let Some((_, span)) = self.state.sudden.take() {
            self.alerts
                .push(Alert::warning(span, "no note after it to stress"));
        }
    }

    /// Warns about a cresc/dim that never reached its target.
    fn end_cresc(&mut self) {
        if let Some(c) = self.cresc.take() {
            self.alerts.push(Alert::warning(
                c.span,
                "cresc/dim has no target dynamic, e.g. `p cresc C D E f`",
            ));
        }
    }

    /// Resolves a note name; returns the folded pitch and the raw MIDI number.
    fn pitch(&mut self, name: &NoteName, span: SrcSpan) -> Option<(Pitch, i32)> {
        let (n, tie) = self.state.resolve_note(name);
        let (pitch, folded) = Pitch::fold(n);
        let raw = crate::builtins::midi_name(n);
        if tie {
            let prev = crate::builtins::midi_name(self.state.reference);
            let below = crate::builtins::midi_name(n - 12);
            self.alerts.push(
                Alert::warning(
                    span,
                    format!(
                        "{prev} → {} is ambiguous (6 semitones), chose {raw}.",
                        letter_text(name)
                    ),
                )
                .with_fix(format!("Write {below} or {raw} to fix.")),
            );
        }
        if folded {
            self.alerts.push(
                Alert::warning(span, format!("{raw} folded to {pitch}"))
                    .with_fix("(outside A0-C8)"),
            );
        }
        Some((pitch, n))
    }

    fn sound(
        &mut self,
        pitches: Vec<Pitch>,
        len: Option<Duration>,
        marks: Marks,
        ties: Vec<bool>,
        span: SrcSpan,
    ) -> Item {
        if let Some(l) = len {
            self.state.len = l;
        }
        let idx = self.sounds.len();
        let sudden = self.state.sudden.take().map(|(s, _)| s);
        self.sounds.push(Sound {
            pitches,
            len: self.state.len.ticks(),
            velocity: self.state.dynamic.velocity(),
            marks,
            sudden,
            ties,
            span,
        });
        // `fp`: the notes after it are soft.
        if sudden.is_some_and(Sudden::then_soft) {
            self.state.dynamic = Dynamic::P;
        }
        if let Some(c) = &mut self.cresc {
            c.sounds.push(idx);
        }
        Item::Sound(idx)
    }

    fn dynamic(&mut self, d: Dynamic, span: SrcSpan) {
        if let Some(c) = self.cresc.take() {
            let target = d.velocity();
            if c.rising != (target > c.start) {
                self.alerts.push(Alert::warning(
                    span,
                    format!(
                        "{} ends at `{}`, which is not {}",
                        if c.rising { "cresc" } else { "dim" },
                        d,
                        if c.rising { "louder" } else { "softer" }
                    ),
                ));
            }
            let n = c.sounds.len() as i32;
            for (i, idx) in c.sounds.iter().enumerate() {
                let v = i32::from(c.start)
                    + (i32::from(target) - i32::from(c.start)) * i as i32 / n.max(1);
                self.sounds[*idx].velocity = crate::builtins::clamp_velocity(v);
            }
        }
        self.state.dynamic = d;
    }
}

fn letter_text(name: &NoteName) -> String {
    let acc = match name.accidental {
        piano_core::Accidental::Natural => "",
        piano_core::Accidental::Sharp => "#",
        piano_core::Accidental::Flat => "b",
    };
    format!("{:?}{acc}", name.letter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::lex, parser::parse};

    fn run(src: &str) -> (Resolved, Vec<Alert>) {
        let mut alerts = Vec::new();
        let nodes = parse(&lex(src), &mut alerts);
        let (r, _) = resolve(&nodes, &CompileOptions::default(), None, &mut alerts);
        (r, alerts)
    }

    fn names(r: &Resolved) -> Vec<String> {
        r.sounds
            .iter()
            .map(|s| {
                s.pitches
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join("+")
            })
            .collect()
    }

    #[test]
    fn nearest_octave() {
        let (r, alerts) = run("C4 D E F G A B C");
        assert!(alerts.is_empty());
        assert_eq!(names(&r).last().unwrap(), "C5");
    }

    #[test]
    fn start_reference_is_c4() {
        let (r, _) = run("E");
        assert_eq!(names(&r), ["E4"]);
    }

    #[test]
    fn tie_goes_up_with_warning() {
        let (r, alerts) = run("C4 F#");
        assert_eq!(names(&r), ["C4", "F#4"]);
        assert_eq!(alerts.len(), 1);
        assert_eq!(
            alerts[0].to_string(),
            "⚠ line 1: C4 → F# is ambiguous (6 semitones), chose F#4. Write F#3 or F#4 to fix."
        );
    }

    #[test]
    fn chord_reference_is_first_note() {
        let (r, _) = run("{C4 E G} B");
        assert_eq!(names(&r), ["C4+E4+G4", "B3"]);
    }

    #[test]
    fn length_and_dots_inherit() {
        let (r, _) = run("C/4. D F/8 G");
        let q = piano_core::TICKS_PER_QUARTER;
        let lens: Vec<Tick> = r.sounds.iter().map(|s| s.len).collect();
        assert_eq!(lens, [q * 3 / 2, q * 3 / 2, q / 2, q / 2]);
    }

    #[test]
    fn rests_keep_reference() {
        let (r, _) = run("B4 r C");
        assert_eq!(names(&r), ["B4", "C5"]);
    }

    #[test]
    fn crescendo_interpolates() {
        let (r, alerts) = run("p cresc C D E F f G");
        assert!(alerts.is_empty(), "{alerts:?}");
        let v: Vec<u8> = r.sounds.iter().map(|s| s.velocity).collect();
        assert_eq!(v, [49, 60, 72, 84, 96]);
    }

    #[test]
    fn folding_written_note() {
        let (r, alerts) = run("C9");
        assert_eq!(names(&r), ["C8"]);
        assert_eq!(
            alerts[0].to_string(),
            "⚠ line 1: C9 folded to C8 (outside A0-C8)"
        );
    }

    #[test]
    fn voices_keep_their_own_state() {
        let (r, alerts) = run("C4/8 voices{ G E mp A }{ p C3/2 E } D");
        assert!(alerts.is_empty(), "{alerts:?}");
        assert_eq!(names(&r), ["C4", "G3", "E3", "A3", "C3", "E3", "D4"]);
        let lens: Vec<Tick> = r.sounds.iter().map(|s| s.len).collect();
        let e = piano_core::TICKS_PER_QUARTER / 2;
        assert_eq!(lens, [e, e, e, e, 4 * e, 4 * e, e]);
        let v: Vec<u8> = r.sounds.iter().map(|s| s.velocity).collect();
        assert_eq!(v, [80, 80, 80, 64, 49, 49, 64]);
    }

    #[test]
    fn tempo_inside_voices_goes_on_after_the_block() {
        let mut alerts = Vec::new();
        let nodes = parse(&lex("voices{ C }{ tempo 90 time 3/4 E } G"), &mut alerts);
        let (r, s) = resolve(&nodes, &CompileOptions::default(), None, &mut alerts);
        assert!(alerts.is_empty(), "{alerts:?}");
        assert!(matches!(&r.items[0], Item::Voices { voices, .. } if voices[1].len() == 3));
        assert_eq!(s.tempo, 90);
        assert_eq!(s.time.to_string(), "3/4");
    }

    #[test]
    fn probe_state() {
        let mut alerts = Vec::new();
        let nodes = parse(&lex("C4/8 ped on\nf E G"), &mut alerts);
        let (_, s) = resolve(
            &nodes,
            &CompileOptions::default(),
            Some((2, 3)),
            &mut alerts,
        );
        assert_eq!(s.dynamic, Dynamic::F);
        assert!(s.pedal(PedalKind::Damper));
        assert_eq!(s.len.denom(), 8);
        assert_eq!(s.reference_pitch().to_string(), "C4");
    }
}
