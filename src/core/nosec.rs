//! `# nosec` comment handling (port of `manager.py::_parse_nosec_comment`
//! and `utils.get_nosec`). See PLAN.md (M3) and docs/spec/core.md §1.5.
//!
//! Semantics to implement:
//! * Every `Comment` token of the file yields an entry keyed by its line:
//!   `None` when the comment is not a nosec comment, `Some(set)` otherwise.
//! * `NOSEC_COMMENT = r"#\s*nosec:?\s*(?P<tests>[^#]+)?#?"` (searched, not
//!   anchored: `# type: ... # nosec B607 # noqa` matches at the second `#`).
//!   No `tests` group → empty set (blanket nosec).
//! * Test tokens are taken from the `tests` group with
//!   `NOSEC_COMMENT_TESTS = r"(?:(B\d+|[a-z\d_]+),?)+"` (case-insensitive).
//!   Deliberate deviation (DEVIATIONS.md #1): split on commas/whitespace so
//!   that `B101,B102` (no space) yields both ids.
//! * Each token: a known id (`registry::check_id`) → kept; a known test or
//!   blacklist name (`registry::get_test_id`) → mapped to its id; otherwise
//!   `log_warning!("manager", "Test in comment: {} is not a test name or id, ignoring", token)`
//!   and dropped. An id set that ends up empty after dropping unknown tokens
//!   is still an *empty set* (blanket nosec).
//! * `get_nosec(lines, context)`: the first non-`None` entry over
//!   `context.linerange` (first hit wins, no union across lines).

use std::sync::LazyLock;

use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::issue::LineRange;

pub static NOSEC_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#\s*nosec:?\s*(?P<tests>[^#]+)?#?").unwrap());

/// Map: line number → `None` (plain comment) / `Some(ids)` (nosec).
#[derive(Debug, Clone, Default)]
pub struct NosecLines {
    pub lines: FxHashMap<u32, Option<FxHashSet<String>>>,
}

impl NosecLines {
    /// Build from the comment tokens of a parsed file: `comments` yields
    /// `(line, comment_text)` pairs (`text` starts with `#`).
    pub fn from_comments<'a>(comments: impl Iterator<Item = (u32, &'a str)>) -> NosecLines {
        let mut lines = FxHashMap::default();
        for (line, text) in comments {
            lines.insert(line, parse_nosec_comment(text));
        }
        NosecLines { lines }
    }

    /// `nosec_lines.get(lineno)`.
    pub fn get(&self, line: u32) -> Option<&FxHashSet<String>> {
        self.lines.get(&line).and_then(|o| o.as_ref())
    }

    /// `utils.get_nosec(nosec_lines, context)`: first non-`None` entry over
    /// the line range.
    pub fn for_range(&self, range: LineRange) -> Option<&FxHashSet<String>> {
        range.iter().find_map(|l| self.get(l))
    }
}

/// `_parse_nosec_comment(comment)`: `None` when the comment is not a nosec
/// comment, `Some(empty)` for a blanket nosec, `Some(ids)` otherwise.
pub fn parse_nosec_comment(comment: &str) -> Option<FxHashSet<String>> {
    let caps = NOSEC_COMMENT.captures(comment)?;
    let mut ids = FxHashSet::default();
    if let Some(tests) = caps.name("tests") {
        for token in tests.as_str().split(|c: char| c == ',' || c.is_whitespace()).filter(|t| !t.is_empty()) {
            if let Some(id) = crate::core::registry::resolve_test_token(token) {
                ids.insert(id);
            } else {
                crate::log_warning!("manager", "Test in comment: {} is not a test name or id, ignoring", token);
            }
        }
    }
    Some(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comments() {
        assert!(parse_nosec_comment("# noqa").is_none());
        assert_eq!(parse_nosec_comment("# nosec").unwrap().len(), 0);
        assert_eq!(parse_nosec_comment("#nosec  # noqa").unwrap().len(), 0);
        let ids = parse_nosec_comment("# nosec: B101").unwrap();
        assert!(ids.contains("B101") && ids.len() == 1);
        let ids = parse_nosec_comment("# nosec B101, B102").unwrap();
        assert!(ids.contains("B101") && ids.contains("B102"));
        let ids = parse_nosec_comment("# nosec B101,B102").unwrap();
        assert_eq!(ids.len(), 2, "deliberate deviation: comma without space keeps both ids");
        let ids = parse_nosec_comment("# type: ... # nosec B607 # noqa: E501").unwrap();
        assert!(ids.contains("B607") && ids.len() == 1);
        assert_eq!(parse_nosec_comment("#nosec (on the line)").unwrap().len(), 0);
        let ids = parse_nosec_comment("# nosec import_subprocess").unwrap();
        assert!(ids.contains("B404"));
    }
}
