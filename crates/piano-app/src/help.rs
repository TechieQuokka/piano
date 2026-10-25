//! Command table (for help and completion) and the hierarchical help text.

pub struct CmdSpec {
    pub group: &'static str,
    /// Command words, e.g. `export midi`.
    pub path: &'static str,
    pub usage: &'static str,
    pub summary: &'static str,
    pub detail: &'static [&'static str],
}

macro_rules! spec {
    ($group:expr, $path:expr, $usage:expr, $summary:expr $(, $detail:expr)* $(,)?) => {
        CmdSpec {
            group: $group,
            path: $path,
            usage: $usage,
            summary: $summary,
            detail: &[$($detail),*],
        }
    };
}

pub const COMMANDS: &[CmdSpec] = &[
    spec!(
        "PLAYBACK",
        "play",
        "play [lines|here|song|<piece>]",
        "Play current piece, lines, cursor, song, or a piece",
        "play          from the start",
        "play 2-4      lines 2 to 4 (also 3, 4-)",
        "play here     from the cursor (F5)",
        "play song     the whole arrangement",
        "play p2       another piece"
    ),
    spec!(
        "PLAYBACK",
        "loop",
        "loop <lines>",
        "Loop line range until stop",
        "e.g. loop 2-4"
    ),
    spec!("PLAYBACK", "stop", "stop", "Stop playback (F6)"),
    spec!(
        "PLAYBACK",
        "listen",
        "listen <notes...>",
        "Preview notes without writing",
        "e.g. listen C4 E G  or  listen {C4 E G}"
    ),
    spec!(
        "FILE",
        "new",
        "new <name>",
        "Create a new piece and open it"
    ),
    spec!(
        "FILE",
        "open",
        "open <file|piece>",
        "Open a piece or file (switches if open)",
        "Files outside the project open read-only."
    ),
    spec!(
        "FILE",
        "close",
        "close [piece]",
        "Close a piece (asks if unsaved)"
    ),
    spec!(
        "FILE",
        "save",
        "save [file]",
        "Save current piece (inside project only)"
    ),
    spec!(
        "FILE",
        "reload",
        "reload",
        "Reload file changed by an external editor"
    ),
    spec!(
        "FILE",
        "recover",
        "recover",
        "Restore piece from its recovery copy"
    ),
    spec!(
        "FILE",
        "export",
        "export midi|wav [file]",
        "Export current piece or song"
    ),
    spec!(
        "FILE",
        "export midi",
        "export midi [file]",
        "Export as MIDI (.mid)",
        "file   output path (default: <piece>.mid)",
        "Outside the project only new files are written."
    ),
    spec!(
        "FILE",
        "export wav",
        "export wav [file]",
        "Export as audio (.wav)",
        "file   output path (default: <piece>.wav)",
        "Needs a SoundFont (see font)."
    ),
    spec!("FILESYSTEM", "ls", "ls [path]", "List files"),
    spec!("FILESYSTEM", "cd", "cd <path>", "Change directory"),
    spec!("FILESYSTEM", "pwd", "pwd", "Print current directory"),
    spec!("FILESYSTEM", "cat", "cat <file>", "Show file contents"),
    spec!(
        "FILESYSTEM",
        "cp",
        "cp <src> <dst>",
        "Copy file (outside: new files only)"
    ),
    spec!(
        "FILESYSTEM",
        "mv",
        "mv <src> <dst>",
        "Move or rename (project only)"
    ),
    spec!(
        "FILESYSTEM",
        "rm",
        "rm <file>",
        "Remove file, asks first (project only)"
    ),
    spec!(
        "FILESYSTEM",
        "mkdir",
        "mkdir <dir>",
        "Make directory (project only)"
    ),
    spec!("EDIT", "undo", "undo", "Undo last edit"),
    spec!("EDIT", "redo", "redo", "Redo last undone edit"),
    spec!("EDIT", "history", "history", "Show edit history"),
    spec!("EDIT", "goto", "goto <line>", "Move cursor to line"),
    spec!(
        "INPUT",
        "dyn",
        "dyn <dynamic>",
        "Set how loud the keys sound (ppp..fff, off = mf)"
    ),
    spec!(
        "INPUT",
        "oct",
        "oct <n>",
        "Move playable range to octave n (0-7)"
    ),
    spec!(
        "INPUT",
        "ped",
        "ped on|off",
        "Insert damper pedal at cursor"
    ),
    spec!(
        "INPUT",
        "sos",
        "sos on|off",
        "Insert sostenuto pedal at cursor"
    ),
    spec!(
        "INPUT",
        "soft",
        "soft on|off",
        "Insert soft pedal at cursor"
    ),
    spec!(
        "PROJECT",
        "project",
        "project [new|open|recent]",
        "Show current project"
    ),
    spec!(
        "PROJECT",
        "project new",
        "project new <dir>",
        "Create project in any folder (keeps existing files)"
    ),
    spec!(
        "PROJECT",
        "project open",
        "project open <dir|n>",
        "Switch project (n = number from recent)"
    ),
    spec!(
        "PROJECT",
        "project recent",
        "project recent",
        "List recently opened projects"
    ),
    spec!(
        "VIEW & SYSTEM",
        "played",
        "played",
        "Show the notes played on the keys",
        "The Display keeps a view until another view command."
    ),
    spec!(
        "VIEW & SYSTEM",
        "ref",
        "ref",
        "Show builtins and the state at the Code cursor",
        "The Display keeps a view until another view command."
    ),
    spec!(
        "VIEW & SYSTEM",
        "output",
        "output",
        "Show command results and alerts",
        "The Display keeps a view until another view command."
    ),
    spec!(
        "VIEW & SYSTEM",
        "song",
        "song",
        "Show the pieces and the song arrangement",
        "The Display keeps a view until another view command."
    ),
    spec!(
        "VIEW & SYSTEM",
        "clear",
        "clear [played]",
        "Clear the output log (or the played notes)"
    ),
    spec!(
        "VIEW & SYSTEM",
        "help",
        "help [topic] [sub]",
        "Show help (try: help keys)"
    ),
    spec!(
        "VIEW & SYSTEM",
        "man",
        "man [chapter] [section]",
        "Show language manual"
    ),
    spec!(
        "VIEW & SYSTEM",
        "lang",
        "lang en|ko",
        "Switch manual / tutorial language"
    ),
    spec!(
        "VIEW & SYSTEM",
        "tutorial",
        "tutorial [next|prev|n]",
        "Start or move in the tutorial"
    ),
    spec!(
        "VIEW & SYSTEM",
        "font",
        "font <path>",
        "Load SoundFont (.sf2)"
    ),
    spec!("VIEW & SYSTEM", "quit", "quit", "Quit (asks if unsaved)"),
];

