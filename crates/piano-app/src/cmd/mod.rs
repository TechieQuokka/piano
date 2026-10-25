//! Cmd: parsing and running commands, history, and Tab completion.

mod edit;
mod file;
mod fs;
mod input;
mod playback;
mod project;
mod view;

use std::path::{Path, PathBuf};

use crate::app::{App, Pending};
use crate::focus::View;
use crate::output::OutKind;

/// Splits a command line into words; `"..."` keeps spaces.
pub fn split(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut has = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                has = true;
            }
            c if c.is_whitespace() && !quoted => {
                if has {
                    words.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            c => {
                cur.push(c);
                has = true;
            }
        }
    }
    if has {
        words.push(cur);
    }
    words
}

impl App {
    /// Enter in Cmd.
    pub fn submit_cmd(&mut self) {
        let line = self.cmdline.take();
        self.cmd_hist_pos = None;
        if line.trim().is_empty() {
            return;
        }
        if self.cmd_history.last() != Some(&line) {
            self.cmd_history.push(line.clone());
        }
        self.output.push(OutKind::Cmd, format!("> {line}"));
        self.execute(&line);
    }

    pub fn history_prev(&mut self) {
        if self.cmd_history.is_empty() {
            return;
        }
        let i = match self.cmd_hist_pos {
            None => self.cmd_history.len() - 1,
            Some(i) => i.saturating_sub(1),
        };
        self.cmd_hist_pos = Some(i);
        self.cmdline.set(&self.cmd_history[i].clone());
    }

    pub fn history_next(&mut self) {
        match self.cmd_hist_pos {
            Some(i) if i + 1 < self.cmd_history.len() => {
                self.cmd_hist_pos = Some(i + 1);
                self.cmdline.set(&self.cmd_history[i + 1].clone());
            }
            _ => {
                self.cmd_hist_pos = None;
                self.cmdline.set("");
            }
        }
    }

    /// Runs one command line.
    pub fn execute(&mut self, line: &str) {
        let words = split(line);
        let Some(cmd) = words.first().map(|w| w.to_ascii_lowercase()) else {
            return;
        };
        let args: Vec<&str> = words[1..].iter().map(String::as_str).collect();

        if let Some(pending) = self.pending.take() {
            if matches!(cmd.as_str(), "y" | "yes") {
                self.confirm(pending);
            } else {
                self.output.info("cancelled");
                if !matches!(cmd.as_str(), "n" | "no") {
                    self.execute(line);
                }
            }
            return;
        }

        match cmd.as_str() {
            "play" => self.cmd_play(&args),
            "loop" => self.cmd_loop(&args),
            "stop" => {
                self.stop();
                self.output.info("■ stopped");
            }
            "listen" => self.cmd_listen(&args),
            "new" => self.cmd_new(&args),
            "open" => self.cmd_open(&args),
            "close" => self.cmd_close(&args),
            "save" => self.cmd_save(&args),
            "reload" => self.cmd_reload(),
            "recover" => self.cmd_recover(),
            "export" => self.cmd_export(&args),
            "ls" => {
                self.cmd_ls(&args);
                self.set_view(View::Output);
            }
            "cd" => self.cmd_cd(&args),
            "pwd" => {
                self.cmd_pwd();
                self.set_view(View::Output);
            }
            "cat" => {
                self.cmd_cat(&args);
                self.set_view(View::Output);
            }
            "cp" => self.cmd_cp(&args),
            "mv" => self.cmd_mv(&args),
            "rm" => self.cmd_rm(&args),
            "mkdir" => self.cmd_mkdir(&args),
            "undo" => self.cmd_undo(),
            "redo" => self.cmd_redo(),
            "history" => {
                self.cmd_history();
                self.set_view(View::Output);
            }
            "goto" => self.cmd_goto(&args),
            "dyn" => self.cmd_dyn(&args),
            "oct" => self.cmd_oct(&args),
            "ped" | "sos" | "soft" => self.cmd_pedal(&cmd, &args),
            "project" => {
                self.cmd_project(&args);
                // `project` and `project recent` show information; new / open act.
                if matches!(args.as_slice(), [] | ["recent"]) {
                    self.set_view(View::Output);
                }
            }
            "played" => self.cmd_show(View::Played, &args),
            "ref" => self.cmd_show(View::Ref, &args),
            "output" => self.cmd_show(View::Output, &args),
            "song" => self.cmd_show(View::Song, &args),
            "clear" => match args.as_slice() {
                [] => {
                    self.output.lines.clear();
                    self.display_scroll = 0;
                }
                ["played"] => self.played.clear(),
                _ => self.output.error("usage: clear [played]"),
            },
            "help" => self.cmd_help(&args),
            "man" => self.cmd_man(&args),
            "lang" => self.cmd_lang(&args),
            "tutorial" => self.cmd_tutorial(&args),
            "font" => self.cmd_font(&args),
            "quit" | "exit" | "q" => self.cmd_quit(),
            _ => self
                .output
                .error(format!("unknown command `{cmd}` — type `help`")),
        }
    }

    pub(crate) fn ask(&mut self, pending: Pending, question: &str) {
        self.output.warn(format!("{question} (y/n)"));
        self.pending = Some(pending);
    }

    fn confirm(&mut self, pending: Pending) {
        match pending {
            Pending::Quit => self.quit = true,
            Pending::Close(name) => self.close_buffer(&name),
            Pending::Remove(path) => self.remove_file(&path),
            Pending::SwitchProject(dir) => self.switch_project(&dir),
        }
    }

