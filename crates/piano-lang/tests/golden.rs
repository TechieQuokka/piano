//! End-to-end checks: source text → notes on the timeline.

use piano_core::{PedalKind, Severity, TICKS_PER_QUARTER};
use piano_lang::{Compiled, compile_piece};

/// Renders notes as `PITCH@start+len` in eighth notes, sorted by time.
fn notes(c: &Compiled) -> String {
    let e = TICKS_PER_QUARTER / 2;
    c.score
        .notes
        .iter()
        .map(|n| format!("{}@{}+{}", n.pitch, n.start / e, n.len / e))
        .collect::<Vec<_>>()
        .join(" ")
}

fn clean(src: &str) -> Compiled {
    let c = compile_piece(src);
    assert!(c.alerts.is_empty(), "{src}: {:?}", c.alerts);
    c
}

#[test]
fn melody_with_inheritance() {
    let c = clean("C4/8 D E F G/4 A B C");
    assert_eq!(
        notes(&c),
        "C4@0+1 D4@1+1 E4@2+1 F4@3+1 G4@4+2 A4@6+2 B4@8+2 C5@10+2"
    );
}

#[test]
fn repeat_plays_identical_notes() {
    // G after C4 is G3 (nearest), every pass.
    let c = clean("|: C4 G :| x3");
    assert_eq!(notes(&c), "C4@0+2 G3@2+2 C4@4+2 G3@6+2 C4@8+2 G3@10+2");
}

#[test]
fn voltas() {
    let c = clean("|: C4 D [1 E :| [2 G | A");
    assert_eq!(
        notes(&c),
        "C4@0+2 D4@2+2 E4@4+2 C4@6+2 D4@8+2 G4@10+2 A4@12+2"
    );
}

#[test]
fn bar_repeat() {
    let c = clean("| C4 D E F | % |");
    assert_eq!(
        notes(&c),
        "C4@0+2 D4@2+2 E4@4+2 F4@6+2 C4@8+2 D4@10+2 E4@12+2 F4@14+2"
    );
}

#[test]
fn ties_merge_across_bars() {
    let c = clean("C4 D G/2~ | G/4 F");
    assert_eq!(notes(&c), "C4@0+2 D4@2+2 G4@4+6 F4@10+2");
}

#[test]
fn chord_tie() {
    let c = clean("{C4 E G}/2~ {C E G}/4");
    assert_eq!(notes(&c), "C4@0+6 E4@0+6 G4@0+6");
}

#[test]
fn ties_cross_function_bodies() {
    // into a tuplet and on inside it: G held 1 + 3 x 2/3 quarters
    let c = clean("C4/4 G4~ tuplet(3){ G~ G~ G } A");
    let q = TICKS_PER_QUARTER;
    let g = c.score.notes.iter().find(|n| n.pitch.midi() == 67).unwrap();
    assert_eq!((g.start, g.len), (q, q + q * 2 / 3 * 3));
    assert_eq!(c.score.notes.len(), 3);
    // out of a tuplet
    let c = clean("tuplet(3){ C4/8 E G4~ } G/4");
    let g = c.score.notes.iter().find(|n| n.pitch.midi() == 67).unwrap();
    assert_eq!((g.start, g.len), (q * 2 / 3, q / 3 + q));
    // ties match the pitch after the function: C up 2 is D
    let c = clean("up(2){ C4/4~ } D");
    assert_eq!(notes(&c), "D4@0+4");
}

#[test]
fn hundred_twenty_eighths_and_nonuplets_are_exact() {
    let q = TICKS_PER_QUARTER;
    let c = clean("C4/128 D E F G/4");
    assert_eq!(c.score.notes[4].start, q / 8);
    // nine sixteenths in the time of eight: each lasts 2/9 of a beat
    let c = clean("tuplet(9){ C4/16 D E F G A B C D } E/4");
    assert_eq!(c.score.notes[1].start, q * 2 / 9);
    assert_eq!(c.score.notes[9].start, q * 2);
    let c = clean("tuplet(11){ C4/16 D E F G A B C D E F } G/4");
    assert_eq!(c.score.notes[1].start * 11, q * 2);
}

#[test]
fn tie_to_other_pitch_warns() {
    let c = compile_piece("C4~ D");
    assert_eq!(notes(&c), "C4@0+2 D4@2+2");
    assert_eq!(c.alerts.len(), 1);
    assert_eq!(c.alerts[0].severity, Severity::Warning);
}

