//! Port of `bandit/plugins/ssh_no_host_key_verification.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `ssh_no_host_key_verification` (docs/spec/plugins.md).
pub fn ssh_no_host_key_verification(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.ssh_no_host_key_verification:ssh_no_host_key_verification")
}
