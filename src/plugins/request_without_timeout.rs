//! Port of `bandit/plugins/request_without_timeout.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `request_without_timeout` (docs/spec/plugins.md).
pub fn request_without_timeout(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.request_without_timeout:request_without_timeout")
}
