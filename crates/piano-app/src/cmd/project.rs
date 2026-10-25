//! project, project new / open / recent.

use std::path::Path;

use piano_project::Project;

use crate::app::{App, Pending};

impl App {
    pub(crate) fn cmd_project(&mut self, args: &[&str]) {
        match args {
            [] => {
                let pieces = self.project.pieces();
                self.output.info(format!(
                    "{}  {}\n{} pieces: {}",
                    self.project.name(),
                    self.project.root.display(),
                    pieces.len(),
                    pieces.join(" ")
                ));
            }
            ["new", dir] => {
                let path = self.resolve(dir);
                match Project::create(&path) {
                    Ok(p) => {
                        self.output
                            .ok(format!("created project {}", p.root.display()));
                        self.request_switch(&p.root);
                    }
                    Err(e) => self.output.error(format!("project new: {e}")),
                }
            }
            ["open", arg] => {
                let path = match arg.parse::<usize>() {
                    Ok(n) if n >= 1 && n <= self.recent.paths.len() => {
                        self.recent.paths[n - 1].clone()
                    }
                    _ => self.resolve(arg),
                };
                match Project::open(&path) {
                    Ok(p) => self.request_switch(&p.root),
                    Err(e) => self.output.error(format!("project open: {e}")),
                }
            }
            ["recent"] => {
                if self.recent.paths.is_empty() {
                    self.output.info("no recent projects");
                }
                let lines: Vec<String> = self
                    .recent
                    .paths
                    .iter()
                    .enumerate()
                    .map(|(i, p)| format!("{:>2}  {}", i + 1, p.display()))
                    .collect();
                for l in lines {
                    self.output.info(l);
                }
                self.output.info("`project open <n>` opens one");
            }
            _ => self
                .output
                .error("usage: project [new <dir> | open <dir|n> | recent]"),
        }
    }

    fn request_switch(&mut self, dir: &Path) {
        if self.buffers.iter().any(|b| b.dirty) {
            self.ask(
                Pending::SwitchProject(dir.to_path_buf()),
                "unsaved changes will be lost — switch project anyway?",
            );
        } else {
            self.switch_project(dir);
        }
    }

    pub(crate) fn switch_project(&mut self, dir: &Path) {
        match Project::open(dir) {
            Ok(p) => {
                self.stop();
                self.sandbox = piano_project::Sandbox::new(&p.root);
                self.project = p;
                self.enter_project();
            }
            Err(e) => self.output.error(format!("project: {e}")),
        }
    }
}
