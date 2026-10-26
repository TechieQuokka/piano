# piano — Design Document

> Status: implemented (version 1.23.0, 2026-10-25). This document was the baseline;
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
| Keys | Top, always visible. A zoomed keyboard that shows more octaves as the terminal widens (§13.10). Below 24 rows it shrinks to one line, `keys C4 - B5  (← → octave)`. The 88-key map line was removed in 1.19.0. |
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
| `guide` | Detailed language guide: contents, a chapter, or one topic (§8.6). |
| `song` | Pieces in the project and the song arrangement. |
| `tutorial` | Step-by-step tutorial. |

Display rules (1.18.0):

1. The Display only shows things; the only control in it is scrolling (`PgUp` / `PgDn`).
2. It does not depend on the focus: moving focus (`F2`–`F4`, `F12`) and `Esc` never
   change the view.
3. It keeps the last view until another view is opened, and a view is opened only by
   a command:
   - view commands: `played`, `ref`, `output`, `song`, `help`, `man`, `guide`,
     `tutorial`;
   - show commands, whose purpose is their result, open `output` (1.23.0): `ls`,
     `pwd`, `cat`, `history`, `project`, `project recent`;
   - action commands (`play`, `save`, `new`, `open`, `undo`, ...) leave the Display
     as it is; their last result is shown on the Cmd border, the full log in `output`.

At startup the Display shows `output`, so startup messages are visible.

### 2.3 Keys sketch

```
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
  export flac [file]      Export as lossless audio (.flac)
                          (default file: <piece>.<ext>; the song is named after
                          the project folder, e.g. la_campanella.flac)

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
  guide [n|id|next|prev]  Show the detailed language guide
  lang en|ko              Switch manual / guide / tutorial language
  tutorial                Start tutorial
  font <path>             Load SoundFont (.sf2)
  quit                    Quit (warns if unsaved)
```

7 groups, 47 commands (`clear` added in 1.10.0; `len` removed after 1.13.0;
`view <name>` replaced by `played` / `ref` / `output` / `song` in 1.18.0; `guide`
added in 1.28.0).

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
| `/n` | Length: `1 2 4 8 16 32 64 128` (others = Error) | `C/8` |
| `.` / `..` | Dotted / double-dotted (max 2; more = Warning) | `C/4.` |
| `~` | Tie (same pitch only; may cross bars and function bodies; works on chords: after `}` every note, after a note inside `{ }` that note alone) | `G/2~ G/4`, `{C~ E G}/2 C` |
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
| `free{ ... }` | Unmetered bar (cadenza): the bar holding it is not checked; no `\|` inside (Error) | – |

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

### 5.8 Voices (1.24.0)

`voices{ ... }{ ... } ...` plays several sections at once, starting together. Piano
music needs this for a melody over an accompaniment over a long bass: a chord `{ }`
gives all its notes one length and one velocity, voices give each its own.

```
voices{ p r/2. G#4/8. G#4/16 }                       // melody
      { pp tuplet(3){ G#3/8 C#4 E4 G#3 C#4 E4 G#3 C#4 E4 G#3 C#4 E4 } }
      { pp {C#2 C#3}/1 }                             // bass
```

| Topic | Rule |
|---|---|
| State | Each voice starts from the state before the block; octave, length, dynamic, cresc are its own |
| After | The first voice's state continues after the block |
| Pedals | Shared by the whole piano; a pedal changed in any voice stays changed after the block |
| Bars | Bar lines inside a voice continue the current bar; bars after the block follow the first voice |
| Length | Different voice lengths → Warning; the block lasts as long as the longest voice |
| Header | `tempo` / `time` in any voice apply to every voice from that point and after the block; voices that differ at the same point → Warning, the first voice's is used |
| Ties | A tie goes in (the first voice starting with that note) and out (from a note ending with the block); otherwise Warning |
| Chord after | A `{ }` right after the block is another voice, unless a length or `~` follows its `}` |

