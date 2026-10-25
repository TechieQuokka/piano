//! [`Model`] → pieces of piano-language code, one `voices` block per bar.
//!
//! - Blocks are bars, split at the half bar when the tempo changes there, and
//!   merged across bar lines that a tie crosses (ties do not leave a block).
//! - Each MusicXML voice is a voice; notes that overlap inside one voice go to an
//!   extra voice. Pedals and grace notes get voices of their own.
//! - Rubato becomes one `tempo` per block: the harmonic mean, so time is kept.
//! - Dynamics follow the marks of each staff; wedges step through the levels.

use std::collections::BTreeMap;

use crate::ImportError;
use crate::musicxml::{Model, Pedal, StaffId, VoiceId, Wedge};
use crate::rational::Q;

const LEVELS: [&str; 8] = ["ppp", "pp", "p", "mp", "mf", "f", "ff", "fff"];
/// Level before the first mark (the language's default, mf).
const DEFAULT_LEVEL: usize = 4;
/// Tuplets the language plays exactly (`tuplet(n)`, in the time of a power of two).
const TUPLETS: [u32; 3] = [3, 5, 7];
/// Pieces longer than this many bars are cut into chunks of [`CHUNK`].
const MAX_PIECE: usize = 32;
const CHUNK: usize = 16;

fn grace_len() -> Q {
    Q::new(1, 16) // a 64th
}

fn trill_len() -> Q {
    Q::new(1, 8) // a 32nd
}

/// `/n`, `/n.`, `/n..` for a written length in quarters.
fn length_token(len: Q) -> Option<String> {
    for d in [1, 2, 4, 8, 16, 32, 64] {
        let base = Q::new(4, d);
        if len == base {
            return Some(format!("/{d}"));
        }
        if len == base * Q::new(3, 2) {
            return Some(format!("/{d}."));
        }
        if len == base * Q::new(7, 4) {
            return Some(format!("/{d}.."));
        }
    }
    None
}

/// Sounding / written length inside `tuplet(n)`.
fn scale(n: u32) -> Q {
    let mut p = 1;
    while p * 2 < n {
        p *= 2;
    }
    Q::new(i64::from(p), i64::from(n))
}

/// Written length and tuplet for a note sounding `len` quarters.
fn note_length(len: Q) -> Option<(Q, Option<u32>)> {
    if length_token(len).is_some() {
        return Some((len, None));
    }
    TUPLETS.iter().find_map(|&n| {
        let w = len / scale(n);
        length_token(w).map(|_| (w, Some(n)))
    })
}

/// Undotted lengths adding up to `len` (a multiple of a 64th).
fn binary(mut len: Q) -> Vec<Q> {
    let mut out = Vec::new();
    for d in [1, 2, 4, 8, 16, 32, 64] {
        let t = Q::new(4, d);
        while len >= t {
            out.push(t);
            len = len - t;
        }
    }
    out
}

fn is_64ths(len: Q) -> bool {
    (len / grace_len()).is_integer()
}

/// Rests filling `len` quarters: (written length, tuplet).
fn rest_lengths(len: Q) -> Option<Vec<(Q, Option<u32>)>> {
    if length_token(len).is_some() {
        return Some(vec![(len, None)]);
    }
    if is_64ths(len) {
        return Some(binary(len).into_iter().map(|t| (t, None)).collect());
    }
    let tuplet = |r: Q| {
        TUPLETS.iter().find_map(|&n| {
            let w = r / scale(n);
            is_64ths(w).then(|| {
                binary(w)
                    .into_iter()
                    .map(|t| (t, Some(n)))
                    .collect::<Vec<_>>()
            })
        })
    };
    if let Some(t) = tuplet(len) {
        return Some(t);
    }
    // a plain part, then a tuplet part
    let units = (len / grace_len()).floor();
    for k in (1..=units).rev() {
        let x = grace_len() * Q::int(k);
        if let Some(t) = tuplet(len - x) {
            let mut out: Vec<_> = binary(x).into_iter().map(|t| (t, None)).collect();
            out.extend(t);
            return Some(out);
        }
    }
    None
}

#[derive(Debug, Clone)]
struct Ev {
    start: Q,
    len: Q,
    /// (MIDI, name), sorted by pitch.
    notes: Vec<(i32, String)>,
    tie: bool,
    staff: StaffId,
}

