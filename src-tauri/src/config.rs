//! Mod info, template detection, chapter config read/write (JSONC-
//! preserving via the jsonc-parser CST), engine version/hash.

use jsonc_parser::cst::{CstInputValue, CstObject, CstRootNode};
use jsonc_parser::ParseOptions;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn parse_jsonc_value(text: &str) -> Option<Value> {
    jsonc_parser::parse_to_serde_value::<Value>(text, &ParseOptions::default()).ok()
}

pub fn read_mod_json(mod_root: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(mod_root.join("mod.json")).ok()?;
    parse_jsonc_value(&text)
}

pub fn mod_name_subtitle(mod_root: &Path) -> (String, String) {
    let Some(v) = read_mod_json(mod_root) else {
        return (String::new(), String::new());
    };
    (
        v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        v.get("subtitle").and_then(|x| x.as_str()).unwrap_or("").to_string(),
    )
}

/// mod.json's config.kristal block (the overrides the engine reads).
/// Only values that actually exist and are not JSON null count as user
/// overrides; null means "delete the override", so Kristal falls back to
/// the selected chapter's default.
pub fn config_overrides(mod_root: &Path) -> Map<String, Value> {
    read_mod_json(mod_root)
        .and_then(|v| v.get("config").cloned())
        .and_then(|v| v.get("kristal").cloned())
        .and_then(|v| v.as_object().cloned())
        .map(|obj| {
            obj.into_iter()
                .filter(|(_, value)| !value.is_null())
                .collect()
        })
        .unwrap_or_default()
}

pub fn current_chapter(mod_root: &Path) -> i64 {
    read_mod_json(mod_root)
        .and_then(|v| v.get("chapter").and_then(|x| x.as_i64()))
        .unwrap_or(2)
}

/// Chapter defaults from the engine's configs/chapterN.json.
pub fn chapter_defaults(engine_root: &Path) -> Vec<Map<String, Value>> {
    let mut out = Vec::new();
    for n in 1..=4 {
        let mut m = Map::new();
        if let Ok(text) = std::fs::read_to_string(engine_root.join("configs").join(format!("chapter{}.json", n))) {
            if let Some(v) = parse_jsonc_value(&text) {
                if let Some(obj) = v.as_object() {
                    m = obj.clone();
                }
            }
        }
        out.push(m);
    }
    out
}