const KEYS: &[&str] = &[
    "KEYS",
    "  F12 / Shift+F12   cycle focus Keys → Code → Cmd",
    "  F2 / F3 / F4      jump to Keys / Code / Cmd",
    "  F5 / F6           play from cursor / stop",
    "  Esc               cancel a question (y/n)",
    "  PgUp / PgDn       scroll the Display (Keys, Cmd focus)",
    "",
    "  The Display keeps its view; change it with a command:",
    "    played ref output song help man tutorial",
    "",
    "  Keys focus        listen only (nothing is written)",
    "    Z X C V B N M   white keys (lower octave)   S D G H J  black",
    "    Q W E R T Y U   white keys (upper octave)   2 3 5 6 7  black",
    "    hold a key      it keeps sounding, fades when let go",
    "    ← →             octave down / up",
    "",
    "  Code focus        type freely; Tab indents; PgUp/PgDn scroll",
    "  Cmd focus         Tab completes; ↑ ↓ command history",
];

/// Help text for `help [topic] [sub]`; `None` if the topic is unknown.
pub fn help(args: &[&str]) -> Option<Vec<String>> {
    let topic = args.join(" ").to_ascii_lowercase();
    if topic.is_empty() {
        let mut out = Vec::new();
        let mut group = "";
        for c in COMMANDS.iter().filter(|c| !c.path.contains(' ')) {
            if c.group != group {
                if !group.is_empty() {
                    out.push(String::new());
                }
                group = c.group;
                out.push(group.to_string());
            }
            out.push(format!("  {:<10}{}", c.path, c.summary));
        }
        out.push(String::new());
        out.push(
            "Type 'help <topic>' for details, 'help keys' for keys, 'man' for the language.".into(),
        );
        return Some(out);
    }
    if topic == "keys" {
        return Some(KEYS.iter().map(|s| s.to_string()).collect());
    }
    if topic == "lang" || topic == "language" {
        return Some(vec![
            "The piano language is described in the manual.".into(),
            "  man              chapters".into(),
            "  man octave       one section".into(),
            "  lang ko          Korean descriptions".into(),
        ]);
    }
    let subs: Vec<&CmdSpec> = COMMANDS
        .iter()
        .filter(|c| c.path.starts_with(&format!("{topic} ")))
        .collect();
    let exact = COMMANDS.iter().find(|c| c.path == topic);
    match (exact, subs.is_empty()) {
        (Some(c), true) => {
            let mut out = vec![c.usage.to_string(), format!("  {}", c.summary)];
            out.extend(c.detail.iter().map(|d| format!("  {d}")));
            Some(out)
        }
        (_, false) => {
            let mut out: Vec<String> = subs
                .iter()
                .map(|c| format!("  {:<16}{}", c.path, c.summary))
                .collect();
            out.push(format!("Type 'help {topic} <sub>' for details."));
            Some(out)
        }
        (None, true) => None,
    }
}

/// Top-level command names (for completion).
pub fn command_names() -> Vec<&'static str> {
    let mut v: Vec<&str> = COMMANDS
        .iter()
        .filter(|c| !c.path.contains(' '))
        .map(|c| c.path)
        .collect();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels() {
        let top = help(&[]).unwrap();
        assert!(top.iter().any(|l| l.contains("export")));
        assert!(!top.iter().any(|l| l.contains("export midi")));
        let export = help(&["export"]).unwrap();
        assert!(export[0].contains("export midi"));
        let midi = help(&["export", "midi"]).unwrap();
        assert_eq!(midi[0], "export midi [file]");
        assert!(help(&["nope"]).is_none());
    }

    #[test]
    fn every_command_has_one_entry() {
        let mut paths: Vec<&str> = COMMANDS.iter().map(|c| c.path).collect();
        let n = paths.len();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), n);
    }
}
