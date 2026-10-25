//! Step-by-step tutorial with light checking, parsed from `docs/TUTORIAL(.ko).md`.

use piano_core::Score;
use piano_project::Lang;

use crate::manual::heading;

const EN: &str = include_str!("../../../docs/TUTORIAL.md");
const KO: &str = include_str!("../../../docs/TUTORIAL.ko.md");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub id: String,
    pub title: String,
    /// Shown lines (without `Expect`).
    pub lines: Vec<String>,
    /// Code whose notes must appear in the piece.
    pub expect: Option<String>,
    /// `Piece <name>`: the piece the step's code goes into (default: unchanged).
    pub piece: Option<String>,
    /// `Type <code>` / `Line <n> <code>`: what the reader writes, in order.
    pub edits: Vec<Edit>,
}

/// A change the reader makes to the piece (shown, and replayed by the tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// `Type <code>`: the code on a new line at the end.
    Append(String),
    /// `Line <n> <code>`: line `n` (1-based) becomes the code.
    Replace(usize, String),
}

fn field<'a>(content: &'a str, name: &str) -> Option<&'a str> {
    let rest = content.strip_prefix(name)?;
    rest.starts_with(' ').then(|| rest.trim())
}

fn edit(content: &str) -> Option<Edit> {
    if let Some(code) = field(content, "Type") {
        return Some(Edit::Append(code.to_string()));
    }
    let rest = field(content, "Line")?;
    let (n, code) = rest.split_once(' ')?;
    Some(Edit::Replace(n.parse().ok()?, code.trim().to_string()))
}

pub fn parse(text: &str) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            if let Some((title, id)) = heading(rest) {
                steps.push(Step {
                    id,
                    title,
                    lines: Vec::new(),
                    expect: None,
                    piece: None,
                    edits: Vec::new(),
                });
            }
        } else if line.trim().is_empty() {
            // A blank line separates paragraphs (kept once, never leading).
            if let Some(step) = steps.last_mut()
                && step.lines.last().is_some_and(|l| !l.is_empty())
            {
                step.lines.push(String::new());
            }
        } else if let Some(content) = line.strip_prefix("    ")
            && let Some(step) = steps.last_mut()
        {
            if let Some(code) = field(content, "Expect") {
                step.expect = Some(code.to_string());
                continue;
            }
            if let Some(name) = field(content, "Piece") {
                step.piece = Some(name.to_string());
            } else if let Some(e) = edit(content) {
                step.edits.push(e);
            }
            step.lines.push(content.to_string());
        }
    }
    for step in &mut steps {
        while step.lines.last().is_some_and(String::is_empty) {
            step.lines.pop();
        }
    }
    steps
}

pub fn steps(lang: Lang) -> Vec<Step> {
    parse(match lang {
        Lang::En => EN,
        Lang::Ko => KO,
    })
}

/// `true` if the notes of `expect` appear consecutively in `score` with the same
/// intervals, lengths, and relative timing. Light check: the octave may differ,
/// since earlier steps in the same piece move the octave reference.
pub fn satisfied(score: &Score, expect: &str) -> bool {
    let pattern = piano_lang::compile_piece(expect).score.notes;
    let Some(first) = pattern.first() else {
        return false;
    };
    let interval =
        |a: piano_core::Pitch, b: piano_core::Pitch| i32::from(a.midi()) - i32::from(b.midi());
    let notes = &score.notes;
    (0..notes.len()).any(|i| {
        let (base, root) = (notes[i].start, notes[i].pitch);
        pattern.iter().enumerate().all(|(k, p)| {
            notes.get(i + k).is_some_and(|n| {
                interval(n.pitch, root) == interval(p.pitch, first.pitch)
                    && n.len == p.len
                    && n.start - base == p.start - first.start
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_languages_match() {
        let (en, ko) = (steps(Lang::En), steps(Lang::Ko));
        assert!(en.len() > 5);
        let ids = |s: &[Step]| {
            s.iter()
                .map(|x| (x.id.clone(), x.expect.clone(), x.piece.clone(), x.edits.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&en), ids(&ko));
    }

    #[test]
    fn expectations_compile_cleanly() {
        for s in steps(Lang::En) {
            if let Some(e) = &s.expect {
                let c = piano_lang::compile_piece(e);
                assert!(c.alerts.is_empty(), "{}: {:?}", s.id, c.alerts);
                assert!(!c.score.notes.is_empty());
            }
        }
    }

    /// Follows the whole tutorial as a reader would, from a new project
    /// (`p1` = `// p1`, `song` = `p1`): every step with an `Expect` must be done
    /// right after its edits, and every piece must end up without alerts.
    #[test]
    fn following_the_tutorial_completes_every_step() {
        let mut pieces: Vec<(String, Vec<String>)> = vec![
            ("p1".into(), vec!["// p1".into()]),
            ("song".into(), vec!["p1".into()]),
        ];
        let mut current = "p1".to_string();
        let mut checked = 0;
        for s in steps(Lang::En) {
            if let Some(p) = &s.piece {
                current = p.clone();
                if !pieces.iter().any(|(n, _)| *n == current) {
                    pieces.push((current.clone(), vec![String::new()]));
                }
            }
            let lines = &mut pieces.iter_mut().find(|(n, _)| *n == current).unwrap().1;
            for e in &s.edits {
                match e {
                    Edit::Append(code) if lines.len() == 1 && lines[0].is_empty() => {
                        lines[0] = code.clone();
                    }
                    Edit::Append(code) => lines.push(code.clone()),
                    Edit::Replace(n, code) => {
                        assert!(*n >= 1 && *n <= lines.len(), "{}: no line {n}", s.id);
                        lines[n - 1] = code.clone();
                    }
                }
            }
            if let Some(expect) = &s.expect {
                let score = piano_lang::compile_piece(&lines.join("\n")).score;
                assert!(satisfied(&score, expect), "{}: {lines:?}", s.id);
                checked += 1;
            }
        }
        assert!(checked >= 4, "most writing steps are checked");
        let source = |name: &str| {
            pieces
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, l)| l.join("\n"))
        };
        for (name, lines) in &pieces {
            let text = lines.join("\n");
            let c = if name == "song" {
                let load = |n: &str| source(n);
                piano_lang::song::compile_song(&text, &load, &Default::default())
            } else {
                piano_lang::compile_piece(&text)
            };
            assert!(c.alerts.is_empty(), "{name}: {:?} {lines:?}", c.alerts);
            assert!(!c.score.notes.is_empty(), "{name} has notes");
        }
    }

    #[test]
    fn manual_sections_named_in_hints_exist() {
        for lang in [Lang::En, Lang::Ko] {
            let m = crate::manual::manual(lang);
            for s in steps(lang) {
                for l in s.lines.iter().filter(|l| l.starts_with("Hint")) {
                    let words: Vec<&str> = l
                        .split(|c: char| c == ',' || c == '.' || c == ';' || c.is_whitespace())
                        .collect();
                    for w in words.windows(2).filter(|w| w[0] == "man") {
                        assert!(m.lookup(&[w[1]]).is_some(), "{}: man {}", s.id, w[1]);
                    }
                }
            }
        }
    }

    #[test]
    fn matching() {
        let score = piano_lang::compile_piece("G3 C4/8 D E F r G").score;
        assert!(satisfied(&score, "C4/8 D E F"));
        assert!(!satisfied(&score, "C4 D E F"));
        assert!(!satisfied(&score, "C4/8 D E F G"));
    }
}
