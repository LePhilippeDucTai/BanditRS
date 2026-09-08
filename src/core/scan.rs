//! Per-file scanning pipeline (port of `BanditManager._parse_file` +
//! `_execute_ast_visitor`). See docs/spec/core.md §1.4. Status: implemented.
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

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use ruff_python_ast::token::TokenKind;
use ruff_text_size::Ranged;

use crate::ast::PyCompat;
use crate::ast::joined_str::ViewArena;
use crate::ast::walker::Walker;
use crate::core::issue::Issue;
use crate::core::metrics::{FileMetrics, Scores};
use crate::core::nosec::NosecLines;
use crate::core::test_set::TestSet;
use crate::core::tester::Tester;
use crate::log::Entry;
use crate::pycompat::encoding;
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
    pub logs: Vec<Entry>,
}

const SYNTAX_ERROR: &str = "syntax error while parsing AST from file";

type ScanOk = (FileMetrics, Scores, Vec<Issue>, Arc<SourceFile>, Vec<Entry>);

fn run_scan(name: &str, bytes: &[u8], test_set: &TestSet, ignore_nosec: bool, compat: PyCompat, mut metrics: FileMetrics) -> Result<ScanOk, String> {
    let text = encoding::decode_source(bytes).map_err(|_| SYNTAX_ERROR.to_string())?;
    let file = Arc::new(SourceFile::new(name, text));
    let parsed = crate::source::parse::parse_module(&file.text, compat).map_err(|_| SYNTAX_ERROR.to_string())?;

    let nosec = if ignore_nosec {
        NosecLines::default()
    } else {
        let comments = parsed
            .tokens()
            .iter()
            .filter(|t| t.kind() == TokenKind::Comment)
            .map(|t| (file.line_index(t.start().to_u32()), &file.text[t.range()]));
        NosecLines::from_comments(comments)
    };

    let arena = ViewArena::new();
    let mut tester = Tester { test_set, nosec: &nosec, metrics: &mut metrics, results: Vec::new(), source: file.clone() };
    let (scores, logs) = crate::log::with_buffer(|| {
        let mut walker = Walker::new(&file, &arena, compat, &mut tester);
        walker.process(parsed.syntax())
    });
    let results = std::mem::take(&mut tester.results);
    drop(tester);
    metrics.count_issues(&scores);
    Ok((metrics, scores, results, file, logs))
}

/// Scan the bytes of `name` (`BanditManager._parse_file` + `_execute_ast_visitor`).
pub fn scan_file(name: &str, bytes: &[u8], test_set: &TestSet, ignore_nosec: bool, compat: PyCompat) -> FileOutcome {
    let mut metrics = FileMetrics::default();
    metrics.count_locs(bytes);

    match catch_unwind(AssertUnwindSafe(|| run_scan(name, bytes, test_set, ignore_nosec, compat, metrics.clone()))) {
        Ok(Ok((metrics, scores, issues, source, logs))) => {
            FileOutcome { metrics, scores, issues, source: Some(source), skipped: None, logs }
        }
        Ok(Err(reason)) => FileOutcome { metrics, scores: Scores::default(), issues: Vec::new(), source: None, skipped: Some(reason), logs: Vec::new() },
        Err(_) => FileOutcome {
            metrics,
            scores: Scores::default(),
            issues: Vec::new(),
            source: None,
            skipped: Some("exception while scanning file".to_string()),
            logs: Vec::new(),
        },
    }
}
