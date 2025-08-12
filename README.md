tiny-reporter
=================

A tiny CLI that periodically runs shell commands and records their output to CSV or JSONL. The binary name is `trep`.

Install
-------
- Build locally:
  - `cargo build --release`
  - Binary at `target/release/trep`
- Or install from source in this repo:
  - `cargo install --path .`

Usage
-----
Basic structure:

```
trep run --as <name> [--every <dur>] [--format csv|jsonl] [--timeout <dur>] -- <command>
```

Examples:
- Run once and log:
  - `trep run --as hello -- echo "hello world"`
- Run every 10 seconds, write JSONL:
  - `trep run --as cpu --every 10s --format jsonl -- sh -c "ps -A -o %cpu | awk '{s+=$1} END {print s}'"`
- Set a timeout of 5 seconds per run:
  - `trep run --as slow --every 1m --timeout 5s -- ./script_that_might_hang`

Options:
- `--as, -n <name>`: Job name; used in directory and file names (required).
- `--every <dur>`: Interval like `10s`, `1m`; if omitted, runs once.
- `--format <fmt>`: `csv` (default) or `jsonl`.
- `--timeout <dur>`: Per-run timeout like `5s`.
- `--` then the command to execute.

Output Location
---------------
- Base dir: `~/.tiny-reporter/` (fallback: `./.tiny-reporter/`).
- Job dir: `~/.tiny-reporter/<name>/`.
- File name: `<YYYY-MM-DD>.csv` or `.jsonl`.
- Lock file: `~/.tiny-reporter/<name>/<name>.lock` prevents concurrent runs.

Notes
-----
- Shell used: Unix uses `bash -lc`, Windows uses `cmd /C`.
- On timeout, the process is terminated by PID (Windows `taskkill`, Unix `kill -9`).

Development
-----------
- Format and build: `cargo fmt && cargo build`
- Run tests (if added later): `cargo test`

Records
-------
- CSV rows: `timestamp,value,exit_code` (no header row is written).
- JSONL lines: objects with fields `timestamp` (RFC3339), `value` (string), `exit_code` (number).
- Rotation: one file per day; file name is the UTC/local date formatted as `YYYY-MM-DD` plus the chosen extension.

Contributing
------------
- Local checks: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --all-targets`, `cargo test`.
- CI: GitHub Actions runs fmt, clippy (deny warnings), build, and tests on pushes and PRs.
- Optional pre-push hook: to automatically run checks before pushing, add this as `.git/hooks/pre-push` and make it executable:

```
#!/usr/bin/env bash
set -euo pipefail
echo "pre-push: verifying Rust project (fmt, clippy, build, test)"
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo build --all-targets
cargo test
```
