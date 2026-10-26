//! Imports MusicXML scores (`.mxl`, `.musicxml`, `.xml`) and Humdrum `**kern`
//! scores (`.krn`) as piano-language pieces.
//!
//! ```text
//! file ─▶ musicxml | kern (model in playing order) ─▶ write (pieces, song) ─▶ check
//! ```
//!
//! The result is text only; the caller writes the files. Every import is checked:
//! the pieces are compiled and their notes compared with the score.

pub mod check;
pub mod kern;
pub mod musicxml;
pub mod rational;
pub mod write;

use std::io::Read;
use std::path::Path;

pub use check::Check;
pub use musicxml::Notices;

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("mxl: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("xml: {0}")]
    Xml(String),
    #[error("{0}")]
    Format(String),
}

/// An imported score, ready to be written as a project.
#[derive(Debug, Clone)]
pub struct Import {
    pub title: Option<String>,
    /// (piece name, source), in playing order.
    pub pieces: Vec<(String, String)>,
    /// `song.piano`: the pieces in order.
    pub song: String,
    /// First time signature, for `piano.toml`.
    pub time: (u32, u32),
    pub stats: Stats,
    pub check: Check,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    pub measures: usize,
    pub voices: usize,
    pub blocks: usize,
    pub tempos: usize,
    pub graces: usize,
    pub trills: usize,
    pub pedals: usize,
    pub notices: Notices,
}

/// Reads a `.mxl` (compressed) file, or a plain MusicXML or kern file.
pub fn read_file(path: &Path) -> Result<String, ImportError> {
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(b"PK") {
        return read_mxl(&bytes);
    }
    String::from_utf8(bytes).map_err(|e| ImportError::Xml(e.to_string()))
}

/// The score inside a `.mxl`: the rootfile named in `META-INF/container.xml`, or
/// the first `.xml` / `.musicxml` file outside `META-INF`.
fn read_mxl(bytes: &[u8]) -> Result<String, ImportError> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    let mut entry = |name: &str| -> Result<String, ImportError> {
        let mut s = String::new();
        zip.by_name(name)?.read_to_string(&mut s)?;
        Ok(s)
    };
    let root = entry("META-INF/container.xml").ok().and_then(|c| {
        let start = c.find("full-path=\"")? + "full-path=\"".len();
        let end = c[start..].find('"')? + start;
        Some(c[start..end].to_string())
    });
    if let Some(r) = root {
        return entry(&r);
    }
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        let name = f.name()?.to_string();
        if !name.starts_with("META-INF") && (name.ends_with(".xml") || name.ends_with(".musicxml"))
        {
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            return Ok(s);
        }
    }
    Err(ImportError::Format("no score in the .mxl".into()))
}

/// A title on one line: line breaks, real or written `\n` (kern), become ` — `.
fn one_line(title: &str) -> String {
    title
        .replace("\\n", "\n")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" — ")
}

