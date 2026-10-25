//! The detailed language guide (`guide`), parsed from `docs/GUIDE(.ko).md`.
//!
//! Plain Markdown: `## N. Title {#id}` starts a chapter, `### Title {#id}` a section,
//! `#### Name` a part of a section. Paragraphs, `- ` lists, and fenced code blocks are
//! shown in the Display; inline `**` and backticks are dropped there.

use piano_core::Severity;
use piano_project::Lang;

use crate::manual::heading;

const EN: &str = include_str!("../../../docs/GUIDE.md");
const KO: &str = include_str!("../../../docs/GUIDE.ko.md");

/// What a fenced code block holds, from its info string (` ```piano warning `).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// ` ```piano [error|warning|hint] `: piece code; the alert it must raise.
    Piece(Option<Severity>),
    /// ` ```song [error|warning|hint] `: song code.
    Song(Option<Severity>),
    /// Any other block (`text`, `toml`): shown, not compiled.
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    pub kind: BlockKind,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub id: String,
    pub title: String,
    pub lines: Vec<String>,
    pub blocks: Vec<CodeBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    pub id: String,
    /// e.g. `2. Pitch`.
    pub title: String,
    /// Lines before the first section.
    pub intro: Vec<String>,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guide {
    pub chapters: Vec<Chapter>,
}

/// A page of the guide: the contents, a chapter, or one section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GuidePos {
    #[default]
    Contents,
    Chapter(usize),
    Section(usize, usize),
}

fn block_kind(info: &str) -> BlockKind {
    let mut words = info.split_whitespace();
    let lang = words.next().unwrap_or("");
    let severity = match words.next() {
        Some("error") => Some(Severity::Error),
        Some("warning") => Some(Severity::Warning),
        Some("hint") => Some(Severity::Hint),
        _ => None,
    };
    match lang {
        "piano" => BlockKind::Piece(severity),
        "song" => BlockKind::Song(severity),
        _ => BlockKind::Other,
    }
}

/// Drops inline Markdown that reads badly in a terminal.
fn inline(text: &str) -> String {
    text.replace("**", "").replace('`', "")
}

#[derive(Default)]
struct Builder {
    chapters: Vec<Chapter>,
    /// The paragraph or list item being collected.
    para: Option<String>,
    fence: Option<CodeBlock>,
}

impl Builder {
    fn lines(&mut self) -> Option<&mut Vec<String>> {
        let ch = self.chapters.last_mut()?;
        Some(match ch.sections.last_mut() {
            Some(s) => &mut s.lines,
            None => &mut ch.intro,
        })
    }

    fn push(&mut self, line: String) {
        if let Some(lines) = self.lines() {
            lines.push(line);
        }
    }

    /// A blank line, kept once and never leading or right after a `▸` heading.
    fn gap(&mut self) {
        self.flush();
        if let Some(lines) = self.lines()
            && lines
                .last()
                .is_some_and(|l| !l.is_empty() && !l.starts_with('▸'))
        {
            lines.push(String::new());
        }
    }

    fn flush(&mut self) {
        if let Some(p) = self.para.take() {
            self.push(inline(&p));
        }
    }

    fn line(&mut self, line: &str) {
        if let Some(block) = &mut self.fence {
            if line.trim_start().starts_with("```") {
                let block = self.fence.take().unwrap();
                if let Some(s) = self.chapters.last_mut().and_then(|c| c.sections.last_mut()) {
                    s.blocks.push(block);
                }
            } else {
                block.code.push_str(line);
                block.code.push('\n');
                self.push(format!("  │ {line}"));
            }
            return;
        }
        if let Some(rest) = line.strip_prefix("```") {
            self.flush();
            self.fence = Some(CodeBlock {
                kind: block_kind(rest),
                code: String::new(),
            });
        } else if let Some(rest) = line.strip_prefix("#### ") {
            self.gap();
            self.push(format!("▸ {}", inline(rest.trim())));
        } else if let Some(rest) = line.strip_prefix("### ") {
            self.flush();
            if let (Some((title, id)), Some(ch)) = (heading(rest), self.chapters.last_mut()) {
                ch.sections.push(Section {
                    id,
                    title,
                    lines: Vec::new(),
                    blocks: Vec::new(),
                });
            }
        } else if let Some(rest) = line.strip_prefix("## ") {
            self.flush();
            if let Some((title, id)) = heading(rest) {
                self.chapters.push(Chapter {
                    id,
                    title,
                    intro: Vec::new(),
                    sections: Vec::new(),
                });
            }
        } else if line.starts_with('#') {
            // The document title.
        } else if line.trim().is_empty() {
            self.gap();
        } else if let Some(item) = line.strip_prefix("- ") {
            self.flush();
            self.para = Some(format!("•  {}", item.trim()));
        } else {
            let text = line.trim();
            match &mut self.para {
                Some(p) => {
                    p.push(' ');
                    p.push_str(text);
                }
                None => self.para = Some(text.to_string()),
            }
        }
    }
}

