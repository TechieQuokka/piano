//! Top-level application state. Knows nothing about the terminal.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use piano_audio::Player;
use piano_core::{Duration, Dynamic, Pitch, Score, TimeSig};
use piano_lang::{CompileOptions, Compiled, State};
use piano_project::{GlobalConfig, Lang, Project, Recent, Sandbox};

use crate::buffer::Buffer;
use crate::focus::{Focus, View};
use crate::input::{self, InputState, MAX_OCTAVE, MIN_OCTAVE};
use crate::output::Output;

/// Idle time after an edit before the recovery copy is written.
const RECOVERY_IDLE: StdDuration = StdDuration::from_secs(2);
/// How often files are checked for external changes.
const WATCH_EVERY: StdDuration = StdDuration::from_secs(1);
/// How long a pressed key stays highlighted.
const PRESS_GLOW: StdDuration = StdDuration::from_millis(350);

/// Content of the Help / Man views.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
    pub title: String,
    pub lines: Vec<String>,
}

/// An action waiting for `y` in Cmd.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    Quit,
    Close(String),
    Remove(PathBuf),
    SwitchProject(PathBuf),
}

/// Single-line editor for Cmd. `cursor` is a char index.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineEdit {
    pub text: String,
    pub cursor: usize,
}

impl LineEdit {
    pub fn insert(&mut self, c: char) {
        let at = crate::buffer::byte_index(&self.text, self.cursor);
        self.text.insert(at, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let at = crate::buffer::byte_index(&self.text, self.cursor);
            self.text.remove(at);
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.text.chars().count() {
            let at = crate::buffer::byte_index(&self.text, self.cursor);
            self.text.remove(at);
        }
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.text.chars().count());
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.text.chars().count();
    }

    pub fn set(&mut self, text: &str) {
        self.text = text.to_string();
        self.end();
    }

    pub fn take(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.text)
    }
}

/// What is being played, for highlighting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playing {
    pub piece: String,
    pub score: Arc<Score>,
}

pub struct App {
    pub project: Project,
    pub sandbox: Sandbox,
    pub cwd: PathBuf,
    pub global: GlobalConfig,
    pub recent: Recent,
    pub buffers: Vec<Buffer>,
    pub active: usize,
    pub focus: Focus,
    pub view: View,
    pub input: InputState,
    pub cmdline: LineEdit,
    pub cmd_history: Vec<String>,
    pub(crate) cmd_hist_pos: Option<usize>,
    pub output: Output,
    pub page: Page,
    /// Scroll offset of the Display area.
    pub display_scroll: usize,
    pub player: Player,
    pub playing: Option<Playing>,
    pub lang: Lang,
    pub tutorial_step: usize,
    pub pending: Option<Pending>,
    pub quit: bool,
    /// Recently pressed keys with the time they were pressed (Keys view glow).
    pub pressed: Vec<(Pitch, Instant)>,
    pub soundfont: Option<PathBuf>,
    /// Notes struck in Keys focus, oldest first (Played view).
    pub played: Vec<(Pitch, Instant)>,
    pub(crate) held: Vec<crate::keys::Held>,
    /// The terminal reports key releases (set by the UI).
    pub exact_keys: bool,
    last_watch: Instant,
    pub(crate) user_files: UserFiles,
}

/// How the app uses user-level files under `~/.config/piano`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserFiles {
    /// Tests: neither read nor written.
    Ignore,
    /// Command-line tools: the global config is read; nothing is written.
    Read,
    /// The editor: also records recent projects and saves `lang`.
    ReadWrite,
}

impl App {
    /// The editor. Audio problems are reported in the output, never fatal.
    pub fn new(project: Project) -> App {
        Self::build(project, Player::new(), UserFiles::ReadWrite)
    }

    /// Command-line tools (`piano play`, `piano export`): audio, but no user files written.
    pub fn for_tool(project: Project) -> App {
        Self::build(project, Player::new(), UserFiles::Read)
    }

    /// Tests: no audio device and no user-level files at all.
    pub fn headless(project: Project) -> App {
        Self::build(project, Player::silent(), UserFiles::Ignore)
    }

    /// Tests: a given player, no user-level files.
    #[doc(hidden)]
    pub fn headless_with(project: Project, player: Player) -> App {
        Self::build(project, player, UserFiles::Ignore)
    }

