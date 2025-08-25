use std::fs;
use std::process::Command;

use chrono::Local;
use tempfile::TempDir;

#[test]
fn csv_output_and_lock_in_temp_home() {
    let home = TempDir::new().unwrap();
    let name = "demo";
    let status = Command::new(env!("CARGO_BIN_EXE_trep"))
        .args([
            "run", "--as", name, "--format", "csv", "--", "echo", "hello",
        ])
        .env("HOME", home.path())
        .status()
        .expect("failed to run trep");
    assert!(status.success());

    let date = Local::now().format("%Y-%m-%d").to_string();
    let base = home.path().join(".tiny-reporter").join(name);
    let csv_path = base.join(format!("{date}.csv"));
    let lock_path = base.join(format!("{name}.lock"));
    assert!(csv_path.exists(), "missing csv file");
    assert!(lock_path.exists(), "missing lock file");

    let contents = fs::read_to_string(csv_path).unwrap();
    assert!(contents.contains("hello"));
}

#[test]
fn jsonl_output_in_temp_home() {
    let home = TempDir::new().unwrap();
    let name = "demo";
    let status = Command::new(env!("CARGO_BIN_EXE_trep"))
        .args(["run", "--as", name, "--format", "jsonl", "--", "echo", "hi"])
        .env("HOME", home.path())
        .status()
        .expect("failed to run trep");
    assert!(status.success());

    let date = Local::now().format("%Y-%m-%d").to_string();
    let base = home.path().join(".tiny-reporter").join(name);
    let jsonl_path = base.join(format!("{date}.jsonl"));
    let lock_path = base.join(format!("{name}.lock"));
    assert!(jsonl_path.exists(), "missing jsonl file");
    assert!(lock_path.exists(), "missing lock file");

    let contents = fs::read_to_string(jsonl_path).unwrap();
    assert!(contents.contains("hi"));
}
