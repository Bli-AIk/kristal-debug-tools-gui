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
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kristal_debug_tools_gui_lib::launcher;

/// Justfile recipes default to shell `sh`; on a stock Windows box no sh is on
/// PATH, so every recipe dies with "could not find the shell `sh`". System Git
/// ships a POSIX stack (bin\sh.exe, usr\bin\{sh,bash,rm,unzip,tar,grep}.exe),
/// so prepend those directories to PATH before just runs. When the user has no
/// Git at all, fall back to a project-local PortableGit downloaded on demand.
fn prepend_git_bash_to_path() {
    #[cfg(windows)]
    {
        let mut dirs: Vec<PathBuf> = Vec::new();
        for var in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(p) = std::env::var(var).ok() {
                dirs.extend(git_bash_dirs(&PathBuf::from(p).join("Git")));
            }
        }
        if let Some(p) = std::env::var("LOCALAPPDATA").ok() {
            dirs.extend(git_bash_dirs(&PathBuf::from(p).join("Programs").join("Git")));
        }
        if dirs.is_empty() {
            // No system Git: only then download a PortableGit into the shared
            // tools dir next to the Kristal engine (<engine-root>/.tools/portablegit,
            // same .tools convention as the build scripts) so recipes still get
            // sh/bash. The engine root is resolved the same way the launcher
            // does it (walk up from the mod root), falling back to the cwd.
            let cwd = std::env::current_dir().unwrap_or_default();
            let engine_root = launcher::find_engine(&cwd).unwrap_or(cwd);
            let portable = engine_root.join(".tools").join("portablegit");
            if !git_bash_dirs(&portable).is_empty() || ensure_portable_git(&portable) {
                dirs.extend(git_bash_dirs(&portable));
            }
        }
        if !dirs.is_empty() {
            let prefix = dirs
                .iter()
                .map(|d| d.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(";");
            let old = std::env::var("PATH").unwrap_or_default();
            std::env::set_var("PATH", format!("{prefix};{old}"));
        }
    }
    // No-op on non-Windows: sh is present everywhere else.
}

/// `[<root>/bin, <root>/usr/bin]` when <root> is a usable Git Bash install (has
/// bin\sh.exe); empty otherwise. Written platform-neutral so the Windows logic
/// still gets type-checked on Linux; the `.exe` probe just never matches there.
#[cfg_attr(not(windows), allow(dead_code))]
fn git_bash_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let bin = root.join("bin");
    let usrbin = root.join("usr").join("bin");
    if bin.join("sh.exe").is_file() {
        dirs.push(bin);
        if usrbin.join("sh.exe").is_file() {
            dirs.push(usrbin);
        }
    }
    dirs
}

/// Download and self-extract a PortableGit (7z SFX from git-for-windows) into
/// `root` so just recipes get sh/bash without any system Git. Uses only Windows
/// built-ins — PowerShell (always present) does the GitHub API lookup, download
/// and extraction, so no Rust HTTP/archive dependencies are needed. Returns
/// false on any failure; PATH is simply left alone and just reports its usual
/// "could not find the shell `sh`".
#[cfg_attr(not(windows), allow(dead_code))]
fn ensure_portable_git(root: &Path) -> bool {
    if !git_bash_dirs(root).is_empty() {
        return true;
    }
    let Some(tmpdir) = std::env::var_os("TEMP")
        .or_else(|| std::env::var_os("TMP"))
        .map(PathBuf::from)
    else {
        return false;
    };
    let sfx = tmpdir.join("kristal-run-PortableGit.7z.exe");
    let script = tmpdir.join("kristal-run-fetch-portablegit.ps1");
    let _ = std::fs::write(&script, PORTABLE_GIT_PS);
    let ok = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .arg(&root)
        .arg(&sfx)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = std::fs::remove_file(&script);
    if !ok {
        let _ = std::fs::remove_file(&sfx);
        return false;
    }
    !git_bash_dirs(root).is_empty()
}

const PORTABLE_GIT_PS: &str = r#"
param([string]$Root, [string]$Sfx)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
try {
  $rel = Invoke-RestMethod -Uri 'https://api.github.com/repos/git-for-windows/git/releases/latest' -Headers @{ 'User-Agent' = 'kristal-run' }
  $asset = @($rel.assets | Where-Object { $_.name -match '^PortableGit-.*-64-bit\.7z\.exe$' })[0]
  if (-not $asset) { exit 2 }
  Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $Sfx
  New-Item -ItemType Directory -Force -Path $Root | Out-Null
  $p = Start-Process -FilePath $Sfx -ArgumentList @('-y', ('-o"' + $Root + '"')) -Wait -PassThru -WindowStyle Hidden
  exit $p.ExitCode
} catch {
  Write-Host $_
  exit 3
}
"#;

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
            prepend_git_bash_to_path();
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
