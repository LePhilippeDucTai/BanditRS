//! Metric gathering (port of `bandit/core/metrics.py`).

use indexmap::IndexMap;
use serde_json::{Map, Value};

use crate::constants::{CRITERIA, RANKING, Rank};
use crate::pycompat::splitlines::{bytes_splitlines, bytes_strip};

/// Weighted scores accumulated for one file (`SEVERITY` / `CONFIDENCE`
/// vectors indexed by rank, weighted by `RANKING_VALUES`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Scores {
    pub severity: [u64; 4],
    pub confidence: [u64; 4],
}

impl Scores {
    /// Account for one reported issue.
    pub fn note(&mut self, severity: Rank, confidence: Rank) {
        self.severity[severity.index()] += severity.value();
        self.confidence[confidence.index()] += confidence.value();
    }

    /// Element-wise addition (`update_scores`).
    pub fn add(&mut self, other: &Scores) {
        for i in 0..4 {
            self.severity[i] += other.severity[i];
            self.confidence[i] += other.confidence[i];
        }
    }

    /// Issue counts per criteria and rank (`_get_issue_counts`).
    pub fn issue_counts(&self) -> [[u64; 4]; 2] {
        let mut out = [[0u64; 4]; 2];
        for (i, rank) in Rank::ALL.iter().enumerate() {
            out[0][i] = self.severity[i] / rank.value();
            out[1][i] = self.confidence[i] / rank.value();
        }
        out
    }

    /// Sum of the severity vector (used by the verbose file listing).
    pub fn severity_total(&self) -> u64 {
        self.severity.iter().sum()
    }

    /// Sum of the confidence vector.
    pub fn confidence_total(&self) -> u64 {
        self.confidence.iter().sum()
    }
}

/// Metrics of a single file (or the totals).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileMetrics {
    pub loc: u64,
    pub nosec: u64,
    pub skipped_tests: u64,
    /// `[SEVERITY counts, CONFIDENCE counts]`, absent for files whose scan
    /// did not complete (syntax errors).
    pub issues: Option<[[u64; 4]; 2]>,
}

impl FileMetrics {
    /// Count lines of code: non-blank lines whose first non-space byte is
    /// not `#`.
    pub fn count_locs(&mut self, data: &[u8]) {
        self.loc += bytes_splitlines(data)
            .filter(|line| {
                let t = bytes_strip(line);
                !t.is_empty() && !t.starts_with(b"#")
            })
            .count() as u64;
    }

    /// Record the issue counts of a completed scan.
    pub fn count_issues(&mut self, scores: &Scores) {
        self.issues = Some(scores.issue_counts());
    }

    /// Value of a metric label (`loc`, `nosec`, `skipped_tests`,
    /// `SEVERITY.LOW`, ...).
    pub fn get(&self, label: &str) -> Option<u64> {
        match label {
            "loc" => Some(self.loc),
            "nosec" => Some(self.nosec),
            "skipped_tests" => Some(self.skipped_tests),
            _ => {
                let (criteria, rank) = label.split_once('.')?;
                let c = CRITERIA.iter().position(|c| *c == criteria)?;
                let r = Rank::parse(rank)?;
                self.issues.map(|i| i[c][r.index()])
            }
        }
    }

    /// Per-file JSON object in bandit's key order.
    fn to_json_file(&self) -> Value {
        let mut m = Map::new();
        m.insert("loc".into(), Value::from(self.loc));
        m.insert("nosec".into(), Value::from(self.nosec));
        m.insert("skipped_tests".into(), Value::from(self.skipped_tests));
        if let Some(issues) = self.issues {
            for (c, criteria) in CRITERIA.iter().enumerate() {
                for (r, rank) in RANKING.iter().enumerate() {
                    m.insert(format!("{criteria}.{rank}"), Value::from(issues[c][r]));
                }
            }
        }
        Value::Object(m)
    }

