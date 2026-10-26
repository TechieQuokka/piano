# piano Language Guide

The detailed guide to the piano language, shown in the app with `guide` (the short
reference is `man`, from `docs/LANGUAGE.md`).

Format: `## N. Title {#id}` starts a chapter, `### Title {#id}` a section, `#### Name`
a part of a section. Every section of the manual has a section with the same ID here.
Code blocks are tested: ` ```piano ` must compile without errors or warnings,
` ```piano warning ` (or `error`, `hint`) must raise that alert, ` ```song ` is a song
file, ` ```text ` and ` ```toml ` are only shown. `docs/GUIDE.ko.md` must have the
same IDs and the same piano and song code (comments may differ).

## 1. Getting started {#start}

This guide explains the piano language step by step: what each part means in music,
how to write it, the exact rules, examples you can copy into Code, and the mistakes
that are easy to make. Read it in order with `guide next`, or jump to one topic with
`guide <section>`, e.g. `guide octave`. The short reference for the same topics is
`man`.

### Your first piece {#first}

#### Concept

A piece is a text file. Every word in it is a note or an instruction, and the editor
plays the words in order, like a program that runs from top to bottom. Whatever you
do not write is filled in from defaults, so a few letters are already music.

#### Try it

Press F3 to go to Code and type:

```piano
C D E F G
```

Press F5 to play from the cursor, or type `play` in Cmd (F4) to play from the start.
You hear five notes going up from middle C, each one beat long, at a medium volume.

#### What was filled in

The line above is short for the line below. Each value is explained in its own
section; for now it is enough to know that they exist and that you can change them.

```piano
tempo 100 time 4/4 mf C4/4 D E F G   // the same piece, written out
```

- `tempo 100`: 100 beats per minute (`guide tempo`).
- `time 4/4`: four beats per bar (`guide time`).
- `mf`: medium loud (`guide dynamic`).
- `C4`: the C in the middle of the keyboard (`guide octave`).
- `/4`: each note is one beat, a quarter note (`guide length`).

#### See also

`guide file` · `guide continuity` · `guide grammar`

### Pieces and files {#file}

#### Concept

A project is a folder. It holds `piano.toml` (settings), one file per piece
(`p1.piano`, `verse.piano`, ...) and `song.piano`, which puts the pieces in order.
A piece is one part of the music, such as an intro or a chorus. Its name is the file
name without `.piano`; `new <name>` makes a new one.

#### Rules

- The code is read left to right, top to bottom, like a score.
- Line breaks and spaces only separate words. A new line is not a new bar and does not
  pause the music.
- Write one or two bars per line. Line numbers are how you play part of a piece
  (`play 3`, `play 2-4`, `loop 2-4`) and how alerts point at a problem.
- Mistakes never stop the piece. A part that cannot be understood becomes an alert
  (`guide levels`); everything else still plays.

#### Examples

```piano
// p1: two bars per line
| C4 D E F | G A B C |
| C B A G  | F E D C |
```

The `$` below is not part of the language. It is reported as an error and skipped;
C, D, and E still play.

```piano error
C D $ E   // Error: unknown symbol, the notes still play
```

#### Common mistakes

- Writing a whole piece on one line. It plays, but `play 3` has nothing to pick and
  every alert points at line 1.
- Expecting a line break to end a bar. Only `|` marks a bar (`guide bar`).

#### See also

`guide comment` · `guide bar` · `guide levels`

### Comments {#comment}

#### Concept

A comment is a note to yourself that the editor ignores, like `//` in C or Rust. Use
it to name parts of a piece, or to silence a line for a moment.

#### Syntax

`//` starts a comment; it runs to the end of the line. There are no block comments.

#### Examples

```piano
// Verse 1: melody only
C4 D E F   // going up
// G A B C   (commented out: not played)
```

#### Common mistakes

- `#` is not a comment. It is the sharp sign and only works right after a note name.

```piano error
# intro   // Error: # is a sharp, not a comment
C D E
```

- A single `/` is not a comment either: it starts a length, as in `C/8`.

#### See also

`guide file` · `guide accidental`

### Upper and lower case {#case}

#### Rules

- Note names and keywords ignore case: `c` is `C`, `TEMPO 90` is `tempo 90`,
  `PED ON` is `ped on`, `R` is a rest.
- One exception: a lone lowercase `f` is the dynamic mark forte (loud). For the note F
  write `F`, `f4`, or `f/4`.
- The flat sign is always a lowercase `b`: `Eb`, `eb`. `EB` is an unknown word.
- Words need spaces between them: `CDE` is one unknown word; write `C D E`. How many
  spaces does not matter.
- A length, a dot, or a tie sticks to its note without a space: `C4/8.~`.

#### Examples

```piano
c d e F f g   // c d e F are notes, f is forte, g is a note
```

```piano
C D E f G   // four notes: f only makes G loud
```

```piano error
CDE   // Error: one unknown word, write C D E
```

#### See also

`guide name` · `guide dynamic`

## 2. Pitch {#pitch}

Pitch is how high or low a note sounds. This chapter explains the keyboard, the names
of the notes, sharps and flats, and the octave rule that decides which C you mean when
you write `C`.

### Keys and semitones {#semitone}

#### Concept

A piano has 88 keys: low on the left, high on the right. The keys repeat a pattern of
twelve: seven white keys and five black keys. Going from one key to the very next one
(white or black) is a semitone, the smallest step. Twelve semitones make an octave:
the note there has the same name and sounds like the same note, only higher.

If you think in code: a pitch is an integer (C4 = 60, as in MIDI), a semitone is +1
and an octave is +12. Functions such as `up(n)` simply add to that integer.

#### The pattern

```text
C  C#  D  D#  E  F  F#  G  G#  A  A#  B  C
0  1   2  3   4  5  6   7  8   9  10  11 12   semitones from C
```

- The black keys are the sharps: C#, D#, F#, G#, A#.
- E–F and B–C have no black key between them, so they are one semitone apart. The other
  neighboring white keys are two semitones (a whole tone) apart.

#### Try it

In Keys (F2) your keyboard is a piano: `Z S X D C V G B H N J M` play
C C# D D# E F F# G G# A A# B. Play them in that order and listen to the even steps.

#### See also

`guide name` · `guide accidental` · `guide octave`

### Note names {#name}

#### Concept

The seven white keys are named with letters: C D E F G A B, the same notes as
do re mi fa sol la si. After B the names start again at C, one octave higher.

