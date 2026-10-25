//! Layout: places resolved items on the timeline, expanding repeats, `%`, ties, and
//! builtin functions.

use piano_core::{Alert, NoteEvent, PedalEvent, SrcSpan, TempoEvent, Tick, TimeSig, TimeSigEvent};

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
    track
}

struct Ctx {
    time: TimeSig,
    bar_prev: Tick,
    bar_cur: Tick,
    bars_seen: u32,
    /// Part of the current bar played before this context started (inside `voices`).
    lead: Tick,
    /// Notes waiting for a tie continuation: (note index, span of the tied sound).
    ties: Vec<(usize, SrcSpan)>,
}

impl Ctx {
    fn new(time: TimeSig) -> Ctx {
        Ctx {
            time,
            bar_prev: 0,
            bar_cur: 0,
            bars_seen: 0,
            lead: 0,
            ties: Vec::new(),
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

/// Lays out `items` in a fresh context (own ties and bars) as a separate track.
fn sub_track(items: &[Item], resolved: &Resolved, time: TimeSig, alerts: &mut Vec<Alert>) -> Track {
    let mut ctx = Ctx::new(time);
    let mut track = Track::default();
    lay_seq(items, resolved, &mut ctx, &mut track, alerts);
    ctx.flush_ties(alerts);
    track
}

fn lay_item(
    item: &Item,
    resolved: &Resolved,
    ctx: &mut Ctx,
    track: &mut Track,
    alerts: &mut Vec<Alert>,
) {
    let now = track.len;
    match item {
        Item::Sound(idx) => {
            let sound = &resolved.sounds[*idx];
            let mut pending = std::mem::take(&mut ctx.ties);
            let mut new_ties = Vec::new();
            for &pitch in &sound.pitches {
                let continued = pending
                    .iter()
                    .position(|(i, _)| track.notes[*i].pitch == pitch);
                let note_idx = match continued {
                    Some(k) => {
                        let (i, _) = pending.remove(k);
                        let note = &mut track.notes[i];
                        note.len = now + sound.len - note.start;
                        i
                    }
                    None => {
                        track.notes.push(NoteEvent {
                            pitch,
                            start: now,
                            len: sound.len,
                            velocity: sound.velocity,
                            src: sound.span,
                        });
                        track.notes.len() - 1
                    }
                };
                if sound.tie {
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
            ctx.time = *sig;
            track.time_sigs.push(TimeSigEvent { at: now, sig: *sig });
        }
        Item::Bar(span) => {
            let len = now - ctx.bar_cur + std::mem::take(&mut ctx.lead);
            let expected = ctx.time.bar_ticks();
            let check = if ctx.bars_seen > 0 {
                len != expected
            } else {
                len > expected
            };
            if check && len > 0 {
                alerts.push(
                    Alert::warning(
                        *span,
                        format!(
                            "bar has {} beats, {} expects {}",
                            beats(len, ctx.time),
                            ctx.time,
                            ctx.time.beats
                        ),
                    )
                    .with_fix("(bar lines are optional)"),
                );
            }
            ctx.bar_prev = ctx.bar_cur;
            ctx.bar_cur = now;
            ctx.bars_seen += 1;
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
        } => {
            ctx.flush_ties(alerts);
            let body_track = sub_track(body, resolved, ctx.time, alerts);
            let passes: Vec<Track> = if endings.is_empty() {
                (0..*times).map(|_| body_track.clone()).collect()
            } else {
                endings
                    .iter()
                    .map(|e| {
                        let mut pass = body_track.clone();
                        let at = pass.len;
                        pass.append(sub_track(e, resolved, ctx.time, alerts), at);
                        pass
                    })
                    .collect()
            };
            for pass in passes {
                let at = track.len;
                track.append(pass, at);
            }
            ctx.bar_prev = track.len;
            ctx.bar_cur = track.len;
        }
        Item::Func { builtin, body } => {
            ctx.flush_ties(alerts);
            let body_track = sub_track(body, resolved, ctx.time, alerts);
            let out = builtin.apply(body_track, alerts);
            track.append(out, now);
        }
        Item::Voices { voices, span } => {
            ctx.flush_ties(alerts);
            let (tracks, ctxs): (Vec<Track>, Vec<Ctx>) = voices
                .iter()
                .map(|v| voice_track(v, resolved, ctx, now, alerts))
                .unzip();
            // Bar lines continue from those of the first voice.
            if let Some(v) = ctxs.first()
                && v.bars_seen > ctx.bars_seen
            {
                let outer = |t: Tick| if t == 0 { ctx.bar_cur } else { now + t };
                (ctx.bar_prev, ctx.bar_cur) = (outer(v.bar_prev), outer(v.bar_cur));
                ctx.bars_seen = v.bars_seen;
                ctx.lead = 0;
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
            for t in tracks {
                track.append(t, now);
            }
        }
        Item::Group(items) => lay_seq(items, resolved, ctx, track, alerts),
    }
}

/// Lays out one voice starting at `now`; its bar lines continue the current bar.
/// Returns the voice's context too, for the bar lines after the block.
fn voice_track(
    items: &[Item],
    resolved: &Resolved,
    outer: &Ctx,
    now: Tick,
    alerts: &mut Vec<Alert>,
) -> (Track, Ctx) {
    let mut ctx = Ctx::new(outer.time);
    ctx.bars_seen = outer.bars_seen;
    ctx.lead = now - outer.bar_cur + outer.lead;
    let mut track = Track::default();
    lay_seq(items, resolved, &mut ctx, &mut track, alerts);
    ctx.flush_ties(alerts);
    (track, ctx)
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
