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
        }
    }

    pub fn with_fix(mut self, fix: impl Into<String>) -> Alert {
        self.fix = Some(fix.into());
        self
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
    }
}
