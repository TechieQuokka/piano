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
piano doctor           # check SoundFont, audio device, terminal keys
```

In the editor: `F4` → type `tutorial`. `help` lists commands, `man` explains the
language, `lang ko` switches the manual and tutorial to Korean.

A piano `.sf2` SoundFont is needed for sound; set it with `font <path>`.

## Docs

- `docs/DESIGN.md` — design and implementation notes
- `docs/LANGUAGE.md`, `docs/LANGUAGE.ko.md` — language manual (`man`)
- `docs/TUTORIAL.md`, `docs/TUTORIAL.ko.md` — tutorial

## License

Source-available under the [PolyForm Noncommercial License 1.0.0](LICENSE).

- **Free** for noncommercial use: personal study, learning, research, hobby projects,
  and use by educational or other noncommercial organizations.
- **Commercial use requires a paid license.** Contact: everestfesf@gmail.com

공부·개인·연구 등 비상업적 사용은 무료입니다. 상업적 목적으로 사용하려면 별도의
유료 라이선스가 필요합니다. 문의: everestfesf@gmail.com