    fn build(project: Project, player: Player, user_files: UserFiles) -> App {
        let persist = user_files != UserFiles::Ignore;
        let (global, global_err) = if persist {
            piano_project::recent::load_global()
        } else {
            (GlobalConfig::default(), None)
        };
        let mut app = App {
            sandbox: Sandbox::new(&project.root),
            cwd: project.root.clone(),
            project,
            lang: global.ui.lang,
            global,
            recent: if persist {
                Recent::load()
            } else {
                Recent::default()
            },
            buffers: Vec::new(),
            active: 0,
            focus: Focus::Keys,
            view: View::Output,
            input: InputState::default(),
            cmdline: LineEdit::default(),
            cmd_history: Vec::new(),
            cmd_hist_pos: None,
            output: Output::default(),
            page: Page::default(),
            display_scroll: 0,
            player,
            playing: None,
            tutorial_step: 0,
            pending: None,
            quit: false,
            pressed: Vec::new(),
            soundfont: None,
            played: Vec::new(),
            held: Vec::new(),
            exact_keys: false,
            last_watch: Instant::now(),
            user_files,
        };
        app.output.info(format!(
            "piano {} — type `help` in Cmd (F4), `tutorial` to start.",
            env!("CARGO_PKG_VERSION")
        ));
        if let Some(e) = global_err {
            app.output.warn(format!("config: {e}"));
        }
        app.enter_project();
        app.setup_sound();
        app
    }

    /// Opens the first piece of the current project and records it as recent.
    pub(crate) fn enter_project(&mut self) {
        self.buffers.clear();
        self.active = 0;
        self.cwd = self.project.root.clone();
        self.input = InputState {
            octave: self.options().octave.clamp(MIN_OCTAVE, MAX_OCTAVE),
            ..InputState::default()
        };
        self.recent.push(&self.project.root);
        if self.user_files == UserFiles::ReadWrite {
            let _ = self.recent.save();
        }
        let pieces = self.project.pieces();
        let first = pieces
            .iter()
            .find(|p| p.as_str() != piano_project::SONG)
            .or(pieces.first())
            .cloned();
        match first {
            Some(name) => {
                self.open_piece(&name);
            }
            None => {
                let path = self.project.piece_path("p1");
                self.buffers.push(Buffer::new("p1", &path, "", false));
                self.recompile(0);
            }
        }
        self.output.ok(format!(
            "project {} ({})",
            self.project.name(),
            self.project.root.display()
        ));
    }

    fn setup_sound(&mut self) {
        let path = self
            .project
            .config
            .sound
            .soundfont
            .clone()
            .or_else(|| self.global.sound.soundfont.clone())
            .or_else(|| {
                (self.user_files != UserFiles::Ignore)
                    .then(piano_audio::find_soundfont)
                    .flatten()
            });
        match path {
            Some(p) => match self.player.load_soundfont(&p) {
                Ok(()) => {
                    self.output.ok(format!("soundfont {}", p.display()));
                    self.soundfont = Some(p);
                }
                Err(e) => self.output.warn(e.to_string()),
            },
            None => self.output.warn(
                "no SoundFont found: editing works, but there is no sound.\n\
                 Get a piano .sf2 (e.g. Salamander Grand Piano, FluidR3_GM) and run `font <path>`.",
            ),
        }
        match (self.player.device_info(), self.player.device_error()) {
            (Some(info), _) => self.output.info(format!("audio: {info}")),
            (None, Some(e)) => self.output.warn(format!("audio: {e}")),
            (None, None) => {}
        }
    }

    // ---- buffers ----------------------------------------------------------

    pub fn buf(&self) -> &Buffer {
        &self.buffers[self.active]
    }

