//! Port of `tests/unit/formatters/test_screen.py` (`bandit.formatters.screen`).
//! Work package: `docs/plan/wp/WP-12-unit-formatters-text-screen.md`.
//!
//! `screen::report` currently prints straight to stdout; WP-12 makes it
//! testable (render into an `impl Write`, or expose `get_results`/`header`
//! and assert on the assembled string) before porting the four upstream tests.

mod common;

use banditrs::constants::Rank;
use banditrs::formatters::screen;
use common::formatters::base_manager;

/// Extra (no upstream equivalent): with `-o <file>` the screen formatter only
/// logs `Screen formatter output was not written to file: <name>, consider '-f txt'`.
#[test]
fn extra_output_file_hint() {
    let mut mgr = base_manager();
    mgr.quiet = true; // avoid touching real stdout with ANSI issue output
    let (_lock, entries) = banditrs::log::with_buffer(|| {
        banditrs::log::set_level(banditrs::log::Level::Info);
        screen::report(&mgr, Rank::Low, Rank::Low, -1, Some("report.txt")).unwrap();
    });
    assert!(entries.iter().any(|e| {
        e.message
            .contains("Screen formatter output was not written to file: report.txt")
    }));
}

/// Port of `tests/unit/formatters/test_screen.py::ScreenFormatterTests::test_no_issues`.
#[test]
#[ignore = "WP-12: not ported yet — see docs/plan/wp/WP-12-unit-formatters-text-screen.md"]
fn test_no_issues() {
    unimplemented!(
        "WP-12: port tests/unit/formatters/test_screen.py::ScreenFormatterTests::test_no_issues"
    );
}

/// Port of `tests/unit/formatters/test_screen.py::ScreenFormatterTests::test_output_issue`.
#[test]
#[ignore = "WP-12: not ported yet — see docs/plan/wp/WP-12-unit-formatters-text-screen.md"]
fn test_output_issue() {
    unimplemented!(
        "WP-12: port tests/unit/formatters/test_screen.py::ScreenFormatterTests::test_output_issue"
    );
}

/// Port of `tests/unit/formatters/test_screen.py::ScreenFormatterTests::test_report_baseline`.
#[test]
#[ignore = "WP-12: not ported yet — see docs/plan/wp/WP-12-unit-formatters-text-screen.md"]
fn test_report_baseline() {
    unimplemented!(
        "WP-12: port tests/unit/formatters/test_screen.py::ScreenFormatterTests::test_report_baseline"
    );
}

/// Port of `tests/unit/formatters/test_screen.py::ScreenFormatterTests::test_report_nobaseline`.
#[test]
#[ignore = "WP-12: not ported yet — see docs/plan/wp/WP-12-unit-formatters-text-screen.md"]
fn test_report_nobaseline() {
    unimplemented!(
        "WP-12: port tests/unit/formatters/test_screen.py::ScreenFormatterTests::test_report_nobaseline"
    );
}
