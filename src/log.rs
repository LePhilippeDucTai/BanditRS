//! Minimal logger reproducing bandit's logging output.
//!
//! Python bandit configures the root logger with the format
//! `"[%(module)s]\t%(levelname)s\t%(message)s"` writing to stderr. This module
//! reproduces that output, supports the `log_format` config override for the
//! common `%(...)s` fields, and offers a thread-local buffer so that scans
//! running in parallel can emit their log lines in a deterministic order.

use std::cell::RefCell;
use std::fmt;
use std::io::Write;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

/// Log levels (values mirror Python's `logging` numeric levels / 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Level {
    Debug = 1,
    Info = 2,
    Warning = 3,
    Error = 4,
    Critical = 5,
}

impl Level {
    pub const fn name(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warning => "WARNING",
            Level::Error => "ERROR",
            Level::Critical => "CRITICAL",
        }
    }

    fn from_u8(v: u8) -> Level {
        match v {
            1 => Level::Debug,
            2 => Level::Info,
            3 => Level::Warning,
            4 => Level::Error,
            _ => Level::Critical,
        }
    }
}

/// Python's root logger defaults to WARNING until `_init_logger` runs.
static LEVEL: AtomicU8 = AtomicU8::new(Level::Warning as u8);
static FORMAT: OnceLock<Mutex<String>> = OnceLock::new();
/// `bandit` logs to stderr; `bandit-baseline`/`bandit-config-generator` set
/// up a `StreamHandler(sys.stdout)` instead.
static USE_STDOUT: AtomicU8 = AtomicU8::new(0);

/// Direct log output to stdout instead of the default stderr (used by the
/// `bandit-baseline`/`bandit-config-generator` binaries, which configure
/// their own stdout handler).
pub fn set_stdout(use_stdout: bool) {
    USE_STDOUT.store(use_stdout as u8, Ordering::Relaxed);
}

/// A buffered log entry: `(module, level, message)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub module: &'static str,
    pub level: Level,
    pub message: String,
}

thread_local! {
    static SINK: RefCell<Option<Vec<Entry>>> = const { RefCell::new(None) };
}

/// Set the global threshold level.
pub fn set_level(level: Level) {
    LEVEL.store(level as u8, Ordering::Relaxed);
}

/// Current threshold level.
pub fn level() -> Level {
    Level::from_u8(LEVEL.load(Ordering::Relaxed))
}

/// Whether a message at `level` would be emitted.
pub fn enabled(level: Level) -> bool {
    level >= level_threshold()
}

fn level_threshold() -> Level {
    level()
}

/// Override the log format (`log_format` config option). Supports the
/// `%(module)s`, `%(levelname)s`, `%(message)s` and `%(name)s` fields.
pub fn set_format(format: &str) {
    let m = FORMAT.get_or_init(|| Mutex::new(crate::constants::LOG_FORMAT_STRING.to_string()));
    *m.lock().unwrap_or_else(|e| e.into_inner()) = format.to_string();
}

fn format_entry(module: &str, level: Level, message: &str) -> String {
    let default = crate::constants::LOG_FORMAT_STRING;
    let fmt_guard = FORMAT
        .get()
        .map(|m| m.lock().unwrap_or_else(|e| e.into_inner()));
    let fmt = fmt_guard.as_deref().map(String::as_str).unwrap_or(default);
    if fmt == default {
        return format!("[{module}]\t{}\t{message}", level.name());
    }
    substitute_log_format(fmt, module, level, message)
}

/// `"%(name)s"` / `"%(name)5s"` (right-justified, width 5) style
/// substitution for `module`, `name`, `levelname`, `message`.
fn substitute_log_format(fmt: &str, module: &str, level: Level, message: &str) -> String {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"%\((\w+)\)(\d*)s").unwrap());
    re.replace_all(fmt, |caps: &regex::Captures| {
        let value = match &caps[1] {
            "module" => module,
            "name" => {
                if module == "main" {
                    "root"
                } else {
                    module
                }
            }
            "levelname" => level.name(),
            "message" => message,
            _ => "",
        };
        match caps[2].parse::<usize>() {
            Ok(width) => format!("{value:>width$}"),
            Err(_) => value.to_string(),
        }
    })
    .into_owned()
}

/// Emit a log record. When a thread-local buffer is active (see
/// [`with_buffer`]) the record is stored instead of being written.
pub fn log(module: &'static str, level: Level, args: fmt::Arguments<'_>) {
    if !enabled(level) {
        return;
    }
    let message = args.to_string();
    let buffered = SINK.with(|sink| {
        if let Some(buf) = sink.borrow_mut().as_mut() {
            buf.push(Entry {
                module,
                level,
                message: message.clone(),
            });
            true
        } else {
            false
        }
    });
    if !buffered {
        write_entry(module, level, &message);
    }
}

/// Write a record directly to stderr (or stdout, see [`set_stdout`]).
pub fn write_entry(module: &str, level: Level, message: &str) {
    let line = format_entry(module, level, message);
    if USE_STDOUT.load(Ordering::Relaxed) != 0 {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        let _ = writeln!(lock, "{line}");
    } else {
        let stderr = std::io::stderr();
        let mut lock = stderr.lock();
        let _ = writeln!(lock, "{line}");
    }
}

/// Run `f` with a thread-local buffer collecting every record emitted on this
/// thread; returns the collected records together with `f`'s result.
pub fn with_buffer<T>(f: impl FnOnce() -> T) -> (T, Vec<Entry>) {
    let previous = SINK.with(|sink| sink.borrow_mut().replace(Vec::new()));
    let result = f();
    let entries = SINK.with(|sink| {
        let mut sink = sink.borrow_mut();
        let entries = sink.take().unwrap_or_default();
        *sink = previous;
        entries
    });
    (result, entries)
}

/// Flush buffered records (typically in file order after a parallel scan).
pub fn flush_entries(entries: &[Entry]) {
    for e in entries {
        if enabled(e.level) {
            write_entry(e.module, e.level, &e.message);
        }
    }
}

#[macro_export]
macro_rules! log_debug {
    ($module:expr, $($arg:tt)*) => { $crate::log::log($module, $crate::log::Level::Debug, format_args!($($arg)*)) };
}
#[macro_export]
macro_rules! log_info {
    ($module:expr, $($arg:tt)*) => { $crate::log::log($module, $crate::log::Level::Info, format_args!($($arg)*)) };
}
#[macro_export]
macro_rules! log_warning {
    ($module:expr, $($arg:tt)*) => { $crate::log::log($module, $crate::log::Level::Warning, format_args!($($arg)*)) };
}
#[macro_export]
macro_rules! log_error {
    ($module:expr, $($arg:tt)*) => { $crate::log::log($module, $crate::log::Level::Error, format_args!($($arg)*)) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_collects_entries() {
        set_level(Level::Info);
        let ((), entries) = with_buffer(|| {
            crate::log_info!("manager", "hello {}", 1);
            crate::log_debug!("manager", "hidden");
        });
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "hello 1");
        assert_eq!(entries[0].module, "manager");
    }

    #[test]
    fn default_format() {
        assert_eq!(
            format_entry("manager", Level::Warning, "msg"),
            "[manager]\tWARNING\tmsg"
        );
    }
}
