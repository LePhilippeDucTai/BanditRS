//! Port of `bandit/plugins/django_xss.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `django_mark_safe` (docs/spec/plugins.md).
pub fn django_mark_safe(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.django_xss:django_mark_safe")
}
