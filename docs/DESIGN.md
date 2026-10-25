# piano — Design Document

> Status: implemented (version 1.18.1, 2026-10-25). This document was the baseline;
> §13 lists where the implementation differs.

## Table of Contents

1. [Overview](#1-overview)
2. [UI](#2-ui)
3. [Key Bindings](#3-key-bindings)
4. [Cmd Commands](#4-cmd-commands)
5. [Piano Language](#5-piano-language)
6. [Pieces and Song](#6-pieces-and-song)
7. [Alerts](#7-alerts)
8. [Help and Tutorial](#8-help-and-tutorial)
9. [Architecture](#9-architecture)
10. [Dependency Policy](#10-dependency-policy)
11. [Settings, Sound, and Files](#11-settings-sound-and-files)
12. [Open Topics](#12-open-topics)

---

## 1. Overview

`piano` is a terminal (TUI) **piano composition editor** written in Rust.

- **Not a live instrument.** The goal is to try notes, hear them, write them down,
  edit, and play the result back.
- **Target user:** someone with no piano or music theory knowledge who composes
  freely by ear.
- **Sound:** a real piano via SoundFont (`.sf2`).
- **Source of truth:** a small, score-like text language (the *piano language*).
  Keys only sound (listening); the user writes what they heard in the Code area
  (§2.4, changed after 1.13.0).

### Core principles

| Principle | Meaning |
|---|---|
| Continuity | Code reads left to right like a score. State (octave, length, dynamic, pedals) carries over to following notes. |
| Simplicity | Few, simple rules. No user-defined functions; only builtin transforms and adjacent repeats. |
| Leniency | The language never fails as a whole. Unknown parts become alerts; the rest still plays. |
| Safety | Outside the project folder, nothing existing is ever modified. |

---

## 2. UI

### 2.1 Layout

Changed in 1.11.0: the keyboard is always visible at the top, the Display moved to
the right, and the piano roll was removed (little value in real time).

```
┌─ Keys (always) ──────────────────────────────────────────────────┐
│ A0 █▄██▄█▄██ … playable range highlighted … ██▄█ C8               │
│        S   D       G   H   J       2   3       5   6   7          │
│ │C3│ … │ Z │ X │ C │ V │ B │ N │ M │ Q │ W │ … │ U │ … │C6│        │
└──────────────────────────────────────────────────────────────────┘
┌─ Code ─────────────────────────────┐┌─ Display ──────────────────┐
│  1   │ tempo 100                   ││ (ref | output | help | man │
│  2   │ |: mf C4 E G C5/2 :| x2 up( ││  | song | tutorial)        │
│  ↪   │ 12){ C E G }                ││                            │
│  3 ⚠ │ C4 F# G                     ││ tall and narrow: long      │
│  4   │ ▌                           ││ lines are wrapped          │
└────────────────────────────────────┘└────────────────────────────┘
┌─ Cmd ────────────────────────────────────────────────────────────┐
│ > play 2-3                                     ▶ playing p1 ...  │
└──────────────────────────────────────────────────────────────────┘
 [Code]  keys C4-B5 (mf) · pedal off · p1* · my_song · EN
```

| Area | Role |
|---|---|
| Keys | Top, always visible. 88-key map plus a zoomed keyboard that shows more octaves as the terminal widens (§13.10). Below 26 rows only the map line is shown. |
| Code | Left (55%). Piano language text with line numbers and an alert gutter; long lines wrap on screen only (`↪`), line numbers are unchanged. In-app editing is the default; changes from an external editor are auto-reloaded. |
| Display | Right (45%). One view at a time; long lines are wrapped. |
| Cmd | Single input line; the last result is shown on its bottom border. Results go to the Display `output` view. |
| Status bar | Focus, playable octave, key loudness, pedals, piece, project, UI language (`EN` / `KO`). |

### 2.2 Display views

| View | Content |
|---|---|
| `played` | Notes played on the Keys (name and 도레미), newest last; `clear played` empties it. |
| `ref` | Builtin function list, details of the function under the cursor, and the inherited state at the cursor (octave reference, length, dynamic, pedals). |
| `output` | Command results and alerts (with line numbers), scrollable. |
| `help` | Hierarchical help (commands, keys). |
| `man` | Language manual, one section at a time (§8.2). |
| `song` | Pieces in the project and the song arrangement. |
| `tutorial` | Step-by-step tutorial. |

Display rules (1.18.0):

1. The Display only shows things; the only control in it is scrolling (`PgUp` / `PgDn`).
2. It does not depend on the focus: moving focus (`F2`–`F4`, `F12`) and `Esc` never
   change the view.
3. It keeps the last view until another view is opened, and a view is opened only by
   its command: `played`, `ref`, `output`, `song`, `help`, `man`, `tutorial`.

At startup the Display shows `output`, so startup messages are visible.

### 2.3 Keys sketch

```
 A0                          88 keys                                  C8
 ▌▐▌▐▐▌▐▌▐▌▐▐▌▐▌▐▐▌▐▌▐▌▐▐▌▐▌[▐▐▌▐▌▐▌▐▐▌▐▌▐▌▐▐▌▐▌]▐▐▌▐▌▐▌▐▐▌▐▌▐▐▌▐▌▐▌▐▐▌▐▌▐
┌─ Playable: C4 ~ B5 ────────────────────────────────────────┐
│  ┃ S ┃ D ┃   ┃ G ┃ H ┃ J ┃   ┃ 2 ┃ 3 ┃   ┃ 5 ┃ 6 ┃ 7 ┃      │
│ │ Z │ X │ C │ V │ B │ N │ M │ Q │ W │ E │ R │ T │ Y │ U │    │
│ │ 도│ 레│ 미│ 파│ 솔│ 라│ 시│ 도│ 레│ 미│ 파│ 솔│ 라│ 시│    │
│ │ C │ D │ E │ F │ G │ A │ B │ C │ D │ E │ F │ G │ A │ B │    │
└────────────────────────────────────────────────────────────┘
```

### 2.4 Listening (Keys focus)

- Pressing a piano key only sounds the note; nothing is written to the code.
- A held key keeps sounding and fades when let go (exact with the kitty keyboard
  protocol, inferred from auto-repeats elsewhere).
- Played notes are listed in the `played` view, so they can be written in Code (F3).
- Loudness of the keys: `dyn <dynamic>`. `listen <notes...>` plays notes from Cmd.

| Key (Keys focus) | Action |
|---|---|
| Piano keys | Sound the note (listed in `played`) |
| `←` `→` | Shift the playable range by one octave |

---

## 3. Key Bindings

| Key | Action |
|---|---|
| `F12` / `Shift+F12` | Cycle focus forward / backward (Keys → Code → Cmd) |
| `F2` / `F3` / `F4` | Jump to Keys / Code / Cmd |
| `F5` | Play from cursor |
| `F6` | Stop |
| `Esc` | Cancel a pending question (y/n); clear the Cmd line (Cmd focus). The Display is not changed. |
| `PgUp` `PgDn` | Scroll the Display (Keys / Cmd focus); scroll the code (Code focus) |
| `Tab` | Autocomplete (Cmd), indent (Code) |
| `←` `→` | Shift playable range by one octave (Keys focus); clamps at A0 / C8; starts near C4 |
| `↑` `↓` | Command history (Cmd focus) |
| `Z X C V B N M` / `S D G H J` | Lower octave white / black keys |
| `Q W E R T Y U` / `2 3 5 6 7` | Upper octave white / black keys |

- `F1` is avoided (commonly captured by terminals as help).
- Everything else is done through Cmd commands.
- To verify on real terminals: F-key capture (e.g. drop-down terminals use `F12`),
  `Shift+F12` detection.

---

## 4. Cmd Commands

Notation: `<required>`, `[optional]`, `a|b` (one of).
Line ranges: `3`, `2-4`, `4-` (to end). Partial playback is **line-based only**.

```
PLAYBACK
  play                    Play current piece from start
  play <lines>            Play line range (3, 2-4, 4-)
  play here               Play from cursor (F5)
  play song               Play whole song arrangement
  play <piece>            Play a specific piece
  loop <lines>            Loop line range
  stop                    Stop playback (F6)
  listen <notes...>       Preview notes without writing

FILE
  new <name>              Create a new piece
  open <file>             Open a piece or song file (switches if already open)
  close [piece]           Close a piece (warns if unsaved)
  save [file]             Save current piece
  reload                  Reload file changed by an external editor
  recover                 Restore piece from its recovery copy
  export midi [file]      Export as MIDI (.mid)
  export wav [file]       Export as audio (.wav)

FILESYSTEM
  ls [path]               List files
  cd <path>               Change directory
  pwd                     Print current directory
  cat <file>              Show file contents
  cp <src> <dst>          Copy file (outside: new files only)
  mv <src> <dst>          Move or rename (project only)
  rm <file>               Remove file, with confirm (project only)
  mkdir <dir>             Make directory (project only)

EDIT
  undo                    Undo last edit
  redo                    Redo last undone edit
  history                 Show edit history
  goto <line>             Move cursor to line

INPUT
  dyn <dynamic>           Set how loud the keys sound (ppp..fff, off = mf)
  oct <n>                 Move playable range to octave n
  ped on|off              Insert damper pedal at cursor
  sos on|off              Insert sostenuto pedal at cursor
  soft on|off             Insert soft pedal at cursor

PROJECT
  project                 Show current project
  project new <dir>       Create project in any folder (keeps existing files)
  project open <dir>      Switch to another project (needs piano.toml)
  project recent          List recently opened projects

VIEW & SYSTEM
  played                  Show the notes played on the keys
  ref                     Show builtins and the state at the Code cursor
  output                  Show command results and alerts
  song                    Show the pieces and the song arrangement
  clear [played]          Clear the output log (or the played notes)
  help [topic] [sub]      Show help
  man [chapter] [section] Show language manual
  lang en|ko              Switch manual / tutorial language
  tutorial                Start tutorial
  font <path>             Load SoundFont (.sf2)
  quit                    Quit (warns if unsaved)
```

7 groups, 46 commands (`clear` added in 1.10.0; `len` removed after 1.13.0;
`view <name>` replaced by `played` / `ref` / `output` / `song` in 1.18.0).

### 4.1 Filesystem commands

Linux-like commands are **implemented in-app** (not passed to a shell), so their
behavior and safety rules are fully controlled.

### 4.2 Outside-project permissions

| Operation | Inside project | Outside project |
|---|---|---|
| Read / navigate (`ls cd pwd cat open`) | ✅ | ✅ |
| Copy outside → inside (`cp`) | ✅ | ✅ |
| Copy / export inside → outside (`cp`, `export`) | ✅ | ✅ new files only |
| Overwrite existing file | ✅ | ❌ |
| `rm`, `mv`, `mkdir`, `save` | ✅ | ❌ |
| `project new <dir>` | ✅ | ✅ (explicit exception; never touches existing files) |

- Paths are canonicalized before checking (blocks `..` and symlink escapes).
- Denials are reported in one line, e.g. `✖ rm: outside project is read-only`.
- Switching projects (`project open`) moves the writable root to the new project.

---

## 5. Piano Language

A score-like, left-to-right language. Whitespace and case are lenient.

### 5.1 Notes

| Syntax | Meaning | Example |
|---|---|---|
| `C D E F G A B` | Note names (lowercase accepted) | `C D E` |
| `#` / `b` | Sharp / flat | `C#` `Bb` |
| `0`–`8` | Octave (scientific pitch; `C4` = middle C; range A0–C8) | `C4` |
| `/n` | Length: `1 2 4 8 16 32 64` (others = Error) | `C/8` |
| `.` / `..` | Dotted / double-dotted (max 2; more = Warning) | `C/4.` |
| `~` | Tie (same pitch only; may cross bars; works on chords) | `G/2~ G/4` |
| `r` | Rest | `r/4` |
| `{ }` | Chord; length after the brace | `{C E G}/2` |
| `//` | Line comment | `C D // intro` |

Flat parsing rule: the first character is the note letter; an attached `b` is a flat.
`bb` = B♭ (one note); `b b` = two B notes.

### 5.2 Inheritance (continuity)

| Property | Rule | Default |
|---|---|---|
| Octave | **Nearest-note**: omitted octave picks the octave closest to the previous note. Explicit number resets the reference. A 6-semitone tie goes **up** and emits a Warning. | Reference `C4` |
| Length | Inherited until changed, **including dots**. Rests use and update it. | `/4` |
| Dynamic | Inherited until the next dynamic mark. | `mf` |
| Pedals | On/off state persists until toggled. | all off |

Reference details:

- After a rest, the reference is the last note before the rest.
- After a chord, the reference is the chord's first note; notes inside a chord resolve
  against the previous note in the chord.
- Inside a builtin function, octaves are resolved on the written text **before** the
  transform is applied, so functions do not disturb the surrounding flow.

```
C4/8 D E F G/4 A B C          // last C resolves to C5 (nearest to B4)
C/4. D E F/8 G                // D, E are dotted quarters; G is an eighth
```

### 5.3 Header and structure

| Syntax | Meaning | Default |
|---|---|---|
| `tempo n` | Quarter notes per minute; may change mid-piece | `100` |
| `time a/b` | Time signature; may change mid-piece | `4/4` |
| `\|` | Bar line (optional; length mismatch = Warning) | – |

### 5.4 Dynamics

| Syntax | Meaning |
|---|---|
| `ppp pp p mp mf f ff fff` | Very soft … very loud |
| `cresc` | Gradually louder until the next dynamic mark |
| `dim` | Gradually softer until the next dynamic mark |

Keywords are used instead of `<` / `>` to avoid clashing with song connectors.

### 5.5 Pedals

| Syntax | Pedal |
|---|---|
| `ped on` / `ped off` | Damper (sustain), right |
| `sos on` / `sos off` | Sostenuto, middle — holds only notes sounding when engaged |
| `soft on` / `soft off` | Soft (una corda), left |

Pedals are toggles, not blocks, so their ranges may overlap freely. The Cmd commands
`ped on` etc. insert exactly this text.

### 5.6 Repeats (adjacent only)

| Syntax | Meaning | Example |
|---|---|---|
| `\|: :\|` | Repeat (plays twice) | `\|: C D E F :\|` |
| `x n` | Repeat count | `\|: C D :\| x3` |
| `%` | Repeat previous bar | `\| C D E F \| % \|` |
| `[1` `[2` | Voltas (different endings) | `\|: C D [1 E :\| [2 G \|` |

Long-distance jumps (D.C., D.S., coda) and user-defined motifs are intentionally
excluded: reuse in scores is local.

### 5.7 Builtin transform functions

Syntax: `name(args){ ... }`. Builtins only (no user-defined functions, so recursion is
impossible).

| Function | Args | Effect | Example |
|---|---|---|---|
| `up(n)` | semitones | Raise section | `up(12){ C E G }` |
| `down(n)` | semitones | Lower section | `down(2){ G A B }` |
| `accent(rule)` | `1/n` or note name | Emphasize matching notes | `accent(1/3){ ... }`, `accent(G){ ... }` |
| `louder(n)` | velocity | Raise section volume | `louder(20){ C D }` |
| `softer(n)` | velocity | Lower section volume | `softer(20){ C D }` |
| `tuplet(n)` | note count | n notes in the time of the nearest lower power of two | `tuplet(3){ C/8 D E }` |
| `seq(times, step)` | count, semitones | Sequence: repeat shifted by step | `seq(3, 1){ C D E }` → C D E, D E F, E F G |

Rules:

- **Nesting** is allowed; transforms apply **inner → outer**.
- Nesting depth limit: about 32 (exceeding = Warning, never a hard stop).
- **Octave folding**: results outside A0–C8 are moved by octaves until in range
  (note name preserved) and reported as a Warning.

### 5.8 Example

```
tempo 90  time 4/4            // header
p cresc
| C4/8 D E F G/4 A~ | A/2 f {C5 E G}/2 |
ped on
|: up(12){ accent(1/3){ C/8 D E F G A } } r/4 :| x2
ped off
```

---

## 6. Pieces and Song

### 6.1 Project folder

Any folder becomes a project when it contains `piano.toml`.

```
my_song/
├── piano.toml      // project settings (name, soundfont, defaults)
├── song.piano      // arrangement
├── p1.piano        // independent piece
└── bridge1.piano   // user-made connecting piece
```

- Each piece is **independent**: its own tempo, defaults, and pedal state.
- Recent projects are stored in `~/.config/piano/`.

### 6.2 Song arrangement

```
p1 > bridge1 > p4 > fade > p7 > r/2 > up(2){ p1 } > ped > ending
```

| Syntax | Meaning |
|---|---|
| `<piece>` | Play a piece (may appear multiple times) |
| `>` | Continue directly |
| `fade` | Fade out the previous piece into the next |
| `r/n` | Rest between pieces |
| `ped` | Carry pedal resonance into the next piece |
| `fn(...){ piece }` | Apply builtin transforms to a piece (original file unchanged) |

User-made connecting pieces are just ordinary pieces placed between others.

---

## 7. Alerts

| Level | Marker | Meaning | Examples |
|---|---|---|---|
| Error | `✖` | That part cannot be played | Unknown syntax, invalid length |
| Warning | `⚠` | Plays, but may differ from intent | Octave folding, ambiguous octave, bar mismatch |
| Hint | `·` | Information | How an omitted value was resolved |

- Each alert has a line number, the chosen result, and how to fix it:
  `⚠ line 3: C4 → F# is ambiguous (6 semitones), chose F#4. Write F#3 or F#4 to fix.`
- The Code gutter shows the most severe marker per line; details are in `output`.

---

## 8. Help and Tutorial

### 8.1 Help

Hierarchical, compact English, shown in the Display `help` view.

```
> help                    top-level topics, one line each
> help export             subcommands of export
> help export midi        usage, arguments, example
```

Topics cover commands and keys (`help keys`). `help lang` points to `man`.
Help text is English only.

### 8.2 Language manual (`man`)

A grammar reference: rules first, minimal examples. Shown section by section in the
Display `man` view.

```
man                 List chapters
man octave          Show the "octave" section
man fn accent       Show one builtin function
```

Chapters:

```
1. Basics        file, lines, comments, whitespace, case
2. Notes         names, accidentals, octave, length, dots, ties, rests, chords
3. Inheritance   octave (nearest), length, dynamic, pedals, defaults
4. Structure     bars, time, tempo
5. Dynamics      marks, cresc, dim, velocity table
6. Pedals        ped, sos, soft
7. Repeats       |: :|, x n, %, voltas
8. Functions     syntax, nesting, folding, one section per builtin
9. Song          pieces, >, fade, r, ped, transforms
10. Alerts       Error / Warning / Hint list
Appendix         one-page grammar summary
```

Section format:

```
OCTAVE
  Syntax   <note><0-8>                  C4  A0  C8
  Range    A0 – C8 (88 keys). C4 = middle C. Number changes at C.
  Omitted  Nearest to previous note. Start reference: C4.
           Tie (6 semitones) → up, Warning.
  Resets   Writing a number sets a new reference.
  Rests    Do not change the reference.
  Chords   First note becomes the reference.
  Example  C4 D E F G A B C   → last C = C5
  See      inheritance, fn up, fn down
```

Fields: `Syntax`, rule lines, exactly one `Example`, `See` (related sections).

### 8.3 Tutorial

- `tutorial` opens the Display `tutorial` view; it stays while the user moves focus to
  follow the steps.
- Steps go from basics (screen, keys, entering notes) to length, chords, pedals,
  repeats, and functions.
- Each step: short explanation → example to follow → next.
- **Light checking**: the program checks the user's code and gives hints, but every
  step can be skipped.
- Recommends writing one or two bars per line (partial playback is line-based).

```
STEP 3 · LENGTH
  Add /n after a note to set its length. Bigger n = shorter.
  The length carries over until changed.
  Try      C4/8 D E F G/4
  Check    ✔ 4 eighths + 1 quarter
  Hint     D E F inherit /8. G switches to /4.
```

### 8.4 Korean mode

| Target | `lang en` | `lang ko` |
|---|---|---|
| Manual (`man`) | English | Titles and field names in English; descriptions in Korean |
| Tutorial | English | Titles and step names in English; descriptions and hints in Korean |
| Command help | English | English |
| Alerts | English | English |
| Code, keywords, examples | English | English |

- Set with `lang en|ko` or `[ui] lang = "ko"` in the global config.
- Korean text is laid out using display width (CJK characters are two cells wide).

### 8.5 Documentation files

```
docs/
├── DESIGN.md
├── LANGUAGE.md        # manual, English (source of truth)
├── LANGUAGE.ko.md     # manual, Korean descriptions
├── TUTORIAL.md        # tutorial, English
└── TUTORIAL.ko.md     # tutorial, Korean descriptions
```

- All four are embedded in the binary (e.g. `include_str!`).
- English and Korean files share the same section / step IDs; a test fails if the ID
  sets differ.
- Bodies are written **during implementation**, matching actual behavior: each
  language feature adds its manual section in both languages.
- Every `Example` is also run as a test, so the manual cannot drift from behavior.
- The tutorial is written once enough language features exist.

---

## 9. Architecture

### 9.1 Principles

1. **Pure language core.** Text → score processing knows nothing about UI, audio, or files.
2. **State separate from rendering.** `piano-app` is terminal-agnostic and testable headless.
3. **One-way dependencies.** Lower crates never depend on higher ones.

### 9.2 Workspace

Virtual workspace, edition 2024, resolver 3. Shared `[workspace.package]`,
`[workspace.dependencies]`, and `[workspace.lints]` (`unsafe_code = "forbid"`).

```
piano/
├── Cargo.toml                    # virtual workspace
├── rust-toolchain.toml           # latest stable
├── docs/DESIGN.md
└── crates/
    ├── piano-core/               # shared types
    │   └── src/ time.rs pitch.rs score.rs alert.rs
    ├── piano-lang/               # text → Score
    │   ├── src/ lexer.rs token.rs parser.rs ast.rs resolve.rs expand.rs
    │   │        song.rs linemap.rs builtins/{mod,pitch,dynamics,rhythm}.rs
    │   └── tests/ cases/*.piano golden.rs
    ├── piano-audio/              # Score → sound
    │   └── src/ synth.rs player.rs scheduler.rs render.rs
    ├── piano-export/             # Score → MIDI / WAV
    │   └── src/ midi.rs wav.rs
    ├── piano-project/            # project, config, sandbox, recent, watch
    │   └── src/ project.rs config.rs sandbox.rs recent.rs watch.rs
    ├── piano-app/                # state, buffers, history, commands, help, tutorial
    │   └── src/ app.rs buffer.rs history.rs input.rs focus.rs output.rs
    │            help.rs manual.rs tutorial.rs i18n.rs
    │            cmd/{mod,playback,file,fs,edit,input,project,view}.rs
    ├── piano-tui/                # rendering + input only
    │   └── src/ event.rs keymap.rs layout.rs
    │            widgets/{keyboard,code,status}.rs
    │            views.rs (played, ref, output, help/man/tutorial, song)
    └── piano-cli/                # binary `piano`
        └── src/ main.rs
```

### 9.3 Dependency graph

```
            piano-cli (bin: piano)
                │
            piano-tui
                │
            piano-app ───────────┬──────────────┬───────────────┐
                │                │              │               │
           piano-lang      piano-audio ◀─ piano-export    piano-project
                │                │              │
                └───────── piano-core ──────────┘
```

`piano-export` depends on `piano-audio` for offline rendering (WAV).
Headless CLI subcommands (`play`, `export`) use `piano-app` without `piano-tui`.

### 9.4 Core types (`piano-core`)

```rust
pub type Tick = u64;
/// Divisible by 2^6, 3, 5, 7: exact for /64, triplets, quintuplets, septuplets.
pub const TICKS_PER_QUARTER: Tick = 6720;

pub struct Pitch(u8);                 // MIDI number 21 (A0) ..= 108 (C8)

pub enum PedalKind { Damper, Sostenuto, Soft }

pub struct NoteEvent  { pub pitch: Pitch, pub start: Tick, pub len: Tick,
                        pub velocity: u8, pub src: SrcSpan }
pub struct PedalEvent { pub kind: PedalKind, pub on: bool, pub at: Tick, pub src: SrcSpan }
pub struct TempoEvent { pub at: Tick, pub bpm: u16 }

pub struct Score {
    pub notes: Vec<NoteEvent>, pub pedals: Vec<PedalEvent>,
    pub tempos: Vec<TempoEvent>, pub end: Tick,
}

pub enum Severity { Error, Warning, Hint }
pub struct SrcSpan { pub line: u32, pub col: u32, pub len: u32 }
pub struct Alert   { pub severity: Severity, pub span: SrcSpan,
                     pub message: String, pub fix: Option<String> }
```

Every event carries its `SrcSpan`, which drives the playing-line highlight and alert
placement.

### 9.5 Language pipeline (`piano-lang`)

```
 text ─▶ lexer ─▶ parser ─▶ resolve ─▶ expand ─▶ builtins ─▶ Score
        (tokens)   (AST)   (inherit)  (repeats)  (transform)
           └──────────── alerts collected at every stage ────────────┘
```

```rust
pub struct Compiled { pub score: Score, pub alerts: Vec<Alert>, pub linemap: LineMap }

pub fn compile_piece(src: &str) -> Compiled;
pub fn compile_song(src: &str, load: &dyn Fn(&str) -> Option<String>) -> Compiled;

/// Inherited state at a cursor position (Ref view, play from cursor).
pub struct State { pub octave_ref: Pitch, pub len: Tick, pub dots: u8,
                   pub dynamic: Dynamic, pub pedals: [bool; 3] }
pub fn state_at(src: &str, line: u32, col: u32) -> State;
```

Compilation never fails; it always returns a `Score` plus alerts.

### 9.6 Audio (`piano-audio`)

```rust
impl Player {
    pub fn new(soundfont: &Path) -> Result<Player>;
    pub fn play(&self, score: Arc<Score>, from: Tick, to: Option<Tick>);
    pub fn play_loop(&self, score: Arc<Score>, from: Tick, to: Tick);
    pub fn preview(&self, pitches: &[Pitch], velocity: u8);
    pub fn stop(&self);
    pub fn position(&self) -> Option<Tick>;
}
pub fn render(score: &Score, soundfont: &Path, sample_rate: u32) -> Vec<f32>;
```

- Playback runs on a dedicated audio thread; the UI only sends messages.
- Playing from the middle (line range or cursor) reproduces exactly what a full
  playback sounds like at that point: inherited state, active pedals, and enclosing
  repeats / functions.
- Sostenuto: if the SoundFont synth does not support it, `synth.rs` records notes
  sounding at engage time and sustains them itself.

### 9.7 Project and sandbox (`piano-project`)

```rust
impl Project {
    pub fn create(dir: &Path) -> Result<Project>;   // keeps existing files
    pub fn open(dir: &Path) -> Result<Project>;     // requires piano.toml
}

pub enum Access { Read, CreateNew, Modify }
impl Sandbox {
    /// Canonicalizes the path, then applies the outside-project rules (§4.2).
    pub fn check(&self, path: &Path, access: Access) -> Result<PathBuf, Denied>;
}
```

All file operations must go through `Sandbox::check`.

### 9.8 App (`piano-app`)

```rust
pub struct App {
    pub project: Project, pub sandbox: Sandbox,
    pub buffers: Vec<Buffer>, pub active: usize,
    pub focus: Focus, pub view: View,
    pub input: InputDefaults, pub output: Vec<OutputLine>,
    pub player: Player,
}

pub struct Buffer {
    pub path: PathBuf, pub lines: Vec<String>, pub cursor: (u32, u32),
    pub dirty: bool, pub history: History, pub compiled: Compiled,
}

pub fn parse(line: &str) -> Result<Command, ParseError>;
impl App {
    pub fn execute(&mut self, cmd: Command);           // results → self.output
    pub fn on_piano_key(&mut self, key: PianoKey);     // sound only (listen)
}
```

### 9.9 TUI (`piano-tui`)

```
 event.rs (crossterm key / tick) ─▶ keymap.rs (key + focus → action)
     ─▶ piano-app (on_piano_key / execute) ─▶ layout + widgets + views: draw(&App)
```

### 9.10 CLI (`piano-cli`)

```
piano edit [dir]                              # open TUI (default: current dir)
piano play <dir|file> [--lines 2-4]
piano export <dir|file> --midi|--wav [-o out]
piano new <dir>                               # create project
piano doctor                                  # check SoundFont, audio device, terminal keys
```

### 9.11 Example flow: pressing `C` in Keys focus

```
1. tui/event     KeyCode::Char('c')
2. tui/keymap    focus == Keys → PianoKey::White(2)
3. app/input     playable octave 4 → Pitch E4
4. audio/player  preview([E4])
5. app/buffer    insert "E/4 " at cursor; history.push(Edit::Insert)
6. lang          compile_piece() → new Score + alerts
7. app/input     cursor advances by 1/4
8. tui           redraw Keys (highlight), Code, Display, status
```

---

## 10. Dependency Policy

- Always use the **latest** crate versions.
- Never take versions or APIs from memory: check crates.io / `cargo search`, add with
  `cargo add`, and read current docs.rs documentation and changelogs before coding.
  APIs of `ratatui`, `crossterm`, and `cpal` in particular change between releases.
- Verify by building against the actual latest versions; check for outdated
  dependencies regularly (e.g. `cargo outdated`).
- Versions live only in the root `Cargo.toml` (`[workspace.dependencies]`); this
  document lists names and purposes only.

| Purpose | Crate |
|---|---|
| TUI | `ratatui` (its re-exported `ratatui::crossterm`, so versions always match) |
| Text width | `unicode-width` |
| SoundFont synthesis | `rustysynth` |
| Audio output | `cpal` (optional feature, see §13) |
| MIDI / WAV writing | `midly` / `hound` |
| CLI arguments | `clap` |
| Config | `serde`, `toml`, `dirs` |
| Errors | `thiserror` |

File watching uses modification-time polling instead of `notify` (§13).

---

## 11. Settings, Sound, and Files

### 11.1 SoundFont

- **Not bundled** (large files, varying licenses); the user provides one.
- On first run, common system paths (e.g. `/usr/share/sounds/sf2/`) are searched.
- If none is found, editing still works without sound, and `output` explains how to
  get one. Candidates to suggest: free piano SoundFonts such as Salamander Grand Piano
  or FluidR3 (licenses to be re-verified at implementation time).
- Set via `font <path>`, `piano.toml`, or the global config.

### 11.2 Configuration

Precedence: piece header (`tempo`, `time`) > `piano.toml` > global
`~/.config/piano/config.toml` > builtin defaults.

The global config additionally holds user-level settings such as `[ui] lang = "en"`.

```toml
# piano.toml
[project]
name  = "my_song"
entry = "song.piano"

[sound]
soundfont = "/usr/share/sounds/sf2/piano.sf2"   # optional, overrides global

[defaults]
tempo   = 100
time    = "4/4"
length  = "1/4"
dynamic = "mf"
octave  = 4

[editor]
autosave_secs = 30
```

### 11.3 Autosave and recovery

- Original files change **only on `save`**.
- A recovery copy is written to `.piano/recovery/<piece>.piano` every
  `autosave_secs` and when editing goes idle.
- On open, if the recovery copy is newer than the file, `output` reports it;
  `recover` restores it.
- External edits: if the buffer has no unsaved changes, the file is reloaded
  automatically; otherwise nothing is overwritten, a Warning is shown, and `reload`
  loads the external version.

### 11.4 Multiple pieces

- Several pieces may be open; one is shown at a time (no tab bar).
- `open <piece>` switches to it if already open; `close [piece]` closes (warns if
  unsaved).
- The `song` view marks open pieces with `*`; the status bar shows the current piece.

### 11.5 Terminal compatibility

- `piano doctor` checks the SoundFont, audio device, and terminal keys (F2–F6, F12,
  `Shift+F12`) and suggests fixes (e.g. disabling a terminal's F12 binding).
- Key-release events are not required (editor, not a live instrument).

### 11.6 Velocity mapping (MIDI 1–127)

| Mark | ppp | pp | p | mp | mf | f | ff | fff |
|---|---|---|---|---|---|---|---|---|
| Velocity | 16 | 33 | 49 | 64 | 80 | 96 | 112 | 127 |

- `accent`: +20 (capped at 127).
- `louder(n)` / `softer(n)`: ±n, clamped to 1–127.
- `cresc` / `dim`: linear per-note interpolation between the two marks.
- Soft pedal: uses the synth's support if available; otherwise reduces velocity by
  about 20%.

---

## 12. Open Topics

None at the moment.

---

## 13. Implementation Notes

Where the implementation differs from or adds to the design above.

### 13.1 Audio output is an optional feature

- `cpal` needs the ALSA development package on Linux (`libasound2-dev`).
- `piano-audio` has a `device` feature; `piano-cli` exposes it as `audio`:
  `cargo build --release --features audio`.
- Without it, everything else works (editing, MIDI export, WAV rendering with a
  SoundFont); the app reports that there is no audio output.
- `Player::silent()` never opens a device (tests, headless tools).

### 13.2 Pedals in the synthesizer

`rustysynth` supports only the damper pedal (CC 64). Sostenuto (captures the keys
down when pressed, defers their release) and soft (velocity × 0.8) are implemented in
`piano-audio/src/synth.rs`. MIDI export writes CC 64 / 66 / 67.

### 13.3 Scheduling order

At the same instant: note-offs, pedal releases, note-ons, pedal presses. So
`sos on C2` catches C2. Starting in the middle strikes notes still sounding and
presses pedals that are down at that point.

### 13.4 File watching

Each open buffer keeps a modification-time stamp, checked once per second in
`App::tick` (no background thread, no `notify` dependency).

### 13.5 User-level files

`App::new` (editor) reads `~/.config/piano/config.toml` and records recent projects;
`App::for_tool` (`piano play`, `piano export`) only reads; `App::headless` (tests)
touches nothing. Tests create projects only under the workspace `target/`.

### 13.6 Editing details

- A buffer opens with the cursor at the end, ready to continue.
- Notes entered while the cursor is inside a `//` comment go to a new line.
- Keys never write to the code (since the listen-only change after 1.13.0); `dyn` only sets how loud they sound.
- Export functions return bytes; the app writes them through the sandbox
  (inside: overwrite allowed; outside: `create_new` only). `piano export` never
  overwrites.

### 13.7 Tutorial checking

`Expect` notes must appear consecutively in the piece with the same intervals,
lengths, and relative timing (octave may differ, since earlier steps move the
octave reference).

### 13.8 Language details settled during implementation

- A lone lowercase `f` is forte; the note F is `F`, `f4`, or `f/4`.
- Repeats and `%` copy laid-out events, so every pass sounds identical.
- `:|` without `|:` repeats from the start (Hint). The last volta ends at the next
  `|`, `|:`, or the end.
- Song pieces are independent: each starts with its own tempo / time signature, and
  pedals left down are released at the piece end.
- `fade` scales velocities over the last bar (to 30%); `ped` holds the damper from a
  quarter before the join to a quarter after.

### 13.9 `piano doctor`

Reports the SoundFont search, the opened audio device (name, rate, channels,
format), keyboard-enhancement support, and runs an interactive key test (F2–F6, F12,
Shift+F12; Shift+F12 may arrive as F24).

Sound check (1.9.0): plays C-E-G through the key-preview path and the playback path,
prints the signal level that reached the device (dBFS), and asks whether it was
heard. No signal → audio pipeline problem; signal but not heard → output device or
volume.

### 13.10 Keyboard width (1.13.0)

The zoomed keyboard always contains the playable two octaves and adds whole octaves
on both sides while they fit (key cells 4–6 columns, never beyond A0–C8). Keys
outside the playable range are dimmed and unlabeled except C with its octave number.

### 13.11 Audio level and diagnostics (1.8.2)

- rustysynth's default master volume (0.5) left a single note around -24 dBFS, easy
  to miss. Master volume is 1.6 with a soft clipper (linear below 0.6, smooth toward
  ±1 above), so chords never hard-clip.
- The editor prints the opened device at startup and shows audio stream errors in
  the output (they were silently dropped before).
