//! Humdrum `**kern` → [`Model`], the same model MusicXML is read into.
//!
//! Each `**kern` spine is a staff (`*staffN`, else counted from the right); the
//! sub-spines of `*^` / `*v` are its voices. A `**dynam` spine gives the dynamics of
//! its staves. Sections (`*>A`) are played in the order of the expansion list
//! (`*>[A,B,A]`), or as written when there is none. Times are quarters.

use std::collections::HashMap;

use crate::ImportError;
use crate::musicxml::{
    self, Grace, LETTERS, Marks, Measure, Model, Note, Pedal, SOFT, Sudden, Wedge, level, midi_of,
    spell,
};
use crate::rational::Q;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Kern,
    Dynam,
    Other,
}

#[derive(Debug, Clone)]
struct Column {
    kind: Kind,
    /// The spine it was split from, so its sub-spines are told apart.
    spine: usize,
    staves: Vec<u32>,
    /// When its last note or rest ends.
    end: Q,
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
        marks: Marks,
    },
    Grace {
        voice: u32,
        staff: u32,
        midi: i32,
        name: String,
        chord: bool,
    },
    Dynamic(u32, usize),
    Sudden(u32, Sudden),
    /// A fermata over a rest of this length.
    RestHold(Q),
    Wedge(u32, Wedge),
    Pedal(Pedal),
    Tempo(f64),
}

/// One written bar; event times are from the start of the score as written.
struct Bar {
    number: String,
    start: Q,
    len: Q,
    sig: (u32, u32),
    /// Section label starting here.
    label: Option<String>,
    /// The bar line before it is a double or repeat bar line.
    double_before: bool,
    events: Vec<(Q, Event)>,
}

impl Bar {
    fn new(number: String, start: Q, sig: (u32, u32), double_before: bool) -> Bar {
        Bar {
            number,
            start,
            len: Q::ZERO,
            sig,
            label: None,
            double_before,
            events: Vec::new(),
        }
    }
}

/// A note of a kern token: `4.cc#[`, `16eq`, `2aaT`.
struct Token {
    /// Quarters; zero for grace notes.
    len: Q,
    /// (letter, alteration, octave); `None` for a rest.
    pitch: Option<(char, i32, i32)>,
    grace: bool,
    tie: bool,
    /// Semitones to the trill's upper note.
    trill: Option<i32>,
    marks: Marks,
    /// `z`: sforzando.
    sf: bool,
}

fn token(s: &str) -> Option<Token> {
    let digits: String = s
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '%')
        .collect();
    let grace = s.contains(['q', 'Q']);
    let len = if grace {
        Q::ZERO
    } else {
        let (n, m) = digits.split_once('%').unwrap_or((&digits, "1"));
        let n: i64 = n.parse().ok()?;
        let m: i64 = m.parse().ok()?;
        // `0` is a breve, `00` a long.
        let base = match (n, digits.starts_with("00")) {
            (0, true) => Q::int(16),
            (0, false) => Q::int(8),
            _ => Q::new(4 * m, n),
        };
        let dots = s.chars().filter(|c| *c == '.').count() as i64;
        base * (Q::int(2) - Q::new(1, 1 << dots))
    };
    let tie = s.contains(['[', '_']);
    if s.contains('r') {
        return Some(Token {
            len,
            pitch: None,
            grace,
            tie: false,
            trill: None,
            marks: Marks {
                fermata: s.contains(';'),
                ..Marks::default()
            },
            sf: false,
        });
    }
    let letter = s
        .chars()
        .find(|c| matches!(c.to_ascii_lowercase(), 'a'..='g'))?;
    let count = s.chars().filter(|c| *c == letter).count() as i32;
    let octave = if letter.is_ascii_lowercase() {
        3 + count
    } else {
        4 - count
    };
    let alter = s.chars().filter(|c| *c == '#').count() as i32
        - s.chars().filter(|c| *c == '-').count() as i32;
    let trill = if s.contains('T') {
        Some(2)
    } else if s.contains('t') {
        Some(1)
    } else {
        None
    };
    // `'` staccato, `` ` `` staccatissimo, `^` accent (`^^` heavy), `~` tenuto.
    let marks = Marks {
        staccato: if s.contains('`') {
            2
        } else {
            u8::from(s.contains('\''))
        },
        accent: s.contains('^'),
        tenuto: s.contains('~'),
        fermata: s.contains(';'),
    };
    Some(Token {
        len,
        pitch: Some((letter.to_ascii_uppercase(), alter, octave)),
        grace,
        tie,
        trill,
        marks,
        sf: s.contains('z'),
    })
}

