use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use std::thread;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use chrono::{Local, Datelike};
use serde::Serialize;

/// A tiny reporter that periodically runs shell commands and records their output.
#[derive(Parser)]
#[command(name = "trep")]
#[command(author = "trep developers")]
#[command(version)]
#[command(about = "Periodically run commands and record their output to CSV or JSONL", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a command on a schedule and record its output
    Run(RunOpts),
}

/// Options for the run subcommand
#[derive(Parser, Debug)]
struct RunOpts {
    /// Name for this job (used for directory and file naming)
    #[arg(long = "as", short = 'n')]
    name: String,
    /// Interval at which to run the command (e.g. "1m", "10s"). If omitted, runs once.
    #[arg(long)]
    every: Option<String>,
    /// Output format: "csv" or "jsonl". Defaults to csv.
    #[arg(long, default_value = "csv")]
    format: String,
    /// Timeout for each command run (e.g. "5s"). Optional.
    #[arg(long)]
    timeout: Option<String>,
    /// Command to execute, use after `--` to separate from options
    #[arg(last = true, required = true)]
    cmd: Vec<String>,
}

#[derive(Serialize)]
struct JsonRecord<'a> {
    timestamp: &'a str,
    value: &'a str,
    exit_code: i32,
}

fn parse_duration_str(s: &str) -> Result<Duration, humantime::DurationError> {
    // humantime::parse_duration parses strings like "1s", "2m", "500ms"
    humantime::parse_duration(s)
}

fn acquire_lock(lock_path: &Path) -> io::Result<File> {
    let file = OpenOptions::new().read(true).write(true).create(true).open(lock_path)?;
    // Try to acquire exclusive lock
    match fs2::FileExt::try_lock_exclusive(&file) {
        Ok(()) => Ok(file),
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, format!("failed to acquire lock: {}", e))),
    }
}

/// Run the provided shell command and capture its stdout.
/// Returns (output trimmed, exit code).
fn run_shell_command(command: &str, timeout: Option<Duration>) -> io::Result<(String, i32)> {
    // Use bash on Unix to interpret the command; fall back to sh if bash isn't available.
    // On Windows, users should write fully qualified commands; we still try bash if available.
    let shell = if cfg!(target_os = "windows") { "cmd" } else { "bash" };
    // For Windows, "/C" executes command and terminates; on Unix, "-lc" runs a login shell command
    let args: &[&str] = if cfg!(target_os = "windows") { &["/C"] } else { &["-lc"] };
    let mut child = Command::new(shell)
        .args(args)
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let pid = child.id();

    // Channel to collect output
    let (tx, rx) = std::sync::mpsc::channel();
    // Move the child into a background thread that waits and captures output
    thread::spawn(move || {
        let output = match child.wait_with_output() {
            Ok(out) => out,
            Err(e) => {
                let _ = tx.send(Err(io::Error::new(io::ErrorKind::Other, format!("wait error: {}", e))));
                return;
            }
        };
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Send output and exit code
        let _ = tx.send(Ok((stdout_str, exit_code)));
    });

    if let Some(to) = timeout {
        match rx.recv_timeout(to) {
            Ok(res) => res,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Attempt to terminate the process by PID on timeout
                if cfg!(target_os = "windows") {
                    let _ = Command::new("taskkill")
                        .args(["/PID", &pid.to_string(), "/T", "/F"])
                        .status();
                } else {
                    let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
                }
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("command timed out after {:?}", to),
                ))
            }
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "command execution error")),
        }
    } else {
        match rx.recv() {
            Ok(res) => res,
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "command execution error")),
        }
    }
}

fn write_csv_record(path: &Path, timestamp: &str, value: &str, exit_code: i32) -> io::Result<()> {
    let file_exists = path.exists();
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut wtr = csv::WriterBuilder::new().has_headers(!file_exists).from_writer(file);
    wtr.write_record(&[timestamp, value, &exit_code.to_string()])?;
    wtr.flush()?;
    Ok(())
}

