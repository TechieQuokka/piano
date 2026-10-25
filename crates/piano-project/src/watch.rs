//! External change detection by polling modification times (no background threads).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::project::modified;

/// Remembers a file's modification time to detect changes made by other programs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStamp {
    path: PathBuf,
    stamp: Option<SystemTime>,
}

impl FileStamp {
    pub fn new(path: &Path) -> FileStamp {
        FileStamp {
            path: path.to_path_buf(),
            stamp: modified(path),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// `true` once per external change.
    pub fn changed(&mut self) -> bool {
        let now = modified(&self.path);
        let changed = now != self.stamp;
        self.stamp = now;
        changed
    }

    /// Call after writing the file ourselves so it is not seen as external.
    pub fn refresh(&mut self) {
        self.stamp = modified(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::TempDir;

    #[test]
    fn detects_changes() {
        let tmp = TempDir::new("watch");
        let f = tmp.0.join("a.piano");
        std::fs::write(&f, "C").unwrap();
        let mut s = FileStamp::new(&f);
        assert!(!s.changed());
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&f, "D").unwrap();
        assert!(s.changed());
        assert!(!s.changed());
    }
}