#[test]
fn bar_length_check() {
    let c = compile_piece("| C4 D E | F G A B |");
    assert_eq!(c.alerts.len(), 1);
    assert_eq!(
        c.alerts[0].to_string(),
        "⚠ line 1: bar has 3 beats, 4/4 expects 4 (bar lines are optional)"
    );
    // A short first bar (pickup) is fine.
    clean("C4 | D E F G |");
}

#[test]
fn time_signature_changes_bar_check() {
    clean("time 3/4 | C4 D E | F G A |");
}

#[test]
fn functions_nest_inner_first() {
    let c = clean("up(12){ accent(1/3){ C4/8 D E F G A } }");
    assert_eq!(notes(&c), "C5@0+1 D5@1+1 E5@2+1 F5@3+1 G5@4+1 A5@5+1");
    let v: Vec<u8> = c.score.notes.iter().map(|n| n.velocity).collect();
    assert_eq!(v, [100, 80, 80, 100, 80, 80]);
}

#[test]
fn function_does_not_disturb_flow() {
    // B after the function resolves against the written G4, not the raised G5.
    let c = clean("up(12){ C4 E G } B");
    assert_eq!(notes(&c), "C5@0+2 E5@2+2 G5@4+2 B4@6+2");
}

#[test]
fn accent_by_pitch() {
    let c = clean("accent(G){ C4 G E G }");
    let v: Vec<u8> = c.score.notes.iter().map(|n| n.velocity).collect();
    assert_eq!(v, [80, 100, 80, 100]);
}

#[test]
fn tuplet_triplet() {
    let c = clean("tuplet(3){ C4/8 D E } F/4");
    let q = TICKS_PER_QUARTER;
    let starts: Vec<_> = c.score.notes.iter().map(|n| n.start).collect();
    assert_eq!(starts, [0, q / 3, 2 * q / 3, q]);
}

#[test]
fn sequence() {
    let c = clean("seq(3, 1){ C4 D E }");
    let names: Vec<String> = c.score.notes.iter().map(|n| n.pitch.to_string()).collect();
    assert_eq!(
        names,
        ["C4", "D4", "E4", "C#4", "D#4", "F4", "D4", "E4", "F#4"]
    );
}

#[test]
fn octave_folding_in_function() {
    let c = compile_piece("up(12){ up(12){ up(12){ C7 } } }");
    // C10 folds down to C8, the top key.
    assert_eq!(notes(&c), "C8@0+2");
    assert!(c.alerts.iter().all(|a| a.severity == Severity::Warning));
    assert!(!c.alerts.is_empty());
}

#[test]
fn pedals_and_tempo() {
    let c = clean("tempo 90 ped on C4 E ped off sos on G sos off");
    assert_eq!(c.score.tempo_at(0), 90);
    let kinds: Vec<(PedalKind, bool)> = c.score.pedals.iter().map(|p| (p.kind, p.on)).collect();
    assert_eq!(
        kinds,
        [
            (PedalKind::Damper, true),
            (PedalKind::Damper, false),
            (PedalKind::Sostenuto, true),
            (PedalKind::Sostenuto, false)
        ]
    );
}

#[test]
fn design_example_compiles() {
    let src = "tempo 90  time 4/4            // header
p cresc
| C4/8 D E F G/4 A~ | A/2 f {C5 E G}/2 |
ped on
|: up(12){ accent(1/3){ C/8 D E F G A } } r/4 :| x2
ped off";
    let c = compile_piece(src);
    assert!(
        c.alerts.iter().all(|a| a.severity != Severity::Error),
        "{:?}",
        c.alerts
    );
    assert!(!c.score.notes.is_empty());
}

#[test]
fn garbage_never_panics() {
    for src in [
        "",
        "}}}",
        "|: [1 [2 :| :|",
        "up(",
        "up(1){",
        "{",
        "C/",
        "~~~",
        "x3 x 5",
        "tempo",
        "time 4",
        "ped",
        "%%%",
        "C9 A0 Cb0 B#8",
        "accent(0/0){C}",
        "tuplet(1){C}",
        "seq(0,1){C}",
        "> > >",
    ] {
        let _ = compile_piece(src);
    }
}

