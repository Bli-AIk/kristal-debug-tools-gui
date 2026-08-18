//! Tauri commands: the GUI's backend API.

use crate::{config, launcher, tasks, term};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tauri::{Manager, State};

pub struct AppState {
    pub mod_root: PathBuf,
    pub mod_id: String,
    pub engine_root: PathBuf,
    pub justfile: PathBuf,
}

/// Candidate sidecar file names, in order: the Tauri externalBin name (dev
/// build / NSIS installer), then the raw release binary's platform+arch
/// suffixed name.
fn sidecar_candidates() -> Vec<String> {
    let exe = if cfg!(windows) { ".exe" } else { "" };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    vec![
        format!("kristal-run{}", exe),
        format!("kristal-run-{}-{}{}", std::env::consts::OS, arch, exe),
    ]
}

/// Look for the bundled kristal-run in `dir`: exact candidate names first,
/// then any `kristal-run*` file (raw release names drift as the build
/// matrix grows).
fn find_sidecar(dir: &Path) -> Option<PathBuf> {
    for name in sidecar_candidates() {
        let p = dir.join(&name);
        if p.is_file() {
            return Some(p);
        }
    }
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.is_file()
                && p.file_name().and_then(|n| n.to_str()).map_or(false, |n| {
                    n.starts_with("kristal-run") && !n.ends_with(".tmp")
                })
        })
}

/// Locate the just runner: the bundled kristal-run sidecar (just compiled
/// in as a crate) — in the Tauri resource dir (released bundle) or next to
/// the current exe (dev) — else a system `just` on PATH as a fallback.
fn just_runner(app: &tauri::AppHandle) -> Option<(PathBuf, tasks::JustSource)> {
    // 1. released bundle: sidecar lands in the resource dir
    if let Ok(dir) = app.path().resource_dir() {
        if let Some(p) = find_sidecar(&dir) {
            return Some((p, tasks::JustSource::Embedded));
        }
    }
    // 2. dev: next to the current exe (cargo build produces both bins)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if let Some(p) = find_sidecar(dir) {
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

/// All GUI settings live in one JSON file, shared with the launcher
/// scripts: <mod-root>/.tools/gui/settings.json
/// { lang, scale, keepOpen }
fn settings_file(state: &AppState) -> PathBuf {
    state
        .mod_root
        .join(".tools")
        .join("gui")
        .join("settings.json")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChapterConfigLanguage {
    Chinese,
    English,
}

fn chapter_config_language(lang: Option<&str>) -> ChapterConfigLanguage {
    let Some(lang) = lang else {
        return ChapterConfigLanguage::Chinese;
    };
    if ["zh", "zh-hans", "zh_hans", "zh-cn", "zh_cn", "zh-hant", "zh_hant", "zh-tw", "zh_tw"]
        .iter()
        .any(|candidate| lang.eq_ignore_ascii_case(candidate))
    {
        ChapterConfigLanguage::Chinese
    } else {
        ChapterConfigLanguage::English
    }
}

/// Label text without JSON quoting: "Money" -> Money. Boolean and null
/// values use the UI language rather than leaking the default Chinese labels.
fn label_str(v: &Value, language: ChapterConfigLanguage) -> String {
    match v {
        Value::Bool(true) => match language {
            ChapterConfigLanguage::Chinese => "是".to_string(),
            ChapterConfigLanguage::English => "Yes".to_string(),
        },
        Value::Bool(false) => match language {
            ChapterConfigLanguage::Chinese => "否".to_string(),
            ChapterConfigLanguage::English => "No".to_string(),
        },
        Value::Null => match language {
            ChapterConfigLanguage::Chinese => "未设置".to_string(),
            ChapterConfigLanguage::English => "Unset".to_string(),
        },
        Value::String(s) => s.to_string(),
        other => other.to_string(),
    }
}

/// Catalog chapter labels are Chinese-only today. Preserve their richer
/// meaning in Chinese; use the engine value in English unless the catalog
/// label represents one of the shared boolean/unset semantics.
fn label_from_feature(feature_label: &str, raw: &Value, language: ChapterConfigLanguage) -> String {
    let label = feature_label.trim_matches('\"');
    match language {
        ChapterConfigLanguage::Chinese => label.to_string(),
        ChapterConfigLanguage::English => match label {
            "是" => "Yes".to_string(),
            "否" => "No".to_string(),
            "未设置" => "Unset".to_string(),
            _ => label_str(raw, language),
        },
    }
}

fn read_settings(state: &AppState) -> Value {
    std::fs::read_to_string(settings_file(state))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}))
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
        "capabilities": { "i18n": config::has_kristal_i18n(&state.mod_root) },
        "template": config::detect_template(&state.mod_root),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "settings": read_settings(&state),
    })
}

