//! `piano.toml` (per project) and `~/.config/piano/config.toml` (per user).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Per-project settings (`piano.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    pub project: ProjectSection,
    pub sound: SoundSection,
    pub defaults: Defaults,
    pub editor: EditorSection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectSection {
    pub name: String,
    /// File played by `play song`.
    pub entry: String,
}

impl Default for ProjectSection {
    fn default() -> Self {
        ProjectSection {
            name: String::new(),
            entry: "song.piano".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soundfont: Option<PathBuf>,
}

/// Starting values for pieces. Unset fields fall back to the next level
/// (piece header > project > global > builtin).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Defaults {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tempo: Option<u16>,
    /// e.g. "4/4"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// e.g. "1/4"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<String>,
    /// e.g. "mf"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub octave: Option<i32>,
}

impl Defaults {
    /// Fields of `self` win; missing ones are taken from `fallback`.
    pub fn or(&self, fallback: &Defaults) -> Defaults {
        Defaults {
            tempo: self.tempo.or(fallback.tempo),
            time: self.time.clone().or_else(|| fallback.time.clone()),
            length: self.length.clone().or_else(|| fallback.length.clone()),
            dynamic: self.dynamic.clone().or_else(|| fallback.dynamic.clone()),
            octave: self.octave.or(fallback.octave),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorSection {
    pub autosave_secs: u64,
}

impl Default for EditorSection {
    fn default() -> Self {
        EditorSection { autosave_secs: 30 }
    }
}

/// UI language for the manual and tutorial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    En,
    Ko,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "EN",
            Lang::Ko => "KO",
        }
    }

    pub fn parse(s: &str) -> Option<Lang> {
        match s.to_ascii_lowercase().as_str() {
            "en" => Some(Lang::En),
            "ko" => Some(Lang::Ko),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSection {
    pub lang: Lang,
}

/// Per-user settings (`~/.config/piano/config.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    pub sound: SoundSection,
    pub defaults: Defaults,
    pub ui: UiSection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_partial_project_config() {
        let c: ProjectConfig = toml::from_str(
            r#"
            [project]
            name = "my_song"

            [defaults]
            tempo = 90
            "#,
        )
        .unwrap();
        assert_eq!(c.project.name, "my_song");
        assert_eq!(c.project.entry, "song.piano");
        assert_eq!(c.defaults.tempo, Some(90));
        assert_eq!(c.editor.autosave_secs, 30);
    }

    #[test]
    fn defaults_fallback() {
        let project = Defaults {
            tempo: Some(90),
            ..Default::default()
        };
        let global = Defaults {
            tempo: Some(120),
            octave: Some(3),
            ..Default::default()
        };
        let d = project.or(&global);
        assert_eq!((d.tempo, d.octave), (Some(90), Some(3)));
    }

    #[test]
    fn global_lang() {
        let g: GlobalConfig = toml::from_str("[ui]\nlang = \"ko\"").unwrap();
        assert_eq!(g.ui.lang, Lang::Ko);
    }
}