/// Engine version (VERSION file) and git commit hash (best effort).
pub fn engine_info(engine_root: &Path) -> (String, String) {
    let version = std::fs::read_to_string(engine_root.join("VERSION"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let hash = std::fs::read_to_string(engine_root.join(".git").join("HEAD"))
        .ok()
        .map(|h| h.trim().to_string())
        .and_then(|head| {
            if let Some(ref_path) = head.strip_prefix("ref: ") {
                std::fs::read_to_string(engine_root.join(".git").join(ref_path))
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                Some(head)
            }
        })
        .map(|h| h.chars().take(7).collect())
        .unwrap_or_default();
    (version, hash)
}

/// The thrash-machine template: subtitle marker + start.sh + git HEAD
/// id/name still matching the working copy (start.sh never rewrites the
/// subtitle, so HEAD comparison is the "already initialized?" probe).
pub fn detect_template(mod_root: &Path) -> Option<serde_json::Value> {
    if !mod_root.join("start.sh").is_file() {
        return None;
    }
    let v = read_mod_json(mod_root)?;
    if v.get("subtitle").and_then(|x| x.as_str()) != Some("Kristal Lua template") {
        return None;
    }
    if let Some((head_id, head_name)) = git_head_mod_json(mod_root) {
        let cur_id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
        let cur_name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
        if head_id != cur_id || head_name != cur_name {
            return None; // already initialized
        }
    }
    let chapter = v.get("chapter").and_then(|x| x.as_i64()).unwrap_or(2);
    Some(serde_json::json!({
        "isTemplate": true,
        "name": mod_root.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        "chapter": chapter,
    }))
}

fn git_head_mod_json(mod_root: &Path) -> Option<(String, String)> {
    let out = std::process::Command::new("git")
        .args(["-C", &mod_root.to_string_lossy(), "show", "HEAD:mod.json"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let v = parse_jsonc_value(&text)?;
    Some((
        v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
    ))
}

/// Recursively convert a serde_json value into the CST input value type.
/// This keeps future array/object config values compatible with the same
/// save path.
fn value_to_input(value: &Value) -> Result<CstInputValue, String> {
    Ok(match value {
        Value::Null => CstInputValue::Null,
        Value::Bool(b) => CstInputValue::Bool(*b),
        Value::Number(n) => CstInputValue::Number(n.to_string()),
        Value::String(s) => CstInputValue::String(s.clone()),
        Value::Array(items) => CstInputValue::Array(
            items
                .iter()
                .map(value_to_input)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Value::Object(obj) => CstInputValue::Object(
            obj.iter()
                .map(|(key, value)| Ok((key.clone(), value_to_input(value)?)))
                .collect::<Result<Vec<_>, String>>()?,
        ),
    })
}

/// Get an existing object property, or append a new empty object when the
/// property is absent. A present non-object is an error: the caller must
/// never overwrite someone's unusual config data.
fn object_value_or_create(parent: &CstObject, name: &str) -> Result<CstObject, String> {
    match parent.get(name) {
        Some(prop) => prop
            .object_value()
            .ok_or_else(|| format!("`{}` must be a JSON object", name)),
        None => {
            let prop = parent.append(name, CstInputValue::Object(Vec::new()));
            Ok(prop.object_value().expect("appended object must exist"))
        }
    }
}

fn set_chapter(root_obj: &CstObject, chapter: i64) {
    let value = CstInputValue::Number(chapter.to_string());
    match root_obj.get("chapter") {
        Some(prop) => prop.set_value(value),
        None => {
            root_obj.append("chapter", value);
        }
    }
}

fn delete_override(root_obj: &CstObject, key: &str) {
    let Some(config_prop) = root_obj.get("config") else {
        return;
    };
    let Some(config_obj) = config_prop.object_value() else {
        return;
    };
    let Some(kristal_prop) = config_obj.get("kristal") else {
        return;
    };
    let Some(kristal_obj) = kristal_prop.object_value() else {
        return;
    };
    if let Some(prop) = kristal_obj.get(key) {
        prop.remove();
    }
}

fn apply_chapter_config_text(
    text: &str,
    chapter: i64,
    changes: &BTreeMap<String, Option<Value>>,
) -> Result<String, String> {
    let root = CstRootNode::parse(text, &ParseOptions::default())
        .map_err(|e| format!("failed to parse mod.json: {}", e))?;
    let root_obj = root
        .object_value()
        .ok_or_else(|| "mod.json root must be a JSON object".to_string())?;

    set_chapter(&root_obj, chapter);

    let writes: Vec<(&String, &Value)> = changes
        .iter()
        .filter_map(|(key, value)| value.as_ref().map(|value| (key, value)))
        .collect();
    if !writes.is_empty() {
        let config_obj = object_value_or_create(&root_obj, "config")?;
        let kristal_obj = object_value_or_create(&config_obj, "kristal")?;
        for (key, value) in writes {
            let input = value_to_input(value)?;
            match kristal_obj.get(key) {
                Some(prop) => prop.set_value(input),
                None => {
                    kristal_obj.append(key, input);
                }
            }
        }
    }

    for (key, value) in changes {
        if value.is_none() {
            delete_override(&root_obj, key);
        }
    }

    Ok(root.to_string())
}

/// Persist a chapter baseline and a batch of config.kristal overrides with
/// one mod.json write. Selecting a chapter never materializes its defaults.
/// `None` deletes an override; JSON null is never written because Kristal
/// decodes it as Lua nil and it does not constitute a valid override.
pub fn mod_chapter_config_save(
    mod_root: &Path,
    chapter: i64,
    changes: &BTreeMap<String, Option<Value>>,
) -> Result<(), String> {
    let path = mod_root.join("mod.json");
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let new_text = apply_chapter_config_text(&text, chapter, changes)?;
    std::fs::write(&path, new_text).map_err(|e| e.to_string())
}

pub fn libraries(mod_root: &Path) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(mod_root.join("libraries")) else {
        return out;
    };
    for e in entries.flatten() {
        if !e.path().is_dir() {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(e.path().join("lib.json")) {
            if let Some(v) = parse_jsonc_value(&text) {
                if v.get("id").and_then(|x| x.as_str()).is_some() {
                    out.push(v);
                }
            }
        }
    }
    out.sort_by(|a, b| a["id"].as_str().unwrap_or("").cmp(b["id"].as_str().unwrap_or("")));
    out
}

pub fn find_justfile(mod_root: &Path) -> Option<PathBuf> {
    let p = mod_root.join("libraries").join("kristal-debug-tools").join("justfile");
    if p.is_file() {
        Some(p)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOD_JSON: &str = r#"{
    "chapter": 4,
    "config": {
        "kristal": {
            // A comment that should survive edits.
            "enemyAuras": true,
            "darkCandyForm": "darker",
            // End of config
        }
    }
}
"#;

    fn change(key: &str, value: Option<Value>) -> BTreeMap<String, Option<Value>> {
        let mut changes = BTreeMap::new();
        changes.insert(key.to_string(), value);
        changes
    }

    #[test]
    fn changing_chapter_does_not_materialize_defaults() {
        let out = apply_chapter_config_text(MOD_JSON, 2, &BTreeMap::new()).unwrap();
        let parsed = parse_jsonc_value(&out).unwrap();

        assert_eq!(parsed["chapter"], 2);
        assert_eq!(parsed["config"]["kristal"]["enemyAuras"], true);
        assert_eq!(parsed["config"]["kristal"]["darkCandyForm"], "darker");
        assert!(out.contains("A comment that should survive edits."));
        assert!(out.contains("// End of config"));
    }

    #[test]
    fn batch_save_updates_and_removes_only_requested_overrides() {
        let mut changes = BTreeMap::new();
        changes.insert("enemyAuras".to_string(), None);
        changes.insert("growStronger".to_string(), Some(Value::Bool(false)));

        let out = apply_chapter_config_text(MOD_JSON, 1, &changes).unwrap();
        let parsed = parse_jsonc_value(&out).unwrap();

        assert_eq!(parsed["chapter"], 1);
        assert!(parsed["config"]["kristal"].get("enemyAuras").is_none());
        assert_eq!(parsed["config"]["kristal"]["growStronger"], false);
        assert_eq!(parsed["config"]["kristal"]["darkCandyForm"], "darker");
        assert!(out.contains("A comment that should survive edits."));
        assert!(out.contains("// End of config"));
    }

    #[test]
    fn add_to_a_compact_empty_object_keeps_valid_jsonc() {
        let text = r#"{"chapter": 1, "config": {"kristal": {}}}"#;
        let out = apply_chapter_config_text(text, 3, &change("enemyAuras", Some(Value::Bool(true)))).unwrap();
        let parsed = parse_jsonc_value(&out).unwrap();

        assert_eq!(parsed["chapter"], 3);
        assert_eq!(parsed["config"]["kristal"]["enemyAuras"], true);
    }

    #[test]
    fn missing_config_block_is_created_for_an_override() {
        let text = r#"{"chapter": 1}"#;
        let out = apply_chapter_config_text(text, 1, &change("enemyAuras", Some(Value::Bool(false)))).unwrap();
        let parsed = parse_jsonc_value(&out).unwrap();

        assert_eq!(parsed["config"]["kristal"]["enemyAuras"], false);
    }

    #[test]
    fn pure_chapter_switch_or_deletion_does_not_create_config() {
        let out = apply_chapter_config_text(r#"{"chapter": 1}"#, 3, &BTreeMap::new()).unwrap();
        assert!(!out.contains("\"config\""));

        let out = apply_chapter_config_text(
            r#"{"chapter": 1}"#,
            3,
            &change("enemyAuras", None),
        )
        .unwrap();
        assert!(!out.contains("\"config\""));
    }

    #[test]
    fn wrong_config_types_are_rejected_when_writing() {
        let text = r#"{"chapter": 1, "config": []}"#;
        let err = apply_chapter_config_text(text, 1, &change("enemyAuras", Some(Value::Bool(true)))).unwrap_err();
        assert!(err.contains("`config` must be a JSON object"));

        let text = r#"{"chapter": 1, "config": {"kristal": []}}"#;
        let err = apply_chapter_config_text(text, 1, &change("enemyAuras", Some(Value::Bool(true)))).unwrap_err();
        assert!(err.contains("`kristal` must be a JSON object"));
    }

    #[test]
    fn null_change_removes_override_and_never_writes_null() {
        let out = apply_chapter_config_text(MOD_JSON, 4, &change("enemyAuras", None)).unwrap();
        let parsed = parse_jsonc_value(&out).unwrap();

        assert!(parsed["config"]["kristal"].get("enemyAuras").is_none());
        assert!(!out.contains("null"));
    }

    #[test]
    fn first_middle_and_last_properties_can_be_removed() {
        for key in ["a", "b", "c"] {
            let text = r#"{"chapter": 1, "config": {"kristal": {"a": 1, "b": 2, "c": 3}}}"#;
            let out = apply_chapter_config_text(text, 1, &change(key, None)).unwrap();
            let parsed = parse_jsonc_value(&out).unwrap();
            assert!(parsed["config"]["kristal"].get(key).is_none());
            for other in ["a", "b", "c"] {
                if other != key {
                    assert!(parsed["config"]["kristal"].get(other).is_some());
                }
            }
        }
    }

    #[test]
    fn set_value_preserves_same_line_comments() {
        let text = r#"{
    "chapter": 4,
    "config": {
        "kristal": {
            "enemyAuras": true, // aura comment
        }
    }
}"#;
        let out = apply_chapter_config_text(text, 4, &change("enemyAuras", Some(Value::Bool(false)))).unwrap();
        let parsed = parse_jsonc_value(&out).unwrap();

        assert_eq!(parsed["config"]["kristal"]["enemyAuras"], false);
        assert!(out.contains("// aura comment"));
    }

    #[test]
    fn appending_respects_trailing_commas() {
        let text = r#"{
    "chapter": 4,
    "config": {
        "kristal": {
            "enemyAuras": true,
        }
    }
}"#;
        let out = apply_chapter_config_text(text, 4, &change("darkCandyForm", Some(Value::String("darker".into())))).unwrap();
        assert!(out.contains("\"darkCandyForm\": \"darker\","));
        assert!(parse_jsonc_value(&out).is_some());
    }
}
