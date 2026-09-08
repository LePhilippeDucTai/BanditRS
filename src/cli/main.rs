//! The `bandit` command (port of `bandit/cli/main.py`). See
//! docs/spec/cli_formatters_tests.md §A.11 for the exact flow.

use std::collections::BTreeSet;
use std::path::Path;

use crate::cli::argparse::{self, Args, OutputTarget, ParseOutcome};
use crate::constants::{RANKING, Rank};
use crate::core::config::BanditConfig;
use crate::core::manager::{AggType, Manager};
use crate::core::registry;
use crate::core::test_set::TestSet;
use crate::formatters::{self, Output};
use crate::log::Level;
use crate::pycompat::configparser;

const DESCRIPTION: &str = "Bandit - a Python source code security analyzer";

fn epilog() -> String {
    let plugin_list: String = registry::all_ids_and_names()
        .iter()
        .map(|(id, name)| format!("{id}\t{name}"))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join("\n\t");
    format!(
        "\nCUSTOM FORMATTING\n-----------------\n\nAvailable tags:\n\n    {{abspath}}, {{relpath}}, {{line}}, {{col}}, {{test_id}},\n    {{severity}}, {{msg}}, {{confidence}}, {{range}}\n\nExample usage:\n\n    Default template:\n    bandit -r examples/ --format custom --msg-template \\\n    \"{{abspath}}:{{line}}: {{test_id}}[bandit]: {{severity}}: {{msg}}\"\n\n    Provides same output as:\n    bandit -r examples/ --format custom\n\n    Tags can also be formatted in python string.format() style:\n    bandit -r examples/ --format custom --msg-template \\\n    \"{{relpath:20.20s}}: {{line:03}}: {{test_id:^8}}: DEFECT: {{msg:>20}}\"\n\n    See python documentation for more information about formatting style:\n    https://docs.python.org/3/library/string.html\n\nThe following tests were discovered and loaded:\n-----------------------------------------------\n\t{plugin_list}"
    )
}

fn print_help() {
    print!("{}", argparse::USAGE);
    println!("\n{DESCRIPTION}\n");
    println!("positional arguments:");
    println!("  targets               source file(s) or directory(s) to be tested\n");
    println!("options:");
    println!("  -h, --help            show this help message and exit");
    println!("  -r, --recursive       find and process files in subdirectories");
    println!("  -a {{file,vuln}}, --aggregate {{file,vuln}}");
    println!("                        aggregate output by vulnerability (default) or by filename");
    println!("  -n CONTEXT_LINES, --number CONTEXT_LINES");
    println!("                        maximum number of code lines to output for each issue");
    println!("  -c CONFIG_FILE, --configfile CONFIG_FILE");
    println!(
        "                        optional config file to use for selecting plugins and overriding defaults"
    );
    println!("  -p PROFILE, --profile PROFILE");
    println!("                        profile to use (defaults to executing all tests)");
    println!("  -t TESTS, --tests TESTS");
    println!("                        comma-separated list of test IDs to run");
    println!("  -s SKIPS, --skip SKIPS");
    println!("                        comma-separated list of test IDs to skip");
    println!("  -l, --level           report only issues of a given severity level or higher");
    println!("  --severity-level {{all,low,medium,high}}");
    println!("                        report only issues of a given severity level or higher");
    println!("  -i, --confidence      report only issues of a given confidence level or higher");
    println!("  --confidence-level {{all,low,medium,high}}");
    println!("                        report only issues of a given confidence level or higher");
    println!(
        "  -f {{csv,custom,html,json,sarif,screen,txt,xml,yaml}}, --format {{csv,custom,html,json,sarif,screen,txt,xml,yaml}}"
    );
    println!("                        specify output format");
    println!("  --msg-template MSG_TEMPLATE");
    println!(
        "                        specify output message template (only usable with --format custom)"
    );
    println!("  -o [OUTPUT_FILE], --output [OUTPUT_FILE]");
    println!("                        write report to filename");
    println!("  -v, --verbose         output extra information like excluded and included files");
    println!("  -d, --debug           turn on debug mode");
    println!("  -q, --quiet, --silent");
    println!("                        only show output in the case of an error");
    println!("  --ignore-nosec        do not skip lines with # nosec comments");
    println!("  -x EXCLUDED_PATHS, --exclude EXCLUDED_PATHS");
    println!(
        "                        comma-separated list of paths (glob patterns supported) to exclude from scan"
    );
    println!("  -b BASELINE, --baseline BASELINE");
    println!(
        "                        path of a baseline report to compare against (only JSON-formatted files are accepted)"
    );
    println!("  --ini INI_PATH        path to a .bandit file that supplies command line arguments");
    println!("  --exit-zero           exit with 0, even with results found");
    println!("  --version             show program's version number and exit");
    println!("{}", epilog());
}

