//! Display commands (played, ref, output, song, help, man, tutorial), lang, font.

use std::path::Path;

use piano_project::{Access, Lang};

use crate::app::{App, Page};
use crate::focus::View;

impl App {
    /// `played`, `ref`, `output`, `song`: show that view (takes no arguments).
    pub(crate) fn cmd_show(&mut self, view: View, args: &[&str]) {
        if args.is_empty() {
            self.set_view(view);
        } else {
            self.output.error(format!("usage: {}", view.name()));
        }
    }

    pub(crate) fn cmd_help(&mut self, args: &[&str]) {
        match crate::help::help(args) {
            Some(lines) => {
                let title = if args.is_empty() {
                    "HELP".to_string()
                } else {
                    format!("HELP · {}", args.join(" "))
                };
                self.page = Page { title, lines };
                self.set_view(View::Help);
            }
            None => self
                .output
                .error(format!("no help for `{}` — type `help`", args.join(" "))),
        }
    }

    pub(crate) fn cmd_man(&mut self, args: &[&str]) {
        match crate::manual::manual(self.lang).lookup(args) {
            Some((title, lines)) => {
                self.page = Page {
                    title: format!("MAN · {title}"),
                    lines,
                };
                self.set_view(View::Man);
            }
            None => self.output.error(format!(
                "no manual section `{}` — type `man`",
                args.join(" ")
            )),
        }
    }

    pub(crate) fn cmd_lang(&mut self, args: &[&str]) {
        match args.first().and_then(|a| Lang::parse(a)) {
            Some(lang) => {
                self.lang = lang;
                self.global.ui.lang = lang;
                if self.user_files == crate::app::UserFiles::ReadWrite
                    && let Err(e) = piano_project::recent::save_global(&self.global)
                {
                    self.output.warn(format!("lang: not saved ({e})"));
                }
                self.output
                    .info(format!("manual / tutorial language: {}", lang.code()));
                if self.view == View::Man {
                    let title = self
                        .page
                        .title
                        .trim_start_matches("MAN · ")
                        .to_ascii_lowercase();
                    let args: Vec<&str> = if title == "manual" {
                        vec![]
                    } else {
                        vec![title.as_str()]
                    };
                    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
                    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
                    self.cmd_man(&refs);
                }
            }
            None => self.output.error("usage: lang en|ko"),
        }
    }

    pub(crate) fn cmd_tutorial(&mut self, args: &[&str]) {
        let n = crate::tutorial::steps(self.lang).len();
        match args {
            [] => {}
            ["next"] => self.tutorial_step = (self.tutorial_step + 1).min(n - 1),
            ["prev"] => self.tutorial_step = self.tutorial_step.saturating_sub(1),
            [k] => match k.parse::<usize>() {
                Ok(k) if (1..=n).contains(&k) => self.tutorial_step = k - 1,
                _ => {
                    self.output
                        .error(format!("usage: tutorial [next|prev|1-{n}]"));
                    return;
                }
            },
            _ => {
                self.output
                    .error(format!("usage: tutorial [next|prev|1-{n}]"));
                return;
            }
        }
        self.set_view(View::Tutorial);
    }

    pub(crate) fn cmd_font(&mut self, args: &[&str]) {
        let [arg] = args else {
            match &self.soundfont {
                Some(p) => self.output.info(format!("soundfont {}", p.display())),
                None => self
                    .output
                    .info("no SoundFont loaded — `font <path to .sf2>`"),
            }
            return;
        };
        let path = match self.sandbox.check(&self.cwd, Path::new(arg), Access::Read) {
            Ok(p) => p,
            Err(e) => {
                self.output.error(format!("font: {e}"));
                return;
            }
        };
        match self.player.load_soundfont(&path) {
            Ok(()) => {
                self.project.config.sound.soundfont = Some(path.clone());
                if let Err(e) = self.project.save_config() {
                    self.output
                        .warn(format!("font: not saved to piano.toml ({e})"));
                }
                self.output.ok(format!("soundfont {}", path.display()));
                self.soundfont = Some(path);
            }
            Err(e) => self.output.error(format!("font: {e}")),
        }
    }
}
