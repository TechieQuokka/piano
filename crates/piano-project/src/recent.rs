//! User-level files under `~/.config/piano/`: recent projects and global config.

use std::path::{Path, PathBuf};

use crate::config::GlobalConfig;

const MAX_RECENT: usize = 20;

pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("piano"))
}

/// Loads `config.toml`; a missing or invalid file yields defaults plus an error text.
pub fn load_global() -> (GlobalConfig, Option<String>) {
    let Some(path) = config_dir().map(|d| d.join("config.toml")) else {
        return (GlobalConfig::default(), None);
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => match toml::from_str(&text) {
            Ok(c) => (c, None),
            Err(e) => (
                GlobalConfig::default(),
                Some(format!("{}: {e}", path.display())),
            ),
        },
        Err(_) => (GlobalConfig::default(), None),
    }
}

pub fn save_global(config: &GlobalConfig) -> std::io::Result<()> {
    let dir = config_dir().ok_or_else(|| std::io::Error::other("no config directory"))?;
    std::fs::create_dir_all(&dir)?;
    let text = toml::to_string(config).map_err(std::io::Error::other)?;
    std::fs::write(dir.join("config.toml"), text)
}

/// Most recent first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Recent {
    pub paths: Vec<PathBuf>,
}

impl Recent {
    fn file() -> Option<PathBuf> {
        config_dir().map(|d| d.join("recent"))
    }

    pub fn load() -> Recent {
        let paths = Self::file()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .map(|t| {
                t.lines()
                    .filter(|l| !l.is_empty())
                    .map(PathBuf::from)
                    .collect()
            })
            .unwrap_or_default();
        Recent { paths }
    }

    pub fn push(&mut self, path: &Path) {
        self.paths.retain(|p| p != path);
        self.paths.insert(0, path.to_path_buf());
        self.paths.truncate(MAX_RECENT);
    }

    pub fn save(&self) -> std::io::Result<()> {
        let file = Self::file().ok_or_else(|| std::io::Error::other("no config directory"))?;
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text: String = self
            .paths
            .iter()
            .map(|p| format!("{}\n", p.display()))
            .collect();
        std::fs::write(file, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_moves_to_front() {
        let mut r = Recent::default();
        r.push(Path::new("/a"));
        r.push(Path::new("/b"));
        r.push(Path::new("/a"));
        assert_eq!(r.paths, [PathBuf::from("/a"), PathBuf::from("/b")]);
    }
}
