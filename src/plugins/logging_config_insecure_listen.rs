//! Port of `bandit/plugins/logging_config_insecure_listen.py` — see docs/spec/plugins.md.

use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// `logging_config_insecure_listen` (B612).
pub fn logging_config_insecure_listen(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if ctx.call_function_name_qual() == Some("logging.config.listen") {
        let has_verify = ctx.call_keywords()?.is_some_and(|k| k.contains("verify"));
        if !has_verify {
            return Ok(Some(IssueDraft::new(
                Rank::Medium,
                Rank::High,
                Cwe::CODE_INJECTION,
                "Use of insecure logging.config.listen detected.",
            )));
        }
    }
    Ok(None)
}
