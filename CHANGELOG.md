# Changelog

Versioning: +0.1.0 for each feature added or changed, +0.0.1 for each bug fix.
Earlier changes are described in `docs/DESIGN.md` §13.

## 1.30.0
- `piano import <folder> <dir>` imports every score in a folder (`.mxl`, `.musicxml`,
  `.xml`, `.krn`) as its own project, `dir/<file name>`, named after the file.
  Projects already there are skipped; a score that fails does not stop the rest.
  One line per score, then a total. See `docs/DESIGN.md` §13.16.
- Language: a tie goes on into and out of a function body, e.g.
  `G~ tuplet(3){ G E C }` and `tuplet(3){ C E G~ } G`. Before, it was ignored with a
  warning, and the note was struck again. Notes are matched after the function, so
  `up(2){ C~ } D` holds D. Ties still stop at repeats and `voices`.
- Fix (import): a held note under moving notes of the same voice (a tied note in a
  chord with new notes) could lose its tie and be struck again.
- Fix (import): a note tied from a chord that goes on with a different length now
  leaves the chord, so the other notes are not tied to nothing; a tie with no note
  after it in the score is not written.
- Fix (import): an upbeat is written under the piece's time signature without a bar
  line before it, and a short last bar has no bar line after it, so neither warns.

## 1.29.1
- Fix (import): hairpins were written as a dynamic mark on each note, a whole level
  at a time, and a diminuendo started from the last written mark instead of the top
  of the crescendo before it, so a swell (`p < >`) fell below where it began
  (`p mp p pp p`; a single note dropped to half the velocity). Hairpins are now
  written as `cresc` / `dim` to the level they end on, so the language ramps the
  velocity note by note, and a swell returns to its level.

## 1.29.0
- `piano import` reads Humdrum `**kern` scores (`.krn`, e.g. from kern.humdrum.org)
  as well as MusicXML: staves, split voices, chords, tuplets, ties, grace notes,
  trills, dynamics and hairpins, tempo, and the expansion list of the sections
  (repeats). A numbered ending missing from the list is still played after its
  repeat. Cue-size notes (a cadenza) are played. See `docs/DESIGN.md` §13.15.
- Fix (import): a note tied from a chord where only some notes are tied lost its tie
  (struck again, with a warning). It now continues in the same voice.
- Fix (import): in a block of several bars, a rest at the start of a bar had no bar
  line before it, so the bar check warned.

## 1.28.1
- Fix: `soft on` did not soften the note written right after it (`soft on C4`): at
  the same instant the note was struck before the pedal went down. The soft pedal now
  goes down first, in playback, WAV / FLAC, and MIDI export; damper and sostenuto
  still go down after the notes so they catch them.

## 1.28.0
- New `guide`: a detailed language guide in the Display, to read while composing.
  11 chapters (pitch, rhythm, chords and voices, dynamics, pedals, repeats,
  functions, song, alerts, reference) explain each topic with its musical meaning,
  syntax, exact rules, examples to copy, and common mistakes. `guide` shows the
  contents, `guide 3` a chapter, `guide octave` one topic, `guide next` / `prev` read
  on. English and Korean (`lang ko`); source `docs/GUIDE.md`, `docs/GUIDE.ko.md`.
  Every example is compiled by the tests, including the ones that must raise an
  alert.
- `man <section>` ends with `More guide <section>`.
- Manual: ranges of `tempo`, `time`, `up`, `down`, `accent`, `tuplet`, and `seq`;
  dots on rests in the summary; `accent(1/n)` counts a chord once. Removed a stale
  line about notes written from the keyboard. Korean manual: `accent` counts
  onsets (타건), not beats; example notes in Korean.

## 1.27.1
- Fix: exporting the song wrote `song.mid` / `.wav` / `.flac` in every project. The
  song is now named after the project folder (`la_campanella.flac`); pieces keep
  their own name (`p1.flac`), and a file given to `export` / `-o` is used as is.
- Fix: the keyboard and the Played view always showed Korean solfège. They now
  follow `lang`: `Do Re Mi Fa Sol La Ti` in English, `도 레 미 파 솔 라 시` in Korean.

## 1.27.0
- New FLAC export: `export flac` in the editor, `piano export --flac` on the command
  line. Lossless; the same samples as WAV at about 40 % of the size.
- WAV and FLAC are rendered at the SoundFont's own sample rate (48 kHz for
  Salamander) with 24-bit samples. `piano export --rate --bits` choose others, e.g.
  `--rate 44100 --bits 16` for the previous CD format.

## 1.26.0
- New `piano import <score.mxl|.musicxml|.xml> <dir>`: a MusicXML score becomes a
  project of pieces with voices, pedals, tempo, and dynamics; repeats are unrolled.
  The result is compiled and checked note by note against the score.
- Fix: the two keyboard tests shared a temporary folder and could fail when run in
  parallel.

## 1.25.2
- Fix: `piano doctor` reported (and sound-checked) the first SoundFont found in the
  usual folders, ignoring `piano.toml` and the global config. It now uses the same
  choice as the editor and shows where it comes from.

## 1.25.1
- Fix: `ls` broke its list after every 6 names even when the Display had room; the
  list is now wrapped to the Display width.

## 1.25.0
- The dimmed keys around the playable two octaves show their note names (`C#`, `D`,
  ...) and solfege (`레`); only the PC key labels stay on the playable keys.

## 1.24.0
- New `voices{ ... }{ ... }`: sections that start together and sound at once, each
  with its own octave, length, and dynamic (e.g. a melody over an accompaniment over
  a long bass). Pedals are shared. `man voices`.

## 1.23.0
- `ls`, `pwd`, `cat`, `history`, `project`, `project recent` show their result: they
  switch the Display to Output. Action commands (`play`, `save`, ...) keep the view.

## 1.22.0
- During playback the keyboard's labeled two octaves follow the sounding notes, and
  return to the input octave when playback ends.

## 1.21.0
- During playback the keys of the sounding notes are drawn pressed.

## 1.20.0
- New follow-along tutorial: build "Twinkle Twinkle Little Star" line by line in
  Code (pieces `p1`, `p2`, `end`, `song`); a test replays the whole tutorial.
- `Shift+PgUp` / `Shift+PgDn` (or `Alt`) scroll the Display from any focus.
- Paragraph breaks in tutorial steps are kept.

## 1.19.0
- The 88-key map line was removed; the Keys area is the zoomed keyboard only (one
  line of text below 24 rows).

## 1.18.1
- Fix: holding a key in a kitty-protocol terminal (e.g. Ghostty) re-struck the note
  with every auto-repeat.
- License: PolyForm Noncommercial 1.0.0; commercial use needs a paid license.

## 1.18.0
- Display rules: it only shows (scrolling is its only control), it never changes with
  focus or `Esc`, and it keeps its view until a command opens another.
- `view <name>` replaced by `played`, `ref`, `output`, `song`.

## 1.17.1
- Fix: the Keys/Played test and the tutorial still assumed that Keys write notes;
  the tutorial, `DESIGN.md` and README were brought in line with listen-only Keys.
