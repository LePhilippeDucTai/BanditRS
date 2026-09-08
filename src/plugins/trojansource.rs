//! Port of `bandit/plugins/trojansource.py` — see docs/spec/plugins.md.

use crate::ast::literal::repr_str;
use crate::constants::Rank;
use crate::core::context::Context;
use crate::core::issue::{Cwe, IssueDraft, LineRange};
use crate::core::plugin_config::PluginConfigs;
use crate::plugins::PluginResult;
use crate::pycompat::splitlines::str_splitlines;

const BIDI_CHARACTERS: &[char] = &['\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}', '\u{200f}'];

/// `trojansource` (B613, `File`).
pub fn trojansource(ctx: &Context<'_, '_>, _cfg: &PluginConfigs) -> PluginResult {
    for (i, line) in str_splitlines(&ctx.file.text).into_iter().enumerate() {
        let lineno = (i + 1) as u32;
        for &ch in BIDI_CHARACTERS {
            if let Some(idx) = line.chars().position(|c| c == ch) {
                let col_offset = (idx + 1) as u32;
                let text = format!("A Python source file contains bidirectional control characters ({}).", repr_str(&ch.to_string()));
                return Ok(Some(
                    IssueDraft::new(Rank::High, Rank::Medium, Cwe::INAPPROPRIATE_ENCODING_FOR_OUTPUT_CONTEXT, text)
                        .with_lineno(Some(lineno))
                        .with_col_offset(col_offset)
                        .with_linerange(LineRange::single(lineno)),
                ));
            }
        }
    }
    Ok(None)
}
