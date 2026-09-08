//! Port of `bandit/plugins/insecure_ssl_tls.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `ssl_with_bad_version` (docs/spec/plugins.md).
pub fn ssl_with_bad_version(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.insecure_ssl_tls:ssl_with_bad_version")
}

/// TODO(M3/M4): port `ssl_with_bad_defaults` (docs/spec/plugins.md).
pub fn ssl_with_bad_defaults(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.insecure_ssl_tls:ssl_with_bad_defaults")
}

/// TODO(M3/M4): port `ssl_with_no_version` (docs/spec/plugins.md).
pub fn ssl_with_no_version(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.insecure_ssl_tls:ssl_with_no_version")
}