The syntax extends the function form (`name{ ... }`) instead of adding symbols; line
breaks between the bodies are free.

### 5.9 Example

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

Topics cover commands and keys (`help keys`). `help lang` points to `man` and
`guide`.
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
- Since 1.20.0 the reader builds one song, "Twinkle Twinkle Little Star", by typing
  every line into Code: pieces `p1` (A part), `p2` (middle, repeated), `end` (rest and
  chord with pedal), then `song` (`p1 > p2 > up(12){ p1 } > end`). Along the way it
  covers notes, octaves, lengths, bars, playback, tempo, dynamics, repeats, chords,
  pedals, songs, functions, save/export, and a list of more features to try.
- Each step: explanation → the code to write (`Type` = a new line, `Line n` = change
  line n, `Piece` = which piece) → what to listen for → Check ✔ → `tutorial next`.
- A test replays the whole tutorial from a new project: every checked step passes
  right after its edits, and every piece (and the song) compiles without alerts.
- `Shift+PgUp` / `Shift+PgDn` scroll the Display from any focus, so a long step can be
  read while typing in Code.
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
| Guide (`guide`) | English | Titles and subheadings in English; text and code comments in Korean |
| Tutorial | English | Titles and step names in English; descriptions and hints in Korean |
| Solfège (keyboard, Played view) | `Do Re Mi Fa Sol La Ti` | `도 레 미 파 솔 라 시` |
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
├── GUIDE.md           # detailed guide, English
├── GUIDE.ko.md        # detailed guide, Korean
├── TUTORIAL.md        # tutorial, English
└── TUTORIAL.ko.md     # tutorial, Korean descriptions
```

- All six are embedded in the binary (e.g. `include_str!`).
- English and Korean files share the same section / step IDs; a test fails if the ID
  sets differ.
- Bodies are written **during implementation**, matching actual behavior: each
  language feature adds its manual section in both languages.
- Every `Example` is also run as a test, so the manual cannot drift from behavior.
- The tutorial is written once enough language features exist.

### 8.6 Language guide (`guide`, 1.28.0)

`man` is a reference card that fits the Display; `guide` is the in-depth version of
the same language, written to be read while composing (the Display keeps it while
the user works in Code).

```
guide               contents: chapters and their section IDs
guide 3             chapter 3 (also by ID: guide rhythm)
guide octave        one section (also guide fn up = fn.up)
guide next | prev   the next / previous section from the last guide page
```

- Source: `docs/GUIDE.md` / `GUIDE.ko.md`, plain Markdown readable on GitHub.
  `## N. Title {#id}` = chapter, `### Title {#id}` = section, `#### Name` = part of a
  section (Concept, Syntax, Rules, Examples, Common mistakes, See also).
- In the Display: paragraphs are joined and wrapped, `- ` items become bullets,
  code blocks are shown with a `│` bar, inline `**` and backticks are dropped.
- Chapters: Getting started, Pitch, Rhythm, Chords and voices, Dynamics and
  expression, Pedals, Repeats, Functions, Song, Alerts, Reference (grammar,
  defaults, music terms). Every `man` section ID is also a guide section ID, and a
  `man` section ends with `More guide <id>`.
- Tests: same IDs and the same piano / song code in both languages (comments may
  differ); every ` ```piano ` / ` ```song ` block compiles without errors or
  warnings, and a block marked `error`, `warning`, or `hint` must raise that alert;
  every `` `guide x` `` and `` `man x` `` reference resolves.

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
    ├── piano-export/             # Score → MIDI / WAV / FLAC
    │   └── src/ midi.rs wav.rs
    ├── piano-import/             # MusicXML (1.26.0), **kern (1.29.0) → pieces
    │   └── src/ musicxml.rs kern.rs write.rs check.rs rational.rs
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
            piano-cli (bin: piano) ──── piano-import ── piano-lang
                │
            piano-tui
                │
            piano-app ───────────┬──────────────┬───────────────┐
                │                │              │               │
           piano-lang      piano-audio ◀─ piano-export    piano-project
                │                │              │
                └───────── piano-core ──────────┘
