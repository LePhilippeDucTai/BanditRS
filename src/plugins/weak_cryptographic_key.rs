//! Port of `bandit/plugins/weak_cryptographic_key.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `weak_cryptographic_key` (docs/spec/plugins.md).
pub fn weak_cryptographic_key(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.weak_cryptographic_key:weak_cryptographic_key")
}
