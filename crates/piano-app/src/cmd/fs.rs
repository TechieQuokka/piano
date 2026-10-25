//! ls, cd, pwd, cat, cp, mv, rm, mkdir — implemented in-app with the sandbox rules.

use std::path::Path;

use piano_project::Access;

use crate::app::{App, Pending, short_path};

/// Lines shown by `cat` at most.
const CAT_LINES: usize = 500;

impl App {
    fn shown(&self, path: &Path) -> String {
        short_path(&self.project.root, path)
    }

    pub(crate) fn cmd_ls(&mut self, args: &[&str]) {
        let arg = args.first().copied().unwrap_or(".");
        let dir = match self.sandbox.check(&self.cwd, Path::new(arg), Access::Read) {
            Ok(p) => p,
            Err(e) => {
                self.output.error(format!("ls: {e}"));
                return;
            }
        };
        if dir.is_file() {
            self.output.info(self.shown(&dir));
            return;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            self.output.error(format!("ls: cannot read {arg}"));
            return;
        };
        let mut items: Vec<(bool, String)> = entries
            .flatten()
            .map(|e| {
                (
                    e.path().is_dir(),
                    e.file_name().to_string_lossy().into_owned(),
                )
            })
            .filter(|(_, n)| !n.starts_with('.'))
            .collect();
        items.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        if items.is_empty() {
            self.output.info("(empty)");
        }
        let line: Vec<String> = items
            .iter()
            .map(|(d, n)| {
                let open = self.buffers.iter().any(|b| b.path == dir.join(n));
                format!(
                    "{n}{}{}",
                    if *d { "/" } else { "" },
                    if open { "*" } else { "" }
                )
            })
            .collect();
        if !line.is_empty() {
            self.output.list(&line);
        }
    }

    pub(crate) fn cmd_cd(&mut self, args: &[&str]) {
        let arg = args.first().copied().unwrap_or("");
        let target = if arg.is_empty() {
            self.project.root.clone()
        } else {
            match self.sandbox.check(&self.cwd, Path::new(arg), Access::Read) {
                Ok(p) => p,
                Err(e) => {
                    self.output.error(format!("cd: {e}"));
                    return;
                }
            }
        };
        if !target.is_dir() {
            self.output.error(format!("cd: {arg} is not a directory"));
            return;
        }
        self.cwd = target;
        self.cmd_pwd();
    }

    pub(crate) fn cmd_pwd(&mut self) {
        let inside = self.cwd.starts_with(self.sandbox.root());
        self.output.info(format!(
            "{}{}",
            self.cwd.display(),
            if inside {
                ""
            } else {
                "  (outside project: read-only)"
            }
        ));
    }

