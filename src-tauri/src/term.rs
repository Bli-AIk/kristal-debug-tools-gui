//! Spawn commands in a NEW interactive terminal window, detached from the
//! GUI — the game and just tasks need a real tty (the terminal-cli console).

use std::path::Path;
use std::process::Command;

const TERMINALS: &[&str] = &["kitty", "gnome-terminal", "konsole", "xfce4-terminal", "x-terminal-emulator", "xterm"];

fn find_terminal() -> Option<String> {
    if let Ok(t) = std::env::var("TERMINAL") {
        if which(&t) {
            return Some(t);
        }
    }
    TERMINALS.iter().find(|t| which(t)).map(|t| t.to_string())
}

fn which(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path)
                .any(|d| d.join(name).is_file() || d.join(format!("{}.exe", name)).is_file())
        })
        .unwrap_or(false)
}

fn shell_quote(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || "_@%+=:,./-!".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Open a new terminal window running argv (dir as working directory).
/// pause keeps the window open after the command exits (for reading task
/// output); without it the window closes with the command (the game's
/// companion terminal).
pub fn spawn_in_terminal(argv: &[String], dir: &Path, pause: bool) -> Result<(), String> {
    if cfg!(windows) {
        let cmdline = argv
            .iter()
            .map(|a| shell_quote(a))
            .collect::<Vec<_>>()
            .join(" ");
        let keep = if pause { "/k" } else { "/c" };
        Command::new("cmd")
            .args(["/c", "start", "", "cmd", keep, &cmdline])
            .current_dir(dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    } else {
        let term = find_terminal().ok_or_else(|| "no terminal emulator found".to_string())?;
        let mut wrapper = argv
            .iter()
            .map(|a| shell_quote(a))
            .collect::<Vec<_>>()
            .join(" ");
        if pause {
            wrapper.push_str("; echo; echo \"[kristal-debug-tools] finished — press Enter to close\"; read _");
        }
        let mut cmd = Command::new(&term);
        if term.contains("gnome-terminal") {
            cmd.arg("--");
        } else {
            cmd.arg("-e");
        }
        cmd.args(["sh", "-c", &wrapper]).current_dir(dir);
        cmd.spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
