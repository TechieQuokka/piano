//! Layout: places resolved items on the timeline, expanding repeats, `%`, ties, and
//! builtin functions.

use piano_core::{Alert, NoteEvent, PedalEvent, SrcSpan, TempoEvent, Tick, TimeSig, TimeSigEvent};

use crate::builtins::Builtin;
use crate::resolve::{Item, Resolved};

/// A laid-out section, starting at tick 0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Track {
    pub notes: Vec<NoteEvent>,
    pub pedals: Vec<PedalEvent>,
    pub tempos: Vec<TempoEvent>,
    pub time_sigs: Vec<TimeSigEvent>,
    pub len: Tick,
}

impl Track {
    /// Appends `other` shifted to start at `at`, extending `len` as needed.
    pub fn append(&mut self, other: Track, at: Tick) {
        self.notes.extend(other.notes.into_iter().map(|mut n| {
            n.start += at;
            n
        }));
        self.pedals.extend(other.pedals.into_iter().map(|mut p| {
            p.at += at;
            p
        }));
        self.tempos.extend(other.tempos.into_iter().map(|mut t| {
            t.at += at;
            t
        }));
        self.time_sigs
            .extend(other.time_sigs.into_iter().map(|mut t| {
                t.at += at;
                t
            }));
        self.len = self.len.max(at + other.len);
    }
}

/// Lays out a resolved piece.
pub fn layout(resolved: &Resolved, time: TimeSig, alerts: &mut Vec<Alert>) -> Track {
    let mut ctx = Ctx::new(time);
    let mut track = Track::default();
    lay_seq(&resolved.items, resolved, &mut ctx, &mut track, alerts);
    ctx.flush_ties(alerts);
    // A short bar with nothing after it is the last bar: it may be short.
    ctx.settle(track.len, alerts);
    track
}

struct Ctx {
    time: TimeSig,
    bar_prev: Tick,
    bar_cur: Tick,
    bars_seen: u32,
    /// Part of the current bar played before this context started (inside a block).
    lead: Tick,
    /// Real time per tick of this context, as (numerator, denominator): less than one
    /// inside `tuplet`.
    scale: (Tick, Tick),
    /// The current bar holds a `free` block: its length is not checked.
    free: bool,
    /// In a voice: the block's time changes (from any voice), by time in the voice.
    /// They replace the voice's own `time`.
    schedule: Option<Vec<(Tick, TimeSig)>>,
    /// Notes waiting for a tie continuation: (note index, span of the tied sound).
    ties: Vec<(usize, SrcSpan)>,
    /// Bars shorter than the time signature, by the time they end: a warning once
    /// anything takes time after them (the last bar may be short).
    short: Vec<(Tick, Alert)>,
}

impl Ctx {
    fn new(time: TimeSig) -> Ctx {
        Ctx {
            time,
            bar_prev: 0,
            bar_cur: 0,
            bars_seen: 0,
            lead: 0,
            scale: (1, 1),
            free: false,
            schedule: None,
            ties: Vec::new(),
            short: Vec::new(),
        }
    }

    /// How much of the current bar has passed at `now`, in real ticks.
    fn bar_so_far(&self, now: Tick) -> Tick {
        (now - self.bar_cur) * self.scale.0 / self.scale.1 + self.lead
    }

    /// A context for a block starting at `now` that goes on with this one's bar: its
    /// own ties and track. `scale` is the block's time against this context's.
    fn child(&self, now: Tick, scale: (Tick, Tick)) -> Ctx {
        let mut c = Ctx::new(self.time);
        c.bars_seen = self.bars_seen;
        c.lead = self.bar_so_far(now);
        c.free = self.free;
        c.scale = (self.scale.0 * scale.0, self.scale.1 * scale.1);
        c
    }

    /// Goes on after a block laid out in `sub` from `now`: its bars, time, and short
    /// bars. `map` turns the block's times into this context's.
    fn rejoin(&mut self, sub: &mut Ctx, now: Tick, map: impl Fn(Tick) -> Tick) {
        if sub.bars_seen > self.bars_seen {
            // With one bar line inside, the bar it ended began out here.
            if sub.bars_seen - self.bars_seen >= 2 {
                self.bar_prev = now + map(sub.bar_prev);
            } else {
                self.bar_prev = self.bar_cur;
            }
            self.bar_cur = now + map(sub.bar_cur);
            self.bars_seen = sub.bars_seen;
            self.lead = 0;
        }
        self.free = sub.free;
        self.time = sub.time;
        self.short
            .extend(sub.short.drain(..).map(|(at, a)| (now + map(at), a)));
    }