/// The trill's upper note: the next letter, `semis` above.
fn upper(letter: char, alter: i32, octave: i32, semis: i32) -> (String, i32) {
    let i = LETTERS.iter().position(|c| *c == letter).unwrap_or(0) + 1;
    let (l, o) = (LETTERS[i % 7], octave + (i / 7) as i32);
    let midi = midi_of(letter, alter, octave) + semis;
    (spell(l, midi - midi_of(l, 0, o), o), midi)
}

/// Voice of column `c`: its place among the sub-spines of its spine, per staff.
fn voice(cols: &[Column], c: usize) -> (u32, u32) {
    let staff = cols[c].staves.first().copied().unwrap_or(1);
    let pos = cols[..c]
        .iter()
        .filter(|x| x.spine == cols[c].spine)
        .count() as u32;
    (staff * 10 + pos + 1, staff)
}

pub fn parse(text: &str) -> Result<Model, ImportError> {
    let mut title: Option<String> = None;
    let mut movement: Option<String> = None;
    let mut cols: Vec<Column> = Vec::new();
    let mut expansion: Option<Vec<String>> = None;
    let mut sig = (4, 4);
    let mut t = Q::ZERO;
    let mut bars = vec![Bar::new("0".into(), Q::ZERO, sig, false)];
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("!!!") {
            if let Some((key, value)) = rest.split_once(':') {
                let value = value.trim().to_string();
                match key {
                    "OTL" if title.is_none() => title = Some(value),
                    "OMD" if movement.is_none() && cols.is_empty() => movement = Some(value),
                    _ => {}
                }
            }
            continue;
        }
        if line.starts_with('!') {
            continue;
        }
        let tokens: Vec<&str> = line.split('\t').collect();
        if tokens[0].starts_with("**") {
            let kerns = tokens.iter().filter(|t| **t == "**kern").count();
            let mut k = 0;
            cols = tokens
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let kind = match *t {
                        "**kern" => Kind::Kern,
                        "**dynam" => Kind::Dynam,
                        _ => Kind::Other,
                    };
                    let staves = if kind == Kind::Kern {
                        k += 1;
                        vec![(kerns - k + 1) as u32]
                    } else {
                        Vec::new()
                    };
                    Column {
                        kind,
                        spine: i,
                        staves,
                        end: Q::ZERO,
                    }
                })
                .collect();
            continue;
        }
        if cols.is_empty() {
            continue;
        }
        let bar = bars.last_mut().expect("a bar");
        if tokens[0].starts_with('*') {
            interpretation(&tokens, &mut cols, bar, t, &mut sig, &mut expansion)?;
            continue;
        }
        if let Some(b) = tokens[0].strip_prefix('=') {
            bar.len = t - bar.start;
            bar.sig = sig;
            if b.starts_with('=') {
                break; // final bar line
            }
            // A bar line without a number splits a bar (e.g. at a repeat sign).
            let mut number: String = b.chars().take_while(char::is_ascii_digit).collect();
            if number.is_empty() {
                number.clone_from(&bar.number);
            }
            let double =
                b.contains("||") || b.contains(':') || b.contains("!|") || b.contains("|!");
            bars.push(Bar::new(number, t, sig, double));
            continue;
        }
        if tokens.len() != cols.len() {
            return Err(ImportError::Format(format!(
                "line has {} spines, expected {}: {line}",
                tokens.len(),
                cols.len()
            )));
        }
        // A data line: notes start at `t`; the line lasts until the first one ends.
        let mut next: Option<Q> = None;
        for (c, tok) in tokens.iter().enumerate() {
            let col = &cols[c];
            if *tok == "." {
                // A note that ends here makes a line of marks only (`.` everywhere
                // but a dynamic) last no time.
                if col.kind == Kind::Kern && col.end >= t {
                    next = Some(next.map_or(col.end, |n| n.min(col.end)));
                }
                continue;
            }
            match col.kind {
                Kind::Kern => {
                    let (v, staff) = voice(&cols, c);
                    let mut len = Q::ZERO;
                    for (i, sub) in tok.split(' ').enumerate() {
                        let Some(n) = token(sub) else { continue };
                        len = len.max(n.len);
                        let Some((l, a, o)) = n.pitch else {
                            if n.marks.fermata && !n.grace {
                                bar.events.push((t, Event::RestHold(n.len)));
                            }
                            continue;
                        };
                        let midi = midi_of(l, a, o);
                        let name = spell(l, a, o);
                        if n.sf && !n.grace {
                            bar.events.push((t, Event::Sudden(staff, Sudden::Sf)));
                        }
                        let ev = if n.grace {
                            Event::Grace {
                                voice: v,
                                staff,
                                midi,
                                name,
                                chord: i > 0,
                            }
                        } else {
                            Event::Note {
                                voice: v,
                                staff,
                                len: n.len,
                                midi,
                                name,
                                tie: n.tie,
                                trill: n.trill.map(|s| upper(l, a, o, s)),
                                marks: n.marks,
                            }
                        };
                        bar.events.push((t, ev));
                    }
                    // Grace notes last no time: the note held in the column goes on.
                    if len > Q::ZERO {
                        cols[c].end = t + len;
                    }
                    next = Some(next.map_or(t + len, |n| n.min(t + len)));
                }
                Kind::Dynam => dynamics(tok, &col.staves, t, bar),
                Kind::Other => {}
            }
        }
        t = next.unwrap_or(t);
    }
    let last = bars.last_mut().expect("a bar");
    if last.len == Q::ZERO {
        last.len = t - last.start;
        last.sig = sig;
    }
    let mut model = Model {
        title: match (title, movement) {
            (Some(t), Some(m)) => Some(format!("{t} — {m}")),
            (t, m) => t.or(m),
        },
        ..Model::default()
    };
    let bars = drop_empty(bars);
    if bars.is_empty() {
        return Err(ImportError::Format("no notes".into()));
    }
    let order = order(&bars, expansion.as_deref());
    place(&mut model, &bars, &order);
    musicxml::finish(&mut model);
    Ok(model)
}

