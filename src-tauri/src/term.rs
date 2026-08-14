//! Spawn commands in a NEW interactive terminal window, detached from the
//! GUI — the game and just tasks need a real tty (the terminal-cli console).

use std::path::Path;

/// Open a new terminal window running argv (dir as working directory).
/// pause keeps the window open after the command exits (for reading task
/// output); without it the window closes with the command (the game's
/// companion terminal).
#[cfg(windows)]
pub fn spawn_in_terminal(argv: &[String], dir: &Path, pause: bool) -> Result<(), String> {
    windows::spawn(argv, dir, pause)
}

#[cfg(not(windows))]
pub fn spawn_in_terminal(argv: &[String], dir: &Path, pause: bool) -> Result<(), String> {
    unix::spawn(argv, dir, pause)
}

#[cfg(windows)]
mod windows {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    /// Give the child its own console window, detached from the GUI.
    const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

    /// Launch argv directly via CreateProcess — no shell is involved, so
    /// Windows path quoting can never break the command line. (The old
    /// `cmd /c` + POSIX-single-quote path made every Windows launch die on
    /// the opening quote before the command ever ran.)
    pub fn spawn(argv: &[String], dir: &Path, pause: bool) -> Result<(), String> {
        let mut cmd = if pause {
            // keep-open: run argv from a PowerShell script that waits for
            // Enter after the command finishes. The script is passed
            // base64-encoded (-EncodedCommand) so nothing can mangle its
            // quoting along the way.
            let script = keep_open_script(argv);
            let encoded = STANDARD.encode(
                script
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes)
                    .collect::<Vec<_>>(),
            );
            let mut c = Command::new("powershell");
            c.args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-EncodedCommand",
                encoded.as_str(),
            ]);
            c
        } else {
            let mut c = Command::new(&argv[0]);
            c.args(&argv[1..]);
            c
        };
        cmd.current_dir(dir).creation_flags(CREATE_NEW_CONSOLE);
        cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
    }

    fn psh_quote(s: &str) -> String {
        // PowerShell single-quoted strings escape a literal quote as ''.
        format!("'{}'", s.replace('\'', "''"))
    }

    /// `& '<exe>' '<arg>' ...` — the call operator passes each argument
    /// through as-is (no string re-parsing), so paths with spaces stay
    /// intact; the script then waits for Enter in its own console.
    fn keep_open_script(argv: &[String]) -> String {
        let mut body = String::from("& ");
        body.push_str(&psh_quote(&argv[0]));
        for a in &argv[1..] {
            body.push(' ');
            body.push_str(&psh_quote(a));
        }
        body.push_str(
            "\r\nWrite-Host ''\r\n\
             Write-Host '[kristal-debug-tools] task finished - press Enter to close'\r\n\
             Read-Host\r\n",
        );
        body
    }
}

#[cfg(not(windows))]
mod unix {
    use super::*;
    use std::process::Command;

    const TERMINALS: &[&str] = &[
        "kitty",
        "gnome-terminal",
        "konsole",
        "xfce4-terminal",
        "x-terminal-emulator",
        "xterm",
    ];

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

    pub fn spawn(argv: &[String], dir: &Path, pause: bool) -> Result<(), String> {
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
