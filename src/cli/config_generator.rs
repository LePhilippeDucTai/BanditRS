//! `bandit-config-generator` (port of `bandit/cli/config_generator.py`) —
//! see docs/spec/cli_formatters_tests.md §A.14.

use crate::core::plugin_config::PluginConfigs;
use crate::core::registry;

const TEMPLATE: &str = "\n### Bandit config file generated from:\n# '{cli}'\n\n### This config may optionally select a subset of tests to run or skip by\n### filling out the 'tests' and 'skips' lists given below. If no tests are\n### specified for inclusion then it is assumed all tests are desired. The skips\n### set will remove specific tests from the include set. This can be controlled\n### using the -t/-s CLI options. Note that the same test ID should not appear\n### in both 'tests' and 'skips', this would be nonsensical and is detected by\n### Bandit at runtime.\n\n# Available tests:\n{test_list}\n\n# (optional) list included test IDs here, eg '[B101, B406]':\n{test}\n\n# (optional) list skipped test IDs here, eg '[B101, B406]':\n{skip}\n\n### (optional) plugin settings - some test plugins require configuration data\n### that may be given here, per-plugin. All bandit test plugins have a built in\n### set of sensible defaults and these will be used if no configuration is\n### provided. It is not necessary to provide settings for every (or any) plugin\n### if the defaults are acceptable.\n\n{settings}\n";

fn print_help() {
    println!(
        "usage: bandit-config-generator [-h] [--show-defaults] [-o OUTPUT_FILE] [-t TESTS] [-s SKIPS]\n"
    );
    println!("Bandit Config Generator\n");
    println!("    This tool is used to generate an optional profile.  The profile may be used");
    println!("    to include or skip tests and override values for plugins.\n");
    println!("    When used to store an output profile, this tool will output a template that");
    println!("    includes all plugins and their default settings.  Any settings which aren't");
    println!("    being overridden can be safely removed from the profile and default values");
    println!("    will be used.  Bandit will prefer settings from the profile over the built");
    println!("    in values.\n");
    println!("options:");
    println!("  -h, --help            show this help message and exit");
    println!(
        "  --show-defaults       show the default settings values for each plugin but do not output a profile"
    );
    println!("  -o OUTPUT_FILE, --out OUTPUT_FILE");
    println!("                        output file to save profile");
    println!("  -t TESTS, --tests TESTS");
    println!("                        list of test names to run");
    println!("  -s SKIPS, --skip SKIPS");
    println!("                        list of test names to skip");
}

/// `str(list_of_strings)`: Python list repr.
fn py_list_repr(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|s| format!("'{s}'")).collect();
    format!("[{}]", parts.join(", "))
}

/// Entry point; returns the exit code (always 0 unless argument errors).
pub fn main(args: Vec<String>) -> i32 {
    crate::log::set_level(crate::log::Level::Info);
    crate::log::set_format("[%(levelname)5s]: %(message)s");
    crate::log::set_stdout(true);

    let mut show_defaults = false;
    let mut output_file: Option<String> = None;
    let mut tests: Option<String> = None;
    let mut skips: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        let mut next_value = || -> Option<String> {
            i += 1;
            args.get(i).cloned()
        };
        match arg {
            "-h" | "--help" => {
                print_help();
                return 0;
            }
            "--show-defaults" => show_defaults = true,
            "-o" | "--out" => output_file = next_value(),
            "-t" | "--tests" => tests = next_value(),
            "-s" | "--skip" => skips = next_value(),
            _ => {
                eprintln!("bandit-config-generator: error: unrecognized arguments: {arg}");
                return 2;
            }
        }
        i += 1;
    }

    if output_file.is_none() && !show_defaults {
        print_help();
        return 1;
    }

    let yaml_settings = PluginConfigs::defaults_yaml();

    if show_defaults {
        println!("{yaml_settings}");
    }

    if let Some(path) = output_file {
        if std::path::Path::new(&path).exists() {
            crate::log_error!("config_generator", "File {} already exists, exiting", path);
            return 2;
        }

        // Python opens (creates/truncates) the file before validating
        // skips/tests, so a validation error still leaves an empty file.
        if std::fs::File::create(&path).is_err() {
            crate::log_error!("config_generator", "Unable to open {} for writing", path);
            return 0;
        }

        let skips_list: Vec<String> = skips
            .as_deref()
            .map(|s| s.split(',').map(str::to_string).collect())
            .unwrap_or_default();
        let tests_list: Vec<String> = tests
            .as_deref()
            .map(|s| s.split(',').map(str::to_string).collect())
            .unwrap_or_default();

        let mut invalid = None;
        for skip in &skips_list {
            if !registry::check_id(skip) {
                invalid = Some(format!("unknown ID in skips: {skip}"));
                break;
            }
        }
        if invalid.is_none() {
            for test in &tests_list {
                if !registry::check_id(test) {
                    invalid = Some(format!("unknown ID in tests: {test}"));
                    break;
                }
            }
        }
        if let Some(e) = invalid {
            crate::log_error!("config_generator", "Error: {}", e);
            return 0;
        }

        let mut test_list: Vec<String> = registry::all_ids_and_names()
            .iter()
            .map(|(id, name)| format!("# {id} : {name}"))
            .collect();
        test_list.sort();

        let cli = std::iter::once("bandit-config-generator".to_string())
            .chain(args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");
        let contents = TEMPLATE
            .replace("{cli}", &cli)
            .replace("{settings}", &yaml_settings)
            .replace("{test_list}", &test_list.join("\n"))
            .replace(
                "{skip}",
                &if skips_list.is_empty() {
                    "skips:".to_string()
                } else {
                    format!("skips: {}", py_list_repr(&skips_list))
                },
            )
            .replace(
                "{test}",
                &if tests_list.is_empty() {
                    "tests:".to_string()
                } else {
                    format!("tests: {}", py_list_repr(&tests_list))
                },
            );

        match std::fs::write(&path, contents) {
            Ok(()) => crate::log_info!("config_generator", "Successfully wrote profile: {}", path),
            Err(_) => crate::log_error!("config_generator", "Unable to open {} for writing", path),
        }
    }

    0
}
