//! Port of `bandit/plugins/snmp_security_check.py` — see docs/spec/plugins.md.

use crate::ast::literal::PyValue;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

/// `snmp_insecure_version_check` (B508).
pub fn snmp_insecure_version_check(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if ctx.call_function_name_qual() == Some("pysnmp.hlapi.CommunityData") {
        let a = ctx.check_call_arg_value("mpModel", &[PyValue::Int(0)])? == Some(true);
        let b = ctx.check_call_arg_value("mpModel", &[PyValue::Int(1)])? == Some(true);
        if a || b {
            return Ok(Some(
                IssueDraft::new(
                    Rank::Medium,
                    Rank::High,
                    Cwe::CLEARTEXT_TRANSMISSION,
                    "The use of SNMPv1 and SNMPv2 is insecure. You should use SNMPv3 if able.",
                )
                .with_lineno(ctx.get_lineno_for_call_arg("CommunityData")),
            ));
        }
    }
    Ok(None)
}

/// `snmp_crypto_check` (B509).
pub fn snmp_crypto_check(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if ctx.call_function_name_qual() == Some("pysnmp.hlapi.UsmUserData") && ctx.call_args_count().unwrap_or(0) < 3 {
        return Ok(Some(
            IssueDraft::new(
                Rank::Medium,
                Rank::High,
                Cwe::CLEARTEXT_TRANSMISSION,
                "You should not use SNMPv3 without encryption. noAuthNoPriv & authNoPriv is insecure",
            )
            .with_lineno(ctx.get_lineno_for_call_arg("UsmUserData")),
        ));
    }
    Ok(None)
}
