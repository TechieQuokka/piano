//! Undo / redo with snapshots of the whole piece text.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub text: String,
    pub cursor: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub desc: String,
    pub before: Snapshot,
    pub after: Snapshot,
    coalesce: bool,
}

const MAX_ENTRIES: usize = 1000;

#[derive(Debug, Clone, Default)]
pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
}

impl History {
    /// Records an edit. With `coalesce`, consecutive edits with the same description
    /// (e.g. typing) merge into one entry.
    pub fn push(&mut self, desc: &str, before: Snapshot, after: Snapshot, coalesce: bool) {
        self.redo.clear();
        if coalesce
            && let Some(last) = self.undo.last_mut()
            && last.coalesce
            && last.desc == desc
            && last.after == before
        {
            last.after = after;
            return;
        }
        self.undo.push(Entry {
            desc: desc.to_string(),
            before,
            after,
            coalesce,
        });
        if self.undo.len() > MAX_ENTRIES {
            self.undo.remove(0);
        }
    }

    /// Returns the state to restore.
    pub fn undo(&mut self) -> Option<Snapshot> {
        let e = self.undo.pop()?;
        let s = e.before.clone();
        self.redo.push(e);
        Some(s)
    }

    pub fn redo(&mut self) -> Option<Snapshot> {
        let e = self.redo.pop()?;
        let s = e.after.clone();
        self.undo.push(e);
        Some(s)
    }

    /// Descriptions of past edits, oldest first.
    pub fn entries(&self) -> impl Iterator<Item = &str> {
        self.undo.iter().map(|e| e.desc.as_str())
    }

    /// Lets the caller refine the description of the last edit.
    pub fn last_desc_mut(&mut self) -> Option<&mut String> {
        self.undo.last_mut().map(|e| &mut e.desc)
    }

    pub fn len(&self) -> usize {
        self.undo.len()
    }

    pub fn is_empty(&self) -> bool {
        self.undo.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(t: &str) -> Snapshot {
        Snapshot {
            text: t.into(),
            cursor: (0, t.len()),
        }
    }

    #[test]
    fn undo_redo() {
        let mut h = History::default();
        h.push("+ C", s(""), s("C "), false);
        h.push("+ D", s("C "), s("C D "), false);
        assert_eq!(h.undo().unwrap().text, "C ");
        assert_eq!(h.undo().unwrap().text, "");
        assert!(h.undo().is_none());
        assert_eq!(h.redo().unwrap().text, "C ");
    }

    #[test]
    fn typing_coalesces() {
        let mut h = History::default();
        h.push("type", s(""), s("a"), true);
        h.push("type", s("a"), s("ab"), true);
        assert_eq!(h.len(), 1);
        assert_eq!(h.undo().unwrap().text, "");
    }
}
