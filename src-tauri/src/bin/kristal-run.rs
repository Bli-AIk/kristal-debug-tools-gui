//! kristal-run — CLI sidecar shipped next to the GUI binary (console
//! subsystem, so it works inside interactive terminal windows on Windows).
//!
//! Modes (first argument):
//! - `just-task <justfile> <task> [args...]` — run a just recipe; stdio is
//!   inherited so the terminal stays interactive. `just` is a Rust crate
//!   compiled into this binary — no just.exe distribution needed.
//! - `just-dump <justfile>` — print `just --dump --format json` to stdout
//!   (the GUI captures this via Command::output).
//! - launcher flags (`-wf`, `--lang`, …) — start the game through love,
//!   mirroring bin/kristal-run in the library.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use kristal_debug_tools_gui_lib::launcher;

const USAGE: &str = "\
kristal-run — Kristal mod debug launcher

Usage:
  kristal-run [OPTIONS] [-- ARGS...]      start the game (love)
  kristal-run just-task <justfile> <task> [args...]
  kristal-run just-dump <justfile>

Options:
  -l, --lang <code>          game language (e.g. zh-hans)
  -e, --encounter <id>       encounter id
  -w, --wave <n|id>          wave number or id
  -wf, --wave-force <n|id>   force a wave (skips earlier waves)
  -tp, --tp <0-100>          initial TP
  -m, --mercy <0-100>        initial mercy
  -h, --help                 show this help
  --                         everything after is passed to the game

Environment:
  KDT_MOD_ROOT     mod root (default: walk up from the current directory)
  KRISTAL_ROOT     Kristal engine root (default: walk up from the mod root)
  KRISTAL_DEBUG_TOOLS_DRY_RUN=1   print the love command instead of running it
";

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    match argv.get(1).map(|s| s.as_str()) {
        Some("just-task") => {
            let (Some(jf), Some(task)) = (argv.get(2), argv.get(3)) else {
                eprintln!("usage: kristal-run just-task <justfile> <task> [args...]");
                return ExitCode::from(64);
            };
            let mut args: Vec<OsString> = vec![
                "just".into(),
                "--justfile".into(),
                jf.into(),
                task.into(),
            ];
            args.extend(argv[4..].iter().map(OsString::from));
            just_run(args)
        }
        Some("just-dump") => {
            let Some(jf) = argv.get(2) else {
                eprintln!("usage: kristal-run just-dump <justfile>");
                return ExitCode::from(64);
            };
            just_run(vec![
                "just".into(),
                "--justfile".into(),
                jf.into(),
                "--dump".into(),
                "--dump-format".into(),
                "json".into(),
            ])
        }
        Some("--help") | Some("-h") | None => {
            print!("{}", USAGE);
            ExitCode::SUCCESS
        }
        _ => launch(&argv[1..]),
    }
}

/// just::run writes to the process-wide stdout/stderr — exactly what the
/// terminal / pipe caller wants. Exit with just's own code.
fn just_run(args: Vec<OsString>) -> ExitCode {
    match just::run(args.into_iter()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => ExitCode::from(code as u8),
    }
}

/// Game-launcher mode: parse flags, resolve mod/engine, exec love.
fn launch(args: &[String]) -> ExitCode {
    let passthrough = match launcher::parse_args(args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("kristal-run: {}", e);
            print!("{}", USAGE);
            return ExitCode::from(64);
        }
    };
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mod_env = std::env::var("KDT_MOD_ROOT").ok();
    let kristal_env = std::env::var("KRISTAL_ROOT").ok();
    let resolved = match launcher::resolve(&cwd, mod_env.as_deref(), kristal_env.as_deref()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("kristal-run: {}", e);
            return ExitCode::from(1);
        }
    };
    let love = match launcher::lookup_love() {
        Some(p) => p,
        None => {
            eprintln!(
                "kristal-run: love not found on PATH (install LÖVE or add its directory to PATH)"
            );
            return ExitCode::from(1);
        }
    };
    if !resolved.engine_root.join("main.lua").is_file() {
        eprintln!(
            "kristal-run: engine main.lua not found: {}",
            resolved.engine_root.display()
        );
        return ExitCode::from(1);
    }

    let mut cmdline = vec![
        love.to_string_lossy().into_owned(),
        resolved.engine_root.to_string_lossy().into_owned(),
        "--mod".into(),
        resolved.mod_id.clone(),
        "--auto-mod-start".into(),
    ];
    cmdline.extend(passthrough.clone());

    if std::env::var_os("KRISTAL_DEBUG_TOOLS_DRY_RUN").is_some() {
        println!("{}", cmdline.join(" "));
        return ExitCode::SUCCESS;
    }

    let mut cmd = std::process::Command::new(&love);
    cmd.arg(&resolved.engine_root)
        .arg("--mod")
        .arg(&resolved.mod_id)
        .arg("--auto-mod-start")
        .args(&passthrough)
        .current_dir(&resolved.engine_root);
    match cmd.status() {
        Ok(status) => status.code().map(|c| ExitCode::from(c as u8)).unwrap_or(ExitCode::FAILURE),
        Err(e) => {
            eprintln!("kristal-run: failed to start love: {}", e);
            ExitCode::FAILURE
        }
    }
}
