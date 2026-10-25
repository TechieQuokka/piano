//! The language manual (`man`), parsed from `docs/LANGUAGE(.ko).md`.

use piano_project::Lang;

const EN: &str = include_str!("../../../docs/LANGUAGE.md");
const KO: &str = include_str!("../../../docs/LANGUAGE.ko.md");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub id: String,
    pub title: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manual {
    pub chapters: Vec<Chapter>,
}

/// `TITLE {#id}` → (title, id).
pub(crate) fn heading(rest: &str) -> Option<(String, String)> {
    let (title, id) = rest.split_once("{#")?;
    Some((
        title.trim().to_string(),
        id.trim_end_matches('}').trim().to_string(),
    ))
}

/// Parses `## chapter` / `### section` headings; indented lines are the content.
pub fn parse(text: &str) -> Manual {
    let mut chapters: Vec<Chapter> = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("### ") {
            if let (Some((title, id)), Some(ch)) = (heading(rest), chapters.last_mut()) {
                ch.sections.push(Section {
                    id,
                    title,
                    lines: Vec::new(),
                });
            }
        } else if let Some(rest) = line.strip_prefix("## ") {
            if let Some((title, id)) = heading(rest) {
                chapters.push(Chapter {
                    id,
                    title,
                    sections: Vec::new(),
                });
            }
        } else if let Some(content) = line.strip_prefix("    ")
            && let Some(sec) = chapters.last_mut().and_then(|c| c.sections.last_mut())
        {
            sec.lines.push(content.to_string());
        }
    }
    Manual { chapters }
}

pub fn manual(lang: Lang) -> Manual {
    parse(match lang {
        Lang::En => EN,
        Lang::Ko => KO,
    })
}

impl Manual {
    pub fn sections(&self) -> impl Iterator<Item = (&Chapter, &Section)> {
        self.chapters
            .iter()
            .flat_map(|c| c.sections.iter().map(move |s| (c, s)))
    }

    /// `man`, `man notes`, `man octave`, `man fn accent` (= `fn.accent`).
    pub fn lookup(&self, args: &[&str]) -> Option<(String, Vec<String>)> {
        if args.is_empty() {
            let mut out = Vec::new();
            for c in &self.chapters {
                let ids: Vec<&str> = c.sections.iter().map(|s| s.id.as_str()).collect();
                out.push(format!("{:<12} {}", c.id, ids.join(" ")));
            }
            out.push(String::new());
            out.push(
                "Type 'man <chapter>' or 'man <section>', e.g. man octave, man fn accent.".into(),
            );
            return Some(("MANUAL".into(), out));
        }
        let key = args.join(".").to_ascii_lowercase();
        if let Some((_, s)) = self.sections().find(|(_, s)| s.id == key) {
            let mut out = vec![s.title.clone()];
            out.extend(s.lines.iter().map(|l| format!("  {l}")));
            return Some((s.id.clone(), out));
        }
        let c = self.chapters.iter().find(|c| c.id == key)?;
        let mut out = Vec::new();
        for s in &c.sections {
            out.push(s.title.clone());
            out.extend(s.lines.iter().map(|l| format!("  {l}")));
            out.push(String::new());
        }
        Some((c.title.clone(), out))
    }
}

/// The `Example` code of a section (text before `→`).
pub fn example(section: &Section) -> Option<String> {
    section.lines.iter().find_map(|l| {
        let rest = l.strip_prefix("Example")?;
        let code = rest.split('→').next()?.trim();
        Some(code.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use piano_core::Severity;

    fn ids(m: &Manual) -> Vec<String> {
        m.chapters
            .iter()
            .flat_map(|c| {
                std::iter::once(c.id.clone()).chain(c.sections.iter().map(|s| s.id.clone()))
            })
            .collect()
    }

    #[test]
    fn same_ids_in_both_languages() {
        let (en, ko) = (manual(Lang::En), manual(Lang::Ko));
        assert!(!en.chapters.is_empty());
        assert_eq!(ids(&en), ids(&ko));
    }

    #[test]
    fn same_examples_in_both_languages() {
        let (en, ko) = (manual(Lang::En), manual(Lang::Ko));
        for ((_, a), (_, b)) in en.sections().zip(ko.sections()) {
            assert_eq!(example(a), example(b), "{}", a.id);
        }
    }

    #[test]
    fn every_section_has_one_example_that_compiles() {
        for (chapter, s) in manual(Lang::En).sections() {
            let examples = s.lines.iter().filter(|l| l.starts_with("Example")).count();
            assert_eq!(examples, 1, "{}", s.id);
            let code = example(s).unwrap();
            let compiled = if chapter.id == "song" {
                piano_lang::compile_song(&code, &|_| Some("C D".into()), &Default::default())
            } else {
                piano_lang::compile_piece(&code)
            };
            let errors: Vec<_> = compiled
                .alerts
                .iter()
                .filter(|a| a.severity == Severity::Error)
                .collect();
            assert!(errors.is_empty(), "{}: {code}: {errors:?}", s.id);
            assert!(!compiled.score.notes.is_empty(), "{}: {code}", s.id);
        }
    }

    #[test]
    fn lookup() {
        let m = manual(Lang::En);
        let (_, octave) = m.lookup(&["octave"]).unwrap();
        assert_eq!(octave[0], "OCTAVE");
        assert!(m.lookup(&["fn", "accent"]).is_some());
        assert!(m.lookup(&["notes"]).is_some());
        assert!(m.lookup(&["nope"]).is_none());
        assert!(m.lookup(&[]).is_some());
    }

    #[test]
    fn every_builtin_has_a_section() {
        let m = manual(Lang::En);
        for b in piano_lang::BUILTINS {
            let id = match b.name {
                "softer" => "louder",
                n => n,
            };
            assert!(m.lookup(&["fn", id]).is_some(), "{}", b.name);
        }
    }
}
