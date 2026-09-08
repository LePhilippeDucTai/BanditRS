//! Port of `bandit/plugins/request_without_timeout.py` — see docs/spec/plugins.md.

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

/// `request_without_timeout` (B113).
pub fn request_without_timeout(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    let qualname = ctx
        .call_function_name_qual()
        .unwrap_or("")
        .split('.')
        .next()
        .unwrap_or("");
    let name = ctx.call_function_name().unwrap_or("");

    if qualname == "requests"
        && HTTP_VERBS.contains(&name)
        && ctx.check_call_arg_value("timeout", &[])?.is_none()
    {
        return Ok(Some(IssueDraft::new(
            Rank::Medium,
            Rank::Low,
            Cwe::UNCONTROLLED_RESOURCE_CONSUMPTION,
            format!("Call to {qualname} without timeout"),
        )));
    }
    if ((qualname == "requests" && HTTP_VERBS.contains(&name))
        || (qualname == "httpx" && HTTPX_ATTRS.contains(&name)))
        && ctx.check_call_arg_is("timeout", &PyValue::str("None"))? == Some(true)
    {
        return Ok(Some(IssueDraft::new(
            Rank::Medium,
            Rank::Low,
            Cwe::UNCONTROLLED_RESOURCE_CONSUMPTION,
            format!("Call to {qualname} with timeout set to None"),
        )));
    }
    Ok(None)
}