fn interpretation(
    tokens: &[&str],
    cols: &mut Vec<Column>,
    bar: &mut Bar,
    t: Q,
    sig: &mut (u32, u32),
    expansion: &mut Option<Vec<String>>,
) -> Result<(), ImportError> {
    if tokens.len() != cols.len() {
        return Err(ImportError::Format(format!(
            "line has {} spines, expected {}: {}",
            tokens.len(),
            cols.len(),
            tokens.join("\t")
        )));
    }
    let mut out: Vec<Column> = Vec::with_capacity(cols.len());
    let mut tempo = None;
    let mut new_sig = None;
    let mut merging = false;
    for (c, tok) in tokens.iter().enumerate() {
        let mut col = cols[c].clone();
        let tok = tok.trim_start_matches('*');
        if tok == "v" {
            if merging && let Some(prev) = out.last_mut() {
                prev.end = prev.end.max(col.end);
                continue;
            }
            merging = true;
            out.push(col);
            continue;
        }
        merging = false;
        match tok {
            "^" => {
                out.push(col.clone());
                out.push(col);
                continue;
            }
            "-" => continue,
            "ped" if col.kind == Kind::Kern => bar.events.push((t, Event::Pedal(Pedal::Down))),
            "Xped" if col.kind == Kind::Kern => bar.events.push((t, Event::Pedal(Pedal::Up))),
            _ => {}
        }
        if let Some(s) = tok.strip_prefix("staff") {
            col.staves = s.split('/').filter_map(|n| n.parse().ok()).collect();
        } else if let Some(m) = tok.strip_prefix("MM") {
            tempo = tempo.or(m.parse::<f64>().ok());
        } else if let Some((b, u)) = tok.strip_prefix('M').and_then(|m| m.split_once('/')) {
            if let (Ok(b), Ok(u)) = (b.parse::<u32>(), u.parse::<u32>()) {
                new_sig = Some((b.max(1), u.max(1)));
            }
        } else if c == 0
            && let Some(e) = tok.strip_prefix('>')
        {
            if let Some(list) = e.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                expansion.get_or_insert_with(|| list.split(',').map(str::to_string).collect());
            } else if !e.contains('[') {
                bar.label = Some(e.to_string());
            }
        }
        out.push(col);
    }
    *cols = out;
    if let Some(s) = new_sig {
        *sig = s;
        bar.sig = s;
    }
    if let Some(m) = tempo {
        bar.events.push((t, Event::Tempo(m)));
    }
    Ok(())
}

