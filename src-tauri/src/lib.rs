//! kristal-debug-tools GUI — Tauri backend.

mod commands;
mod config;
mod icons;
pub mod launcher; // used by the kristal-run sidecar bin
mod tasks;
mod term;

use commands::AppState;
use std::path::PathBuf;

/// Resolve the runtime context: mod root (walk up from cwd or
/// KDT_MOD_ROOT), engine (walk up or KRISTAL_ROOT), library justfile.
fn resolve_state() -> AppState {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let kdt_mod_root = std::env::var("KDT_MOD_ROOT").ok();
    let kristal_root = std::env::var("KRISTAL_ROOT").ok();
    let resolved = launcher::resolve(
        &cwd,
        kdt_mod_root.as_deref(),
        kristal_root.as_deref(),
    );
    match resolved {
        Ok(r) => AppState {
            justfile: config::find_justfile(&r.mod_root).unwrap_or_else(|| r.mod_root.join("justfile")),
            mod_root: r.mod_root,
            mod_id: r.mod_id,
            engine_root: r.engine_root,
        },
        Err(e) => {
            eprintln!("[kristal-debug-tools-gui] {}", e);
            AppState {
                mod_root: cwd,
                mod_id: String::new(),
                engine_root: PathBuf::new(),
                justfile: PathBuf::new(),
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(resolve_state())
        .invoke_handler(tauri::generate_handler![
            commands::status,
            commands::set_settings,
            commands::tasks,
            commands::run_task,
            commands::launch_game,
            commands::chapter_config,
            commands::chapter_config_save,
            commands::template_init,
            commands::engine_info_command,
            icons::icon_status,
            icons::icon_set,
            icons::icon_clear,
            icons::icon_generate,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
