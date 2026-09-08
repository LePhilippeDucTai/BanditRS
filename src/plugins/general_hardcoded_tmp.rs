//! Port of `bandit/plugins/general_hardcoded_tmp.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `hardcoded_tmp_directory` (docs/spec/plugins.md).
pub fn hardcoded_tmp_directory(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.general_hardcoded_tmp:hardcoded_tmp_directory")
}