impl Ev {
    fn end(&self) -> Q {
        self.start + self.len
    }
}

/// A voice of the output: a MusicXML voice, an extra lane of it, or the grace voice.
type Lane = (usize, u32, u32);
const GRACE_LANE: Lane = (usize::MAX, 0, 0);

/// Spreads the notes of each voice over lanes so that nothing overlaps.
fn lanes(model: &Model) -> BTreeMap<Lane, Vec<Ev>> {
    let mut out: BTreeMap<Lane, Vec<Ev>> = BTreeMap::new();
    for n in &model.notes {
        let (part, voice): VoiceId = n.voice;
        let expanded = n.trill.as_ref().and_then(|(upper, umidi)| {
            let step = [trill_len(), grace_len()]
                .into_iter()
                .find(|s| (n.len / *s).is_integer() && n.len / *s >= Q::int(2))?;
            let count = (n.len / step).floor();
            Some(
                (0..count)
                    .map(|i| Ev {
                        start: n.start + step * Q::int(i),
                        len: step,
                        notes: vec![if i % 2 == 0 {
                            (n.midi, n.name.clone())
                        } else {
                            (*umidi, upper.clone())
                        }],
                        tie: false,
                        staff: n.staff,
                    })
                    .collect::<Vec<_>>(),
            )
        });
        let evs = expanded.unwrap_or_else(|| {
            vec![Ev {
                start: n.start,
                len: n.len,
                notes: vec![(n.midi, n.name.clone())],
                tie: n.tie,
                staff: n.staff,
            }]
        });
        let single = evs.len() == 1;
        for ev in evs {
            let mut lane = 0;
            loop {
                let key = (part, voice, lane);
                let list = out.entry(key).or_default();
                match list.last_mut() {
                    Some(last)
                        if single
                            && last.start == ev.start
                            && last.len == ev.len
                            && last.tie == ev.tie =>
                    {
                        if !last.notes.contains(&ev.notes[0]) {
                            last.notes.push(ev.notes[0].clone());
                            last.notes.sort();
                        }
                        break;
                    }
                    Some(last) if last.end() > ev.start => lane += 1,
                    _ => {
                        list.push(ev);
                        break;
                    }
                }
            }
        }
    }
    // Grace notes: just before their note (on the beat if that leaves the bar).
    let mut graces = Vec::new();
    for g in &model.graces {
        let before = g.principal - grace_len() * Q::int((g.count - g.index) as i64);
        let bar = |t: Q| model.measures.iter().rposition(|m| m.start <= t);
        let start = if before >= Q::ZERO && bar(before) == bar(g.principal) {
            before
        } else {
            g.principal + grace_len() * Q::int(g.index as i64)
        };
        graces.push(Ev {
            start,
            len: grace_len(),
            notes: vec![(g.midi, g.name.clone())],
            tie: false,
            staff: g.staff,
        });
    }
    graces.sort_by_key(|e| e.start);
    let mut merged: Vec<Ev> = Vec::new();
    for g in graces {
        match merged.last_mut() {
            Some(last) if last.start == g.start => {
                if !last.notes.contains(&g.notes[0]) {
                    last.notes.push(g.notes[0].clone());
                    last.notes.sort();
                }
            }
            Some(last) if last.end() > g.start => {} // overlapping graces: keep the first
            _ => merged.push(g),
        }
    }
    if !merged.is_empty() {
        out.insert(GRACE_LANE, merged);
    }
    out.retain(|_, v| !v.is_empty());
    out
}

pub struct Written {
    /// (name, source)
    pub pieces: Vec<(String, String)>,
    pub song: String,
    /// Notes the pieces must play: (start, MIDI, length), ties merged.
    pub expected: Vec<(Q, i32, Q)>,
    pub blocks: usize,
    pub tempos: usize,
    pub graces: usize,
    pub trills: usize,
}

struct Ctx<'a> {
    model: &'a Model,
    lanes: BTreeMap<Lane, Vec<Ev>>,
    starts: Vec<Q>,
    /// (time, text) with the pedal state machine applied.
    pedals: Vec<(Q, &'static str)>,
}

