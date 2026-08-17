//! Task listing via `just --dump --format json`. `just` is a Rust crate
//! compiled into the kristal-run sidecar (just-dump subcommand), whose
//! stdout the GUI captures; `just` itself writes to the process-wide
//! stdout, so it can never run in-process.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

const DESCS_JSON: &str = include_str!("../resources/config-features.json");

/// Normalize a language name to one of "zh" | "en" | "default".
fn lang_alias(lang: &str) -> &'static str {
    match lang {
        "zh" | "zh_hans" | "zh-hans" | "zh_cn" | "zh-cn" | "zh_hant" | "zh-hant" | "zh_tw"
        | "zh-tw" => "zh",
        "en" => "en",
        _ => "default",
    }
}

/// Parse language-prefixed doc comments from a justfile into a map of
/// recipe name -> {lang -> doc}. A comment line starting with `# lang:`
/// is stored under that language; plain comment lines go to "default".
/// A comment block attaches to the next recipe/alias definition below it.
fn parse_doc_comments(text: &str) -> HashMap<String, HashMap<String, String>> {
    let mut out: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut block: Vec<String> = Vec::new();

    let commit =
        |block: &Vec<String>, out: &mut HashMap<String, HashMap<String, String>>, name: &str| {
            let mut map = HashMap::new();
            for line in block {
                if let Some((lang, doc)) = line.split_once(':') {
                    let lang = lang.trim();
                    if matches!(
                        lang,
                        "en" | "zh"
                            | "zh_hans"
                            | "zh-hans"
                            | "zh_cn"
                            | "zh-cn"
                            | "zh_hant"
                            | "zh-hant"
                            | "zh_tw"
                            | "zh-tw"
                    ) {
                        map.insert(lang_alias(lang).to_string(), doc.trim().to_string());
                        continue;
                    }
                }
                map.entry("default".to_string())
                    .or_insert_with(|| line.trim().to_string());
            }
            if !map.is_empty() {
                out.insert(name.to_string(), map);
            }
        };

    for line in text.lines() {
        let t = line.trim_start();
        if let Some(c) = t.strip_prefix('#') {
            block.push(c.trim_start().to_string());
        } else if !t.is_empty() && !t.starts_with('\t') {
            // A definition line ends the comment block — attach it if it
            // looks like a recipe or alias definition (`name:`).
            let head = t.split(':').next().unwrap_or(t);
            let word = head.split_whitespace().next().unwrap_or("");
            let is_recipe = !word.is_empty()
                && word
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
                && !word
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(true);
            if is_recipe {
                commit(&block, &mut out, word);
            }
            block.clear();
        }
    }
    out
}

/// Pick the doc for `lang` from the parsed comment map, falling back to
/// the default-language doc.
fn doc_for(
    lang: &str,
    docs: &HashMap<String, HashMap<String, String>>,
    name: &str,
) -> Option<String> {
    let want = lang_alias(lang);
    docs.get(name)
        .and_then(|m| m.get(want).or_else(|| m.get("default")).cloned())
}

/// How the dump is invoked: through the embedded sidecar (just compiled
/// in) or through a system `just` binary found on PATH.
#[derive(Clone, Copy, PartialEq)]
pub enum JustSource {
    /// `kristal-run just-dump <justfile>` — just is compiled into the sidecar.
    Embedded,
    /// A `just` binary discovered on PATH (dev fallback).
    System,
}

fn run_dump(source: JustSource, runner: &Path, justfile: &Path, dir: &Path) -> Option<Value> {
    let out = match source {
        JustSource::Embedded => Command::new(runner)
            .args(["just-dump", &justfile.to_string_lossy()])
            .current_dir(dir)
            .output(),
        JustSource::System => Command::new(runner)
            .args([
                "--justfile",
                &justfile.to_string_lossy(),
                "--dump",
                "--dump-format",
                "json",
            ])
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
    out.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });
    out
}

/// Full listing: library justfile tasks + the mod root justfile's recipes
/// (deduplicated against the library's), like the old GUI.
pub fn list(
    source: JustSource,
    runner: &Path,
    library_justfile: &Path,
    mod_root: &Path,
    lang: &str,
) -> Value {
    // Language-prefixed doc comments from both justfiles override the
    // dump's doc (just itself has no i18n in 1.58).
    let lib_docs = std::fs::read_to_string(library_justfile)
        .map(|t| parse_doc_comments(&t))
        .unwrap_or_default();
    let proj_docs = std::fs::read_to_string(mod_root.join("justfile"))
        .map(|t| parse_doc_comments(&t))
        .unwrap_or_default();

    let with_docs = |name: &str,
                     docs: &HashMap<String, HashMap<String, String>>|
     -> Option<String> { doc_for(lang, docs, name) };

    let dump = run_dump(source, runner, library_justfile, mod_root);
    let (source_name, mut tasks) = match dump {
        Some(d) => ("dump", parse_tasks(&d)),
        None => ("builtin", Vec::new()),
    };
    for t in &mut tasks {
        if let Some(name) = t["name"].as_str() {
            if let Some(doc) = with_docs(name, &lib_docs) {
                t["doc"] = Value::String(doc);
            }
        }
    }
    let mut mod_group = None;
    let project_justfile = mod_root.join("justfile");
    if project_justfile.is_file() {
        if let Some(d) = run_dump(source, runner, &project_justfile, mod_root) {
            let mut mt = parse_tasks(&d);
            let library_names: std::collections::HashSet<String> = tasks
                .iter()
                .filter_map(|t| t["name"].as_str().map(String::from))
                .collect();
            mt.retain(|t| !library_names.contains(t["name"].as_str().unwrap_or("")));
            for t in &mut mt {
                if let Some(name) = t["name"].as_str() {
                    if let Some(doc) = with_docs(name, &proj_docs) {
                        t["doc"] = Value::String(doc);
                    }
                }
            }
            if !mt.is_empty() {
                mod_group = Some(json!({ "source": "dump", "tasks": mt }));
            }
        }
    }
    json!({ "source": source_name, "tasks": tasks, "mod": mod_group })
}

/// Full config-features rows: key -> copy, override candidates, and optional
/// human-readable per-chapter values. Engine JSON determines which chapters
/// and config keys are actually available.
pub fn config_feature_rows(
) -> std::collections::BTreeMap<String, std::collections::BTreeMap<String, Value>> {
    serde_json::from_str(DESCS_JSON)
        .ok()
        .and_then(|v: Value| v.as_array().cloned())
        .map(|items| {
            items
                .into_iter()
                .filter_map(|it| {
                    let key = it.get("key")?.as_str()?.to_string();
                    let mut row = std::collections::BTreeMap::new();
                    for field in ["name", "desc", "descEn"] {
                        if let Some(v) = it.get(field).and_then(|d| d.as_str()) {
                            row.insert(field.to_string(), Value::String(v.to_string()));
                        }
                    }
                    if let Some(v) = it.get("opts") {
                        row.insert("opts".to_string(), v.clone());
                    }
                    for (field, value) in it.as_object()? {
                        if let Some(chapter) = field
                            .strip_prefix("ch")
                            .and_then(|number| number.parse::<i64>().ok())
                        {
                            row.insert(chapter.to_string(), value.clone());
                        }
                    }
                    Some((key, row))
                })
                .collect()
        })
        .unwrap_or_default()
}
