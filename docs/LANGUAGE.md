# piano Language Manual

Format: `## TITLE {#chapter}` starts a chapter, `### TITLE {#section}` a section.
Indented lines are shown in the app (`man <section>`). Every `Example` is compiled by
the test suite. `docs/LANGUAGE.ko.md` must have the same IDs.

## BASICS {#basics}

### FILE {#file}

    Syntax   <piece>.piano                  p1.piano  song.piano
    Flow     Read left to right, top to bottom, like a score.
    Lines    Line breaks are free. Write 1-2 bars per line:
             playback ranges and alerts use line numbers.
    Errors   Never stop the piece. Unknown parts become alerts;
             the rest still plays.
    Example  C4 D E F
    See      comment, case

### COMMENT {#comment}

    Syntax   // text                        C D // intro
    Rule     Everything after // to the end of the line is ignored.
    Note     # is a sharp, not a comment.
    Example  C D // intro
    See      file

### CASE {#case}

    Syntax   Note names and keywords ignore case.     c d e  =  C D E
    Except   A lone lowercase f is the dynamic forte.
             Write F, f4, or f/4 for the note F.
    Space    Spaces separate tokens. Count does not matter.
    Example  c d e F f g
    See      name, dynamic

## NOTES {#notes}

### NAME {#name}

    Syntax   C D E F G A B                  C  D  E
    Meaning  International note names (C = 도, D = 레, ... B = 시).
    Example  C D E F G A B
    See      accidental, octave

### ACCIDENTAL {#accidental}

    Syntax   <name>#  <name>b               C#  Bb
    Meaning  # raises a semitone (black key above), b lowers one.
    Same     C# and Db are the same key.
    Parse    First letter = note name; a b right after it = flat.
             bb = B flat (one note). b b = two B notes.
    Written  Notes entered from the keyboard use #.
    Example  C C# D Eb E
    See      name

### OCTAVE {#octave}

    Syntax   <note><0-8>                    C4  A0  C8
    Range    A0 - C8 (88 keys). C4 = middle C. Number changes at C.
    Omitted  Nearest to the previous note. Start reference: C4.
             Tie (6 semitones) -> up, Warning.
    Resets   Writing a number sets a new reference.
    Rests    Do not change the reference.
    Chords   First note becomes the reference.
    Folding  Outside A0-C8 -> moved by octaves into range, Warning.
    Example  C4 D E F G A B C   → last C = C5
    See      inheritance, fn.up, fn.down

### LENGTH {#length}

    Syntax   <note>/<n>   n = 1 2 4 8 16 32 64      C/8  G/2
    Meaning  /1 whole, /2 half, /4 quarter, /8 eighth, ...
             Bigger n = shorter note.
    Omitted  Inherited from the previous note or rest. Start: /4.
    Invalid  Other numbers (e.g. /3) -> Error, length unchanged.
    Example  C4/8 D E F G/4   → D E F are eighths
    See      dot, tuplet, inheritance

### DOT {#dot}

    Syntax   /<n>.   /<n>..                 C/4.  C/4..
    Meaning  . adds half the length, .. adds half + quarter.
             /4. = 1.5 beats, /4.. = 1.75 beats.
    Limit    Only right after /n. At most 2 dots (more -> Warning).
    Inherit  Dots are inherited with the length.
    Example  C/4. D E F/8 G   → D E are dotted quarters
    See      length

### TIE {#tie}

    Syntax   <note>~ <same note>            G/2~ G/4
    Meaning  Joins two notes of the same pitch into one long note.
             The second note is not struck again.
    Bars     May cross bar lines.
    Chords   {C E G}/2~ {C E G}/4 ties all three.
    Other    Different pitch after ~ -> Warning, tie ignored.
    Example  C4 D G/2~ | G/4 F
    See      length, chord

### REST {#rest}

    Syntax   r   r/<n>                      r  r/8  r/2.
    Meaning  Silence for the given length.
    Inherit  Uses and updates the inherited length.
             Does not change the octave reference.
    Example  C4 r D r/8 E
    See      length, octave

### CHORD {#chord}

    Syntax   {<notes>}/<n>                  {C E G}/2
    Meaning  Notes struck together.
    Length   Written once, after }. A length inside {} -> Warning.
    Octave   Each note resolves against the previous one in the chord.
             After the chord, the reference is its first note.
    Example  {C4 E G}/2 {F A C}/2
    See      octave, tie, voices

## INHERITANCE {#inheritance}

### CONTINUITY {#continuity}

    Rule     Values carry over to following notes until changed.
    Octave   nearest to the previous note (start: C4)
    Length   previous length, with dots (start: /4)
    Dynamic  previous dynamic mark (start: mf)
    Pedals   stay on/off until toggled (start: off)
    View     Cmd `ref` shows the state at the cursor.
    Example  C4/8 D E f F G   → D E are C4-near eighths; F G are forte
    See      octave, length, dynamic, pedal

