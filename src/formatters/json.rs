//! `json` formatter — port of `bandit/formatters/json.py`.
//!
//! `{"errors": [{filename, reason}], "generated_at": "%Y-%m-%dT%H:%M:%SZ", "metrics": metrics.data, "results": [issue dicts + more_info (+ candidates when >1)]}` sorted by `filename` (or `test_name` for `-a vuln`), `json.dumps(sort_keys=True, indent=2)` with ensure_ascii escaping, no trailing newline.

use std::io::{self, Write};

use serde_json::{Map, Value};

use crate::constants::Rank;
use crate::core::docs_utils::get_url;
use crate::core::manager::{AggType, IssueList, Manager};
use crate::pycompat::datetime::UtcDateTime;
use crate::pycompat::json::dumps_sorted_indent2;

/// Build the `results` array (as JSON objects) for the machine-readable
/// formatters (json/yaml).
pub fn build_results(manager: &Manager, sev_level: Rank, conf_level: Rank, lines: i64, with_candidates: bool) -> Vec<Value> {
    let issue_list = manager.get_issue_list(sev_level, conf_level);
    let mut collector = Vec::new();
    match issue_list {
        IssueList::Plain(issues) => {
            for r in issues {
                let mut d = r.as_dict(true, lines);
                d.as_object_mut().unwrap().insert("more_info".into(), Value::from(get_url(&r.test_id)));
                collector.push(d);
            }
        }
        IssueList::Baseline(pairs) => {
            for (r, candidates) in pairs {
                let mut d = r.as_dict(true, lines);
                let obj = d.as_object_mut().unwrap();
                obj.insert("more_info".into(), Value::from(get_url(&r.test_id)));
                if with_candidates && candidates.len() > 1 {
                    let cands: Vec<Value> = candidates.iter().map(|c| c.as_dict(true, lines)).collect();
                    obj.insert("candidates".into(), Value::Array(cands));
                }
                collector.push(d);
            }
        }
    }
    collector
}

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(manager: &Manager, out: &mut dyn Write, sev_level: Rank, conf_level: Rank, lines: i64) -> io::Result<()> {
    let mut machine_output = Map::new();

    let errors: Vec<Value> = manager
        .get_skipped()
        .iter()
        .map(|(fname, reason)| {
            let mut m = Map::new();
            m.insert("filename".into(), Value::from(fname.as_str()));
            m.insert("reason".into(), Value::from(reason.as_str()));
            Value::Object(m)
        })
        .collect();
    machine_output.insert("errors".into(), Value::Array(errors));

    let mut collector = build_results(manager, sev_level, conf_level, lines, true);
    let key = if manager.agg_type == AggType::Vuln { "test_name" } else { "filename" };
    collector.sort_by(|a, b| a[key].as_str().cmp(&b[key].as_str()));
    machine_output.insert("results".into(), Value::Array(collector));

    machine_output.insert("metrics".into(), manager.metrics.to_json());
    machine_output.insert("generated_at".into(), Value::from(UtcDateTime::now().iso_z()));

    out.write_all(dumps_sorted_indent2(&Value::Object(machine_output)).as_bytes())
}
