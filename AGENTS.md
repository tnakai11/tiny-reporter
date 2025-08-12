# Repository Guidelines

This repository contains a small Rust CLI called `trep` (tiny-reporter). It periodically runs shell commands and records their output to CSV or JSONL.

## Project Structure & Module Organization
- `src/main.rs`: CLI (`clap`), scheduler, IO, and record writing.
- `src/util.rs`: helpers for file paths; includes unit tests.
- `.github/workflows/`: CI for fmt, clippy, build, test, and release.
- Output at runtime: `~/.tiny-reporter/<name>/` with daily files and a lock file.

## Build, Test, and Development Commands
- Build: `cargo build --all-targets` (release: `cargo build --release`).
- Run: `cargo run -- run --as demo -- echo hello` or use the built binary `trep`.
- Format check: `cargo fmt --all -- --check` (format in-place: `cargo fmt`).
- Lint: `cargo clippy --all-targets -- -D warnings` (CI fails on warnings).
- Test: `cargo test` (verbose: `cargo test -- --nocapture`).

## Coding Style & Naming Conventions
- Edition: Rust 2021; use `rustfmt` defaults (4-space indent, max width per toolchain).
- Naming: `snake_case` for functions/modules, `UpperCamelCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Imports and modules: group std/external/internal; prefer explicit `use` over glob imports.
- Keep public surface minimal; add docs to public items when introduced.

## Testing Guidelines
- Framework: built-in Rust test harness (`#[cfg(test)]` modules).
- Location: colocate unit tests with modules (see `src/util.rs`, `src/main.rs`).
- Style: name tests for behavior, e.g., `duration_parse_valid`, `write_csv_and_jsonl`.
- Running: `cargo test`; add tests for parsing, path building, and record writing.

## Commit & Pull Request Guidelines
- Commits: use Conventional Commits (used by Release Please):
  - `feat(scope): ...` adds a feature; `fix(scope): ...` for bug fixes.
  - Use `feat!:` or include `BREAKING CHANGE:` for breaking changes.
- PRs: include a clear description, linked issues (e.g., `Closes #123`), and any relevant screenshots/logs. Ensure CI is green (fmt, clippy, build, tests).
- PR template: `.github/pull_request_template.md` (complete Summary, Changes, Testing, and Checklist sections).

## Security & Configuration Tips
- Shell execution: on Unix uses `bash -lc`; on Windows uses `cmd /C`. Quote commands appropriately.
- Filesystem: data written under `~/.tiny-reporter/`; one process per `name` guarded by `<name>.lock`.
- Timeouts: prefer `--timeout` to prevent hung commands; intervals via `--every` (e.g., `10s`, `1m`).