/// `p`, `ff`, `<` (crescendo), `>` (diminuendo), `[` / `]` (end of a hairpin).
fn dynamics(tok: &str, staves: &[u32], t: Q, bar: &mut Bar) {
    let staves = if staves.is_empty() {
        &[1, 2][..]
    } else {
        staves
    };
    for sub in tok.split(' ') {
        let mut push = |f: &dyn Fn(u32) -> Event| {
            for &s in staves {
                bar.events.push((t, f(s)));
            }
        };
        if sub.contains(['[', ']']) {
            push(&|s| Event::Wedge(s, Wedge::Stop));
        }
        let mark = sub.trim_matches(['[', ']']);
        match mark {
            "<" => push(&|s| Event::Wedge(s, Wedge::Cresc)),
            ">" => push(&|s| Event::Wedge(s, Wedge::Dim)),
            _ => {
                if let Some(l) = level(mark) {
                    push(&|s| Event::Dynamic(s, l));
                } else if let Some(k) = Sudden::parse(mark) {
                    push(&|s| Event::Sudden(s, k));
                    if k.then_soft() {
                        push(&|s| Event::Dynamic(s, SOFT));
                    }
                }
            }
        }
    }
}

/// Bars without time (the header before the first bar line) give their label,
/// signature, and events to the next bar.
fn drop_empty(bars: Vec<Bar>) -> Vec<Bar> {
    let mut out: Vec<Bar> = Vec::new();
    let mut carry: Option<Bar> = None;
    for mut b in bars {
        if let Some(c) = carry.take() {
            b.label = b.label.or(c.label);
            let mut events = c.events;
            events.append(&mut b.events);
            b.events = events;
            b.double_before |= c.double_before;
        }
        if b.len == Q::ZERO {
            carry = Some(b);
        } else {
            out.push(b);
        }
    }
    out
}

/// Playing order of the bars. Endings missing from the list (`A2` when the list
/// goes `A, A1, A, B`) are played after the matching repeat of their section.
fn order(bars: &[Bar], expansion: Option<&[String]>) -> Vec<usize> {
    let all = (0..bars.len()).collect();
    let Some(list) = expansion else { return all };
    let mut sections: Vec<(String, Vec<usize>)> = Vec::new();
    let mut lead = Vec::new();
    for (i, b) in bars.iter().enumerate() {
        match (&b.label, sections.last_mut()) {
            (Some(l), _) => sections.push((l.clone(), vec![i])),
            (None, Some(s)) => s.1.push(i),
            (None, None) => lead.push(i),
        }
    }
    let mut list: Vec<String> = list.to_vec();
    for (label, _) in &sections {
        if list.contains(label) {
            continue;
        }
        let base = label.trim_end_matches(|c: char| c.is_ascii_digit());
        let Ok(n) = label[base.len()..].parse::<usize>() else {
            continue;
        };
        if let Some(at) = list
            .iter()
            .enumerate()
            .filter(|(_, l)| *l == base)
            .nth(n - 1)
            .map(|(i, _)| i)
        {
            list.insert(at + 1, label.clone());
        }
    }
    let mut order = lead;
    for l in &list {
        if let Some((_, bars)) = sections.iter().find(|(s, _)| s == l) {
            order.extend(bars);
        }
    }
    if order.is_empty() { all } else { order }
}