pub fn parse(text: &str) -> Guide {
    let mut b = Builder::default();
    for line in text.lines() {
        b.line(line);
    }
    b.flush();
    for ch in &mut b.chapters {
        for lines in
            std::iter::once(&mut ch.intro).chain(ch.sections.iter_mut().map(|s| &mut s.lines))
        {
            while lines.last().is_some_and(|l| l.is_empty()) {
                lines.pop();
            }
        }
    }
    Guide {
        chapters: b.chapters,
    }
}

pub fn guide(lang: Lang) -> Guide {
    parse(match lang {
        Lang::En => EN,
        Lang::Ko => KO,
    })
}

/// `2. Pitch` → `CHAPTER 2 · PITCH`.
fn chapter_heading(title: &str) -> String {
    match title.split_once(". ") {
        Some((n, name)) if n.chars().all(|c| c.is_ascii_digit()) => {
            format!("CHAPTER {n} · {}", name.to_uppercase())
        }
        _ => title.to_uppercase(),
    }
}

impl Guide {
    pub fn sections(&self) -> impl Iterator<Item = (&Chapter, &Section)> {
        self.chapters
            .iter()
            .flat_map(|c| c.sections.iter().map(move |s| (c, s)))
    }

    /// `guide`, `guide 3`, `guide rhythm`, `guide octave`, `guide fn up` (= `fn.up`).
    pub fn find(&self, args: &[&str]) -> Option<GuidePos> {
        if args.is_empty() {
            return Some(GuidePos::Contents);
        }
        let key = args.join(".").to_ascii_lowercase();
        if let Ok(n) = key.parse::<usize>() {
            return (1..=self.chapters.len())
                .contains(&n)
                .then_some(GuidePos::Chapter(n - 1));
        }
        if let Some(c) = self.chapters.iter().position(|c| c.id == key) {
            return Some(GuidePos::Chapter(c));
        }
        self.chapters.iter().enumerate().find_map(|(c, ch)| {
            ch.sections
                .iter()
                .position(|s| s.id == key)
                .map(|s| GuidePos::Section(c, s))
        })
    }

    /// The next section in reading order (from the contents: the first one).
    pub fn next(&self, pos: GuidePos) -> GuidePos {
        let flat: Vec<(usize, usize)> = self.flat();
        let target = match pos {
            GuidePos::Contents => flat.first(),
            GuidePos::Chapter(c) => flat.iter().find(|(fc, _)| *fc == c),
            GuidePos::Section(c, s) => {
                let i = flat.iter().position(|&p| p == (c, s)).unwrap_or(0);
                flat.get(i + 1).or(flat.last())
            }
        };
        target.map_or(pos, |&(c, s)| GuidePos::Section(c, s))
    }

    /// The previous section (from a chapter: the last section of the one before).
    pub fn prev(&self, pos: GuidePos) -> GuidePos {
        let flat: Vec<(usize, usize)> = self.flat();
        let before = match pos {
            GuidePos::Contents => return pos,
            GuidePos::Chapter(c) => flat.iter().position(|(fc, _)| *fc == c).unwrap_or(0),
            GuidePos::Section(c, s) => flat.iter().position(|&p| p == (c, s)).unwrap_or(0),
        };
        match before.checked_sub(1).and_then(|i| flat.get(i)) {
            Some(&(c, s)) => GuidePos::Section(c, s),
            None => GuidePos::Contents,
        }
    }

