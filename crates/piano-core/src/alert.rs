//! Alerts (Error / Warning / Hint) tied to source positions.

/// Alert level, ordered from least to most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Hint,
    Warning,
    Error,
}

impl Severity {
    /// Gutter / output marker.
    pub fn marker(self) -> char {
        match self {
            Severity::Error => '✖',
            Severity::Warning => '⚠',
            Severity::Hint => '·',
        }
    }
}

/// A location in source text. `line` and `col` are 1-based; `len` is in chars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SrcSpan {
    pub line: u32,
    pub col: u32,
    pub len: u32,
}

impl SrcSpan {
    pub fn new(line: u32, col: u32, len: u32) -> SrcSpan {
        SrcSpan { line, col, len }
    }
}

/// A message about the source, with an optional fix suggestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    pub severity: Severity,
    pub span: SrcSpan,
    pub message: String,
    pub fix: Option<String>,
    /// For an alert of a piece shown in a song: the piece and the line in it. The
    /// span is then the song's line that plays the piece.
    pub piece: Option<(String, u32)>,
}

impl Alert {
    pub fn error(span: SrcSpan, message: impl Into<String>) -> Alert {
        Alert::new(Severity::Error, span, message)
    }

    pub fn warning(span: SrcSpan, message: impl Into<String>) -> Alert {
        Alert::new(Severity::Warning, span, message)
    }

    pub fn hint(span: SrcSpan, message: impl Into<String>) -> Alert {
        Alert::new(Severity::Hint, span, message)
    }

    fn new(severity: Severity, span: SrcSpan, message: impl Into<String>) -> Alert {
        Alert {
            severity,
            span,
            message: message.into(),
            fix: None,
            piece: None,
        }
    }

    pub fn with_fix(mut self, fix: impl Into<String>) -> Alert {
        self.fix = Some(fix.into());
        self
    }

    /// The alert with the file it is in, for a terminal: `⚠ part1 line 69: ...`.
    /// `file` is the name of what was compiled (a song or a piece).
    pub fn located(&self, file: &str) -> String {
        let (file, line, message) = match &self.piece {
            Some((name, line)) => {
                let prefix = format!("[{name} line {line}] ");
                let message = self.message.strip_prefix(&prefix).unwrap_or(&self.message);
                (name.as_str(), *line, message)
            }
            None => (file, self.span.line, self.message.as_str()),
        };
        let mut out = format!("{} {file} line {line}: {message}", self.severity.marker());
        if let Some(fix) = &self.fix {
            out += " ";
            out += fix;
        }
        out
    }
}

impl std::fmt::Display for Alert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} line {}: {}",
            self.severity.marker(),
            self.span.line,
            self.message
        )?;
        if let Some(fix) = &self.fix {
            write!(f, " {fix}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_order() {
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Hint);
    }

    #[test]
    fn display() {
        let a = Alert::warning(
            SrcSpan::new(3, 4, 2),
            "C4 → F# is ambiguous (6 semitones), chose F#4.",
        )
        .with_fix("Write F#3 or F#4 to fix.");
        assert_eq!(
            a.to_string(),
            "⚠ line 3: C4 → F# is ambiguous (6 semitones), chose F#4. Write F#3 or F#4 to fix."
        );
        assert_eq!(
            a.located("p1"),
            "⚠ p1 line 3: C4 → F# is ambiguous (6 semitones), chose F#4. Write F#3 or F#4 to fix."
        );
    }

    #[test]
    fn located_in_a_piece_of_a_song() {
        let a = Alert {
            piece: Some(("part1".into(), 69)),
            ..Alert::warning(SrcSpan::new(1, 1, 5), "[part1 line 69] bar has 7 beats")
        };
        assert_eq!(a.to_string(), "⚠ line 1: [part1 line 69] bar has 7 beats");
        assert_eq!(a.located("song"), "⚠ part1 line 69: bar has 7 beats");
    }
}
