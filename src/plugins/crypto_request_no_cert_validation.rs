//! Port of `bandit/plugins/crypto_request_no_cert_validation.py` — see docs/spec/plugins.md.

use crate::ast::literal::PyValue;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

const HTTP_VERBS: &[&str] = &["get", "options", "head", "post", "put", "patch", "delete"];
const HTTPX_ATTRS: &[&str] = &[
    "request",
    "stream",
    "Client",
    "AsyncClient",
    "get",
    "options",
    "head",
    "post",
    "put",
    "patch",
    "delete",
];

/// `request_with_no_cert_validation` (B501).
pub fn request_with_no_cert_validation(
    ctx: &Context<'_, '_>,
    _cfg: &PluginConfigs,
) -> PluginResult {
    let qualname = ctx
        .call_function_name_qual()
        .unwrap_or("")
        .split('.')
        .next()
        .unwrap_or("");
    let name = ctx.call_function_name().unwrap_or("");
    if ((qualname == "requests" && HTTP_VERBS.contains(&name))
        || (qualname == "httpx" && HTTPX_ATTRS.contains(&name)))
        && ctx.check_call_arg_is("verify", &PyValue::str("False"))? == Some(true)
    {
        return Ok(Some(
                IssueDraft::new(
                    Rank::High,
                    Rank::High,
                    Cwe::IMPROPER_CERT_VALIDATION,
                    format!("Call to {qualname} with verify=False disabling SSL certificate checks, security issue."),
                )
                .with_lineno(ctx.get_lineno_for_call_arg("verify")),
            ));
    }
    Ok(None)
}
