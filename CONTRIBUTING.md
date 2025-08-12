Contributing
============

Thank you for your interest in contributing!

Local workflow
--------------
- Format: `cargo fmt --all -- --check`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Build: `cargo build --all-targets`
- Test: `cargo test`

Pre-push hook (optional)
------------------------
To automatically run the checks before you push:

1. Create `.git/hooks/pre-push` with:

   ```bash
   #!/usr/bin/env bash
   set -euo pipefail
   echo "pre-push: verifying Rust project (fmt, clippy, build, test)"
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo build --all-targets
   cargo test
   ```

2. Make it executable: `chmod +x .git/hooks/pre-push`.

If needed, bypass temporarily with `GIT_SKIP_HOOKS=1 git push`.

CI
--
GitHub Actions (`.github/workflows/ci.yml`) runs the same checks on pushes to `main` and on pull requests.

