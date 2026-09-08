//! Port of `bandit/plugins/general_bad_file_permissions.py` — see docs/spec/plugins.md.

use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

const S_IWOTH: i128 = 0o002;
const S_IWGRP: i128 = 0o020;
const S_IXGRP: i128 = 0o010;
const S_IXOTH: i128 = 0o001;

/// `set_bad_file_permissions` (B103).
pub fn set_bad_file_permissions(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if !ctx.call_function_name().unwrap_or("").contains("chmod") {
        return Ok(None);
    }
    if ctx.call_args_count() != Some(2) {
        return Ok(None);
    }
    let Some(mode) = ctx.get_call_arg_at_position(1)?.and_then(|v| v.as_int()) else {
        return Ok(None);
    };
    let dangerous = mode & S_IWOTH != 0 || mode & S_IWGRP != 0 || mode & S_IXGRP != 0 || mode & S_IXOTH != 0;
    if !dangerous {
        return Ok(None);
    }
    let severity = if mode & S_IWOTH != 0 { Rank::High } else { Rank::Medium };
    let filename = match ctx.get_call_arg_at_position(0)? {
        Some(v) if v.truthy() => v.py_str(),
        _ => "NOT PARSED".to_string(),
    };
    Ok(Some(IssueDraft::new(
        severity,
        Rank::High,
        Cwe::INCORRECT_PERMISSION_ASSIGNMENT,
        format!("Chmod setting a permissive mask 0o{mode:o} on file ({filename})."),
    )))
}
