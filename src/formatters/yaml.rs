//! `yaml` formatter — port of `bandit/formatters/yaml.py`.
//!
//! same data as json (no baseline candidates), `code` with literal `\n`, `generated_at`, `yaml.safe_dump(default_flow_style=False)` (sorted keys, block style) via `pycompat::yaml_emit`.
//!
//! Status: stub (PLAN.md M6). Log line on file output:
//! `"YAML output written to file: {}"` (module tag `yaml`).

use std::io::{self, Write};

use crate::constants::Rank;
use crate::core::manager::Manager;

/// `report(manager, fileobj, sev_level, conf_level, lines)`.
pub fn report(_manager: &Manager, _out: &mut dyn Write, _sev_level: Rank, _conf_level: Rank, _lines: i64) -> io::Result<()> {
    todo!("M6: yaml formatter")
}