fn print_version() {
    println!("bandit {}\n  python version = n/a", crate::VERSION);
}

/// `os.walk(t)` looking for files named `.bandit`.
fn find_dot_bandit_files(target: &str, out: &mut Vec<String>) {
    let path = Path::new(target);
    if !path.is_dir() {
        return;
    }
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut children = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                children.push(p);
            } else if p.file_name().and_then(|n| n.to_str()) == Some(".bandit") {
                out.push(p.to_string_lossy().into_owned());
            }
        }
        stack.extend(children);
    }
}

/// Raised in place of Python's `sys.exit(2)` inside `_get_options_from_ini` when `ini_path` is
/// absent and more than one `.bandit` file is discovered under `targets` (`os.walk`). Carries the
/// full list of paths found so the caller can log and exit exactly as `main` does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultipleIniFiles(pub Vec<String>);

/// `_get_options_from_ini(ini_path, target)`. `Ok(None)` when nothing applies — no `ini_path` and
/// no `.bandit` file under `targets`, or the resolved file has no `[bandit]` section / can't be
/// read (`utils.parse_ini_file` swallows those and returns `None` after a warning).
/// `Err(MultipleIniFiles(paths))` when `ini_path` is absent and more than one `.bandit` file is
/// found; the caller is responsible for the log message and `exit(2)`.
pub fn get_options_from_ini(
    ini_path: Option<&str>,
    targets: &[String],
) -> Result<Option<indexmap::IndexMap<String, String>>, MultipleIniFiles> {
    let ini_file = if let Some(p) = ini_path {
        Some(p.to_string())
    } else {
        let mut found = Vec::new();
        for t in targets {
            find_dot_bandit_files(t, &mut found);
        }
        if found.len() > 1 {
            return Err(MultipleIniFiles(found));
        }
        if found.len() == 1 {
            crate::log_info!("main", "Found project level .bandit file: {}", found[0]);
            Some(found.remove(0))
        } else {
            None
        }
    };
    Ok(ini_file.and_then(|f| configparser::parse_ini_file(&f)))
}

/// `_log_option_source(default_val, arg_val, ini_val, option_name)`. Python truthiness applies to
/// every optional value here: `Some("")` counts as false, same as `if arg_val:` on an empty string.
pub fn log_option_source(
    default_val: Option<&str>,
    arg_val: Option<&str>,
    ini_val: Option<&str>,
    option_name: &str,
) -> Option<String> {
    fn truthy(v: Option<&str>) -> Option<&str> {
        v.filter(|s| !s.is_empty())
    }
    if default_val.is_none() {
        if let Some(v) = truthy(arg_val) {
            crate::log_info!("main", "Using command line arg for {}", option_name);
            return Some(v.to_string());
        }
        if let Some(v) = truthy(ini_val) {
            crate::log_info!("main", "Using ini file for {}", option_name);
            return Some(v.to_string());
        }
        return None;
    }
    if default_val == arg_val {
        return Some(truthy(ini_val).or(arg_val).unwrap_or_default().to_string());
    }
    arg_val.map(str::to_string)
}

