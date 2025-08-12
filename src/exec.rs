use std::io;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Run the provided shell command and capture its stdout.
/// Returns (output trimmed, exit code).
pub fn run_shell_command(command: &str, timeout: Option<Duration>) -> io::Result<(String, i32)> {
    let shell = if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "bash"
    };
    let args: &[&str] = if cfg!(target_os = "windows") {
        &["/C"]
    } else {
        &["-lc"]
    };
    let child = Command::new(shell)
        .args(args)
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let pid = child.id();

    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let output = match child.wait_with_output() {
            Ok(out) => out,
            Err(e) => {
                let _ = tx.send(Err(io::Error::other(format!("wait error: {e}"))));
                return;
            }
        };
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let _ = tx.send(Ok((stdout_str, exit_code)));
    });

    if let Some(to) = timeout {
        match rx.recv_timeout(to) {
            Ok(res) => res,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if cfg!(target_os = "windows") {
                    let _ = Command::new("taskkill")
                        .args(["/PID", &pid.to_string(), "/T", "/F"])
                        .status();
                } else {
                    let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
                }
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("command timed out after {to:?}"),
                ))
            }
            Err(_) => Err(io::Error::other("command execution error")),
        }
    } else {
        match rx.recv() {
            Ok(res) => res,
            Err(_) => Err(io::Error::other("command execution error")),
        }
    }
}
