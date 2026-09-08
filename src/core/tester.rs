//! Runs the tests of a node and applies the `# nosec` gate (port of
//! `bandit/core/tester.py`). See docs/spec/core.md §4. Status: stub (M3).
//!
//! `run_tests(context, kind)` for each test of `test_set.get_tests(kind)`:
//! 1. call the plugin (or `blacklist::blacklist`); `Err(PyErr)` →
//!    `log_error!("tester", "Bandit internal error running: {name} on file {fname} at line {lineno}: {err}")`
//!    and continue;
//! 2. on `Some(draft)`: `nosec_tests_to_skip = union(nosec.get(draft.lineno) if draft.lineno is Some, nosec.for_range(context.linerange))`
//!    (computed BEFORE the lineno default is applied); then build the
//!    `Issue`: fname = context file name, lineno = draft.lineno or
//!    context.lineno, linerange = draft.linerange or context.linerange,
//!    col_offset = draft.col_offset or context.col_offset (0 when the
//!    context has none), end_col_offset = context.end_col_offset() (always),
//!    test = plugin func_name (`"blacklist"` for B001), test_id =
//!    draft.test_id or plugin id;
//! 3. gate: both lookups `None` → report; empty set → skip and
//!    `metrics.nosec += 1`; contains test_id → skip and
//!    `metrics.skipped_tests += 1`; otherwise report;
//! 4. reported issues are pushed to `results` and `scores.note(sev, conf)`.
//! On `None` results, if the nosec set for the context names this test id
//! log `"nosec encountered ({id}), but no failed test on file {fname}:{lineno}"`
//! as a warning.

use crate::ast::NodeKind;
use crate::ast::walker::TestRunner;
use crate::core::context::Context;
use crate::core::issue::Issue;
use crate::core::metrics::{FileMetrics, Scores};
use crate::core::nosec::NosecLines;
use crate::core::test_set::TestSet;

/// The tester for one file.
pub struct Tester<'t> {
    pub test_set: &'t TestSet,
    pub nosec: &'t NosecLines,
    pub metrics: &'t mut FileMetrics,
    pub results: Vec<Issue>,
    /// Shared source text attached to every reported issue (for snippets).
    pub source: std::sync::Arc<crate::source::SourceFile>,
}

impl<'a, 't> TestRunner<'a> for Tester<'t> {
    fn wants(&self, kind: NodeKind) -> bool {
        !self.test_set.get_tests(kind).is_empty()
    }

    fn run_tests(&mut self, _ctx: &Context<'a, '_>, _kind: NodeKind) -> Scores {
        todo!("M3: Tester::run_tests")
    }
}
