//! Port of `bandit/plugins/injection_paramiko.py` — see docs/spec/plugins.md.

use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// `paramiko_calls` (B601).
pub fn paramiko_calls(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if ctx.is_module_imported_like("paramiko") && ctx.call_function_name() == Some("exec_command") {
        return Ok(Some(IssueDraft::new(
            Rank::Medium,
            Rank::Medium,
            Cwe::OS_COMMAND_INJECTION,
            "Possible shell injection via Paramiko call, check inputs are properly sanitized.",
        )));
    }
    Ok(None)
}
