//! Binary `piano`: entry point for the `edit`, `play`, `export`, `new`, `import`, and `doctor` subcommands.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use piano_app::App;
use piano_core::{Score, Severity};
use piano_project::{CONFIG_FILE, Project, SONG};

#[derive(Parser)]
#[command(
    name = "piano",
    version,
    about = "Piano composition editor for the terminal"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Open the editor (default; DIR defaults to the current directory)
    Edit { dir: Option<PathBuf> },
    /// Play a project's song, or a .piano file, without the editor
    Play {
        target: PathBuf,
        /// Line range, e.g. 2-4
        #[arg(long)]
        lines: Option<String>,
    },
    /// Export a project's song, or a .piano file, to MIDI, WAV, or FLAC
    Export {
        target: PathBuf,
        #[arg(long, conflicts_with_all = ["wav", "flac"])]
        midi: bool,
        #[arg(long, conflicts_with = "flac")]
        wav: bool,
        /// Lossless FLAC (same sound as WAV, about half the size)
        #[arg(long)]
        flac: bool,
        /// Sample rate in Hz for WAV / FLAC (default: the SoundFont's, e.g. 48000)
        #[arg(long)]
        rate: Option<u32>,
        /// Bits per sample for WAV / FLAC: 16 or 24 (default 24)
        #[arg(long)]
        bits: Option<u16>,
        /// Output file (default: <name>.mid / .wav / .flac in the current directory;
        /// the song is named after the project folder)
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Create a project in any folder (existing files are kept)
    New { dir: PathBuf },
    /// Import a score (MusicXML or Humdrum kern), or every score in a folder, as new projects
    #[command(after_help = "Examples:\n  \
        piano import score.mxl my_song\n  \
        piano import sonata14-1.krn moonlight\n  \
        piano import ~/Downloads/kern beethoven     \
        # beethoven/sonata01-1/, beethoven/sonata01-2/, ...")]
    Import {
        /// A score file (.mxl, .musicxml, .xml, .krn), or a folder of them (not its subfolders)
        #[arg(value_name = "SOURCE")]
        file: PathBuf,
        /// The new project's folder. For a folder of scores: the folder that gets one
        /// project per score, named after its file
        dir: PathBuf,
    },
    /// Check SoundFont, audio device, and terminal keys
    Doctor,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.cmd.unwrap_or(Cmd::Edit { dir: None }) {
        Cmd::Edit { dir } => edit(&dir.unwrap_or_else(|| PathBuf::from("."))),
        Cmd::Play { target, lines } => play(&target, lines.as_deref()),
        Cmd::Export {
            target,
            midi: _,
            wav,
            flac,
            rate,
            bits,
            out,
        } => {
            let audio = if flac {
                Some("flac")
            } else if wav {
                Some("wav")
            } else {
                None
            };
            export(&target, audio, rate, bits, out.as_deref())
        }
        Cmd::New { dir } => new(&dir),
        Cmd::Import { file, dir } => import(&file, &dir),
        Cmd::Doctor => doctor(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("piano: {e}");
            ExitCode::FAILURE
        }
    }
}

fn edit(dir: &Path) -> Result<(), String> {
    if !dir.join(CONFIG_FILE).exists() {
        return Err(format!(
            "{} is not a project (no {CONFIG_FILE}). Create one with: piano new {}",
            dir.display(),
            dir.display()
        ));
    }
    let project = Project::open(dir).map_err(|e| e.to_string())?;
    let mut app = App::new(project);
    piano_tui::run(&mut app).map_err(|e| e.to_string())
}

fn new(dir: &Path) -> Result<(), String> {
    let p = Project::create(dir).map_err(|e| e.to_string())?;
    println!("created project {}", p.root.display());
    println!("open it with: piano edit {}", dir.display());
    Ok(())
}

/// Imports a score into a new project; files that exist are never overwritten.
fn import(file: &Path, dir: &Path) -> Result<(), String> {
    if file.is_dir() {
        return import_folder(file, dir);
    }
    let imp = import_score(file, dir)?;
    report(&imp, dir)
}

