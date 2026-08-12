//! Mod info, template detection, chapter config read/write (JSONC-
//! preserving), engine version/hash.

use crate::jsonc;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub fn read_mod_json(mod_root: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(mod_root.join("mod.json")).ok()?;
    serde_json::from_str(&jsonc::strip(&text)).ok()
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
pub fn config_overrides(mod_root: &Path) -> Map<String, Value> {
    read_mod_json(mod_root)
        .and_then(|v| v.get("config").cloned())
        .and_then(|v| v.get("kristal").cloned())
        .and_then(|v| v.as_object().cloned())
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
            if let Ok(v) = serde_json::from_str::<Value>(&jsonc::strip(&text)) {
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
    let v: Value = serde_json::from_str(&jsonc::strip(&text)).ok()?;
    Some((
        v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
    ))
}

/// Set config.kristal.<key> in mod.json (JSONC-preserving, textual
/// replace). value == None removes the key.
pub fn mod_config_set(mod_root: &Path, key: &str, value: Option<Value>) -> Result<(), String> {
    let path = mod_root.join("mod.json");
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let config_start = jsonc::find_object_start(&text, "config")
        .ok_or_else(|| "config block not found in mod.json".to_string())?;
    let config_end = jsonc::object_end(&text, config_start);
    let config_slice = &text[config_start..config_end];
    let kristal_start_rel = jsonc::find_object_start(config_slice, "kristal")
        .ok_or_else(|| "config.kristal block not found in mod.json".to_string())?;
    let kristal_start = config_start + kristal_start_rel;
    let kristal_end = jsonc::object_end(&text, kristal_start);
    let block = &text[kristal_start..kristal_end];

    let key_line = regex::Regex::new(&format!(r#"(?m)^([ \t]*)"{}"\s*:\s*[^\r\n]*\r?\n"#, regex::escape(key)))
        .unwrap();
    let value_re = regex::Regex::new(&format!(r#"("{}"\s*:\s*)[^,\r\n]*"#, regex::escape(key))).unwrap();

    let new_block = match value {
        None => {
            let removed = key_line.replace_all(block, "").to_string();
            if removed == block {
                return Ok(()); // nothing to remove
            }
            removed
        }
        Some(v) => {
            let json_val = serde_json::to_string(&v).map_err(|e| e.to_string())?;
            let updated = value_re.replace(block, format!("${{1}}{}", json_val).as_str()).to_string();
            if updated != block {
                updated
            } else {
                let insert = format!("\n    \"{}\": {},", key, json_val);
                format!("{}{}{}", &block[..1], insert, &block[1..])
            }
        }
    };

    let new_text = format!("{}{}{}", &text[..kristal_start], new_block, &text[kristal_end..]);
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
            if let Ok(v) = serde_json::from_str::<Value>(&jsonc::strip(&text)) {
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