#### Syntax

A note is its letter, optionally followed by a sharp or flat, an octave number, a
length, and a tie, in that order: `C`, `F#4`, `Bb3/8.`, `G/2~`.

#### Examples

```piano
C D E F G A B   // do re mi fa sol la si
```

```piano
E4 D C D E E E/2   // Mary Had a Little Lamb
```

#### Common mistakes

- Solfège syllables and the German H are not note names.

```piano error
do re mi   // Error: write C D E
```

#### See also

`guide accidental` · `guide octave` · `guide case`

### Sharps and flats {#accidental}

#### Concept

A sharp `#` raises a note by one semitone, a flat `b` lowers it by one. Usually that is
a black key: C# is the black key right of C. C# and Db are the same key, written two
ways; use whichever reads better to you.

Sheet music has a key signature that adds sharps or flats to every F, C, ... of the
piece. The piano language has no key signature and no accidentals that last to the end
of the bar: every note says exactly what it is. In a piece in G major, every F that
should be F# is written `F#`.

#### Rules

- `#` and `b` come right after the letter: `F#`, `Bb`, `F#4`, `Bb3/8`.
- The first letter is the note name and a `b` right after it is a flat: `bb` is one
  B flat, `b b` is two B notes.
- E# is F, Fb is E, B# is C, Cb is B: the sharp or flat moves one semitone, whatever
  key that lands on.
- There is no natural sign: a plain letter is natural. There are no double sharps or
  flats.

#### Examples

```piano
C C# D Eb E   // Eb and D# are the same key
```

```piano
G A B C D E F# G   // G major: every F is written F#
```

```piano
bb Bb b   // B flat, B flat, B
```

```piano error
C## D   // Error: no double sharps, write D
```

#### Common mistakes

- Expecting a key signature: writing `F` in a G major melody plays F, not F#.
- An uppercase flat: `EB` is an unknown word, write `Eb`.

#### See also

`guide semitone` · `guide name`

### Octaves {#octave}

#### Concept

The same note name exists in every octave, so a number says which one: `C4` is middle
C, near the center of the keyboard. The number goes up by one at every C: B3 is the key
just below C4. The lowest key is A0, the highest C8.

#### Rules

- With a number, the note is exactly that key: `G4`, `Bb2`.
- Without a number, the note is the one nearest to the previous note, the reference.
  At the start of a piece the reference is C4.
- So the largest step you can write without a number is six semitones. For a bigger
  leap, write the octave.