/// Score files `piano import` reads.
const SCORE_EXTENSIONS: [&str; 4] = ["mxl", "musicxml", "xml", "krn"];

/// Score files directly in `dir`, sorted.
fn scores_in(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.extension().is_some_and(|x| {
                    SCORE_EXTENSIONS.contains(&x.to_string_lossy().to_lowercase().as_str())
                })
        })
        .collect();
    files.sort();
    Ok(files)
}

/// Folders below `dir` (a few levels, not hidden ones) that hold scores, with their
/// number of scores.
fn score_folders(dir: &Path, depth: usize) -> Vec<(PathBuf, usize)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut subs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_dir()
                && !p
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        })
        .collect();
    subs.sort();
    let mut out = Vec::new();
    for sub in subs {
        let n = scores_in(&sub).map_or(0, |f| f.len());
        if n > 0 {
            out.push((sub.clone(), n));
        }
        if depth > 1 {
            out.extend(score_folders(&sub, depth - 1));
        }
    }
    out
}

/// Folders shown when a folder has no scores of its own.
const MAX_HINTS: usize = 5;

/// Imports each score directly in `src` (by name) into `out/<file name>`.
/// Existing projects are skipped; a failed score does not stop the rest.
fn import_folder(src: &Path, out: &Path) -> Result<(), String> {
    let files = scores_in(src).map_err(|e| format!("{}: {e}", src.display()))?;
    if files.is_empty() {
        let mut msg = format!(
            "{}: no scores (.{})",
            src.display(),
            SCORE_EXTENSIONS.join(" .")
        );
        let found = score_folders(src, 3);
        for (dir, n) in found.iter().take(MAX_HINTS) {
            let rel = dir.strip_prefix(src).unwrap_or(dir);
            msg += &format!(
                "\n  {}/ has {}: piano import {} {}",
                rel.display(),
                count(*n, "score"),
                dir.display(),
                out.display()
            );
        }
        if found.len() > MAX_HINTS {
            msg += &format!(
                "\n  ... and {} more",
                count(found.len() - MAX_HINTS, "folder")
            );
        }
        return Err(msg);
    }
    let (mut ok, mut mismatch, mut failed, mut skipped) = (0, 0, 0, 0);
    for file in &files {
        let name = file
            .file_stem()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let dir = out.join(&name);
        let used = std::fs::read_dir(&dir).is_ok_and(|mut d| d.next().is_some());
        if used {
            println!("  skip  {name}: {} is not empty", dir.display());
            skipped += 1;
            continue;
        }
        match import_score(file, &dir) {
            Ok(imp) if imp.check.ok() => {
                ok += 1;
                println!(
                    "  ok    {name}: {} · {} · {}",
                    imp.title.as_deref().unwrap_or("score"),
                    count(imp.stats.measures, "bar"),
                    count(imp.pieces.len(), "piece")
                );
            }
            Ok(imp) => {
                mismatch += 1;
                let c = &imp.check;
                println!(
                    "  check {name}: {} · {}/{} notes match, {} extra, {} ✖",
                    imp.title.as_deref().unwrap_or("score"),
                    c.matched,
                    c.expected,
                    c.extra,
                    count(c.alerts.len(), "alert")
                );
                let lines = problems(c);
                for l in lines.iter().take(MAX_PROBLEMS) {
                    println!("          {l}");
                }
                if lines.len() > MAX_PROBLEMS {
                    println!(
                        "          ... {} more: piano import {} <empty folder> lists all",
                        lines.len() - MAX_PROBLEMS,
                        file.display()
                    );
                }
            }
            Err(e) => {
                failed += 1;
                println!("  fail  {name}: {e}");
            }
        }
    }
    println!(
        "{} → {}: {ok} imported, {mismatch} not matching, {failed} failed, {skipped} skipped",
        count(files.len(), "score"),
        out.display()
    );
    if mismatch + failed > 0 {
        return Err(format!(
            "{} of {} did not import cleanly (see above)",
            mismatch + failed,
            count(files.len(), "score")
        ));
    }
    Ok(())
}

