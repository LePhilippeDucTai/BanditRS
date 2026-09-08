//! Port of `bandit/plugins/exec.py` — see docs/spec/plugins.md.

use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// `exec_used` (B102).
pub fn exec_used(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if ctx.call_function_name_qual() == Some("exec") {
        return Ok(Some(IssueDraft::new(Rank::Medium, Rank::High, Cwe::OS_COMMAND_INJECTION, "Use of exec detected.")));
    }
    Ok(None)
}
