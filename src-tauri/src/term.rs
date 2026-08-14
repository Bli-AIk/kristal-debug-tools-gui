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
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE};
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW, CREATE_NEW_CONSOLE,
    };

    pub fn spawn(argv: &[String], dir: &Path, pause: bool) -> Result<(), String> {
        let real_argv: Vec<String> = if pause {
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
            vec![
                "powershell".to_string(),
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-EncodedCommand".to_string(),
                encoded,
            ]
        } else {
            argv.to_vec()
        };
        spawn_create_process(&real_argv, dir)
    }

    /// Launch argv directly via CreateProcessW with CREATE_NEW_CONSOLE and
    /// bInheritHandles=FALSE — no shell, no quoting surprises. The child gets
    /// NO inherited handles, so its standard handles default to NULL; being a
    /// console app attached to a fresh console, its CRT binds them to that new
    /// console's CONIN$/CONOUT$.
    ///
    /// This matters in dev builds: the debug exe is a console-subsystem app
    /// attached to the `just gui-dev` terminal, and std::process::Command
    /// forwards the parent's stdio handles to the child — the child writes
    /// into the dev terminal while its new window stays blank. (In release the
    /// exe is a windows-subsystem app with no console, so its handles are
    /// already NULL and Rust's Command happened to work by accident.) The
    /// explicit bInheritHandles=FALSE makes both modes behave the same.
    fn spawn_create_process(argv: &[String], dir: &Path) -> Result<(), String> {
        let exe = resolve_exe(&argv[0]).ok_or_else(|| format!("cannot resolve {}", argv[0]))?;
        let cmdline = argv
            .iter()
            .map(|a| quote_cmd_arg(a))
            .collect::<Vec<_>>()
            .join(" ");
        let mut cmdline_w: Vec<u16> = cmdline.encode_utf16().chain(std::iter::once(0)).collect();
        let exe_w = to_wide(exe.as_os_str());
        let dir_w = to_wide(dir.as_os_str());
        let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
        let ok = unsafe {
            CreateProcessW(
                exe_w.as_ptr(),
                cmdline_w.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                FALSE, // do not inherit the GUI's stdio/console handles
                CREATE_NEW_CONSOLE,
                std::ptr::null(),
                dir_w.as_ptr(),
                &si,
                &mut pi,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        unsafe {
            CloseHandle(pi.hThread);
            CloseHandle(pi.hProcess);
        }
        Ok(())
    }

    /// Resolve a program name to a path CreateProcessW can use, mirroring
    /// std::process::Command: paths with a separator pass through unchanged,
    /// bare names are searched on PATH with .exe appended.
    fn resolve_exe(name: &str) -> Option<std::ffi::OsString> {
        let p = std::path::Path::new(name);
        if p.is_absolute() || name.contains('\\') || name.contains('/') {
            return Some(p.as_os_str().to_owned());
        }
        let candidate = if p.extension().is_none() {
            format!("{}.exe", name)
        } else {
            name.to_string()
        };
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path)
            .map(|d| d.join(&candidate))
            .find(|c| c.is_file())
            .map(|c| c.into_os_string())
    }

    /// Quote one argument for a Windows command line (CRT CommandLineToArgvW
    /// rules): wrap in quotes when it contains whitespace, double backslashes
    /// before an embedded quote, and double trailing backslashes.
    fn quote_cmd_arg(s: &str) -> String {
        if s.is_empty() {
            return "\"\"".to_string();
        }
        let has_meta =
            s.contains('"') || s.chars().any(|c| c == ' ' || c == '\t' || c == '\n' || c == '\r');
        if !has_meta {
            return s.to_string();
        }
        let mut out = String::with_capacity(s.len() + 2);
        out.push('"');
        let mut backslashes = 0usize;
        for c in s.chars() {
            match c {
                '\\' => backslashes += 1,
                '"' => {
                    // Backslashes directly before a quote are doubled so they
                    // stay literal, then the quote itself is escaped.
                    for _ in 0..backslashes * 2 {
                        out.push('\\');
                    }
                    backslashes = 0;
                    out.push('\\');
                    out.push('"');
                }
                _ => {
                    for _ in 0..backslashes {
                        out.push('\\');
                    }
                    backslashes = 0;
                    out.push(c);
                }
            }
        }
        // Trailing backslashes are doubled before the closing quote.
        for _ in 0..backslashes * 2 {
            out.push('\\');
        }
        out.push('"');
        out
    }

    fn to_wide(s: &OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
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
