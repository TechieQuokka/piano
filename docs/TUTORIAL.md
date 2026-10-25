# piano Tutorial

Format: `## TITLE {#id}` starts a step. Indented lines are shown in the app, except
`Expect`: piano code whose notes must appear in the current piece for the step to be
marked done. Fields the reader follows, also replayed by the tests from a new project:
`Piece <name>` (the piece the code goes into), `Type <code>` (the code on a new line at
the end), `Line <n> <code>` (line n becomes the code). `docs/TUTORIAL.ko.md` must have
the same IDs, `Expect`, `Piece`, `Type` and `Line` lines.

## WELCOME {#screen}

    In this tutorial you write a whole song, line by line:
    "Twinkle Twinkle Little Star". You type every line into the
    Code area yourself; a step that asks you to write something
    checks it for you (Check ✔).

    The screen has four areas:
      Keys (top)       the piano keyboard, always visible
      Code (left)      your piece, with line numbers
      Display (right)  this tutorial, results, help, manual ...
      Cmd (bottom)     commands such as play, save, help
    F12 moves between them (back: Shift+F12), or jump directly:
    F2 Keys, F3 Code, F4 Cmd. The status bar shows where you are.

    The Display stays as it is while you move. Only a command
    changes it: played, ref, output, song, help, man, tutorial.
    Shift+PgUp / Shift+PgDn scroll it from any area.

    Moving through the tutorial (in Cmd, F4):
      tutorial next    next step
      tutorial prev    previous step
      tutorial 5       jump to step 5
    Try      Press F12 a few times, then F4 and type tutorial next.

## LISTEN {#listen}

    First, find the melody by ear. Press F2: your keyboard becomes
    a piano. In Keys, notes only sound; nothing is written.

      lower row   Z X C V B N M  =  C D E F G A B  (도 레 미 파 솔 라 시)
      black keys  S D   G H J    =  C# D#   F# G# A#
      upper row   Q W E R T Y U  =  the same, one octave higher
      ← →         move the whole keyboard by an octave

    Twinkle Twinkle begins with two phrases. Play them:
      Z Z B B N N B      C C G G A A G      도 도 솔 솔 라 라 솔
      V V C C X X Z      F F E E D D C      파 파 미 미 레 레 도
    Hold a key to let it ring, tap it for a short note.
    Try      Press F2 and play both phrases.
    Hint     Cmd `played` lists what you played; `tutorial` comes back.

## FIRST LINE {#first}

    Now write the first phrase. Press F3 (Code). The piece p1 has
    one line, `// p1`: // starts a comment, and the rest of that
    line is ignored. So press End, then Enter, and on line 2 type:

    Type     | C4 C G G | A A G/2 |

    What each part means:
      C4      the note C (도) in octave 4, the middle of the piano
      C G G   no number: the octave nearest to the note before
      /2      a half note (2 beats). A note without / is a quarter.
      |       a bar line. Optional, but lines are easier to read,
              and a bar of the wrong length gets a ⚠ in the gutter.
    Spaces do not matter, and c g a works as well as C G A.
    Expect   C4 C G G A A G/2
    Hint     man octave, man length, man bar.

## SECOND LINE {#second}

    Press Enter and write the second phrase on line 3:

    Type     | F/4 F E E | D D C/2 |

    Why /4 again? A length is inherited: each note keeps the length
    of the note before it. Line 2 ended with G/2, so without /4 this
    F would be a half note too.
    The octave is inherited the same way: F lands next to the G
    before it (F4), and the last C next to the D before it (C4).
    Expect   F4/4 F E E D D C/2
    Hint     man inheritance. Cmd `ref` shows the state at the cursor.

## PLAY {#play}

    Listen to what you wrote. Press F4 and type a command:
      play           the whole piece
      play 3         only line 3
      loop 2-3       lines 2 to 3, again and again
      stop           stop (or F6)
    F5 plays from the Code cursor: press F3, move the cursor up to
    line 2 (↑, Home) and press F5.
    Try      F4, play, Enter. Then F3, ↑, Home, F5.
    Hint     No sound? Cmd `output` shows why (font loads a SoundFont).

