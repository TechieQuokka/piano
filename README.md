# piano

A piano composition editor for the terminal. Press keys to hear notes, write them
into a score-like language, edit, and play back with a real piano SoundFont.

## Build

```sh
cargo build --release                    # editor, export; no live sound
cargo build --release --features audio   # with live sound (Linux: libasound2-dev)
```

The binary is `target/release/piano`.

## Use

```sh
piano new my_song      # create a project (any folder; existing files are kept)
piano edit my_song     # open the editor
piano play my_song     # play song.piano without the editor
piano export my_song --midi
piano export my_song --flac            # lossless audio, 24-bit at the SoundFont's rate
piano import score.mxl my_song   # a MusicXML score (MuseScore, ...) as a new project
piano import score.krn my_song   # a Humdrum **kern score (kern.humdrum.org, ...)
piano import scores/ my_songs    # every score in a folder → my_songs/<file name>
piano doctor           # check SoundFont, audio device, terminal keys
```

In the editor: `F4` → type `tutorial`. `help` lists commands, `man` is the short
language reference, `guide` explains the language in depth with examples, and
`lang ko` switches the manual, guide, and tutorial to Korean.

A piano `.sf2` SoundFont is needed for sound; set it with `font <path>`.

## How it works

- **Keys** (top): your keyboard is a piano for listening — `Z X C V B N M` /
  `S D G H J` and `Q W E R T Y U` / `2 3 5 6 7`, `←` `→` change the octave.
  Keys only sound; nothing is written.
- **Code** (left): write the piece in the piano language, e.g. `| C4 C G G | A A G/2 |`.
- **Display** (right): shows one view and keeps it until a command changes it
  (`played`, `ref`, `output`, `song`, `help`, `man`, `guide`, `tutorial`; `ls`, `pwd`, `cat`,
  `history`, `project` show their result in `output`).
- **Cmd** (bottom): commands such as `play`, `save`, `new p2`, `open song`.
- While playing, the keys of the sounding notes are pressed on the keyboard, which
  follows the melody across octaves.

| Key | Action |
|---|---|
| `F2` `F3` `F4` | Keys / Code / Cmd (`F12`, `Shift+F12` cycle) |
| `F5` `F6` | play from the cursor / stop |
| `Shift+PgUp` `Shift+PgDn` | scroll the Display from any area |
| `Esc` | cancel a question |

## Terminal

For exact note lengths when you press and release keys, use a terminal with the kitty
keyboard protocol, which reports key releases: **Ghostty**, kitty, WezTerm, foot.
`piano doctor` shows `keyboard enhancement: yes` there.

Other terminals (e.g. GNOME Terminal) only send key repeats, so a release is guessed
from the repeat stream: holding works, but even a short tap rings for about a second.

## Docs

- `docs/DESIGN.md` — design and implementation notes
- `docs/LANGUAGE.md`, `docs/LANGUAGE.ko.md` — language manual (`man`)
- `docs/GUIDE.md`, `docs/GUIDE.ko.md` — detailed language guide (`guide`)
- `docs/TUTORIAL.md`, `docs/TUTORIAL.ko.md` — tutorial
- `CHANGELOG.md` — changes by version

## License

Source-available under the [PolyForm Noncommercial License 1.0.0](LICENSE).

- **Free** for noncommercial use: personal study, learning, research, hobby projects,
  and use by educational or other noncommercial organizations.
- **Commercial use requires a paid license.** Contact: everestfesf@gmail.com

공부·개인·연구 등 비상업적 사용은 무료입니다. 상업적 목적으로 사용하려면 별도의
유료 라이선스가 필요합니다. 문의: everestfesf@gmail.com
