//! dyn, oct, ped / sos / soft.

use piano_core::Dynamic;

use crate::app::App;
use crate::input::{MAX_OCTAVE, MIN_OCTAVE};

impl App {
    pub(crate) fn cmd_dyn(&mut self, args: &[&str]) {
        match args.first() {
            Some(&"off") => {
                self.input.dynamic = None;
                self.output.info("keys sound at mf");
            }
            Some(a) => match Dynamic::parse(a) {
                Some(d) => {
                    self.input.dynamic = Some(d);
                    self.output.info(format!("keys sound at {d}"));
                }
                None => self.output.error("usage: dyn ppp|pp|p|mp|mf|f|ff|fff|off"),
            },
            None => self.output.error("usage: dyn ppp|pp|p|mp|mf|f|ff|fff|off"),
        }
    }

    pub(crate) fn cmd_oct(&mut self, args: &[&str]) {
        match args.first().and_then(|a| a.parse::<i32>().ok()) {
            Some(o) if (MIN_OCTAVE..=MAX_OCTAVE).contains(&o) => {
                self.input.octave = o;
                self.output.info(format!("keys: C{o} - B{}", o + 1));
            }
            _ => self
                .output
                .error(format!("usage: oct <n> ({MIN_OCTAVE}-{MAX_OCTAVE})")),
        }
    }

    pub(crate) fn cmd_pedal(&mut self, kind: &str, args: &[&str]) {
        match args {
            [state @ ("on" | "off")] => {
                if self.insert_word(&format!("{kind} {state}")) {
                    self.output.info(format!("inserted `{kind} {state}`"));
                }
            }
            _ => self.output.error(format!("usage: {kind} on|off")),
        }
    }
}
