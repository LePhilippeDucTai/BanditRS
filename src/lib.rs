//! BanditRS: a pure-Rust reimplementation of [bandit](https://github.com/PyCQA/bandit),
//! the security-oriented static analyser for Python code.
//!
//! The crate exposes the scanning engine as a library (used by the `bandit`,
//! `bandit-baseline` and `bandit-config-generator` binaries) and reproduces the
//! observable behaviour of the Python implementation: same tests, same results,
//! same output formats and exit codes.

pub mod ast;
pub mod cli;
pub mod constants;
pub mod core;
pub mod formatters;
pub mod log;
pub mod plugins;
pub mod pycompat;
pub mod source;

/// Documentation version segment used in `more_info` URLs.
pub const DOCS_VERSION: &str = "latest";

/// Version string reported by `--version`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Author reported in SARIF output (matches the upstream project metadata).
pub const AUTHOR: &str = "PyCQA";