## TEMPO {#tempo}

    tempo sets the speed in quarter notes per minute (default 100).
    Code may share a line with a comment. Go to line 1 (F3, ↑) and
    change it to:

    Line 1   tempo 90 // p1

    time 3/4 would set three beats per bar (default 4/4).
    Try      Play it again: a little slower.
    Hint     man tempo, man time.

## DYNAMICS {#dynamics}

    Start soft and grow louder. Change lines 2 and 3 (move with
    ↑ ↓ Home End, delete with Backspace):

    Line 2   | p C4 C G G | A A G/2 |
    Line 3   | cresc F/4 F E E | D D C/2 mf |

    Dynamic marks, from very soft to very loud:
      ppp pp p mp mf f ff fff     (p = soft, mf = medium, f = loud)
    A mark lasts until the next one. cresc grows note by note until
    the next mark (here mf); dim does the opposite.
    Try      Play it and listen to line 3 growing.
    Hint     man dynamic, man cresc. Cmd `undo` takes back an edit.

## NEW PIECE AND REPEAT {#repeat}

    The middle of the song is a new part, so it gets its own piece
    (file). In Cmd (F4) type: new p2
    Code now shows p2, an empty piece. Press F3 and type on line 1:

    Piece    p2
    Type     |: G4 G F F | E E D/2 :|

    |: ... :| plays the section twice; :| x3 plays it three times.
    Why G4 and not G? A new piece starts fresh: octave reference C4,
    length /4, mf. The G nearest to C4 is G3, below it, so write G4.
    Expect   |: G4 G F F | E E D/2 :|
    Hint     man repeat, man volta. Cmd `song` lists your pieces.

## ENDING {#ending}

    End with a pause and a big chord, in a third piece.
    In Cmd: new end. Then F3 and type:

    Piece    end
    Type     r/2 ped on {C4 E G C}/1 ped off

      r/2      a half rest (silence for 2 beats)
      { }      notes struck together: a chord; its length goes after }.
               In a chord each note is nearest to the one before it,
               so the last C is C5.
      /1       a whole note (4 beats)
      ped on   the damper (right) pedal: notes keep ringing until
               ped off. Also sos on|off (middle), soft on|off (left).
    Expect   {C4 E G C5}/1
    Hint     man chord, man rest, man pedal.

## SONG {#song}

    Now put the parts in order. In Cmd: open song. The song file is
    a list of pieces joined by >. Change line 1 to:

    Piece    song
    Line 1   p1 > p2 > p1 > end

    Play the whole song: F4, play song.
    Between pieces you can also write fade (fade out), r/2 (a pause)
    or ped (keep the pedal down across the join): p1 > fade > p2.
    Try      F4, play song.
    Hint     man arrangement, man connector.

## FUNCTIONS {#functions}

    Functions change a section without rewriting it. Play the last
    A part an octave higher: change line 1 of song to

    Piece    song
    Line 1   p1 > p2 > up(12){ p1 } > end

    up(12) raises by 12 semitones (one octave). Others:
      down(n)  louder(n)  softer(n)  accent(1/3)  tuplet(3)  seq(3, 2)
    They work on notes inside a piece too: up(12){ C4 E G }.
    Try      F4, play song.
    Hint     Cmd `ref` lists every function. man fn.

## SAVE AND EXPORT {#save}

    save writes the piece shown in Code. Unsaved work is also kept
    in a recovery copy (recover). Save every piece:
      open p1, save      open p2, save
      open end, save     open song, save
    Then write files to share: export midi, export wav (needs a
    SoundFont).
    Try      F4: save each piece, then export midi.
    Hint     help export.

## MORE {#more}

    You wrote a whole song. More to try in a scratch piece (new try),
    one line at a time:
      C C# D Eb E               sharps (#) and flats (b)
      C/4. D/8 E/2              . a dotted note (one and a half)
      G/2~ G/4                  ~ a tie: one long note
      tuplet(3){ C/8 D E }      three notes in the time of two
      |: C D [1 E :| [2 G |     different endings
      | C D E F | % |           % repeats the previous bar
      time 3/4 | C D E |        three beats per bar
    The manual explains every rule:
      man            chapters          man octave     one section
      lang ko        Korean            tutorial 1     start again
    Try      F4, new try, then type the lines above.
