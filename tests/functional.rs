//! Port of `tests/functional/test_functional.py`: bandit is run against each
//! example file and the issue counts by severity / confidence are compared to
//! the known-good values (the acceptance specification of the rewrite).
//!
//! Counts are `[UNDEFINED, LOW, MEDIUM, HIGH]`.

mod common;

use common::{check_example, check_metrics};

macro_rules! example_test {
    ($name:ident, $file:literal, $sev:expr, $conf:expr) => {
        #[test]
        fn $name() {
            check_example($file, $sev, $conf, false);
        }
    };
}

example_test!(test_binding, "binding.py", [0, 0, 1, 0], [0, 0, 1, 0]);
example_test!(test_crypto_md5, "crypto-md5.py", [0, 0, 16, 9], [0, 0, 0, 25]);
example_test!(test_ciphers, "ciphers.py", [0, 0, 1, 24], [0, 0, 0, 25]);
example_test!(test_cipher_modes, "cipher-modes.py", [0, 0, 1, 0], [0, 0, 0, 1]);
example_test!(test_eval, "eval.py", [0, 0, 3, 0], [0, 0, 0, 3]);
example_test!(test_mark_safe, "mark_safe.py", [0, 0, 1, 0], [0, 0, 0, 1]);
example_test!(test_exec, "exec.py", [0, 0, 1, 0], [0, 0, 0, 1]);
example_test!(test_hardcoded_passwords, "hardcoded-passwords.py", [0, 16, 0, 0], [0, 0, 16, 0]);
example_test!(test_hardcoded_tmp, "hardcoded-tmp.py", [0, 0, 3, 0], [0, 0, 3, 0]);
example_test!(test_imports_aliases, "imports-aliases.py", [0, 4, 1, 4], [0, 0, 0, 9]);
example_test!(test_imports_from, "imports-from.py", [0, 3, 0, 0], [0, 0, 0, 3]);
example_test!(test_imports_function, "imports-function.py", [0, 2, 0, 0], [0, 0, 0, 2]);
example_test!(test_telnet_usage, "telnetlib.py", [0, 0, 0, 2], [0, 0, 0, 2]);
example_test!(test_ftp_usage, "ftplib.py", [0, 0, 0, 3], [0, 0, 0, 3]);
example_test!(test_imports, "imports.py", [0, 2, 0, 0], [0, 0, 0, 2]);
example_test!(test_imports_using_importlib, "imports-with-importlib.py", [0, 4, 0, 0], [0, 0, 0, 4]);
example_test!(test_mktemp, "mktemp.py", [0, 0, 4, 0], [0, 0, 0, 4]);
example_test!(test_okay, "okay.py", [0, 0, 0, 0], [0, 0, 0, 0]);
example_test!(test_subdirectory_okay, "init-py-test/subdirectory-okay.py", [0, 0, 0, 0], [0, 0, 0, 0]);
example_test!(test_os_chmod, "os-chmod.py", [0, 0, 4, 8], [0, 0, 1, 11]);
example_test!(test_os_exec, "os-exec.py", [0, 8, 0, 0], [0, 0, 8, 0]);
example_test!(test_os_popen, "os-popen.py", [0, 8, 0, 1], [0, 0, 0, 9]);
example_test!(test_os_spawn, "os-spawn.py", [0, 8, 0, 0], [0, 0, 8, 0]);
example_test!(test_os_startfile, "os-startfile.py", [0, 3, 0, 0], [0, 0, 3, 0]);
example_test!(test_os_system, "os_system.py", [0, 1, 0, 0], [0, 0, 0, 1]);
example_test!(test_pickle, "pickle_deserialize.py", [0, 1, 3, 0], [0, 0, 0, 4]);
example_test!(test_dill, "dill.py", [0, 1, 3, 0], [0, 0, 0, 4]);
example_test!(test_shelve, "shelve_open.py", [0, 1, 2, 0], [0, 0, 0, 3]);
example_test!(test_jsonpickle, "jsonpickle.py", [0, 0, 3, 0], [0, 0, 0, 3]);
example_test!(test_pandas_read_pickle, "pandas_read_pickle.py", [0, 1, 1, 0], [0, 0, 0, 2]);
example_test!(test_popen_wrappers, "popen_wrappers.py", [0, 7, 0, 0], [0, 0, 0, 7]);
example_test!(test_random_module, "random_module.py", [0, 12, 0, 0], [0, 0, 0, 12]);
example_test!(test_requests_ssl_verify_disabled, "requests-ssl-verify-disabled.py", [0, 0, 0, 18], [0, 0, 0, 18]);
example_test!(test_requests_without_timeout, "requests-missing-timeout.py", [0, 0, 25, 0], [0, 25, 0, 0]);
example_test!(test_skip, "skip.py", [0, 5, 0, 0], [0, 0, 0, 5]);
example_test!(test_sql_statements, "sql_statements.py", [0, 0, 23, 0], [0, 11, 12, 0]);
example_test!(test_multiline_sql_statements, "sql_multiline_statements.py", [0, 0, 26, 0], [0, 13, 13, 0]);
example_test!(test_ssl_insecure_version, "ssl-insecure-version.py", [0, 1, 13, 9], [0, 0, 14, 9]);
example_test!(test_subprocess_shell, "subprocess_shell.py", [0, 24, 1, 11], [0, 1, 0, 35]);
example_test!(test_urlopen, "urlopen.py", [0, 0, 8, 0], [0, 0, 0, 8]);
example_test!(test_wildcard_injection, "wildcard-injection.py", [0, 10, 0, 4], [0, 0, 5, 9]);
example_test!(test_django_sql_injection, "django_sql_injection_extra.py", [0, 0, 11, 0], [0, 0, 11, 0]);
example_test!(test_django_sql_injection_raw, "django_sql_injection_raw.py", [0, 0, 6, 0], [0, 0, 6, 0]);
example_test!(test_yaml, "yaml_load.py", [0, 0, 2, 0], [0, 0, 0, 2]);
example_test!(test_host_key_verification, "no_host_key_verification.py", [0, 0, 0, 8], [0, 0, 8, 0]);
example_test!(test_jinja2_templating, "jinja2_templating.py", [0, 0, 0, 5], [0, 0, 2, 3]);
example_test!(test_mako_templating, "mako_templating.py", [0, 0, 3, 0], [0, 0, 0, 3]);
example_test!(test_xml_etree_celementtree, "xml_etree_celementtree.py", [0, 1, 4, 0], [0, 0, 0, 5]);
example_test!(test_xml_expatbuilder, "xml_expatbuilder.py", [0, 1, 2, 0], [0, 0, 0, 3]);
example_test!(test_xml_pulldom, "xml_pulldom.py", [0, 2, 2, 0], [0, 0, 0, 4]);
example_test!(test_xml_xmlrpc, "xml_xmlrpc.py", [0, 0, 0, 1], [0, 0, 0, 1]);
example_test!(test_xml_etree_elementtree, "xml_etree_elementtree.py", [0, 1, 4, 0], [0, 0, 0, 5]);
example_test!(test_xml_expatreader, "xml_expatreader.py", [0, 1, 1, 0], [0, 0, 0, 2]);
example_test!(test_xml_minidom, "xml_minidom.py", [0, 2, 2, 0], [0, 0, 0, 4]);
example_test!(test_xml_sax, "xml_sax.py", [0, 2, 6, 0], [0, 0, 0, 8]);
example_test!(test_httpoxy_cgihandler, "httpoxy_cgihandler.py", [0, 0, 0, 1], [0, 0, 0, 1]);
example_test!(test_httpoxy_twisted_script, "httpoxy_twisted_script.py", [0, 0, 0, 1], [0, 0, 0, 1]);
example_test!(test_httpoxy_twisted_directory, "httpoxy_twisted_directory.py", [0, 0, 0, 1], [0, 0, 0, 1]);
example_test!(test_paramiko_injection, "paramiko_injection.py", [0, 0, 1, 0], [0, 0, 1, 0]);
example_test!(test_partial_path, "partial_path_process.py", [0, 11, 0, 0], [0, 0, 0, 11]);
example_test!(test_weak_cryptographic_key, "weak_cryptographic_key_sizes.py", [0, 0, 8, 8], [0, 0, 0, 16]);
example_test!(test_flask_debug_true, "flask_debug.py", [0, 0, 0, 1], [0, 0, 1, 0]);
example_test!(test_nosec, "nosec.py", [0, 5, 0, 0], [0, 0, 0, 5]);
example_test!(test_unverified_context, "unverified_context.py", [0, 0, 1, 0], [0, 0, 0, 1]);
example_test!(test_hashlib_new_insecure_functions, "hashlib_new_insecure_functions.py", [0, 0, 0, 9], [0, 0, 0, 9]);
example_test!(test_blacklist_pycrypto, "pycrypto.py", [0, 0, 0, 2], [0, 0, 0, 2]);
example_test!(test_no_blacklist_pycryptodome, "pycryptodome.py", [0, 0, 0, 0], [0, 0, 0, 0]);
example_test!(test_blacklist_pyghmi, "pyghmi.py", [0, 1, 0, 1], [0, 0, 1, 1]);
example_test!(test_snmp_security_check, "snmp.py", [0, 0, 3, 0], [0, 0, 0, 3]);
example_test!(test_tarfile_unsafe_members, "tarfile_extractall.py", [0, 1, 2, 2], [0, 1, 2, 2]);
example_test!(test_pytorch_load, "pytorch_load.py", [0, 0, 3, 0], [0, 0, 0, 3]);
example_test!(test_trojansource, "trojansource.py", [0, 0, 0, 1], [0, 0, 1, 0]);
example_test!(test_trojansource_latin1, "trojansource_latin1.py", [0, 0, 0, 0], [0, 0, 0, 0]);
example_test!(test_markupsafe_markup_xss, "markupsafe_markup_xss.py", [0, 0, 4, 0], [0, 0, 0, 4]);
example_test!(test_huggingface_unsafe_download, "huggingface_unsafe_download.py", [0, 0, 15, 0], [0, 0, 0, 15]);

