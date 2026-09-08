//! Port of `bandit/plugins/pytorch_load.py` — see docs/spec/plugins.md.

use crate::ast::literal::PyValue;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// `pytorch_load` (B614).
pub fn pytorch_load(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if !ctx.is_module_imported_exact("torch") {
        return Ok(None);
    }
    let qual = ctx.call_function_name_qual().unwrap_or("");
    if qual == "torch.load" || qual == "torch.serialization.load" {
        let weights_only = ctx.get_call_arg_value("weights_only")?;
        if weights_only.is_some_and(|v| v.py_eq(&PyValue::str("True"))) {
            return Ok(None);
        }
        return Ok(Some(
            IssueDraft::new(Rank::Medium, Rank::High, Cwe::DESERIALIZATION_OF_UNTRUSTED_DATA, "Use of unsafe PyTorch load")
                .with_lineno(ctx.get_lineno_for_call_arg("load")),
        ));
    }
    Ok(None)
}
