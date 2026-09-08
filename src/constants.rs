//! Constants shared by the whole crate (port of `bandit/core/constants.py`).

use std::fmt;

/// Severity / confidence ranking. The discriminant is the position in
/// bandit's `RANKING` list, which makes ordering comparisons meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u8)]
pub enum Rank {
    #[default]
    Undefined = 0,
    Low = 1,
    Medium = 2,
    High = 3,
}

impl Rank {
    /// All ranks, in `RANKING` order.
    pub const ALL: [Rank; 4] = [Rank::Undefined, Rank::Low, Rank::Medium, Rank::High];

    /// Upper-case name, e.g. `"MEDIUM"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Rank::Undefined => "UNDEFINED",
            Rank::Low => "LOW",
            Rank::Medium => "MEDIUM",
            Rank::High => "HIGH",
        }
    }

    /// Capitalised name, e.g. `"Medium"` (Python's `str.capitalize`).
    pub const fn capitalize(self) -> &'static str {
        match self {
            Rank::Undefined => "Undefined",
            Rank::Low => "Low",
            Rank::Medium => "Medium",
            Rank::High => "High",
        }
    }

    /// Lower-case name, e.g. `"medium"`.
    pub const fn lower(self) -> &'static str {
        match self {
            Rank::Undefined => "undefined",
            Rank::Low => "low",
            Rank::Medium => "medium",
            Rank::High => "high",
        }
    }

    /// Weight used by bandit when accumulating scores (`RANKING_VALUES`).
    pub const fn value(self) -> u64 {
        match self {
            Rank::Undefined => 1,
            Rank::Low => 3,
            Rank::Medium => 5,
            Rank::High => 10,
        }
    }

    /// Index in `RANKING`.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Inverse of [`Rank::index`].
    pub const fn from_index(index: usize) -> Option<Rank> {
        match index {
            0 => Some(Rank::Undefined),
            1 => Some(Rank::Low),
            2 => Some(Rank::Medium),
            3 => Some(Rank::High),
            _ => None,
        }
    }

    /// Parse an upper-case rank name (exact match, like `RANKING.index`).
    pub fn parse(name: &str) -> Option<Rank> {
        match name {
            "UNDEFINED" => Some(Rank::Undefined),
            "LOW" => Some(Rank::Low),
            "MEDIUM" => Some(Rank::Medium),
            "HIGH" => Some(Rank::High),
            _ => None,
        }
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `RANKING` from bandit.
pub const RANKING: [&str; 4] = ["UNDEFINED", "LOW", "MEDIUM", "HIGH"];

/// `CRITERIA` from bandit: the two score dimensions.
pub const CRITERIA: [&str; 2] = ["SEVERITY", "CONFIDENCE"];

/// Default plugin file name pattern.
pub const PLUGIN_NAME_PATTERN: &str = "*.py";

/// Default log format (`log_format_string`).
pub const LOG_FORMAT_STRING: &str = "[%(module)s]\t%(levelname)s\t%(message)s";

/// Paths excluded by default (`constants.EXCLUDE`).
pub const EXCLUDE: [&str; 9] = [
    ".svn",
    "CVS",
    ".bzr",
    ".hg",
    ".git",
    "__pycache__",
    ".tox",
    ".eggs",
    "*.egg",
];

/// Number of files above which bandit shows a progress bar (informational).
pub const PROGRESS_THRESHOLD: usize = 50;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_roundtrip() {
        for (i, r) in Rank::ALL.iter().enumerate() {
            assert_eq!(r.index(), i);
            assert_eq!(Rank::from_index(i), Some(*r));
            assert_eq!(Rank::parse(r.as_str()), Some(*r));
            assert_eq!(RANKING[i], r.as_str());
        }
        assert!(Rank::Low < Rank::High);
        assert_eq!(Rank::High.value(), 10);
        assert_eq!(Rank::Medium.capitalize(), "Medium");
        assert_eq!(Rank::parse("low"), None);
    }
}
