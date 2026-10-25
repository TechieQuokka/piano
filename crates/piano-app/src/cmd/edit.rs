//! undo, redo, history, goto.

use crate::app::App;

impl App {
    pub(crate) fn cmd_undo(&mut self) {
        let i = self.active;
        match self.buffers[i].history.undo() {
            Some(s) => {
                self.buffers[i].restore(&s);
                self.recompile(i);
                self.output.info("undo");
            }
            None => self.output.info("nothing to undo"),
        }
    }

    pub(crate) fn cmd_redo(&mut self) {
        let i = self.active;
        match self.buffers[i].history.redo() {
            Some(s) => {
                self.buffers[i].restore(&s);
                self.recompile(i);
                self.output.info("redo");
            }
            None => self.output.info("nothing to redo"),
        }
    }

    pub(crate) fn cmd_history(&mut self) {
        let entries: Vec<String> = self.buf().history.entries().map(str::to_string).collect();
        if entries.is_empty() {
            self.output.info("no edits yet");
            return;
        }
        let start = entries.len().saturating_sub(30);
        for (k, e) in entries.iter().enumerate().skip(start) {
            let mark = if k + 1 == entries.len() {
                "  ← current"
            } else {
                ""
            };
            self.output.info(format!("#{:<4} {e}{mark}", k + 1));
        }
    }

    pub(crate) fn cmd_goto(&mut self, args: &[&str]) {
        match args.first().and_then(|a| a.parse::<usize>().ok()) {
            Some(n) if n >= 1 => {
                self.buf_mut().goto(n);
                let line = self.buf().cursor.line + 1;
                self.output.info(format!("line {line}"));
            }
            _ => self.output.error("usage: goto <line>"),
        }
    }
}
