//! Tauri commands: the GUI's backend API.

use crate::{config, launcher, tasks, term};
use serde_json::{json, Map, Value};
use std::path::PathBuf;
use tauri::{Manager, State};

pub struct AppState {
    pub mod_root: PathBuf,
    pub mod_id: String,
    pub engine_root: PathBuf,
    pub justfile: PathBuf,
}

/// Locate the just runner: the bundled kristal-run sidecar (just compiled
/// in as a crate) — in the Tauri resource dir (released bundle) or next to
/// the current exe (dev) — else a system `just` on PATH as a fallback.
fn just_runner(app: &tauri::AppHandle) -> Option<(PathBuf, tasks::JustSource)> {
    // 1. released bundle: sidecar lands in the resource dir
    if let Ok(dir) = app.path().resource_dir() {
        for name in ["kristal-run", "kristal-run.exe"] {
            let p = dir.join(name);
            if p.is_file() {
                return Some((p, tasks::JustSource::Embedded));
            }
        }
    }
    // 2. dev: next to the current exe (cargo build produces both bins)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join(if cfg!(windows) { "kristal-run.exe" } else { "kristal-run" });
            if p.is_file() {
                return Some((p, tasks::JustSource::Embedded));
            }
        }
    }
    // 3. system just on PATH (dev fallback)
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|d| d.join(if cfg!(windows) { "just.exe" } else { "just" }))
            .find(|p| p.is_file())
            .map(|p| (p, tasks::JustSource::System))
    })
}

#[tauri::command]
pub fn status(app: tauri::AppHandle, state: State<AppState>) -> Value {
    let (engine_version, engine_hash) = config::engine_info(&state.engine_root);
    let (name, subtitle) = config::mod_name_subtitle(&state.mod_root);
    let love = launcher::lookup_love();
    let (just_path, just_source) = just_runner(&app)
        .map(|(p, s)| (p, Some(s)))
        .unwrap_or((PathBuf::new(), None));
    json!({
        "modRoot": state.mod_root,
        "modID": state.mod_id,
        "engineRoot": state.engine_root,
        "engine": { "version": engine_version, "hash": engine_hash },
        "love": { "found": love.is_some(), "path": love.map(|p| p.to_string_lossy().into_owned()).unwrap_or_default() },
        "just": {
            "found": !just_path.as_os_str().is_empty(),
            "path": just_path.to_string_lossy().into_owned(),
            "mode": match just_source { Some(tasks::JustSource::Embedded) => "embedded", Some(tasks::JustSource::System) => "system", None => "none" },
        },
        "project": { "id": state.mod_id, "name": name, "subtitle": subtitle },
        "libraries": config::libraries(&state.mod_root),
        "template": config::detect_template(&state.mod_root),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    })
}

#[tauri::command]
pub fn tasks(app: tauri::AppHandle, state: State<AppState>, lang: Option<String>) -> Value {
    let lang = lang.as_deref().unwrap_or("default");
    match just_runner(&app) {
        Some((jp, source)) => tasks::list(source, &jp, &state.justfile, &state.mod_root, lang),
        None => json!({ "source": "builtin", "tasks": [], "mod": null }),
    }
}

#[derive(serde::Deserialize)]
pub struct RunTaskArgs {
    pub task: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub justfile: String, // "" | "library" | "project"
    #[serde(default)]
    pub pause: bool, // keep the terminal open after the task finishes
}

#[tauri::command]
pub fn run_task(app: tauri::AppHandle, state: State<AppState>, req: RunTaskArgs) -> Result<Value, String> {
    let (runner, _) = just_runner(&app).ok_or_else(|| {
        "just runner unavailable (neither the bundled sidecar nor a system just was found)".to_string()
    })?;
    let justfile = if req.justfile == "project" {
        let p = state.mod_root.join("justfile");
        if !p.is_file() {
            return Err("project justfile not found".to_string());
        }
        p
    } else {
        state.justfile.clone()
    };
    // The terminal runs the sidecar, which executes `just` in-process with
    // inherited stdio — the terminal stays interactive.
    let mut argv = vec![
        runner.to_string_lossy().into_owned(),
        "just-task".into(),
        justfile.to_string_lossy().into_owned(),
        req.task,
    ];
    argv.extend(req.args);
    term::spawn_in_terminal(&argv, &state.mod_root, req.pause)?;
    Ok(json!({ "ok": true }))
}

#[derive(serde::Deserialize)]
pub struct LaunchArgs {
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub encounter: Option<String>,
    #[serde(default)]
    pub wave: Option<String>,
    #[serde(default)]
    pub wave_force: Option<String>,
    #[serde(default)]
    pub tp: Option<String>,
    #[serde(default)]
    pub mercy: Option<String>,
    #[serde(default)]
    pub passthrough: Vec<String>,
}

