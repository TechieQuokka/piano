//! MusicXML → [`Model`]: timed notes, marks, and measures in playing order.
//!
//! Every part is read; its voices and staves stay apart. Repeats and endings are
//! unrolled, so the model is what is heard. Times are quarters from the start.

use std::collections::{HashMap, HashSet};

use roxmltree::{Document, Node, ParsingOptions};

use crate::ImportError;
use crate::rational::Q;

/// (part, voice) as written in the file.
pub type VoiceId = (usize, u32);
/// (part, staff).
pub type StaffId = (usize, u32);

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub voice: VoiceId,
    pub staff: StaffId,
    pub start: Q,
    pub len: Q,
    pub midi: i32,
    /// Name in the language, e.g. `C#4` (double accidentals respelled).
    pub name: String,
    /// Tied to the next note of the same pitch in the same voice.
    pub tie: bool,
    /// Trill: the upper neighbor (name, MIDI).
    pub trill: Option<(String, i32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grace {
    pub voice: VoiceId,
    pub staff: StaffId,
    /// Start of the note the grace leads to.
    pub principal: Q,
    /// Position in its group, and the group size.
    pub index: usize,
    pub count: usize,
    pub midi: i32,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wedge {
    Cresc,
    Dim,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pedal {
    Down,
    Up,
    /// Up and down again at once.
    Change,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
    /// Number as written (for comments).
    pub number: String,
    pub start: Q,
    pub len: Q,
    /// Time signature in force (written), as (beats, unit).
    pub sig: (u32, u32),
    /// A double bar line, rehearsal mark, or repeat jump: a good place for a new piece.
    pub break_before: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Model {
    pub title: Option<String>,
    pub notes: Vec<Note>,
    pub graces: Vec<Grace>,
    /// (time, staff, level 0 = ppp .. 7 = fff)
    pub dynamics: Vec<(Q, StaffId, usize)>,
    pub wedges: Vec<(Q, StaffId, Wedge)>,
    pub pedals: Vec<(Q, Pedal)>,
    /// (time, quarter notes per minute)
    pub tempos: Vec<(Q, f64)>,
    /// In playing order.
    pub measures: Vec<Measure>,
    /// Things read but not played as written, for the report.
    pub notices: Notices,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Notices {
    pub cue: usize,
    pub unpitched: usize,
    pub arpeggios: usize,
    /// Repeats that never ended (unrolling stopped).
    pub runaway_repeats: usize,
}

pub(crate) const LETTERS: [char; 7] = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];

fn step_semitone(letter: char) -> i32 {
    match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        _ => 11,
    }
}

pub(crate) fn midi_of(letter: char, alter: i32, octave: i32) -> i32 {
    (octave + 1) * 12 + step_semitone(letter) + alter
}

/// Note name in the language. Double sharps / flats become the neighbor letter.
pub fn spell(letter: char, alter: i32, octave: i32) -> String {
    if alter.abs() >= 2 {
        let midi = midi_of(letter, alter, octave);
        let i = LETTERS.iter().position(|c| *c == letter).unwrap_or(0) as i32 + alter.signum();
        let o = octave + i.div_euclid(7);
        let l = LETTERS[i.rem_euclid(7) as usize];
        return spell(l, midi - midi_of(l, 0, o), o);
    }
    let acc = match alter {
        1 => "#",
        -1 => "b",
        _ => "",
    };
    format!("{letter}{acc}{octave}")
}

fn child<'a>(n: Node<'a, 'a>, name: &str) -> Option<Node<'a, 'a>> {
    n.children().find(|c| c.has_tag_name(name))
}

fn text<'a>(n: Node<'a, 'a>, name: &str) -> Option<&'a str> {
    child(n, name).and_then(|c| c.text()).map(str::trim)
}

fn int(n: Node, name: &str) -> Option<i64> {
    text(n, name)
        .and_then(|t| t.parse::<f64>().ok())
        .map(|f| f.round() as i64)
}

const LEVELS: [&str; 8] = ["ppp", "pp", "p", "mp", "mf", "f", "ff", "fff"];

pub(crate) fn level(tag: &str) -> Option<usize> {
    if let Some(i) = LEVELS.iter().position(|l| *l == tag) {
        return Some(i);
    }
    if tag.len() > 3 && tag.chars().all(|c| c == 'p') {
        return Some(0);
    }
    if tag.len() > 3 && tag.chars().all(|c| c == 'f') {
        return Some(7);
    }
    None
}

/// One written measure of one part, times relative to its start.
#[derive(Default)]
struct RawMeasure {
    number: String,
    len: Q,
    sig: Option<(u32, u32)>,
    forward: bool,
    backward: Option<u32>,
    /// Numbers of an ending that starts here.
    ending: Option<Vec<u32>>,
    ending_end: bool,
    double_left: bool,
    double_right: bool,
    rehearsal: bool,
    events: Vec<(Q, Event)>,
}

enum Event {
    Note {
        voice: u32,
        staff: u32,
        len: Q,
        midi: i32,
        name: String,
        tie: bool,
        trill: Option<(String, i32)>,
    },
    Grace {
        voice: u32,
        staff: u32,
        midi: i32,
        name: String,
        chord: bool,
    },
    Dynamic(u32, usize),
    Wedge(u32, Wedge),
    Pedal(Pedal),
    Tempo(f64),
}

pub fn parse(xml: &str) -> Result<Model, ImportError> {
    let opts = ParsingOptions {
        allow_dtd: true,
        ..ParsingOptions::default()
    };
    let doc =
        Document::parse_with_options(xml, opts).map_err(|e| ImportError::Xml(e.to_string()))?;
    let root = doc.root_element();
    if !root.has_tag_name("score-partwise") {
        return Err(ImportError::Format(format!(
            "<{}>: only score-partwise MusicXML is supported",
            root.tag_name().name()
        )));
    }
    let title = child(root, "work")
        .and_then(|w| text(w, "work-title"))
        .or_else(|| text(root, "movement-title"))
        .filter(|t| !t.is_empty())
        .map(str::to_string);
    let mut notices = Notices::default();
    let parts: Vec<Vec<RawMeasure>> = root
        .children()
        .filter(|c| c.has_tag_name("part"))
        .map(|p| read_part(p, &mut notices))
        .collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(ImportError::Format("no measures".into()));
    }
    let count = parts.iter().map(Vec::len).max().unwrap_or(0);
    // Measure lengths: the longest part; signatures and bar lines from the first.
    let mut sig = (4, 4);
    let mut written = Vec::with_capacity(count);
    for i in 0..count {
        if let Some(s) = parts[0].get(i).and_then(|m| m.sig) {
            sig = s;
        }
        let mut len = parts
            .iter()
            .filter_map(|p| p.get(i))
            .map(|m| m.len)
            .max()
            .unwrap_or(Q::ZERO);
        if len == Q::ZERO {
            len = Q::new(4 * i64::from(sig.0), i64::from(sig.1));
        }
        written.push((len, sig));
    }
    let order = unroll(&parts[0], &mut notices);
    let mut model = Model {
        title,
        notices,
        ..Model::default()
    };
    let mut start = Q::ZERO;
    let mut prev: Option<usize> = None;
    for &i in &order {
        let (len, sig) = written[i];
        let m0 = &parts[0][i];
        let brk = m0.double_left
            || m0.rehearsal
            || prev.is_some_and(|p| parts[0][p].double_right || p + 1 != i);
        model.measures.push(Measure {
            number: m0.number.clone(),
            start,
            len,
            sig,
            break_before: brk && prev.is_some(),
        });
        for (pi, part) in parts.iter().enumerate() {
            if let Some(m) = part.get(i) {
                place(&mut model, pi, m, start);
            }
        }
        start = start + len;
        prev = Some(i);
    }
    finish(&mut model);
    Ok(model)
}

/// Adds one measure's events at `start`.
fn place(model: &mut Model, part: usize, m: &RawMeasure, start: Q) {
    // Grace notes wait for the next note of their voice.
    let mut pending: HashMap<u32, Vec<(u32, i32, String, bool)>> = HashMap::new();
    for (off, ev) in &m.events {
        let at = start + *off;
        match ev {
            Event::Note {
                voice,
                staff,
                len,
                midi,
                name,
                tie,
                trill,
            } => {
                if let Some(gs) = pending.remove(voice) {
                    let mut index = 0;
                    let mut group = Vec::new();
                    for (i, (gstaff, gmidi, gname, chord)) in gs.into_iter().enumerate() {
                        if i > 0 && !chord {
                            index += 1;
                        }
                        group.push(Grace {
                            voice: (part, *voice),
                            staff: (part, gstaff),
                            principal: at,
                            index,
                            count: 0,
                            midi: gmidi,
                            name: gname,
                        });
                    }
                    for g in &mut group {
                        g.count = index + 1;
                    }
                    model.graces.extend(group);
                }
                model.notes.push(Note {
                    voice: (part, *voice),
                    staff: (part, *staff),
                    start: at,
                    len: *len,
                    midi: *midi,
                    name: name.clone(),
                    tie: *tie,
                    trill: trill.clone(),
                });
            }
            Event::Grace {
                voice,
                staff,
                midi,
                name,
                chord,
            } => pending
                .entry(*voice)
                .or_default()
                .push((*staff, *midi, name.clone(), *chord)),
            Event::Dynamic(staff, l) => model.dynamics.push((at, (part, *staff), *l)),
            Event::Wedge(staff, w) => model.wedges.push((at, (part, *staff), *w)),
            Event::Pedal(p) => model.pedals.push((at, *p)),
            Event::Tempo(t) => model.tempos.push((at, *t)),
        }
    }
}

/// Sorts events and drops ties that lead nowhere.
pub(crate) fn finish(model: &mut Model) {
    model.notes.sort_by_key(|n| (n.start, n.voice, n.midi));
    let starts: HashSet<(VoiceId, i32, Q)> = model
        .notes
        .iter()
        .map(|n| (n.voice, n.midi, n.start))
        .collect();
    for n in &mut model.notes {
        if n.tie && !starts.contains(&(n.voice, n.midi, n.start + n.len)) {
            n.tie = false;
        }
    }
    model.dynamics.sort_by_key(|d| d.0);
    model.wedges.sort_by_key(|w| w.0);
    model.pedals.sort_by_key(|p| (p.0, p.1 != Pedal::Up));
    model.tempos.sort_by_key(|t| t.0);
}

/// Playing order of the written measures: repeats and endings unrolled.
fn unroll(ms: &[RawMeasure], notices: &mut Notices) -> Vec<usize> {
    let limit = ms.len() * 100;
    let mut order = Vec::new();
    let (mut i, mut start, mut pass, mut jumped) = (0, 0, 1, false);
    while i < ms.len() && order.len() < limit {
        let m = &ms[i];
        if m.forward && !jumped {
            start = i;
            pass = 1;
        }
        jumped = false;
        if let Some(numbers) = &m.ending
            && !numbers.contains(&pass)
        {
            // Not this pass's ending: skip to the measure after it.
            let mut j = i;
            while j + 1 < ms.len() && !ms[j].ending_end {
                j += 1;
            }
            i = j + 1;
            continue;
        }
        order.push(i);
        match m.backward {
            Some(times) if pass < times => {
                pass += 1;
                i = start;
                jumped = true;
                continue;
            }
            Some(_) => {
                pass = 1;
                start = i + 1;
            }
            None => {}
        }
        i += 1;
    }
    if order.len() >= limit {
        notices.runaway_repeats += 1;
    }
    order
}

fn read_part(part: Node, notices: &mut Notices) -> Vec<RawMeasure> {
    let mut out = Vec::new();
    let mut div = Q::int(1); // divisions per quarter
    let mut fifths = 0;
    for m in part.children().filter(|c| c.has_tag_name("measure")) {
        let mut rm = RawMeasure {
            number: m.attribute("number").unwrap_or("").to_string(),
            ..RawMeasure::default()
        };
        let mut cur = Q::ZERO;
        let mut top = Q::ZERO;
        let mut last_onset = Q::ZERO;
        for el in m.children().filter(Node::is_element) {
            match el.tag_name().name() {
                "attributes" => {
                    if let Some(d) = int(el, "divisions") {
                        div = Q::int(d.max(1));
                    }
                    if let Some(k) = child(el, "key").and_then(|k| int(k, "fifths")) {
                        fifths = k as i32;
                    }
                    if let Some(t) = child(el, "time") {
                        let beats: u32 = text(t, "beats")
                            .unwrap_or("4")
                            .split('+')
                            .filter_map(|b| b.trim().parse::<u32>().ok())
                            .sum();
                        let unit = int(t, "beat-type").unwrap_or(4) as u32;
                        rm.sig = Some((beats.max(1), unit.max(1)));
                    }
                }
                "backup" => cur = cur - Q::int(int(el, "duration").unwrap_or(0)) / div,
                "forward" => {
                    cur = cur + Q::int(int(el, "duration").unwrap_or(0)) / div;
                    top = top.max(cur);
                }
                "direction" => {
                    let off = Q::int(int(el, "offset").unwrap_or(0)) / div;
                    let staff = int(el, "staff").unwrap_or(1) as u32;
                    direction(el, staff, cur + off, &mut rm);
                }
                "sound" => {
                    if let Some(t) = el.attribute("tempo").and_then(|t| t.parse::<f64>().ok()) {
                        rm.events.push((cur, Event::Tempo(t)));
                    }
                }
                "barline" => barline(el, &mut rm),
                "note" => {
                    let voice = text(el, "voice").and_then(|v| v.parse().ok()).unwrap_or(1);
                    let staff = int(el, "staff").unwrap_or(1) as u32;
                    let chord = child(el, "chord").is_some();
                    let spelled = child(el, "pitch").map(|p| {
                        let letter = text(p, "step")
                            .and_then(|s| s.chars().next())
                            .unwrap_or('C');
                        let alter = text(p, "alter")
                            .and_then(|a| a.parse::<f64>().ok())
                            .map_or(0, |a| a.round() as i32);
                        (letter, alter, int(p, "octave").unwrap_or(4) as i32)
                    });
                    if child(el, "grace").is_some() {
                        if let Some((l, a, o)) = spelled {
                            rm.events.push((
                                cur,
                                Event::Grace {
                                    voice,
                                    staff,
                                    midi: midi_of(l, a, o),
                                    name: spell(l, a, o),
                                    chord,
                                },
                            ));
                        }
                        continue;
                    }
                    let len = Q::int(int(el, "duration").unwrap_or(0)) / div;
                    let onset = if chord { last_onset } else { cur };
                    if !chord {
                        cur = cur + len;
                        top = top.max(cur);
                    }
                    last_onset = onset;
                    if child(el, "cue").is_some() {
                        notices.cue += 1;
                        continue;
                    }
                    if child(el, "unpitched").is_some() {
                        notices.unpitched += 1;
                        continue;
                    }
                    let Some((l, a, o)) = spelled else { continue };
                    let tie = el
                        .children()
                        .any(|t| t.has_tag_name("tie") && t.attribute("type") == Some("start"));
                    let notations = child(el, "notations");
                    if notations.is_some_and(|n| child(n, "arpeggiate").is_some()) {
                        notices.arpeggios += 1;
                    }
                    let trill = notations
                        .and_then(|n| child(n, "ornaments"))
                        .filter(|orn| child(*orn, "trill-mark").is_some())
                        .map(|orn| upper_neighbor(l, o, fifths, marked_accidental(orn)));
                    rm.events.push((
                        onset,
                        Event::Note {
                            voice,
                            staff,
                            len,
                            midi: midi_of(l, a, o),
                            name: spell(l, a, o),
                            tie,
                            trill,
                        },
                    ));
                }
                _ => {}
            }
        }
        rm.len = top;
        out.push(rm);
    }
    out
}

/// Accidental written for a trill's upper note.
fn marked_accidental(ornaments: Node) -> Option<i32> {
    match text(ornaments, "accidental-mark") {
        Some("sharp") => Some(1),
        Some("natural") => Some(0),
        Some("flat") => Some(-1),
        Some("double-sharp") => Some(2),
        Some("flat-flat") => Some(-2),
        _ => None,
    }
}

/// Diatonic step above `letter` in the key of `fifths`, or with the marked accidental.
fn upper_neighbor(letter: char, octave: i32, fifths: i32, marked: Option<i32>) -> (String, i32) {
    let i = LETTERS.iter().position(|c| *c == letter).unwrap_or(0) + 1;
    let (l, o) = (LETTERS[i % 7], octave + (i / 7) as i32);
    let sharps = &"FCGDAEB"[..fifths.clamp(0, 7) as usize];
    let flats = &"BEADGCF"[..(-fifths).clamp(0, 7) as usize];
    let alter = marked.unwrap_or(if sharps.contains(l) {
        1
    } else if flats.contains(l) {
        -1
    } else {
        0
    });
    (spell(l, alter, o), midi_of(l, alter, o))
}

fn direction(el: Node, staff: u32, at: Q, rm: &mut RawMeasure) {
    let mut metronome = None;
    for dt in el.children().filter(|c| c.has_tag_name("direction-type")) {
        for e in dt.children().filter(Node::is_element) {
            match e.tag_name().name() {
                "dynamics" => {
                    for d in e.children().filter(Node::is_element) {
                        if let Some(l) = level(d.tag_name().name()) {
                            rm.events.push((at, Event::Dynamic(staff, l)));
                        }
                    }
                }
                "wedge" => {
                    let w = match e.attribute("type") {
                        Some("crescendo") => Some(Wedge::Cresc),
                        Some("diminuendo") => Some(Wedge::Dim),
                        Some("stop") => Some(Wedge::Stop),
                        _ => None,
                    };
                    if let Some(w) = w {
                        rm.events.push((at, Event::Wedge(staff, w)));
                    }
                }
                "pedal" => {
                    let p = match e.attribute("type") {
                        Some("start" | "resume") => Some(Pedal::Down),
                        Some("stop" | "discontinue") => Some(Pedal::Up),
                        Some("change") => Some(Pedal::Change),
                        _ => None,
                    };
                    if let Some(p) = p {
                        rm.events.push((at, Event::Pedal(p)));
                    }
                }
                "metronome" => {
                    let per = text(e, "per-minute").and_then(|t| t.parse::<f64>().ok());
                    let unit = match text(e, "beat-unit") {
                        Some("whole") => 4.0,
                        Some("half") => 2.0,
                        Some("eighth") => 0.5,
                        Some("16th") => 0.25,
                        _ => 1.0,
                    };
                    let dotted = e.children().any(|c| c.has_tag_name("beat-unit-dot"));
                    metronome = per.map(|p| p * unit * if dotted { 1.5 } else { 1.0 });
                }
                "rehearsal" => rm.rehearsal = true,
                _ => {}
            }
        }
    }
    let sound = child(el, "sound")
        .and_then(|s| s.attribute("tempo"))
        .and_then(|t| t.parse::<f64>().ok());
    if let Some(t) = sound.or(metronome) {
        rm.events.push((at, Event::Tempo(t)));
    }
}

fn barline(el: Node, rm: &mut RawMeasure) {
    let left = el.attribute("location") == Some("left");
    if text(el, "bar-style").is_some_and(|s| {
        matches!(
            s,
            "light-light" | "light-heavy" | "heavy-light" | "heavy-heavy"
        )
    }) {
        if left {
            rm.double_left = true;
        } else {
            rm.double_right = true;
        }
    }
    if let Some(r) = child(el, "repeat") {
        match r.attribute("direction") {
            Some("forward") => rm.forward = true,
            Some("backward") => {
                let times = r
                    .attribute("times")
                    .and_then(|t| t.parse().ok())
                    .unwrap_or(2);
                rm.backward = Some(times);
            }
            _ => {}
        }
    }
    if let Some(e) = child(el, "ending") {
        let numbers: Vec<u32> = e
            .attribute("number")
            .unwrap_or("1")
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter_map(|n| n.trim().parse().ok())
            .collect();
        match e.attribute("type") {
            Some("start") => rm.ending = Some(numbers),
            Some("stop" | "discontinue") => rm.ending_end = true,
            _ => {}
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn score(measures: &str) -> String {
        format!(
            r#"<?xml version="1.0"?>
<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 3.1 Partwise//EN" "x.dtd">
<score-partwise><work><work-title>Test</work-title></work>
<part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
<part id="P1">{measures}</part></score-partwise>"#
        )
    }

    /// A note of `dur` divisions in voice 1, staff 1; `extra` goes first (e.g. `<chord/>`).
    pub(crate) fn note(step: &str, octave: i32, dur: u32, extra: &str) -> String {
        format!(
            "<note>{extra}<pitch><step>{step}</step><octave>{octave}</octave></pitch>\
             <duration>{dur}</duration><voice>1</voice><staff>1</staff></note>"
        )
    }

    /// Two quarters per measure, two divisions per quarter.
    pub(crate) const ATTRS: &str = "<attributes><divisions>2</divisions><key><fifths>0</fifths></key>\
        <time><beats>2</beats><beat-type>4</beat-type></time></attributes>";

    #[test]
    fn notes_chords_and_backup() {
        // E4 with C5 on top, then A4; a bass voice under both.
        let m = format!(
            r#"<measure number="1">{ATTRS}{}{}{}<backup><duration>4</duration></backup>
            <note><pitch><step>C</step><octave>3</octave></pitch><duration>4</duration><voice>5</voice><staff>2</staff></note></measure>"#,
            note("E", 4, 2, ""),
            note("C", 5, 2, "<chord/>"),
            note("A", 4, 2, ""),
        );
        let model = parse(&score(&m)).unwrap();
        assert_eq!(model.title.as_deref(), Some("Test"));
        let got: Vec<(&str, Q)> = model
            .notes
            .iter()
            .map(|n| (n.name.as_str(), n.start))
            .collect();
        assert_eq!(
            got,
            [
                ("E4", Q::ZERO),
                ("C5", Q::ZERO),
                ("C3", Q::ZERO),
                ("A4", Q::int(1))
            ]
        );
        assert_eq!(model.measures.len(), 1);
        assert_eq!(model.measures[0].len, Q::int(2));
    }

    #[test]
    fn repeats_and_endings_unroll() {
        let bar = |n: u32, extra: &str| {
            let attrs = if n == 1 { ATTRS } else { "" };
            format!(
                r#"<measure number="{n}">{attrs}{extra}{}</measure>"#,
                note("C", 4, 4, "")
            )
        };
        let m = [
            bar(1, r#"<barline location="left"><repeat direction="forward"/></barline>"#),
            bar(
                2,
                r#"<barline location="left"><ending number="1" type="start"/></barline>
                <barline location="right"><ending number="1" type="stop"/><repeat direction="backward"/></barline>"#,
            ),
            bar(
                3,
                r#"<barline location="left"><ending number="2" type="start"/></barline>
                <barline location="right"><ending number="2" type="discontinue"/></barline>"#,
            ),
        ]
        .concat();
        let model = parse(&score(&m)).unwrap();
        let order: Vec<&str> = model.measures.iter().map(|m| m.number.as_str()).collect();
        assert_eq!(order, ["1", "2", "1", "3"]);
        assert_eq!(model.notes.len(), 4);
        assert_eq!(model.notes[3].start, Q::int(6));
        assert!(
            model.measures[2].break_before,
            "a jump back starts a new piece"
        );
    }

    #[test]
    fn double_sharps_are_respelled() {
        assert_eq!(spell('F', 2, 4), "G4");
        assert_eq!(spell('B', 2, 3), "C#4");
        assert_eq!(spell('C', -2, 4), "Bb3");
        assert_eq!(spell('E', 1, 4), "E#4");
    }

    #[test]
    fn trill_neighbor_follows_the_key() {
        // D# in G# minor (5 sharps): the upper neighbor is E.
        assert_eq!(upper_neighbor('D', 7, 5, None), ("E7".to_string(), 100));
        assert_eq!(upper_neighbor('E', 4, 5, None), ("F#4".to_string(), 66));
        assert_eq!(upper_neighbor('B', 4, 0, Some(-1)).0, "Cb5");
    }
}
