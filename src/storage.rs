use chrono::Local;
use fs2::FileExt;
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct JsonRecord<'a> {
    timestamp: &'a str,
    value: &'a str,
    exit_code: i32,
}

pub fn write_csv_record(
    path: &Path,
    timestamp: &str,
    value: &str,
    exit_code: i32,
) -> io::Result<()> {
    let file_exists = path.exists();
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(!file_exists)
        .from_writer(file);
    wtr.write_record([timestamp, value, &exit_code.to_string()])?;
    wtr.flush()?;
    Ok(())
}

pub fn write_jsonl_record(
    path: &Path,
    timestamp: &str,
    value: &str,
    exit_code: i32,
) -> io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let record = JsonRecord {
        timestamp,
        value,
        exit_code,
    };
    let json = serde_json::to_string(&record)?;
    writeln!(file, "{json}")?;
    Ok(())
}

pub fn ensure_data_dir(name: &str) -> io::Result<PathBuf> {
    // Determine base directory: ~/.tiny-reporter/<name>
    let base = match directories::BaseDirs::new() {
        Some(b) => b.home_dir().to_path_buf(),
        None => PathBuf::from("."),
    };
    let dir = base.join(".tiny-reporter").join(name);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn acquire_lock(lock_path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .create(true)
        .append(true)
        .open(lock_path)?;
    match FileExt::try_lock_exclusive(&file) {
        Ok(()) => Ok(file),
        Err(e) => Err(io::Error::other(format!("failed to acquire lock: {e}"))),
    }
}

pub fn current_date() -> chrono::NaiveDate {
    Local::now().date_naive()
}
