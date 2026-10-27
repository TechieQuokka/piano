//! [`Model`] → pieces of piano-language code, one `voices` block per bar.
//!
//! - Blocks are bars, split at the half bar when the tempo changes there, and
//!   merged across bar lines that a tie crosses (ties do not leave a block).
//! - Each MusicXML voice is a voice; notes that overlap inside one voice go to an
//!   extra voice. Pedals and grace notes get voices of their own.
//! - Rubato becomes one `tempo` per block: the harmonic mean, so time is kept.
//! - Dynamics follow the marks of each staff; wedges become `cresc` / `dim` to
//!   the level they end on (a swell goes back to where it started).

use std::collections::BTreeMap;

use crate::ImportError;
use crate::musicxml::{Marks, Model, Pedal, StaffId, Sudden, VoiceId, Wedge};
use crate::rational::Q;

const LEVELS: [&str; 8] = ["ppp", "pp", "p", "mp", "mf", "f", "ff", "fff"];
/// Level before the first mark (the language's default, mf).
const DEFAULT_LEVEL: usize = 4;
/// Tuplets the language plays exactly (`tuplet(n)`, in the time of a power of two).
const TUPLETS: [u32; 5] = [3, 5, 7, 9, 11];
/// Written lengths, `/1` .. `/128`.
const DENOMS: [i64; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
/// Pieces longer than this many bars are cut into chunks of [`CHUNK`].
const MAX_PIECE: usize = 32;
const CHUNK: usize = 16;

fn sixty_fourth() -> Q {
    Q::new(1, 16)
}

/// Length of each of `count` grace notes before a note of `len`, as the language
/// plays them: a 32nd, halved (at most 4 times) until they take half of the note.
fn grace_len(count: usize, len: Q) -> Q {
    let mut each = Q::new(1, 8);
    for _ in 0..4 {
        if each * Q::int(2 * count as i64) <= len {
            break;
        }
        each = each / Q::int(2);
    }
    each
}

fn trill_len() -> Q {
    Q::new(1, 8) // a 32nd
}

/// `/n`, `/n.`, `/n..` for a written length in quarters.
fn length_token(len: Q) -> Option<String> {
    for d in DENOMS {
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

/// Undotted lengths adding up to `len` (a multiple of a 128th).
fn binary(mut len: Q) -> Vec<Q> {
    let mut out = Vec::new();
    for d in DENOMS {
        let t = Q::new(4, d);
        while len >= t {
            out.push(t);
            len = len - t;
        }
    }
    out
}

/// The shortest written length, a 128th.
fn unit_len() -> Q {
    Q::new(1, 32)
}

fn is_128ths(len: Q) -> bool {
    (len / unit_len()).is_integer()
}

/// Rests filling `len` quarters: (written length, tuplet).
fn rest_lengths(len: Q) -> Option<Vec<(Q, Option<u32>)>> {
    if length_token(len).is_some() {
        return Some(vec![(len, None)]);
    }
    if is_128ths(len) {
        return Some(binary(len).into_iter().map(|t| (t, None)).collect());
    }
    let tuplet = |r: Q| {
        TUPLETS.iter().find_map(|&n| {
            let w = r / scale(n);
            is_128ths(w).then(|| {
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
    let units = (len / unit_len()).floor();
    for k in (1..=units).rev() {
        let x = unit_len() * Q::int(k);
        if let Some(t) = tuplet(len - x) {
            let mut out: Vec<_> = binary(x).into_iter().map(|t| (t, None)).collect();
            out.extend(t);
            return Some(out);
        }
    }
    None
}

/// (MIDI, name) of the notes struck together, sorted by pitch.
type Chord = Vec<(i32, String)>;

#[derive(Debug, Clone)]
struct Ev {
    start: Q,
    len: Q,
    /// (MIDI, name), sorted by pitch.
    notes: Vec<(i32, String)>,
    /// MIDI numbers of the notes tied to the next note of their key.
    tied: Vec<i32>,
    staff: StaffId,
    marks: Marks,
    /// `sf`, `fp`, ... written before it.
    sudden: Option<Sudden>,
    /// Grace notes before it, one entry (a note or chord) each.
    graces: Vec<Chord>,
}

impl Ev {
    fn end(&self) -> Q {
        self.start + self.len
    }
}

/// A piece of a staff's dynamics curve: `level` from `at`, moving in a straight
/// line to the ramp's level by its end (a cresc or dim).
#[derive(Debug, Clone, Copy)]
struct Seg {
    at: Q,
    level: usize,
    ramp: Option<(Q, usize)>,
}

impl Seg {
    fn level_at(&self, t: Q) -> usize {
        match self.ramp {
            Some((end, target)) if t < end => {
                let f = ((t - self.at) / (end - self.at)).to_f64();
                (self.level as f64 + (target as f64 - self.level as f64) * f).round() as usize
            }
            Some((_, target)) => target,
            None => self.level,
        }
    }
}

/// The dynamics curve of each staff. A wedge starts from the level reached so
/// far (the top of a swell, not the last mark) and ends on the mark written at
/// its end, or one level further.
fn curves(model: &Model) -> BTreeMap<StaffId, Vec<Seg>> {
    // At one time: a wedge ends, then a mark, then a wedge starts.
    #[derive(Clone, Copy)]
    enum Mark {
        Stop,
        Level(usize),
        Start(Wedge),
    }
    let mut staves: BTreeMap<StaffId, Vec<(Q, u8, Mark)>> = BTreeMap::new();
    for &(at, s, l) in &model.dynamics {
        staves.entry(s).or_default().push((at, 1, Mark::Level(l)));
    }
    for &(at, s, w) in &model.wedges {
        let m = match w {
            Wedge::Stop => (at, 0, Mark::Stop),
            _ => (at, 2, Mark::Start(w)),
        };
        staves.entry(s).or_default().push(m);
    }
    // A wedge's mark may come up to the end of the bar after the one it stops in.
    let reach = |t: Q| {
        let i = model
            .measures
            .iter()
            .rposition(|m| m.start <= t)
            .unwrap_or(0);
        model
            .measures
            .get(i + 1)
            .or(model.measures.get(i))
            .map_or(t + Q::int(4), |m| m.start + m.len)
    };
    let mut out = BTreeMap::new();
    for (staff, mut marks) in staves {
        marks.sort_by_key(|m| (m.0, m.1));
        let mut segs = Vec::new();
        let mut cur = DEFAULT_LEVEL;
        let mut open: Option<(Q, Wedge)> = None;
        for (i, &(at, _, mark)) in marks.iter().enumerate() {
            // A wedge ends at its stop, a mark, or the next wedge.
            if let Some((from, kind)) = open.take() {
                let written = match mark {
                    Mark::Level(l) => Some(l),
                    // The mark just after the stop, if no wedge comes first.
                    Mark::Stop => marks[i + 1..]
                        .iter()
                        .take_while(|m| m.0 < reach(at) && !matches!(m.2, Mark::Start(_)))
                        .find_map(|m| match m.2 {
                            Mark::Level(l) => Some(l),
                            _ => None,
                        }),
                    Mark::Start(_) => None,
                };
                let up = kind == Wedge::Cresc;
                let target = match written {
                    Some(l) if (l > cur) == up && l != cur => l,
                    _ if up => (cur + 1).min(7),
                    _ => cur.saturating_sub(1),
                };
                if target != cur && at > from {
                    segs.push(Seg {
                        at: from,
                        level: cur,
                        ramp: Some((at, target)),
                    });
                    cur = target;
                    segs.push(Seg {
                        at,
                        level: cur,
                        ramp: None,
                    });
                }
            }
            match mark {
                Mark::Level(l) => {
                    cur = l;
                    segs.push(Seg {
                        at,
                        level: l,
                        ramp: None,
                    });
                }
                Mark::Start(w) => open = Some((at, w)),
                Mark::Stop => {}
            }
        }
        out.insert(staff, segs);
    }
    out
}

/// A note tied from a chord in lane `o` went on in `lane` (its chord there is
/// shaped differently): it leaves the chord and is struck in `lane` too, if
/// that lane is free then, since a tie holds every note of a chord.
fn move_tied(
    out: &mut BTreeMap<Lane, Vec<Ev>>,
    (part, voice): VoiceId,
    (o, i): (u32, usize),
    lane: u32,
    note: &(i32, String),
) {
    let origin = &out[&(part, voice, o)][i];
    if origin.notes.len() < 2 {
        return;
    }
    let (start, len, staff, marks) = (origin.start, origin.len, origin.staff, origin.marks);
    let list = &out[&(part, voice, lane)];
    let at = list.len() - 1;
    if at > 0 && list[at - 1].end() > start {
        return;
    }
    if let Some(origin) = out.get_mut(&(part, voice, o)).and_then(|l| l.get_mut(i)) {
        origin.notes.retain(|n| n != note);
        origin.tied.retain(|m| *m != note.0);
    }
    if let Some(list) = out.get_mut(&(part, voice, lane)) {
        list.insert(
            at,
            Ev {
                start,
                len,
                notes: vec![note.clone()],
                tied: vec![note.0],
                staff,
                marks,
                sudden: None,
                graces: Vec::new(),
            },
        );
    }
}

/// A bar shorter than its time signature (an upbeat, or the bar that ends it).
fn short(m: &crate::musicxml::Measure) -> bool {
    m.len < full(m.sig)
}

/// Length of a bar of `sig`, in quarters.
fn full((beats, unit): (u32, u32)) -> Q {
    Q::new(4 * i64::from(beats), i64::from(unit))
}

/// A voice of the output: a MusicXML voice, or an extra lane of it.
type Lane = (usize, u32, u32);

/// Spreads the notes of each voice over lanes so that nothing overlaps.
fn lanes(model: &Model) -> BTreeMap<Lane, Vec<Ev>> {
    let mut out: BTreeMap<Lane, Vec<Ev>> = BTreeMap::new();
    // Notes that continue a tie take their lane first, before notes struck with
    // them can take it (a held note under moving ones).
    let tied: std::collections::HashSet<(VoiceId, i32, Q)> = model
        .notes
        .iter()
        .filter(|n| n.tie)
        .map(|n| (n.voice, n.midi, n.start + n.len))
        .collect();
    // A tie with no note after it in its voice (in the score) is dropped.
    let starts: std::collections::HashSet<(VoiceId, i32, Q)> = model
        .notes
        .iter()
        .map(|n| (n.voice, n.midi, n.start))
        .collect();
    let mut notes: Vec<_> = model.notes.iter().collect();
    notes.sort_by_key(|n| (n.start, !tied.contains(&(n.voice, n.midi, n.start))));
    for n in notes {
        let tie = n.tie && starts.contains(&(n.voice, n.midi, n.start + n.len));
        let (part, voice): VoiceId = n.voice;
        let expanded = n.trill.as_ref().and_then(|(upper, umidi)| {
            let step = [trill_len(), sixty_fourth()]
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
                        tied: Vec::new(),
                        staff: n.staff,
                        marks: Marks::default(),
                        sudden: None,
                        graces: Vec::new(),
                    })
                    .collect::<Vec<_>>(),
            )
        });
        let expanded = expanded.or_else(|| {
            let orn = n.ornament.as_ref()?;
            let main = (n.midi, n.name.clone());
            let flip = |(name, midi): &(String, i32)| (*midi, name.clone());
            let lead = match orn {
                crate::musicxml::Ornament::Mordent(nb) => vec![main.clone(), flip(nb)],
                crate::musicxml::Ornament::Turn { first, second } => {
                    vec![flip(first), main.clone(), flip(second)]
                }
            };
            // Quick notes, then the note for the rest of its time.
            let k = Q::int(lead.len() as i64);
            let step = [trill_len(), sixty_fourth()]
                .into_iter()
                .find(|s| n.len >= *s * (k + Q::int(1)))?;
            let mut evs: Vec<Ev> = lead
                .into_iter()
                .enumerate()
                .map(|(i, note)| Ev {
                    start: n.start + step * Q::int(i as i64),
                    len: step,
                    notes: vec![note],
                    tied: Vec::new(),
                    staff: n.staff,
                    marks: Marks::default(),
                    sudden: None,
                    graces: Vec::new(),
                })
                .collect();
            // The rest of the note, tied in parts that can be written.
            let parts = rest_lengths(n.len - step * k)?;
            let mut at = n.start + step * k;
            for (i, (w, t)) in parts.iter().enumerate() {
                let len = t.map_or(*w, |t| *w * scale(t));
                let last = i + 1 == parts.len();
                evs.push(Ev {
                    start: at,
                    len,
                    notes: vec![main.clone()],
                    tied: if !last || tie {
                        vec![n.midi]
                    } else {
                        Vec::new()
                    },
                    staff: n.staff,
                    marks: if last { n.marks } else { Marks::default() },
                    sudden: None,
                    graces: Vec::new(),
                });
                at = at + len;
            }
            Some(evs)
        });
        let evs = expanded.unwrap_or_else(|| {
            vec![Ev {
                start: n.start,
                len: n.len,
                notes: vec![(n.midi, n.name.clone())],
                tied: if tie { vec![n.midi] } else { Vec::new() },
                staff: n.staff,
                marks: n.marks,
                sudden: None,
                graces: Vec::new(),
            }]
        });
        let single = evs.len() == 1;
        for ev in evs {
            // A tied note goes on in the lane of the note it is tied from (the
            // last event there, or just before notes tied on with it).
            let from = (0..)
                .map_while(|l| out.get(&(part, voice, l)).map(|list| (l, list)))
                .find_map(|(l, list)| {
                    let k = list.len().checked_sub(1)?;
                    let k = if list[k].start == ev.start {
                        k.checked_sub(1)?
                    } else {
                        k
                    };
                    let e = &list[k];
                    (e.end() == ev.start
                        && e.tied.contains(&ev.notes[0].0)
                        && e.notes.contains(&ev.notes[0]))
                    .then_some((l, k))
                });
            let mut lane = from.map_or(0, |f| f.0);
            let note = ev.notes[0].clone();
            loop {
                let key = (part, voice, lane);
                let list = out.entry(key).or_default();
                match list.last_mut() {
                    Some(last) if single && last.start == ev.start && last.len == ev.len => {
                        // One key once: E#4 and F4 in a chord are the same key.
                        if !last.notes.iter().any(|n| n.0 == ev.notes[0].0) {
                            last.notes.push(ev.notes[0].clone());
                            last.notes.sort();
                        }
                        last.marks = last.marks.chord(ev.marks);
                        // Some notes of a chord may be tied and others not (`{C~ E}`).
                        for m in ev.tied {
                            if !last.tied.contains(&m) {
                                last.tied.push(m);
                            }
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
            if let Some((o, i)) = from
                && o != lane
            {
                move_tied(&mut out, (part, voice), (o, i), lane, &note);
            }
        }
    }
    attach_graces(model, &mut out);
    place_suddens(model, &mut out);
    out.retain(|_, v| !v.is_empty());
    out
}

/// Puts each group of grace notes before its note: the first lane of the voice
/// that strikes a note then. A note tied on is not struck, so its graces are
/// dropped.
fn attach_graces(model: &Model, lanes: &mut BTreeMap<Lane, Vec<Ev>>) {
    let mut groups: BTreeMap<(VoiceId, Q), Vec<Chord>> = BTreeMap::new();
    for g in &model.graces {
        let group = groups.entry((g.voice, g.principal)).or_default();
        if group.len() <= g.index {
            group.resize(g.index + 1, Vec::new());
        }
        let entry = &mut group[g.index];
        if !entry.iter().any(|n| n.0 == g.midi) {
            entry.push((g.midi, g.name.clone()));
            entry.sort();
        }
    }
    for (((part, voice), at), group) in groups {
        let group: Vec<_> = group.into_iter().filter(|g| !g.is_empty()).collect();
        for (&(p, v, _), list) in lanes.iter_mut() {
            if (p, v) != (part, voice) {
                continue;
            }
            let Some(k) = list.iter().position(|e| e.start == at) else {
                continue;
            };
            let held = k > 0 && {
                let prev = &list[k - 1];
                prev.end() == at && list[k].notes.iter().any(|n| prev.tied.contains(&n.0))
            };
            if !held {
                list[k].graces = group;
                break;
            }
        }
    }
}

/// Gives each `sf`, `fp`, ... to the notes of its staff struck then, or the next
/// ones struck in the same bar.
fn place_suddens(model: &Model, lanes: &mut BTreeMap<Lane, Vec<Ev>>) {
    let bar = |t: Q| model.measures.iter().rposition(|m| m.start <= t);
    for &(at, staff, kind) in &model.suddens {
        let next = lanes
            .values()
            .flatten()
            .filter(|e| e.staff == staff && e.start >= at && bar(e.start) == bar(at))
            .map(|e| e.start)
            .min();
        let Some(t) = next else { continue };
        for e in lanes.values_mut().flatten() {
            if e.staff == staff && e.start == t && e.sudden.is_none() {
                e.sudden = Some(kind);
            }
        }
    }
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
    pub ornaments: usize,
}

struct Ctx<'a> {
    model: &'a Model,
    lanes: BTreeMap<Lane, Vec<Ev>>,
    curves: BTreeMap<StaffId, Vec<Seg>>,
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
        curves: curves(model),
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
        .values()
        .flatten()
        .flat_map(|e| &e.graces)
        .map(Vec::len)
        .sum();
    let trills = model.notes.iter().filter(|n| n.trill.is_some()).count();
    let ornaments = model
        .notes
        .iter()
        .filter(|n| n.trill.is_none() && n.ornament.is_some())
        .count();
    Ok(Written {
        pieces,
        song: song + "\n",
        expected: ctx.expected(),
        blocks,
        tempos,
        graces,
        trills,
        ornaments,
    })
}

impl Ctx<'_> {
    /// No note sounds across `t`, and no tie leads over it.
    fn free(&self, t: Q) -> bool {
        self.lanes
            .values()
            .flatten()
            .all(|e| !(e.start < t && t < e.end()) && !(!e.tied.is_empty() && e.end() == t))
    }

    /// A block may end at `t`: no note sounds across it, and every tie over it can be
    /// followed into the next block, where it goes to the first voice that starts
    /// the key (so one voice ties each key out, and only one starts it at `t`).
    fn splittable(&self, t: Q) -> bool {
        let evs = || self.lanes.values().flatten();
        if evs().any(|e| e.start < t && t < e.end()) {
            return false;
        }
        let mut out: Vec<i32> = Vec::new();
        for e in evs().filter(|e| e.end() == t) {
            for &m in &e.tied {
                if out.contains(&m) {
                    return false;
                }
                out.push(m);
            }
        }
        out.iter().all(|m| {
            evs()
                .filter(|e| e.start == t && e.notes.iter().any(|n| n.0 == *m))
                .count()
                <= 1
        })
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

    /// The piece of the dynamics curve of `staff` that holds at `t`.
    fn seg(&self, staff: StaffId, t: Q) -> Seg {
        self.curves
            .get(&staff)
            .and_then(|c| c.iter().rfind(|s| s.at <= t))
            .copied()
            .unwrap_or(Seg {
                at: Q::ZERO,
                level: DEFAULT_LEVEL,
                ramp: None,
            })
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

    /// A bar no time signature can write (a cadenza): it goes in `free{ }`.
    fn unmetered(&self, i: usize) -> bool {
        let m = &self.model.measures[i];
        m.len != Q::new(4 * i64::from(m.sig.0), i64::from(m.sig.1)) && self.signature(i) == m.sig
    }

    /// Items of a voice starting at `a`: inside `free{` if its bar is unmetered.
    fn begin(&self, a: Q) -> Vec<Item> {
        if self.unmetered(self.measure_at(a)) {
            vec![Item::Free(true)]
        } else {
            Vec::new()
        }
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
            if self.splittable(mid)
                && self
                    .mean_tempo(m.start, mid)
                    .abs_diff(self.mean_tempo(mid, end))
                    >= 3
            {
                points.push(mid);
            }
            if end == hi || self.splittable(end) {
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
            "// One voice per voice of the score, plus a pedal voice.".to_string(),
            String::new(),
        ];
        let (mut last_tempo, mut last_sig, mut tempos) = (None, None, 0);
        // The signature to write for measure `i`. A short last bar keeps the one
        // before it (the language lets the last bar be short), if that one is long
        // enough for it.
        let sig_of = |i: usize, before: Option<(u32, u32)>| {
            let m = &ms[i];
            match before {
                Some(s) if i == last && i != first && short(m) && m.len <= full(s) => s,
                _ => self.signature(i),
            }
        };
        for w in points.windows(2) {
            let (a, b) = (w[0], w[1]);
            let mut head = String::new();
            // A short first bar (an upbeat) is written under the bar's own
            // signature without a bar line before it, as the language allows.
            let upbeat = a == lo && short(&ms[first]);
            if self.starts.contains(&a) {
                let sig = if upbeat {
                    ms[first].sig
                } else {
                    sig_of(self.measure_at(a), last_sig)
                };
                if last_sig != Some(sig) {
                    head += &format!("time {}/{} ", sig.0, sig.1);
                    last_sig = Some(sig);
                }
            }
            // Tempo bar by bar: a block that ties hold together may span changes.
            let bar_end = |t: Q| {
                self.starts
                    .iter()
                    .copied()
                    .find(|&s| s > t)
                    .map_or(b, |s| s.min(b))
            };
            let tempo = self.mean_tempo(a, bar_end(a));
            if last_tempo != Some(tempo) {
                head += &format!("tempo {tempo} ");
                last_tempo = Some(tempo);
                tempos += 1;
            }
            let mut texts = Vec::new();
            for lane in self.lanes.values() {
                let evs: Vec<&Ev> = lane
                    .iter()
                    .filter(|e| a <= e.start && e.start < b)
                    .collect();
                // The event after the block, for a tie into the next one.
                let next = lane.iter().find(|e| e.start >= b);
                if !evs.is_empty() {
                    texts.push(self.voice(&evs, next, a, b)?);
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
                let mut items = self.begin(a);
                self.rests(&mut items, a, b)?;
                texts.push(items);
            }
            // Changes at the bars inside the block go after the bar line of the first
            // voice that has one; they apply to every voice.
            for t in self.starts.iter().copied().filter(|&t| a < t && t < b) {
                let sig = sig_of(self.measure_at(t), last_sig);
                let tempo = self.mean_tempo(t, bar_end(t));
                let mut marks = Vec::new();
                if last_sig != Some(sig) {
                    marks.push(format!("time {}/{}", sig.0, sig.1));
                }
                if last_tempo != Some(tempo) {
                    marks.push(format!("tempo {tempo}"));
                }
                if marks.is_empty() {
                    continue;
                }
                let place = texts.iter_mut().find_map(|items| {
                    let i = items
                        .iter()
                        .position(|x| matches!(x, Item::Bar(at) if *at == t))?;
                    Some((items, i))
                });
                if let Some((items, i)) = place {
                    items.insert(i + 1, Item::Raw(marks.join(" ")));
                    if last_tempo != Some(tempo) {
                        tempos += 1;
                    }
                    (last_sig, last_tempo) = (Some(sig), Some(tempo));
                }
            }
            let texts: Vec<String> = texts.iter_mut().map(emit).collect();
            let opens = if self.starts.contains(&a) && !upbeat {
                "| "
            } else {
                ""
            };
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
        if t < until {
            self.bar_line(items, t);
        }
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
            let mut at = t;
            for (w, n) in lens {
                // A fermata over a rest goes on the part of it that starts then.
                let held = self.model.rest_holds.iter().any(|h| h.0 == at);
                items.push(Item::Len(
                    w,
                    n,
                    "r".into(),
                    if held { "@" } else { "" }.into(),
                ));
                at = at + n.map_or(w, |k| w * scale(k));
            }
            t = next;
            if t < until {
                self.bar_line(items, t);
            }
        }
        Ok(())
    }

    /// A bar line if a bar starts at `t`, once, and not at the start of the voice
    /// (nothing that takes time yet); an unmetered bar goes in `free{ }`.
    fn bar_line(&self, items: &mut Vec<Item>, t: Q) {
        if self.starts.contains(&t)
            && items.iter().any(|i| matches!(i, Item::Len(..)))
            && !matches!(items.last(), Some(Item::Bar(_) | Item::Free(true)))
        {
            close_free(items);
            items.push(Item::Bar(t));
            let i = self.measure_at(t);
            if self.model.measures[i].start == t && self.unmetered(i) {
                items.push(Item::Free(true));
            }
        }
    }

    fn voice(&self, evs: &[&Ev], next: Option<&Ev>, a: Q, b: Q) -> Result<Vec<Item>, ImportError> {
        let mut items = self.begin(a);
        let mut cur = a;
        let mut lvl = None;
        // An open cresc/dim: (start of its wedge, level it ends on).
        let mut open: Option<(Q, usize)> = None;
        for (k, e) in evs.iter().enumerate() {
            self.rests(&mut items, cur, e.start)?;
            if e.start > a {
                self.bar_line(&mut items, e.start);
            }
            let seg = self.seg(e.staff, e.start);
            let ramp = seg.ramp.filter(|r| e.start < r.0);
            let mut text = String::new();
            if let Some((from, target)) = open
                && (ramp.is_none() || seg.at != from)
            {
                text += &format!("{} ", LEVELS[target]);
                lvl = Some(target);
                open = None;
            }
            if open.is_none() {
                let now = seg.level_at(e.start);
                if lvl != Some(now) {
                    text += &format!("{} ", LEVELS[now]);
                    lvl = Some(now);
                }
                if let Some((end, _)) = ramp {
                    // Ends inside the block, or on the level reached at its end.
                    let target = seg.level_at(end.min(b));
                    if target != now {
                        text += if target > now { "cresc " } else { "dim " };
                        open = Some((seg.at, target));
                    }
                }
            }
            if let Some(k) = e.sudden {
                text += k.keyword();
                text.push(' ');
                // The language goes on soft after `fp`.
                if k.then_soft() {
                    lvl = Some(crate::musicxml::SOFT);
                }
            }
            if !e.graces.is_empty() {
                let g: Vec<String> = e
                    .graces
                    .iter()
                    .map(|c| match c.as_slice() {
                        [one] => one.1.clone(),
                        _ => format!(
                            "{{{}}}",
                            c.iter().map(|n| n.1.as_str()).collect::<Vec<_>>().join(" ")
                        ),
                    })
                    .collect();
                text += &format!("grace{{ {} }} ", g.join(" "));
            }
            // Tied notes that go on in the next event: all of them take one `~`
            // after the note or chord, some of them one each (`{C~ E}`).
            let after = if k + 1 == evs.len() {
                next
            } else {
                evs.get(k + 1).copied()
            };
            let held: Vec<i32> = after
                .filter(|n| n.start == e.end())
                .map(|n| {
                    e.tied
                        .iter()
                        .copied()
                        .filter(|m| n.notes.iter().any(|x| x.0 == *m))
                        .collect()
                })
                .unwrap_or_default();
            let whole = !held.is_empty() && held.len() == e.notes.len();
            if e.notes.len() == 1 {
                text += &e.notes[0].1;
            } else {
                if e.marks.arp {
                    text += "arp";
                }
                let names: Vec<String> = e
                    .notes
                    .iter()
                    .map(|n| {
                        if !whole && held.contains(&n.0) {
                            format!("{}~", n.1)
                        } else {
                            n.1.clone()
                        }
                    })
                    .collect();
                text += &format!("{{{}}}", names.join(" "));
            }
            let (w, n) = note_length(e.len).ok_or_else(|| {
                ImportError::Format(format!(
                    "bar {}: cannot write a note of {} quarters",
                    self.model.measures[self.measure_at(e.start)].number,
                    e.len.to_f64()
                ))
            })?;
            // A note tied on is held on: its staccato would be ignored.
            let mut marks = e.marks;
            if whole {
                marks.staccato = 0;
            }
            items.push(Item::Len(
                w,
                n,
                text,
                marks.text() + if whole { "~" } else { "" },
            ));
            cur = e.end();
        }
        if let Some((_, target)) = open {
            items.push(Item::Raw(LEVELS[target].to_string()));
        }
        self.rests(&mut items, cur, b)?;
        Ok(items)
    }

    fn pedal(&self, ped: &[(Q, &str)], a: Q, b: Q) -> Result<Vec<Item>, ImportError> {
        let mut items = self.begin(a);
        let mut cur = a;
        for &(at, text) in ped {
            self.rests(&mut items, cur, at)?;
            if at > a {
                self.bar_line(&mut items, at);
            }
            items.push(Item::Raw(text.to_string()));
            cur = at;
        }
        self.rests(&mut items, cur, b)?;
        Ok(items)
    }

    /// The notes the written pieces play, ties merged per voice.
    fn expected(&self) -> Vec<(Q, i32, Q)> {
        let mut out = Vec::new();
        for evs in self.lanes.values() {
            let mut open: BTreeMap<i32, usize> = BTreeMap::new();
            for e in evs {
                // Grace notes on the beat, then the note in the time left. Inside a
                // tuplet they are measured on the written length, then squeezed.
                let written = note_length(e.len).map_or(e.len, |(w, _)| w);
                let each = grace_len(e.graces.len(), written) * e.len / written;
                for (k, g) in e.graces.iter().enumerate() {
                    for &(midi, _) in g {
                        out.push((e.start + each * Q::int(k as i64), midi, each));
                    }
                }
                let lead = each * Q::int(e.graces.len() as i64);
                for &(midi, _) in &e.notes {
                    if let Some(&k) = open.get(&midi)
                        && out
                            .get(k)
                            .is_some_and(|o: &(Q, i32, Q)| o.0 + o.2 == e.start)
                    {
                        let o = &mut out[k];
                        o.2 = e.end() - o.0;
                        if !e.tied.contains(&midi) {
                            open.remove(&midi);
                        }
                        continue;
                    }
                    open.remove(&midi);
                    out.push((e.start + lead, midi, e.len - lead));
                    if e.tied.contains(&midi) {
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
    /// A bar line at a time.
    Bar(Q),
    Raw(String),
    /// `free{` (true) or its `}`.
    Free(bool),
}

/// Closes an open `free{`; one with nothing in it is dropped.
fn close_free(items: &mut Vec<Item>) {
    let open = items.iter().rev().find_map(|i| match i {
        Item::Free(o) => Some(*o),
        _ => None,
    });
    if open == Some(true) {
        if matches!(items.last(), Some(Item::Free(true))) {
            items.pop();
        } else {
            items.push(Item::Free(false));
        }
    }
}

/// Joins items: tuplets grouped, lengths written only when they change.
fn emit(items: &mut Vec<Item>) -> String {
    close_free(items);
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
            Item::Bar(_) => {
                flush(&mut out, &mut group, &mut group_n);
                out.push("|".into());
            }
            Item::Raw(t) => {
                flush(&mut out, &mut group, &mut group_n);
                out.push(t.clone());
            }
            Item::Free(open) => {
                flush(&mut out, &mut group, &mut group_n);
                out.push(if *open { "free{" } else { "}" }.into());
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
    fn a_swell_goes_back_to_its_level() {
        let s = (0, 1);
        let model = Model {
            dynamics: vec![(Q::ZERO, s, 2)],
            wedges: vec![
                (Q::int(1), s, Wedge::Cresc),
                (Q::int(2), s, Wedge::Stop),
                (Q::int(2), s, Wedge::Dim),
                (Q::int(3), s, Wedge::Stop),
            ],
            ..Model::default()
        };
        let c = &curves(&model)[&s];
        let at = |t: Q| c.iter().rfind(|g| g.at <= t).unwrap().level_at(t);
        // p, up to mp, back to p (not down to pp)
        assert_eq!(at(Q::new(1, 2)), 2);
        assert_eq!(at(Q::int(2)), 3);
        assert_eq!(at(Q::new(5, 2)), 3);
        assert_eq!(at(Q::int(3)), 2);
        // a written mark at the end is the target
        let mut model = model;
        model.wedges.truncate(2);
        model.dynamics.push((Q::int(2), s, 5));
        let c = &curves(&model)[&s];
        assert_eq!(c[1].ramp, Some((Q::int(2), 5)));
    }

    #[test]
    fn a_wedge_finds_its_mark_in_the_next_bar() {
        // A pickup of one quarter, then 2/2 bars: the wedge stops at quarter 3 and
        // the mark comes at the next bar (quarter 5), more than a pickup later.
        let s = (0, 1);
        let bar = |start: i64, len: i64| crate::musicxml::Measure {
            number: String::new(),
            start: Q::int(start),
            len: Q::int(len),
            sig: (2, 2),
            break_before: false,
        };
        let model = Model {
            dynamics: vec![(Q::ZERO, s, 2), (Q::int(5), s, 6)],
            wedges: vec![(Q::int(1), s, Wedge::Cresc), (Q::int(3), s, Wedge::Stop)],
            measures: vec![bar(0, 1), bar(1, 4), bar(5, 4)],
            ..Model::default()
        };
        let c = &curves(&model)[&s];
        assert_eq!(c[1].ramp, Some((Q::int(3), 6)));
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
