//! Port of `bandit/plugins/injection_wildcard.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `linux_commands_wildcard_injection` (docs/spec/plugins.md).
pub fn linux_commands_wildcard_injection(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.injection_wildcard:linux_commands_wildcard_injection")
}