### DEFAULTS {#defaults}

    Values   tempo 100, time 4/4, length /4, dynamic mf, reference C4
    Change   piano.toml [defaults], or ~/.config/piano/config.toml.
    Order    piece header > piano.toml > global config > builtin
    Example  tempo 120 C D E
    See      tempo, time

## STRUCTURE {#structure}

### BAR {#bar}

    Syntax   |                              | C D E F |
    Meaning  Bar line. Optional.
    Check    A bar whose length differs from the time signature -> Warning.
             A short first bar (pickup) is fine.
    Example  | C4 D E F | G A B C |
    See      time, bar-repeat

### TIME {#time}

    Syntax   time <beats>/<unit>            time 3/4  time 6/8
    Meaning  Time signature, used for the bar check and %.
    Where    Anywhere; applies from that point. Default 4/4.
    Example  time 3/4 | C4 D E | F G A |
    See      bar

### TEMPO {#tempo}

    Syntax   tempo <bpm>                    tempo 90
    Meaning  Quarter notes per minute (20-400).
    Where    Anywhere; applies from that point. Default 100.
    Example  tempo 90 C D tempo 140 E F
    See      defaults

### VOICES {#voices}

    Syntax   voices{ ... }{ ... } ...       voices{ G4/2 A }{ C3/1 }
    Meaning  Each { } is a voice. All voices start together and sound
             at once: a melody over chords, a long bass under it.
    State    Each voice starts from the state before the block and
             keeps its own octave, length, dynamic, and cresc.
    After    The first voice continues after the block.
    Pedals   Shared by the whole piano: a pedal in any voice acts on
             every voice, and stays as set after the block.
    Bars     Bar lines inside a voice continue the current bar.
    Length   Voices of different lengths -> Warning; the block lasts
             as long as the longest voice.
    Header   tempo and time inside a voice -> Warning, ignored.
    Chord    A { } right after the block is another voice, unless a
             length follows it: voices{ C }{ E } {C E G}/2
    Example  voices{ p r/2 G4/8. G4/16 G4/4 }{ pp {C3 C4}/1 }
    See      chord, pedal, bar

## DYNAMICS {#dynamics}

### DYNAMIC {#dynamic}

    Syntax   ppp pp p mp mf f ff fff        p C D f E
    Velocity ppp 16, pp 33, p 49, mp 64, mf 80, f 96, ff 112, fff 127
    Inherit  Applies until the next mark. Default mf.
    Example  p C4 D f E F
    See      cresc, case

### CRESC {#cresc}

    Syntax   cresc ... <mark>   dim ... <mark>        p cresc C D E F f
    Meaning  Gradually louder (cresc) or softer (dim), note by note,
             reaching the next dynamic mark.
    Target   No mark after it -> Warning.
             cresc toward a softer mark (or dim toward louder) -> Warning.
    Example  p cresc C4 D E F f G
    See      dynamic

## PEDALS {#pedals}

### PEDAL {#pedal}

    Syntax   ped on|off   sos on|off   soft on|off
    ped      Damper (right): every note keeps ringing.
    sos      Sostenuto (middle): only notes sounding when it is
             pressed keep ringing; later notes stop normally.
    soft     Soft / una corda (left): softer sound.
    Overlap  Toggles, not blocks: pedals may overlap freely.
    Same     Turning on a pedal that is on -> Hint.
    Order    A pedal written right before notes catches them:
             sos on C2/1 holds C2.
    Example  sos on C2/1 sos off ped on C4 E G ped off
    See      continuity

## REPEATS {#repeats}

### REPEAT {#repeat}

    Syntax   |: ... :|   |: ... :| x<n>     |: C D :| x3
    Meaning  Plays the section n times (default 2). Notes sound the same
             every pass; the octave is resolved once on the text.
    Start    :| without |: repeats from the start (or after the last repeat).
    Count    1-99. Others -> Warning, 2 is used.
    Example  |: C4 E G :| x3
    See      volta, bar-repeat