    /// Warns about the short bars that end before `now`.
    fn settle(&mut self, now: Tick, alerts: &mut Vec<Alert>) {
        let (late, keep): (Vec<_>, Vec<_>) = self.short.drain(..).partition(|s| s.0 < now);
        alerts.extend(late.into_iter().map(|s| s.1));
        self.short = keep;
    }

    /// Takes the block's time changes before `now`, or at `now` too with `at`.
    fn follow(&mut self, now: Tick, at: bool) {
        if let Some(s) = &self.schedule {
            for &(t, sig) in s {
                if t < now || (at && t == now) {
                    self.time = sig;
                }
            }
        }
    }

    fn flush_ties(&mut self, alerts: &mut Vec<Alert>) {
        for (_, span) in self.ties.drain(..) {
            alerts.push(Alert::warning(
                span,
                "tie `~` is not followed by the same note, ignored",
            ));
        }
    }
}

fn lay_seq(
    items: &[Item],
    resolved: &Resolved,
    ctx: &mut Ctx,
    track: &mut Track,
    alerts: &mut Vec<Alert>,
) {
    for item in items {
        lay_item(item, resolved, ctx, track, alerts);
    }
}

fn lay_item(
    item: &Item,
    resolved: &Resolved,
    ctx: &mut Ctx,
    track: &mut Track,
    alerts: &mut Vec<Alert>,
) {
    let now = track.len;
    ctx.follow(now, false);
    ctx.settle(now, alerts);
    match item {
        Item::Sound(idx) => {
            let sound = &resolved.sounds[*idx];
            let mut pending = std::mem::take(&mut ctx.ties);
            let mut new_ties = Vec::new();
            // A staccato lets the key go early; a note tied on is held to its end.
            let cut = sound.len - sound.len * sound.marks.gate() / 100;
            for (&pitch, &tie) in sound.pitches.iter().zip(&sound.ties) {
                let cut = if tie { 0 } else { cut };
                let continued = pending
                    .iter()
                    .position(|(i, _)| track.notes[*i].pitch == pitch);
                let note_idx = match continued {
                    Some(k) => {
                        let (i, _) = pending.remove(k);
                        let note = &mut track.notes[i];
                        note.len = now + sound.len - note.start;
                        note.cut = cut;
                        i
                    }
                    None => {
                        track.notes.push(NoteEvent {
                            pitch,
                            start: now,
                            len: sound.len,
                            velocity: crate::builtins::clamp_velocity(
                                i32::from(sound.velocity) + sound.marks.boost(),
                            ),
                            cut,
                            src: sound.span,
                        });
                        track.notes.len() - 1
                    }
                };
                if tie {
                    new_ties.push((note_idx, sound.span));
                }
            }
            ctx.ties = pending;
            ctx.flush_ties(alerts);
            ctx.ties = new_ties;
            track.len = now + sound.len;
        }
        Item::Rest { len, .. } => {
            ctx.flush_ties(alerts);
            track.len = now + len;
        }
        Item::Pedal { kind, on, span } => track.pedals.push(PedalEvent {
            kind: *kind,
            on: *on,
            at: now,
            src: *span,
        }),
        Item::Tempo(bpm) => track.tempos.push(TempoEvent { at: now, bpm: *bpm }),
        Item::Time(sig) => {
            if ctx.schedule.is_none() {
                ctx.time = *sig;
            }
            track.time_sigs.push(TimeSigEvent { at: now, sig: *sig });
        }
        Item::Bar(span) => {
            let len = ctx.bar_so_far(now);
            ctx.lead = 0;
            let expected = ctx.time.bar_ticks();
            let check = if ctx.bars_seen > 0 {
                len != expected
            } else {
                len > expected
            };
            if check && len > 0 && !ctx.free {
                let alert = Alert::warning(
                    *span,
                    format!(
                        "bar has {} beats, {} expects {}",
                        beats(len, ctx.time),
                        ctx.time,
                        ctx.time.beats
                    ),
                )
                .with_fix("(bar lines are optional)");
                if len < expected {
                    ctx.short.push((now, alert));
                } else {
                    alerts.push(alert);
                }
            }
            ctx.bar_prev = ctx.bar_cur;
            ctx.bar_cur = now;
            ctx.bars_seen += 1;
            ctx.free = false;
            ctx.follow(now, true);
        }
        Item::BarRepeat(span) => {
            let (from, to) = (ctx.bar_prev, ctx.bar_cur);
            if to <= from {
                alerts.push(Alert::warning(*span, "`%` has no previous bar to repeat"));
                return;
            }
            ctx.flush_ties(alerts);
            let copy = Track {
                notes: track
                    .notes
                    .iter()
                    .filter(|n| (from..to).contains(&n.start))
                    .map(|n| NoteEvent {
                        start: n.start - from,
                        len: n.len.min(to - n.start),
                        src: *span,
                        ..n.clone()
                    })
                    .collect(),
                pedals: track
                    .pedals
                    .iter()
                    .filter(|p| (from..to).contains(&p.at))
                    .map(|p| PedalEvent {
                        at: p.at - from,
                        src: *span,
                        ..p.clone()
                    })
                    .collect(),
                len: to - from,
                ..Track::default()
            };
            track.append(copy, now);
        }
        Item::Repeat {
            body,
            endings,
            times,
            span,
        } => {
            // The passes are laid out as they are heard, so bars, `time`, and `%` go
            // on through them. Ties stop at the repeat signs.
            ctx.flush_ties(alerts);
            repeat_sign(ctx, now, *span, alerts);
            let passes = if endings.is_empty() {
                *times as usize
            } else {
                endings.len()
            };
            // Every pass reads the same text, under the time written before it.
            let time = ctx.time;
            for k in 0..passes {
                // Later passes only add new alerts.
                let mut quiet = Vec::new();
                let sink = if k == 0 { &mut *alerts } else { &mut quiet };
                ctx.time = time;
                lay_seq(body, resolved, ctx, track, sink);
                if let Some(ending) = endings.get(k) {
                    repeat_sign(ctx, track.len, *span, sink);
                    lay_seq(ending, resolved, ctx, track, sink);
                }
                ctx.flush_ties(sink);
                if endings.is_empty() || k + 1 < passes {
                    repeat_sign(ctx, track.len, *span, sink);
                }
                for a in quiet {
                    if !alerts.contains(&a) {
                        alerts.push(a);
                    }
                }
            }
        }
        Item::Func { builtin, body } => {
            // Ties go on into the body and out of it, as across a bar line. So do bars,
            // in the body's real time: squeezed by `tuplet`, every pass of `seq`.
            let squeeze = match builtin {
                Builtin::Tuplet(n) => (Tick::from(lower_power_of_two(*n)), Tick::from(*n)),
                _ => (1, 1),
            };
            let mut sub = ctx.child(now, squeeze);
            let mut body_track = Track::default();
            lay_seq(body, resolved, &mut sub, &mut body_track, alerts);
            if let Builtin::Seq { times, .. } = builtin {
                // The other passes only move the bars on; their alerts repeat the first's.
                let ties = std::mem::take(&mut sub.ties);
                let mut quiet = Vec::new();
                let mut pass = Track {
                    len: body_track.len,
                    ..Track::default()
                };
                for _ in 1..*times {
                    lay_seq(body, resolved, &mut sub, &mut pass, &mut quiet);
                    sub.flush_ties(&mut quiet);
                }
                for a in quiet {
                    if !alerts.contains(&a) {
                        alerts.push(a);
                    }
                }
                sub.ties = ties;
            }
            let count = body_track.notes.len();
            let out = builtin.apply(body_track, alerts);
            // A function that repeats its body (`seq`) ties out of its last pass.
            let offset = out.notes.len().saturating_sub(count);
            if offset + count > out.notes.len() {
                sub.flush_ties(alerts);
            }
            let base = track.notes.len();
            track.append(out, now);
            let leaving: Vec<(usize, SrcSpan)> = std::mem::take(&mut sub.ties)
                .into_iter()
                .map(|(i, span)| (base + offset + i, span))
                .collect();
            let into = std::mem::take(&mut ctx.ties);
            ctx.ties = tie_into(track, into, base, now, leaving, alerts);
            ctx.rejoin(&mut sub, now, |t| t * squeeze.0 / squeeze.1);
        }
        Item::Voices { voices, span } => {
            // Ties go on into the block (in the first voice with the note) and out of
            // it (from a note that ends with the block).
            let into = std::mem::take(&mut ctx.ties);
            // `time` in any voice applies to all of them: lay the voices out once to
            // find the changes, then again following them.
            let mut first = Vec::new();
            let (mut tracks, mut ctxs) = lay_voices(voices, resolved, ctx, now, None, &mut first);
            let (sigs, sig_clash) = merge(tracks.iter().map(|t| {
                t.time_sigs
                    .iter()
                    .map(|e| (e.at, e.sig))
                    .collect::<Vec<_>>()
            }));
            if sigs.is_empty() {
                alerts.extend(first);
            } else {
                (tracks, ctxs) = lay_voices(voices, resolved, ctx, now, Some(&sigs), alerts);
            }
            let (tempos, tempo_clash) = merge(
                tracks
                    .iter()
                    .map(|t| t.tempos.iter().map(|e| (e.at, e.bpm)).collect::<Vec<_>>()),
            );
            for (clash, what) in [(sig_clash, "time"), (tempo_clash, "tempo")] {
                if clash {
                    alerts.push(
                        Alert::warning(
                            *span,
                            format!("voices set `{what}` differently at the same point"),
                        )
                        .with_fix("(the first voice's is used)"),
                    );
                }
            }
            // The first voice carries the block's tempo and time changes.
            for (k, t) in tracks.iter_mut().enumerate() {
                t.time_sigs.clear();
                t.tempos.clear();
                if k == 0 {
                    t.time_sigs = sigs
                        .iter()
                        .map(|&(at, sig)| TimeSigEvent { at, sig })
                        .collect();
                    t.tempos = tempos
                        .iter()
                        .map(|&(at, bpm)| TempoEvent { at, bpm })
                        .collect();
                }
            }
            // Bar lines continue from those of the first voice.
            let free = ctxs.iter().any(|v| v.free);
            for (k, v) in ctxs.iter_mut().enumerate() {
                if k == 0 {
                    ctx.rejoin(v, now, |t| t);
                } else {
                    ctx.short
                        .extend(v.short.drain(..).map(|(at, a)| (now + at, a)));
                }
            }
            // A `free` block in any voice frees the bar the block ends in.
            ctx.free = free;
            if let Some(&(_, sig)) = sigs.last() {
                ctx.time = sig;
            }
            let lens: Vec<Tick> = tracks.iter().map(|t| t.len).collect();
            if lens.iter().any(|&l| l != lens[0]) {
                let beats: Vec<String> = lens.iter().map(|&l| beats(l, ctx.time)).collect();
                alerts.push(
                    Alert::warning(
                        *span,
                        format!("voices have different lengths ({} beats)", beats.join(", ")),
                    )
                    .with_fix("(the longest one is used)"),
                );
            }
            let end = lens.iter().copied().max().unwrap_or(0);
            let mut leaving: Vec<(usize, SrcSpan)> = Vec::new();
            let mut pitches = Vec::new();
            let base = track.notes.len();
            for (t, v) in tracks.into_iter().zip(&mut ctxs) {
                let first = track.notes.len();
                for (i, span) in v.ties.drain(..) {
                    let n = &t.notes[i];
                    if n.start + n.len == end && !pitches.contains(&n.pitch) {
                        pitches.push(n.pitch);
                        leaving.push((first + i, span));
                    } else {
                        alerts.push(Alert::warning(
                            span,
                            "tie `~` is not followed by the same note, ignored",
                        ));
                    }
                }
                track.append(t, now);
            }
            ctx.ties = tie_into(track, into, base, now, leaving, alerts);
        }
        Item::Group(items) => lay_seq(items, resolved, ctx, track, alerts),
        Item::Free(items) => {
            lay_seq(items, resolved, ctx, track, alerts);
            ctx.free = true;
        }
    }
}

