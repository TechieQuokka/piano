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
                });
            }
        } else if let Some(content) = line.strip_prefix("    ")
            && let Some(step) = steps.last_mut()
        {
            match content.strip_prefix("Expect") {
                Some(code) => step.expect = Some(code.trim().to_string()),
                None => step.lines.push(content.to_string()),
            }
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
                .map(|x| (x.id.clone(), x.expect.clone()))
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

    #[test]
    fn the_try_text_satisfies_its_step() {
        // Steps whose Try line is code to type must pass when typed as-is.
        for s in steps(Lang::En) {
            let (Some(expect), Some(try_line)) = (
                &s.expect,
                s.lines.iter().find_map(|l| l.strip_prefix("Try")),
            ) else {
                continue;
            };
            let Some(code) = try_line.trim().strip_prefix("Type ") else {
                continue;
            };
            let code = code.trim_end_matches('.');
            let score = piano_lang::compile_piece(code).score;
            assert!(satisfied(&score, expect), "{}: {code}", s.id);
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
