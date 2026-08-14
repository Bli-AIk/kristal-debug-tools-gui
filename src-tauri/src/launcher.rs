//! Rust port of bin/kristal-run: flag parsing, mod/engine resolution and
//! love invocation. Kept behavior-identical to the bash original (the
//! library's bin/kristal-run is the source of truth).

use std::path::{Path, PathBuf};

/// Mirrors the bash launcher's flag surface.
#[derive(Default, Clone)]
pub struct LaunchOptions {
    pub lang: Option<String>,
    pub encounter: Option<String>,
    pub wave: Option<String>,
    pub wave_force: Option<String>,
    pub tp: Option<String>,
    pub mercy: Option<String>,
    pub passthrough: Vec<String>,
}

fn required(argv: &[String], i: usize, flag: &str) -> Result<String, String> {
    argv.get(i + 1)
        .cloned()
        .ok_or_else(|| format!("{} requires a value.", flag))
}

/// Parse launcher flags into the final kristal args. Mirrors the FIXED
/// bin/kristal-run: -wf / -wfX reach the wave-force cases before the -w
/// prefix case (the bash shadowing bug was fixed in the library).
pub fn parse_args(argv: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < argv.len() {
        let a = argv[i].clone();
        if a == "--help" || a == "-h" {
            return Err("help".into());
        }
        if a == "--" {
            out.extend_from_slice(&argv[i + 1..]);
            return Ok(out);
        }
        if let Some(v) = a.strip_prefix("--encounter=") {
            if v.is_empty() {
                out.push("--encounter".into());
            } else {
                out.extend(["--encounter".into(), v.into()]);
            }
        } else if a == "--encounter" || a == "-e" {
            if i + 1 < argv.len() && !argv[i + 1].starts_with('-') {
                out.extend(["--encounter".into(), argv[i + 1].clone()]);
                i += 1;
            } else {
                out.push("--encounter".into());
            }
        } else if let Some(v) = a.strip_prefix("-e") {
            out.extend(["--encounter".into(), v.into()]);
        } else if a.starts_with("--lang=") || a.starts_with("--language=") {
            let v = a.split_once('=').unwrap().1;
            if v.is_empty() {
                return Err("--lang requires a value.".into());
            }
            out.extend(["--lang".into(), v.into()]);
        } else if a == "--lang" || a == "--language" || a == "-l" {
            let v = required(argv, i, &a)?;
            out.extend(["--lang".into(), v]);
            i += 1;
        } else if let Some(v) = a.strip_prefix("-l") {
            out.extend(["--lang".into(), v.into()]);
        } else if let Some(v) = a.strip_prefix("--wave=") {
            if v.is_empty() {
                return Err("--wave requires a value.".into());
            }
            out.extend(["--wave".into(), v.into()]);
        } else if let Some(v) = a.strip_prefix("--wave-force=") {
            if v.is_empty() {
                return Err("--wave-force requires a value.".into());
            }
            out.extend(["--wave-force".into(), v.into()]);
        } else if a == "--wave-force" || a == "-wf" {
            let v = required(argv, i, &a)?;
            out.extend(["--wave-force".into(), v]);
            i += 1;
        } else if let Some(v) = a.strip_prefix("-wf") {
            out.extend(["--wave-force".into(), v.into()]);
        } else if a == "--wave" || a == "-w" {
            let v = required(argv, i, &a)?;
            out.extend(["--wave".into(), v]);
            i += 1;
        } else if let Some(v) = a.strip_prefix("-w") {
            out.extend(["--wave".into(), v.into()]);
        } else if a.starts_with("--initial-tp=") || a.starts_with("--tp=") {
            let v = a.split_once('=').unwrap().1;
            if v.is_empty() {
                return Err("--tp requires a value.".into());
            }
            out.extend(["--tp".into(), v.into()]);
        } else if a == "--initial-tp" || a == "--tp" || a == "-tp" {
            let v = required(argv, i, &a)?;
            out.extend(["--tp".into(), v]);
            i += 1;
        } else if let Some(v) = a.strip_prefix("-tp") {
            out.extend(["--tp".into(), v.into()]);
        } else if a.starts_with("--initial-mercy=") || a.starts_with("--mercy=") {
            let v = a.split_once('=').unwrap().1;
            if v.is_empty() {
                return Err("--mercy requires a value.".into());
            }
            out.extend(["--mercy".into(), v.into()]);
        } else if a == "--initial-mercy" || a == "--mercy" || a == "-m" {
            let v = required(argv, i, &a)?;
            out.extend(["--mercy".into(), v]);
            i += 1;
        } else if let Some(v) = a.strip_prefix("-m") {
            out.extend(["--mercy".into(), v.into()]);
        } else if a.starts_with('-') {
            return Err(format!("unknown launcher option: {}", a));
        } else {
            out.push(a);
        }
        i += 1;
    }
    Ok(out)
}

