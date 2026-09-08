//! Shared fixture for the formatter tests — port of the common `setUp` of
//! `tests/unit/formatters/*.py`: a `Manager` built from the default
//! configuration plus one `hardcoded_bind_all_interfaces` issue
//! (`fname = <tmpfile>`, lineno 4, linerange `[4]`, severity/confidence
//! MEDIUM, CWE 605 `MULTIPLE_BINDS`, col 8, end col 16).
//!
//! Owned by WP-13 (docs/plan/wp/WP-13-unit-formatters-structured.md). Other
//! work packages must not edit this file — add local helpers to your own test
//! file instead.

use banditrs::constants::Rank;
use banditrs::core::config::BanditConfig;
use banditrs::core::issue::{Cwe, Issue, LineRange};
use banditrs::core::manager::{AggType, Manager};
use banditrs::core::test_set::TestSet;

pub const TEXT: &str = "Possible binding to all interfaces.";
pub const TEST_NAME: &str = "hardcoded_bind_all_interfaces";
pub const TEST_ID: &str = "B104";

/// `BanditManager(BanditConfig(), "file")`.
pub fn base_manager() -> Manager {
    let config = BanditConfig::default();
    let profile = config.default_profile();
    let test_set = TestSet::new(&config, &profile);
    Manager::new(config, AggType::File, test_set)
}

/// The issue of the Python `setUp` (`Issue(MEDIUM, Cwe.MULTIPLE_BINDS, MEDIUM, text)`
/// with `fname`, `lineno`, `linerange = [lineno]`, `col_offset`, `end_col_offset`).
pub fn make_issue(fname: &str, lineno: u32, col: u32, end_col: u32) -> Issue {
    let mut issue = Issue::new(
        Rank::Medium,
        Rank::Medium,
        Cwe::MULTIPLE_BINDS,
        TEXT,
        fname,
        TEST_NAME,
        TEST_ID,
        lineno,
    );
    issue.linerange = LineRange::single(lineno);
    issue.col_offset = col;
    issue.end_col_offset = end_col;
    issue
}

/// `tempfile.mkstemp()`: the file handle (kept alive) and its path.
pub fn tmp_name() -> (tempfile::NamedTempFile, String) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let name = tmp.path().to_string_lossy().into_owned();
    (tmp, name)
}