pub fn write(model: &Model, source: &str) -> Result<Written, ImportError> {
    if model.measures.is_empty() {
        return Err(ImportError::Format("no measures".into()));
    }
    let lanes = lanes(model);
    let mut starts: Vec<Q> = model.measures.iter().map(|m| m.start).collect();
    let end = model.measures.last().map_or(Q::ZERO, |m| m.start + m.len);
    starts.push(end);
    // Pedal: a press while down is a change; a release while up is dropped.
    let mut pedals = Vec::new();
    let mut down = false;
    for &(at, p) in &model.pedals {
        match (p, down) {
            (Pedal::Down | Pedal::Change, true) => pedals.push((at, "ped off ped on")),
            (Pedal::Down | Pedal::Change, false) => pedals.push((at, "ped on")),
            (Pedal::Up, true) => pedals.push((at, "ped off")),
            (Pedal::Up, false) => continue,
        }
        down = p != Pedal::Up;
    }
    let ctx = Ctx {
        model,
        lanes,
        starts,
        pedals,
    };
    let sections = ctx.sections();
    let width = if sections.len() >= 10 { 2 } else { 1 };
    let mut pieces = Vec::new();
    let (mut blocks, mut tempos) = (0, 0);
    for (k, &(first, last)) in sections.iter().enumerate() {
        let name = format!("part{:0width$}", k + 1);
        let (text, b, t) = ctx.piece(&name, first, last, source)?;
        blocks += b;
        tempos += t;
        pieces.push((name, text));
    }
    let song = pieces
        .iter()
        .map(|(n, _)| n.as_str())
        .collect::<Vec<_>>()
        .join(" > ");
    let graces = ctx
        .lanes
        .get(&GRACE_LANE)
        .map_or(0, |g| g.iter().map(|e| e.notes.len()).sum());
    let trills = model.notes.iter().filter(|n| n.trill.is_some()).count();
    Ok(Written {
        pieces,
        song: song + "\n",
        expected: ctx.expected(),
        blocks,
        tempos,
        graces,
        trills,
    })
}