#[test]
fn voices_sound_together() {
    let c = clean("| voices{ r/2 G4/4 G } { C3/1 } | D4/4 E F G |");
    assert_eq!(
        notes(&c),
        "C3@0+8 G4@4+2 G4@6+2 D4@8+2 E4@10+2 F4@12+2 G4@14+2"
    );
}

#[test]
fn voices_bars_continue_the_current_bar() {
    let c = clean("| C4/2 voices{ D/2 | E/1 }{ F/2 | G/1 } |");
    assert_eq!(notes(&c), "C4@0+4 D4@4+4 F4@4+4 E4@8+8 G4@8+8");
    let c = compile_piece("| C4/2 voices{ D/4 | E/1 }{ F/2 | G/1 } |");
    assert!(
        c.alerts
            .iter()
            .any(|a| a.message.contains("bar has 3 beats")),
        "{:?}",
        c.alerts
    );
}

#[test]
fn voices_of_different_lengths() {
    let c = compile_piece("voices{ C4/1 }{ E4/2 } D");
    assert_eq!(c.alerts.len(), 1, "{:?}", c.alerts);
    assert_eq!(c.alerts[0].severity, Severity::Warning);
    assert_eq!(c.score.notes.last().unwrap().start, 4 * TICKS_PER_QUARTER);
}

#[test]
fn voices_share_the_pedal() {
    let c = clean("voices{ ped on C4/2 ped off }{ E4/4 F } ped on G ped off");
    let on: Vec<bool> = c
        .score
        .pedals
        .iter()
        .filter(|p| p.kind == PedalKind::Damper)
        .map(|p| p.on)
        .collect();
    assert_eq!(on, [true, false, true, false]);
}

#[test]
fn bar_repeat_after_voices() {
    let c = clean("| voices{ C4/2 D }{ E3/1 } | % |");
    assert_eq!(notes(&c), "E3@0+8 C4@0+4 D4@4+4 E3@8+8 C4@8+4 D4@12+4");
}

#[test]
fn a_free_bar_has_any_length() {
    // Six beats in 4/4, then bars are checked again.
    let c = clean("| C4 D E F | free{ G/8 A B C D E F G A B C D } | C4/1 |");
    assert_eq!(c.score.notes.len(), 17);
    assert_eq!(c.score.notes.last().unwrap().start, 10 * TICKS_PER_QUARTER);
    let c = compile_piece("| free{ C4/1 D/2 } | E/4 F G | A/1 |");
    assert_eq!(c.alerts.len(), 1, "{:?}", c.alerts);
    assert!(c.alerts[0].message.contains("bar has 3 beats"));
}

#[test]
fn free_bars_in_voices_and_ties_through_free() {
    let c = clean("| voices{ free{ C4/1 D/2 } | E/1 }{ free{ C3/1. } | G2/1 } |");
    assert_eq!(c.score.notes.len(), 5);
    let c = clean("| C4/1~ | free{ C/2 D/1 } |");
    assert_eq!(notes(&c), "C4@0+12 D4@12+8");
}

#[test]
fn bar_lines_inside_free_are_errors() {
    let c = compile_piece("free{ C4 D | E F G }");
    assert_eq!(c.alerts.len(), 1, "{:?}", c.alerts);
    assert_eq!(c.alerts[0].severity, Severity::Error);
    assert_eq!(c.score.notes.len(), 5);
}

#[test]
fn a_tie_inside_a_chord_holds_only_that_note() {
    let c = clean("{C4~ E G}/2 C/4");
    assert_eq!(notes(&c), "C4@0+6 E4@0+4 G4@0+4");
    let c = clean("{C4 E~ G}/2 {E G C5}/2");
    assert_eq!(notes(&c), "C4@0+4 E4@0+8 G4@0+4 G4@4+4 C5@4+4");
    let c = compile_piece("{C4~ E}/2 D");
    assert_eq!(c.alerts.len(), 1, "{:?}", c.alerts);
    assert!(c.alerts[0].message.contains("not followed"));
}

#[test]
fn time_inside_voices_applies_to_every_voice() {
    clean("| time 2/4 voices{ C4/4 D~ | time 3/4 D/4 E F }{ E3/2 | G/2. } | C4/2. |");
    // in the second voice too, and after the block
    clean("| C4/1 | voices{ D4/2. }{ time 3/4 F3/2. } | E4/2. |");
    let c = compile_piece("| voices{ C4/2. }{ time 3/4 E3/2. } | C4/1 |");
    assert_eq!(c.alerts.len(), 1, "{:?}", c.alerts);
    assert!(c.alerts[0].message.contains("3/4 expects 3"));
}

