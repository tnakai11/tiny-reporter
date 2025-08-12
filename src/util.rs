use chrono::NaiveDate;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn parse_duration_str(s: &str) -> Result<Duration, humantime::DurationError> {
    humantime::parse_duration(s)
}

pub fn record_file_path(data_dir: &Path, date: &NaiveDate, fmt: &str) -> PathBuf {
    let ext = if fmt == "csv" { "csv" } else { "jsonl" };
    let date_str = date.format("%Y-%m-%d").to_string();
    data_dir.join(format!("{date_str}.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_expected_paths() {
        let base = PathBuf::from("/tmp/data");
        let date = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        assert_eq!(
            record_file_path(&base, &date, "csv"),
            PathBuf::from("/tmp/data/2025-01-02.csv")
        );
        assert_eq!(
            record_file_path(&base, &date, "jsonl"),
            PathBuf::from("/tmp/data/2025-01-02.jsonl")
        );
    }
}
