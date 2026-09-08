//! `text` formatter — port of `bandit/formatters/text.py`.
//!
//! `Run started:{datetime}`, verbose file lists, `Test results:`, issue blocks (`>> Issue: [id:test] text`, `   Severity: X   Confidence: Y`, `   CWE: ...`, `   More Info: url`, `   Location: file:line:col`, tabbed code lines, 50 dashes), `Code scanned:` totals, `Run metrics:`, `Files skipped (N):`; nothing printed in quiet mode with zero results; baseline candidates indented by 10 spaces.

use std::io::{self, Write};

use crate::constants::{CRITERIA, RANKING, Rank};
use crate::core::docs_utils::get_url;
use crate::core::issue::Issue;
use crate::core::manager::{IssueList, Manager};
use crate::pycompat::datetime::UtcDateTime;

pub fn get_verbose_details(manager: &Manager) -> String {
    let mut bits = Vec::new();
    bits.push(format!("Files in scope ({}):", manager.files_list.len()));
    for (item, score) in manager.files_list.iter().zip(manager.scores.iter()) {
        bits.push(format!(
            "\t{} (score: {{SEVERITY: {}, CONFIDENCE: {}}})",
            item,
            score.severity_total(),
            score.confidence_total()
        ));
    }
    bits.push(format!(
        "Files excluded ({}):",
        manager.excluded_files.len()
    ));
    for fname in &manager.excluded_files {
        bits.push(format!("\t{fname}"));
    }
    bits.join("\n")
}

pub fn get_metrics(manager: &Manager) -> String {
    let mut bits = Vec::new();
    bits.push("\nRun metrics:".to_string());
    for criteria in CRITERIA {
        bits.push(format!("\tTotal issues (by {}):", criteria.to_lowercase()));
        for rank in RANKING {
            let value = manager
                .metrics
                .totals
                .get(&format!("{criteria}.{rank}"))
                .unwrap_or(0);
            bits.push(format!("\t\t{}: {}", capitalize(rank), value));
        }
    }
    bits.join("\n")
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}

fn output_issue_str(
    issue: &Issue,
    indent: &str,
    show_lineno: bool,
    show_code: bool,
    lines: i64,
) -> String {
    let mut bits = Vec::new();
    bits.push(format!(
        "{indent}>> Issue: [{}:{}] {}",
        issue.test_id, issue.test, issue.text
    ));
    bits.push(format!(
        "{indent}   Severity: {}   Confidence: {}",
        issue.severity.capitalize(),
        issue.confidence.capitalize()
    ));
    bits.push(format!("{indent}   CWE: {}", issue.cwe));
    bits.push(format!("{indent}   More Info: {}", get_url(&issue.test_id)));
    let (lineno_s, col_s) = if show_lineno {
        (issue.lineno.to_string(), issue.col_offset.to_string())
    } else {
        (String::new(), String::new())
    };
    bits.push(format!(
        "{indent}   Location: {}:{}:{}",
        issue.fname, lineno_s, col_s
    ));
    if show_code {
        for line in issue.get_code(lines, true).split('\n') {
            bits.push(format!("{indent}{line}"));
        }
    }
    bits.join("\n")
}

pub fn get_results(manager: &Manager, sev_level: Rank, conf_level: Rank, lines: i64) -> String {
    let issue_list = manager.get_issue_list(sev_level, conf_level);
    if issue_list.is_empty() {
        return "\tNo issues identified.".to_string();
    }
    let candidate_indent = " ".repeat(10);
    let mut bits = Vec::new();
    match issue_list {
        IssueList::Plain(issues) => {
            for issue in issues {
                bits.push(output_issue_str(issue, "", true, true, lines));
                bits.push("-".repeat(50));
            }
        }
        IssueList::Baseline(pairs) => {
            for (issue, candidates) in pairs {
                if candidates.len() == 1 {
                    bits.push(output_issue_str(issue, "", true, true, lines));
                } else {
                    bits.push(output_issue_str(issue, "", false, false, lines));
                    bits.push("\n-- Candidate Issues --".to_string());
                    for candidate in candidates {
                        bits.push(output_issue_str(
                            candidate,
                            &candidate_indent,
                            true,
                            true,
                            lines,
                        ));
                        bits.push("\n".to_string());
                    }
                }
                bits.push("-".repeat(50));
            }
        }
    }
    bits.join("\n")
}

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(
    manager: &Manager,
    out: &mut dyn Write,
    sev_level: Rank,
    conf_level: Rank,
    lines: i64,
) -> io::Result<()> {
    if !manager.quiet || manager.results_count(sev_level, conf_level) > 0 {
        let mut bits = Vec::new();
        bits.push(format!("Run started:{}", UtcDateTime::now().python_str()));
        if manager.verbose {
            bits.push(get_verbose_details(manager));
        }
        bits.push("\nTest results:".to_string());
        bits.push(get_results(manager, sev_level, conf_level, lines));
        bits.push("\nCode scanned:".to_string());
        bits.push(format!(
            "\tTotal lines of code: {}",
            manager.metrics.totals.loc
        ));
        bits.push(format!(
            "\tTotal lines skipped (#nosec): {}",
            manager.metrics.totals.nosec
        ));
        bits.push(format!(
            "\tTotal potential issues skipped due to specifically being disabled (e.g., #nosec BXXX): {}",
            manager.metrics.totals.skipped_tests
        ));
        bits.push(get_metrics(manager));
        bits.push(format!("Files skipped ({}):", manager.skipped.len()));
        for (fname, reason) in &manager.skipped {
            bits.push(format!("\t{fname} ({reason})"));
        }
        let result = bits.join("\n") + "\n";
        out.write_all(result.as_bytes())?;
    }
    Ok(())
}