/// Reads `file` and writes it as a new project in `dir`.
fn import_score(file: &Path, dir: &Path) -> Result<piano_import::Import, String> {
    if dir.join(CONFIG_FILE).exists() {
        return Err(format!("{} is already a project", dir.display()));
    }
    let xml = piano_import::read_file(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let source = file.file_name().map_or_else(
        || file.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let imp = piano_import::import(&xml, &source).map_err(|e| format!("{source}: {e}"))?;
    let names = imp.pieces.iter().map(|(n, _)| n.as_str()).chain([SONG]);
    if let Some(p) = names
        .map(|n| dir.join(format!("{n}.piano")))
        .find(|p| p.exists())
    {
        return Err(format!(
            "{} exists; import into an empty folder",
            p.display()
        ));
    }
    let had_p1 = dir.join("p1.piano").exists();
    let mut project = Project::create(dir).map_err(|e| e.to_string())?;
    if !had_p1 {
        // `Project::create` adds an empty p1; the import has its own pieces.
        let _ = std::fs::remove_file(project.piece_path("p1"));
    }
    let (beats, unit) = imp.time;
    project.config.defaults.time = Some(format!("{beats}/{unit}"));
    project.save_config().map_err(|e| e.to_string())?;
    let write = |name: &str, text: &str| {
        std::fs::write(project.piece_path(name), text).map_err(|e| format!("{name}: {e}"))
    };
    for (name, text) in &imp.pieces {
        write(name, text)?;
    }
    write(SONG, &imp.song)?;
    Ok(imp)
}

/// Problems shown for a score in a folder import.
const MAX_PROBLEMS: usize = 3;

/// What does not match: the alerts, then the missing notes.
fn problems(c: &piano_import::Check) -> Vec<String> {
    c.alerts
        .iter()
        .cloned()
        .chain(
            c.missing
                .iter()
                .map(|(at, midi)| format!("missing: MIDI {midi} at quarter {at}")),
        )
        .collect()
}

/// `1 bar`, `2 bars`.
fn count(n: usize, one: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {one}s")
    }
}

/// Prints what an import did; an error if the song does not match the score.
fn report(imp: &piano_import::Import, dir: &Path) -> Result<(), String> {
    let s = &imp.stats;
    let c = &imp.check;
    println!(
        "imported {} → {}",
        imp.title.as_deref().unwrap_or("score"),
        dir.display()
    );
    println!(
        "  {} · {} · {} · {} · {}",
        count(s.measures, "bar"),
        count(s.voices, "voice"),
        count(imp.pieces.len(), "piece"),
        count(s.blocks, "block"),
        count(s.tempos, "tempo mark")
    );
    let mut approx = Vec::new();
    if s.graces > 0 {
        approx.push(format!(
            "{} (32nds on the beat)",
            count(s.graces, "grace note")
        ));
    }
    if s.trills > 0 {
        approx.push(format!("{} (32nds)", count(s.trills, "trill")));
    }
    if s.ornaments > 0 {
        approx.push(format!(
            "{} (mordents and turns, in 32nds)",
            count(s.ornaments, "ornament")
        ));
    }
    if !approx.is_empty() {
        println!("  approximated: {}", approx.join(", "));
    }
    let skipped: Vec<String> = [
        (s.notices.cue, "cue note"),
        (s.notices.unpitched, "unpitched note"),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, what)| count(n, what))
    .collect();
    if !skipped.is_empty() {
        println!("  skipped: {}", skipped.join(", "));
    }
    if s.notices.runaway_repeats > 0 {
        println!("  repeats did not end; unrolling stopped");
    }
    println!(
        "  check: {}/{} notes match, {} extra, {} {}",
        c.matched,
        c.expected,
        c.extra,
        count(c.alerts.len(), "alert"),
        if c.ok() { "✔" } else { "✖" }
    );
    if !c.ok() {
        let lines = problems(c);
        for l in lines.iter().take(10) {
            println!("    {l}");
        }
        if lines.len() > 10 {
            println!("    ... {} more", lines.len() - 10);
        }
        return Err("the imported song does not match the score (files were written)".into());
    }
    println!("  play it with: piano play {}", dir.display());
    Ok(())
}