#[tauri::command]
pub fn launch_game(state: State<AppState>, req: LaunchArgs) -> Result<Value, String> {
    let opts = launcher::LaunchOptions {
        lang: req.lang,
        encounter: req.encounter,
        wave: req.wave,
        wave_force: req.wave_force,
        tp: req.tp,
        mercy: req.mercy,
        passthrough: req.passthrough,
    };
    let args = launcher::build_argv(&opts)?;
    let love = launcher::lookup_love().ok_or_else(|| {
        "love executable not found on PATH. Install LÖVE (https://love2d.org) or add its install directory to PATH.".to_string()
    })?;
    if !state.engine_root.join("main.lua").is_file() {
        return Err(format!(
            "Kristal engine main.lua not found: {}/main.lua",
            state.engine_root.display()
        ));
    }
    let mut argv = vec![
        love.to_string_lossy().into_owned(),
        state.engine_root.to_string_lossy().into_owned(),
        "--mod".into(),
        state.mod_id.clone(),
        "--auto-mod-start".into(),
    ];
    argv.extend(args);
    term::spawn_in_terminal(&argv, &state.engine_root, false)?;
    Ok(json!({ "ok": true }))
}

#[tauri::command]
pub fn chapter_config(state: State<AppState>) -> Value {
    let defaults = config::chapter_defaults(&state.engine_root);
    let overrides = config::config_overrides(&state.mod_root);
    let descs = tasks::config_feature_descs();

    let mut keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for m in &defaults {
        keys.extend(m.keys().cloned());
    }
    keys.extend(descs.keys().cloned());

    let items: Vec<Value> = keys
        .into_iter()
        .map(|k| {
            let values: Map<String, Value> = defaults
                .iter()
                .enumerate()
                .filter_map(|(i, m)| m.get(&k).map(|v| ((i + 1).to_string(), v.clone())))
                .collect();
            json!({
                "key": k,
                "desc": descs.get(&k).cloned().unwrap_or(Value::Null),
                "values": values,
                "override": overrides.get(&k).cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    json!({ "chapter": config::current_chapter(&state.mod_root), "items": items })
}

#[derive(serde::Deserialize)]
pub struct ChapterConfigSetArgs {
    pub key: String,
    pub value: Option<Value>,
}

#[tauri::command]
pub fn chapter_config_set(state: State<AppState>, req: ChapterConfigSetArgs) -> Result<Value, String> {
    if req.key.is_empty()
        || !req
            .key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        || req.key.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true)
    {
        return Err("invalid config key".into());
    }
    config::mod_config_set(&state.mod_root, &req.key, req.value)?;
    Ok(json!({ "ok": true }))
}

#[tauri::command]
pub fn template_chapter(state: State<AppState>, chapter: i64) -> Result<Value, String> {
    if !(1..=4).contains(&chapter) {
        return Err("chapter must be 1-4".into());
    }
    let path = state.mod_root.join("mod.json");
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let re = regex::Regex::new(r#"("chapter"\s*:\s*)[0-9]+"#).unwrap();
    if !re.is_match(&text) {
        return Err("chapter field not found in mod.json".into());
    }
    let out = re.replace(&text, format!("${{1}}{}", chapter)).to_string();
    std::fs::write(&path, out).map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true }))
}

#[derive(serde::Deserialize)]
pub struct TemplateInitArgs {
    pub name: String,
}

#[tauri::command]
pub fn template_init(state: State<AppState>, req: TemplateInitArgs) -> Result<Value, String> {
    let valid = !req.name.is_empty()
        && req.name.chars().count() <= 64
        && req
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == ' ' || c == '_' || c == '-');
    if !valid {
        return Err("invalid project name (letters, digits, space, dash, underscore; max 64)".into());
    }
    if config::detect_template(&state.mod_root).is_none() {
        return Err("not a thrash-machine template".into());
    }
    let argv = vec![
        "bash".into(),
        state.mod_root.join("start.sh").to_string_lossy().into_owned(),
        "--name".into(),
        req.name,
    ];
    term::spawn_in_terminal(&argv, &state.mod_root, true)?;
    Ok(json!({ "ok": true }))
}

/// Run the engine version + hash probe (used by the status bar).
#[tauri::command]
pub fn engine_info_command(state: State<AppState>) -> Value {
    let (v, h) = config::engine_info(&state.engine_root);
    json!({ "version": v, "hash": h })
}

// silence unused warning for Command import in non-listed paths
#[allow(unused)]
fn _unused(_: &PathBuf) {}