```

`piano-export` depends on `piano-audio` for offline rendering (WAV, FLAC).
`piano-import` (1.26.0) only produces text; it compiles its result with `piano-lang`
to check it.
Headless CLI subcommands (`play`, `export`) use `piano-app` without `piano-tui`.

### 9.4 Core types (`piano-core`)

```rust
pub type Tick = u64;
/// Divisible by 2^7, 3^2, 5, 7, 11: exact for double-dotted /128 and tuplets of
/// 3, 5, 6, 7, 9, 10, 11 (1.31.0; was 6720). MIDI files use 6720 per quarter.
pub const TICKS_PER_QUARTER: Tick = 443_520;

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
piano export <dir|file> --midi|--wav|--flac [--rate 48000] [--bits 24] [-o out]
                                              # default out: <piece>.<ext>; a project dir → <folder>.<ext>
piano new <dir>                               # create project
piano import <score.mxl|.krn> <dir>           # MusicXML / **kern → new project (§13.13, §13.15)
piano import <folder> <dir>                    # every score in folder → dir/<file name> (§13.16)
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
| MIDI / WAV / FLAC writing | `midly` / `hound` / `flacenc` (tests decode with `claxon`) |
| MusicXML reading | `roxmltree`, `zip` (`.mxl`, deflate only) |
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

At the same instant: note-offs, pedal releases, soft pedal press, note-ons, damper /
sostenuto presses. So `sos on C2` catches C2, and `soft on C4` softens C4 (the soft
pedal acts when a key is struck; fixed in 1.28.1). MIDI export uses the same order.
Starting in the middle strikes notes still sounding and
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
- `voices` (1.24.0): each voice is laid out as its own track and merged at the block
  start; a voice starting mid-bar carries the part of the bar already played, so its
  bar lines are checked against the real bar.

### 13.9 `piano doctor`

Reports the SoundFont the editor would use and where it comes from (`piano.toml` in
the current folder, the global config, or the folder search; since 1.25.2, before
that only the search), the opened audio device (name, rate, channels,
format), keyboard-enhancement support, and runs an interactive key test (F2–F6, F12,
Shift+F12; Shift+F12 may arrive as F24).

Sound check (1.9.0): plays C-E-G through the key-preview path and the playback path,
prints the signal level that reached the device (dBFS), and asks whether it was
heard. No signal → audio pipeline problem; signal but not heard → output device or
volume.

### 13.10 Keyboard width (1.13.0)

The zoomed keyboard always contains the playable two octaves and adds whole octaves
on both sides while they fit (key cells 4–6 columns, never beyond A0–C8). Keys
outside the playable range are dimmed and unlabeled except C with its octave number
(since 1.25.0 they show their note names and solfege too; only the PC key labels
are left out).
Since 1.19.0 this is the whole Keys area (the 88-key map line above it was removed);
the dimmed octaves and the status bar (`keys C4-B5`) show where the range is.
Since 1.21.0 the keys of the notes being played (`play`, `loop`, `F5`, `play song`)
are drawn pressed while each note lasts, like the keys pressed by the user.
Since 1.22.0 the labeled two octaves follow the playback: when sounding notes leave
the range, it moves by octaves to the lowest of them (within A0-C8), and returns to
the input octave when playback ends. The status bar range follows too; the input
octave (`←` `→`, `oct`) is unchanged.

### 13.11 Audio level and diagnostics (1.8.2)

- rustysynth's default master volume (0.5) left a single note around -24 dBFS, easy
  to miss. Master volume is 1.6 with a soft clipper (linear below 0.6, smooth toward
  ±1 above), so chords never hard-clip.
- The editor prints the opened device at startup and shows audio stream errors in
  the output (they were silently dropped before).

### 13.12 Key input in terminals (1.18.1)

