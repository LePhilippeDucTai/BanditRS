//! Port of `bandit/plugins/injection_paramiko.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `paramiko_calls` (docs/spec/plugins.md).
pub fn paramiko_calls(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.injection_paramiko:paramiko_calls")
}