fn write_jsonl_record(path: &Path, timestamp: &str, value: &str, exit_code: i32) -> io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let record = JsonRecord {
        timestamp,
        value,
        exit_code,
    };
    let json = serde_json::to_string(&record)?;
    writeln!(file, "{}", json)?;
    Ok(())
}

fn ensure_data_dir(name: &str) -> io::Result<PathBuf> {
    // Determine base directory: ~/.tiny-reporter/<name>
    let base = match directories::BaseDirs::new() {
        Some(b) => b.home_dir().to_path_buf(),
        None => PathBuf::from("."),
    };
    let dir = base.join(".tiny-reporter").join(name);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run(opts) => {
            if let Err(e) = run(opts) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn run(opts: RunOpts) -> io::Result<()> {
    let RunOpts {
        name,
        every,
        format,
        timeout,
        cmd,
    } = opts;
    // Build command string from cmd Vec
    let command_str = cmd.join(" ");
    // Parse durations
    let interval = match &every {
        Some(s) => Some(parse_duration_str(s).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("invalid interval '{}': {}", s, e)))?),
        None => None,
    };
    let timeout_dur = match &timeout {
        Some(s) => Some(parse_duration_str(s).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("invalid timeout '{}': {}", s, e)))?),
        None => None,
    };
    let fmt = format.to_lowercase();
    if fmt != "csv" && fmt != "jsonl" {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "format must be 'csv' or 'jsonl'"));
    }

    // Acquire global lock to prevent concurrent runs of same name
    let data_dir = ensure_data_dir(&name)?;
    let lock_path = data_dir.join(format!("{}.lock", name));
    let _lock_file = acquire_lock(&lock_path)?;

    // Set up Ctrl-C handler for graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
        }).expect("Error setting Ctrl-C handler");
    }

    // Determine initial date for rotation
    let mut current_date = Local::now().date_naive();
    loop {
        // Determine file path based on current date
        let date_str = current_date.format("%Y-%m-%d").to_string();
        let ext = if fmt == "csv" { "csv" } else { "jsonl" };
        let file_path = data_dir.join(format!("{}.{}", date_str, ext));

        // Run the command
        let timestamp = Local::now().to_rfc3339();
        match run_shell_command(&command_str, timeout_dur) {
            Ok((output, exit_code)) => {
                if fmt == "csv" {
                    write_csv_record(&file_path, &timestamp, &output, exit_code)?;
                } else {
                    write_jsonl_record(&file_path, &timestamp, &output, exit_code)?;
                }
            }
            Err(e) => {
                // Write error message as output with exit_code -1
                let msg = format!("error: {}", e);
                if fmt == "csv" {
                    write_csv_record(&file_path, &timestamp, &msg, -1)?;
                } else {
                    write_jsonl_record(&file_path, &timestamp, &msg, -1)?;
                }
            }
        }

        // If no interval specified, run once and exit
        if interval.is_none() {
            break;
        }

        // Check for shutdown
        if !running.load(Ordering::SeqCst) {
            break;
        }

        // Sleep for the specified interval
        if let Some(dur) = interval {
            let start = Instant::now();
            while running.load(Ordering::SeqCst) {
                let elapsed = Instant::now().duration_since(start);
                if elapsed >= dur {
                    break;
                }
                let remaining = dur - elapsed;
                // Sleep in smaller chunks to allow quicker Ctrl-C response
                let sleep_dur = if remaining > Duration::from_millis(100) {
                    Duration::from_millis(100)
                } else {
                    remaining
                };
                thread::sleep(sleep_dur);
            }
        }

        // Update current date for rotation
        let now_date = Local::now().date_naive();
        if now_date != current_date {
            current_date = now_date;
        }

        // Break if shutdown after sleeping
        if !running.load(Ordering::SeqCst) {
            break;
        }
    }
    Ok(())
}