    pub fn buf_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.active]
    }

    pub fn buffer_index(&self, name: &str) -> Option<usize> {
        self.buffers.iter().position(|b| b.name == name)
    }

    /// Opens (or switches to) a project piece by name. Returns `false` if missing.
    pub fn open_piece(&mut self, name: &str) -> bool {
        if let Some(i) = self.buffer_index(name) {
            self.active = i;
            return true;
        }
        let path = self.project.piece_path(name);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return false;
        };
        self.open_buffer(Buffer::new(name, &path, &text, false));
        true
    }

    pub(crate) fn open_buffer(&mut self, buffer: Buffer) {
        let name = buffer.name.clone();
        let readonly = buffer.readonly;
        self.buffers.push(buffer);
        self.active = self.buffers.len() - 1;
        self.recompile(self.active);
        if !readonly && self.project.newer_recovery(&name).is_some() {
            self.output.warn(format!(
                "{name}: a newer recovery copy exists — type `recover` to restore it"
            ));
        }
    }

    pub fn options(&self) -> CompileOptions {
        let d = self.project.config.defaults.or(&self.global.defaults);
        let mut o = CompileOptions::default();
        if let Some(t) = d.tempo {
            o.tempo = t.clamp(20, 400);
        }
        if let Some(sig) = d.time.as_deref().and_then(parse_time) {
            o.time = sig;
        }
        if let Some(len) = d.length.as_deref().and_then(parse_len) {
            o.len = len;
        }
        if let Some(dynamic) = d.dynamic.as_deref().and_then(Dynamic::parse) {
            o.dynamic = dynamic;
        }
        if let Some(oct) = d.octave {
            o.octave = oct.clamp(0, 8);
        }
        o
    }

    /// Source of a piece: the open buffer if any, else the file.
    pub fn piece_source(&self, name: &str) -> Option<String> {
        match self.buffer_index(name) {
            Some(i) => Some(self.buffers[i].text()),
            None => self.project.load_piece(name),
        }
    }

    pub(crate) fn compile_named(&self, name: &str, text: &str) -> Compiled {
        let opts = self.options();
        if name == piano_project::SONG {
            piano_lang::compile_song(text, &|n| self.piece_source(n), &opts)
        } else {
            piano_lang::compile_piece_with(text, &opts)
        }
    }

    pub fn recompile(&mut self, i: usize) {
        let compiled = self.compile_named(&self.buffers[i].name, &self.buffers[i].text());
        self.buffers[i].compiled = compiled;
    }

    /// Inherited state at the cursor of the active buffer.
    pub fn state_at_cursor(&self) -> State {
        let b = self.buf();
        let (line, col) = b.pos1();
        piano_lang::state_at(&b.text(), line, col, &self.options())
    }

    fn check_writable(&mut self) -> bool {
        if self.buf().readonly {
            self.output.error(
                "this file is outside the project (read-only); `cp` it into the project to edit",
            );
            return false;
        }
        true
    }

    /// Runs an edit on the active buffer and recompiles.
    pub(crate) fn edit(&mut self, desc: &str, coalesce: bool, f: impl FnOnce(&mut Buffer)) -> bool {
        if !self.check_writable() {
            return false;
        }
        self.buf_mut().edit(desc, coalesce, f);
        self.recompile(self.active);
        true
    }

    // ---- focus ------------------------------------------------------------

    pub fn set_focus(&mut self, focus: Focus) {
        if focus != Focus::Keys {
            self.release_all_keys();
        }
        // The Display keeps its view: only commands change it.
        self.focus = focus;
    }

    pub fn set_view(&mut self, view: View) {
        self.view = view;
        self.display_scroll = 0;
    }

    /// Esc: cancels a pending question (the Display is left as it is).
    pub fn escape(&mut self) {
        if self.pending.take().is_some() {
            self.output.info("cancelled");
        }
    }

    // ---- keys focus (see keys.rs) ----------------------------------------------

    /// Inserts a word such as `ped on` at the Code cursor.
    pub fn insert_word(&mut self, text: &str) -> bool {
        self.edit(&format!("+ {text}"), false, |b| {
            input::insert_token(b, text)
        })
    }

    pub fn shift_octave(&mut self, delta: i32) {
        self.release_all_keys();
        self.input.octave = (self.input.octave + delta).clamp(MIN_OCTAVE, MAX_OCTAVE);
    }

    // ---- code focus -------------------------------------------------------

    pub fn code_char(&mut self, c: char) {
        self.edit("type", true, |b| b.insert_str(&c.to_string()));
    }

    pub fn code_newline(&mut self) {
        self.edit("newline", false, |b| b.insert_str("\n"));
    }

    pub fn code_backspace(&mut self) {
        self.edit("delete", true, Buffer::backspace);
    }

    pub fn code_delete(&mut self) {
        self.edit("delete", true, Buffer::delete);
    }

    pub fn code_indent(&mut self) {
        self.edit("indent", false, |b| b.insert_str("  "));
    }

    // ---- playback ---------------------------------------------------------

    /// Compiled score of a piece (open buffer or file).
    pub fn score_of(&self, name: &str) -> Option<(Arc<Score>, Compiled)> {
        let text = self.piece_source(name)?;
        let compiled = self.compile_named(name, &text);
        Some((Arc::new(compiled.score.clone()), compiled))
    }

    pub(crate) fn start_play(
        &mut self,
        piece: &str,
        score: Arc<Score>,
        from: piano_core::Tick,
        to: Option<piano_core::Tick>,
        looping: bool,
    ) {
        if !self.player.has_sound() {
            self.output
                .warn("no sound: see `font` (SoundFont) and the audio messages at startup");
        }
        if looping {
            self.player
                .play_loop(Arc::clone(&score), from, to.unwrap_or(score.end));
        } else {
            self.player.play(Arc::clone(&score), from, to);
        }
        self.playing = Some(Playing {
            piece: piece.to_string(),
            score,
        });
    }

    /// F5: play the active piece from the cursor (a song plays from its start).
    pub fn play_here(&mut self) {
        let b = self.buf();
        let (line, col) = b.pos1();
        let name = b.name.clone();
        let score = Arc::new(b.compiled.score.clone());
        let from = if b.is_song() {
            0
        } else {
            piano_lang::tick_at(&score, line, col)
        };
        self.report_alerts();
        self.output.info(format!(
            "▶ {name} from line {line} ({})",
            clock(score.duration_secs() - score.seconds_at(from))
        ));
        self.start_play(&name, score, from, None, false);
    }

    pub fn stop(&mut self) {
        self.player.stop();
        self.playing = None;
    }

    /// Writes the active buffer's alerts to the output.
    pub(crate) fn report_alerts(&mut self) {
        let alerts = self.buf().compiled.alerts.clone();
        for a in &alerts {
            self.output.alert(a);
        }
    }

    /// Source line being played in the active buffer.
    pub fn playing_line(&self) -> Option<u32> {
        let p = self.playing.as_ref()?;
        if p.piece != self.buf().name || self.buf().is_song() {
            return None;
        }
        let tick = self.player.position()?;
        piano_lang::line_at(&p.score, tick)
    }

    /// Pitches whose notes are sounding at the playback position (keys held down
    /// in the score; the pedal may keep them ringing longer).
    pub fn sounding_pitches(&self) -> Vec<Pitch> {
        let (Some(p), Some(tick)) = (self.playing.as_ref(), self.player.position()) else {
            return Vec::new();
        };
        p.score
            .notes
            .iter()
            .filter(|n| n.start <= tick && tick < n.end())
            .map(|n| n.pitch)
            .collect()
    }

    // ---- periodic -----------------------------------------------------------

    /// Call regularly (e.g. every 100 ms): playback state, autosave, file watching.
    pub fn tick(&mut self) {
        let now = Instant::now();
        self.pressed
            .retain(|(_, t)| now.duration_since(*t) < PRESS_GLOW);
        if self.playing.is_some() && !self.player.is_playing() {
            self.playing = None;
        }
        for e in self.player.take_errors() {
            self.output.warn(format!("audio: {e}"));
        }
        self.tick_keys(now);
        self.autosave(now);
        if now.duration_since(self.last_watch) >= WATCH_EVERY {
            self.last_watch = now;
            self.watch();
        }
    }

    fn autosave(&mut self, now: Instant) {
        let every = StdDuration::from_secs(self.project.config.editor.autosave_secs.max(1));
        for i in 0..self.buffers.len() {
            let b = &self.buffers[i];
            if !b.dirty || b.readonly {
                continue;
            }
            let idle = b
                .last_edit
                .is_some_and(|t| now.duration_since(t) >= RECOVERY_IDLE);
            let due = b.recovery_at.is_none_or(|t| now.duration_since(t) >= every);
            if (idle && !b.recovery_written) || (due && !b.recovery_written) {
                let (name, text) = (b.name.clone(), b.text());
                match self.project.write_recovery(&name, &text) {
                    Ok(()) => {
                        let b = &mut self.buffers[i];
                        b.recovery_written = true;
                        b.recovery_at = Some(now);
                    }
                    Err(e) => self.output.warn(format!("recovery copy: {e}")),
                }
            }
        }
    }

    fn watch(&mut self) {
        for i in 0..self.buffers.len() {
            if !self.buffers[i].stamp.changed() {
                continue;
            }
            let name = self.buffers[i].name.clone();
            if self.buffers[i].dirty {
                self.output.warn(format!(
                    "{name} changed outside, but has unsaved changes here — `reload` to load it"
                ));
                continue;
            }
            match std::fs::read_to_string(&self.buffers[i].path) {
                Ok(text) => {
                    self.buffers[i].set_text(&text);
                    self.recompile(i);
                    self.output
                        .info(format!("{name} reloaded (changed outside)"));
                }
                Err(_) => self
                    .output
                    .warn(format!("{name} was removed or renamed outside")),
            }
        }
    }

    // ---- views --------------------------------------------------------------

    /// Lines of the Tutorial view for the current step, with the check result.
    pub fn tutorial_page(&self) -> Page {
        let steps = crate::tutorial::steps(self.lang);
        let Some(step) = steps.get(self.tutorial_step) else {
            return Page::default();
        };
        let mut lines = step.lines.clone();
        if let Some(expect) = &step.expect {
            let done = crate::tutorial::satisfied(&self.buf().compiled.score, expect);
            lines.push(if done {
                "Check    ✔ done — `tutorial next`".into()
            } else {
                "Check    … not yet".into()
            });
        }
        lines.push(String::new());
        lines.push(format!(
            "step {}/{} · tutorial next | prev | <n>",
            self.tutorial_step + 1,
            steps.len()
        ));
        Page {
            title: format!("STEP {} · {}", self.tutorial_step + 1, step.title),
            lines,
        }
    }

    /// Builtin function under the cursor (for the Ref view).
    pub fn function_at_cursor(&self) -> Option<&'static piano_lang::BuiltinInfo> {
        let b = self.buf();
        let line: Vec<char> = b.line().chars().collect();
        let mut i = b.cursor.col.min(line.len());
        // Search backwards for `name(` on this line.
        while i > 0 {
            if line[i - 1] == '(' {
                let end = i - 1;
                let mut start = end;
                while start > 0 && line[start - 1].is_ascii_alphabetic() {
                    start -= 1;
                }
                let name: String = line[start..end].iter().collect();
                if let Some(info) = piano_lang::builtins::info(&name) {
                    return Some(info);
                }
            }
            i -= 1;
        }
        let word: String = line[b.cursor.col.min(line.len())..]
            .iter()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        piano_lang::builtins::info(&word)
    }
}