fn apply_ini_options(args: &mut Args, ini: &indexmap::IndexMap<String, String>) {
    let defaults = Args::default();

    args.config_file = log_option_source(
        None,
        args.config_file.as_deref(),
        ini.get("configfile").map(String::as_str),
        "config file",
    );

    if let Some(v) = log_option_source(
        Some(defaults.excluded_paths.as_str()),
        Some(args.excluded_paths.as_str()),
        ini.get("exclude").map(String::as_str),
        "excluded paths",
    ) {
        args.excluded_paths = v;
    }

    args.skips = log_option_source(
        None,
        args.skips.as_deref(),
        ini.get("skips").map(String::as_str),
        "skipped tests",
    );
    args.tests = log_option_source(
        None,
        args.tests.as_deref(),
        ini.get("tests").map(String::as_str),
        "selected tests",
    );

    if let Some(ini_targets) = ini.get("targets").filter(|v| !v.is_empty()) {
        if args.targets == defaults.targets {
            crate::log_info!("main", "Using ini file for selected targets");
            args.targets = ini_targets.split(',').map(str::to_string).collect();
        } else {
            crate::log_info!("main", "Using command line arg for selected targets");
        }
    }

    if let Some(v) = ini.get("recursive")
        && !args.recursive
    {
        args.recursive = matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
    }
    if let Some(v) = log_option_source(
        Some(defaults.agg_type.as_str()),
        Some(args.agg_type.as_str()),
        ini.get("aggregate").map(String::as_str),
        "aggregate output type",
    ) {
        args.agg_type = v;
    }
    if let Some(n) = ini
        .get("number")
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v != 0)
        && args.context_lines == defaults.context_lines
    {
        args.context_lines = n;
    }
    args.profile = log_option_source(
        None,
        args.profile.as_deref(),
        ini.get("profile").map(String::as_str),
        "profile",
    );
    if let Some(n) = ini.get("level").and_then(|v| v.parse::<u32>().ok())
        && args.severity == defaults.severity
    {
        args.severity = n;
    }
    if let Some(n) = ini.get("confidence").and_then(|v| v.parse::<u32>().ok())
        && args.confidence == defaults.confidence
    {
        args.confidence = n;
    }
    let default_output_format = formatters::default_format();
    let current_output_format = args
        .output_format
        .clone()
        .unwrap_or_else(|| default_output_format.to_string());
    args.output_format = log_option_source(
        Some(default_output_format),
        Some(current_output_format.as_str()),
        ini.get("format").map(String::as_str),
        "output format",
    );
    args.msg_template = log_option_source(
        None,
        args.msg_template.as_deref(),
        ini.get("msg-template").map(String::as_str),
        "output message template",
    );
    if let Some(v) = ini.get("output").filter(|v| !v.is_empty())
        && args.output_file == OutputTarget::Stdout
    {
        args.output_file = OutputTarget::File(v.clone());
    }
    if let Some(v) = ini.get("verbose")
        && !args.verbose
    {
        args.verbose = matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
    }
    if let Some(v) = ini.get("debug")
        && !args.debug
    {
        args.debug = matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
    }
    if let Some(v) = ini.get("quiet")
        && !args.quiet
    {
        args.quiet = matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
    }
    if let Some(v) = ini.get("ignore-nosec")
        && !args.ignore_nosec
    {
        args.ignore_nosec = matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
    }
    args.baseline = log_option_source(
        None,
        args.baseline.as_deref(),
        ini.get("baseline").map(String::as_str),
        "path of a baseline report",
    );
}

/// `_init_logger(log_level=logging.INFO, log_format=None)`: resets the logger's threshold and
/// output format, falling back to `constants::LOG_FORMAT_STRING` when `log_format` is absent
/// (Python's default argument value).
pub fn init_logger(level: Level, log_format: Option<&str>) {
    crate::log::set_level(level);
    crate::log::set_format(log_format.unwrap_or(crate::constants::LOG_FORMAT_STRING));
    crate::log_debug!("main", "logging initialized");
}

