//! new, open, close, save, reload, recover, export.

use std::path::{Path, PathBuf};

use piano_project::{Access, PIECE_EXT};

use crate::app::{App, Pending, short_path};
use crate::buffer::Buffer;

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl App {
    pub(crate) fn cmd_new(&mut self, args: &[&str]) {
        let [name] = args else {
            self.output.error("usage: new <name>");
            return;
        };
        let name = name.trim_end_matches(".piano");
        if !valid_name(name) {
            self.output
                .error("piece names use letters, digits, - and _ (e.g. p2, bridge1)");
            return;
        }
        let path = self.project.piece_path(name);
        if path.exists() {
            self.output
                .error(format!("{name} already exists — `open {name}`"));
            return;
        }
        if let Err(e) = std::fs::write(&path, "") {
            self.output.error(format!("new: {e}"));
            return;
        }
        self.open_buffer(Buffer::new(name, &path, "", false));
        self.output.ok(format!("created {name}.{PIECE_EXT}"));
    }

    pub(crate) fn cmd_open(&mut self, args: &[&str]) {
        let [arg] = args else {
            self.output.error("usage: open <file|piece>");
            return;
        };
        let piece = arg.trim_end_matches(".piano");
        if !arg.contains('/') && valid_name(piece) && self.open_piece(piece) {
            self.output.info(format!("opened {piece}"));
            return;
        }
        let path = match self.sandbox.check(&self.cwd, Path::new(arg), Access::Read) {
            Ok(p) => p,
            Err(e) => {
                self.output.error(format!("open: {e}"));
                return;
            }
        };
        if !path.is_file() {
            self.output.error(format!("open: {arg} is not a file"));
            return;
        }
        if let Some(i) = self.buffers.iter().position(|b| b.path == path) {
            self.active = i;
            return;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                self.output.error(format!("open: {e}"));
                return;
            }
        };
        let inside = path.starts_with(self.sandbox.root());
        let name = path
            .file_stem()
            .map_or_else(|| arg.to_string(), |s| s.to_string_lossy().into_owned());
        self.open_buffer(Buffer::new(&name, &path, &text, !inside));
        if inside {
            self.output
                .info(format!("opened {}", short_path(&self.project.root, &path)));
        } else {
            self.output.warn(format!(
                "opened {} read-only (outside project)",
                path.display()
            ));
        }
    }

    pub(crate) fn cmd_close(&mut self, args: &[&str]) {
        let name = match args {
            [] => self.buf().name.clone(),
            [n] => n.trim_end_matches(".piano").to_string(),
            _ => {
                self.output.error("usage: close [piece]");
                return;
            }
        };
        let Some(i) = self.buffer_index(&name) else {
            self.output.error(format!("{name} is not open"));
            return;
        };
        if self.buffers[i].dirty {
            self.ask(
                Pending::Close(name.clone()),
                &format!("{name} has unsaved changes — close anyway?"),
            );
        } else {
            self.close_buffer(&name);
        }
    }

    pub(crate) fn close_buffer(&mut self, name: &str) {
        let Some(i) = self.buffer_index(name) else {
            return;
        };
        self.buffers.remove(i);
        if self.buffers.is_empty() {
            let first = self.project.pieces().into_iter().next();
            if !first.is_some_and(|p| self.open_piece(&p)) {
                let path = self.project.piece_path("p1");
                self.buffers.push(Buffer::new("p1", &path, "", false));
                self.recompile(0);
            }
        }
        self.active = self.active.min(self.buffers.len() - 1);
        self.output.info(format!("closed {name}"));
    }

    pub(crate) fn cmd_save(&mut self, args: &[&str]) {
        let target: PathBuf = match args {
            [] => self.buf().path.clone(),
            [p] => {
                let p = if Path::new(p).extension().is_none() {
                    format!("{p}.{PIECE_EXT}")
                } else {
                    p.to_string()
                };
                self.resolve(&p)
            }
            _ => {
                self.output.error("usage: save [file]");
                return;
            }
        };
        if !target.starts_with(self.sandbox.root()) {
            self.output.error(
                "save: outside project is read-only — `cp` the file into the project, or `save <name>` inside it",
            );
            return;
        }
        let text = self.buf().text();
        if let Err(e) = std::fs::write(&target, &text) {
            self.output.error(format!("save: {e}"));
            return;
        }
        let i = self.active;
        let b = &mut self.buffers[i];
        let old_name = b.name.clone();
        if target != b.path {
            b.path = target.clone();
            b.name = target
                .file_stem()
                .map_or(old_name.clone(), |s| s.to_string_lossy().into_owned());
            b.readonly = false;
        }
        b.dirty = false;
        b.stamp = piano_project::FileStamp::new(&target);
        let name = b.name.clone();
        self.project.clear_recovery(&old_name);
        self.output
            .ok(format!("saved {}", short_path(&self.project.root, &target)));
        if name != old_name {
            self.recompile(i);
        }
    }

    pub(crate) fn cmd_reload(&mut self) {
        let path = self.buf().path.clone();
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let i = self.active;
                self.buffers[i].set_text(&text);
                self.buffers[i].dirty = false;
                self.buffers[i].stamp.refresh();
                self.recompile(i);
                self.output.ok(format!("reloaded {}", self.buffers[i].name));
            }
            Err(e) => self.output.error(format!("reload: {e}")),
        }
    }

    pub(crate) fn cmd_recover(&mut self) {
        let name = self.buf().name.clone();
        match self.project.newer_recovery(&name) {
            Some(text) => {
                self.edit("recover", false, |b| b.set_text(&text));
                self.output.ok(format!(
                    "recovered {name} — `save` to keep it, `undo` to go back"
                ));
            }
            None => self
                .output
                .info(format!("no newer recovery copy for {name}")),
        }
    }

    pub(crate) fn cmd_export(&mut self, args: &[&str]) {
        let (format, file) = match args {
            [f] => (*f, None),
            [f, file] => (*f, Some(*file)),
            _ => {
                self.output.error("usage: export midi|wav|flac [file]");
                return;
            }
        };
        let ext = match format {
            "midi" | "mid" => "mid",
            "wav" => "wav",
            "flac" => "flac",
            _ => {
                self.output.error("export: format is midi, wav, or flac");
                return;
            }
        };
        let name = self.buf().name.clone();
        let file = file.map_or_else(
            || format!("{}.{ext}", self.project.export_stem(&name)),
            str::to_string,
        );
        let score = self.buf().compiled.score.clone();
        if score.notes.is_empty() {
            self.output.warn(format!("{name} has no notes to export"));
            return;
        }
        let bytes = if ext == "mid" {
            piano_export::midi_bytes(&score)
        } else {
            let Some(sf) = self.player.sound_font().cloned() else {
                self.output
                    .error(format!("export {ext} needs a SoundFont — `font <path>`"));
                return;
            };
            let format = piano_export::AudioFormat::native(&sf);
            let bytes = if ext == "flac" {
                piano_export::flac_bytes(&score, &sf, format)
            } else {
                piano_export::wav_bytes(&score, &sf, format)
            };
            match bytes {
                Ok(b) => b,
                Err(e) => {
                    self.output.error(format!("export: {e}"));
                    return;
                }
            }
        };
        match self.write_new_or_inside(&file, &bytes) {
            Ok(path) => self.output.ok(format!(
                "exported {} ({} KB)",
                short_path(&self.project.root, &path),
                bytes.len().div_ceil(1024)
            )),
            Err(e) => self.output.error(format!("export: {e}")),
        }
    }

    /// Writes `bytes` to `file`: anything inside the project, only new files outside.
    pub(crate) fn write_new_or_inside(&self, file: &str, bytes: &[u8]) -> Result<PathBuf, String> {
        let inside = self.sandbox.is_inside(&self.cwd, Path::new(file));
        let access = if inside {
            Access::Modify
        } else {
            Access::CreateNew
        };
        let path = self
            .sandbox
            .check(&self.cwd, Path::new(file), access)
            .map_err(|e| e.to_string())?;
        if inside {
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        } else {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| e.to_string())?;
            f.write_all(bytes).map_err(|e| e.to_string())?;
        }
        Ok(path)
    }
}