- **Exact mode**: terminals with the kitty keyboard protocol (Ghostty, kitty, WezTerm,
  foot) report releases. The app pushes `DISAMBIGUATE_ESCAPE_CODES |
  REPORT_EVENT_TYPES`; a key sounds from press to release (+0.25 s tail, at least
  0.4 s in all).
- Without `REPORT_ALL_KEYS_AS_ESCAPE_CODES`, repeats of a held text key (Z, X, ...)
  still arrive as plain presses. Since 1.18.1 a press of a key that is still held
  (no release yet) is treated as a repeat; before, it re-struck the note ~30 times
  a second.
- **Inferred mode**: other terminals (e.g. GNOME Terminal / VTE) send no releases.
  Holding is inferred from auto-repeats; a tap is only known after the OS repeat
  delay (~0.5 s), so the shortest note rings about a second.
- `piano doctor` reports which mode the terminal supports.

### 13.13 Importing MusicXML (1.26.0)

`piano import <score.mxl|.musicxml|.xml> <dir>` writes a new project: pieces,
`song.piano`, and `piano.toml` (time signature). Existing files are never
overwritten.

- **Model** (`musicxml.rs`): every part, voice, and staff; times are exact
  fractions of a quarter (`rational.rs`). Repeats and endings are unrolled; chords,
  backup / forward, ties, grace notes, trills, dynamics, wedges, pedals, tempo
  (`<sound tempo>` or the metronome mark), and irregular bars are read. Cue and
  unpitched notes are skipped.
- **Pieces** (`write.rs`): a new piece at double bars, rehearsal marks, and repeat
  jumps; pieces longer than 32 bars are cut every 16. One block per bar, split at
  the half bar when the tempo changes there, merged across bar lines a tie crosses.
  Each MusicXML voice is a voice (overlaps go to an extra voice); pedals and grace
  notes have voices of their own.
- **Lengths**: written lengths are `/1`-`/64` with up to two dots; other lengths go
  into `tuplet(3|5|7)` (e.g. 5 in the time of 6 = dotted 32nds in `tuplet(5)`).
  Rests are split into plain and tuplet parts.
- **Approximations**: grace notes are 64ths just before their note; trills
  alternate with the upper neighbor (key or marked accidental) in 32nds; arpeggios
  are struck together; wedges become `cresc` / `dim` from the level reached so far
  to the mark at their end, or one level on (a swell `<` `>` returns to its level;
  1.29.1); rubato becomes one tempo per block (harmonic mean, so durations are kept). Before
  the first mark the level is mf, like the language and MuseScore.
- **Check** (`check.rs`): the song is compiled and its notes (start, pitch, length)
  compared with the model; mismatches and alerts are reported and make the command
  fail (the files are still written for inspection).

### 13.14 Audio files (1.27.0)

`export wav` / `export flac` (and `piano export --wav|--flac`) render at the
SoundFont's own sample rate (the rate most of its samples use, e.g. 48 kHz for
Salamander), so its samples are not resampled, and write 24-bit samples. Before
1.27.0 WAV was always 44.1 kHz / 16-bit; `--rate 44100 --bits 16` still gives that.
The mix is normalized only if it would clip, then rounded to the bit depth. FLAC is
lossless: it decodes to exactly the samples of the WAV (tested), at about 40 % of
its size.

### 13.15 Importing Humdrum **kern (1.29.0)

`piano import <score.krn> <dir>` reads a Humdrum `**kern` score into the same model
as MusicXML (`kern.rs`); pieces, song, and check are shared (§13.13). The format is
told by the content: MusicXML starts with `<`, kern has a `**kern` spine.

- **Spines**: each `**kern` spine is a staff (`*staffN`, else counted from the
  right); its sub-spines (`*^` / `*v`) are its voices, numbered by position. A
  `**dynam` spine gives the dynamics, hairpins (`<` `>`, ended by `[` `]`), of the
  staves in its `*staff1/2`.
- **Time**: a line starts when the first note of the line before ends; grace notes
  (`q`) and lines of marks only last no time.
