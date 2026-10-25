//! Headless end-to-end tests of the app: keys, code editing, and Cmd commands.
//! Temporary projects live under the workspace `target/` directory.

use std::path::{Path, PathBuf};

use piano_app::{App, Focus, OutKind, View};
use piano_project::Project;

struct Tmp(PathBuf);

impl Tmp {
    fn new(name: &str) -> Tmp {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("app-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir.canonicalize().unwrap())
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn app(tmp: &Tmp) -> App {
    let project = Project::create(&tmp.0.join("song")).unwrap();
    App::headless(project)
}

fn run(app: &mut App, line: &str) {
    app.cmdline.set(line);
    app.submit_cmd();
}

fn last(app: &App) -> String {
    app.output
        .last_result()
        .map(|l| l.text.clone())
        .unwrap_or_default()
}

fn last_kind(app: &App) -> OutKind {
    app.output.last_result().unwrap().kind
}

#[test]
fn starts_on_first_piece() {
    let tmp = Tmp::new("start");
    let a = app(&tmp);
    assert_eq!(a.buf().name, "p1");
    assert_eq!(a.focus, Focus::Keys);
    assert_eq!(a.view, View::Output);
}

/// Types `text` on a new line at the end of the current piece (Code focus).
fn type_line(a: &mut App, text: &str) {
    a.buf_mut().goto(usize::MAX);
    a.buf_mut().end();
    a.code_newline();
    for c in text.chars() {
        a.code_char(c);
    }
}

#[test]
fn keys_only_listen() {
    let tmp = Tmp::new("keys");
    let mut a = app(&tmp);
    let before = a.buf().text();
    for c in ['z', 'x', 'c', 'Q'] {
        assert!(a.key_down(c));
    }
    assert!(!a.key_down('a'), "A is not a piano key");
    assert_eq!(a.buf().text(), before, "Keys never write to the code");
    assert!(!a.buf().dirty);
    let names: Vec<String> = a.played.iter().map(|(p, _)| p.to_string()).collect();
    assert_eq!(names, ["C4", "D4", "E4", "C5"]);
    a.shift_octave(1);
    a.key_down('z');
    assert_eq!(a.played.last().unwrap().0.to_string(), "C5");
    run(&mut a, "clear played");
    assert!(a.played.is_empty());
    run(&mut a, "dyn f");
    assert_eq!(a.input.dynamic, Some(piano_core::Dynamic::F));
    run(&mut a, "played");
    assert_eq!(a.view, View::Played);
}

#[test]
fn code_typing_and_alerts() {
    let tmp = Tmp::new("code");
    let mut a = app(&tmp);
    a.set_focus(Focus::Code);
    a.buf_mut().lines = vec![String::new()];
    for c in "C4 F# /3".chars() {
        a.code_char(c);
    }
    let alerts = &a.buf().compiled.alerts;
    assert_eq!(alerts.len(), 2, "{alerts:?}");
    // Typing coalesces into one undo step.
    run(&mut a, "undo");
    assert_eq!(a.buf().line(), "");
}

#[test]
fn files_and_sandbox() {
    let tmp = Tmp::new("files");
    let mut a = app(&tmp);
    let outside = tmp.0.join("out");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("old.mid"), b"x").unwrap();
    std::fs::write(outside.join("ext.piano"), "C D E").unwrap();

    run(&mut a, "new p2");
    assert_eq!(a.buf().name, "p2");
    for c in "C ".chars() {
        a.code_char(c);
    }
    run(&mut a, "save");
    assert_eq!(last_kind(&a), OutKind::Ok);
    assert_eq!(
        std::fs::read_to_string(a.project.piece_path("p2")).unwrap(),
        "C \n"
    );

    run(&mut a, "export midi");
    assert!(a.project.root.join("p2.mid").exists(), "{}", last(&a));

    // Outside: new files only, never modify.
    run(&mut a, "export midi ../out/new.mid");
    assert!(outside.join("new.mid").exists(), "{}", last(&a));
    run(&mut a, "export midi ../out/old.mid");
    assert_eq!(last_kind(&a), OutKind::Error);
    assert_eq!(std::fs::read(outside.join("old.mid")).unwrap(), b"x");
    run(&mut a, "rm ../out/old.mid");
    assert_eq!(last_kind(&a), OutKind::Error);
    run(&mut a, "mkdir ../out/dir");
    assert_eq!(last_kind(&a), OutKind::Error);
    run(&mut a, "mv ../out/ext.piano ext.piano");
    assert_eq!(last_kind(&a), OutKind::Error);

    // Copy in, open read-only outside.
    run(&mut a, "cp ../out/ext.piano ext.piano");
    assert!(a.project.root.join("ext.piano").exists());
    run(&mut a, "open ../out/ext.piano");
    assert!(a.buf().readonly);
    a.code_char('z');
    assert_eq!(a.buf().line(), "C D E");
    run(&mut a, "save");
    assert_eq!(last_kind(&a), OutKind::Error);

    // rm asks first.
    run(&mut a, "rm p2.mid");
    assert!(a.pending.is_some());
    run(&mut a, "y");
    assert!(!a.project.root.join("p2.mid").exists());

    run(&mut a, "ls");
    assert!(a.output.lines.iter().any(|l| l.text.contains("p2.piano*")));
    run(&mut a, "cd ../out");
    assert!(last(&a).contains("read-only"));
}

#[test]
fn quit_asks_when_unsaved() {
    let tmp = Tmp::new("quit");
    let mut a = app(&tmp);
    type_line(&mut a, "C");
    run(&mut a, "quit");
    assert!(!a.quit);
    run(&mut a, "n");
    assert!(!a.quit);
    run(&mut a, "quit");
    run(&mut a, "y");
    assert!(a.quit);
}

#[test]
fn views_and_docs() {
    let tmp = Tmp::new("views");
    let mut a = app(&tmp);
    run(&mut a, "help export");
    assert_eq!(a.view, View::Help);
    assert!(a.page.lines[0].contains("export midi"));
    run(&mut a, "man octave");
    assert_eq!(a.view, View::Man);
    assert_eq!(a.page.lines[0], "OCTAVE");
    run(&mut a, "lang ko");
    assert!(a.page.lines.iter().any(|l| l.contains("가운데 도")));
    let code_step = piano_app::tutorial::steps(piano_project::Lang::En)
        .iter()
        .position(|s| s.id == "first")
        .unwrap();
    run(&mut a, &format!("tutorial {}", code_step + 1));
    assert_eq!(a.view, View::Tutorial);
    assert!(
        a.tutorial_page()
            .lines
            .iter()
            .any(|l| l.contains("not yet"))
    );
    type_line(&mut a, "| C4 C G G | A A G/2 |");
    assert!(a.tutorial_page().lines.iter().any(|l| l.contains("✔")));
    run(&mut a, "song");
    assert_eq!(a.view, View::Song);
    run(&mut a, "song extra");
    assert_eq!(last_kind(&a), OutKind::Error);
    run(&mut a, "view song");
    assert_eq!(last_kind(&a), OutKind::Error, "`view` was replaced");
    run(&mut a, "nonsense");
    assert_eq!(last_kind(&a), OutKind::Error);
}

#[test]
fn song_and_playback_without_audio() {
    let tmp = Tmp::new("song");
    let mut a = app(&tmp);
    type_line(&mut a, "C");
    run(&mut a, "save");
    run(&mut a, "new p2");
    type_line(&mut a, "E");
    run(&mut a, "open song");
    assert!(a.buf().is_song());
    a.set_focus(Focus::Code);
    a.buf_mut().lines = vec![String::new()];
    for c in "p1 > p2 > up(12){ p1 }".chars() {
        a.code_char(c);
    }
    let names: Vec<String> = a
        .buf()
        .compiled
        .score
        .notes
        .iter()
        .map(|n| n.pitch.to_string())
        .collect();
    // p2 is unsaved but open: the song uses the buffer.
    assert_eq!(names, ["C4", "E4", "C5"]);
    run(&mut a, "play song");
    run(&mut a, "play 1");
    run(&mut a, "loop 1");
    run(&mut a, "listen C4 E G");
    run(&mut a, "stop");
    a.play_here();
    a.tick();
}

#[test]
fn completion() {
    let tmp = Tmp::new("complete");
    let mut a = app(&tmp);
    a.cmdline.set("expo");
    a.complete();
    assert_eq!(a.cmdline.text, "export ");
    a.cmdline.set("export w");
    a.complete();
    assert_eq!(a.cmdline.text, "export wav ");
    a.cmdline.set("open p");
    a.complete();
    assert_eq!(a.cmdline.text, "open p1 ");
}

#[test]
fn recovery_copy() {
    let tmp = Tmp::new("recovery");
    let mut a = app(&tmp);
    type_line(&mut a, "C");
    // Force the autosave now.
    a.buf_mut().last_edit = Some(std::time::Instant::now() - std::time::Duration::from_secs(5));
    a.tick();
    let rec = a.project.root.join(".piano/recovery/p1.piano");
    assert_eq!(std::fs::read_to_string(rec).unwrap(), "// p1\nC\n");

    // A new session sees it and can recover.
    let project = Project::open(&a.project.root).unwrap();
    drop(a);
    let mut b = App::headless(project);
    assert!(b.output.lines.iter().any(|l| l.text.contains("recover")));
    run(&mut b, "recover");
    assert_eq!(b.buf().text(), "// p1\nC\n");
}

#[test]
fn piano_keys_are_audible() {
    let tmp = Tmp::new("audible");
    let project = Project::create(&tmp.0.join("song")).unwrap();
    let mut player = piano_audio::Player::silent();
    player
        .set_soundfont(piano_audio::testing::tiny_soundfont())
        .unwrap();
    let mut a = App::headless_with(project, player);
    assert_eq!(a.player.pull_peak(0.1), 0.0);
    assert!(a.key_down('z'));
    assert!(a.player.pull_peak(0.2) > 0.01, "key press must sound");
}

#[test]
fn clear_empties_output_only() {
    let tmp = Tmp::new("clear");
    let mut a = app(&tmp);
    run(&mut a, "help");
    run(&mut a, "pwd");
    assert!(!a.output.lines.is_empty());
    let history = a.cmd_history.len();
    run(&mut a, "clear");
    assert!(a.output.lines.is_empty());
    assert_eq!(a.cmd_history.len(), history + 1);
}

#[test]
fn display_keeps_its_view_until_a_command() {
    let tmp = Tmp::new("display");
    let mut a = app(&tmp);
    run(&mut a, "tutorial");
    for focus in [Focus::Code, Focus::Cmd, Focus::Keys, Focus::Code] {
        a.set_focus(focus);
        assert_eq!(a.view, View::Tutorial, "focus changes never switch the Display");
    }
    a.escape();
    assert_eq!(a.view, View::Tutorial, "Esc leaves the Display alone");
    for (cmd, view) in [
        ("played", View::Played),
        ("ref", View::Ref),
        ("output", View::Output),
        ("song", View::Song),
        ("help", View::Help),
        ("man", View::Man),
        ("tutorial", View::Tutorial),
    ] {
        run(&mut a, cmd);
        assert_eq!(a.view, view, "{cmd}");
    }
}