/// Compiles a project directory (its song) or a `.piano` file.
/// Returns the app (for options and sound) and the score name.
fn load(target: &Path) -> Result<(App, String), String> {
    let (dir, name) = if target.is_dir() {
        (target.to_path_buf(), SONG.to_string())
    } else {
        let name = target
            .file_stem()
            .ok_or("not a .piano file")?
            .to_string_lossy()
            .into_owned();
        let dir = target
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        (
            if dir.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                dir
            },
            name,
        )
    };
    if !dir.join(CONFIG_FILE).exists() {
        return Err(format!(
            "{} is not inside a project (no {CONFIG_FILE})",
            target.display()
        ));
    }
    let project = Project::open(&dir).map_err(|e| e.to_string())?;
    Ok((App::for_tool(project), name))
}

fn compiled(app: &App, name: &str) -> Result<Arc<Score>, String> {
    let (score, compiled) = app
        .score_of(name)
        .ok_or_else(|| format!("no piece `{name}`"))?;
    for a in &compiled.alerts {
        eprintln!("{}", a.located(name));
    }
    if compiled
        .alerts
        .iter()
        .any(|a| a.severity == Severity::Error)
    {
        eprintln!("(errors above were skipped)");
    }
    Ok(score)
}

fn play(target: &Path, lines: Option<&str>) -> Result<(), String> {
    let (app, name) = load(target)?;
    let score = compiled(&app, &name)?;
    if let Some(e) = app.player.device_error() {
        return Err(format!("audio: {e}"));
    }
    if app.player.sound_font().is_none() {
        return Err(
            "no SoundFont: set one with `font <path>` in the editor, or in piano.toml [sound]"
                .into(),
        );
    }
    let (from, to) = match lines {
        None => (0, None),
        Some(l) => {
            let (a, b) = piano_app::cmd::parse_lines(l).ok_or("lines: e.g. 2-4")?;
            let (f, t) = piano_lang::line_range(&score, a, b).ok_or("no notes on those lines")?;
            (f, Some(t))
        }
    };
    let total = score.seconds_at(to.unwrap_or(score.end)) - score.seconds_at(from);
    println!("▶ {name} ({})", piano_app::app::clock(total));
    app.player.play(Arc::clone(&score), from, to);
    let report = || {
        for e in app.player.take_errors() {
            eprintln!("audio: {e}");
        }
    };
    while app.player.is_playing() {
        std::thread::sleep(Duration::from_millis(100));
        report();
    }
    // Let the last notes ring.
    std::thread::sleep(Duration::from_millis(1500));
    report();
    Ok(())
}

fn export(
    target: &Path,
    audio: Option<&str>,
    rate: Option<u32>,
    bits: Option<u16>,
    out: Option<&Path>,
) -> Result<(), String> {
    let (app, name) = load(target)?;
    let score = compiled(&app, &name)?;
    let ext = audio.unwrap_or("mid");
    let out = out.map_or_else(
        || PathBuf::from(format!("{}.{ext}", app.project.export_stem(&name))),
        Path::to_path_buf,
    );
    let mut detail = String::new();
    let bytes = match audio {
        Some(kind) => {
            let sf = app.player.sound_font().ok_or(
                "audio export needs a SoundFont (piano.toml or ~/.config/piano/config.toml [sound])",
            )?;
            let native = piano_export::AudioFormat::native(sf);
            let format = piano_export::AudioFormat {
                rate: rate.unwrap_or(native.rate),
                bits: bits.unwrap_or(native.bits),
            };
            detail = format!(", {} Hz / {}-bit", format.rate, format.bits);
            let bytes = if kind == "flac" {
                piano_export::flac_bytes(&score, sf, format)
            } else {
                piano_export::wav_bytes(&score, sf, format)
            };
            bytes.map_err(|e| e.to_string())?
        }
        None => piano_export::midi_bytes(&score),
    };
    // Never overwrite: the same rule as outside the project in the editor.
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&out)
        .map_err(|e| format!("{}: {e}", out.display()))?;
    f.write_all(&bytes).map_err(|e| e.to_string())?;
    println!(
        "exported {} ({} KB{detail})",
        out.display(),
        bytes.len().div_ceil(1024)
    );
    Ok(())
}