fn lay_voices(
    voices: &[Vec<Item>],
    resolved: &Resolved,
    outer: &Ctx,
    now: Tick,
    schedule: Option<&[(Tick, TimeSig)]>,
    alerts: &mut Vec<Alert>,
) -> (Vec<Track>, Vec<Ctx>) {
    voices
        .iter()
        .map(|v| voice_track(v, resolved, outer, now, schedule, alerts))
        .unzip()
}

/// Joins the changes of each voice in order; at a time already taken the first
/// voice's change stays. Also says whether two voices disagreed.
fn merge<T: PartialEq + Copy>(
    voices: impl Iterator<Item = Vec<(Tick, T)>>,
) -> (Vec<(Tick, T)>, bool) {
    let mut out: Vec<(Tick, T)> = Vec::new();
    let mut clash = false;
    for changes in voices {
        let theirs = out.len();
        for (at, v) in changes {
            // Within one voice the last change at a time wins, as outside `voices`.
            match out.iter().position(|e| e.0 == at) {
                Some(i) if i >= theirs => out[i].1 = v,
                Some(i) => clash |= out[i].1 != v,
                None => out.push((at, v)),
            }
        }
    }
    out.sort_by_key(|e| e.0);
    (out, clash)
}

/// Lays out one voice starting at `now`; its bar lines continue the current bar.
/// Returns the voice's context too, for the bar lines after the block.
fn voice_track(
    items: &[Item],
    resolved: &Resolved,
    outer: &Ctx,
    now: Tick,
    schedule: Option<&[(Tick, TimeSig)]>,
    alerts: &mut Vec<Alert>,
) -> (Track, Ctx) {
    let mut ctx = outer.child(now, (1, 1));
    ctx.schedule = schedule.map(<[_]>::to_vec);
    let mut track = Track::default();
    lay_seq(items, resolved, &mut ctx, &mut track, alerts);
    // Ties left at the end go on after the block, if the note ends with it.
    ctx.settle(track.len, alerts);
    (track, ctx)
}

