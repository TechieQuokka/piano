# Changelog

Versioning: +0.1.0 for each feature added or changed, +0.0.1 for each bug fix.
Earlier changes are described in `docs/DESIGN.md` §13.

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
