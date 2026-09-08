//! Port of `bandit/plugins/yaml_load.py` — see docs/spec/plugins.md.

use crate::ast::literal::{PyErr, PyValue};
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

fn pos_matches(ctx: &Context<'_, '_>, pos: usize, val: &str) -> Result<bool, PyErr> {
    Ok(ctx
        .get_call_arg_at_position(pos)?
        .map(|v| v.py_eq(&PyValue::str(val)))
        .unwrap_or(false))
}

/// `yaml_load` (B506).
pub fn yaml_load(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if !ctx.is_module_imported_exact("yaml") {
        return Ok(None);
    }
    let qual = ctx.call_function_name_qual().unwrap_or("");
    let parts: Vec<&str> = qual.split('.').collect();
    let func = parts.last().copied().unwrap_or("");
    if !parts.contains(&"yaml") || func != "load" {
        return Ok(None);
    }
    let loader_safe = ctx.check_call_arg_is("Loader", &PyValue::str("SafeLoader"))? == Some(true);
    let loader_csafe = ctx.check_call_arg_is("Loader", &PyValue::str("CSafeLoader"))? == Some(true);
    let pos_safe = pos_matches(ctx, 1, "SafeLoader")?;
    let pos_csafe = pos_matches(ctx, 1, "CSafeLoader")?;
    if !loader_safe && !loader_csafe && !pos_safe && !pos_csafe {
        return Ok(Some(
            IssueDraft::new(
                Rank::Medium,
                Rank::High,
                Cwe::IMPROPER_INPUT_VALIDATION,
                "Use of unsafe yaml load. Allows instantiation of arbitrary objects. Consider yaml.safe_load().",
            )
            .with_lineno(ctx.lineno()),
        ));
    }
    Ok(None)
}
