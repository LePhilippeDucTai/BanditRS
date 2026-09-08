//! Port of `bandit/plugins/weak_cryptographic_key.py` — see docs/spec/plugins.md.

use crate::ast::literal::PyValue;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft};
use crate::core::plugin_config::{PluginConfigs, WeakKeyConfig};
use crate::plugins::PluginResult;

const CURVE_KEY_SIZES: &[(&str, i128)] = &[
    ("SECT571K1", 571),
    ("SECT571R1", 570),
    ("SECP521R1", 521),
    ("BrainpoolP512R1", 512),
    ("SECT409K1", 409),
    ("SECT409R1", 409),
    ("BrainpoolP384R1", 384),
    ("SECP384R1", 384),
    ("SECT283K1", 283),
    ("SECT283R1", 283),
    ("BrainpoolP256R1", 256),
    ("SECP256K1", 256),
    ("SECP256R1", 256),
    ("SECT233K1", 233),
    ("SECT233R1", 233),
    ("SECP224R1", 224),
    ("SECP192R1", 192),
    ("SECT163K1", 163),
    ("SECT163R2", 163),
];

/// `X or Y`: keep `a` when truthy, otherwise fall back to `b`.
fn truthy_or<'a>(a: Option<PyValue<'a>>, b: Option<PyValue<'a>>) -> Option<PyValue<'a>> {
    match &a {
        Some(v) if v.truthy() => a,
        _ => b,
    }
}

fn classify_key_size(cfg: &WeakKeyConfig, key_type: &str, key_size: i128) -> PluginResult {
    let (high, medium) = match key_type {
        "DSA" => (cfg.weak_key_size_dsa_high.clone()?, cfg.weak_key_size_dsa_medium.clone()?),
        "RSA" => (cfg.weak_key_size_rsa_high.clone()?, cfg.weak_key_size_rsa_medium.clone()?),
        "EC" => (cfg.weak_key_size_ec_high.clone()?, cfg.weak_key_size_ec_medium.clone()?),
        _ => unreachable!("key_type is one of DSA/RSA/EC"),
    };
    for (size, level) in [(high, Rank::High), (medium, Rank::Medium)] {
        if key_size < size as i128 {
            return Ok(Some(IssueDraft::new(
                level,
                Rank::High,
                Cwe::INADEQUATE_ENCRYPTION_STRENGTH,
                format!("{key_type} key sizes below {size} bits are considered breakable. "),
            )));
        }
    }
    Ok(None)
}

fn crypto_io(ctx: &Context<'_, '_>, cfg: &WeakKeyConfig) -> PluginResult {
    let qual = ctx.call_function_name_qual().unwrap_or("");
    let key_type = match qual {
        "cryptography.hazmat.primitives.asymmetric.dsa.generate_private_key" => "DSA",
        "cryptography.hazmat.primitives.asymmetric.rsa.generate_private_key" => "RSA",
        "cryptography.hazmat.primitives.asymmetric.ec.generate_private_key" => "EC",
        _ => return Ok(None),
    };
    if key_type == "DSA" || key_type == "RSA" {
        let pos = if key_type == "RSA" { 1 } else { 0 };
        let key_size_val = truthy_or(ctx.get_call_arg_value("key_size")?, ctx.get_call_arg_at_position(pos)?);
        let key_size_val = truthy_or(key_size_val, Some(PyValue::Int(2048))).expect("default provided");
        if key_size_val.is_str() {
            return Ok(None);
        }
        let Some(key_size) = key_size_val.as_int() else { return Ok(None) };
        return classify_key_size(cfg, key_type, key_size);
    }
    // EC
    let curve_val = ctx.get_call_arg_value("curve")?;
    let curve_val = match &curve_val {
        Some(v) if v.truthy() => curve_val,
        _ => {
            let args = ctx.call_args()?;
            args.first().cloned()
        }
    };
    let curve_name = curve_val.as_ref().and_then(PyValue::as_str);
    let key_size = curve_name.and_then(|n| CURVE_KEY_SIZES.iter().find(|(k, _)| *k == n).map(|(_, v)| *v)).unwrap_or(224);
    classify_key_size(cfg, "EC", key_size)
}

fn pycrypto(ctx: &Context<'_, '_>, cfg: &WeakKeyConfig) -> PluginResult {
    let qual = ctx.call_function_name_qual().unwrap_or("");
    let key_type = match qual {
        "Crypto.PublicKey.DSA.generate" | "Cryptodome.PublicKey.DSA.generate" => "DSA",
        "Crypto.PublicKey.RSA.generate" | "Cryptodome.PublicKey.RSA.generate" => "RSA",
        _ => return Ok(None),
    };
    let key_size_val = truthy_or(ctx.get_call_arg_value("bits")?, ctx.get_call_arg_at_position(0)?);
    let key_size_val = truthy_or(key_size_val, Some(PyValue::Int(2048))).expect("default provided");
    if key_size_val.is_str() {
        return Ok(None);
    }
    let Some(key_size) = key_size_val.as_int() else { return Ok(None) };
    classify_key_size(cfg, key_type, key_size)
}

/// `weak_cryptographic_key` (B505).
pub fn weak_cryptographic_key(ctx: &Context<'_, '_>, cfg: &PluginConfigs) -> PluginResult {
    let cfg = &cfg.weak_cryptographic_key;
    if let Some(draft) = crypto_io(ctx, cfg)? {
        return Ok(Some(draft));
    }
    pycrypto(ctx, cfg)
}
