//! Task listing via `just --dump --format json`. `just` is a Rust crate
//! compiled into the kristal-run sidecar (just-dump subcommand), whose
//! stdout the GUI captures; `just` itself writes to the process-wide
//! stdout, so it can never run in-process.

use serde_json::{json, Map, Value};
use std::path::Path;
use std::process::Command;

const DESCS_JSON: &str = include_str!("../resources/config-features.json");

/// How the dump is invoked: through the embedded sidecar (just compiled
/// in) or through a system `just` binary found on PATH.
#[derive(Clone, Copy, PartialEq)]
pub enum JustSource {
    /// `kristal-run just-dump <justfile>` — just is compiled into the sidecar.
    Embedded,
    /// A `just` binary discovered on PATH (dev fallback).
    System,
}

fn desc_map() -> Map<String, Value> {
    serde_json::from_str(DESCS_JSON)
        .ok()
        .and_then(|v: Value| v.as_array().cloned())
        .map(|items| {
            items
                .into_iter()
                .filter_map(|it| {
                    let key = it.get("key")?.as_str()?.to_string();
                    let desc = it.get("desc").and_then(|d| d.as_str()).unwrap_or("").to_string();
                    Some((key, Value::String(desc)))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn run_dump(source: JustSource, runner: &Path, justfile: &Path, dir: &Path) -> Option<Value> {
    let out = match source {
        JustSource::Embedded => Command::new(runner)
            .args(["just-dump", &justfile.to_string_lossy()])
            .current_dir(dir)
            .output(),
        JustSource::System => Command::new(runner)
            .args(["--justfile", &justfile.to_string_lossy(), "--dump", "--dump-format", "json"])
            .current_dir(dir)
            .output(),
    }
    .ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).ok()
}

/// Parse the dump into task items: {name, doc, private, params:[{name,kind}]}.
fn parse_tasks(dump: &Value) -> Vec<Value> {
    let recipes = dump.get("recipes").and_then(|r| r.as_object());
    let aliases = dump.get("aliases").and_then(|r| r.as_object());
    let mut out = Vec::new();
    if let Some(recipes) = recipes {
        for (name, recipe) in recipes {
            let mut item = json!({
                "name": recipe.get("name").and_then(|n| n.as_str()).unwrap_or(name),
                "doc": recipe.get("doc").and_then(|d| d.as_str()).unwrap_or(""),
                "private": recipe.get("private").and_then(|p| p.as_bool()).unwrap_or(false),
            });
            let params: Vec<Value> = recipe
                .get("parameters")
                .and_then(|p| p.as_array())
                .map(|ps| {
                    ps.iter()
                        .map(|p| json!({
                            "name": p.get("name").and_then(|n| n.as_str()).unwrap_or(""),
                            "kind": p.get("kind").and_then(|k| k.as_str()).unwrap_or("singleton"),
                        }))
                        .collect()
                })
                .unwrap_or_default();
            item["params"] = Value::Array(params);
            if let Some(aliases) = aliases {
                let als: Vec<Value> = aliases
                    .iter()
                    .filter(|(_, a)| a.get("target").and_then(|t| t.as_str()) == Some(name))
                    .map(|(an, _)| Value::String(an.clone()))
                    .collect();
                if !als.is_empty() {
                    item["aliases"] = Value::Array(als);
                }
            }
            out.push(item);
        }
    }
    out.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
    out
}

/// Full listing: library justfile tasks + the mod root justfile's recipes
/// (deduplicated against the library's), like the old GUI.
pub fn list(source: JustSource, runner: &Path, library_justfile: &Path, mod_root: &Path) -> Value {
    let dump = run_dump(source, runner, library_justfile, mod_root);
    let (source_name, tasks) = match dump {
        Some(d) => ("dump", parse_tasks(&d)),
        None => ("builtin", Vec::new()),
    };
    let mut mod_group = None;
    let project_justfile = mod_root.join("justfile");
    if project_justfile.is_file() {
        if let Some(d) = run_dump(source, runner, &project_justfile, mod_root) {
            let mut mt = parse_tasks(&d);
            let library_names: std::collections::HashSet<String> =
                tasks.iter().filter_map(|t| t["name"].as_str().map(String::from)).collect();
            mt.retain(|t| !library_names.contains(t["name"].as_str().unwrap_or("")));
            if !mt.is_empty() {
                mod_group = Some(json!({ "source": "dump", "tasks": mt }));
            }
        }
    }
    json!({ "source": source_name, "tasks": tasks, "mod": mod_group })
}

/// Configurable-features descriptions (zh, from the Kristal website).
pub fn config_feature_descs() -> Map<String, Value> {
    desc_map()
}