#[test]
fn tempo_inside_voices_applies_to_every_voice() {
    let c = clean("voices{ C4/1 }{ r/2 tempo 60 E4/2 } D");
    let q = TICKS_PER_QUARTER;
    assert!(c.score.tempos.iter().any(|t| t.at == 2 * q && t.bpm == 60));
    let c = compile_piece("voices{ tempo 80 C4 }{ tempo 90 E4 }");
    assert_eq!(c.alerts.len(), 1, "{:?}", c.alerts);
    assert!(c.alerts[0].message.contains("`tempo` differently"));
    assert!(c.score.tempos.iter().all(|t| t.bpm != 90));
}

#[test]
fn time_inside_functions_and_repeats_goes_on_after_them() {
    clean("| C4 D E F |: time 3/4 C D E :| F G A | B C D |");
    clean("| C4 D E F | up(2){ time 3/4 C D E } | F G A |");
    clean("| C4 D E F |: G A B C [1 time 3/4 D E F :| [2 time 2/4 G A | B C |");
}

#[test]
fn the_last_bar_may_be_short() {
    clean("G4 | C5 C C D | E/2 C/4 |");
    clean("| C4 D E F | G A |");
    clean("ped on | C4 D E F | G A | ped off");
    clean("| voices{ C4/1 }{ C3/1 } | voices{ D4/2 }{ D3/2 } |");
    let warns = |src: &str| {
        let c = compile_piece(src);
        assert_eq!(c.alerts.len(), 1, "{src}: {:?}", c.alerts);
        assert!(c.alerts[0].message.contains("bar has"), "{src}");
    };
    warns("| C4 D E F | G A | B C D E |"); // short in the middle
    warns("| C4 D E F | G A | r"); // something after it
    warns("| C4 D E F | G A B C D |"); // longer is never fine
    warns("| voices{ C4/1 }{ C3/1 } | voices{ D4/2 }{ D3/2 } | E4/1 |");
}

#[test]
fn bar_lines_inside_functions_continue_the_bar() {
    clean("| C4 D up(2){ E F | G A B C } |");
    // in the squeezed time of a tuplet: F G A is one beat
    clean("| C4 D E tuplet(3){ F/8 G A | B C D } E/4 F G |");
    // through every pass of seq
    clean("| C4 D E F | G A seq(2, 12){ B C | D E } F G |");
    let c = compile_piece("| C4 D up(2){ E | F G A B } |");
    assert_eq!(c.alerts.len(), 1, "{:?}", c.alerts);
    assert!(c.alerts[0].message.contains("bar has 3 beats"));
}

#[test]
fn ties_go_into_and_out_of_voices() {
    let c = clean("C4/2~ voices{ C/2 }{ E3/2 }");
    assert_eq!(notes(&c), "C4@0+8 E3@4+4");
    let c = clean("voices{ C4/2~ }{ E3/2 } C/2");
    assert_eq!(notes(&c), "E3@0+4 C4@0+8");
    // from block to block, as the bars go
    let c = clean("| voices{ C4/1~ }{ E3/1 } | voices{ C4/2 }{ G3/2 } |");
    assert_eq!(notes(&c), "E3@0+8 C4@0+12 G3@8+4");
    // the first voice with the note takes the tie
    let c = clean("C4/2~ voices{ C/2 }{ C/2 }");
    assert_eq!(notes(&c), "C4@0+8 C4@4+4");
    // a voice that ends before the block cannot tie out of it
    let c = compile_piece("voices{ C4/4~ }{ E3/2 } C/2");
    assert_eq!(c.alerts.len(), 2, "{:?}", c.alerts);
    assert!(c.alerts.iter().any(|a| a.message.contains("not followed")));
}

