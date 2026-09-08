//! Runs the tests of a node and applies the `# nosec` gate (port of
//! `bandit/core/tester.py`). See docs/spec/core.md §4. Status: implemented.
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

use std::borrow::Cow;

use rustc_hash::FxHashSet;

use crate::ast::NodeKind;
use crate::ast::walker::TestRunner;
use crate::core::blacklist;
use crate::core::context::Context;
use crate::core::issue::{Issue, LineRange};
use crate::core::metrics::{FileMetrics, Scores};
use crate::core::nosec::NosecLines;
use crate::core::test_set::{TestRef, TestSet};

/// The tester for one file.
pub struct Tester<'t> {
    pub test_set: &'t TestSet,
    pub nosec: &'t NosecLines,
    pub metrics: &'t mut FileMetrics,
    pub results: Vec<Issue>,
    /// Shared source text attached to every reported issue (for snippets).
    pub source: std::sync::Arc<crate::source::SourceFile>,
}

/// `_get_nosecs_from_contexts`: `base` (only when a result carries an explicit
/// lineno) unioned with the first non-`None` entry over `linerange`; both
/// absent → `None`.
fn combined_nosec(nosec: &NosecLines, draft_lineno: Option<u32>, linerange: LineRange) -> Option<FxHashSet<String>> {
    let base = draft_lineno.and_then(|l| nosec.get(l));
    let ctx_tests = nosec.for_range(linerange);
    match (base, ctx_tests) {
        (None, None) => None,
        (Some(a), None) => Some(a.clone()),
        (None, Some(b)) => Some(b.clone()),
        (Some(a), Some(b)) => Some(a.union(b).cloned().collect()),
    }
}

impl<'a, 't> TestRunner<'a> for Tester<'t> {
    fn wants(&self, kind: NodeKind) -> bool {
        !self.test_set.get_tests(kind).is_empty()
    }

    fn run_tests(&mut self, ctx: &Context<'a, '_>, kind: NodeKind) -> Scores {
        let mut scores = Scores::default();
        for test_ref in self.test_set.get_tests(kind) {
            let (name, default_id, result) = match test_ref {
                TestRef::Plugin(p) => (p.func_name, p.id, (p.func)(ctx, &self.test_set.configs)),
                TestRef::Blacklist => {
                    let table = self.test_set.blacklist.as_ref().expect("B001 registered without a table");
                    ("blacklist", "B001", blacklist::blacklist(ctx, kind, table))
                }
            };
            match result {
                Err(e) => {
                    crate::log_error!(
                        "tester",
                        "Bandit internal error running: {} on file {} at line {}: {}",
                        name,
                        ctx.filename(),
                        ctx.lineno().unwrap_or(0),
                        e
                    );
                }
                Ok(None) => {
                    if let Some(ids) = combined_nosec(self.nosec, None, ctx.linerange) {
                        if !ids.is_empty() && ids.contains(default_id) {
                            crate::log_warning!(
                                "tester",
                                "nosec encountered ({}), but no failed test on file {}:{}",
                                default_id,
                                ctx.filename(),
                                ctx.lineno().unwrap_or(0)
                            );
                        }
                    }
                }
                Ok(Some(draft)) => {
                    let nosec_ids = combined_nosec(self.nosec, draft.lineno, ctx.linerange);
                    let lineno = draft.lineno.unwrap_or_else(|| ctx.lineno().unwrap_or(0));
                    let col_offset = draft.col_offset.unwrap_or_else(|| ctx.col_offset().unwrap_or(0));
                    let linerange = draft.linerange.unwrap_or(ctx.linerange);
                    let end_col_offset = ctx.end_col_offset();
                    let test_id = draft.test_id.clone().unwrap_or(Cow::Borrowed(default_id));
                    let report = match &nosec_ids {
                        None => true,
                        Some(ids) if ids.is_empty() => {
                            self.metrics.nosec += 1;
                            false
                        }
                        Some(ids) if ids.contains(test_id.as_ref()) => {
                            self.metrics.skipped_tests += 1;
                            false
                        }
                        Some(_) => true,
                    };
                    if report {
                        let issue = Issue {
                            severity: draft.severity,
                            confidence: draft.confidence,
                            cwe: draft.cwe,
                            text: draft.text,
                            ident: draft.ident,
                            fname: ctx.filename().to_string(),
                            test: Cow::Borrowed(name),
                            test_id,
                            lineno,
                            col_offset,
                            end_col_offset,
                            linerange,
                            source: Some(self.source.clone()),
                        };
                        scores.note(issue.severity, issue.confidence);
                        self.results.push(issue);
                    }
                }
            }
        }
        scores
    }
}
