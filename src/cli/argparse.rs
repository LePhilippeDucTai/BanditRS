//! Argument parsing for the `bandit` command (docs/spec/cli_formatters_tests.md
//! §A.9). A hand-written parser tailored to bandit's exact option set (not a
//! generic argparse clone): long/short options, `--opt=value`, bundled short
//! flags (`-lll`, `-rv`), `--` end-of-options, and the mutually exclusive
//! `-l`/`--severity-level`, `-i`/`--confidence-level`, `-v`/`-q` groups.
//!
//! Errors print `usage: ...` + `bandit: error: <message>` to stderr and
//! return exit code 2; `-h`/`--help` prints the full help (built in
//! `cli::main`, which owns the plugin-list epilog) and returns exit code 0.

use crate::constants::EXCLUDE;

pub const USAGE: &str = "usage: bandit [-h] [-r] [-a {file,vuln}] [-n CONTEXT_LINES] [-c CONFIG_FILE]\n              [-p PROFILE] [-t TESTS] [-s SKIPS] [-l | --severity-level {all,low,medium,high}]\n              [-i | --confidence-level {all,low,medium,high}]\n              [-f {csv,custom,html,json,sarif,screen,txt,xml,yaml}] [--msg-template MSG_TEMPLATE]\n              [-o [OUTPUT_FILE]] [-v | -q] [-d] [--ignore-nosec] [-x EXCLUDED_PATHS]\n              [-b BASELINE] [--ini INI_PATH] [--exit-zero] [--version]\n              [targets ...]\n";

/// Parsed CLI arguments (`argparse.Namespace`).
#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    pub targets: Vec<String>,
    pub recursive: bool,
    pub agg_type: String,
    pub context_lines: i64,
    pub config_file: Option<String>,
    pub profile: Option<String>,
    pub tests: Option<String>,
    pub skips: Option<String>,
    pub severity: u32,
    pub confidence: u32,
    pub output_format: Option<String>,
    pub msg_template: Option<String>,
    /// `-o`/`--output` (`nargs="?"`, `default=sys.stdout`): absent → `Stdout`;
    /// bare `-o` → `Bare` (Python: `None`, a `nargs="?"` default with no
    /// `const`; treated the same as `Stdout` here — a corner case no test
    /// exercises, see DEVIATIONS.md); `-o path` → `File(path)`.
    pub output_file: OutputTarget,
    pub verbose: bool,
    pub debug: bool,
    pub quiet: bool,
    pub ignore_nosec: bool,
    pub excluded_paths: String,
    pub baseline: Option<String>,
    pub ini_path: Option<String>,
    pub exit_zero: bool,
}

impl Default for Args {
    fn default() -> Args {
        Args {
            targets: Vec::new(),
            recursive: false,
            agg_type: "file".to_string(),
            context_lines: 3,
            config_file: None,
            profile: None,
            tests: None,
            skips: None,
            severity: 1,
            confidence: 1,
            output_format: None,
            msg_template: None,
            output_file: OutputTarget::Stdout,
            verbose: false,
            debug: false,
            quiet: false,
            ignore_nosec: false,
            excluded_paths: EXCLUDE.join(","),
            baseline: None,
            ini_path: None,
            exit_zero: false,
        }
    }
}

/// The outcome of parsing: either arguments to run with, or a request to
/// exit immediately with the given code (help/version/usage already printed).
/// `-o`/`--output` destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputTarget {
    Stdout,
    Bare,
    File(String),
}

pub enum ParseOutcome {
    Run(Box<Args>),
    Help,
    Version,
    /// A usage/error message was already printed to stderr; exit with this code.
    Exit(i32),
}

const FORMAT_CHOICES: &[&str] = &[
    "csv", "custom", "html", "json", "sarif", "screen", "txt", "xml", "yaml",
];
const SEV_CONF_CHOICES: &[&str] = &["all", "low", "medium", "high"];

fn error(message: &str) -> i32 {
    eprint!("{USAGE}");
    eprintln!("bandit: error: {message}");
    2
}