/// Build the launcher argv from the structured form.
pub fn build_argv(opts: &LaunchOptions) -> Result<Vec<String>, String> {
    let mut argv = Vec::new();
    if let Some(v) = &opts.lang {
        argv.extend(["--lang".into(), v.clone()]);
    }
    if let Some(v) = &opts.encounter {
        argv.extend(["--encounter".into(), v.clone()]);
    }
    if let Some(v) = &opts.wave {
        argv.extend(["--wave".into(), v.clone()]);
    }
    if let Some(v) = &opts.wave_force {
        argv.extend(["--wave-force".into(), v.clone()]);
    }
    if let Some(v) = &opts.tp {
        argv.extend(["--tp".into(), v.clone()]);
    }
    if let Some(v) = &opts.mercy {
        argv.extend(["--mercy".into(), v.clone()]);
    }
    if !opts.passthrough.is_empty() {
        argv.push("--".into());
        argv.extend(opts.passthrough.clone());
    }
    parse_args(&argv)
}

#[derive(Clone)]
pub struct Resolved {
    pub mod_root: PathBuf,
    pub mod_id: String,
    pub engine_root: PathBuf,
}

/// Resolve the mod root and engine, mirroring bin/kristal-run.
pub fn resolve(
    cwd: &Path,
    mod_root_env: Option<&str>,
    kristal_root_env: Option<&str>,
) -> Result<Resolved, String> {
    let cwd = cwd
        .canonicalize()
        .map_err(|e| format!("Could not resolve current directory: {}", e))?;
    let mod_root = match mod_root_env {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => find_mod_root(&cwd)
            .ok_or_else(|| "Could not find mod.json. Run this command from a Kristal project or set KRISTAL_MOD_ROOT.".to_string())?,
    };
    let mod_root = mod_root
        .canonicalize()
        .map_err(|e| format!("Could not resolve mod root: {}", e))?;
    let mod_id = mod_id(&mod_root);

    let engine_root = find_engine(&mod_root).or_else(|| {
        kristal_root_env
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
    });
    let engine_root = engine_root
        .ok_or_else(|| "Kristal engine not found. Set KRISTAL_ROOT=/path/to/Kristal.".to_string())?;
    Ok(Resolved {
        mod_root,
        mod_id,
        engine_root,
    })
}

fn find_mod_root(dir: &Path) -> Option<PathBuf> {
    let mut cur = dir.to_path_buf();
    loop {
        if cur.join("mod.json").is_file() {
            return Some(cur);
        }
        if !cur.pop() {
            return None;
        }
    }
}

fn find_engine(dir: &Path) -> Option<PathBuf> {
    let mut cur = dir.to_path_buf();
    loop {
        if cur.join("main.lua").is_file() && cur.join("src").join("kristal.lua").is_file() {
            return Some(cur);
        }
        if !cur.pop() {
            return None;
        }
    }
}

fn mod_id(mod_root: &Path) -> String {
    if let Ok(text) = std::fs::read_to_string(mod_root.join("mod.json")) {
        // First "id": "..." in the (JSONC) text — same probe as the bash
        // launcher's sed.
        for line in text.lines() {
            let line = line.trim_start();
            if let Some(rest) = line.strip_prefix("\"id\"") {
                let rest = rest.trim_start();
                if let Some(rest) = rest.strip_prefix(':') {
                    let rest = rest.trim_start();
                    if let Some(rest) = rest.strip_prefix('"') {
                        if let Some(id) = rest.split('"').next() {
                            if !id.is_empty() {
                                return id.to_string();
                            }
                        }
                    }
                }
            }
        }
    }
    mod_root
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Find the love executable: PATH first, then Windows install locations.
/// On Windows prefer the console build (lovec.exe) so the terminal-cli debug
/// console (stdin/stdout) can attach; love.exe is a GUI-subsystem binary that
/// never gets a console.
pub fn lookup_love() -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["lovec.exe", "love.exe"]
    } else {
        &["love"]
    };
    for name in names {
        if let Some(p) = which_love(name) {
            return Some(p);
        }
        if cfg!(windows) {
            for dir in [
                std::env::var("ProgramFiles").unwrap_or_default(),
                std::env::var("LOCALAPPDATA").unwrap_or_default(),
            ] {
                let p = PathBuf::from(&dir).join("LOVE").join(name);
                if p.is_file() {
                    return Some(p);
                }
                let p = PathBuf::from(&dir).join("Programs").join("LOVE").join(name);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn which_love(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