/// `1/8`, `/8`, `8`, `1/4.` → Duration.
pub fn parse_len(s: &str) -> Option<Duration> {
    let s = s.trim();
    let dots = s.chars().rev().take_while(|c| *c == '.').count();
    let s = &s[..s.len() - dots];
    let denom = s
        .strip_prefix("1/")
        .or_else(|| s.strip_prefix('/'))
        .unwrap_or(s);
    Duration::new(denom.parse().ok()?, dots as u8)
}

/// `3/4` → TimeSig.
pub fn parse_time(s: &str) -> Option<TimeSig> {
    let (a, b) = s.trim().split_once('/')?;
    let beats: u8 = a.parse().ok()?;
    let unit: u8 = b.parse().ok()?;
    (beats >= 1 && piano_core::time::DENOMINATORS.contains(&unit))
        .then_some(TimeSig { beats, unit })
}

/// `m:ss`
pub fn clock(secs: f64) -> String {
    let s = secs.max(0.0).round() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Display path relative to the project root when inside it.
pub fn short_path(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(rel) if rel.as_os_str().is_empty() => ".".into(),
        Ok(rel) => rel.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_parsing() {
        assert_eq!(parse_len("1/8"), Duration::new(8, 0));
        assert_eq!(parse_len("/8"), Duration::new(8, 0));
        assert_eq!(parse_len("8"), Duration::new(8, 0));
        assert_eq!(parse_len("1/4."), Duration::new(4, 1));
        assert_eq!(parse_len("3"), None);
    }

    #[test]
    fn time_parsing() {
        assert_eq!(parse_time("3/4"), Some(TimeSig { beats: 3, unit: 4 }));
        assert_eq!(parse_time("3/5"), None);
    }
}
