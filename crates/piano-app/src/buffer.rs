//! An open piece: its text, cursor, edit history, and compiled result.

use std::path::{Path, PathBuf};
use std::time::Instant;

use piano_lang::Compiled;
use piano_project::FileStamp;

use crate::history::{History, Snapshot};

/// Cursor position: 0-based line and character column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct Cursor {
    pub line: usize,
    pub col: usize,
}

pub struct Buffer {
    /// Piece name (file stem), e.g. `p1` or `song`.
    pub name: String,
    pub path: PathBuf,
    pub lines: Vec<String>,
    pub cursor: Cursor,
    /// First visible line in the Code area.
    pub scroll: usize,
    pub dirty: bool,
    /// Files outside the project are read-only.
    pub readonly: bool,
    pub history: History,
    pub compiled: Compiled,
    pub stamp: FileStamp,
    pub last_edit: Option<Instant>,
    pub recovery_written: bool,
    /// When the recovery copy was last written.
    pub recovery_at: Option<Instant>,
}

impl Buffer {
    /// The cursor starts at the end, ready to continue the piece.
    pub fn new(name: &str, path: &Path, text: &str, readonly: bool) -> Buffer {
        let lines = split_lines(text);
        let cursor = Cursor {
            line: lines.len() - 1,
            col: lines[lines.len() - 1].chars().count(),
        };
        Buffer {
            name: name.to_string(),
            path: path.to_path_buf(),
            lines,
            cursor,
            scroll: 0,
            dirty: false,
            readonly,
            history: History::default(),
            compiled: Compiled::default(),
            stamp: FileStamp::new(path),
            last_edit: None,
            recovery_written: false,
            recovery_at: None,
        }
    }

    pub fn text(&self) -> String {
        let mut t = self.lines.join("\n");
        t.push('\n');
        t
    }