/// Length of a bar of `sig`, in quarters.
fn full(sig: (u32, u32)) -> Q {
    Q::new(4 * i64::from(sig.0), i64::from(sig.1))
}

fn place(model: &mut Model, bars: &[Bar], order: &[usize]) {
    let mut start = Q::ZERO;
    let mut prev: Option<usize> = None;
    let mut pending: HashMap<u32, Vec<(u32, i32, String, bool)>> = HashMap::new();
    for &i in order {
        let b = &bars[i];
        let brk = b.double_before || b.label.is_some() || prev.is_some_and(|p| p + 1 != i);
        match model.measures.last_mut() {
            // A bar split by a repeat sign is whole again in playing order
            // (the end of a section and the upbeat that follows it).
            Some(m) if m.len < full(m.sig) && m.len + b.len <= full(m.sig) => m.len = m.len + b.len,
            _ => model.measures.push(Measure {
                number: b.number.clone(),
                start,
                len: b.len,
                sig: b.sig,
                break_before: brk && prev.is_some(),
            }),
        }
        for (at, ev) in &b.events {
            let at = start + (*at - b.start);
            match ev {
                Event::Note {
                    voice,
                    staff,
                    len,
                    midi,
                    name,
                    tie,
                    trill,
                    marks,
                } => {
                    if let Some(gs) = pending.remove(voice) {
                        let mut index = 0;
                        let mut group = Vec::new();
                        for (k, (gstaff, gmidi, gname, chord)) in gs.into_iter().enumerate() {
                            if k > 0 && !chord {
                                index += 1;
                            }
                            group.push(Grace {
                                voice: (0, *voice),
                                staff: (0, gstaff),
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
                        voice: (0, *voice),
                        staff: (0, *staff),
                        start: at,
                        len: *len,
                        midi: *midi,
                        name: name.clone(),
                        tie: *tie,
                        trill: trill.clone(),
                        marks: *marks,
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
                Event::Dynamic(s, l) => model.dynamics.push((at, (0, *s), *l)),
                Event::Sudden(s, k) => model.suddens.push((at, (0, *s), *k)),
                Event::RestHold(len) => model.rest_holds.push((at, *len)),
                Event::Wedge(s, w) => model.wedges.push((at, (0, *s), *w)),
                Event::Pedal(p) => model.pedals.push((at, *p)),
                Event::Tempo(m) => model.tempos.push((at, *m)),
            }
        }
        start = start + b.len;
        prev = Some(i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "!!!OTL: Test\n**kern\t**kern\t**dynam\n*staff2\t*staff1\t*staff1/2\n*M2/4\t*M2/4\t*\n*MM60\t*MM60\t*\n";

    #[test]
    fn tokens() {
        let n = token("4.cc#[").unwrap();
        assert_eq!(n.len, Q::new(3, 2));
        assert_eq!(n.pitch, Some(('C', 1, 5)));
        assert!(n.tie);
        assert_eq!(token("12GG-").unwrap().len, Q::new(1, 3));
        assert_eq!(token("12GG-").unwrap().pitch, Some(('G', -1, 2)));
        assert_eq!(token("3%2r").unwrap().len, Q::new(8, 3));
        let g = token("16eqLL").unwrap();
        assert!(g.grace && g.len == Q::ZERO);
        assert_eq!(token("2aaT;").unwrap().trill, Some(2));
        assert_eq!(upper('A', 0, 5, 2), ("B5".into(), 83));
        assert_eq!(upper('B', 1, 4, 1), ("C#5".into(), 73));
    }

    #[test]
    fn spines_split_and_bars() {
        let k = format!(
            "{HEAD}=1-\t=1-\t=1-\n*\t*^\t*\n\
             4C\t8e\t4g\tp\n.\t8f\t.\t.\n\
             4D\t4e\t4a\t<\n*\t*v\t*v\t*\n=2\t=2\t=2\n\
             2E\t2cc\t[\n==\t==\t==\n"
        );
        let m = parse(&k).unwrap();
        assert_eq!(m.title.as_deref(), Some("Test"));
        assert_eq!(m.measures.len(), 2);
        assert_eq!(m.measures[0].len, Q::int(2));
        assert_eq!(m.measures[0].sig, (2, 4));
        let at = |name: &str| m.notes.iter().find(|n| n.name == name).unwrap();
        assert_eq!(at("F4").start, Q::new(1, 2));
        assert_eq!(at("G4").voice, (0, 12));
        assert_eq!(at("E4").voice, (0, 11));
        assert_eq!(at("C5").start, Q::int(2));
        assert_eq!(at("C5").voice, (0, 11));
        assert_eq!(m.tempos, [(Q::ZERO, 60.0)]);
        // p for both staves, a crescendo, and its end
        assert_eq!(m.dynamics.len(), 2);
        assert_eq!(m.wedges.len(), 4);
    }

    #[test]
    fn expansion_list_with_a_missing_ending() {
        let bar = |n: u32, label: &str, note: &str| {
            let l = if label.is_empty() {
                String::new()
            } else {
                format!("*>{label}\t*>{label}\t*>{label}\n")
            };
            format!("={n}\t={n}\t={n}\n{l}2C\t2{note}\t.\n")
        };
        let k = format!(
            "{HEAD}*>[A,A1,A,B]\t*>[A,A1,A,B]\t*>[A,A1,A,B]\n{}{}{}{}==\t==\t==\n",
            bar(1, "A", "c"),
            bar(2, "A1", "d"),
            bar(3, "A2", "e"),
            bar(4, "B", "f")
        );
        let m = parse(&k).unwrap();
        let order: Vec<&str> = m.measures.iter().map(|b| b.number.as_str()).collect();
        assert_eq!(order, ["1", "2", "1", "3", "4"]);
        assert!(m.measures[2].break_before);
    }

    #[test]
    fn partly_tied_chords_and_split_bars_import_cleanly() {
        // Bb3 + tied Bb4, a rest at a bar line, and a bar split by a repeat sign.
        let k = format!(
            "{HEAD}*>[A,B,B]\t*>[A,B,B]\t*>[A,B,B]\n*>A\t*>A\t*>A\n=1\t=1\t=1\n\
             2C\t4B- [4b-\t.\n.\t4b-]\t.\n=2\t=2\t=2\n2D\t2c\t.\n=3\t=3\t=3\n\
             4E\t4d\t.\n=:|!|:\t=:|!|:\t=:|!|:\n*>B\t*>B\t*>B\n4F\t[4e\t.\n\
             =4\t=4\t=4\n2G\t4e]\t.\n.\t4r\t.\n=5\t=5\t=5\n4A\t4f\t.\n==\t==\t==\n"
        );
        let i = crate::import(&k, "test.krn").unwrap();
        assert!(
            i.check.ok(),
            "{:?}\n{}",
            i.check,
            i.pieces.iter().map(|p| p.1.as_str()).collect::<String>()
        );
    }

    #[test]
    fn articulations_are_written_after_the_notes() {
        // Staccato, staccatissimo, accent, tenuto; a chord is short only if every
        // note is; a note tied on drops its staccato.
        let k = format!(
            "{HEAD}=1\t=1\t=1\n4C'\t4c` 4e`\t.\n4D^\t4d' 4f\t.\n=2\t=2\t=2\n\
             4E~\t[4g'\t.\n4F'~\t4g]'\t.\n==\t==\t==\n"
        );
        let i = crate::import(&k, "test.krn").unwrap();
        let text: String = i.pieces.iter().map(|p| p.1.as_str()).collect();
        assert!(i.check.ok(), "{:?}\n{text}", i.check);
        for want in [
            "C3/4'",
            "{C4 E4}/4''",
            "D3^",
            "{D4 F4} ",
            "E3/4_",
            "F3'_",
            "G4/4~",
            "G4'",
        ] {
            assert!(text.contains(want), "{want} in\n{text}");
        }
    }

    #[test]
    fn sforzandos_are_written_before_their_notes() {
        // `z` on a note; `sf` and `fp` in the dynamics, `fp` going on soft.
        let k = format!(
            "{HEAD}=1\t=1\t=1\n4C\t4cz\t.\n4D\t4d\tsf\n=2\t=2\t=2\n\
             4E\t4e\tfp\n4F\t4f\t.\n==\t==\t==\n"
        );
        let i = crate::import(&k, "test.krn").unwrap();
        let text: String = i.pieces.iter().map(|p| p.1.as_str()).collect();
        assert!(i.check.ok(), "{:?}\n{text}", i.check);
        for want in [
            "mf sf C4/4 sf D4",
            "mf C3/4 sf D3",
            "p fp E4/4 F4",
            "p fp E3/4 F3",
        ] {
            assert!(text.contains(want), "{want} in\n{text}");
        }
    }

    #[test]
    fn fermatas_on_notes_and_rests() {
        let k = format!(
            "{HEAD}=1\t=1\t=1\n2C;\t2c;\t.\n=2\t=2\t=2\n2r\t4d\t.\n.\t4r;\t.\n\
             =3\t=3\t=3\n2r;\t2r;\t.\n==\t==\t==\n"
        );
        let i = crate::import(&k, "test.krn").unwrap();
        let text: String = i.pieces.iter().map(|p| p.1.as_str()).collect();
        assert!(i.check.ok(), "{:?}\n{text}", i.check);
        for want in ["C4/2@", "C3/2@", "D4/4 r@", "| r/2@ |"] {
            assert!(text.contains(want), "{want} in\n{text}");
        }
    }

    #[test]
    fn graces_ties_and_pickup() {
        let k = format!(
            "{HEAD}4C\t4c\t.\n=1\t=1\t=1\n.\t16dq\t.\n2D\t2e[\t.\n=2\t=2\t=2\n2E\t2e]\t.\n==\t==\t==\n"
        );
        let m = parse(&k).unwrap();
        assert_eq!(m.measures[0].number, "0");
        assert_eq!(m.measures[0].len, Q::int(1));
        assert_eq!(m.graces.len(), 1);
        assert_eq!(m.graces[0].principal, Q::int(1));
        let e = m.notes.iter().find(|n| n.name == "E4").unwrap();
        assert!(e.tie);
    }

    #[test]
    fn a_bar_no_signature_can_write_goes_in_free() {
        // 4 1/8 quarters in 2/4: 33/32 has more than 32 beats.
        let k = format!(
            "{HEAD}=1\t=1\t=1\n2C\t2c\t.\n=2\t=2\t=2\n1D\t1d\t.\n32E\t32e\t.\n\
             =3\t=3\t=3\n2F\t2f\t.\n==\t==\t==\n"
        );
        let i = crate::import(&k, "test.krn").unwrap();
        let text: String = i.pieces.iter().map(|p| p.1.as_str()).collect();
        assert!(i.check.ok(), "{:?}\n{text}", i.check);
        assert!(text.contains("free{"), "{text}");
    }

    #[test]
    fn a_grace_does_not_cut_the_note_held_in_its_column() {
        // B4 sounds 0-1.5; the grace line in its column must not end it at 1.
        let k = format!(
            "{HEAD}=1\t=1\t=1\n8c\t4.b\t.\n8d\t.\t.\n.\t16aq\t.\n8e\t.\t.\n\
             8f\t8a\t.\n==\t==\t==\n"
        );
        let m = parse(&k).unwrap();
        let at = |name: &str| m.notes.iter().find(|n| n.name == name).unwrap().start;
        assert_eq!(at("E4"), Q::int(1));
        assert_eq!(at("A4"), Q::new(3, 2));
        assert_eq!(m.measures[0].len, Q::int(2));
    }
}