/// Run the `bandit` command with the given arguments (without the program
/// name) and return the process exit code.
pub fn main(argv: Vec<String>) -> i32 {
    let debug_early = argv.iter().any(|a| a == "-d" || a == "--debug");
    init_logger(
        if debug_early {
            Level::Debug
        } else {
            Level::Info
        },
        None,
    );

    let mut args = match argparse::parse(&argv) {
        ParseOutcome::Run(a) => *a,
        ParseOutcome::Help => {
            print_help();
            return 0;
        }
        ParseOutcome::Version => {
            print_version();
            return 0;
        }
        ParseOutcome::Exit(code) => return code,
    };

    if args.output_format.as_deref() != Some("custom") && args.msg_template.is_some() {
        eprint!("{}", argparse::USAGE);
        eprintln!("bandit: error: --msg-template can only be used with --format=custom");
        return 2;
    }

    let ini_options = match get_options_from_ini(args.ini_path.as_deref(), &args.targets) {
        Ok(opt) => opt,
        Err(MultipleIniFiles(paths)) => {
            crate::log_error!(
                "main",
                "Multiple .bandit files found - scan separately or choose one with --ini\n\t{}",
                paths.join(", ")
            );
            return 2;
        }
    };
    if let Some(ini) = ini_options {
        apply_ini_options(&mut args, &ini);
    }

    let b_conf = match BanditConfig::new(args.config_file.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            crate::log_error!("main", "{}", e);
            return 2;
        }
    };

    if args.targets.is_empty() {
        eprint!("{}", argparse::USAGE);
        return 2;
    }

    if let Some(fmt) = b_conf.get_option("log_format").and_then(|v| v.as_str()) {
        init_logger(Level::Debug, Some(fmt));
    }
    if args.quiet {
        init_logger(Level::Warning, None);
    }

    let mut profile = if let Some(name) = &args.profile {
        match b_conf.profile(name) {
            Some(p) => p,
            None => {
                crate::log_error!(
                    "main",
                    "Unable to find profile ({}) in config file: {}",
                    name,
                    args.config_file.as_deref().unwrap_or("")
                );
                return 2;
            }
        }
    } else {
        b_conf.default_profile()
    };

    crate::log_info!(
        "main",
        "profile include tests: {}",
        if profile.include.is_empty() {
            "None".to_string()
        } else {
            profile
                .include
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(",")
        }
    );
    crate::log_info!(
        "main",
        "profile exclude tests: {}",
        if profile.exclude.is_empty() {
            "None".to_string()
        } else {
            profile
                .exclude
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(",")
        }
    );
    crate::log_info!(
        "main",
        "cli include tests: {}",
        args.tests.as_deref().unwrap_or("None")
    );
    crate::log_info!(
        "main",
        "cli exclude tests: {}",
        args.skips.as_deref().unwrap_or("None")
    );

    if let Some(t) = &args.tests {
        profile.include.extend(t.split(',').map(str::to_string));
    }
    if let Some(s) = &args.skips {
        profile.exclude.extend(s.split(',').map(str::to_string));
    }
    for inc in &profile.include {
        if !registry::check_id(inc) {
            crate::log_warning!("main", "Unknown test found in profile: {}", inc);
        }
    }
    for exc in &profile.exclude {
        if !registry::check_id(exc) {
            crate::log_warning!("main", "Unknown test found in profile: {}", exc);
        }
    }
    let overlap: Vec<&String> = profile.include.intersection(&profile.exclude).collect();
    if !overlap.is_empty() {
        crate::log_error!(
            "main",
            "Non-exclusive include/exclude test sets: {{{}}}",
            overlap
                .iter()
                .map(|s| format!("'{s}'"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        return 2;
    }

    let test_set = TestSet::new(&b_conf, &profile);
    let agg_type = if args.agg_type == "vuln" {
        AggType::Vuln
    } else {
        AggType::File
    };
    let mut b_mgr = Manager::new(b_conf, agg_type, test_set);
    b_mgr.debug = args.debug;
    b_mgr.verbose = args.verbose;
    b_mgr.quiet = args.quiet;
    b_mgr.ignore_nosec = args.ignore_nosec;

    let baseline_formatters = formatters::BASELINE_FORMATTERS;
    let effective_format = args
        .output_format
        .clone()
        .unwrap_or_else(|| formatters::default_format().to_string());

    if let Some(baseline_path) = &args.baseline {
        match std::fs::read_to_string(baseline_path) {
            Ok(data) => b_mgr.populate_baseline(&data),
            Err(_) => {
                crate::log_warning!("main", "Could not open baseline report: {}", baseline_path);
                return 2;
            }
        }
        if !baseline_formatters.contains(&effective_format.as_str()) {
            crate::log_warning!(
                "main",
                "Baseline must be used with one of the following formats: {:?}",
                baseline_formatters
            );
            return 2;
        }
    }

    if effective_format != "json"
        && let Some(cf) = &args.config_file
    {
        crate::log_info!("main", "using config: {}", cf);
    }

    b_mgr.discover_files(&args.targets, args.recursive, Some(&args.excluded_paths));

    if b_mgr.test_set.is_empty() {
        crate::log_error!("main", "No tests would be run, please check the profile.");
        return 2;
    }

    // Read stdin now (before scanning) when "-" is a target, matching
    // Manager::run_tests's handling of the pseudo-file.
    b_mgr.run_tests();

    let sev_level = Rank::from_index((args.severity.clamp(1, 4) - 1) as usize).unwrap_or(Rank::Low);
    let conf_level =
        Rank::from_index((args.confidence.clamp(1, 4) - 1) as usize).unwrap_or(Rank::Low);
    let _ = RANKING;

    let mut output = match &args.output_file {
        OutputTarget::Stdout | OutputTarget::Bare => Output::Stdout,
        OutputTarget::File(path) => match std::fs::File::create(path) {
            Ok(file) => Output::File {
                name: path.clone(),
                file,
            },
            Err(e) => {
                crate::log_error!("main", "{}", e);
                return 2;
            }
        },
    };

    if let Err(e) = formatters::output_results(
        &b_mgr,
        args.context_lines,
        sev_level,
        conf_level,
        &mut output,
        &effective_format,
        args.msg_template.as_deref(),
    ) {
        crate::log_error!("main", "{}", e);
        return 2;
    }

    if b_mgr.results_count(sev_level, conf_level) > 0 && !args.exit_zero {
        1
    } else {
        0
    }
}