/// Joins the notes tied into a block (`into`, notes before `base`) with the notes
/// that start the block at `now`, in order (the first voice first). `leaving` are the
/// block's notes tied out of it; returns them with their indexes after the joins.
fn tie_into(
    track: &mut Track,
    into: Vec<(usize, SrcSpan)>,
    base: usize,
    now: Tick,
    mut leaving: Vec<(usize, SrcSpan)>,
    alerts: &mut Vec<Alert>,
) -> Vec<(usize, SrcSpan)> {
    let mut removed = Vec::new();
    for (p, span) in into {
        let pitch = track.notes[p].pitch;
        let q = (base..track.notes.len()).find(|&q| {
            let n = &track.notes[q];
            n.start == now && n.pitch == pitch && !removed.contains(&q)
        });
        let Some(q) = q else {
            alerts.push(Alert::warning(
                span,
                "tie `~` is not followed by the same note, ignored",
            ));
            continue;
        };
        let end = track.notes[q].start + track.notes[q].len;
        track.notes[p].len = end - track.notes[p].start;
        for l in &mut leaving {
            if l.0 == q {
                l.0 = p;
            }
        }
        removed.push(q);
    }
    removed.sort_unstable();
    for &q in removed.iter().rev() {
        track.notes.remove(q);
    }
    for l in &mut leaving {
        l.0 -= removed.iter().filter(|&&q| q < l.0).count();
    }
    leaving
}

