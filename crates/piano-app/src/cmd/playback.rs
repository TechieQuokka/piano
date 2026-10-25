//! play, loop, listen.

use std::sync::Arc;

use super::parse_lines;
use crate::app::{App, clock};

impl App {
    pub(crate) fn cmd_play(&mut self, args: &[&str]) {
        match args {
            [] => {
                let name = self.buf().name.clone();
                self.play_piece(&name, None, false);
            }
            ["here"] => self.play_here(),
            [arg] if parse_lines(arg).is_some() => {
                let name = self.buf().name.clone();
                self.play_piece(&name, parse_lines(arg), false);
            }
            [name] => {
                let name = if *name == "song" {
                    piano_project::SONG.to_string()
                } else {
                    name.trim_end_matches(".piano").to_string()
                };
                self.play_piece(&name, None, false);
            }
            _ => self.output.error("usage: play [lines|here|song|<piece>]"),
        }
    }

    pub(crate) fn cmd_loop(&mut self, args: &[&str]) {
        match args {
            [arg] if parse_lines(arg).is_some() => {
                let name = self.buf().name.clone();
                self.play_piece(&name, parse_lines(arg), true);
            }
            _ => self.output.error("usage: loop <lines>, e.g. loop 2-4"),
        }
    }

    fn play_piece(&mut self, name: &str, lines: Option<(u32, Option<u32>)>, looping: bool) {
        let Some((score, compiled)) = self.score_of(name) else {
            self.output.error(format!("no piece `{name}` (see `ls`)"));
            return;
        };
        for a in &compiled.alerts {
            self.output.alert(a);
        }
        let (from, to) = match lines {
            None => (0, None),
            Some((a, b)) => match piano_lang::line_range(&score, a, b) {
                Some((from, to)) => (from, Some(to)),
                None => {
                    self.output.warn(format!(
                        "no notes on line{} {}",
                        if b == Some(a) { "" } else { "s" },
                        describe(a, b)
                    ));
                    return;
                }
            },
        };
        if score.notes.is_empty() {
            self.output.warn(format!("{name} has no notes yet"));
            return;
        }
        let secs = score.seconds_at(to.unwrap_or(score.end)) - score.seconds_at(from);
        let what = lines.map_or(String::new(), |(a, b)| format!(" lines {}", describe(a, b)));
        self.output.info(format!(
            "{} {name}{what} ({})",
            if looping {
                "⟳ looping"
            } else {
                "▶ playing"
            },
            clock(secs)
        ));
        self.start_play(name, score, from, to, looping);
    }

    pub(crate) fn cmd_listen(&mut self, args: &[&str]) {
        if args.is_empty() {
            self.output
                .error("usage: listen <notes...>, e.g. listen C4 E G");
            return;
        }
        let text = args.join(" ");
        let compiled = piano_lang::compile_piece_with(&text, &self.options());
        for a in &compiled.alerts {
            self.output.alert(a);
        }
        if compiled.score.notes.is_empty() {
            self.output.warn("nothing to play");
            return;
        }
        let names: Vec<String> = compiled
            .score
            .notes
            .iter()
            .map(|n| n.pitch.to_string())
            .collect();
        self.output.info(format!("♪ {}", names.join(" ")));
        self.start_play("", Arc::new(compiled.score), 0, None, false);
    }
}

fn describe(a: u32, b: Option<u32>) -> String {
    match b {
        Some(b) if b == a => a.to_string(),
        Some(b) => format!("{a}-{b}"),
        None => format!("{a}-"),
    }
}
