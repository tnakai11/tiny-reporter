use std::io;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Local;
mod cli;
mod exec;
mod storage;
mod util;
use clap::Parser;

use cli::{Cli, Commands, RunOpts};

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run(opts) => {
            if let Err(e) = run(opts) {
                eprintln!("Error: {e}");
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
        Some(s) => Some(util::parse_duration_str(s).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid interval '{s}': {e}"),
            )
        })?),
        None => None,
    };
    let timeout_dur = match &timeout {
        Some(s) => Some(util::parse_duration_str(s).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid timeout '{s}': {e}"),
            )
        })?),
        None => None,
    };
    let fmt = format.to_lowercase();
    if fmt != "csv" && fmt != "jsonl" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "format must be 'csv' or 'jsonl'",
        ));
    }

    // Acquire global lock to prevent concurrent runs of same name
    let data_dir = storage::ensure_data_dir(&name)?;
    let lock_path = data_dir.join(format!("{name}.lock"));
    let _lock_file = storage::acquire_lock(&lock_path)?;

    // Set up Ctrl-C handler for graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");
    }

    // Determine initial date for rotation
    let mut current_date = storage::current_date();
    loop {
        // Determine file path based on current date
        let file_path = util::record_file_path(&data_dir, &current_date, &fmt);

        // Run the command
        let timestamp = Local::now().to_rfc3339();
        match exec::run_shell_command(&command_str, timeout_dur) {
            Ok((output, exit_code)) => {
                if fmt == "csv" {
                    storage::write_csv_record(&file_path, &timestamp, &output, exit_code)?;
                } else {
                    storage::write_jsonl_record(&file_path, &timestamp, &output, exit_code)?;
                }
            }
            Err(e) => {
                // Write error message as output with exit_code -1
                let msg = format!("error: {e}");
                if fmt == "csv" {
                    storage::write_csv_record(&file_path, &timestamp, &msg, -1)?;
                } else {
                    storage::write_jsonl_record(&file_path, &timestamp, &msg, -1)?;
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
        let now_date = storage::current_date();
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn duration_parse_valid() {
        assert_eq!(
            util::parse_duration_str("1s").unwrap(),
            Duration::from_secs(1)
        );
        assert_eq!(
            util::parse_duration_str("2m").unwrap(),
            Duration::from_secs(120)
        );
        assert!(util::parse_duration_str("500ms").unwrap() <= Duration::from_millis(500));
    }

    #[test]
    fn duration_parse_invalid() {
        assert!(util::parse_duration_str("").is_err());
        assert!(util::parse_duration_str("notaduration").is_err());
    }

    #[test]
    fn write_csv_and_jsonl() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("out.csv");
        let jsonl_path = dir.path().join("out.jsonl");
        storage::write_csv_record(&csv_path, "2025-01-01T00:00:00Z", "hello", 0).unwrap();
        let csv_contents = std::fs::read_to_string(&csv_path).unwrap();
        assert!(csv_contents.contains("2025-01-01T00:00:00Z,hello,0"));

        storage::write_jsonl_record(&jsonl_path, "2025-01-01T00:00:00Z", "hello", 0).unwrap();
        let jsonl_contents = std::fs::read_to_string(&jsonl_path).unwrap();
        assert!(jsonl_contents.trim().starts_with("{"));
        assert!(jsonl_contents.contains("\"timestamp\":"));
        assert!(jsonl_contents.contains("\"value\":"));
        assert!(jsonl_contents.contains("\"exit_code\":"));
    }
}
