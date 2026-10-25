//! Path rules: inside the project everything is allowed; outside, reading and
//! creating new files is allowed, but nothing existing may be modified.

use std::path::{Component, Path, PathBuf};

/// What an operation wants to do with a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// List, read, change directory.
    Read,
    /// Create a file that must not exist yet (copy / export outside).
    CreateNew,
    /// Overwrite, delete, move, or create directories.
    Modify,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Denied {
    #[error("outside project is read-only")]
    OutsideReadOnly,
    #[error("{0} already exists (outside project, no overwrite)")]
    Exists(PathBuf),
    #[error("{0}: no such file or directory")]
    NotFound(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    /// `root` should be the canonical project directory.
    pub fn new(root: impl Into<PathBuf>) -> Sandbox {
        let root = root.into();
        let root = root.canonicalize().unwrap_or(root);
        Sandbox { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolves `path` (relative to `cwd`), canonicalizing it so `..` and symlinks
    /// cannot escape, then applies the rules for `access`.
    pub fn check(&self, cwd: &Path, path: &Path, access: Access) -> Result<PathBuf, Denied> {
        let resolved = resolve(cwd, path);
        let inside = resolved.starts_with(&self.root);
        match access {
            Access::Read => {
                if resolved.exists() {
                    Ok(resolved)
                } else {
                    Err(Denied::NotFound(path.to_path_buf()))
                }
            }
            Access::CreateNew if !inside => {
                if resolved.exists() {
                    Err(Denied::Exists(path.to_path_buf()))
                } else {
                    Ok(resolved)
                }
            }
            Access::Modify if !inside => Err(Denied::OutsideReadOnly),
            Access::CreateNew | Access::Modify => Ok(resolved),
        }
    }

    pub fn is_inside(&self, cwd: &Path, path: &Path) -> bool {
        resolve(cwd, path).starts_with(&self.root)
    }
}

/// Makes `path` absolute and canonical. The longest existing ancestor is
/// canonicalized (resolving symlinks); the remaining components are normalized.
pub fn resolve(cwd: &Path, path: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(expand_home(path))
    };
    let joined = expand_home(&joined);
    let mut existing = joined.as_path();
    let mut rest = Vec::new();
    while !existing.exists() {
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_os_string());
                existing = parent;
            }
            _ => break,
        }
    }
    let mut out = existing
        .canonicalize()
        .unwrap_or_else(|_| existing.to_path_buf());
    for name in rest.into_iter().rev() {
        out.push(name);
    }
    normalize(&out)
}

fn expand_home(path: &Path) -> PathBuf {
    match path.strip_prefix("~") {
        Ok(rest) => dirs::home_dir().map_or_else(|| path.to_path_buf(), |h| h.join(rest)),
        Err(_) => path.to_path_buf(),
    }
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_util::TempDir;

    #[test]
    fn rules() {
        let tmp = TempDir::new("rules");
        let project = tmp.0.join("proj");
        let outside = tmp.0.join("out");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("old.mid"), b"x").unwrap();
        std::fs::write(project.join("p1.piano"), b"C").unwrap();
        let sb = Sandbox::new(&project);

        assert!(
            sb.check(&project, Path::new("p1.piano"), Access::Modify)
                .is_ok()
        );
        assert!(
            sb.check(&project, Path::new("new.piano"), Access::Modify)
                .is_ok()
        );
        assert!(
            sb.check(&project, Path::new("../out/old.mid"), Access::Read)
                .is_ok()
        );
        assert_eq!(
            sb.check(&project, Path::new("../out/old.mid"), Access::Modify),
            Err(Denied::OutsideReadOnly)
        );
        assert!(matches!(
            sb.check(&project, Path::new("../out/old.mid"), Access::CreateNew),
            Err(Denied::Exists(_))
        ));
        assert!(
            sb.check(&project, Path::new("../out/new.mid"), Access::CreateNew)
                .is_ok()
        );
        // `..` inside a path cannot escape.
        assert_eq!(
            sb.check(&project, Path::new("sub/../../out/x"), Access::Modify),
            Err(Denied::OutsideReadOnly)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_blocked() {
        let tmp = TempDir::new("link");
        let project = tmp.0.join("proj");
        let outside = tmp.0.join("out");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, project.join("link")).unwrap();
        let sb = Sandbox::new(&project);
        assert_eq!(
            sb.check(&project, Path::new("link/file"), Access::Modify),
            Err(Denied::OutsideReadOnly)
        );
    }
}
