//! Port of `bandit/plugins/general_bind_all_interfaces.py` — see docs/spec/plugins.md.

use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// `hardcoded_bind_all_interfaces` (B104).
pub fn hardcoded_bind_all_interfaces(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if ctx.string_val() == Some("0.0.0.0") {
        return Ok(Some(IssueDraft::new(
            Rank::Medium,
            Rank::Medium,
            Cwe::MULTIPLE_BINDS,
            "Possible binding to all interfaces.",
        )));
    }
    Ok(None)
}
