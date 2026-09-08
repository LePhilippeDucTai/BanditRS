//! Port of `bandit/plugins/general_hardcoded_tmp.py` — see docs/spec/plugins.md.

use crate::ast::literal::PyErr;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::{ListOpt, PluginConfigs};
use crate::plugins::PluginResult;

const DEFAULT_TMP_DIRS: &[&str] = &["/tmp", "/var/tmp", "/dev/shm"];

/// `hardcoded_tmp_directory` (B108).
pub fn hardcoded_tmp_directory(ctx: &Context<'_, '_>, cfg: &PluginConfigs) -> PluginResult {
    let tmp_dirs: Vec<&str> = match &cfg.hardcoded_tmp_directory.tmp_dirs {
        ListOpt::Missing => DEFAULT_TMP_DIRS.to_vec(),
        ListOpt::Null => return Err(PyErr::type_error("argument of type 'NoneType' is not iterable")),
        ListOpt::Items(items) => items.iter().map(String::as_str).collect(),
    };
    let value = ctx.string_val().unwrap_or("");
    if tmp_dirs.iter().any(|s| value.starts_with(s)) {
        return Ok(Some(IssueDraft::new(
            Rank::Medium,
            Rank::Medium,
            Cwe::INSECURE_TEMP_FILE,
            "Probable insecure usage of temp file/directory.",
        )));
    }
    Ok(None)
}