/// Merge a partial settings patch into .tools/gui/settings.json — the
/// single file shared with gui.cmd / gui-download.sh.
#[tauri::command]
pub fn set_settings(state: State<AppState>, patch: Value) -> Result<Value, String> {
    let file = settings_file(&state);
    let dir = file.parent().ok_or("bad path")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut cur = read_settings(&state);
    if let (Some(obj), Some(p)) = (cur.as_object_mut(), patch.as_object()) {
        for (k, v) in p {
            obj.insert(k.clone(), v.clone());
        }
    }
    let text = serde_json::to_string_pretty(&cur).map_err(|e| e.to_string())?;
    std::fs::write(&file, text).map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "settings": cur }))
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
pub fn run_task(
    app: tauri::AppHandle,
    state: State<AppState>,
    req: RunTaskArgs,
) -> Result<Value, String> {
    let (runner, _) = just_runner(&app).ok_or_else(|| {
        "just runner unavailable (neither the bundled sidecar nor a system just was found)"
            .to_string()
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

fn validate_launch_language(mod_root: &Path, lang: Option<&str>) -> Result<(), String> {
    if lang.is_some() && !config::has_kristal_i18n(mod_root) {
        return Err(
            "the --lang option requires the kristalI18n library in this project".to_string(),
        );
    }
    Ok(())
}

#[tauri::command]
pub fn launch_game(state: State<AppState>, req: LaunchArgs) -> Result<Value, String> {
    validate_launch_language(&state.mod_root, req.lang.as_deref())?;
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
pub fn chapter_config(state: State<AppState>, lang: Option<String>) -> Value {
    let defaults = config::chapter_defaults(&state.engine_root);
    let overrides = config::config_overrides(&state.mod_root);
    let features = tasks::config_feature_rows();
    let chapter = config::current_chapter(&state.mod_root);
    chapter_config_view_with_language(
        &defaults,
        &overrides,
        &features,
        chapter,
        chapter_config_language(lang.as_deref()),
    )
}

/// Build the chapter-config view. The selectable key set and available
/// chapters come from the engine's `configs/chapterN.json` files;
/// config-features.json only supplies copy and extra option labels.
#[cfg(test)]
fn chapter_config_view(
    defaults: &BTreeMap<i64, Map<String, Value>>,
    overrides: &Map<String, Value>,
    features: &BTreeMap<String, BTreeMap<String, Value>>,
    chapter: i64,
) -> Value {
    chapter_config_view_with_language(
        defaults,
        overrides,
        features,
        chapter,
        ChapterConfigLanguage::Chinese,
    )
}

fn chapter_config_view_with_language(
    defaults: &BTreeMap<i64, Map<String, Value>>,
    overrides: &Map<String, Value>,
    features: &BTreeMap<String, BTreeMap<String, Value>>,
    chapter: i64,
    language: ChapterConfigLanguage,
) -> Value {
    let chapters: Vec<i64> = defaults.keys().copied().collect();

    let mut keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for m in defaults.values() {
        keys.extend(m.keys().cloned());
    }

    let items: Vec<Value> = keys
        .into_iter()
        .map(|k| {
            let frow = features.get(&k);
            let mut options: Vec<(String, Value)> = Vec::new();
            let label_for = |raw: &Value| -> String {
                // Match labels by raw value instead of carrying an old
                // chapter's label into a newer chapter with a new default.
                for (ch, values) in defaults {
                    if values.get(&k) == Some(raw) {
                        if let Some(label) = frow
                            .and_then(|f| f.get(&ch.to_string()))
                            .and_then(|v| v.as_str())
                        {
                            return label_from_feature(label, raw, language);
                        }
                    }
                }
                label_str(raw, language)
            };

            // All native chapter defaults are valid choices. Add the menu's
            // declared alternatives too, because several defaults happen to
            // be identical across every chapter.
            for values in defaults.values() {
                if let Some(raw) = values.get(&k) {
                    if !options.iter().any(|(_, value)| value == raw) {
                        options.push((label_for(raw), raw.clone()));
                    }
                }
            }
            if let Some(opts) = frow.and_then(|f| f.get("opts")).and_then(|v| v.as_array()) {
                for value in opts {
                    if !options.iter().any(|(_, existing)| existing == value) {
                        options.push((label_for(value), value.clone()));
                    }
                }
            }

            // Keep an existing non-standard override selectable and visible
            // instead of making a select control look blank.
            if let Some(value) = overrides.get(&k) {
                if !options.iter().any(|(_, existing)| existing == value) {
                    options.push((label_for(value), value.clone()));
                }
            }

            let (current, is_override) = match overrides.get(&k) {
                Some(ov) => {
                    let label = options
                        .iter()
                        .find(|(_, r)| r == ov)
                        .map(|(l, _)| l.clone())
                        .unwrap_or_else(|| label_str(ov, language));
                    (json!({ "label": label, "value": ov.clone() }), true)
                }
                None => {
                    let raw = defaults
                        .get(&chapter)
                        .and_then(|m| m.get(&k))
                        .cloned()
                        .unwrap_or(Value::Null);
                    let label = label_for(&raw);
                    (json!({ "label": label, "value": raw }), false)
                }
            };

            let ch_values: Map<String, Value> = defaults
                .iter()
                .map(|(ch, values)| {
                    let raw = values
                        .get(&k)
                        .cloned()
                        .unwrap_or(Value::Null);
                    let label = label_for(&raw);
                    (ch.to_string(), json!({ "label": label, "value": raw }))
                })
                .collect();
            json!({
                "key": k,
                "name": frow.and_then(|f| f.get("name")).cloned().unwrap_or(Value::String(k.clone())),
                "desc": frow.and_then(|f| f.get("desc")).cloned().unwrap_or(Value::Null),
                "descEn": frow.and_then(|f| f.get("descEn")).cloned().unwrap_or(Value::Null),
                "options": options.into_iter().map(|(l, v)| json!({ "label": l, "value": v })).collect::<Vec<_>>(),
                "current": current,
                "chValues": ch_values,
                "isOverride": is_override,
                "standard": true,
            })
        })
        .collect();

    json!({ "chapter": chapter, "chapters": chapters, "items": items })
}

#[derive(serde::Deserialize)]
pub struct ChapterConfigSaveArgs {
    pub chapter: i64,
    #[serde(default)]
    pub changes: BTreeMap<String, Option<Value>>,
}

fn valid_config_key(key: &str) -> bool {
    !key.is_empty()
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !key
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(true)
}

fn unknown_change_keys(
    defaults: &BTreeMap<i64, Map<String, Value>>,
    changes: &BTreeMap<String, Option<Value>>,
) -> Vec<String> {
    let valid: BTreeSet<String> = defaults.values().flat_map(|m| m.keys().cloned()).collect();
    changes
        .keys()
        .filter(|key| !valid.contains(*key))
        .cloned()
        .collect()
}

#[tauri::command]
pub fn chapter_config_save(
    state: State<AppState>,
    req: ChapterConfigSaveArgs,
) -> Result<Value, String> {
    let defaults = config::chapter_defaults(&state.engine_root);
    if defaults.is_empty() {
        return Err("no chapter presets found in the engine configs directory".into());
    }
    if !defaults.contains_key(&req.chapter) {
        let chapters = defaults
            .keys()
            .map(|chapter| chapter.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!("chapter must be one of: {}", chapters));
    }
    if req.changes.keys().any(|key| !valid_config_key(key)) {
        return Err("invalid config key".into());
    }
    let unknown = unknown_change_keys(&defaults, &req.changes);
    if !unknown.is_empty() {
        return Err(format!(
            "unknown chapter config key: {}",
            unknown.join(", ")
        ));
    }

    config::mod_chapter_config_save(&state.mod_root, req.chapter, &req.changes)?;
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
        return Err(
            "invalid project name (letters, digits, space, dash, underscore; max 64)".into(),
        );
    }
    if config::detect_template(&state.mod_root).is_none() {
        return Err("not a thrash-machine template".into());
    }
    let argv = vec![
        "bash".into(),
        state
            .mod_root
            .join("start.sh")
            .to_string_lossy()
            .into_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Union of the real engine configs/chapter1..5.json keys (47 keys).
    const REAL_DEFAULT_KEYS: &[&str] = &[
        "awakeMessages",
        "canTossLightWeapons",
        "checkActDescription",
        "damageUnderflowFix",
        "darkCandyForm",
        "darkCurrency",
        "darkCurrencyShort",
        "darkTextboxStyle",
        "defaultInvulnTime",
        "enableRecruits",
        "enemyAuras",
        "enemyBarPercentages",
        "growStronger",
        "growStrongerChara",
        "healthConversion",
        "keepTensionAfterBattle",
        "lessEquipments",
        "lightCurrency",
        "lightCurrencyShort",
        "lightTextboxStyle",
        "mercyBar",
        "mercyMessages",
        "newChoicers",
        "newShopSpaceUI",
        "newSpellCostCalculation",
        "oldDualHealFormula",
        "oldGameOver",
        "oldPacify",
        "oldRudeBuster",
        "oldTensionBar",
        "oldUIPositions",
        "overworldSpells",
        "pacifyGlow",
        "partyActions",
        "prioritySpareableText",
        "pushBlockInputLock",
        "ralseiStyle",
        "recruitsProgressSpaces",
        "shopSpaceUIFont",
        "smallSaveMenu",
        "soulInvBetweenWaves",
        "speechBubble",
        "storageSlots",
        "susieStyle",
        "targetSystem",
        "tiredMessages",
        "tpName",
    ];

    /// Build five chapter-default maps. Every real key is present; keys not
    /// in `rows` default to null so the key set still exercises the union.
    fn defaults_for(
        rows: &[(&str, Value, Value, Value, Value, Value)],
    ) -> BTreeMap<i64, Map<String, Value>> {
        let mut maps: Vec<Map<String, Value>> = (0..5).map(|_| Map::new()).collect();
        for (key, ch1, ch2, ch3, ch4, ch5) in rows {
            maps[0].insert(key.to_string(), ch1.clone());
            maps[1].insert(key.to_string(), ch2.clone());
            maps[2].insert(key.to_string(), ch3.clone());
            maps[3].insert(key.to_string(), ch4.clone());
            maps[4].insert(key.to_string(), ch5.clone());
        }
        for key in REAL_DEFAULT_KEYS {
            for map in &mut maps {
                if !map.contains_key(*key) {
                    map.insert(key.to_string(), Value::Null);
                }
            }
        }
        maps.into_iter()
            .enumerate()
            .map(|(index, map)| ((index + 1) as i64, map))
            .collect()
    }

    fn item<'a>(view: &'a Value, key: &str) -> &'a Value {
        view["items"]
            .as_array()
            .expect("items array")
            .iter()
            .find(|item| item["key"].as_str() == Some(key))
            .expect("item")
    }

    #[test]
    fn chapter_config_language_matches_the_gui_language_contract() {
        assert!(matches!(
            chapter_config_language(None),
            ChapterConfigLanguage::Chinese
        ));
        assert!(matches!(
            chapter_config_language(Some("zh-hans")),
            ChapterConfigLanguage::Chinese
        ));
        assert!(matches!(
            chapter_config_language(Some("en")),
            ChapterConfigLanguage::English
        ));
    }

    #[test]
    fn view_keys_are_the_real_chapter_defaults_union() {
        let defaults = defaults_for(&[]);
        let view = chapter_config_view(&defaults, &Map::new(), &BTreeMap::new(), 1);
        let keys: Vec<String> = view["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["key"].as_str().unwrap().to_string())
            .collect();

        let expected: Vec<String> = REAL_DEFAULT_KEYS.iter().map(|s| s.to_string()).collect();
        assert_eq!(keys, expected);
        assert!(keys.contains(&"storageSlots".to_string()));
        assert!(!keys.contains(&"enableStorage".to_string()));
        assert!(!keys.contains(&"default_encounter".to_string()));
    }

    #[test]
    fn override_wins_over_selected_chapter_default() {
        let defaults = defaults_for(&[(
            "enemyAuras",
            Value::Bool(false),
            Value::Bool(true),
            Value::Bool(true),
            Value::Bool(true),
            Value::Bool(true),
        )]);
        let mut overrides = Map::new();
        overrides.insert("enemyAuras".to_string(), Value::Bool(false));

        let view = chapter_config_view(&defaults, &overrides, &BTreeMap::new(), 2);
        let enemy_auras = item(&view, "enemyAuras");

        assert_eq!(enemy_auras["isOverride"], true);
        assert_eq!(enemy_auras["current"]["value"], false);
        assert_eq!(enemy_auras["current"]["label"], "否");
    }

    #[test]
    fn boolean_and_null_labels_are_semantic() {
        let defaults = defaults_for(&[
            (
                "growStronger",
                Value::Bool(false),
                Value::Bool(true),
                Value::Bool(true),
                Value::Bool(false),
                Value::Bool(true),
            ),
            (
                "growStrongerChara",
                Value::Null,
                Value::String("noelle".into()),
                Value::String("noelle".into()),
                Value::String("noelle".into()),
                Value::String("noelle".into()),
            ),
        ]);
        let view = chapter_config_view(&defaults, &Map::new(), &BTreeMap::new(), 1);

        let grow = item(&view, "growStronger");
        assert_eq!(grow["current"]["label"], "否");
        assert_eq!(grow["current"]["value"], false);
        let labels: Vec<&str> = grow["options"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["label"].as_str().unwrap())
            .collect();
        assert!(labels.contains(&"是"));
        assert!(labels.contains(&"否"));

        let chara = item(&view, "growStrongerChara");
        assert_eq!(chara["current"]["label"], "未设置");
        assert_eq!(chara["current"]["value"], Value::Null);
        assert_eq!(chara["chValues"]["1"]["label"], "未设置");
    }

    #[test]
    fn english_value_labels_follow_the_ui_language() {
        let defaults = defaults_for(&[
            (
                "enemyAuras",
                Value::Bool(false),
                Value::Bool(true),
                Value::Bool(true),
                Value::Bool(true),
                Value::Bool(true),
            ),
            (
                "growStrongerChara",
                Value::Null,
                Value::String("noelle".into()),
                Value::String("noelle".into()),
                Value::String("noelle".into()),
                Value::String("noelle".into()),
            ),
            (
                "speechBubble",
                Value::String("round".into()),
                Value::String("cyber".into()),
                Value::String("cyber".into()),
                Value::String("cyber".into()),
                Value::String("cyber".into()),
            ),
            (
                "darkCurrency",
                Value::String("Dark Dollars".into()),
                Value::String("Dark Dollars".into()),
                Value::String("Dark Dollars".into()),
                Value::String("Dark Dollars".into()),
                Value::String("Dark Dollars".into()),
            ),
            (
                "darkCurrencyShort",
                Value::String("D$".into()),
                Value::String("D$".into()),
                Value::String("D$".into()),
                Value::String("D$".into()),
                Value::String("D$".into()),
            ),
            (
                "lightCurrency",
                Value::String("Money".into()),
                Value::String("Money".into()),
                Value::String("Money".into()),
                Value::String("Money".into()),
                Value::String("Money".into()),
            ),
            (
                "lightCurrencyShort",
                Value::String("$".into()),
                Value::String("$".into()),
                Value::String("$".into()),
                Value::String("$".into()),
                Value::String("$".into()),
            ),
            (
                "tpName",
                Value::String("TP".into()),
                Value::String("TP".into()),
                Value::String("TP".into()),
                Value::String("TP".into()),
                Value::String("TP".into()),
            ),
        ]);
        let mut features = BTreeMap::new();
        let mut enemy_auras = BTreeMap::new();
        enemy_auras.insert("1".to_string(), Value::String("否".into()));
        enemy_auras.insert("2".to_string(), Value::String("是".into()));
        features.insert("enemyAuras".to_string(), enemy_auras);

        let mut chara = BTreeMap::new();
        chara.insert("1".to_string(), Value::String("未设置".into()));
        features.insert("growStrongerChara".to_string(), chara);

        let mut speech_bubble = BTreeMap::new();
        speech_bubble.insert("1".to_string(), Value::String("\"圆形\"".into()));
        speech_bubble.insert("2".to_string(), Value::String("\"赛博\"".into()));
        features.insert("speechBubble".to_string(), speech_bubble);

        let mut dark_currency = BTreeMap::new();
        dark_currency.insert("1".to_string(), Value::String("\"黑暗币\"".into()));
        features.insert("darkCurrency".to_string(), dark_currency);

        let mut dark_currency_short = BTreeMap::new();
        dark_currency_short.insert("1".to_string(), Value::String("\"D$\"".into()));
        features.insert("darkCurrencyShort".to_string(), dark_currency_short);

        let mut light_currency = BTreeMap::new();
        light_currency.insert("1".to_string(), Value::String("\"Money\"".into()));
        features.insert("lightCurrency".to_string(), light_currency);

        let mut light_currency_short = BTreeMap::new();
        light_currency_short.insert("1".to_string(), Value::String("\"$\"".into()));
        features.insert("lightCurrencyShort".to_string(), light_currency_short);

        let mut tp_name = BTreeMap::new();
        tp_name.insert("1".to_string(), Value::String("\"TP\"".into()));
        features.insert("tpName".to_string(), tp_name);

        let mut overrides = Map::new();
        overrides.insert("lightCurrency".to_string(), Value::String("Credits".into()));

        let view = chapter_config_view_with_language(
            &defaults,
            &overrides,
            &features,
            1,
            ChapterConfigLanguage::English,
        );

        let enemy = item(&view, "enemyAuras");
        assert_eq!(enemy["current"]["label"], "No");
        assert_eq!(enemy["chValues"]["2"]["label"], "Yes");
        let enemy_labels: Vec<&str> = enemy["options"]
            .as_array()
            .unwrap()
            .iter()
            .map(|option| option["label"].as_str().unwrap())
            .collect();
        assert!(enemy_labels.contains(&"No"));
        assert!(enemy_labels.contains(&"Yes"));

        let chara = item(&view, "growStrongerChara");
        assert_eq!(chara["current"]["label"], "Unset");

        let speech_bubble = item(&view, "speechBubble");
        assert_eq!(speech_bubble["current"]["label"], "round");
        assert_eq!(speech_bubble["chValues"]["2"]["label"], "cyber");

        let dark_currency = item(&view, "darkCurrency");
        assert_eq!(dark_currency["current"]["label"], "Dark Dollars");
        assert_eq!(item(&view, "darkCurrencyShort")["current"]["label"], "D$");
        assert_eq!(item(&view, "lightCurrencyShort")["current"]["label"], "$");
        assert_eq!(item(&view, "tpName")["current"]["label"], "TP");
        let light_currency = item(&view, "lightCurrency");
        assert_eq!(light_currency["current"]["label"], "Credits");
        assert_eq!(light_currency["isOverride"], true);
        assert!(light_currency["options"]
            .as_array()
            .unwrap()
            .iter()
            .any(|option| option["label"] == "Money"));
    }

    #[test]
    fn save_rejects_keys_outside_the_real_chapter_defaults() {
        let defaults = defaults_for(&[(
            "storageSlots",
            Value::from(0),
            Value::from(24),
            Value::from(24),
            Value::from(36),
            Value::from(48),
        )]);
        let mut changes = BTreeMap::new();
        changes.insert("enableStorage".to_string(), Some(Value::Bool(false)));
        changes.insert("storageSlots".to_string(), Some(Value::from(12)));

        assert_eq!(
            unknown_change_keys(&defaults, &changes),
            vec!["enableStorage"]
        );
    }

    #[test]
    fn chapter_five_values_and_selector_come_from_engine_defaults() {
        let defaults = defaults_for(&[
            (
                "storageSlots",
                Value::from(0),
                Value::from(24),
                Value::from(24),
                Value::from(36),
                Value::from(48),
            ),
            (
                "newChoicers",
                Value::Bool(false),
                Value::Bool(false),
                Value::Bool(false),
                Value::Bool(false),
                Value::Bool(true),
            ),
        ]);
        let view = chapter_config_view(&defaults, &Map::new(), &BTreeMap::new(), 5);

        assert_eq!(view["chapters"], json!([1, 2, 3, 4, 5]));
        assert_eq!(item(&view, "storageSlots")["current"]["value"], 48);
        assert_eq!(item(&view, "newChoicers")["current"]["value"], true);
    }

    #[test]
    fn launch_language_requires_kristal_i18n() {
        let root = std::env::temp_dir().join(format!(
            "kdt-launch-language-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();

        assert!(validate_launch_language(&root, None).is_ok());
        let err = validate_launch_language(&root, Some("zh-hans")).unwrap_err();
        assert!(err.contains("kristalI18n"));

        let library = root.join("libraries").join("any-directory-name");
        std::fs::create_dir_all(&library).unwrap();
        std::fs::write(library.join("lib.json"), r#"{ "id": "kristalI18n" }"#).unwrap();

        assert!(validate_launch_language(&root, Some("zh-hans")).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn find_sidecar_matches_platform_suffixed_release_name() {
        // The raw release binary is named kristal-run-<os>-<arch>[.exe] —
        // the exact file the old probe (kristal-run / kristal-run.exe) missed.
        let dir = std::env::temp_dir().join(format!("kdt-sidecar-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = if cfg!(windows) { ".exe" } else { "" };
        let arch = match std::env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            other => other,
        };
        let name = format!("kristal-run-{}-{}{}", std::env::consts::OS, arch, exe);
        std::fs::write(dir.join(&name), b"sidecar").unwrap();
        let found = super::find_sidecar(&dir);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            found.map(|p| p.file_name().unwrap().to_string_lossy().into_owned()),
            Some(name)
        );
    }

    #[test]
    fn find_sidecar_ignores_tmp_leftovers() {
        let dir = std::env::temp_dir().join(format!("kdt-sidecar-tmp-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("kristal-run-windows-x64.exe.tmp"), b"partial").unwrap();
        let found = super::find_sidecar(&dir);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(found.is_none());
    }
}
