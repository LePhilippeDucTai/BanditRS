//! Port of `bandit/plugins/general_bad_file_permissions.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `set_bad_file_permissions` (docs/spec/plugins.md).
pub fn set_bad_file_permissions(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.general_bad_file_permissions:set_bad_file_permissions")
}