- **Tokens**: lengths `4.`, `12` (triplets), `3%2`, `0` / `00`; ties `[ _ ]`; trills
  `t` (half step) / `T` (whole step); `*M`, `*MM` (quarters per minute), `*ped` /
  `*Xped`. Cue-size notes (`*cue`, e.g. a cadenza) are played; invisible notes and
  rests (`y`) are kept. Accents, slurs, stems, beams, and fermatas are ignored, as
  are `sf` / `fp` and tempo words without `*MM`.
- **Sections**: `*>A` labels are played in the order of the expansion list
  (`*>[A,B,B]`); a numbered ending missing from the list (`A2` in `A,A1,A,B`) is
  played after the repeat of its section with that number. Without a list the score
  is played as written. A bar split by a repeat sign is whole again in playing order
  (the section's end and the upbeat after it).
- Writer fixes found with these scores (they apply to MusicXML too): a tied note
  continues in the voice of the note it is tied from, also when only part of a chord
  is tied; a rest that starts a bar inside a multi-bar block gets its bar line.

### 13.16 Importing a folder (1.30.0)

`piano import <folder> <dir>` imports each score directly in the folder (`.mxl`,
`.musicxml`, `.xml`, `.krn`; not subfolders), by file name, into `dir/<file name
without extension>`, which is also the project's name. A folder that is not empty
is skipped, never overwritten. A score that fails or does not match keeps the rest
going; each prints one line (`ok`, `check`, `fail`, `skip`), then a total. The
command fails if any score failed or did not match. For the full report of one
score, import that file alone.

Tried on the 103 movements of Beethoven's sonatas (kern). Fixes found with them:
ties go on through function bodies (language, `expand.rs`: the body's leftover ties
and the ties waiting before it are joined after the function is applied); tied notes
take their lane before notes struck with them; a note tied from a chord into a
differently shaped chord moves to the lane it goes on in; ties to nothing are
dropped; an upbeat takes the piece's signature without a bar line before it, and a
short last bar has none after it. Then (1.31.0) `/128`, ticks for 9 and 11 in a
tuplet, and one key once in a chord (E#4 with F4). Result: all 103 import; 95 match;
8 match every note but warn: a signature change or unmetered bars inside a block
that ties hold together, cadenza bars longer than 32 beats, a few ties across lanes.
In 1.31.1 a grace note in kern no longer ends the time of the note still sounding in
its column (it lasts no time); that had made bars of op. 109/III short. The note check
compares the song with the reader's own model, so a reader error that moves notes is
caught only by the bar-length warnings.

### 13.17 Free bars (1.32.0)

`free{ ... }` writes an unmetered stretch, such as a cadenza, as one bar of any
length. Parser: `free` before `{` is a keyword like `voices` (no arguments, not a
builtin); `|` anywhere inside is an Error. Resolve: the body is inline (state carries
in and out, like a `Group`). Layout (`expand.rs`): after the body the context marks
the current bar `free`; the next bar line skips the length check and clears the mark.
A voice starts with the outer bar's mark, and after a `voices` block the bar is free if
it is in any voice. Inside a function body the mark goes out with the body.

Import: a bar whose length no time signature can write (beats 1-32 over the units
tried) is wrapped in `free{ }` in every voice, between its bar lines (Beethoven
op. 27/1 III, op. 31/1 II, op. 101 III). Bar lines are no longer written at the start
of a voice before anything that takes time (a pedal at the start of a block gave an
empty bar, `{ ped on | ... }`).

### 13.18 Ties on single notes of a chord (1.33.0)

A `~` right after a note inside `{ }` ties that note alone (`{C3~ E4 G}/2 {C3 F4 A}/2`);
a `~` after `}` still ties every note, and both together give a Hint. The parser keeps
a tie flag per chord note; `Sound` has one tie per pitch (`ties`), which layout uses
note by note.

Import: an event keeps which of its notes are tied (`tied`, MIDI numbers) instead of
one flag, so notes struck together with the same length share a chord even when only
some of them go on. The writer puts one `~` after the chord when every note goes on
in the next event, else one after each note that does. Before, such notes went to
separate lanes and a tie chain could change lanes in the middle, where a tie cannot
follow it; the note was struck again (op. 53 I, op. 106 IV, op. 111 II, one or two
notes each). 74 of the 103 Beethoven movements are written with fewer lanes; their
notes are unchanged.

### 13.19 Tempo and time inside `voices` (1.34.0)

`tempo` and `time` written in any voice apply to every voice from that point, and go
on after the block; before, they were ignored with a warning. Tempo events of a voice
already land on the piece's timeline. A time change also matters to the other
voices' bar checks, so a block whose voices change `time` is laid out twice: once to
collect the changes (alerts dropped), then with that schedule, which each voice
follows by its own clock (a change at a bar line takes effect after that bar is
checked) instead of its own `time` items. Changes are joined voice by voice; two
voices that set a different value at the same point give a warning and the first
voice's is used, and only the first voice keeps the joined events. Resolve takes the
value after the block from the first voice that changed it.

Import: in a block that ties hold together, a bar that changes the signature or the
tempo gets `time` / `tempo` after its bar line in the first voice that has one; the
block's head writes the tempo of its first bar, not the block's mean. The last three
Beethoven movements with warnings (op. 27/1 IV, op. 106 IV, op. 109 III) import
cleanly: 103 of 103.

### 13.20 Import and terminal output (1.35.0)

- A folder with no scores of its own is still an error, and it names the folders
  below it (three levels, hidden ones skipped) that hold scores, at most five, each
  with the command to import it: `kern/ has 103 scores: piano import .../kern out`.
  Subfolders are never imported by themselves.
- In a folder import, a `check` line has the title like an `ok` line, then up to three
  of its alerts and missing notes; with more, it says how many and how to list them
  all (import the file alone). The report of one file lists up to ten.
- In the terminal (`import`, `play`, `export`), an alert is placed in its own file:
  `⚠ part1 line 69: ...`, not `⚠ line 1: [part1 line 69] ...`. `Alert` keeps the piece
  and line (`piece`) when a song takes a piece's alert; `Alert::located(file)` writes
  that form, and the editor keeps the song's line (it jumps there).
- 1.35.1: a metronome mark at the end of a title (`Allegro. [quarter] = 138`, from
  kern's `!!!OMD`) is left out of it; the score's tempo already carries it.

### 13.21 Grammar audit (1.35.2 – )

An audit of the language found that bars and ties did not cross block boundaries the
same way: bar lines, tempo, time, and pedals went through `voices`, but bar lines
inside functions and repeats were counted from the block's start, a `time` inside
them was forgotten after them, the last bar could not be short when closed, and ties
stopped at `voices`. Fixed in order:

- 1.35.2: a `time` inside a function or a repeat (the last ending's) goes on after it.
- 1.36.0: the last bar may be short, like the first: a short bar's warning waits until
  something takes time after it (`Ctx::short`, settled before every item and at the
  end; a voice hands its own to the block, a repeat's pass and a function body warn at
  their end). Only bars closed by `|` are checked, as before; this is now written down.
  Import closes a short last bar with `|` and keeps the signature before it instead of
  writing its own (`time 2/8`).
- 1.37.0: bar lines inside a function body continue the bar around it. Layout gives
  every block a child context (`Ctx::child`: bars seen, the part of the bar already
  played, free, and a `scale` for real time, below one in `tuplet`) and takes its bars,
  time, and short bars back after it (`Ctx::rejoin`, with the block's times mapped).
  `seq` lays its body out again for each further pass only to move the bars on (alerts
  of those passes are kept only if new). Voices use the same two calls.
- 1.38.0: ties go into and out of `voices`. Ties waiting before the block join the
  notes that start it, the first voice first (`tie_into`, shared with functions); a
  voice's ties left at its end go on after the block if the note ends with the block
  (one per key; others warn).