impl Ctx<'_> {
    /// No note sounds across `t`, and no tie leads over it.
    fn free(&self, t: Q) -> bool {
        self.lanes
            .values()
            .flatten()
            .all(|e| !(e.start < t && t < e.end()) && !(e.tie && e.end() == t))
    }

    /// Pieces as (first, last) measure indexes: double bars, rehearsal marks, and
    /// repeat jumps start one; long ones are cut every [`CHUNK`] bars.
    fn sections(&self) -> Vec<(usize, usize)> {
        let ms = &self.model.measures;
        let mut cuts = vec![0];
        for (i, m) in ms.iter().enumerate().skip(1) {
            if m.break_before && self.free(m.start) {
                cuts.push(i);
            }
        }
        cuts.push(ms.len());
        let mut out = Vec::new();
        for w in cuts.windows(2) {
            let (mut a, b) = (w[0], w[1]);
            while b - a > MAX_PIECE {
                let cut = (a + CHUNK..b)
                    .find(|&i| self.free(ms[i].start))
                    .unwrap_or(b);
                if cut >= b {
                    break;
                }
                out.push((a, cut - 1));
                a = cut;
            }
            out.push((a, b - 1));
        }
        out
    }

    fn tempo_at(&self, t: Q) -> f64 {
        let ts = &self.model.tempos;
        let mut bpm = ts.first().map_or(120.0, |x| x.1);
        for &(at, b) in ts {
            if at <= t {
                bpm = b;
            }
        }
        bpm
    }

    /// Harmonic mean over `a..b`, so the block lasts as long as with the rubato.
    fn mean_tempo(&self, a: Q, b: Q) -> u32 {
        let mut pts = vec![a];
        pts.extend(
            self.model
                .tempos
                .iter()
                .map(|x| x.0)
                .filter(|&t| a < t && t < b),
        );
        pts.push(b);
        let secs: f64 = pts
            .windows(2)
            .map(|w| (w[1] - w[0]).to_f64() / self.tempo_at(w[0]).max(1.0))
            .sum();
        ((b - a).to_f64() / secs).round().clamp(20.0, 400.0) as u32
    }

    fn level(&self, staff: StaffId, t: Q) -> usize {
        let marks = &self.model.dynamics;
        let mut lvl = DEFAULT_LEVEL;
        for &(at, s, l) in marks {
            if s == staff && at <= t {
                lvl = l;
            }
        }
        let mut open: Option<(Q, Wedge)> = None;
        for &(at, s, w) in &self.model.wedges {
            if s != staff {
                continue;
            }
            match w {
                Wedge::Cresc | Wedge::Dim => open = Some((at, w)),
                Wedge::Stop => {
                    if let Some((from, kind)) = open.take()
                        && from <= t
                        && t < at
                    {
                        let bar = self.model.measures.first().map_or(Q::int(4), |m| m.len);
                        let target = marks
                            .iter()
                            .find(|&&(x, s2, _)| s2 == staff && at <= x && x <= at + bar)
                            .map_or_else(
                                || {
                                    if kind == Wedge::Cresc {
                                        lvl as i64 + 1
                                    } else {
                                        lvl as i64 - 1
                                    }
                                },
                                |m| m.2 as i64,
                            )
                            .clamp(0, 7);
                        let f = (t - from) / (at - from);
                        let v = Q::int(lvl as i64) + Q::int(target - lvl as i64) * f;
                        return (v.to_f64().round() as i64).clamp(0, 7) as usize;
                    }
                }
            }
        }
        lvl
    }

    fn measure_at(&self, t: Q) -> usize {
        self.model
            .measures
            .iter()
            .rposition(|m| m.start <= t)
            .unwrap_or(0)
    }

    /// Time signature to write for a measure (its real length if irregular).
    fn signature(&self, i: usize) -> (u32, u32) {
        let m = &self.model.measures[i];
        let (beats, unit) = m.sig;
        if m.len == Q::new(4 * i64::from(beats), i64::from(unit)) {
            return m.sig;
        }
        for u in [unit, 8, 16, 4, 2, 32] {
            let b = m.len * Q::int(i64::from(u)) / Q::int(4);
            if b.is_integer() && (1..=32).contains(&b.numer()) {
                return (b.numer() as u32, u);
            }
        }
        m.sig
    }

    fn down_at(&self, t: Q) -> bool {
        self.pedals
            .iter()
            .rfind(|p| p.0 < t)
            .is_some_and(|p| p.1 != "ped off")
    }

    fn label(&self, a: Q, b: Q) -> String {
        let i = self.measure_at(a);
        let ms = &self.model.measures;
        let first = if ms[i].start == a {
            ms[i].number.clone()
        } else {
            format!("{}.5", ms[i].number)
        };
        let j = self.measure_at(b - Q::new(1, 1_000_000));
        if j == i {
            first
        } else {
            format!("{first}-{}", ms[j].number)
        }
    }

    /// One piece: (text, blocks, tempo marks).
    fn piece(
        &self,
        name: &str,
        first: usize,
        last: usize,
        source: &str,
    ) -> Result<(String, usize, usize), ImportError> {
        let ms = &self.model.measures;
        let lo = ms[first].start;
        let hi = ms[last].start + ms[last].len;
        let mut points = vec![lo];
        for m in &ms[first..=last] {
            let (mid, end) = (m.start + m.len / Q::int(2), m.start + m.len);
            if self.free(mid)
                && self
                    .mean_tempo(m.start, mid)
                    .abs_diff(self.mean_tempo(mid, end))
                    >= 3
            {
                points.push(mid);
            }
            if end == hi || self.free(end) {
                points.push(end);
            }
        }
        let title = self.model.title.as_deref().unwrap_or("untitled");
        let mut lines = vec![
            format!(
                "// {title}: {name}, bars {}-{}",
                ms[first].number, ms[last].number
            ),
            format!("// Imported from {source}."),
            "// One voice per voice of the score, plus pedal and grace-note voices.".to_string(),
            String::new(),
        ];
        let (mut last_tempo, mut last_sig, mut tempos) = (None, None, 0);
        for w in points.windows(2) {
            let (a, b) = (w[0], w[1]);
            let mut head = String::new();
            if self.starts.contains(&a) {
                let sig = self.signature(self.measure_at(a));
                if last_sig != Some(sig) {
                    head += &format!("time {}/{} ", sig.0, sig.1);
                    last_sig = Some(sig);
                }
            }
            let tempo = self.mean_tempo(a, b);
            if last_tempo != Some(tempo) {
                head += &format!("tempo {tempo} ");
                last_tempo = Some(tempo);
                tempos += 1;
            }
            let mut texts = Vec::new();
            for evs in self.lanes.values() {
                let evs: Vec<&Ev> = evs.iter().filter(|e| a <= e.start && e.start < b).collect();
                if !evs.is_empty() {
                    texts.push(self.voice(&evs, a, b)?);
                }
            }
            let mut ped: Vec<(Q, &str)> = self
                .pedals
                .iter()
                .copied()
                .filter(|p| a <= p.0 && p.0 < b)
                .collect();
            if a == lo {
                // A piece starts with the pedal up; one held from before is pressed again.
                let held = self.down_at(lo) && !ped.iter().any(|p| p.0 == lo);
                ped.retain(|p| !(p.0 == lo && p.1 == "ped off"));
                for p in &mut ped {
                    if p.0 == lo {
                        p.1 = "ped on";
                    }
                }
                if held {
                    ped.insert(0, (lo, "ped on"));
                }
            }
            if !ped.is_empty() {
                texts.push(self.pedal(&ped, a, b)?);
            }
            if texts.is_empty() {
                let mut items = Vec::new();
                self.rests(&mut items, a, b)?;
                texts.push(emit(&items));
            }
            let opens = if self.starts.contains(&a) { "| " } else { "" };
            let closes = if self.starts.contains(&b) { " |" } else { "" };
            let label = self.label(a, b);
            if texts.len() == 1 {
                lines.push(format!("{opens}{head}{}{closes}  // {label}", texts[0]));
            } else {
                lines.push(format!("{opens}{head}voices{{ {} }}  // {label}", texts[0]));
                for t in &texts[1..] {
                    lines.push(format!("        {{ {t} }}"));
                }
                if let Some(l) = lines.last_mut() {
                    l.push_str(closes);
                }
            }
        }
        if self.down_at(hi) {
            lines.push("ped off".into());
        }
        Ok((lines.join("\n") + "\n", points.len() - 1, tempos))
    }

    /// Rests from `t` to `until`, with bar lines at the bars inside.
    fn rests(&self, items: &mut Vec<Item>, mut t: Q, until: Q) -> Result<(), ImportError> {
        while t < until {
            let next = self
                .starts
                .iter()
                .copied()
                .find(|&s| s > t)
                .map_or(until, |s| s.min(until));
            let lens = rest_lengths(next - t).ok_or_else(|| {
                ImportError::Format(format!(
                    "cannot write a rest of {} quarters",
                    (next - t).to_f64()
                ))
            })?;
            for (w, n) in lens {
                items.push(Item::Len(w, n, "r".into(), String::new()));
            }
            t = next;
            if t < until && self.starts.contains(&t) {
                items.push(Item::Bar);
            }
        }
        Ok(())
    }

    fn voice(&self, evs: &[&Ev], a: Q, b: Q) -> Result<String, ImportError> {
        let mut items = Vec::new();
        let mut cur = a;
        let mut lvl = None;
        for e in evs {
            self.rests(&mut items, cur, e.start)?;
            if e.start > a
                && self.starts.contains(&e.start)
                && !matches!(items.last(), Some(Item::Bar))
            {
                items.push(Item::Bar);
            }
            let want = self.level(e.staff, e.start);
            let mut text = String::new();
            if lvl != Some(want) {
                text = format!("{} ", LEVELS[want]);
                lvl = Some(want);
            }
            if e.notes.len() == 1 {
                text += &e.notes[0].1;
            } else {
                let names: Vec<&str> = e.notes.iter().map(|n| n.1.as_str()).collect();
                text += &format!("{{{}}}", names.join(" "));
            }
            let (w, n) = note_length(e.len).ok_or_else(|| {
                ImportError::Format(format!(
                    "bar {}: cannot write a note of {} quarters",
                    self.model.measures[self.measure_at(e.start)].number,
                    e.len.to_f64()
                ))
            })?;
            items.push(Item::Len(
                w,
                n,
                text,
                if e.tie { "~".into() } else { String::new() },
            ));
            cur = e.end();
        }
        self.rests(&mut items, cur, b)?;
        Ok(emit(&items))
    }

    fn pedal(&self, ped: &[(Q, &str)], a: Q, b: Q) -> Result<String, ImportError> {
        let mut items = Vec::new();
        let mut cur = a;
        for &(at, text) in ped {
            self.rests(&mut items, cur, at)?;
            if at > a && self.starts.contains(&at) && !matches!(items.last(), Some(Item::Bar)) {
                items.push(Item::Bar);
            }
            items.push(Item::Raw(text.to_string()));
            cur = at;
        }
        self.rests(&mut items, cur, b)?;
        Ok(emit(&items))
    }

    /// The notes the written pieces play, ties merged per voice.
    fn expected(&self) -> Vec<(Q, i32, Q)> {
        let mut out = Vec::new();
        for evs in self.lanes.values() {
            let mut open: BTreeMap<i32, usize> = BTreeMap::new();
            for e in evs {
                for &(midi, _) in &e.notes {
                    if let Some(&k) = open.get(&midi)
                        && out
                            .get(k)
                            .is_some_and(|o: &(Q, i32, Q)| o.0 + o.2 == e.start)
                    {
                        let o = &mut out[k];
                        o.2 = e.end() - o.0;
                        if !e.tie {
                            open.remove(&midi);
                        }
                        continue;
                    }
                    open.remove(&midi);
                    out.push((e.start, midi, e.len));
                    if e.tie {
                        open.insert(midi, out.len() - 1);
                    }
                }
            }
        }
        out
    }
}

