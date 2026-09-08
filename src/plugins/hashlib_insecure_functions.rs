//! Port of `bandit/plugins/hashlib_insecure_functions.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `hashlib` (docs/spec/plugins.md).
pub fn hashlib(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.hashlib_insecure_functions:hashlib")
}