/// `parser.parse_args(argv)`.
pub fn parse(argv: &[String]) -> ParseOutcome {
    let mut args = Args::default();
    let mut level_set = false;
    let mut severity_string: Option<String> = None;
    let mut confidence_string: Option<String> = None;
    let mut conf_set = false;
    let mut verbose_or_quiet: Option<&'static str> = None;

    let mut i = 0;
    let mut only_positionals = false;
    while i < argv.len() {
        let arg = argv[i].clone();

        if only_positionals || arg == "-" || !arg.starts_with('-') {
            args.targets.push(arg);
            i += 1;
            continue;
        }
        if arg == "--" {
            only_positionals = true;
            i += 1;
            continue;
        }
        if arg == "-h" || arg == "--help" {
            return ParseOutcome::Help;
        }
        if arg == "--version" {
            return ParseOutcome::Version;
        }

        if let Some(rest) = arg.strip_prefix("--") {
            let (name, inline) = match rest.split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (rest, None),
            };
            macro_rules! take_value {
                () => {
                    match inline {
                        Some(v) => v,
                        None => {
                            i += 1;
                            match argv.get(i) {
                                Some(v) => v.clone(),
                                None => {
                                    return ParseOutcome::Exit(error(&format!(
                                        "argument --{name}: expected one argument"
                                    )));
                                }
                            }
                        }
                    }
                };
            }
            match name {
                "recursive" => args.recursive = true,
                "aggregate" => {
                    let v = take_value!();
                    if !["file", "vuln"].contains(&v.as_str()) {
                        return ParseOutcome::Exit(error(&format!(
                            "argument -a/--aggregate: invalid choice: '{v}' (choose from 'file', 'vuln')"
                        )));
                    }
                    args.agg_type = v;
                }
                "number" => {
                    let v = take_value!();
                    match v.parse() {
                        Ok(n) => args.context_lines = n,
                        Err(_) => {
                            return ParseOutcome::Exit(error(&format!(
                                "argument -n/--number: invalid int value: '{v}'"
                            )));
                        }
                    }
                }
                "configfile" => args.config_file = Some(take_value!()),
                "profile" => args.profile = Some(take_value!()),
                "tests" => args.tests = Some(take_value!()),
                "skip" => args.skips = Some(take_value!()),
                "severity-level" => {
                    if level_set {
                        return ParseOutcome::Exit(error(
                            "argument --severity-level: not allowed with argument -l/--level",
                        ));
                    }
                    let v = take_value!();
                    if !SEV_CONF_CHOICES.contains(&v.as_str()) {
                        return ParseOutcome::Exit(error(&format!(
                            "argument --severity-level: invalid choice: '{v}' (choose from 'all', 'low', 'medium', 'high')"
                        )));
                    }
                    severity_string = Some(v);
                    level_set = true;
                }
                "confidence-level" => {
                    if conf_set {
                        return ParseOutcome::Exit(error(
                            "argument --confidence-level: not allowed with argument -i/--confidence",
                        ));
                    }
                    let v = take_value!();
                    if !SEV_CONF_CHOICES.contains(&v.as_str()) {
                        return ParseOutcome::Exit(error(&format!(
                            "argument --confidence-level: invalid choice: '{v}' (choose from 'all', 'low', 'medium', 'high')"
                        )));
                    }
                    confidence_string = Some(v);
                    conf_set = true;
                }
                "format" => {
                    let v = take_value!();
                    if !FORMAT_CHOICES.contains(&v.as_str()) {
                        return ParseOutcome::Exit(error(&format!(
                            "argument -f/--format: invalid choice: '{v}' (choose from 'csv', 'custom', 'html', 'json', 'sarif', 'screen', 'txt', 'xml', 'yaml')"
                        )));
                    }
                    args.output_format = Some(v);
                }
                "msg-template" => args.msg_template = Some(take_value!()),
                "output" => args.output_file = OutputTarget::File(take_value!()),
                "verbose" => {
                    if verbose_or_quiet == Some("quiet") {
                        return ParseOutcome::Exit(error(
                            "argument -v/--verbose: not allowed with argument -q/--quiet",
                        ));
                    }
                    args.verbose = true;
                    verbose_or_quiet = Some("verbose");
                }
                "debug" => args.debug = true,
                "quiet" | "silent" => {
                    if verbose_or_quiet == Some("verbose") {
                        return ParseOutcome::Exit(error(
                            "argument -q/--quiet: not allowed with argument -v/--verbose",
                        ));
                    }
                    args.quiet = true;
                    verbose_or_quiet = Some("quiet");
                }
                "ignore-nosec" => args.ignore_nosec = true,
                "exclude" => args.excluded_paths = take_value!(),
                "baseline" => args.baseline = Some(take_value!()),
                "ini" => args.ini_path = Some(take_value!()),
                "exit-zero" => args.exit_zero = true,
                _ => {
                    return ParseOutcome::Exit(error(&format!("unrecognized arguments: --{name}")));
                }
            }
            i += 1;
            continue;
        }

        // Short option cluster: -rv, -lll, -n3, -n 3, -ovalue, -o.
        let chars: Vec<char> = arg[1..].chars().collect();
        let mut ci = 0;
        while ci < chars.len() {
            let c = chars[ci];
            match c {
                'h' => return ParseOutcome::Help,
                'r' => args.recursive = true,
                'd' => args.debug = true,
                'v' => {
                    if verbose_or_quiet == Some("quiet") {
                        return ParseOutcome::Exit(error(
                            "argument -v/--verbose: not allowed with argument -q/--quiet",
                        ));
                    }
                    args.verbose = true;
                    verbose_or_quiet = Some("verbose");
                }
                'q' => {
                    if verbose_or_quiet == Some("verbose") {
                        return ParseOutcome::Exit(error(
                            "argument -q/--quiet: not allowed with argument -v/--verbose",
                        ));
                    }
                    args.quiet = true;
                    verbose_or_quiet = Some("quiet");
                }
                'l' => {
                    if level_set && severity_string.is_some() {
                        return ParseOutcome::Exit(error(
                            "argument -l/--level: not allowed with argument --severity-level",
                        ));
                    }
                    if !level_set {
                        args.severity = 0;
                    }
                    args.severity += 1;
                    level_set = true;
                }
                'i' => {
                    if conf_set && confidence_string.is_some() {
                        return ParseOutcome::Exit(error(
                            "argument -i/--confidence: not allowed with argument --confidence-level",
                        ));
                    }
                    if !conf_set {
                        args.confidence = 0;
                    }
                    args.confidence += 1;
                    conf_set = true;
                }
                'x' | 'a' | 'n' | 'c' | 'p' | 't' | 's' | 'f' | 'o' | 'b' => {
                    let rest: String = chars[ci + 1..].iter().collect();
                    let (value, consumed_next) = if !rest.is_empty() {
                        (Some(rest), false)
                    } else if c == 'o' {
                        // nargs="?": only consume the next token if it doesn't look like another option.
                        match argv.get(i + 1) {
                            Some(next) if !next.starts_with('-') || next == "-" => {
                                (Some(next.clone()), true)
                            }
                            _ => (None, false),
                        }
                    } else {
                        match argv.get(i + 1) {
                            Some(next) => (Some(next.clone()), true),
                            None => (None, false),
                        }
                    };
                    if value.is_none() && c != 'o' {
                        let long = long_name_for(c);
                        return ParseOutcome::Exit(error(&format!(
                            "argument -{c}/--{long}: expected one argument"
                        )));
                    }
                    match c {
                        'x' => args.excluded_paths = value.unwrap_or_default(),
                        'a' => {
                            let v = value.unwrap_or_default();
                            if !["file", "vuln"].contains(&v.as_str()) {
                                return ParseOutcome::Exit(error(&format!(
                                    "argument -a/--aggregate: invalid choice: '{v}' (choose from 'file', 'vuln')"
                                )));
                            }
                            args.agg_type = v;
                        }
                        'n' => {
                            let v = value.unwrap_or_default();
                            match v.parse() {
                                Ok(n) => args.context_lines = n,
                                Err(_) => {
                                    return ParseOutcome::Exit(error(&format!(
                                        "argument -n/--number: invalid int value: '{v}'"
                                    )));
                                }
                            }
                        }
                        'c' => args.config_file = value,
                        'p' => args.profile = value,
                        't' => args.tests = value,
                        's' => args.skips = value,
                        'f' => {
                            let v = value.unwrap_or_default();
                            if !FORMAT_CHOICES.contains(&v.as_str()) {
                                return ParseOutcome::Exit(error(&format!(
                                    "argument -f/--format: invalid choice: '{v}' (choose from 'csv', 'custom', 'html', 'json', 'sarif', 'screen', 'txt', 'xml', 'yaml')"
                                )));
                            }
                            args.output_format = Some(v);
                        }
                        'o' => {
                            args.output_file =
                                value.map(OutputTarget::File).unwrap_or(OutputTarget::Bare)
                        }
                        'b' => args.baseline = value,
                        _ => unreachable!(),
                    }
                    if consumed_next {
                        i += 1;
                    }
                    ci = chars.len();
                    continue;
                }
                other => {
                    return ParseOutcome::Exit(error(&format!("unrecognized arguments: -{other}")));
                }
            }
            ci += 1;
        }
        i += 1;
    }

    if let Some(s) = severity_string {
        args.severity = match s.as_str() {
            "all" => 1,
            "low" => 2,
            "medium" => 3,
            "high" => 4,
            _ => unreachable!(),
        };
    }
    if let Some(s) = confidence_string {
        args.confidence = match s.as_str() {
            "all" => 1,
            "low" => 2,
            "medium" => 3,
            "high" => 4,
            _ => unreachable!(),
        };
    }

    ParseOutcome::Run(Box::new(args))
}

fn long_name_for(short: char) -> &'static str {
    match short {
        'x' => "exclude",
        'a' => "aggregate",
        'n' => "number",
        'c' => "configfile",
        'p' => "profile",
        't' => "tests",
        's' => "skip",
        'f' => "format",
        'o' => "output",
        'b' => "baseline",
        _ => "",
    }
}