- Exactly six semitones away (C to F#, F to B, B to F) is a tie: the higher one is
  chosen, with a warning. Write the number to make it clear.
- Every note, with or without a number, becomes the reference for the next one.
- Rests do not change the reference.
- In a chord each note is taken from the note before it in the chord; after the chord
  the reference is its first note (`guide chord`).
- A note outside A0–C8 is moved by octaves into the range, with a warning.
- The `ref` command shows the reference at the cursor.

#### Examples

```piano
C4 D E F G A B C   // the last C is C5: one step up from B4
```

```piano
C4 E G C5 G4 E C   // up and down an arpeggio
```

Going from C4 to G, the G below (G3) is five semitones away and the G above (G4) is
seven, so `G` alone goes down:

```piano
C4 G    // G3: the nearer G
C4 G4   // write the number to go up
```

```piano
C4 C G4 G A A G/2   // Twinkle: G4 once, the rest follows it
```

```piano warning
C4 F#   // Warning: F#3 and F#4 are both 6 away, F#4 is chosen
```

```piano warning
C8 D   // Warning: D8 does not exist, folded to D7
```

#### Common mistakes

- Writing Twinkle as `C4 C G G A A G`: G goes down to G3 and the tune turns upside
  down. One number fixes it.
- Jumping a long way without a number. When in doubt, write the number; it never hurts.

#### See also

`guide continuity` · `guide chord` · `guide fn.up`

## 3. Rhythm {#rhythm}

Rhythm is when notes start and how long they last. This chapter covers note lengths,
dots, rests, ties, tempo, time signatures, bar lines, free bars, and tuplets, and ends
with how values carry from note to note.

### Beats and lengths {#length}

#### Concept

Music counts time in beats. A quarter note is one beat; the other lengths are halves
and doubles of it. The number after `/` is the fraction of a whole note: `/8` is an
eighth of a whole note. A bigger number is a shorter note.

- `/1` whole note: 4 beats
- `/2` half note: 2 beats
- `/4` quarter note: 1 beat
- `/8` eighth note: 1/2 beat
- `/16` sixteenth note: 1/4 beat
- `/32`, `/64`, and `/128`: 1/8, 1/16, and 1/32 beat

#### Rules

- The length comes right after the note (and its octave): `C/8`, `C4/8`, `{C E G}/2`,
  `r/8`.
- Without a length, a note keeps the length of the note or rest before it. The first
  note of a piece is a quarter.
- Only 1, 2, 4, 8, 16, 32, and 64 exist. Anything else is an error and the length stays
  as it was. Thirds of a beat are written with `tuplet` (`guide fn.tuplet`).

#### Examples

```piano
C4/4 D E F   // four quarters: one bar of 4/4
G/2 E/2      // two halves
C/1          // a whole note
```

```piano
C4/8 D E F G/4 E C/2   // eighths, then quarters, then a half
```

```piano error
C/3   // Error: no thirds, use tuplet(3)
```

#### Common mistakes

- Forgetting that the length carries on:

```piano
C4/1 D E F   // all four are whole notes
```

#### See also

`guide dot` · `guide rest` · `guide fn.tuplet` · `guide continuity`

### Dotted notes {#dot}

#### Concept

A dot makes a note half as long again: a dotted quarter is 1 + 1/2 = 1.5 beats. A
second dot adds another quarter of the length: a double-dotted quarter is 1.75 beats.
A dotted quarter followed by an eighth (long, short) fills two beats; it is one of the
most common rhythms in music.

#### Rules

- Dots come right after the length: `C/4.`, `C4/8..`. A dot without a length is an
  error.
- At most two dots; more give a warning and two are used.
- Dots carry on with the length: after `C/4.` the next notes are dotted quarters until
  a new length is written.

#### Examples

```piano
C4/4. D/8 E/4. F/8   // long-short, long-short
```

```piano
C/2. D/4   // 3 + 1 beats
```

```piano
C/4. D E F/8 G   // D and E are dotted too; F and G are eighths
```

```piano error
C. D   // Error: a dot needs a length, write C/4.
```

```piano warning
C/4... D   // Warning: at most 2 dots
```

#### See also

`guide length` · `guide tie`

### Rests {#rest}

#### Concept

A rest is silence for a length of time. Music breathes through its rests.

#### Syntax

`r`, `r/<n>`, `r/<n>.` — like a note without a pitch.

#### Rules

- A rest takes a length and passes it on, just like a note.
- A rest does not change the octave reference.
- A rest cannot be tied.

#### Examples

```piano
C4 D E r   // three notes and one beat of silence
```

```piano
C4/8 r D r E/4 r   // the rests take /8, then /4
```

```piano
B4 r C   // C5: the rest does not move the reference
```

```piano warning
C r/2~ r   // Warning: rests cannot be tied
```

#### Common mistakes

- A rest's length also carries on to the next note:

```piano
C4/4 r/8 D   // D is an eighth too
```

#### See also

`guide length` · `guide octave`

### Ties {#tie}

#### Concept

A tie joins two notes of the same pitch into one sound: their lengths add up and the
second note is not struck again. Use it for lengths that no single symbol can write
(five beats = whole + quarter) and to hold a note across a bar line.

#### Syntax

`<note>~ <same note>`: `G/2~ G/4`.

#### Rules

- The note after `~` must be the same key. Its octave is found as usual, so `G/2~ G`
  works without a number.
- A tie may cross a bar line.
- A chord can be tied to a chord: each note of the first is joined with the same note
  in the second.
- A `~` right after a note inside a chord ties that note alone: `{C4~ E G}/2 C` holds C
  and lets E and G end. A `~` after `}` ties every note.
- A different note, a rest, or the end after `~` gives a warning and the tie is ignored.
- A tie goes on into and out of a function, e.g. `G~ tuplet(3){ G E C }`; its
  notes are matched after the function, so `up(2){ C~ } D` holds D.
- A tie does not reach into or out of a repeat or `voices`.
- There is no slur (legato between different notes); the damper pedal is the way to
  connect notes (`guide pedal`).

#### Examples

```piano
C4/1~ C/4 D E F   // C lasts five beats
```

```piano
| C4 D E F~ | F/2 G/2 |   // F is held across the bar line
```

```piano
{C4 E G}/2~ {C E G}/2   // the whole chord is held for four beats
```

```piano
{C4~ E G}/2 {C F A}/2   // C is held for four beats; F and A are struck over it
```

```piano warning
C4 D/2~ E   // Warning: D and E differ, the tie is ignored
```

#### See also

`guide length` · `guide chord` · `guide bar`

### Tempo {#tempo}

#### Concept

Tempo is the speed: how many quarter-note beats per minute (BPM). At 60 one beat lasts
a second, at 120 half a second. Slow music is around 60–76, walking pace 90–110, fast
120–160.

#### Syntax

`tempo <bpm>`, from 20 to 400.

#### Rules

- The tempo applies from where it is written; write it at the top for the whole piece.
  Without it, the piece uses the default (100, see `guide defaults`).
- The beat is always a quarter note, whatever the time signature.
- There is no gradual change of tempo; slow down or speed up in steps.
- Inside `voices`, tempo is ignored with a warning: write it before the block.
- In a song every piece starts at its own tempo.

#### Examples

```piano
tempo 72 C4 E G C5/2   // slow
```

```piano
tempo 120 C4/8 D E F G A B C   // eight eighths in two seconds
```

```piano
tempo 100 C4 D E F
tempo 90 G A B C
tempo 76 C/1   // slowing down at the end, step by step
```

```piano error
tempo 500 C   // Error: tempo must be 20-400
```

#### See also

`guide time` · `guide defaults`

### Time signatures {#time}

#### Concept

A time signature groups beats into bars. The top number is how many, the bottom one
which note is counted: 4/4 is four quarters per bar, 3/4 three (a waltz), 2/4 two (a
march), 6/8 six eighths felt as two groups of three.

#### Syntax

`time <beats>/<unit>`: beats 1–32, unit 1, 2, 4, 8, 16, 32, or 64.

#### Rules

- It applies from where it is written. The default is 4/4.
- It is used for two things only: checking bar lengths (`guide bar`) and `%`
  (`guide bar-repeat`). It does not change the sound and does not accent any beat by
  itself; for that, use `accent` (`guide fn.accent`).
- Inside `voices`, time is ignored with a warning.
- A bar that holds a `free` block is not checked against it (`guide free`).

#### Examples

```piano
time 3/4 | C4 E G | C5/2. |   // a waltz
```

```piano
time 6/8 | C4/8 D E F G A | G/4. E/4. |
```

```piano
| C4 D E F | time 3/4 G A B |   // 4/4, then 3/4
```

```piano error
time 3/5 C   // Error: the unit must be 1 2 4 8 16 32 64 128
```

#### See also

`guide bar` · `guide tempo`

### Bar lines {#bar}

#### Concept

A bar (a measure) is one group of beats, and `|` marks where it ends. In piano bar
lines are optional: they do not change the sound. They work like an assertion in code:
if a bar does not add up to the time signature, you get a warning.

#### Rules

- Every bar is compared with the time signature; a bar that is shorter or longer gives
  a warning.
- The first bar may be shorter: a pickup (upbeat) before the first full bar. It may not
  be longer.
- A `|` at the very start of a piece is fine, but then the next bar counts as a full
  bar, not a pickup.
- `%` needs bar lines to know what the previous bar was.
- A line break is not a bar line.
- A bar that holds a `free` block is not checked (`guide free`).

#### Examples

```piano
| C4 D E F | G A B C |
```

```piano
G4 | C5 C C D | E/2 C/2 |   // pickup: one beat before the first bar
```

```piano warning
| C4 D E | F G A B |   // Warning: bar has 3 beats, 4/4 expects 4
```

#### Common mistakes

- Starting a pickup with `|`: `| G4 | C5 ...` turns the pickup into a short full bar
  and gives a warning. Leave the first `|` out.

#### See also

`guide time` · `guide bar-repeat` · `guide file` · `guide free`

### Free bars {#free}

#### Concept

Some music has a stretch with no beat to count: a cadenza, a run of small notes the
player takes freely. `free{ }` writes such a stretch as one bar of any length. It turns
the bar-length assertion off for that one bar.

#### Syntax

`free{ <notes> }`: `free{ C/16 D E F G A B C }`.

#### Rules

- The bar that holds a `free` block is not checked against the time signature, whatever
  its length. The bar after it is checked again.
- A `free` block is one bar: `|` inside it is an error. Put the bar lines around it.
- Anything else can be inside: notes, chords, rests, dynamics, pedals, functions,
  `voices`.
- It plays the written lengths at the current tempo; the sound is not changed.
- Octave, length, and dynamic carry into and out of the block as usual, and a tie goes
  on into it and out of it.
- Inside `voices`, each voice checks its own bars, so a `free` in one voice frees only
  that voice's bar. To free a bar for every voice, put the voices inside:
  `free{ voices{ ... }{ ... } }`.

#### Examples

```piano
time 3/4 | C4 E G | free{ G/16 A B C5 D E F G A B C6 B A G F E D C } | C5/2. |
// the run is 4 1/2 beats, the bar around it 3/4
```

```piano
| free{ voices{ G5/32 A G A G A G A G A G A G A G A }{ {C3 E G}/2 } } |
// a trill over a held chord, two beats in 4/4
```

```piano error
free{ C4 D | E F }   // Error: no bar lines inside `free`
```

#### Common mistakes

- Using `free` to hide a counting mistake. In a passage with a beat, a bar of the wrong
  length is usually a typo, and the warning is there to find it.
- Bar lines inside a long cadenza. Leave them out, or split it into several blocks:
  `| free{ ... } | free{ ... } |`.

#### See also

`guide bar` · `guide time` · `guide voices`

### Tuplets {#fn.tuplet}

#### Concept

A triplet plays three notes in the time of two. Lengths in piano are halves and doubles
only, so triplets are written with plain lengths inside `tuplet(3){ ... }`, which
squeezes them: three eighths take the time of two eighths, one beat.

#### Syntax

`tuplet(<n>){ ... }`, n from 3 to 64.

#### Rules

- The body is played in the time of the nearest lower power of two: 3 in 2, 5 in 4,
  6 in 4, 7 in 4, 9 in 8, 11 in 8.
- Times are exact for 3, 5, 6, 7, 9, 10, 11 and numbers made of these (12, 14,
  15, ...); others, such as 13, are rounded to the nearest tick.
- Everything in the body is squeezed: notes, rests, chords.
- After the block the length carries on as written inside, not squeezed: write the
  next length again.

#### Examples

```piano
tuplet(3){ C4/8 D E } F/4 G A   // a triplet beat, then three quarters
```

```piano
| tuplet(3){ C4/4 E G } C5/2 |   // three quarters in two beats
```

```piano
tuplet(5){ C4/16 D E F G } A/4   // five sixteenths in one beat
```

#### Common mistakes

- Forgetting the length after the block:

```piano
tuplet(3){ C4/8 D E } F G   // F and G are eighths: write F/4
```

#### See also

`guide length` · `guide fn.syntax`

### What carries over {#continuity}

#### Concept

piano remembers. Like a program with variables, it keeps a state that every note reads
and changes; a value stays until you write a new one. This is why `C4/8 D E` makes D
and E eighths near C4 without saying so.

#### The state

- Octave reference: the previous note (start: C4).
- Length, with dots: the previous length (start: `/4`).
- Dynamic: the last mark (start: `mf`).
- Pedals: on or off until changed (start: all off).
- Tempo and time signature: the last one written (start: 100 and 4/4).

The starting values can be changed per project (`guide defaults`). The `ref` command
shows the state at the Code cursor.

#### Where the flow branches

- Chords: after a chord the reference is its first note.
- Repeats: the text is read once, so every pass sounds the same.
- Functions: the body is read as written, before the transform; the notes after it go
  on from the last written note, not from the transformed one.
- `voices`: every voice starts from the state before the block; afterwards the first
  voice's state goes on.
- Songs: every piece starts fresh from the defaults.

#### Examples

```piano
C4/8 D E f F G   // D E: eighths near C4; F G: eighths, forte
```

```piano
up(12){ C4 D E } F   // F is F4: it follows E4 as written
```

#### Common mistakes

- A long length or a loud mark that keeps going after you meant it to stop. When notes
  sound wrong, put the cursor on them and run `ref`.

#### See also

`guide octave` · `guide length` · `guide dynamic` · `guide defaults`

## 4. Chords and voices {#harmony}

Piano music is rarely one note at a time. A chord strikes several keys together; voices
let several lines move at once, such as a melody over a bass.

### Chords {#chord}

#### Concept

A chord is a set of keys struck at the same time. The most common chord is a triad:
three notes stacked in thirds, such as C E G (C major). In piano a chord is a single
event: all its notes start together, last the same length, and have the same volume.

#### Syntax

`{<notes>}/<n>`: `{C E G}/2`, `{C4 E G}/2~`, `{C4~ E G}/2`.

#### Rules

- The length is written once, after `}`. A length inside gives a warning and is
  ignored.
- Inside the braces each note is taken from the note before it, so `{C E G}` stacks
  upward. After the chord the reference is its first note.
- Only notes may be inside: no rests, marks, or lengths. A note inside may have its own
  `~`, which ties that note alone (`guide tie`).
- A chord takes and passes on the length like a note.
- The same key written twice is played once.

#### Examples

```piano
{C4 E G}/2 {F A C}/2 {G B D}/2 {C E G}/2
```

```piano
{C3 G3 C4 E}/1   // a wide chord: write numbers for big leaps
```

```piano
C4/8 E G C5 {C4 E G C5}/2   // broken, then struck together
```

```piano warning
{C4/8 E G}   // Warning: write the length after }: {C4 E G}/8
```

```piano error
{C4 r G}/2   // Error: only notes in a chord
```

#### Common mistakes

- Expecting the notes of a chord to have different lengths, for example a long bass
  under moving notes. That is what `voices` is for, or a `~` on the held note:
  `{C3~ E4 G}/2 {C3 F4 A}/2`.

#### See also

`guide voices` · `guide tie` · `guide octave`

### Voices {#voices}

#### Concept

Often one part holds a long note while another moves, or the left hand plays a bass
line under the right hand's melody. A chord cannot do this, because its notes share one
length. `voices` runs several sequences at the same time, like threads that start
together and are joined at the end.

#### Syntax

`voices{ ... }{ ... } ...`: two or more bodies; each `{ }` is one voice.

#### Rules

- All voices start at the same moment.
- Each voice starts from the state before the block and keeps its own octave, length,
  dynamic, and cresc.
- After the block the first voice's state goes on.
- Pedals belong to the whole piano: a pedal in any voice acts on all of them and stays
  as set after the block.
- Bar lines inside a voice continue the current bar.
- Voices of different lengths give a warning; the block lasts as long as the longest.
- `tempo` and `time` inside a voice are ignored with a warning.
- A `{ }` right after the block is another voice, unless a length or `~` follows it:
  then it is a chord.
- Anything can be inside a voice: chords, functions, repeats.

#### Examples

```piano
voices{ E4 D C D E E E/2 }{ C3/1 G2/1 }
```

```piano
voices{ | C5 B4 A G | F/2 E/2 | }{ | C3/1 | F3/2 G/2 | }
```

```piano
C4/8 voices{ G4 E G E G E G E }{ p C3/1 } D   // D: an eighth after E4, mf
```

```piano
voices{ E4/2 }{ C4/2 } {C4 E G}/2   // the last { } has a length: a chord
```

```piano warning
voices{ C4 D E F }{ C3/2 }   // Warning: 4 beats against 2
```

#### See also

`guide hands` · `guide chord` · `guide pedal`

### Writing for two hands {#hands}

#### Concept

On the piano the right hand usually plays the melody above middle C and the left hand
the bass and the harmony below it. With `voices` each hand is one voice (or more).

#### Patterns

- Melody over held chords: the left hand holds one chord per bar.
- Melody over a broken chord: the left hand plays the chord notes one by one, in
  eighths.
- Bass and chord: a low note on the beat, a chord after it.
- Melody in octaves: the same line twice, one octave apart (`guide fn.up`).

#### Tips

- Write one `voices` block per bar or two, on its own line. `play 3` then plays that
  part with both hands, and an alert points at the right place.
- Give every voice of a block the same length; bar lines inside help to check.
- Write the octave at the start of each voice: every voice starts from the state
  before the block, not from where the other voice ended.

#### Examples

```piano
// Twinkle with two hands, one bar per line
voices{ C4 C G4 G }{ C3/2 E/2 }
voices{ A4 A G/2 }{ F3/2 E/2 }
voices{ F4/4 F E E }{ D3/2 C/2 }
voices{ D4 D C/2 }{ G2/2 C3/2 }
```

```piano
voices{ E5/2 D/2 }{ C3/8 E G E C E G E }   // a broken-chord left hand
```

```piano
voices{ G4/1 }{ C3 {E3 G}/4 G2 {E3 G}/4 }   // bass and chord
```

#### See also

`guide voices` · `guide fn.louder` · `guide pedal`

## 5. Dynamics and expression {#expression}

Dynamics are how loud the music is. This chapter covers the marks from ppp to fff,
gradual changes, accents, and changing the volume of a whole section.

### Dynamic marks {#dynamic}

#### Concept

The marks come from Italian: p (piano) is soft, f (forte) is loud, m (mezzo) is
medium. They set how hard the keys are struck, the velocity (1–127). A harder strike
is not only louder; a piano also sounds brighter.

- `ppp` very very soft: 16
- `pp` very soft: 33
- `p` soft: 49
- `mp` medium soft: 64
- `mf` medium loud: 80 (the default)
- `f` loud: 96
- `ff` very loud: 112
- `fff` very very loud: 127

#### Rules

- A mark applies to every note after it until the next mark.
- Marks ignore case except `f`: a lone lowercase `f` is forte, `F` is a note.
- Inside `voices` each voice keeps its own mark.

#### Examples

```piano
p C4 D E F f G A B C   // soft, then loud
```

```piano
f C4 D E/2 p C D E/2   // a phrase and its echo
```

```piano
mf C4 E G pp C5/2   // a sudden quiet top note
```

#### Common mistakes

- Forgetting that a mark lasts: after `ff` everything stays very loud until the next
  mark.

#### See also

`guide cresc` · `guide fn.louder` · `guide case`

### Crescendo and diminuendo {#cresc}

#### Concept

`cresc` (crescendo) grows louder little by little, `dim` (diminuendo) grows softer.
Both lead to the next dynamic mark.

#### Syntax

`cresc <notes> <mark>`, `dim <notes> <mark>`.

#### Rules

- The change starts at the current mark and moves in equal steps per note (a chord
  counts once), reaching the target mark at the note after it. `p cresc C D E F f G`
  plays 49, 60, 72, 84, then G at 96.
- Without a mark after it, nothing changes and you get a warning.
- `cresc` toward a softer mark (or `dim` toward a louder one) gives a warning; the notes
  still move to the target.
- A cresc outside a `voices` block does not count the notes inside it; each voice can
  have its own.

#### Examples

```piano
p cresc C4 D E F G A B f C5   // eight steps from p to f
```

```piano
ff dim C5 B A G F E D pp C   // fading down the scale
```

```piano
mp cresc | C4 D E F |
G A B C | ff D/1 |   // a crescendo over two lines
```

```piano warning
cresc C4 D E   // Warning: no target mark
```

```piano warning
f cresc C4 D E p F   // Warning: cresc toward a softer mark
```

#### See also

`guide dynamic` · `guide fn.accent`

### Accents {#fn.accent}

#### Concept

An accent makes single notes stand out: the first beat of each bar, or every note of
one name in a motif.

#### Syntax

`accent(1/<n>){ ... }` or `accent(<note>){ ... }`. Matching notes get +20 velocity
(at most 127).

#### Rules

- `1/n`: every n-th onset, starting with the first. An onset is a moment where notes
  start; a chord counts once, rests do not count. n is 1–64.
- `<note>`: every note with that name, in any octave. Sharps and flats count by sound:
  `accent(F#)` is `accent(Gb)`.

#### Examples

```piano
accent(1/2){ C4/8 D E F G A B C }   // the start of every beat
```

```piano
time 3/4 accent(1/3){ C4 E G C5 G4 E }   // waltz: beat one of each bar
```

```piano
accent(G){ G4 E G D G C }   // every G stands out
```

#### See also

`guide fn.louder` · `guide dynamic`

### Louder and softer {#fn.louder}

#### Concept

`louder` and `softer` move the volume of a whole section without rewriting its marks.
The most common use is balance: the melody a bit louder than the accompaniment.

#### Syntax

`louder(<n>){ ... }`, `softer(<n>){ ... }`: add or subtract n from every velocity
(kept within 1–127).

#### Rules

- Marks inside the body still work; the change is applied on top of them.
- Any whole number is allowed.

#### Examples

```piano
louder(20){ C4 D E F }   // mf (80) becomes 100
```

```piano
voices{ louder(15){ E5 D C D } }{ softer(15){ C3/8 E G E C E G E } }
```

#### See also

`guide dynamic` · `guide hands`

## 6. Pedals {#pedals}

A piano has three pedals. In piano each one is switched on and off with a word, and
the pedals may overlap freely.

### Damper pedal {#pedal}

#### Concept

The right pedal (sustain) lifts every damper: notes keep ringing after their length
ends and blend into each other. It makes music smooth and full; changing it when the
harmony changes keeps the sound from getting muddy.

#### Syntax

`ped on`, `ped off`. The other two pedals are `sos on|off` (`guide sostenuto`) and
`soft on|off` (`guide soft-pedal`).

#### Rules

- A pedal is a switch, not a block: it stays as set, across lines, until switched
  again.
- It acts at the moment where it is written. A pedal written right before notes catches
  them; written right after them, it comes too late for them.
- `ped off ped on` right before a new chord is a clean pedal change: the old sound stops
  and the new chord is caught.
- Switching on a pedal that is already on gives a hint.
- Pedals belong to the whole piano: in `voices` a pedal in one voice acts on all.
- In a song, a pedal left on at the end of a piece is released there.

#### Examples

```piano
ped on C4/8 E G C5 ped off   // the broken chord rings together
```

```piano
ped on {C3 G3}/2 {E4 G C5}/2 ped off ped on {F3 C4}/2 {F4 A C5}/2 ped off
```

```piano hint
ped on C4 ped on D   // Hint: already on
```

#### Common mistakes

- Forgetting `ped off`: every note after it rings into the next and the music blurs.

#### See also

`guide sostenuto` · `guide soft-pedal` · `guide connector`

### Sostenuto pedal {#sostenuto}

#### Concept

The middle pedal keeps ringing only the notes that are sounding when it is pressed.
Notes played after that stop normally. A low bass note can ring on while the melody
above it stays clear.

#### Syntax

`sos on`, `sos off`.

#### Rules

- Write `sos on` right before the notes it should hold: they are caught at the moment
  they start.
- The held notes ring until `sos off`.

#### Examples

```piano
sos on C2/8 E4 F G A B C5 D/2 sos off   // C2 rings under a clear melody
```

```piano
ped on C2/8 E4 F G A B C5 D/2 ped off   // compare: everything rings
```

#### See also

`guide pedal` · `guide soft-pedal`

### Soft pedal {#soft-pedal}

#### Concept

The left pedal (una corda) makes the sound softer and more veiled. In piano it lowers
the velocity of the notes played while it is on to 80 %.

#### Syntax

`soft on`, `soft off`.

#### Examples

```piano
soft on p C4 E G C5/2 soft off   // very quiet and veiled
```

#### See also

`guide pedal` · `guide dynamic`

## 7. Repeats {#repeats}

Music repeats itself a lot. Repeat signs, different endings, `%`, and `seq` save
writing the same notes twice.

### Repeat signs {#repeat}

#### Concept

`|: ... :|` plays the part between the signs twice, like a loop. `x <n>` after `:|`
sets how many times.

#### Syntax

`|: ... :|`, `|: ... :| x3` (also `x 3`).

#### Rules

- The count is 1–99; anything else gives a warning and 2 is used.
- Every pass sounds the same: the text is read once, so octaves do not drift.
- After the repeat the music goes on from the state at the end of the text.
- `:|` without `|:` repeats from the start, or from the end of the last repeat, with a
  hint.
- Repeats may be nested.
- After a repeat, `%` has no bar to copy.

#### Examples

```piano
|: C4 D E C :|   // played twice
```

```piano
|: C4/8 E G E :| x4   // four times
```

```piano
|: |: C4 D :| E F :|   // (C D C D E F) twice
```

```piano hint
C4 D E F :|   // Hint: no |:, repeats from the start
```

```piano error
|: C4 D E   // Error: |: without :|
```

#### See also

`guide volta` · `guide bar-repeat` · `guide fn.seq`

### Different endings {#volta}

#### Concept

Often a repeated part ends differently the second time: first and second endings
(volta brackets). The shared part is written once.

#### Syntax

`|: <shared> [1 <ending 1> :| [2 <ending 2> |`

#### Rules

- There are as many passes as endings; pass k plays the shared part, then ending k.
- Endings are numbered in order, starting at `[1` (otherwise a warning).
- Every ending but the last ends with `:|`. The last one ends at the next `|`, the next
  `|:`, or the end of the piece.
- Every ending starts from the state after the shared part.

#### Examples

```piano
|: C4 D E F [1 G/1 :| [2 C/1 |   // C D E F G, then C D E F C
```

```piano
|: E4 D C D [1 E E E/2 :| [2 D D C/2 |
```

```piano
|: C4 E G/2 [1 C5/1 :| [2 E/1 :| [3 C4/1 |   // three endings
```

```piano error
C4 D [1 E   // Error: [1 only inside |: ... :|
```

#### See also

`guide repeat`

### Repeat the previous bar {#bar-repeat}

#### Concept

`%` means "the same as the last bar", like the simile sign in sheet music. It saves
writing a repeated accompaniment bar again and again.

#### Rules

- `%` copies the notes and pedals of the bar just before it: between the last two `|`.
- So it needs a finished bar before it; otherwise a warning.
- `%` can follow `%`: the copy is copied again.

#### Examples

```piano
| C3/8 E G E C E G E | % | % |   // one bar, played three times
```

```piano
| C4 E G E | % | F A C5 A4 | % |
```

```piano warning
C4 D % E   // Warning: no finished bar to repeat
```

#### See also

`guide bar` · `guide repeat`

### Sequences {#fn.seq}

#### Concept

A sequence repeats a short motif, each time a bit higher or lower. `seq` writes it in
one go.

#### Syntax

`seq(<times>, <step>){ ... }`: times 1–64, step in semitones (negative goes down).

#### Rules

- Pass k is moved by k × step; the first pass is unchanged.
- Steps are semitones, not scale steps, so a sequence keeps the exact shape of the
  motif.
- Notes moved outside A0–C8 are folded, with a warning.

#### Examples

```piano
seq(3, 2){ C4/8 D E C }   // C D E C, D E F# D, E F# G# E
```

```piano
seq(4, -1){ G4 F E/2 }   // sliding down by semitones
```

```piano
seq(2, 12){ C4 E G E }   // the motif again, one octave higher
```

#### See also

`guide repeat` · `guide fn.up`

## 8. Functions {#functions}

Functions change a whole section at once: move it up or down, louder or softer,
squeeze it, or repeat it shifted.

### How functions work {#fn.syntax}

#### Concept

A function is a transform applied to a section: `name(args){ body }`. If you think in
code, it is like a `map` over the notes of the body. Only the built-in functions exist:
`up`, `down`, `accent`, `louder`, `softer`, `tuplet`, `seq`. There are no user-defined
functions and no recursion.

#### Rules

- The body is ordinary code: chords, repeats, `voices`, pedals, other functions.
- Nested functions are applied from the inside out. They may be nested 32 deep.
- The body is read as written before it is transformed, so the notes after a function
  go on from the last written note.
- Results outside A0–C8 are folded by octaves, with a warning.
- An unknown name or a wrong argument is an error; the body still plays, unchanged.
- A tie goes on into and out of a function: `G~ tuplet(3){ G E C }` holds G.

#### Arguments

- `up(n)`, `down(n)`: semitones, 0–88.
- `accent(1/n)`: n 1–64; or `accent(<note>)`.
- `louder(n)`, `softer(n)`: any whole number.
- `tuplet(n)`: 3–64.
- `seq(times, step)`: times 1–64, any step.

#### Examples

```piano
up(12){ accent(1/3){ C4/8 D E F G A } }   // accent first, then an octave up
```

```piano
louder(10){ |: C4 E G :| }   // a repeat inside a function
```

```piano error
up(100){ C4 }   // Error: at most 88, C4 still plays
```

```piano error
echo(2){ C4 D }   // Error: no such function
```

#### See also

`guide fn.up` · `guide fn.accent` · `guide fn.tuplet` · `guide fn.seq`

### Up {#fn.up}

#### Concept

`up(n)` raises every note of the section by n semitones. Use it to play a phrase again
higher, to move a section to another key, or to double a melody an octave higher.

#### Common steps

- 1: a semitone. 2: a whole tone.
- 3: a minor third. 4: a major third.
- 5: a fourth. 7: a fifth.
- 12: an octave.

#### Examples

```piano
C4 E G up(12){ C4 E G }   // the same, one octave higher
```

```piano
up(2){ C4 D E F G/2 }   // in D: D E F# G A
```

```piano
voices{ C4 D E F }{ up(12){ C4 D E F } }   // the melody in octaves
```

```piano warning
up(12){ C8 }   // Warning: above C8, folded back
```

#### See also

`guide fn.down` · `guide fn.seq` · `guide transform`

### Down {#fn.down}

#### Concept

`down(n)` lowers every note of the section by n semitones; it is the mirror of `up`.

#### Examples

```piano
down(12){ C4 E G }   // one octave lower
```

```piano
voices{ C4/2 E/2 }{ down(12){ C4/2 E/2 } }   // a bass doubling the melody
```

```piano
G4 A B down(7){ G4 A B }   // the same motif a fifth lower
```

#### See also

`guide fn.up` · `guide fn.syntax`

## 9. Song {#song}

A song puts the pieces of a project in order. It is written in `song.piano`, played
with `play song`, shown with `song`, and exported like a piece.

### Arrangement {#arrangement}

#### Concept

Think of the pieces as functions and the song as `main()`: each part is written once
and called as often as needed.

#### Syntax

`<piece> > <piece> > ...`: `p1 > bridge1 > p4`.

#### Rules

- A name is a piece file without `.piano`. A song cannot contain itself.
- Line breaks are free and `//` comments work, as in pieces.
- Every piece is independent: it starts from the project defaults with its own tempo,
  dynamic, and octave reference, and a pedal left on is released at its end.
- The same piece may appear any number of times.
- An unknown piece is an error; the rest still plays. Alerts from inside a piece show
  as `[p2 line 3] ...` on the song line.
- The song file is `song.piano`, or the `entry` set in `piano.toml`.

#### Examples

```song
p1 > p2 > p1
```

```song
// intro, two verses, chorus, verse, ending
intro > verse > verse > chorus
verse > outro
```

#### See also

`guide connector` · `guide transform` · `guide file`

### Connectors {#connector}

#### Concept

Between pieces you can say how they join: directly, with a fade, with a pause, or
with the pedal held over the join.

#### Syntax

- `>`: go on directly.
- `fade`: the end of the previous piece fades out.
- `r/<n>`: a rest between pieces.
- `ped`: the damper pedal is held over the join.

#### Rules

- `fade` lowers the notes of the last four beats of the piece before it, more and more
  toward 30 % of their velocity at the very end.
- `r/<n>` takes lengths and dots like a rest in a piece; plain `r` is one beat.
- `ped` presses the damper one beat before the end of the previous piece and releases
  it one beat into the next one, so the join rings.
- `fade` and `ped` need a piece right before them (a rest in between gives a warning).
- A bridge, a short piece written to connect two others, is just an ordinary piece.

#### Examples

```song
p1 > fade > r/2 > p2
```

```song
p1 > ped > p2 > ped > p3
```

```song warning
fade > p1   // Warning: fade needs a piece before it
```

#### See also

`guide arrangement` · `guide pedal`

### Transforming pieces {#transform}

#### Concept

The built-in functions also work on pieces in a song: the last chorus a whole tone
higher, a repeat softer. The piece files are not changed.

#### Syntax

`<function>(<args>){ <pieces> }`.

#### Examples

```song
chorus > up(2){ chorus }   // the last chorus a tone higher
```

```song
p1 > softer(20){ p1 } > louder(10){ p2 > p3 }
```

```song
seq(3, 2){ p1 }   // p1 three times, a tone higher each time
```

```song error
up(100){ p1 }   // Error: up takes 0-88
```

#### See also

`guide fn.syntax` · `guide arrangement`

## 10. Alerts {#alerts}

Alerts are how the editor tells you about problems. They never stop the music: what
can be played is played. This chapter lists them with their causes and fixes.

### Alert levels {#levels}

#### Concept

- ✖ Error: that part cannot be played and is skipped (an unknown word, a wrong length).
- ⚠ Warning: it plays, but maybe not as you meant (a folded octave, an ambiguous leap,
  a bar of the wrong length, an ignored tie).
- · Hint: information (`:|` without `|:`, a pedal that is already on).

#### Where they show

- The Output view lists them with line numbers (`output`).
- The Code gutter marks each line with its most severe alert.
- They update while you type.

#### Examples

```piano warning
C4 F#   // Warning: line 1: C4 → F# is ambiguous (6 semitones)
```

#### See also

`guide errors` · `guide warnings` · `guide hints`

### Errors {#errors}

#### Notes and words

- unknown word: a typo, solfège (`do`), a missing space (`CDE`), the German `H`, an
  uppercase flat (`EB`). Check the spelling.
- unknown symbol: a character the language does not use, such as `$`, a lone `#`, or a
  dot without a length (`C.`).
- unexpected number: a space between a note and its octave (`C 4`). Write `C4`.
- length without a note: `/8` with no note before it.
- invalid length: `/3`, `/6`, ... Only 1 2 4 8 16 32 64 128; use `tuplet` for triplets.

#### Headers and pedals

- tempo must be 20-400, `tempo` needs a number: write `tempo 90`.
- invalid time signature, `time` needs a signature: write `time 3/4`.
- `ped` needs `on` or `off` (also `sos`, `soft`).

#### Brackets

- only notes are allowed in a chord; chord `{` has no matching `}`; unmatched `}`.
- no bar lines inside `free`: a `free` block is one bar; `free{` has no matching `}`.
- `|:` has no matching `:|`; `[2` outside a repeat; `x` needs a number.

#### Functions

- unknown function: only `up down accent louder softer tuplet seq` exist.
- invalid arguments: see the allowed ranges in `guide fn.syntax`. The message shows the
  usage.
- needs a body `{ ... }`; `(` has no matching `)`; `{` has no matching `}`.

#### Songs

- piece not found: there is no such `.piano` file in the project.
- a song cannot contain itself; unexpected ... in song; missing `}` in song.
- `>` is only used in song files: it appeared in a piece.

#### Examples

```piano error
C 4 D   // Error: unexpected number, write C4
```

#### See also

`guide levels` · `guide warnings`

### Warnings {#warnings}

#### Pitch

- ambiguous (6 semitones): the leap is a tritone either way; write the octave number.
- folded to ...: the note was outside A0–C8 and was moved by octaves.

#### Length and rhythm

- at most 2 dots, using 2.
- bar has N beats, 4/4 expects 4: a bar does not match the time signature
  (`guide bar`).
- tie is not followed by the same note, ignored; `~` must follow a note or chord;
  rests cannot be tied.
- `%` has no previous bar to repeat.

#### Chords and voices

- put the chord length after `}`; empty chord.
- voices have different lengths (the longest one is used).
- `tempo` and `time` inside `voices` are ignored, write them before it.

#### Repeats and dynamics

- repeat count must be 1-99, using 2; repeat count without a repeat; first ending is
  not `[1`.
- cresc/dim has no target dynamic; cresc ends at a mark that is not louder (dim: not
  softer).
- functions or blocks nested deeper than 32.

#### Songs

- `fade` / `ped` needs a piece before it.

#### Examples

```piano warning
| C4 D E | F G A B |   // Warning: bar has 3 beats, 4/4 expects 4
```

#### See also

`guide levels` · `guide hints`

### Hints {#hints}

#### List

- `:|` without `|:` repeats from the start: fine if that is what you meant.
- `ped on` while already on (or off while off): the pedal did not change.
- `voices` has one voice: add another `{ ... }`.
- `~` after the chord ties every note: the `~` inside it are not needed.

#### Examples

```piano hint
voices{ C4 D E F }   // Hint: only one voice
```

#### See also

`guide levels`

## 11. Reference {#reference}

The whole language on one page, the default values and how to change them, and the
music terms used in this guide.

### Grammar at a glance {#grammar}

#### Pieces

```text
note      C D E F G A B  [# b]  [0-8]  [/n [. ..]]  [~]
rest      r [/n [. ..]]
chord     { note [~] ... } [/n [. ..]] [~]
comment   // ...
header    tempo <20-400>   time <beats>/<unit>
bar       |   %
dynamic   ppp pp p mp mf f ff fff   cresc   dim
pedal     ped|sos|soft on|off
repeat    |: ... :| [x n]   |: ... [1 ... :| [2 ... |
voices    voices{ ... }{ ... }
free      free{ ... }
function  up down accent louder softer tuplet seq   name(args){ ... }
```

#### Songs

```text
song      piece > piece   fade   r/n   ped   name(args){ pieces }
```

#### A piece using most of it

```piano
tempo 90 time 3/4
| p G4/8 A B C5 D E | ped on {C4 E G}/2. ped off |
|: voices{ E5/4 D C }{ C3/2. } :| x2
cresc | up(12){ C4 D E } | f G5/2. |
```

#### See also

`guide first` · `man grammar`

### Defaults and settings {#defaults}

#### Builtin defaults

tempo 100, time 4/4, length `/4`, dynamic `mf`, octave reference C4.

#### Changing them

Set them for a project in `piano.toml`, or for all your projects in
`~/.config/piano/config.toml`, under `[defaults]`. Every field is optional.

```toml
[defaults]
tempo = 90
time = "3/4"
length = "1/8"
dynamic = "mp"
octave = 3        # the starting reference is C3
```

#### Order

What is written in the piece wins, then `piano.toml`, then the global config, then the
builtin values. Every piece of a song starts from these defaults.

#### Examples

```piano
tempo 120 C D E   // the piece's own tempo wins over the default
```

#### See also

`guide continuity` · `guide tempo`

### Music terms {#glossary}

#### Terms

- accent: one note played louder than its neighbors.
- arpeggio, broken chord: the notes of a chord played one after another.
- bar, measure: a group of beats, ended by `|`.
- beat: the unit of musical time; in piano a quarter note.
- cadenza: a free passage, often a run of small notes, played without a strict beat;
  written with `free`.
- chord: several keys struck together.
- crescendo / diminuendo: gradually louder / softer.
- dynamics: loudness, from ppp to fff.
- octave: twelve semitones; the same note name, higher or lower.
- pickup (anacrusis): notes before the first full bar.
- semitone: the step from one key to the very next.
- sequence: a motif repeated at a shifted pitch.
- tempo: speed, in beats per minute.
- tie: two notes of the same pitch joined into one.
- time signature: how many beats of which length make a bar.
- transpose: move notes up or down by a fixed number of semitones.
- triplet, tuplet: three (or n) notes in the time of two (or fewer).
- velocity: how hard a key is struck, 1–127.
- voice: one line of music; several voices sound at once.
- volta: a first or second ending of a repeat.

#### See also

`guide grammar`
