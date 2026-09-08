//! Port of `bandit/plugins/try_except_pass.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `try_except_pass` (docs/spec/plugins.md).
pub fn try_except_pass(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.try_except_pass:try_except_pass")
}