enum Item {
    /// (written length, tuplet, text before the length, text after it)
    Len(Q, Option<u32>, String, String),
    Bar,
    Raw(String),
}

/// Joins items: tuplets grouped, lengths written only when they change.
fn emit(items: &[Item]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut group: Vec<String> = Vec::new();
    let mut group_n = None;
    let mut last: Option<String> = None;
    let flush = |out: &mut Vec<String>, group: &mut Vec<String>, n: &mut Option<u32>| {
        if let Some(k) = n.take() {
            out.push(format!("tuplet({k}){{ {} }}", group.join(" ")));
            group.clear();
        }
    };
    for item in items {
        match item {
            Item::Len(w, n, text, suffix) => {
                let tok = length_token(*w).unwrap_or_default();
                let body = if last.as_ref() == Some(&tok) {
                    format!("{text}{suffix}")
                } else {
                    format!("{text}{tok}{suffix}")
                };
                last = Some(tok);
                match n {
                    None => {
                        flush(&mut out, &mut group, &mut group_n);
                        out.push(body);
                    }
                    Some(k) => {
                        if group_n != Some(*k) {
                            flush(&mut out, &mut group, &mut group_n);
                            group_n = Some(*k);
                        }
                        group.push(body);
                    }
                }
            }
            Item::Bar => {
                flush(&mut out, &mut group, &mut group_n);
                out.push("|".into());
            }
            Item::Raw(t) => {
                flush(&mut out, &mut group, &mut group_n);
                out.push(t.clone());
            }
        }
    }
    flush(&mut out, &mut group, &mut group_n);
    out.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths() {
        assert_eq!(length_token(Q::new(3, 2)).as_deref(), Some("/4."));
        assert_eq!(length_token(Q::new(1, 16)).as_deref(), Some("/64"));
        assert_eq!(length_token(Q::new(1, 3)), None);
        // a triplet sixteenth: /16 in tuplet(3)
        assert_eq!(note_length(Q::new(1, 6)), Some((Q::new(1, 4), Some(3))));
        // 5 thirty-seconds in the time of 6: dotted 32nds in tuplet(5)
        assert_eq!(note_length(Q::new(3, 20)), Some((Q::new(3, 16), Some(5))));
    }

    #[test]
    fn rests_mix_plain_and_tuplets() {
        let r = rest_lengths(Q::new(1, 3) + Q::new(1, 4)).unwrap();
        let total = r
            .iter()
            .fold(Q::ZERO, |s, (w, n)| s + *w * n.map_or(Q::int(1), scale));
        assert_eq!(total, Q::new(7, 12));
        assert!(r.iter().any(|x| x.1 == Some(3)));
    }
}
