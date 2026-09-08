//! Port of `bandit/plugins/insecure_ssl_tls.py` — see docs/spec/plugins.md.

use crate::ast::literal::PyValue;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

fn bad_versions<'a>(cfg: &'a PluginConfigs) -> Result<Vec<PyValue<'a>>, crate::ast::literal::PyErr> {
    let items = cfg.ssl_with_bad_version.bad_protocol_versions.items("bad_protocol_versions")?;
    Ok(items.iter().map(|s| PyValue::str(s.as_str())).collect())
}

/// `ssl_with_bad_version` (B502).
pub fn ssl_with_bad_version(ctx: &Context<'_, '_>, cfg: &PluginConfigs) -> PluginResult {
    let bad = bad_versions(cfg)?;
    let qual = ctx.call_function_name_qual().unwrap_or("");
    if qual == "ssl.wrap_socket" {
        if ctx.check_call_arg_value("ssl_version", &bad)? == Some(true) {
            return Ok(Some(
                IssueDraft::new(
                    Rank::High,
                    Rank::High,
                    Cwe::BROKEN_CRYPTO,
                    "ssl.wrap_socket call with insecure SSL/TLS protocol version identified, security issue.",
                )
                .with_lineno(ctx.get_lineno_for_call_arg("ssl_version")),
            ));
        }
    } else if qual == "pyOpenSSL.SSL.Context" {
        if ctx.check_call_arg_value("method", &bad)? == Some(true) {
            return Ok(Some(
                IssueDraft::new(
                    Rank::High,
                    Rank::High,
                    Cwe::BROKEN_CRYPTO,
                    "SSL.Context call with insecure SSL/TLS protocol version identified, security issue.",
                )
                .with_lineno(ctx.get_lineno_for_call_arg("method")),
            ));
        }
    } else {
        let m1 = ctx.check_call_arg_value("method", &bad)? == Some(true);
        let m2 = ctx.check_call_arg_value("ssl_version", &bad)? == Some(true);
        if m1 || m2 {
            let lineno = ctx.get_lineno_for_call_arg("method").or_else(|| ctx.get_lineno_for_call_arg("ssl_version"));
            return Ok(Some(
                IssueDraft::new(
                    Rank::Medium,
                    Rank::Medium,
                    Cwe::BROKEN_CRYPTO,
                    "Function call with insecure SSL/TLS protocol identified, possible security issue.",
                )
                .with_lineno(lineno),
            ));
        }
    }
    Ok(None)
}

/// `ssl_with_bad_defaults` (B503).
pub fn ssl_with_bad_defaults(ctx: &Context<'_, '_>, cfg: &PluginConfigs) -> PluginResult {
    let bad = cfg.ssl_with_bad_version.bad_protocol_versions.items("bad_protocol_versions")?;
    for default in ctx.function_def_defaults_qual() {
        let val = default.rsplit('.').next().unwrap_or(&default);
        if bad.iter().any(|b| b == val) {
            return Ok(Some(IssueDraft::new(
                Rank::Medium,
                Rank::Medium,
                Cwe::BROKEN_CRYPTO,
                "Function definition identified with insecure SSL/TLS protocol version by default, possible security issue.",
            )));
        }
    }
    Ok(None)
}

/// `ssl_with_no_version` (B504).
pub fn ssl_with_no_version(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if ctx.call_function_name_qual() == Some("ssl.wrap_socket") && ctx.check_call_arg_value("ssl_version", &[])?.is_none() {
        return Ok(Some(
            IssueDraft::new(
                Rank::Low,
                Rank::Medium,
                Cwe::BROKEN_CRYPTO,
                "ssl.wrap_socket call with no SSL/TLS protocol version specified, the default SSLv23 could be insecure, possible security issue.",
            )
            .with_lineno(ctx.get_lineno_for_call_arg("ssl_version")),
        ));
    }
    Ok(None)
}
