//! Port of `bandit/plugins/huggingface_unsafe_download.py` — see docs/spec/plugins.md.

use ruff_python_ast::Expr;

use crate::ast::literal::PyValue;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;

const HF_MODULES: &[&str] = &["transformers", "datasets", "huggingface_hub"];

fn is_constant(e: &Expr) -> bool {
    matches!(
        e,
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BooleanLiteral(_) | Expr::NoneLiteral(_) | Expr::BytesLiteral(_) | Expr::EllipsisLiteral(_)
    )
}

/// `huggingface_unsafe_download` (B615).
pub fn huggingface_unsafe_download(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    if !HF_MODULES.iter().any(|m| ctx.is_module_imported_like(m)) {
        return Ok(None);
    }
    let qual = ctx.call_function_name_qual().unwrap_or("");
    let parts: Vec<&str> = qual.split('.').collect();
    let func_name = parts.last().copied().unwrap_or("");
    let required: &[&str] = match func_name {
        "from_pretrained" => &["transformers"],
        "load_dataset" => &["datasets"],
        "hf_hub_download" => &["huggingface_hub"],
        "snapshot_download" => &["huggingface_hub"],
        "repository_id" => &["huggingface_hub"],
        _ => return Ok(None),
    };
    if !required.iter().any(|m| parts.contains(m)) {
        return Ok(None);
    }
    if let Some(call) = ctx.call {
        for kw in &call.arguments.keywords {
            if let Some(arg) = &kw.arg {
                if (arg.as_str() == "revision" || arg.as_str() == "commit_id") && !is_constant(&kw.value) {
                    return Ok(None);
                }
            }
        }
    }
    let revision_value = ctx.get_call_arg_value("revision")?;
    let commit_id_value = ctx.get_call_arg_value("commit_id")?;
    let revision_to_check = match &revision_value {
        Some(v) if v.truthy() => revision_value,
        _ => commit_id_value,
    };
    if let Some(PyValue::Str(s)) = &revision_to_check {
        let trimmed = s.trim_matches(|c| c == '"' || c == '\'');
        let is_hex = trimmed.chars().all(|c| c.is_ascii_hexdigit());
        if trimmed.len() >= 7 && is_hex {
            return Ok(None);
        }
    }
    if let Some(first) = ctx.get_call_arg_at_position(0)? {
        if let Some(s) = first.as_str() {
            if s.starts_with("./") || s.starts_with('/') || s.starts_with("../") {
                return Ok(None);
            }
        }
    }
    Ok(Some(
        IssueDraft::new(
            Rank::Medium,
            Rank::High,
            Cwe::DOWNLOAD_OF_CODE_WITHOUT_INTEGRITY_CHECK,
            format!("Unsafe Hugging Face Hub download without revision pinning in {func_name}()"),
        )
        .with_lineno(ctx.get_lineno_for_call_arg(func_name)),
    ))
}
