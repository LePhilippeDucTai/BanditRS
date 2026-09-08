//! Port of `bandit/plugins/app_debug.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `flask_debug_true` (docs/spec/plugins.md).
pub fn flask_debug_true(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.app_debug:flask_debug_true")
}