    fn cmd_quit(&mut self) {
        let dirty: Vec<String> = self
            .buffers
            .iter()
            .filter(|b| b.dirty)
            .map(|b| b.name.clone())
            .collect();
        if dirty.is_empty() {
            self.quit = true;
        } else {
            self.ask(
                Pending::Quit,
                &format!("unsaved changes in {} — quit anyway?", dirty.join(", ")),
            );
        }
    }

    /// Resolves a user path against the current directory (no permission check).
    pub(crate) fn resolve(&self, p: &str) -> PathBuf {
        piano_project::sandbox::resolve(&self.cwd, Path::new(p))
    }

    // ---- completion ---------------------------------------------------------

    /// Tab in Cmd: completes the last word. Shows candidates if ambiguous.
    pub fn complete(&mut self) {
        let text = self.cmdline.text.clone();
        let words = split(&text);
        let trailing_space = text.ends_with(' ');
        let (prefix, word_index) = match (words.last(), trailing_space) {
            (Some(w), false) => (w.clone(), words.len() - 1),
            _ => (String::new(), words.len()),
        };
        let candidates: Vec<String> = if word_index == 0 {
            crate::help::command_names()
                .into_iter()
                .map(str::to_string)
                .collect()
        } else {
            self.arg_candidates(&words[0].to_ascii_lowercase(), &prefix)
        };
        let matches: Vec<&String> = candidates
            .iter()
            .filter(|c| c.starts_with(&prefix))
            .collect();
        if matches.is_empty() {
            return;
        }
        let common = common_prefix(&matches);
        if common.len() > prefix.len() {
            let base = &text[..text.len() - prefix.len()];
            let done = matches.len() == 1 && !common.ends_with('/');
            self.cmdline
                .set(&format!("{base}{common}{}", if done { " " } else { "" }));
        } else if matches.len() > 1 {
            let shown: Vec<&str> = matches.iter().take(40).map(|s| s.as_str()).collect();
            self.output.info(shown.join("  "));
        }
    }

    fn arg_candidates(&self, cmd: &str, prefix: &str) -> Vec<String> {
        let words = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        match cmd {
            "lang" => words(&["en", "ko"]),
            "ped" | "sos" | "soft" => words(&["on", "off"]),
            "export" => words(&["midi", "wav"]),
            "dyn" => piano_core::Dynamic::ALL
                .iter()
                .map(|d| d.name().to_string())
                .chain(["off".to_string()])
                .collect(),
            "tutorial" => words(&["next", "prev"]),
            "clear" => words(&["played"]),
            "help" => crate::help::command_names()
                .into_iter()
                .map(str::to_string)
                .chain(["keys".to_string()])
                .collect(),
            "man" => crate::manual::manual(self.lang)
                .sections()
                .map(|(_, s)| s.id.clone())
                .chain(
                    crate::manual::manual(self.lang)
                        .chapters
                        .iter()
                        .map(|c| c.id.clone()),
                )
                .collect(),
            "project" if !prefix.contains('/') => {
                let mut v = words(&["new", "open", "recent"]);
                v.extend(self.path_candidates(prefix));
                v
            }
            "play" | "open" | "close" => {
                let mut v = self.project.pieces();
                if cmd == "play" {
                    v.extend(words(&["here", "song"]));
                }
                // Paths only once the word looks like one, so `open p<Tab>` finds pieces.
                if cmd != "close" && prefix.contains(['/', '.', '~']) {
                    v.extend(self.path_candidates(prefix));
                }
                v
            }
            _ => self.path_candidates(prefix),
        }
    }

    /// Entries of the directory named by `prefix`, as completions of `prefix`.
    fn path_candidates(&self, prefix: &str) -> Vec<String> {
        let (dir_part, _) = match prefix.rfind('/') {
            Some(i) => prefix.split_at(i + 1),
            None => ("", prefix),
        };
        let dir = if dir_part.is_empty() {
            self.cwd.clone()
        } else {
            self.resolve(dir_part)
        };
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut v: Vec<String> = entries
            .flatten()
            .map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let slash = if e.path().is_dir() { "/" } else { "" };
                format!("{dir_part}{name}{slash}")
            })
            .collect();
        v.sort();
        v
    }
}

fn common_prefix(items: &[&String]) -> String {
    let first = items[0];
    let mut len = first.len();
    for s in &items[1..] {
        len = first
            .char_indices()
            .zip(s.chars())
            .take_while(|((_, a), b)| a == b)
            .last()
            .map_or(0, |((i, a), _)| i + a.len_utf8())
            .min(len);
    }
    first[..len].to_string()
}

/// Parses a line range: `3`, `2-4`, `4-`.
pub fn parse_lines(s: &str) -> Option<(u32, Option<u32>)> {
    match s.split_once('-') {
        Some((a, "")) => Some((a.parse().ok()?, None)),
        Some((a, b)) => {
            let (a, b): (u32, u32) = (a.parse().ok()?, b.parse().ok()?);
            (a <= b).then_some((a, Some(b)))
        }
        None => {
            let a = s.parse().ok()?;
            Some((a, Some(a)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitting() {
        assert_eq!(split(r#"cp "a b.piano" c"#), ["cp", "a b.piano", "c"]);
        assert_eq!(split("  play   2-4 "), ["play", "2-4"]);
    }

    #[test]
    fn line_ranges() {
        assert_eq!(parse_lines("3"), Some((3, Some(3))));
        assert_eq!(parse_lines("2-4"), Some((2, Some(4))));
        assert_eq!(parse_lines("4-"), Some((4, None)));
        assert_eq!(parse_lines("4-2"), None);
        assert_eq!(parse_lines("x"), None);
    }

    #[test]
    fn prefixes() {
        let (a, b) = ("export".to_string(), "exit".to_string());
        assert_eq!(common_prefix(&[&a, &b]), "ex");
    }
}
