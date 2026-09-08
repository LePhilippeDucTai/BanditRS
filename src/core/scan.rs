//! Per-file scanning pipeline (port of `BanditManager._parse_file` +
//! `_execute_ast_visitor`). See docs/spec/core.md §1.4. Status: stub (M5).
//!
//! Pipeline (order matters for the metrics of files that fail to parse):
//! 1. `metrics.count_locs(bytes)`;
//! 2. decode (`pycompat::encoding::decode_source`); failure → skipped with
//!    `"syntax error while parsing AST from file"` (metrics kept, no issue counts);
//! 3. `parse_module` (`source::parse`); failure → same skip reason;
//! 4. unless `ignore_nosec`, build `NosecLines` from the `Comment` tokens of
//!    `parsed.tokens()` (line = `file.line_index(token.start())`, text =
//!    `&file.text[token.range()]`);
//! 5. walk with `Tester` (`ast::walker::Walker`), then `metrics.count_issues(&scores)`;
//! 6. wrap 2–5 in `std::panic::catch_unwind` → `"exception while scanning file"`.

use std::sync::Arc;

use crate::ast::PyCompat;
use crate::core::issue::Issue;
use crate::core::metrics::{FileMetrics, Scores};
use crate::core::test_set::TestSet;
use crate::source::SourceFile;

/// Outcome of scanning one file.
pub struct FileOutcome {
    pub metrics: FileMetrics,
    pub scores: Scores,
    pub issues: Vec<Issue>,
    /// Kept when the file produced issues (for code snippets).
    pub source: Option<Arc<SourceFile>>,
    /// Skip reason when the scan did not complete.
    pub skipped: Option<String>,
    /// Buffered log records (flushed in file order by the manager).
    pub logs: Vec<crate::log::Entry>,
}

/// Scan the bytes of `name`.
pub fn scan_file(_name: &str, _bytes: &[u8], _test_set: &TestSet, _ignore_nosec: bool, _compat: PyCompat) -> FileOutcome {
    todo!("M5: scan_file")
}
