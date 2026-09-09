//! Normalisation for the CLI differential matrix (WP-18), mirroring the `SUBS`
//! table and the profile-line sort of `scripts/cli_matrix.py`. The two must
//! stay in step: the golden is written by the script and read back here, so a
//! rule present on one side only would either hide a real divergence or
//! manufacture a fake one.

use std::sync::OnceLock;

use regex::Regex;

macro_rules! cached {
    ($name:ident, $pattern:literal) => {
        fn $name() -> &'static Regex {
            static RE: OnceLock<Regex> = OnceLock::new();
            RE.get_or_init(|| Regex::new($pattern).expect("static regex"))
        }
    };
}

cached!(working_line, r"\A\s*Working\.\.\..*\n");
cached!(generated_at_json, r#""generated_at": "[^"]*""#);
cached!(generated_at_yaml, r"generated_at: '[^']*'");
cached!(end_time_utc, r#""endTimeUtc": "[^"]*""#);
cached!(run_started, r"Run started:[^\n]*");
cached!(
    tool_version_pair,
    r#"(?s)"version": "[^"]*",(\s*)"semanticVersion": "[^"]*""#
);
cached!(readthedocs_version, r"readthedocs\.io/en/[^/]+/");
cached!(memory_address, r"0x[0-9a-fA-F]+");
cached!(version_line, r"(?m)^bandit \S+$");
cached!(python_version_line, r"(?m)^\s*python version = .*\n");
cached!(
    running_on_python,
    r"(?m)^\[main\]\tINFO\trunning on Python .*\n"
);
cached!(commit_sha, r"\b[0-9a-f]{40}\b");
cached!(
    baseline_scratch,
    r"/tmp/[^/\s]+/_bandit_baseline_run\.json_"
);
cached!(
    profile_line,
    r"(?m)^(\[main\]\tINFO\tprofile (?:include|exclude) tests: )(.+)$"
);

/// `bandit/cli/main.py:_log_info` joins a Python `set`, whose iteration order
/// comes out of a randomised string hash and changes between runs; BanditRS is
/// deterministic. Sorting both sides is the only way to compare the line at
/// all (DEVIATIONS.md #17).
fn sort_profile_ids(text: &str) -> String {
    profile_line()
        .replace_all(text, |caps: &regex::Captures| {
            let ids = &caps[2];
            if ids == "None" {
                return caps[0].to_string();
            }
            let mut parts: Vec<&str> = ids.split(',').collect();
            parts.sort_unstable();
            format!("{}{}", &caps[1], parts.join(","))
        })
        .into_owned()
}

/// Normalises one stream. `workspace` is the per-case temporary directory,
/// replaced by `<WS>` because the `custom` format prints `{abspath}`.
pub fn normalize(text: &str, workspace: &str) -> String {
    let text = working_line().replace_all(text, "");
    let text = generated_at_json().replace_all(&text, r#""generated_at": "<TS>""#);
    let text = generated_at_yaml().replace_all(&text, "generated_at: '<TS>'");
    let text = end_time_utc().replace_all(&text, r#""endTimeUtc": "<TS>""#);
    let text = run_started().replace_all(&text, "Run started:<TS>");
    let text = tool_version_pair().replace_all(
        &text,
        "\"version\": \"<VER>\",$1\"semanticVersion\": \"<VER>\"",
    );
    let text = readthedocs_version().replace_all(&text, "readthedocs.io/en/X/");
    let text = memory_address().replace_all(&text, "0x0");
    let text = version_line().replace_all(&text, "bandit <VER>");
    let text = python_version_line().replace_all(&text, "");
    let text = running_on_python().replace_all(&text, "");
    let text = commit_sha().replace_all(&text, "<SHA>");
    let text = baseline_scratch().replace_all(&text, "<TMPDIR>/_bandit_baseline_run.json_");
    sort_profile_ids(&text.replace(workspace, "<WS>"))
}