/// A repeat sign at `now` (`|:`, `:|`, `[n`). It is a bar line when the bar is full or
/// is the pickup; inside a bar it is not, and the bar is checked whole as heard (a
/// bar split by the sign: the end of a pass and the start of the next one).
fn repeat_sign(ctx: &mut Ctx, now: Tick, span: SrcSpan, alerts: &mut Vec<Alert>) {
    let len = ctx.bar_so_far(now);
    let expected = ctx.time.bar_ticks();
    if len == 0 || (len < expected && ctx.bars_seen > 0 && !ctx.free) {
        return;
    }
    if len > expected && !ctx.free {
        alerts.push(
            Alert::warning(
                span,
                format!(
                    "bar has {} beats, {} expects {}",
                    beats(len, ctx.time),
                    ctx.time,
                    ctx.time.beats
                ),
            )
            .with_fix("(at a repeat sign)"),
        );
    }
    ctx.lead = 0;
    ctx.bar_prev = ctx.bar_cur;
    ctx.bar_cur = now;
    ctx.bars_seen += 1;
    ctx.free = false;
    ctx.follow(now, true);
}

/// The power of two that `tuplet(n)` plays its notes in the time of.
fn lower_power_of_two(n: u32) -> u32 {
    1 << (31 - (n - 1).leading_zeros())
}

/// Bar length in beats of the time signature's unit, e.g. "3" or "3.5".
fn beats(len: Tick, time: TimeSig) -> String {
    let unit = piano_core::TICKS_PER_WHOLE / Tick::from(time.unit);
    if len.is_multiple_of(unit) {
        (len / unit).to_string()
    } else {
        format!("{:.2}", len as f64 / unit as f64)
            .trim_end_matches('0')
            .to_string()
    }
}