fn doctor() -> Result<(), String> {
    println!("piano {}\n", env!("CARGO_PKG_VERSION"));

    println!("SoundFont");
    // The same order as the editor: piano.toml here, the global config, a search.
    let project = Project::open(Path::new("."))
        .map(|p| p.config)
        .unwrap_or_default();
    let (global, err) = piano_project::recent::load_global();
    if let Some(e) = err {
        println!("  global config: {e}");
    }
    let chosen = piano_app::app::choose_soundfont(&project, &global, true);
    match &chosen {
        Some((p, source)) => {
            let state = if p.is_file() { "" } else { "  (missing!)" };
            println!("  using {} ({}){state}", p.display(), source.label());
        }
        None => {
            println!("  none in:");
            for d in piano_audio::search_dirs() {
                println!("    {}", d.display());
            }
            println!("  → get a piano .sf2 (e.g. Salamander Grand Piano, FluidR3_GM),");
            println!("    then `font <path>` in the editor (saved to piano.toml).");
        }
    }

    println!("\nAudio device");
    let mut player = piano_audio::Player::new();
    match (player.device_info(), player.device_error()) {
        (Some(info), _) => println!("  ok: {info}"),
        (None, Some(e)) => {
            println!("  {e}");
            println!("  → on Linux, build with audio: install libasound2-dev, then");
            println!("    cargo build --release --features audio");
        }
        (None, None) => println!("  ok"),
    }

    let interactive = std::io::stdin().is_terminal();
    if player.device_error().is_none()
        && let Some((sf, _)) = &chosen
    {
        println!("\nSound check");
        match player.load_soundfont(sf) {
            Ok(()) => sound_check(&player, interactive),
            Err(e) => println!("  {e}"),
        }
    }
    drop(player);

    println!("\nTerminal");
    println!(
        "  keyboard enhancement: {}",
        if piano_tui::doctor::keyboard_enhancement() {
            "yes"
        } else {
            "no (Shift+F12 may arrive as F12; use F2-F4 to jump)"
        }
    );
    if !std::io::stdin().is_terminal() {
        println!("  not a terminal: key test skipped");
        return Ok(());
    }
    let results = piano_tui::doctor::key_test().map_err(|e| e.to_string())?;
    let missing: Vec<&str> = results
        .iter()
        .filter(|(_, ok)| !ok)
        .map(|(l, _)| *l)
        .collect();
    if missing.is_empty() {
        println!("  all keys ok");
    } else {
        println!("  not received: {}", missing.join(", "));
        println!("  → the terminal may use these keys itself (e.g. F1 help, F11 fullscreen,");
        println!("    F12 in drop-down terminals). Free them in the terminal's settings.");
    }
    Ok(())
}

