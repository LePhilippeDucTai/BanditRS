//! `custom` formatter — port of `bandit/formatters/custom.py`.
//!
//! `--msg-template` rendering with Python `str.format` semantics: tags abspath, relpath, line, col, end_col, test_id, severity, msg, confidence, range, cwe; default template `{abspath}:{line}: {test_id}[bandit]: {severity}: {msg}`; validation with `SafeMapper(line=0)` (only `line` is an int during validation) → `Template is not in valid format: {err}` + exit 2; no tags → `No tags were found in the template. Are you missing '{}'?` + exit 2; unknown tags → warning `Tag '%s' was not recognized and will be skipped, did you mean to use '%s'?` and emitted as the bare tag name; one line per issue (`template + "\n"`).

use std::collections::BTreeSet;
use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::issue::Issue;
use crate::core::manager::Manager;
use crate::pycompat::path::{abspath, relpath};
use crate::pycompat::pyformat::{Segment, format_int, format_str, parse_spec, parse_template};

const KNOWN_TAGS: [&str; 11] = [
    "abspath",
    "relpath",
    "line",
    "col",
    "end_col",
    "test_id",
    "severity",
    "msg",
    "confidence",
    "range",
    "cwe",
];

/// Value produced by a tag for one issue, formatted with the field's spec.
enum TagValue {
    Int(i64),
    Str(String),
}

fn tag_value(tag: &str, issue: &Issue) -> Option<TagValue> {
    Some(match tag {
        "abspath" => TagValue::Str(abspath(&issue.fname)),
        "relpath" => TagValue::Str(relpath(&issue.fname, None)),
        "line" => TagValue::Int(issue.lineno as i64),
        "col" => TagValue::Int(issue.col_offset as i64),
        "end_col" => TagValue::Int(issue.end_col_offset as i64),
        "test_id" => TagValue::Str(issue.test_id.to_string()),
        "severity" => TagValue::Str(issue.severity.as_str().to_string()),
        "msg" => TagValue::Str(issue.text.clone()),
        "confidence" => TagValue::Str(issue.confidence.as_str().to_string()),
        "range" => TagValue::Str(format!(
            "[{}]",
            issue
                .linerange
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
        "cwe" => TagValue::Str(issue.cwe.to_string()),
        _ => return None,
    })
}

/// `max(similarity_list)` (`len(set(tag) & known_set)`, ties broken by the
/// greatest tag name, matching `sorted(...)[-1]`).
fn similar_tag(tag: &str) -> &'static str {
    let tag_set: BTreeSet<char> = tag.chars().collect();
    KNOWN_TAGS
        .iter()
        .map(|known| {
            let known_set: BTreeSet<char> = known.chars().collect();
            (tag_set.intersection(&known_set).count(), *known)
        })
        .max()
        .map(|(_, known)| known)
        .unwrap_or("")
}

/// `report(manager, fileobj, sev_level, conf_level, template)`.
pub fn report(
    manager: &Manager,
    out: &mut dyn Write,
    sev_level: Rank,
    conf_level: Rank,
    template: Option<&str>,
) -> io::Result<()> {
    let msg_template = template.unwrap_or("{abspath}:{line}: {test_id}[bandit]: {severity}: {msg}");

    let segments = match parse_template(msg_template) {
        Ok(s) => s,
        Err(e) => {
            crate::log_error!("custom", "Template is not in valid format: {}", e);
            std::process::exit(2);
        }
    };

    for seg in &segments {
        if let Segment::Field { name, spec, .. } = seg {
            let parsed = match parse_spec(spec) {
                Ok(p) => p,
                Err(e) => {
                    crate::log_error!("custom", "Template is not in valid format: {}", e);
                    std::process::exit(2);
                }
            };
            let result = if matches!(name.as_str(), "line" | "col" | "end_col") {
                format_int(0, &parsed).map(|_| ())
            } else {
                format_str("", &parsed).map(|_| ())
            };
            if let Err(e) = result {
                crate::log_error!("custom", "Template is not in valid format: {}", e);
                std::process::exit(2);
            }
        }
    }

    let tag_set: BTreeSet<&str> = segments
        .iter()
        .filter_map(|s| match s {
            Segment::Field { name, .. } => Some(name.as_str()),
            Segment::Literal(_) => None,
        })
        .collect();
    if tag_set.is_empty() {
        crate::log_error!(
            "custom",
            "No tags were found in the template. Are you missing '{{}}'?"
        );
        std::process::exit(2);
    }

    for tag in &tag_set {
        if !KNOWN_TAGS.contains(tag) {
            crate::log_warning!(
                "custom",
                "Tag '{}' was not recognized and will be skipped, did you mean to use '{}'?",
                tag,
                similar_tag(tag)
            );
        }
    }

    for issue in manager.get_issue_list(sev_level, conf_level).issues() {
        let mut line = String::new();
        for seg in &segments {
            match seg {
                Segment::Literal(text) => line.push_str(text),
                Segment::Field { name, spec, .. } => {
                    if !KNOWN_TAGS.contains(&name.as_str()) {
                        line.push_str(name);
                        continue;
                    }
                    let value = tag_value(name, issue).expect("known tag");
                    let spec = parse_spec(spec).unwrap_or_else(|_| parse_spec("").unwrap());
                    let rendered = match value {
                        TagValue::Int(v) => format_int(v, &spec).unwrap_or_default(),
                        TagValue::Str(v) => format_str(&v, &spec).unwrap_or(v),
                    };
                    line.push_str(&rendered);
                }
            }
        }
        line.push('\n');
        out.write_all(line.as_bytes())?;
    }
    Ok(())
}
