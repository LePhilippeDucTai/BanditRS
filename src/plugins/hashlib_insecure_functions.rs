//! Port of `bandit/plugins/hashlib_insecure_functions.py` — see docs/spec/plugins.md.

use crate::ast::literal::PyValue;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

const WEAK_HASHES: &[&str] = &["md4", "md5", "sha", "sha1"];
const WEAK_CRYPT_HASHES: &[&str] = &["METHOD_CRYPT", "METHOD_MD5", "METHOD_BLOWFISH"];

/// `hashlib` (B324, entry point `hashlib_insecure_functions`).
pub fn hashlib(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    let qual = ctx.call_function_name_qual().unwrap_or("");
    let parts: Vec<&str> = qual.split('.').collect();
    let func = parts.last().copied().unwrap_or("");
    if parts.contains(&"hashlib") {
        return hashlib_func(ctx, func);
    }
    if parts.contains(&"crypt") && (func == "crypt" || func == "mksalt") {
        return crypt_crypt(ctx, func);
    }
    Ok(None)
}

fn used_for_security(keywords: &crate::core::context::Keywords<'_>) -> bool {
    let default = PyValue::str("True");
    keywords.get_or("usedforsecurity", &default).py_eq(&default)
}

fn hashlib_func(ctx: &Context<'_, '_>, func: &str) -> PluginResult {
    let keywords = ctx.call_keywords()?.unwrap_or_default();
    if WEAK_HASHES.contains(&func) {
        if used_for_security(&keywords) {
            return Ok(Some(
                IssueDraft::new(
                    Rank::High,
                    Rank::High,
                    Cwe::BROKEN_CRYPTO,
                    format!(
                        "Use of weak {} hash for security. Consider usedforsecurity=False",
                        func.to_uppercase()
                    ),
                )
                .with_lineno(ctx.lineno()),
            ));
        }
        return Ok(None);
    }
    if func == "new" {
        let args = ctx.call_args()?;
        let name_val = if let Some(v) = args.first() {
            Some(v.clone())
        } else {
            keywords.get("name").cloned()
        };
        if let Some(PyValue::Str(s)) = &name_val
            && WEAK_HASHES.contains(&s.to_lowercase().as_str())
            && used_for_security(&keywords)
        {
            return Ok(Some(
                IssueDraft::new(
                    Rank::High,
                    Rank::High,
                    Cwe::BROKEN_CRYPTO,
                    format!(
                        "Use of weak {} hash for security. Consider usedforsecurity=False",
                        s.to_uppercase()
                    ),
                )
                .with_lineno(ctx.lineno()),
            ));
        }
    }
    Ok(None)
}

fn crypt_crypt(ctx: &Context<'_, '_>, func: &str) -> PluginResult {
    let args = ctx.call_args()?;
    let keywords = ctx.call_keywords()?.unwrap_or_default();
    let name_val = if func == "crypt" {
        if args.len() > 1 {
            Some(args[1].clone())
        } else {
            keywords.get("salt").cloned()
        }
    } else if !args.is_empty() {
        Some(args[0].clone())
    } else {
        keywords.get("method").cloned()
    };
    if let Some(PyValue::Str(s)) = name_val {
        let up = s.to_uppercase();
        if WEAK_CRYPT_HASHES.contains(&up.as_str()) {
            return Ok(Some(
                IssueDraft::new(
                    Rank::Medium,
                    Rank::High,
                    Cwe::BROKEN_CRYPTO,
                    format!("Use of insecure crypt.{up} hash function."),
                )
                .with_lineno(ctx.lineno()),
            ));
        }
    }
    Ok(None)
}
