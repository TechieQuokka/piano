//! A project is any folder containing `piano.toml`.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::config::ProjectConfig;

pub const CONFIG_FILE: &str = "piano.toml";
pub const PIECE_EXT: &str = "piano";
pub const SONG: &str = "song";
const RECOVERY_DIR: &str = ".piano/recovery";

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("{0}: not a project (no piano.toml), use `project new`")]
    NotAProject(PathBuf),
    #[error("{0}: already a project")]
    AlreadyProject(PathBuf),
    #[error("piano.toml: {0}")]
    Config(#[from] toml::de::Error),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub root: PathBuf,
    pub config: ProjectConfig,
}

impl Project {
    /// Turns `dir` (created if missing) into a project. Existing files are never
    /// touched; only missing `piano.toml`, `song.piano`, and `p1.piano` are added.
    pub fn create(dir: &Path) -> Result<Project, ProjectError> {
        if dir.join(CONFIG_FILE).exists() {
            return Err(ProjectError::AlreadyProject(dir.to_path_buf()));
        }
        std::fs::create_dir_all(dir)?;
        let root = dir.canonicalize()?;
        let mut config = ProjectConfig::default();
        config.project.name = root
            .file_name()
            .map_or_else(|| "piano".into(), |n| n.to_string_lossy().into_owned());
        let project = Project { root, config };
        project.save_config()?;
        write_new(&project.piece_path(SONG), "p1\n")?;
        write_new(&project.piece_path("p1"), "// p1\n")?;
        Ok(project)
    }

    pub fn open(dir: &Path) -> Result<Project, ProjectError> {
        let file = dir.join(CONFIG_FILE);
        if !file.exists() {
            return Err(ProjectError::NotAProject(dir.to_path_buf()));
        }
        let config = toml::from_str(&std::fs::read_to_string(&file)?)?;
        Ok(Project {
            root: dir.canonicalize()?,
            config,
        })
    }

    pub fn name(&self) -> &str {
        &self.config.project.name
    }

    pub fn save_config(&self) -> Result<(), ProjectError> {
        let text = toml::to_string(&self.config).map_err(std::io::Error::other)?;
        std::fs::write(self.root.join(CONFIG_FILE), text)?;
        Ok(())
    }

    /// `<root>/<name>.piano`
    pub fn piece_path(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}.{PIECE_EXT}"))
    }

    /// Piece names (`*.piano` in the root, without extension), sorted; includes `song`.
    pub fn pieces(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.root)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                if p.extension()? != PIECE_EXT {
                    return None;
                }
                Some(p.file_stem()?.to_string_lossy().into_owned())
            })
            .collect();
        names.sort();
        names
    }

    /// Reads `<name>.piano` if it exists.
    pub fn load_piece(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.piece_path(name)).ok()
    }

    fn recovery_path(&self, name: &str) -> PathBuf {
        self.root
            .join(RECOVERY_DIR)
            .join(format!("{name}.{PIECE_EXT}"))
    }

    pub fn write_recovery(&self, name: &str, text: &str) -> std::io::Result<()> {
        let path = self.recovery_path(name);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, text)
    }

    /// The recovery copy of `name`, if it is newer than the piece file and differs.
    pub fn newer_recovery(&self, name: &str) -> Option<String> {
        let rec = self.recovery_path(name);
        let rec_time = modified(&rec)?;
        let piece = self.piece_path(name);
        let text = std::fs::read_to_string(&rec).ok()?;
        match modified(&piece) {
            Some(t) if t >= rec_time => None,
            _ if std::fs::read_to_string(&piece).ok().as_deref() == Some(&text) => None,
            _ => Some(text),
        }
    }

    pub fn clear_recovery(&self, name: &str) {
        let _ = std::fs::remove_file(self.recovery_path(name));
    }
}

fn write_new(path: &Path, text: &str) -> std::io::Result<()> {
    if !path.exists() {
        std::fs::write(path, text)?;
    }
    Ok(())
}

pub fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::TempDir;

    #[test]
    fn create_keeps_existing_files() {
        let tmp = TempDir::new("create");
        let dir = tmp.0.join("song_dir");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("p1.piano"), "C D E").unwrap();
        std::fs::write(dir.join("notes.txt"), "keep").unwrap();

        let p = Project::create(&dir).unwrap();
        assert_eq!(p.name(), "song_dir");
        assert_eq!(p.load_piece("p1").unwrap(), "C D E");
        assert_eq!(
            std::fs::read_to_string(dir.join("notes.txt")).unwrap(),
            "keep"
        );
        assert_eq!(p.pieces(), ["p1", "song"]);
        assert!(matches!(
            Project::create(&dir),
            Err(ProjectError::AlreadyProject(_))
        ));

        let reopened = Project::open(&dir).unwrap();
        assert_eq!(reopened, p);
    }

    #[test]
    fn open_requires_config() {
        let tmp = TempDir::new("open");
        assert!(matches!(
            Project::open(&tmp.0),
            Err(ProjectError::NotAProject(_))
        ));
    }

    #[test]
    fn recovery_copy() {
        let tmp = TempDir::new("recovery");
        let p = Project::create(&tmp.0.join("r")).unwrap();
        assert_eq!(p.newer_recovery("p1"), None);
        std::thread::sleep(std::time::Duration::from_millis(20));
        p.write_recovery("p1", "C D E F").unwrap();
        assert_eq!(p.newer_recovery("p1").as_deref(), Some("C D E F"));
        p.clear_recovery("p1");
        assert_eq!(p.newer_recovery("p1"), None);
    }
}
