# Changelog

Versioning: +0.1.0 for each feature added or changed, +0.0.1 for each bug fix.
Earlier changes are described in `docs/DESIGN.md` §13.

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
