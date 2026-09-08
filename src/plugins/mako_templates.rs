//! Port of `bandit/plugins/mako_templates.py` — see docs/spec/plugins.md.

use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// `use_of_mako_templates` (B702).
pub fn use_of_mako_templates(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    let qual = ctx.call_function_name_qual().unwrap_or("");
    let parts: Vec<&str> = qual.split('.').collect();
    if parts.contains(&"mako") && parts.last() == Some(&"Template") {
        return Ok(Some(IssueDraft::new(
            Rank::Medium,
            Rank::High,
            Cwe::BASIC_XSS,
            "Mako templates allow HTML/JS rendering by default and are inherently open to XSS attacks. Ensure variables in all templates are properly sanitized via the 'n', 'h' or 'x' flags (depending on context). For example, to HTML escape the variable 'data' do ${ data |h }.",
        )));
    }
    Ok(None)
}
