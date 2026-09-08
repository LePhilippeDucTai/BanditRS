//! Port of `bandit/plugins/app_debug.py` — see docs/spec/plugins.md.

use crate::ast::literal::PyValue;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// `flask_debug_true` (B201).
pub fn flask_debug_true(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if !ctx.is_module_imported_like("flask") {
        return Ok(None);
    }
    if !ctx
        .call_function_name_qual()
        .unwrap_or("")
        .ends_with(".run")
    {
        return Ok(None);
    }
    if ctx.check_call_arg_is("debug", &PyValue::str("True"))? == Some(true) {
        return Ok(Some(
            IssueDraft::new(
                Rank::High,
                Rank::Medium,
                Cwe::CODE_INJECTION,
                "A Flask app appears to be run with debug=True, which exposes the Werkzeug debugger and allows the execution of arbitrary code.",
            )
            .with_lineno(ctx.get_lineno_for_call_arg("debug")),
        ));
    }
    Ok(None)
}
