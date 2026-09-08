//! `screen` formatter — port of `bandit/formatters/screen.py`.
//!
//! same layout as text with ANSI colours (`\x1b[95m` header, `\x1b[94m` LOW, `\x1b[93m` MEDIUM, `\x1b[91m` HIGH, `\x1b[0m` reset); no `Total potential issues skipped` line; always printed to stdout (never to `-o`, which only logs a hint).

use std::io::{self, Write};

use crate::constants::{CRITERIA, RANKING, Rank};
use crate::core::docs_utils::get_url;
use crate::core::issue::Issue;
use crate::core::manager::{IssueList, Manager};
use crate::pycompat::datetime::UtcDateTime;

/// `COLOR["DEFAULT"]`.
pub const DEFAULT: &str = "\x1b[0m";
/// `COLOR["HEADER"]`.
pub const HEADER: &str = "\x1b[95m";
/// `COLOR["LOW"]`.
pub const LOW: &str = "\x1b[94m";
/// `COLOR["MEDIUM"]`.
pub const MEDIUM: &str = "\x1b[93m";
/// `COLOR["HIGH"]`.
pub const HIGH: &str = "\x1b[91m";

/// `COLOR[issue.severity]`: the ANSI colour for a given rank (`UNDEFINED`
/// maps to `COLOR["DEFAULT"]`, as `bandit.formatters.screen.COLOR` has no
/// `UNDEFINED` entry but `Rank::UNDEFINED` issues never reach this path in
/// upstream bandit).
pub fn color_for(rank: Rank) -> &'static str {
    match rank {
        Rank::Undefined => DEFAULT,
        Rank::Low => LOW,
        Rank::Medium => MEDIUM,
        Rank::High => HIGH,
    }
}

/// `header(text)` = `COLOR["HEADER"] + text + COLOR["DEFAULT"]`.
pub fn header(text: &str) -> String {
    format!("{HEADER}{text}{DEFAULT}")
}

pub fn get_verbose_details(manager: &Manager) -> String {
    let mut bits = Vec::new();
    bits.push(header(&format!(
        "Files in scope ({}):",
        manager.files_list.len()
    )));
    for (item, score) in manager.files_list.iter().zip(manager.scores.iter()) {
        bits.push(format!(
            "\t{} (score: {{SEVERITY: {}, CONFIDENCE: {}}})",
            item,
            score.severity_total(),
            score.confidence_total()
        ));
    }
    bits.push(header(&format!(
        "Files excluded ({}):",
        manager.excluded_files.len()
    )));
    for fname in &manager.excluded_files {
        bits.push(format!("\t{fname}"));
    }
    bits.join("\n")
}

pub fn get_metrics(manager: &Manager) -> String {
    let mut bits = Vec::new();
    bits.push(header("\nRun metrics:"));
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

/// `_output_issue_str(issue, indent, show_lineno=True, show_code=True, lines=-1)`.
pub fn output_issue_str(
    issue: &Issue,
    indent: &str,
    show_lineno: bool,
    show_code: bool,
    lines: i64,
) -> String {
    let mut bits = Vec::new();
    bits.push(format!(
        "{indent}{}>> Issue: [{}:{}] {}",
        color_for(issue.severity),
        issue.test_id,
        issue.test,
        issue.text
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
        "{indent}   Location: {}:{}:{}{DEFAULT}",
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

/// `report(manager, fileobj, sev_level, conf_level, lines)`, with `do_print`
/// rendered into `out` instead of the real stdout so the formatter is
/// testable (`do_print(bits)` = `print("\n".join(bits))`).
pub fn report_to(
    manager: &Manager,
    out: &mut dyn Write,
    sev_level: Rank,
    conf_level: Rank,
    lines: i64,
) -> io::Result<()> {
    if !manager.quiet || manager.results_count(sev_level, conf_level) > 0 {
        let mut bits = Vec::new();
        bits.push(header(&format!(
            "Run started:{}",
            UtcDateTime::now().python_str()
        )));
        if manager.verbose {
            bits.push(get_verbose_details(manager));
        }
        bits.push(header("\nTest results:"));
        bits.push(get_results(manager, sev_level, conf_level, lines));
        bits.push(header("\nCode scanned:"));
        bits.push(format!(
            "\tTotal lines of code: {}",
            manager.metrics.totals.loc
        ));
        bits.push(format!(
            "\tTotal lines skipped (#nosec): {}",
            manager.metrics.totals.nosec
        ));
        bits.push(get_metrics(manager));
        bits.push(header(&format!(
            "Files skipped ({}):",
            manager.skipped.len()
        )));
        for (fname, reason) in &manager.skipped {
            bits.push(format!("\t{fname} ({reason})"));
        }
        writeln!(out, "{}", bits.join("\n"))?;
    }
    Ok(())
}

/// `report(manager, out, sev_level, conf_level, lines, out_name)`. `out_name`
/// is the `-o` file name (if any); screen always writes ANSI text to real
/// stdout (via [`report_to`]) and only logs a hint when `-o` was given.
pub fn report(
    manager: &Manager,
    sev_level: Rank,
    conf_level: Rank,
    lines: i64,
    out_name: Option<&str>,
) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    report_to(manager, &mut stdout, sev_level, conf_level, lines)?;
    drop(stdout);
    if let Some(name) = out_name {
        crate::log_info!(
            "screen",
            "Screen formatter output was not written to file: {}, consider '-f txt'",
            name
        );
    }
    Ok(())
}