    /// Totals JSON object (interleaved key order, as bandit initialises it).
    fn to_json_totals(&self) -> Value {
        let mut m = Map::new();
        m.insert("loc".into(), Value::from(self.loc));
        m.insert("nosec".into(), Value::from(self.nosec));
        m.insert("skipped_tests".into(), Value::from(self.skipped_tests));
        let issues = self.issues.unwrap_or_default();
        for (r, rank) in RANKING.iter().enumerate() {
            for (c, criteria) in CRITERIA.iter().enumerate() {
                m.insert(format!("{criteria}.{rank}"), Value::from(issues[c][r]));
            }
        }
        Value::Object(m)
    }
}

/// All metrics of a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metrics {
    pub files: IndexMap<String, FileMetrics>,
    pub totals: FileMetrics,
}

impl Default for Metrics {
    fn default() -> Metrics {
        Metrics::new()
    }
}

impl Metrics {
    pub fn new() -> Metrics {
        Metrics {
            files: IndexMap::new(),
            totals: FileMetrics { issues: Some([[0; 4]; 2]), ..FileMetrics::default() },
        }
    }

    /// Start (or restart) the metric block of `fname`.
    pub fn begin(&mut self, fname: &str) -> &mut FileMetrics {
        self.files.insert(fname.to_string(), FileMetrics::default());
        self.files.get_mut(fname).expect("just inserted")
    }

    /// Store a completed metric block.
    pub fn insert(&mut self, fname: &str, metrics: FileMetrics) {
        self.files.insert(fname.to_string(), metrics);
    }

    /// Recompute the totals from the per-file blocks.
    pub fn aggregate(&mut self) {
        let mut totals = FileMetrics { issues: Some([[0; 4]; 2]), ..FileMetrics::default() };
        for fm in self.files.values() {
            totals.loc += fm.loc;
            totals.nosec += fm.nosec;
            totals.skipped_tests += fm.skipped_tests;
            if let Some(issues) = fm.issues {
                let t = totals.issues.as_mut().expect("totals always carry counts");
                for c in 0..2 {
                    for r in 0..4 {
                        t[c][r] += issues[c][r];
                    }
                }
            }
        }
        self.totals = totals;
    }

    /// `metrics.data` as JSON, `_totals` first then files in scan order.
    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("_totals".into(), self.totals.to_json_totals());
        for (name, fm) in &self.files {
            m.insert(name.clone(), fm.to_json_file());
        }
        Value::Object(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_locs_skips_blank_and_comments() {
        let mut fm = FileMetrics::default();
        fm.count_locs(b"import os\n\n   # comment\n  x = 1 # trailing\n\t\n");
        assert_eq!(fm.loc, 2);
    }

    #[test]
    fn scores_and_totals() {
        let mut s = Scores::default();
        s.note(Rank::Low, Rank::High);
        s.note(Rank::Low, Rank::High);
        s.note(Rank::High, Rank::Medium);
        assert_eq!(s.severity, [0, 6, 0, 10]);
        assert_eq!(s.issue_counts(), [[0, 2, 0, 1], [0, 0, 1, 2]]);
        let mut m = Metrics::new();
        let fm = m.begin("a.py");
        fm.loc = 3;
        fm.nosec = 1;
        fm.count_issues(&s);
        m.begin("b.py").loc = 4;
        m.aggregate();
        assert_eq!(m.totals.loc, 7);
        assert_eq!(m.totals.nosec, 1);
        assert_eq!(m.totals.get("SEVERITY.LOW"), Some(2));
        assert_eq!(m.totals.get("CONFIDENCE.HIGH"), Some(2));
        let j = m.to_json();
        let keys: Vec<&String> = j["_totals"].as_object().unwrap().keys().collect();
        assert_eq!(keys[3], "SEVERITY.UNDEFINED");
        assert_eq!(keys[4], "CONFIDENCE.UNDEFINED");
        assert!(j["b.py"].get("SEVERITY.LOW").is_none());
        assert_eq!(j["a.py"]["SEVERITY.LOW"], 2);
    }
}
