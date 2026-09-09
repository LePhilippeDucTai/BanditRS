//! Shared normalisation applied to a freshly generated report before it is
//! compared to a committed golden file (`tests/golden/**`). Mirrors the
//! Python normalisation embedded in `scripts/gen_golden.sh` — see that
//! script's header comment for the exact list and the DEVIATIONS.md entries
//! it encodes. The golden files themselves are already normalised (and, for
//! YAML, already corrected for DEVIATIONS.md #10/#11), so only the binary's
//! fresh output needs this pass; applying it is otherwise a no-op.

use std::sync::OnceLock;

use regex::Regex;

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("static regex")
}

fn working_line() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| re(r"(?m)^Working\.\.\..*\n"))
}

fn generated_at_json() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| re(r#""generated_at": "[^"]*""#))
}

fn generated_at_yaml() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| re(r"generated_at: '[^']*'"))
}

fn end_time_utc() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| re(r#""endTimeUtc": "[^"]*""#))
}

fn run_started() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| re(r"Run started:[^\n]*"))
}

fn tool_version_pair() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| re(r#"(?s)"version": "[^"]*",(\s*)"semanticVersion": "[^"]*""#))
}

fn readthedocs_version() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| re(r"readthedocs\.io/en/[^/]+/"))
}

fn memory_address() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| re(r"0x[0-9a-fA-F]+"))
}

/// Normalises non-deterministic / environment-dependent content: timestamps
/// (`generated_at`, `endTimeUtc`, "Run started:"), the SARIF driver's own
/// version pair, the readthedocs version segment in `more_info` URLs
/// (DEVIATIONS.md #12), memory addresses (DEVIATIONS.md #5) and, given
/// `root`, the absolute repository path (the `custom` format's `abspath`).
pub fn normalize(text: &str, root: &str) -> String {
    let text = working_line().replace_all(text, "");
    let text = generated_at_json().replace_all(&text, r#""generated_at": "<TS>""#);
    let text = generated_at_yaml().replace_all(&text, "generated_at: '<TS>'");
    let text = end_time_utc().replace_all(&text, r#""endTimeUtc": "<TS>""#);
    let text = run_started().replace_all(&text, "Run started:<TS>");
    let text = tool_version_pair().replace_all(&text, "\"version\": \"<VER>\",$1\"semanticVersion\": \"<VER>\"");
    let text = readthedocs_version().replace_all(&text, "readthedocs.io/en/X/");
    let text = memory_address().replace_all(&text, "0x0");
    text.replace(root, "<ROOT>")
}
