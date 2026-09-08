//! Port of `bandit/plugins/general_hardcoded_password.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `hardcoded_password_string` (docs/spec/plugins.md).
pub fn hardcoded_password_string(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.general_hardcoded_password:hardcoded_password_string")
}

/// TODO(M3/M4): port `hardcoded_password_funcarg` (docs/spec/plugins.md).
pub fn hardcoded_password_funcarg(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.general_hardcoded_password:hardcoded_password_funcarg")
}

/// TODO(M3/M4): port `hardcoded_password_default` (docs/spec/plugins.md).
pub fn hardcoded_password_default(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.general_hardcoded_password:hardcoded_password_default")
}