/// Plays C-E-G through both paths (key preview and playback), shows the signal
/// level that reached the device, and asks whether it was heard.
fn sound_check(player: &piano_audio::Player, interactive: bool) {
    let pitches: Vec<piano_core::Pitch> = [60, 64, 67]
        .into_iter()
        .filter_map(piano_core::Pitch::new)
        .collect();
    let wait = |ms| std::thread::sleep(Duration::from_millis(ms));
    let mut results = Vec::new();

    // 1. Key preview (what the Keys focus uses).
    player.take_peak();
    for p in &pitches {
        player.preview(&[*p], 90);
        wait(350);
    }
    wait(600);
    results.push(("key preview (Keys focus)", player.take_peak().0));

    // 2. Playback (play / F5).
    let score = piano_lang::compile_piece("tempo 120 {C4 E G}/2").score;
    player.play(Arc::new(score), 0, None);
    while player.is_playing() {
        wait(50);
    }
    wait(600);
    results.push(("playback (play / F5)", player.take_peak().0));

    let mut silent = false;
    for (what, peak) in &results {
        let db = 20.0 * peak.max(1e-6).log10();
        print!("  {what:<26} signal {db:>6.1} dBFS ");
        if *peak < 0.001 {
            println!("— no signal reached the device");
            silent = true;
            continue;
        }
        if !interactive {
            println!();
            continue;
        }
        print!("— heard it? (y/n) ");
        let _ = std::io::stdout().flush();
        let mut answer = String::new();
        let _ = std::io::stdin().read_line(&mut answer);
        if !answer.trim().eq_ignore_ascii_case("y") {
            silent = true;
        }
    }
    if !interactive && !silent {
        println!(
            "  signal reached the device (run `piano doctor` in a terminal to confirm by ear)"
        );
    } else if silent {
        println!("  → check the system volume and the default output device");
        println!("    (e.g. `wpctl status` / `pavucontrol`); `aplay -L` lists ALSA devices.");
        println!("    A signal with nothing heard usually means the wrong output or muted volume.");
    } else {
        println!("  sound ok");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_of_scores_becomes_projects_and_skips_used_folders() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-tmp")
            .join(format!("cli-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (src, out) = (root.join("kern"), root.join("out"));
        std::fs::create_dir_all(&src).unwrap();
        let score = |notes: &str| {
            format!(
                "**kern\t**kern\n*staff2\t*staff1\n*M2/4\t*M2/4\n=1\t=1\n{notes}\n==\t==\n*-\t*-\n"
            )
        };
        std::fs::write(src.join("a.krn"), score("2C\t4c'\n.\t4d")).unwrap();
        std::fs::write(src.join("b.krn"), score("2E\t2g;")).unwrap();
        std::fs::write(src.join("broken.krn"), "**kern\n=1\n4q\n").unwrap();
        // A used folder is skipped; a broken score fails the command.
        std::fs::create_dir_all(out.join("b")).unwrap();
        std::fs::write(out.join("b/keep.txt"), "mine").unwrap();
        let err = import_folder(&src, &out).unwrap_err();
        assert!(err.contains("1 of 3 scores"), "{err}");
        let piece = std::fs::read_to_string(out.join("a/part1.piano")).unwrap();
        assert!(piece.contains("C4/4' D4"), "{piece}");
        assert!(out.join("a/song.piano").exists() && out.join("a/piano.toml").exists());
        assert_eq!(
            std::fs::read_to_string(out.join("b/keep.txt")).unwrap(),
            "mine"
        );
        assert!(!out.join("b/song.piano").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn folders_with_scores_are_found_below() {
        // Under the workspace `target/`, like the other crates' tests.
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-tmp")
            .join(format!("cli-folders-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (dir, file) in [
            ("kern", "a.krn"),
            ("kern", "b.krn"),
            ("docs", "x.pdf"),
            ("more/deep", "c.mxl"),
            (".git", "d.xml"),
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            std::fs::write(root.join(dir).join(file), "").unwrap();
        }
        let found: Vec<(String, usize)> = score_folders(&root, 3)
            .into_iter()
            .map(|(d, n)| (d.strip_prefix(&root).unwrap().display().to_string(), n))
            .collect();
        assert_eq!(
            found,
            [("kern".to_string(), 2), ("more/deep".to_string(), 1)]
        );
        let err = import_folder(&root, Path::new("out")).unwrap_err();
        assert!(err.contains("kern/ has 2 scores: piano import"), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn problems_list_alerts_then_missing_notes() {
        let c = piano_import::Check {
            alerts: vec!["⚠ part1 line 69: bar has 7 beats".into()],
            missing: vec![(2.5, 60)],
            ..Default::default()
        };
        assert_eq!(
            problems(&c),
            [
                "⚠ part1 line 69: bar has 7 beats",
                "missing: MIDI 60 at quarter 2.5"
            ]
        );
    }

    #[test]
    fn counts() {
        assert_eq!(count(1, "piece"), "1 piece");
        assert_eq!(count(0, "alert"), "0 alerts");
        assert_eq!(count(26, "tempo mark"), "26 tempo marks");
    }
}
