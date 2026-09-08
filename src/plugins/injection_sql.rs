//! Port of `bandit/plugins/injection_sql.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `hardcoded_sql_expressions` (docs/spec/plugins.md).
pub fn hardcoded_sql_expressions(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.injection_sql:hardcoded_sql_expressions")
}
