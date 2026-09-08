//! `bandit-baseline` (port of `bandit/cli/baseline.py`) — see
//! docs/spec/cli_formatters_tests.md §A.13. Status: stub (M8).
//!
//! Git is driven through the `git` CLI (`rev-parse --show-toplevel`,
//! `status --porcelain` for `is_dirty`, `rev-parse HEAD` / `HEAD^`,
//! `name-rev --name-only`, `reset --hard <commit>`), the inner scans call
//! the `bandit` executable next to the current one (fallback: PATH).

/// Entry point; returns the exit code.
pub fn main(_args: Vec<String>) -> i32 {
    todo!("M8: bandit-baseline")
}
