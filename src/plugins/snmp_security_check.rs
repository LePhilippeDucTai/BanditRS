//! Port of `bandit/plugins/snmp_security_check.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `snmp_insecure_version_check` (docs/spec/plugins.md).
pub fn snmp_insecure_version_check(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.snmp_security_check:snmp_insecure_version_check")
}

/// TODO(M3/M4): port `snmp_crypto_check` (docs/spec/plugins.md).
pub fn snmp_crypto_check(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.snmp_security_check:snmp_crypto_check")
}