/// Imports MusicXML or kern text. `source` names the file in the piece comments.
pub fn import(text: &str, source: &str) -> Result<Import, ImportError> {
    let mut model = if text
        .trim_start_matches('\u{feff}')
        .trim_start()
        .starts_with('<')
    {
        musicxml::parse(text)?
    } else if text.lines().any(|l| l.split('\t').any(|t| t == "**kern")) {
        kern::parse(text)?
    } else {
        return Err(ImportError::Format("not a MusicXML or **kern score".into()));
    };
    model.title = model.title.map(|t| one_line(&t));
    let written = write::write(&model, source)?;
    let check = check::check(&written.pieces, &written.song, &written.expected);
    let voices = model
        .notes
        .iter()
        .map(|n| n.voice)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    Ok(Import {
        title: model.title.clone(),
        time: model.measures.first().map_or((4, 4), |m| m.sig),
        stats: Stats {
            measures: model.measures.len(),
            voices,
            blocks: written.blocks,
            tempos: written.tempos,
            graces: written.graces,
            trills: written.trills,
            pedals: model.pedals.len(),
            notices: model.notices.clone(),
        },
        pieces: written.pieces,
        song: written.song,
        check,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::musicxml::tests::{ATTRS, note, score};

    fn imported(measures: &str) -> Import {
        let i = import(&score(measures), "test.musicxml").unwrap();
        assert!(
            i.check.ok(),
            "{:?}\n{}",
            i.check,
            i.pieces.iter().map(|p| p.1.as_str()).collect::<String>()
        );
        i
    }

    #[test]
    fn melody_over_a_bass() {
        let m = format!(
            r#"<measure number="1">{ATTRS}{}{}<backup><duration>4</duration></backup>
            <note><pitch><step>C</step><octave>3</octave></pitch><duration>4</duration><voice>5</voice><staff>2</staff></note></measure>"#,
            note("E", 4, 2, ""),
            note("G", 4, 2, ""),
        );
        let i = imported(&m);
        assert_eq!(i.pieces.len(), 1);
        assert_eq!(i.song, "part1\n");
        assert!(i.pieces[0].1.contains("voices{"), "{}", i.pieces[0].1);
        assert_eq!(i.check.expected, 3);
    }

    #[test]
    fn triplets_ties_and_pedal() {
        let trip = |s: &str| {
            format!(
                "<note><pitch><step>{s}</step><octave>4</octave></pitch><duration>2</duration><voice>1</voice>\
                 <time-modification><actual-notes>3</actual-notes><normal-notes>2</normal-notes></time-modification></note>"
            )
        };
        // 6 divisions per quarter: triplet eighths are 2.
        let attrs = ATTRS.replace("<divisions>2</divisions>", "<divisions>6</divisions>");
        let m1 = format!(
            r#"<measure number="1">{attrs}<direction><direction-type><pedal type="start"/></direction-type></direction>
            {}{}{}<note><pitch><step>G</step><octave>4</octave></pitch><duration>6</duration><voice>1</voice><tie type="start"/></note></measure>"#,
            trip("C"),
            trip("D"),
            trip("E")
        );
        let m2 = r#"<measure number="2"><note><pitch><step>G</step><octave>4</octave></pitch><duration>12</duration><voice>1</voice><tie type="stop"/></note>
            <direction><direction-type><pedal type="stop"/></direction-type></direction></measure>"#;
        let i = imported(&(m1 + m2));
        let text = &i.pieces[0].1;
        assert!(text.contains("tuplet(3)"), "{text}");
        assert!(
            text.contains("ped on") && text.contains("ped off"),
            "{text}"
        );
        // the tie joins two bars into one note
        assert_eq!(i.check.expected, 4);
    }

    #[test]
    fn grace_notes_and_irregular_bars() {
        let m1 = format!(
            r#"<measure number="1">{ATTRS}<note><grace/><pitch><step>B</step><octave>3</octave></pitch><voice>1</voice></note>{}</measure>"#,
            note("C", 4, 4, "")
        );
        // a bar of three quarters without a new time signature
        let m2 = format!(r#"<measure number="2">{}</measure>"#, note("D", 4, 6, ""));
        let i = imported(&(m1 + &m2));
        let text = &i.pieces[0].1;
        assert!(text.contains("time 3/4"), "{text}");
        assert_eq!(i.stats.graces, 1);
    }

    #[test]
    fn upbeat_short_end_and_partly_tied_chords() {
        // An upbeat of a quarter in 2/4; a chord whose F ends its tie while D and
        // A go on; a short last bar with a tie to nothing.
        let k = "!!!OTL: T\n**kern\t**kern\n*staff2\t*staff1\n*M2/4\t*M2/4\n*MM60\t*MM60\n\
                 4C\t4c\n=1\t=1\n[2D [2F [2A\t2d\n=2\t=2\n4D_ 4F] 4A_\t4e\n\
                 4D] 4F 4A]\t4f\n=3\t=3\n4C\t[4c\n==\t==\n*-\t*-\n";
        let i = import(k, "t.krn").unwrap();
        let text: String = i.pieces.iter().map(|p| p.1.as_str()).collect();
        assert!(i.check.ok(), "{:?}\n{text}", i.check);
        // the upbeat: the piece's signature, no bar line before it
        assert!(text.contains("\ntime 2/4 tempo 60 voices{"), "{text}");
        // the tie to nothing is not written
        assert!(text.ends_with("{ mf C3/4 }\n"), "{text}");
        assert!(text.contains("{ mf F3/2~ | F3/4 r }"), "{text}");
        // C C | D F A d | e (F) f | C c: D and A held over the bar, F struck again
        assert_eq!(i.check.expected, 2 + 4 + 3 + 2);
    }

    #[test]
    fn titles_on_one_line() {
        assert_eq!(
            one_line("Sonata — MINUETTO\\nAllegretto"),
            "Sonata — MINUETTO — Allegretto"
        );
        assert_eq!(one_line("A\n  B\r\n"), "A — B");
    }

    #[test]
    fn long_scores_are_cut_into_pieces() {
        let bars: String = (1..=40)
            .map(|n| {
                let attrs = if n == 1 { ATTRS } else { "" };
                format!(
                    r#"<measure number="{n}">{attrs}{}</measure>"#,
                    note("C", 4, 4, "")
                )
            })
            .collect();
        let i = imported(&bars);
        // 40 bars: 16, then the remaining 24 (at most 32 per piece)
        assert_eq!(i.pieces.len(), 2);
        assert_eq!(i.song, "part1 > part2\n");
    }
}