    pub(crate) fn cmd_cat(&mut self, args: &[&str]) {
        let [arg] = args else {
            self.output.error("usage: cat <file>");
            return;
        };
        let path = match self.sandbox.check(&self.cwd, Path::new(arg), Access::Read) {
            Ok(p) => p,
            Err(e) => {
                self.output.error(format!("cat: {e}"));
                return;
            }
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let total = text.lines().count();
                for l in text.lines().take(CAT_LINES) {
                    self.output.info(format!("  {l}"));
                }
                if total > CAT_LINES {
                    self.output
                        .info(format!("  … {} more lines", total - CAT_LINES));
                }
            }
            Err(e) => self.output.error(format!("cat: {e}")),
        }
    }

    pub(crate) fn cmd_cp(&mut self, args: &[&str]) {
        let [src, dst] = args else {
            self.output.error("usage: cp <src> <dst>");
            return;
        };
        let src_path = match self.sandbox.check(&self.cwd, Path::new(src), Access::Read) {
            Ok(p) if p.is_file() => p,
            Ok(_) => {
                self.output.error(format!("cp: {src} is not a file"));
                return;
            }
            Err(e) => {
                self.output.error(format!("cp: {e}"));
                return;
            }
        };
        // `cp a dir/` copies into the directory.
        let mut dst = dst.to_string();
        if self.resolve(&dst).is_dir()
            && let Some(name) = src_path.file_name()
        {
            dst = format!("{}/{}", dst.trim_end_matches('/'), name.to_string_lossy());
        }
        let bytes = match std::fs::read(&src_path) {
            Ok(b) => b,
            Err(e) => {
                self.output.error(format!("cp: {e}"));
                return;
            }
        };
        match self.write_new_or_inside(&dst, &bytes) {
            Ok(p) => self.output.ok(format!(
                "copied {} → {}",
                self.shown(&src_path),
                self.shown(&p)
            )),
            Err(e) => self.output.error(format!("cp: {e}")),
        }
    }

    pub(crate) fn cmd_mv(&mut self, args: &[&str]) {
        let [src, dst] = args else {
            self.output.error("usage: mv <src> <dst>");
            return;
        };
        let checked = (
            self.sandbox
                .check(&self.cwd, Path::new(src), Access::Modify),
            self.sandbox
                .check(&self.cwd, Path::new(dst), Access::Modify),
        );
        let (from, mut to) = match checked {
            (Ok(a), Ok(b)) => (a, b),
            (Err(e), _) | (_, Err(e)) => {
                self.output.error(format!("mv: {e}"));
                return;
            }
        };
        if !from.exists() {
            self.output
                .error(format!("mv: {src}: no such file or directory"));
            return;
        }
        if to.is_dir()
            && let Some(name) = from.file_name()
        {
            to = to.join(name);
        }
        if let Err(e) = std::fs::rename(&from, &to) {
            self.output.error(format!("mv: {e}"));
            return;
        }
        for b in self.buffers.iter_mut().filter(|b| b.path == from) {
            b.path = to.clone();
            if let Some(stem) = to.file_stem() {
                b.name = stem.to_string_lossy().into_owned();
            }
            b.stamp = piano_project::FileStamp::new(&to);
        }
        self.output
            .ok(format!("moved {} → {}", self.shown(&from), self.shown(&to)));
    }

    pub(crate) fn cmd_rm(&mut self, args: &[&str]) {
        let [arg] = args else {
            self.output.error("usage: rm <file>");
            return;
        };
        match self
            .sandbox
            .check(&self.cwd, Path::new(arg), Access::Modify)
        {
            Ok(p) if p.is_file() => {
                let q = format!("remove {}?", self.shown(&p));
                self.ask(Pending::Remove(p), &q);
            }
            Ok(p) if p.is_dir() => self.output.error("rm: only files can be removed"),
            Ok(_) => self.output.error(format!("rm: {arg}: no such file")),
            Err(e) => self.output.error(format!("rm: {e}")),
        }
    }

    pub(crate) fn remove_file(&mut self, path: &Path) {
        // Re-check: the pending answer may come later.
        if !path.starts_with(self.sandbox.root()) {
            self.output.error("rm: outside project is read-only");
            return;
        }
        match std::fs::remove_file(path) {
            Ok(()) => {
                let names: Vec<String> = self
                    .buffers
                    .iter()
                    .filter(|b| b.path == path)
                    .map(|b| b.name.clone())
                    .collect();
                for n in names {
                    self.close_buffer(&n);
                }
                self.output.ok(format!("removed {}", self.shown(path)));
            }
            Err(e) => self.output.error(format!("rm: {e}")),
        }
    }

    pub(crate) fn cmd_mkdir(&mut self, args: &[&str]) {
        let [arg] = args else {
            self.output.error("usage: mkdir <dir>");
            return;
        };
        match self
            .sandbox
            .check(&self.cwd, Path::new(arg), Access::Modify)
        {
            Ok(p) => match std::fs::create_dir_all(&p) {
                Ok(()) => self.output.ok(format!("created {}/", self.shown(&p))),
                Err(e) => self.output.error(format!("mkdir: {e}")),
            },
            Err(e) => self.output.error(format!("mkdir: {e}")),
        }
    }
}
