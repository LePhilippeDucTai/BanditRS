//! Port of `bandit/plugins/crypto_request_no_cert_validation.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `request_with_no_cert_validation` (docs/spec/plugins.md).
pub fn request_with_no_cert_validation(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.crypto_request_no_cert_validation:request_with_no_cert_validation")
}
