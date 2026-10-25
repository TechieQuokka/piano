//! SoundFont synthesis, real-time playback, and offline rendering.

pub mod player;
pub mod render;
pub mod scheduler;
pub mod synth;
#[doc(hidden)]
pub mod testing;

use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use player::Player;
pub use render::render;
pub use rustysynth::SoundFont;

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("soundfont: {0}")]
    SoundFont(String),
    #[error("audio device: {0}")]
    Device(String),
}

pub fn load_soundfont(path: &Path) -> Result<Arc<SoundFont>, AudioError> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| AudioError::SoundFont(format!("{}: {e}", path.display())))?;
    SoundFont::new(&mut file)
        .map(Arc::new)
        .map_err(|e| AudioError::SoundFont(format!("{}: {e}", path.display())))
}

/// Folders searched for a SoundFont when none is configured.
pub fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/sounds/sf2"),
        PathBuf::from("/usr/share/soundfonts"),
        PathBuf::from("/usr/local/share/soundfonts"),
    ];
    if let Some(data) = dirs::data_dir() {
        dirs.push(data.join("soundfonts"));
    }
    dirs
}

/// First `.sf2` found in [`search_dirs`], preferring names that mention a piano.
pub fn find_soundfont() -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = search_dirs()
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("sf2")))
        .collect();
    found.sort_by_key(|p| {
        let name = p.to_string_lossy().to_ascii_lowercase();
        !(name.contains("piano") || name.contains("salamander"))
    });
    found.into_iter().next()
}