    fn flat(&self) -> Vec<(usize, usize)> {
        self.chapters
            .iter()
            .enumerate()
            .flat_map(|(c, ch)| (0..ch.sections.len()).map(move |s| (c, s)))
            .collect()
    }

    /// The page title (`GUIDE · octave`) and its lines.
    pub fn page(&self, pos: GuidePos) -> (String, Vec<String>) {
        match pos {
            GuidePos::Contents => {
                let mut out = Vec::new();
                for (i, c) in self.chapters.iter().enumerate() {
                    out.push(chapter_heading(&c.title));
                    let ids: Vec<&str> = c.sections.iter().map(|s| s.id.as_str()).collect();
                    out.push(format!("  guide {:<12} {}", i + 1, ids.join("  ")));
                }
                out.push(String::new());
                out.push("Type 'guide <n>' for a chapter, 'guide <section>' for one topic,".into());
                out.push("'guide next' / 'guide prev' to read on. Short reference: man.".into());
                ("GUIDE".into(), out)
            }
            GuidePos::Chapter(c) => {
                let ch = &self.chapters[c];
                let mut out = vec![chapter_heading(&ch.title)];
                out.extend(ch.intro.iter().cloned());
                for s in &ch.sections {
                    out.push(String::new());
                    out.push(s.title.to_uppercase());
                    out.extend(s.lines.iter().cloned());
                }
                out.push(String::new());
                out.push("guide next  first section · guide  contents".into());
                (format!("GUIDE · {}", ch.id), out)
            }
            GuidePos::Section(c, s) => {
                let ch = &self.chapters[c];
                let sec = &ch.sections[s];
                let mut out = vec![
                    format!(
                        "{} › {}",
                        chapter_heading(&ch.title),
                        sec.title.to_uppercase()
                    ),
                    String::new(),
                ];
                out.extend(sec.lines.iter().cloned());
                out.push(String::new());
                let name = |p: GuidePos| match p {
                    GuidePos::Section(c, s) => self.chapters[c].sections[s].id.clone(),
                    _ => "contents".into(),
                };
                let (prev, next) = (self.prev(pos), self.next(pos));
                let mut nav = format!("← guide prev ({})", name(prev));
                if next != pos {
                    nav.push_str(&format!(" · guide next ({}) →", name(next)));
                }
                out.push(nav);
                (format!("GUIDE · {}", sec.id), out)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(g: &Guide) -> Vec<String> {
        g.chapters
            .iter()
            .flat_map(|c| {
                std::iter::once(c.id.clone()).chain(c.sections.iter().map(|s| s.id.clone()))
            })
            .collect()
    }

    /// Code with `//` comments removed (comments may be translated; `text` and
    /// `toml` blocks may differ entirely).
    fn code_only(code: &str) -> String {
        code.lines()
            .map(|l| l.split("//").next().unwrap_or("").trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn parses_markdown() {
        let g = parse(
            "# T\n\n## 1. One {#one}\n\nIntro\ntext.\n\n### Part {#part}\n\n#### Rules\n\
             - item **one**\n- `two`\n\n```piano\nC D // hi\n```\n",
        );
        let ch = &g.chapters[0];
        assert_eq!(ch.intro, ["Intro text."]);
        let s = &ch.sections[0];
        assert_eq!(
            s.lines,
            ["▸ Rules", "•  item one", "•  two", "", "  │ C D // hi"]
        );
        assert_eq!(s.blocks[0].kind, BlockKind::Piece(None));
        assert_eq!(chapter_heading("2. Pitch"), "CHAPTER 2 · PITCH");
    }

    #[test]
    fn same_structure_in_both_languages() {
        let (en, ko) = (guide(Lang::En), guide(Lang::Ko));
        assert!(en.chapters.len() >= 10);
        assert_eq!(ids(&en), ids(&ko));
        let mut all = ids(&en);
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n, "ids are unique");
    }

    #[test]
    fn same_code_in_both_languages() {
        let (en, ko) = (guide(Lang::En), guide(Lang::Ko));
        for ((_, a), (_, b)) in en.sections().zip(ko.sections()) {
            assert_eq!(a.blocks.len(), b.blocks.len(), "{}", a.id);
            for (x, y) in a.blocks.iter().zip(&b.blocks) {
                assert_eq!(x.kind, y.kind, "{}", a.id);
                if x.kind != BlockKind::Other {
                    assert_eq!(code_only(&x.code), code_only(&y.code), "{}", a.id);
                }
            }
        }
    }

    #[test]
    fn every_example_does_what_it_says() {
        let mut failures = Vec::new();
        for lang in [Lang::En, Lang::Ko] {
            for (_, s) in guide(lang).sections() {
                for b in &s.blocks {
                    let (compiled, expect) = match b.kind {
                        BlockKind::Piece(e) => (piano_lang::compile_piece(&b.code), e),
                        BlockKind::Song(e) => (
                            piano_lang::compile_song(
                                &b.code,
                                &|_| Some("C4 D E F".into()),
                                &Default::default(),
                            ),
                            e,
                        ),
                        BlockKind::Other => continue,
                    };
                    let has = |sev| compiled.alerts.iter().any(|a| a.severity == sev);
                    let ok = match expect {
                        None => !has(Severity::Error) && !has(Severity::Warning),
                        Some(sev) => has(sev),
                    };
                    if !ok {
                        failures.push(format!(
                            "{}: {}expected {expect:?}, got {:?}",
                            s.id, b.code, compiled.alerts
                        ));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    }

    #[test]
    fn every_manual_section_has_a_guide_section() {
        let g = guide(Lang::En);
        for (_, s) in crate::manual::manual(Lang::En).sections() {
            assert!(
                matches!(g.find(&[&s.id]), Some(GuidePos::Section(..))),
                "guide has no section `{}`",
                s.id
            );
        }
    }

    /// Every `` `guide x` `` and `` `man x` `` in the text names a real page.
    #[test]
    fn references_resolve() {
        let man = crate::manual::manual(Lang::En);
        let g = guide(Lang::En);
        for text in [EN, KO] {
            for (i, _) in text.match_indices('`') {
                let rest = &text[i + 1..];
                let Some(end) = rest.find('`') else { continue };
                let words: Vec<&str> = rest[..end].split_whitespace().collect();
                match words.as_slice() {
                    ["guide", "next" | "prev"] | ["guide"] | ["man"] => {}
                    ["guide", args @ ..] if !args[0].starts_with('<') => {
                        assert!(g.find(args).is_some(), "`guide {}`", args.join(" "))
                    }
                    ["man", args @ ..] if !args[0].starts_with('<') => {
                        assert!(man.lookup(args).is_some(), "`man {}`", args.join(" "))
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn lookup_and_paging() {
        let g = guide(Lang::En);
        assert_eq!(g.find(&[]), Some(GuidePos::Contents));
        assert_eq!(g.find(&["2"]), Some(GuidePos::Chapter(1)));
        assert_eq!(g.find(&["99"]), None);
        assert!(matches!(g.find(&["octave"]), Some(GuidePos::Section(..))));
        assert!(matches!(g.find(&["fn", "up"]), Some(GuidePos::Section(..))));
        assert_eq!(g.find(&["nope"]), None);
        let first = g.next(GuidePos::Contents);
        assert_eq!(first, GuidePos::Section(0, 0));
        assert_eq!(g.prev(first), GuidePos::Contents);
        assert_eq!(g.next(GuidePos::Chapter(1)), GuidePos::Section(1, 0));
        let mut pos = first;
        let total: usize = g.chapters.iter().map(|c| c.sections.len()).sum();
        for _ in 1..total {
            pos = g.next(pos);
        }
        let last = GuidePos::Section(
            g.chapters.len() - 1,
            g.chapters.last().unwrap().sections.len() - 1,
        );
        assert_eq!(pos, last);
        assert_eq!(g.next(last), last);
        let (title, lines) = g.page(GuidePos::Section(1, 0));
        assert!(title.starts_with("GUIDE · "));
        assert!(lines[0].starts_with("CHAPTER 2 · "));
    }
}