    pub fn is_song(&self) -> bool {
        self.name == piano_project::SONG
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text(),
            cursor: (self.cursor.line, self.cursor.col),
        }
    }

    pub fn restore(&mut self, s: &Snapshot) {
        self.lines = split_lines(&s.text);
        self.cursor = Cursor {
            line: s.cursor.0,
            col: s.cursor.1,
        };
        self.clamp_cursor();
        self.touch();
    }

    /// Replaces the text without history (load / reload).
    pub fn set_text(&mut self, text: &str) {
        self.lines = split_lines(text);
        self.clamp_cursor();
    }

    /// Runs an edit and records it in the history.
    pub fn edit(&mut self, desc: &str, coalesce: bool, f: impl FnOnce(&mut Buffer)) {
        let before = self.snapshot();
        f(self);
        let after = self.snapshot();
        if before.text != after.text {
            self.history.push(desc, before, after, coalesce);
            self.touch();
        }
    }

    fn touch(&mut self) {
        self.dirty = true;
        self.last_edit = Some(Instant::now());
        self.recovery_written = false;
    }

    pub fn line(&self) -> &str {
        &self.lines[self.cursor.line]
    }

    fn line_len(&self, line: usize) -> usize {
        self.lines[line].chars().count()
    }

    pub fn clamp_cursor(&mut self) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor.line = self.cursor.line.min(self.lines.len() - 1);
        self.cursor.col = self.cursor.col.min(self.line_len(self.cursor.line));
    }

    /// Inserts `s` (may contain newlines) at the cursor; the cursor ends after it.
    pub fn insert_str(&mut self, s: &str) {
        let Cursor { line, col } = self.cursor;
        let current = &self.lines[line];
        let at = byte_index(current, col);
        let (head, tail) = (current[..at].to_string(), current[at..].to_string());
        let mut parts: Vec<&str> = s.split('\n').collect();
        let last = parts.pop().unwrap_or("");
        if parts.is_empty() {
            self.lines[line] = format!("{head}{last}{tail}");
            self.cursor.col = col + last.chars().count();
        } else {
            let mut new_lines = vec![format!("{head}{}", parts[0])];
            new_lines.extend(parts[1..].iter().map(|p| p.to_string()));
            new_lines.push(format!("{last}{tail}"));
            let n = new_lines.len();
            self.lines.splice(line..=line, new_lines);
            self.cursor = Cursor {
                line: line + n - 1,
                col: last.chars().count(),
            };
        }
    }

    /// Deletes characters `from..to` on line `line` (char columns).
    pub fn delete_range(&mut self, line: usize, from: usize, to: usize) {
        let l = &mut self.lines[line];
        let (a, b) = (byte_index(l, from), byte_index(l, to));
        l.replace_range(a..b, "");
    }

    pub fn backspace(&mut self) {
        let Cursor { line, col } = self.cursor;
        if col > 0 {
            self.delete_range(line, col - 1, col);
            self.cursor.col -= 1;
        } else if line > 0 {
            let cur = self.lines.remove(line);
            let prev_len = self.line_len(line - 1);
            self.lines[line - 1].push_str(&cur);
            self.cursor = Cursor {
                line: line - 1,
                col: prev_len,
            };
        }
    }

    pub fn delete(&mut self) {
        let Cursor { line, col } = self.cursor;
        if col < self.line_len(line) {
            self.delete_range(line, col, col + 1);
        } else if line + 1 < self.lines.len() {
            let next = self.lines.remove(line + 1);
            self.lines[line].push_str(&next);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.line_len(self.cursor.line);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor.col < self.line_len(self.cursor.line) {
            self.cursor.col += 1;
        } else if self.cursor.line + 1 < self.lines.len() {
            self.cursor.line += 1;
            self.cursor.col = 0;
        }
    }

    pub fn move_up(&mut self, n: usize) {
        self.cursor.line = self.cursor.line.saturating_sub(n);
        self.clamp_cursor();
    }

    pub fn move_down(&mut self, n: usize) {
        self.cursor.line = (self.cursor.line + n).min(self.lines.len() - 1);
        self.clamp_cursor();
    }

    pub fn home(&mut self) {
        self.cursor.col = 0;
    }

    pub fn end(&mut self) {
        self.cursor.col = self.line_len(self.cursor.line);
    }

    /// Moves the cursor to a 1-based line.
    pub fn goto(&mut self, line: usize) {
        self.cursor = Cursor {
            line: line.saturating_sub(1),
            col: 0,
        };
        self.clamp_cursor();
    }

    /// 1-based (line, col) for the language functions.
    pub fn pos1(&self) -> (u32, u32) {
        (self.cursor.line as u32 + 1, self.cursor.col as u32 + 1)
    }

    /// Character before the cursor on the current line.
    pub fn char_before(&self) -> Option<char> {
        self.cursor
            .col
            .checked_sub(1)
            .and_then(|c| self.line().chars().nth(c))
    }

    pub fn char_at(&self) -> Option<char> {
        self.line().chars().nth(self.cursor.col)
    }
}

fn split_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Byte index of char column `col` in `s` (clamped to the end).
pub fn byte_index(s: &str, col: usize) -> usize {
    s.char_indices().nth(col).map_or(s.len(), |(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(text: &str) -> Buffer {
        Buffer::new("p1", Path::new("p1.piano"), text, false)
    }

    #[test]
    fn cursor_starts_at_end() {
        assert_eq!(buf("C D\nE F").cursor, Cursor { line: 1, col: 3 });
    }

    #[test]
    fn insert_and_backspace() {
        let mut b = buf("C D");
        b.cursor.col = 1;
        b.insert_str(" E");
        assert_eq!(b.line(), "C E D");
        assert_eq!(b.cursor.col, 3);
        b.insert_str("\nF");
        assert_eq!(b.lines, ["C E", "F D"]);
        assert_eq!(b.cursor, Cursor { line: 1, col: 1 });
        b.home();
        b.backspace();
        assert_eq!(b.lines, ["C EF D"]);
    }

    #[test]
    fn unicode_columns() {
        let mut b = buf("도 C");
        b.cursor.col = 1;
        b.insert_str("x");
        assert_eq!(b.line(), "도x C");
    }

    #[test]
    fn edit_records_history() {
        let mut b = buf("");
        b.edit("+ C", false, |b| b.insert_str("C "));
        assert!(b.dirty);
        let s = b.history.undo().unwrap();
        b.restore(&s);
        assert_eq!(b.text(), "\n");
    }
}