// --- tests needing a specific configuration / profile (see PLAN.md M4) ---
// test_ignore_skip: "skip.py" with ignore_nosec=true -> [0,7,0,0] / [0,0,0,7]
// test_django_xss_secure: "mark_safe_secure.py", profile exclude ["B308"] -> all zeros
// test_django_xss_insecure: "mark_safe_insecure.py", profile exclude ["B308"] -> [0,0,29,0] / [0,0,0,29]
// test_asserts: "assert.py" with assert_used config {skips: []} -> [0,1,0,0]/[0,0,0,1];
//               {skips: ["*assert.py"]} -> zeros; {} (missing key -> defaults to []) -> [0,1,0,0]/[0,0,0,1]
// test_try_except_continue: "try_except_continue.py" check_typed_exception True -> [0,3,0,0]/[0,0,0,3]; False -> [0,2,0,0]/[0,0,0,2]
// test_try_except_pass: "try_except_pass.py" True -> [0,3,0,0]/[0,0,0,3]; False -> [0,2,0,0]/[0,0,0,2]
// test_markupsafe_markup_xss_extend_markup_names: config markupsafe_xss {extend_markup_names: ["webhelpers.html.literal"]} -> [0,0,2,0]/[0,0,0,2]
// test_markupsafe_markup_xss_allowed_calls: config markupsafe_xss {allowed_calls: ["bleach.clean"]} -> [0,0,1,0]/[0,0,0,1]

#[test]
fn test_ignore_skip() {
    check_example("skip.py", [0, 7, 0, 0], [0, 0, 0, 7], true);
}

#[test]
fn test_metric_gathering() {
    check_metrics("skip.py", &[("nosec", 2), ("loc", 7), ("CONFIDENCE.HIGH", 5), ("SEVERITY.LOW", 5)]);
    check_metrics("imports.py", &[("nosec", 0), ("loc", 4), ("CONFIDENCE.HIGH", 2), ("SEVERITY.LOW", 2)]);
}

#[test]
fn test_multiline_sql_statements_metrics() {
    check_metrics("sql_multiline_statements.py", &[("nosec", 7), ("skipped_tests", 8)]);
}

// Also to port (PLAN.md M5): test_nonsense (1 skipped file), test_multiline_code
// (issue 0: lineno 1 / linerange [1] / code contains "subprocess"; issue 1: lineno 5 /
// linerange [3,4,5,6] / "shell=True"; issue 2: lineno 11 / linerange [8..=13] / "shell=True"),
// test_code_line_numbers (binding.py: get_code lines start with "lineno-1 ", "lineno ", "lineno+1 "),
// test_baseline_filter (flask_debug.py with a baseline containing the B201 issue -> empty issue list).
