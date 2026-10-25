# piano Tutorial

Format: `## TITLE {#id}` starts a step. Indented lines are shown in the app, except
`Expect`: piano code whose notes must appear in the current piece for the step to be
marked done. `docs/TUTORIAL.ko.md` must have the same IDs and `Expect` lines.

## SCREEN {#screen}

    The screen has four areas:
      Keys (top)       the piano keyboard, always visible
      Code (left)      your piece, with line numbers
      Display (right)  results, help, manual, tutorial ...
      Cmd (bottom)     commands such as play, save, help
    Move between areas with F12 (back: Shift+F12),
    or jump with F2 (Keys), F3 (Code), F4 (Cmd).
    The Display stays as it is while you move; only a command
    changes it (played, ref, output, song, help, man, tutorial).
    Try      Press F12 a few times, then F2.
    Hint     The status bar at the bottom shows the focus.

## KEYS {#keys}

    In Keys focus, your keyboard is a piano for listening.
    Z X C V B N M are white keys, S D G H J black keys.
    Keys only sound; nothing is written to the code.
    Try      Press Z X C.
    Hint     The keys light up. Cmd `played` lists C4 도  D4 레  E4 미 (back: `tutorial`).

## OCTAVE SHIFT {#shift}

    The upper row Q W E R T Y U is one octave higher.
    ← → move the whole range by an octave.
    The 88-key map and the dimmed keys around the range show where you are.
    Try      Press Q, then → and Z.
    Hint     After →, Z plays C5 (`played` shows C5 C5).

## CODE {#code}

    To keep what you heard, write it in Code focus (F3).
    A note is a name (C D E F G A B), an optional octave (C4),
    and an optional length (/8). Only changes are written:
    /8 appears once, the next notes inherit it.
    Try      Press F3 and type C4/8 D E F G/4.
    Expect   C4/8 D E F G/4
    Hint     D E F inherit /8. man length explains lengths.

## REST AND CHORD {#chord}

    r is a rest. Notes in { } sound together as a chord.
    Try      Type C4 r {C E G}.
    Expect   C4 r {C E G}
    Hint     man rest, man chord.

## PLAY {#play}

    F5 plays from the cursor, F6 stops.
    In Cmd (F4): play, play 2-4 (lines), loop 1-2, stop.
    Try      Press F4, type play, press Enter.
    Hint     No sound? Cmd `output` shows why: a SoundFont may be missing (font).

## NEAREST OCTAVE {#nearest}

    Without a number, a note takes the octave closest to the
    previous note. So C D E F G A B C climbs to the next C.
    Try      Type C4 D E F G A B C.
    Expect   C4 D E F G A B C5
    Hint     Cmd `ref` shows the octave reference at the cursor.

## DYNAMICS {#dynamics}

    p (soft) ... f (loud) change the volume from that point.
    cresc grows until the next mark.
    Try      Type p cresc C4 D E F f G.
    Expect   C4 D E F G
    Hint     man dynamic, man cresc.

## PEDALS {#pedals}

    ped on ... ped off keeps every note ringing.
    sos on keeps only the notes already sounding.
    soft on makes the sound softer.
    Try      Type ped on C3/2 E G ped off.
    Expect   C3/2 E G
    Hint     Cmd `ped on` inserts the same text at the cursor.

## REPEATS {#repeats}

    |: ... :| plays a section twice; x3 three times.
    | C D E F | % | repeats the previous bar.
    Try      Type |: C4 E G :|.
    Expect   C4 E G C4 E G
    Hint     man repeat, man volta.

## FUNCTIONS {#functions}

    Builtin functions change a section: up, down, accent,
    louder, softer, tuplet, seq. They can be nested.
    Try      Type up(12){ C4 E G }.
    Expect   C5 E G
    Hint     Cmd `ref` lists all functions with details.

## SONG {#song}

    A project has pieces (p1.piano, p2.piano ...) and song.piano,
    which lists them in order: p1 > p2 > p1.
    Try      In Cmd: new p2, then open song and write p1 > p2.
    Hint     play song plays the arrangement. man connector.

## SAVE AND EXPORT {#save}

    save writes the piece. export midi / export wav write files.
    Unsaved work is kept in a recovery copy (recover).
    Try      In Cmd: save, then export midi.
    Hint     help export for options.

## NEXT {#next}

    You know the basics. The manual explains every rule:
      man            chapters
      man octave     one section
      lang ko        Korean descriptions
    Try      Type man in Cmd.
    Hint     tutorial 1 starts again.