### VOLTA {#volta}

    Syntax   |: ... [1 ... :| [2 ... |      |: C D [1 E :| [2 G |
    Meaning  Different endings: pass 1 plays [1, pass 2 plays [2, ...
    End      The last ending stops at the next | or |: or the end.
    Example  |: C4 D [1 E :| [2 G | A
    See      repeat

### BAR-REPEAT {#bar-repeat}

    Syntax   %                              | C D E F | % |
    Meaning  Repeats the previous bar exactly.
    Needs    A bar line before it. No previous bar -> Warning.
    Example  | C4 D E F | % |
    See      bar, repeat

## FUNCTIONS {#fn}

### SYNTAX {#fn.syntax}

    Syntax   <name>(<args>){ ... }          up(12){ C E G }
    Builtin  Only builtin functions (no user functions, no recursion).
    Nesting  Allowed, applied inner -> outer. Depth limit 32.
    Flow     Octaves are resolved on the written text before the
             function, so notes after it are not affected.
    Folding  Results outside A0-C8 are moved by octaves, Warning.
    Unknown  Unknown name or bad arguments -> Error, body still plays.
    Example  up(12){ accent(1/3){ C4/8 D E F G A } }
    See      fn.up, fn.accent

### UP {#fn.up}

    Syntax   up(<n>){ ... }                 up(12){ C E G }
    Meaning  Raises every note by n semitones (12 = one octave).
    Example  up(12){ C4 E G }
    See      fn.down, fn.seq

### DOWN {#fn.down}

    Syntax   down(<n>){ ... }               down(2){ G A B }
    Meaning  Lowers every note by n semitones.
    Example  down(2){ G4 A B }
    See      fn.up

### ACCENT {#fn.accent}

    Syntax   accent(1/<n>){ ... }   accent(<note>){ ... }
    Meaning  Matching notes get louder (+20 velocity).
    1/n      Every n-th onset, starting with the first.
             accent(1/3){ C D E F G A } -> C and F.
    note     Every note with that name: accent(G){ ... }.
    Example  accent(1/3){ C4 D E F G A }
    See      fn.louder

### LOUDER {#fn.louder}

    Syntax   louder(<n>){ ... }   softer(<n>){ ... }
    Meaning  Adds / subtracts n to every velocity (kept within 1-127).
    Example  louder(20){ C4 D } softer(20){ E F }
    See      dynamic, fn.accent

### TUPLET {#fn.tuplet}

    Syntax   tuplet(<n>){ ... }             tuplet(3){ C/8 D E }
    Meaning  Plays the section in the time of the nearest lower
             power of two: 3 in 2, 5 in 4, 7 in 4. n >= 3.
    Example  tuplet(3){ C4/8 D E } F/4
    See      length

### SEQ {#fn.seq}

    Syntax   seq(<times>, <step>){ ... }    seq(3, 1){ C D E }
    Meaning  Plays the section `times` times, each shifted by
             `step` more semitones (a musical sequence).
             seq(3, 1){ C D E } -> C D E, C# D# F, D E F#.
    Example  seq(3, 2){ C4 D E }
    See      fn.up, repeat

## SONG {#song}

### ARRANGEMENT {#arrangement}

    Syntax   <piece> > <piece> > ...        p1 > bridge1 > p4
    File     song.piano in the project folder.
    Pieces   Each piece is independent: own tempo, defaults, pedals.
             The same piece may appear many times.
    Missing  Unknown piece -> Error, the rest still plays.
    Example  p1 > p2 > p1
    See      connector

### CONNECTOR {#connector}

    Syntax   >   fade   r/<n>   ped         p1 > fade > p2
    >        Continue directly.
    fade     Fade out the end (last bar) of the previous piece.
    r/n      Rest between pieces.
    ped      Keep the damper down across the join.
    Bridge   A user-made connecting piece is an ordinary piece.
    Example  p1 > fade > r/2 > p2 > ped > p3
    See      arrangement

### TRANSFORM {#transform}

    Syntax   <fn>(<args>){ <pieces> }       up(2){ p1 }
    Meaning  Builtin functions apply to pieces; files are unchanged.
    Example  p1 > up(2){ p1 } > softer(20){ p2 }
    See      fn.syntax

## ALERTS {#alerts}

### LEVELS {#levels}

    ✖ Error    That part cannot be played (unknown syntax, bad length).
    ⚠ Warning  Plays, but may differ from intent (folding, ambiguous
               octave, bar length, ignored tie).
    · Hint     Information (e.g. :| without |:, pedal already on).
    Where      Output view with line numbers; Code gutter marks the
               most severe alert per line.
    Example  C4 F#   → Warning: ambiguous octave
    See      octave

## SUMMARY {#summary}

### GRAMMAR {#grammar}

    note     C D E F G A B  [# b]  [0-8]  [/n [. ..]]  [~]
    rest     r [/n]
    chord    { notes } [/n] [~]
    comment  // ...
    header   tempo <bpm>   time <a>/<b>
    bar      |   %
    dynamic  ppp pp p mp mf f ff fff   cresc   dim
    pedal    ped|sos|soft on|off
    repeat   |: ... :| [x n]   [1 ... :| [2 ...
    function up down accent louder softer tuplet seq  (name(args){ ... })
    voices   voices{ ... }{ ... }
    song     piece > piece   fade   r/n   ped
    Example  tempo 90 | p C4/8 D E F G/4 A | ped on {C E G}/1 ped off |
    See      file
