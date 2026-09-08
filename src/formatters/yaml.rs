//! `yaml` formatter — port of `bandit/formatters/yaml.py`.
//!
//! same data as json (no baseline candidates), `code` with literal `\n`, `generated_at`, `yaml.safe_dump(default_flow_style=False)` (sorted keys, block style) via `pycompat::yaml_emit`.

use std::io::{self, Write};

use serde_json::{Map, Value};

use crate::constants::Rank;
use crate::core::manager::{AggType, Manager};
use crate::formatters::json::build_results;
use crate::pycompat::datetime::UtcDateTime;
use crate::pycompat::yaml_emit::safe_dump_block;

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(
    manager: &Manager,
    out: &mut dyn Write,
    sev_level: Rank,
    conf_level: Rank,
    lines: i64,
) -> io::Result<()> {
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

    let mut collector = build_results(manager, sev_level, conf_level, lines, false);
    let key = if manager.agg_type == AggType::Vuln {
        "test_name"
    } else {
        "filename"
    };
    collector.sort_by(|a, b| a[key].as_str().cmp(&b[key].as_str()));

    for result in collector.iter_mut() {
        if let Some(Value::String(code)) = result.get("code") {
            let escaped = code.replace('\n', "\\n");
            result
                .as_object_mut()
                .unwrap()
                .insert("code".into(), Value::from(escaped));
        }
    }
    machine_output.insert("results".into(), Value::Array(collector));

    machine_output.insert("metrics".into(), manager.metrics.to_json());
    machine_output.insert(
        "generated_at".into(),
        Value::from(UtcDateTime::now().iso_z()),
    );

    out.write_all(safe_dump_block(&Value::Object(machine_output)).as_bytes())
}
