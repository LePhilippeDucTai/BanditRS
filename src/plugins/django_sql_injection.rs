//! Port of `bandit/plugins/django_sql_injection.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `django_extra_used` (docs/spec/plugins.md).
pub fn django_extra_used(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.django_sql_injection:django_extra_used")
}

/// TODO(M3/M4): port `django_rawsql_used` (docs/spec/plugins.md).
pub fn django_rawsql_used(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.django_sql_injection:django_rawsql_used")
}
