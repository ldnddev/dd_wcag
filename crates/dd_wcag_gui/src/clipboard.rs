//! System clipboard. The terminal uses the same programs.

use std::io::{self, Write};
use std::process::{Command, Stdio};

pub fn paste() -> io::Result<String> {
    #[cfg(target_os = "macos")]
    {
        return read_command_stdout("pbpaste", &[]);
    }

    #[cfg(target_os = "windows")]
    {
        return read_command_stdout("powershell", &["-NoProfile", "-Command", "Get-Clipboard"]);
    }

    #[cfg(target_os = "linux")]
    {
        let attempts: [(&str, &[&str]); 4] = [
            ("wl-paste", &["-n"]),
            ("xclip", &["-selection", "clipboard", "-o"]),
            ("xsel", &["--clipboard", "--output"]),
            ("termux-clipboard-get", &[]),
        ];
        let mut last_err = None;
        for (program, args) in attempts {
            match read_command_stdout(program, args) {
                Ok(text) => return Ok(text),
                Err(err) => last_err = Some(err),
            }
        }
        return Err(last_err.unwrap_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "no clipboard command configured")
        }));
    }

    #[allow(unreachable_code)]
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "clipboard paste is not supported on this platform",
    ))
}

pub fn copy(content: &str) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        return write_to_command_stdin("pbcopy", &[], content);
    }

    #[cfg(target_os = "windows")]
    {
        return write_to_command_stdin("clip", &[], content);
    }

    #[cfg(target_os = "linux")]
    {
        let attempts: [(&str, &[&str]); 4] = [
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
            ("termux-clipboard-set", &[]),
        ];
        let mut last_err = None;
        for (program, args) in attempts {
            match write_to_command_stdin(program, args, content) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = Some(err),
            }
        }
        return Err(last_err.unwrap_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "no clipboard command configured")
        }));
    }

    #[allow(unreachable_code)]
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "clipboard copy is not supported on this platform",
    ))
}

fn read_command_stdout(program: &str, args: &[&str]) -> io::Result<String> {
    let output = Command::new(program).args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(io::Error::other(format!(
            "{program} exited with {}",
            output.status
        )))
    }
}

fn write_to_command_stdin(program: &str, args: &[&str], content: &str) -> io::Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(content.as_bytes())?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("{program} exited with {status}")))
    }
}
