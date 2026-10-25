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