#[test]
fn bars_go_on_through_repeats_as_heard() {
    // a pickup inside the repeat: the last bar of each pass completes it
    clean("|: G4 | C5 C C D | E/2 C/4 :|");
    // a pickup before it
    clean("G4 |: C5 C C D | E/2 C/2 :|");
    clean("|: C4 D E F [1 G/1 :| [2 C/1 |");
    // % after a repeat copies its last bar as heard
    let c = clean("| C4 D E F |: G A B C :| % |");
    assert_eq!(c.score.notes.len(), 16);
    let warns = |src: &str, what: &str| {
        let c = compile_piece(src);
        assert_eq!(c.alerts.len(), 1, "{src}: {:?}", c.alerts);
        assert!(c.alerts[0].message.contains(what), "{src}: {:?}", c.alerts);
    };
    // a short bar right after `|:` that nothing completes (warned once, not per pass)
    warns(
        "| C4 D E F |: G A | B C D E :| F G A B |",
        "bar has 2 beats",
    );
    warns("| C4 D E F |: G A B C D :|", "bar has 5 beats");
}

/// Renders notes as `PITCH:velocity:release` with the release in 32nds of the start.
fn touch(c: &Compiled) -> String {
    let t = TICKS_PER_QUARTER / 8;
    c.score
        .notes
        .iter()
        .map(|n| format!("{}:{}:{}", n.pitch, n.velocity, (n.release() - n.start) / t))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn marks_shorten_and_stress_notes() {
    // A quarter is 8 32nds: `'` sounds 4, `''` 2, `'_` 6; `^` +20, `_` +8.
    let c = clean("mf C4' D'' E'_ F^ G_ A");
    assert_eq!(
        touch(&c),
        "C4:80:4 D4:80:2 E4:88:6 F4:100:8 G4:88:8 A4:80:8"
    );
    // The written lengths, and so the bars, do not change.
    assert_eq!(notes(&c), "C4@0+2 D4@2+2 E4@4+2 F4@6+2 G4@8+2 A4@10+2");
    // Not inherited; chords take marks after `}`, with or without a length.
    let c = clean("{C4 E}/8'^ G {C E}' G");
    assert_eq!(
        touch(&c),
        "C4:100:2 E4:100:2 G3:80:4 C4:80:2 E4:80:2 G3:80:4"
    );
}

#[test]
fn a_tie_holds_on_and_the_last_note_lets_go() {
    let c = clean("C4/2~ C/4' | {C E}/4~ {C E}/4''");
    assert_eq!(touch(&c), "C4:80:20 C4:80:10 E4:80:10");
    let c = compile_piece("C4/4'~ C");
    assert_eq!(c.alerts.len(), 1, "{:?}", c.alerts);
    assert_eq!(touch(&c), "C4:80:16");
}

#[test]
fn marks_out_of_place_warn() {
    for src in ["C/4~^ C", "C'/4", "r'", "' C", "{C' E}"] {
        let c = compile_piece(src);
        assert_eq!(c.alerts.len(), 1, "{src}: {:?}", c.alerts);
        assert_eq!(c.alerts[0].severity, Severity::Warning, "{src}");
    }
}

fn velocities(c: &Compiled) -> Vec<u8> {
    c.score.notes.iter().map(|n| n.velocity).collect()
}

#[test]
fn sf_stresses_one_note() {
    // Two levels above the current one, at least f; only the next note.
    let c = clean("p C4 sf D E mf sfz F G ff rfz A B");
    assert_eq!(velocities(&c), [49, 96, 49, 112, 80, 127, 112]);
    // Before a chord, every note; marks add on top.
    let c = clean("pp sf {C4 E}/4^ G");
    assert_eq!(velocities(&c), [116, 116, 33]);
    // In a cresc, from the level reached there (72: between mp and mf, mp).
    let c = clean("p cresc C4 D sf E F f G");
    assert_eq!(velocities(&c), [49, 60, 96, 84, 96]);
}

#[test]
fn fp_is_loud_then_soft() {
    let c = clean("f C4 fp D E F ff sfp G A ffp B C");
    assert_eq!(velocities(&c), [96, 96, 49, 49, 127, 49, 112, 49]);
}

#[test]
fn sf_without_a_note_warns() {
    for src in ["C4 sf", "voices{ C sf }{ E }", "sf sf C"] {
        let c = compile_piece(src);
        assert_eq!(c.alerts.len(), 1, "{src}: {:?}", c.alerts);
        assert_eq!(c.alerts[0].severity, Severity::Warning, "{src}");
    }
}

/// Renders notes as `PITCH@start+len` in 64ths.
fn fine(c: &Compiled) -> String {
    let t = TICKS_PER_QUARTER / 16;
    c.score
        .notes
        .iter()
        .map(|n| format!("{}@{}+{}", n.pitch, n.start / t, n.len / t))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn grace_notes_take_the_time_of_their_note() {
    // Each a 32nd, on the beat; the note starts after them. Bars do not change.
    let c = clean("time 2/4 | C5/4 grace{ D5 } C5/4 | grace{ B4 C5 } D5/8 C5 B4/4 |");
    assert_eq!(
        fine(&c),
        "C5@0+16 D5@16+2 C5@18+14 B4@32+2 C5@34+2 D5@36+4 C5@40+8 B4@48+16"
    );
    // Shortened to fit in half of a short note; a chord counts once.
    let c = clean("grace{ C4 {D F} E } G/16 r/8.");
    assert_eq!(fine(&c), "C4@0+0 D4@0+0 F4@0+0 E4@1+0 G4@1+2");
    // The octave flows through them, the length does not.
    let c = clean("C4/8 grace{ G } A B");
    assert_eq!(fine(&c), "C4@0+8 G3@8+2 A3@10+6 B3@16+8");
}

#[test]
fn grace_notes_out_of_place_warn() {
    for src in [
        "grace{ C4 }",
        "grace{ C4 } r D",
        "C4~ grace{ D } C",
        "grace{ C4/8 } D",
        "grace{ C4 | } D",
    ] {
        let c = compile_piece(src);
        assert!(!c.alerts.is_empty(), "{src}");
    }
    // A bar line or a pedal may stand between them and their note.
    clean("| C4/1 | grace{ D } | ped on C/1 ped off |");
}

#[test]
fn a_fermata_holds_twice_as_long() {
    // tempo 60: a quarter is 1 s; the held half note takes 4 s, the held rest 2 s.
    let c = clean("tempo 60 | C4 D E/2@ | {C E G}/4 r@ C/2 |");
    let q = TICKS_PER_QUARTER;
    assert_eq!(c.score.holds, [(2 * q, 4 * q), (5 * q, 6 * q)]);
    let secs: Vec<f64> = c
        .score
        .notes
        .iter()
        .map(|n| c.score.seconds_at(n.start))
        .collect();
    assert_eq!(secs, [0.0, 1.0, 2.0, 6.0, 6.0, 6.0, 9.0]);
    // The bars still count the written lengths.
    assert_eq!(
        notes(&c),
        "C4@0+2 D4@2+2 E4@4+4 C4@8+2 E4@8+2 G4@8+2 C4@12+4"
    );
    // Other marks on a rest warn.
    assert_eq!(compile_piece("r'@").alerts.len(), 1);
}

#[test]
fn arp_rolls_a_chord_from_the_bottom() {
    let c = clean("arp{G4 C4 E}/2 {C4 E}/2");
    let rolls: Vec<(String, u16)> = c
        .score
        .notes
        .iter()
        .map(|n| (n.pitch.to_string(), n.roll_ms))
        .collect();
    let want = [("C4", 0), ("E4", 30), ("G4", 60), ("C4", 0), ("E4", 0)];
    assert_eq!(rolls, want.map(|(p, r)| (p.to_string(), r)));
    // Written start, length, and marks as any chord.
    let c = clean("arp{C4 E G}/4' r");
    assert_eq!(notes(&c), "C4@0+2 E4@0+2 G4@0+2");
}

#[test]
fn a_cresc_goes_on_into_the_same_voice_of_the_next_block() {
    // Each voice's cresc crosses the bar line into the next block, note by note.
    let c = clean(
        "| voices{ p cresc C5 D E F }{ pp cresc C3 D E F } \
         | voices{ G A f B/2 }{ G3 A p B/2 } |",
    );
    let v = |octave: &str| -> Vec<u8> {
        c.score
            .notes
            .iter()
            .filter(|n| n.pitch.to_string().ends_with(octave))
            .map(|n| n.velocity)
            .collect()
    };
    assert_eq!(v("5"), [49, 56, 64, 72, 80, 88, 96]);
    assert_eq!(v("3"), [33, 35, 38, 41, 43, 46, 49]);
    // After the last block, the first voice's cresc goes on in the main line.
    let c = clean("voices{ p cresc C4 D }{ E3/2 } E F f G");
    assert_eq!(velocities(&c), [80, 49, 60, 72, 84, 96]);
    // A voice whose cresc meets no block and no mark warns.
    let c = compile_piece("voices{ C4 }{ p cresc E3 } G");
    assert_eq!(c.alerts.len(), 1, "{:?}", c.alerts);
}
