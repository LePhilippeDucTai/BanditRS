//! Port of `bandit/plugins/jinja2_templates.py` — see docs/spec/plugins.md.

use crate::core::context::Context;
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// TODO(M3/M4): port `jinja2_autoescape_false` (docs/spec/plugins.md).
pub fn jinja2_autoescape_false(_ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    todo!("port bandit.plugins.jinja2_templates:jinja2_autoescape_false")
}
