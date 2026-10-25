//! Lines shown in the Display `output` view.

use piano_core::{Alert, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutKind {
    /// Echo of an entered command.
    Cmd,
    Info,
    Ok,
    Hint,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutLine {
    pub kind: OutKind,
    pub text: String,
    /// A list of names (`ls`): wrapped to the Display width without hanging indent.
    pub list: bool,
}

/// Maximum lines kept in the output log.
pub const MAX_OUTPUT: usize = 2000;

#[derive(Debug, Clone, Default)]
pub struct Output {
    pub lines: Vec<OutLine>,
}

impl Output {
    pub fn push(&mut self, kind: OutKind, text: impl Into<String>) {
        for line in text.into().lines() {
            self.lines.push(OutLine {
                kind,
                text: line.to_string(),
                list: false,
            });
        }
        self.trim();
    }

    /// Names on one line, wrapped by the Display at its width.
    pub fn list(&mut self, names: &[String]) {
        self.lines.push(OutLine {
            kind: OutKind::Info,
            text: names.join("  "),
            list: true,
        });
        self.trim();
    }

    fn trim(&mut self) {
        if self.lines.len() > MAX_OUTPUT {
            let extra = self.lines.len() - MAX_OUTPUT;
            self.lines.drain(..extra);
        }
    }

    pub fn info(&mut self, text: impl Into<String>) {
        self.push(OutKind::Info, text);
    }

    pub fn ok(&mut self, text: impl Into<String>) {
        self.push(OutKind::Ok, text);
    }

    pub fn warn(&mut self, text: impl Into<String>) {
        self.push(OutKind::Warn, text);
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.push(OutKind::Error, text);
    }

    pub fn alert(&mut self, alert: &Alert) {
        let kind = match alert.severity {
            Severity::Error => OutKind::Error,
            Severity::Warning => OutKind::Warn,
            Severity::Hint => OutKind::Hint,
        };
        self.push(kind, alert.to_string());
    }

    /// The most recent non-echo line (shown above the Cmd input).
    pub fn last_result(&self) -> Option<&OutLine> {
        self.lines.iter().rev().find(|l| l.kind != OutKind::Cmd)
    }
}
