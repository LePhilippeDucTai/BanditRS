//! The scan manager (port of `bandit/core/manager.py`). See docs/spec/core.md §1.
//! Status: implemented (`run_tests` scans files in parallel with `rayon`).

use std::sync::Arc;

use rayon::prelude::*;

use crate::ast::PyCompat;
use crate::constants::Rank;
use crate::core::config::BanditConfig;
use crate::core::issue::{BaselineIssue, Issue};
use crate::core::metrics::{Metrics, Scores};
use crate::core::test_set::TestSet;
use crate::source::SourceFile;

/// `-a/--aggregate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AggType {
    #[default]
    File,
    Vuln,
}

/// Issues selected for output: a plain list, or (with a baseline) the
/// unmatched issues each with its candidate matches (`_find_candidate_matches`).
pub enum IssueList<'m> {
    Plain(Vec<&'m Issue>),
    Baseline(Vec<(&'m Issue, Vec<&'m Issue>)>),
}

impl<'m> IssueList<'m> {
    pub fn len(&self) -> usize {
        match self {
            IssueList::Plain(v) => v.len(),
            IssueList::Baseline(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterate over the issues (dict keys in baseline mode).
    pub fn issues(&self) -> Box<dyn Iterator<Item = &'m Issue> + '_> {
        match self {
            IssueList::Plain(v) => Box::new(v.iter().copied()),
            IssueList::Baseline(v) => Box::new(v.iter().map(|(i, _)| *i)),
        }
    }
}

/// `BanditManager`.
pub struct Manager {
    pub config: BanditConfig,
    pub agg_type: AggType,
    pub debug: bool,
    pub verbose: bool,
    pub quiet: bool,
    pub ignore_nosec: bool,
    pub compat: PyCompat,
    pub test_set: TestSet,
    pub files_list: Vec<String>,
    pub excluded_files: Vec<String>,
    /// `(file name, reason)`.
    pub skipped: Vec<(String, String)>,
    pub results: Vec<Issue>,
    pub baseline: Vec<BaselineIssue>,
    pub metrics: Metrics,
    /// One score entry per scanned file, in `files_list` order.
    pub scores: Vec<Scores>,
    /// Sources retained for code snippets.
    pub sources: Vec<Arc<SourceFile>>,
}

impl Manager {
    /// `BanditManager(config, agg_type, debug, verbose, quiet, profile, ignore_nosec)`.
    pub fn new(config: BanditConfig, agg_type: AggType, test_set: TestSet) -> Manager {
        Manager {
            config,
            agg_type,
            debug: false,
            verbose: false,
            quiet: false,
            ignore_nosec: false,
            compat: PyCompat::from_env(),
            test_set,
            files_list: Vec::new(),
            excluded_files: Vec::new(),
            skipped: Vec::new(),
            results: Vec::new(),
            baseline: Vec::new(),
            metrics: Metrics::new(),
            scores: Vec::new(),
            sources: Vec::new(),
        }
    }

    /// `discover_files(targets, recursive, excluded_paths)`.
    pub fn discover_files(&mut self, targets: &[String], recursive: bool, excluded_paths: Option<&str>) {
        let d = crate::core::discover::discover_files(targets, recursive, excluded_paths, &self.config);
        self.files_list = d.files;
        self.excluded_files = d.excluded;
    }

    /// `run_tests()`: scan every file (in parallel with rayon, merged in
    /// `files_list` order), handle `-` (stdin → `<stdin>`), collect
    /// results/scores/metrics/skipped, then `metrics.aggregate()`.
    pub fn run_tests(&mut self) {
        let mut new_files_list = self.files_list.clone();
        let stdin_data = if new_files_list.iter().any(|f| f == "-") {
            use std::io::Read;
            let mut buf = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut buf);
            for f in new_files_list.iter_mut() {
                if f == "-" {
                    *f = "<stdin>".to_string();
                }
            }
            Some(buf)
        } else {
            None
        };

        enum Outcome {
            OsError(String),
            Scanned(crate::core::scan::FileOutcome),
        }

        let outcomes: Vec<(String, Outcome)> = new_files_list
            .par_iter()
            .map(|fname| {
                let bytes = if fname == "<stdin>" {
                    stdin_data.clone().unwrap_or_default()
                } else {
                    match std::fs::read(fname) {
                        Ok(b) => b,
                        Err(e) => return (fname.clone(), Outcome::OsError(io_error_message(&e))),
                    }
                };
                let outcome = crate::core::scan::scan_file(fname, &bytes, &self.test_set, self.ignore_nosec, self.compat);
                (fname.clone(), Outcome::Scanned(outcome))
            })
            .collect();

        let mut kept_files = Vec::with_capacity(outcomes.len());
        for (fname, outcome) in outcomes {
            match outcome {
                Outcome::OsError(reason) => self.skipped.push((fname, reason)),
                Outcome::Scanned(outcome) => {
                    crate::log::flush_entries(&outcome.logs);
                    self.metrics.insert(&fname, outcome.metrics);
                    match outcome.skipped {
                        Some(reason) => self.skipped.push((fname, reason)),
                        None => {
                            self.results.extend(outcome.issues);
                            self.scores.push(outcome.scores);
                            if let Some(src) = outcome.source {
                                self.sources.push(src);
                            }
                            kept_files.push(fname);
                        }
                    }
                }
            }
        }
        self.files_list = kept_files;
        self.metrics.aggregate();
    }

    /// `populate_baseline(data)`: parse the JSON report; any error →
    /// `log_warning!("manager", "Failed to load baseline data: {}", err)` and
    /// an empty baseline.
    pub fn populate_baseline(&mut self, data: &str) {
        self.baseline.clear();
        let parsed: Result<Vec<BaselineIssue>, String> = (|| {
            let v: serde_json::Value = serde_json::from_str(data).map_err(|e| e.to_string())?;
            let results = v.get("results").ok_or_else(|| "'results'".to_string())?;
            let arr = results.as_array().ok_or_else(|| "results is not a list".to_string())?;
            arr.iter().map(|r| BaselineIssue::from_dict(r).map_err(|e| e.0)).collect()
        })();
        match parsed {
            Ok(items) => self.baseline = items,
            Err(e) => crate::log_warning!("manager", "Failed to load baseline data: {}", e),
        }
    }

    /// `filter_results(sev_filter, conf_filter)` / `get_issue_list`.
    pub fn get_issue_list(&self, sev: Rank, conf: Rank) -> IssueList<'_> {
        let results: Vec<&Issue> = self.results.iter().filter(|i| i.filter(sev, conf)).collect();
        if self.baseline.is_empty() {
            return IssueList::Plain(results);
        }
        let unmatched: Vec<&Issue> = results
            .iter()
            .copied()
            .filter(|r| !self.baseline.iter().any(|b| r.matches_baseline(b)))
            .collect();
        IssueList::Baseline(find_candidate_matches(&unmatched, &results))
    }

    /// `results_count(sev_filter, conf_filter)`.
    pub fn results_count(&self, sev: Rank, conf: Rank) -> usize {
        self.get_issue_list(sev, conf).len()
    }

    /// `get_skipped()`.
    pub fn get_skipped(&self) -> &[(String, String)] {
        &self.skipped
    }
}

/// `e.strerror` equivalent: `std::io::Error`'s `Display` appends `(os error N)`;
/// bandit only reports the OS message itself.
fn io_error_message(e: &std::io::Error) -> String {
    let full = e.to_string();
    full.split(" (os error").next().unwrap_or(&full).to_string()
}

/// `_compare_baseline_results(baseline, results)`: results not in the baseline.
pub fn compare_baseline_results<'m>(baseline: &[&Issue], results: &[&'m Issue]) -> Vec<&'m Issue> {
    results.iter().copied().filter(|r| !baseline.iter().any(|b| r.same_signature(b))).collect()
}

/// `_find_candidate_matches(unmatched_issues, results_list)`.
pub fn find_candidate_matches<'m>(unmatched: &[&'m Issue], results: &[&'m Issue]) -> Vec<(&'m Issue, Vec<&'m Issue>)> {
    unmatched
        .iter()
        .map(|u| (*u, results.iter().copied().filter(|r| u.same_signature(r)).collect()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::issue::Cwe;

    fn issue(text: &str) -> Issue {
        Issue::new(Rank::Low, Rank::Low, Cwe::NOTSET, text, "a.py", "t", "B1", 1)
    }

    #[test]
    fn baseline_helpers() {
        let a = issue("a");
        let b = issue("b");
        assert_eq!(compare_baseline_results(&[&a], &[&a, &b]).len(), 1);
        assert!(compare_baseline_results(&[&a, &b], &[&a, &b]).is_empty());
        assert!(compare_baseline_results(&[&a, &b], &[&a]).is_empty());
        let m = find_candidate_matches(&[&a], &[&a, &b]);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].1.len(), 1);
        let a2 = issue("a");
        let m = find_candidate_matches(&[&a], &[&a, &a2]);
        assert_eq!(m[0].1.len(), 2);
    }
}
