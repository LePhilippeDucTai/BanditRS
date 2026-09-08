# Inventaire des tests : bandit (Python) → BanditRS (Rust)

> Généré depuis la collecte `pytest --collect-only` du dépôt de référence `/home/user/bandit` @ `1d3053d`
> (**273 tests** réels ; pytest en collecte 274 car `test_test_set.py::test_plugin` est un faux positif — une
> fonction-plugin décorée, pas un test ; suite au vert avec Python 3.11 le 2026-09-08 : `274 passed in 7.60s`).
> Règle : **chaque test Python a un test Rust homonyme** dans le fichier miroir indiqué, sauf mention
> « non portable » (justifiée ici et dans `DEVIATIONS.md` #8). Les stubs Rust `#[ignore]` portent le
> nom exact ; un jalon (WP) est terminé quand plus aucun stub de ses fichiers n'est ignoré
> (`scripts/wp_status.sh`).

Statuts : **porté** = équivalent Rust au vert ; **partiel** = test Rust existant à renforcer ;
**à porter** = stub ignoré ; **adapté** = à porter avec une sémantique Rust équivalente (mocks Python
remplacés par des fixtures réelles) ; **non portable** = introspection Python sans équivalent.

## Synthèse

| Fichier Python | Fichier Rust miroir | WP | Tests | porté | partiel | à porter | adapté | non portable |
|---|---|---|---:|---:|---:|---:|---:|---:|
| `tests/functional/test_baseline.py` | `tests/functional_baseline.rs` | [WP-02](wp/WP-02-functional-baseline.md) | 7 | 0 | 0 | 7 | 0 | 0 |
| `tests/functional/test_functional.py` | `tests/functional.rs` | [WP-01](wp/WP-01-functional-config-profiles.md) | 79 | 68 | 0 | 11 | 0 | 0 |
| `tests/functional/test_runtime.py` | `tests/runtime.rs` | — | 9 | 9 | 0 | 0 | 0 | 0 |
| `tests/unit/cli/test_baseline.py` | `tests/unit_cli_baseline.rs` | [WP-04](wp/WP-04-unit-cli-baseline.md) | 12 | 0 | 0 | 8 | 4 | 0 |
| `tests/unit/cli/test_config_generator.py` | `tests/unit_cli_config_generator.rs` | [WP-05](wp/WP-05-unit-cli-config-generator.md) | 6 | 0 | 0 | 6 | 0 | 0 |
| `tests/unit/cli/test_main.py` | `tests/unit_cli_main.rs` | [WP-03](wp/WP-03-unit-cli-main.md) | 20 | 0 | 0 | 14 | 5 | 1 |
| `tests/unit/core/test_blacklisting.py` | `tests/unit_core_blacklisting.rs` | [WP-11](wp/WP-11-unit-core-issue-blacklisting-docs.md) | 2 | 2 | 0 | 0 | 0 | 0 |
| `tests/unit/core/test_config.py` | `tests/unit_core_config.rs` | [WP-08](wp/WP-08-unit-core-config.md) | 26 | 0 | 0 | 26 | 0 | 0 |
| `tests/unit/core/test_context.py` | `tests/unit_core_context.rs` | [WP-09](wp/WP-09-unit-core-context.md) | 19 | 0 | 0 | 0 | 17 | 2 |
| `tests/unit/core/test_docs_util.py` | `tests/unit_core_docs_util.rs` | [WP-11](wp/WP-11-unit-core-issue-blacklisting-docs.md) | 3 | 3 | 0 | 0 | 0 | 0 |
| `tests/unit/core/test_issue.py` | `tests/unit_core_issue.rs` | [WP-11](wp/WP-11-unit-core-issue-blacklisting-docs.md) | 7 | 6 | 0 | 0 | 1 | 0 |
| `tests/unit/core/test_manager.py` | `tests/unit_core_manager.rs` | [WP-06](wp/WP-06-unit-core-manager.md) | 21 | 0 | 0 | 13 | 7 | 1 |
| `tests/unit/core/test_meta_ast.py` | `—` | [WP-11](wp/WP-11-unit-core-issue-blacklisting-docs.md) | 2 | 0 | 0 | 0 | 0 | 2 |
| `tests/unit/core/test_test_set.py` | `tests/unit_core_test_set.rs` | [WP-10](wp/WP-10-unit-core-test-set.md) | 13 | 0 | 0 | 0 | 13 | 0 |
| `tests/unit/core/test_util.py` | `tests/unit_core_util.rs` | [WP-07](wp/WP-07-unit-core-util.md) | 30 | 26 | 0 | 0 | 0 | 4 |
| `tests/unit/formatters/test_csv.py` | `tests/unit_formatters_csv.rs` | [WP-13](wp/WP-13-unit-formatters-structured.md) | 1 | 0 | 1 | 0 | 0 | 0 |
| `tests/unit/formatters/test_custom.py` | `tests/unit_formatters_custom.rs` | [WP-13](wp/WP-13-unit-formatters-structured.md) | 1 | 1 | 0 | 0 | 0 | 0 |
| `tests/unit/formatters/test_html.py` | `tests/unit_formatters_html.rs` | [WP-13](wp/WP-13-unit-formatters-structured.md) | 3 | 1 | 1 | 1 | 0 | 0 |
| `tests/unit/formatters/test_json.py` | `tests/unit_formatters_json.rs` | [WP-13](wp/WP-13-unit-formatters-structured.md) | 1 | 1 | 0 | 0 | 0 | 0 |
| `tests/unit/formatters/test_sarif.py` | `tests/unit_formatters_sarif.rs` | [WP-13](wp/WP-13-unit-formatters-structured.md) | 1 | 1 | 0 | 0 | 0 | 0 |
| `tests/unit/formatters/test_screen.py` | `tests/unit_formatters_screen.rs` | [WP-12](wp/WP-12-unit-formatters-text-screen.md) | 4 | 0 | 0 | 4 | 0 | 0 |
| `tests/unit/formatters/test_text.py` | `tests/unit_formatters_text.rs` | [WP-12](wp/WP-12-unit-formatters-text-screen.md) | 4 | 1 | 1 | 2 | 0 | 0 |
| `tests/unit/formatters/test_xml.py` | `tests/unit_formatters_xml.rs` | [WP-13](wp/WP-13-unit-formatters-structured.md) | 1 | 0 | 1 | 0 | 0 | 0 |
| `tests/unit/formatters/test_yaml.py` | `tests/unit_formatters_yaml.rs` | [WP-13](wp/WP-13-unit-formatters-structured.md) | 1 | 0 | 1 | 0 | 0 | 0 |
| **Total** | | | **273** | **82** | **5** | **125** | **51** | **10** |

Reste à porter/renforcer : **181** tests (176 stubs `#[ignore]` + 5 tests partiels) ; 10 tests non portables ; 82 déjà au vert.

## `tests/functional/test_baseline.py` → `tests/functional_baseline.rs` ([WP-02](wp/WP-02-functional-baseline.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `BaselineFunctionalTests::test_existing_and_new_candidates` | `test_existing_and_new_candidates` | à porter |  |
| `BaselineFunctionalTests::test_new_candidates_include_nosec_new_nosecs` | `test_new_candidates_include_nosec_new_nosecs` | à porter |  |
| `BaselineFunctionalTests::test_new_candidates_include_nosec_only_nosecs` | `test_new_candidates_include_nosec_only_nosecs` | à porter |  |
| `BaselineFunctionalTests::test_no_existing_no_new_candidates` | `test_no_existing_no_new_candidates` | à porter |  |
| `BaselineFunctionalTests::test_no_existing_with_new_candidates` | `test_no_existing_with_new_candidates` | à porter |  |
| `BaselineFunctionalTests::test_no_new_candidates` | `test_no_new_candidates` | à porter |  |
| `BaselineFunctionalTests::test_no_new_candidates_include_nosec` | `test_no_new_candidates_include_nosec` | à porter |  |

## `tests/functional/test_functional.py` → `tests/functional.rs` ([WP-01](wp/WP-01-functional-config-profiles.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `FunctionalTests::test_asserts` | `test_asserts` | à porter | 3 configs `assert_used` (`skips: []`, `skips: ['*assert.py']`, `{}`) via `BanditConfig.raw` |
| `FunctionalTests::test_baseline_filter` | `test_baseline_filter` | à porter | `populate_baseline` JSON B201 → `get_issue_list()` vide |
| `FunctionalTests::test_binding` | `test_binding` | porté |  |
| `FunctionalTests::test_blacklist_pycrypto` | `test_blacklist_pycrypto` | porté |  |
| `FunctionalTests::test_blacklist_pyghmi` | `test_blacklist_pyghmi` | porté |  |
| `FunctionalTests::test_cipher_modes` | `test_cipher_modes` | porté |  |
| `FunctionalTests::test_ciphers` | `test_ciphers` | porté |  |
| `FunctionalTests::test_code_line_numbers` | `test_code_line_numbers` | à porter | préfixes de lignes de `get_code()` sur `binding.py` |
| `FunctionalTests::test_crypto_md5` | `test_crypto_md5` | porté |  |
| `FunctionalTests::test_dill` | `test_dill` | porté |  |
| `FunctionalTests::test_django_sql_injection` | `test_django_sql_injection` | porté |  |
| `FunctionalTests::test_django_sql_injection_raw` | `test_django_sql_injection_raw` | porté |  |
| `FunctionalTests::test_django_xss_insecure` | `test_django_xss_insecure` | à porter | profil `exclude: [B308]` ; dépend de DEVIATIONS.md #9 (DeepAssignation) — à implémenter complètement |
| `FunctionalTests::test_django_xss_secure` | `test_django_xss_secure` | à porter | profil `exclude: [B308]` → `TestSet` construit avec ce profil |
| `FunctionalTests::test_eval` | `test_eval` | porté |  |
| `FunctionalTests::test_exec` | `test_exec` | porté |  |
| `FunctionalTests::test_flask_debug_true` | `test_flask_debug_true` | porté |  |
| `FunctionalTests::test_ftp_usage` | `test_ftp_usage` | porté |  |
| `FunctionalTests::test_hardcoded_passwords` | `test_hardcoded_passwords` | porté |  |
| `FunctionalTests::test_hardcoded_tmp` | `test_hardcoded_tmp` | porté |  |
| `FunctionalTests::test_hashlib_new_insecure_functions` | `test_hashlib_new_insecure_functions` | porté |  |
| `FunctionalTests::test_host_key_verification` | `test_host_key_verification` | porté |  |
| `FunctionalTests::test_httpoxy` | `test_httpoxy_cgihandler, test_httpoxy_twisted_script, test_httpoxy_twisted_directory` | porté |  |
| `FunctionalTests::test_huggingface_unsafe_download` | `test_huggingface_unsafe_download` | porté |  |
| `FunctionalTests::test_ignore_skip` | `test_ignore_skip` | porté |  |
| `FunctionalTests::test_imports` | `test_imports` | porté |  |
| `FunctionalTests::test_imports_aliases` | `test_imports_aliases` | porté |  |
| `FunctionalTests::test_imports_from` | `test_imports_from` | porté |  |
| `FunctionalTests::test_imports_function` | `test_imports_function` | porté |  |
| `FunctionalTests::test_imports_using_importlib` | `test_imports_using_importlib` | porté |  |
| `FunctionalTests::test_jinja2_templating` | `test_jinja2_templating` | porté |  |
| `FunctionalTests::test_jsonpickle` | `test_jsonpickle` | porté |  |
| `FunctionalTests::test_mako_templating` | `test_mako_templating` | porté |  |
| `FunctionalTests::test_mark_safe` | `test_mark_safe` | porté |  |
| `FunctionalTests::test_markupsafe_markup_xss` | `test_markupsafe_markup_xss` | porté |  |
| `FunctionalTests::test_markupsafe_markup_xss_allowed_calls` | `test_markupsafe_markup_xss_allowed_calls` | à porter | config `markupsafe_xss.allowed_calls` |
| `FunctionalTests::test_markupsafe_markup_xss_extend_markup_names` | `test_markupsafe_markup_xss_extend_markup_names` | à porter | config `markupsafe_xss.extend_markup_names` |
| `FunctionalTests::test_metric_gathering` | `test_metric_gathering` | porté |  |
| `FunctionalTests::test_mktemp` | `test_mktemp` | porté |  |
| `FunctionalTests::test_multiline_code` | `test_multiline_code` | à porter | lineno/linerange/get_code des 3 issues de `multiline_statement.py` |
| `FunctionalTests::test_multiline_sql_statements` | `test_multiline_sql_statements + test_multiline_sql_statements_metrics` | porté |  |
| `FunctionalTests::test_no_blacklist_pycryptodome` | `test_no_blacklist_pycryptodome` | porté |  |
| `FunctionalTests::test_nonsense` | `test_nonsense` | à porter | 1 fichier dans `skipped` (syntax error) |
| `FunctionalTests::test_nosec` | `test_nosec` | porté |  |
| `FunctionalTests::test_okay` | `test_okay` | porté |  |
| `FunctionalTests::test_os_chmod` | `test_os_chmod` | porté |  |
| `FunctionalTests::test_os_exec` | `test_os_exec` | porté |  |
| `FunctionalTests::test_os_popen` | `test_os_popen` | porté |  |
| `FunctionalTests::test_os_spawn` | `test_os_spawn` | porté |  |
| `FunctionalTests::test_os_startfile` | `test_os_startfile` | porté |  |
| `FunctionalTests::test_os_system` | `test_os_system` | porté |  |
| `FunctionalTests::test_pandas_read_pickle` | `test_pandas_read_pickle` | porté |  |
| `FunctionalTests::test_paramiko_injection` | `test_paramiko_injection` | porté |  |
| `FunctionalTests::test_partial_path` | `test_partial_path` | porté |  |
| `FunctionalTests::test_pickle` | `test_pickle` | porté |  |
| `FunctionalTests::test_popen_wrappers` | `test_popen_wrappers` | porté |  |
| `FunctionalTests::test_pytorch_load` | `test_pytorch_load` | porté |  |
| `FunctionalTests::test_random_module` | `test_random_module` | porté |  |
| `FunctionalTests::test_requests_ssl_verify_disabled` | `test_requests_ssl_verify_disabled` | porté |  |
| `FunctionalTests::test_requests_without_timeout` | `test_requests_without_timeout` | porté |  |
| `FunctionalTests::test_shelve` | `test_shelve` | porté |  |
| `FunctionalTests::test_skip` | `test_skip` | porté |  |
| `FunctionalTests::test_snmp_security_check` | `test_snmp_security_check` | porté |  |
| `FunctionalTests::test_sql_statements` | `test_sql_statements` | porté |  |
| `FunctionalTests::test_ssl_insecure_version` | `test_ssl_insecure_version` | porté |  |
| `FunctionalTests::test_subdirectory_okay` | `test_subdirectory_okay` | porté |  |
| `FunctionalTests::test_subprocess_shell` | `test_subprocess_shell` | porté |  |
| `FunctionalTests::test_tarfile_unsafe_members` | `test_tarfile_unsafe_members` | porté |  |
| `FunctionalTests::test_telnet_usage` | `test_telnet_usage` | porté |  |
| `FunctionalTests::test_trojansource` | `test_trojansource` | porté |  |
| `FunctionalTests::test_trojansource_latin1` | `test_trojansource_latin1` | porté |  |
| `FunctionalTests::test_try_except_continue` | `test_try_except_continue` | à porter | `check_typed_exception` True/False via config |
| `FunctionalTests::test_try_except_pass` | `test_try_except_pass` | à porter | `check_typed_exception` True/False via config |
| `FunctionalTests::test_unverified_context` | `test_unverified_context` | porté |  |
| `FunctionalTests::test_urlopen` | `test_urlopen` | porté |  |
| `FunctionalTests::test_weak_cryptographic_key` | `test_weak_cryptographic_key` | porté |  |
| `FunctionalTests::test_wildcard_injection` | `test_wildcard_injection` | porté |  |
| `FunctionalTests::test_xml` | `test_xml_etree_celementtree, test_xml_expatbuilder, test_xml_pulldom, test_xml_xmlrpc, test_xml_etree_elementtree, test_xml_expatreader, test_xml_minidom, test_xml_sax` | porté |  |
| `FunctionalTests::test_yaml` | `test_yaml` | porté |  |

## `tests/functional/test_runtime.py` → `tests/runtime.rs` (—)

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `RuntimeTests::test_example_imports` | `test_example_imports` | porté |  |
| `RuntimeTests::test_example_nonexistent` | `test_example_nonexistent` | porté |  |
| `RuntimeTests::test_example_nonsense` | `test_example_nonsense` | porté |  |
| `RuntimeTests::test_example_nonsense2` | `test_example_nonsense2` | porté |  |
| `RuntimeTests::test_example_okay` | `test_example_okay` | porté |  |
| `RuntimeTests::test_help_arg` | `test_help_arg` | porté |  |
| `RuntimeTests::test_no_arguments` | `test_no_arguments` | porté |  |
| `RuntimeTests::test_nonexistent_config` | `test_nonexistent_config` | porté |  |
| `RuntimeTests::test_piped_input` | `test_piped_input` | porté |  |

## `tests/unit/cli/test_baseline.py` → `tests/unit_cli_baseline.rs` ([WP-04](wp/WP-04-unit-cli-baseline.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `BanditBaselineToolTests::test_bandit_baseline` | `test_bandit_baseline` | à porter |  |
| `BanditBaselineToolTests::test_init_logger` | `test_init_logger` | à porter |  |
| `BanditBaselineToolTests::test_initialize_dirty_repo` | `test_initialize_dirty_repo` | à porter |  |
| `BanditBaselineToolTests::test_initialize_existing_report_file` | `test_initialize_existing_report_file` | à porter |  |
| `BanditBaselineToolTests::test_initialize_existing_temp_file` | `test_initialize_existing_temp_file` | à porter |  |
| `BanditBaselineToolTests::test_initialize_git_command_failure` | `test_initialize_git_command_failure` | à porter (adapté) | `git` absent du PATH → `initialize()` renvoie `None` (`Git command not found`/`Git not available`) |
| `BanditBaselineToolTests::test_initialize_no_repo` | `test_initialize_no_repo` | à porter |  |
| `BanditBaselineToolTests::test_initialize_with_output_argument` | `test_initialize_with_output_argument` | à porter (adapté) | `bandit_args = ["-o", "bandit_baseline_result"]` passé explicitement à `initialize(args)` |
| `BanditBaselineToolTests::test_main_git_command_failure` | `test_main_git_command_failure` | à porter (adapté) | mock `git.Repo.commit` → faux `git` en tête de PATH qui échoue sur `rev-parse`/`log` → rc 2 (`Unable to get current or parent commit`) |
| `BanditBaselineToolTests::test_main_no_parent_commit` | `test_main_no_parent_commit` | à porter |  |
| `BanditBaselineToolTests::test_main_non_repo` | `test_main_non_repo` | à porter |  |
| `BanditBaselineToolTests::test_main_subprocess_error` | `test_main_subprocess_error` | à porter (adapté) | mock `subprocess.check_output` → faux `bandit` en tête de PATH retournant 3 → rc 3 |

## `tests/unit/cli/test_config_generator.py` → `tests/unit_cli_config_generator.rs` ([WP-05](wp/WP-05-unit-cli-config-generator.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `BanditConfigGeneratorLoggerTests::test_init_logger` | `test_init_logger` | à porter |  |
| `BanditConfigGeneratorTests::test_get_config_settings` | `test_get_config_settings` | à porter |  |
| `BanditConfigGeneratorTests::test_main_show_defaults` | `test_main_show_defaults` | à porter |  |
| `BanditConfigGeneratorTests::test_parse_args_no_defaults` | `test_parse_args_no_defaults` | à porter |  |
| `BanditConfigGeneratorTests::test_parse_args_out_file` | `test_parse_args_out_file` | à porter |  |
| `BanditConfigGeneratorTests::test_parse_args_show_defaults` | `test_parse_args_show_defaults` | à porter |  |

## `tests/unit/cli/test_main.py` → `tests/unit_cli_main.rs` ([WP-03](wp/WP-03-unit-cli-main.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `BanditCLIMainLoggerTests::test_init_logger` | `test_init_logger` | à porter |  |
| `BanditCLIMainLoggerTests::test_init_logger_debug_mode` | `test_init_logger_debug_mode` | à porter |  |
| `BanditCLIMainTests::test_get_options_from_ini_empty_directory_no_target` | `test_get_options_from_ini_empty_directory_no_target` | à porter |  |
| `BanditCLIMainTests::test_get_options_from_ini_no_ini_path_multi_bandit_files` | `test_get_options_from_ini_no_ini_path_multi_bandit_files` | à porter |  |
| `BanditCLIMainTests::test_get_options_from_ini_no_ini_path_no_bandit_files` | `test_get_options_from_ini_no_ini_path_no_bandit_files` | à porter |  |
| `BanditCLIMainTests::test_get_options_from_ini_no_ini_path_no_target` | `test_get_options_from_ini_no_ini_path_no_target` | à porter |  |
| `BanditCLIMainTests::test_init_extensions` | — | non portable | `extension_loader.MANAGER` n'existe pas (registre statique `registry::PLUGINS`) ; couvert par `registry_is_consistent` |
| `BanditCLIMainTests::test_log_option_source_arg_val` | `test_log_option_source_arg_val` | à porter |  |
| `BanditCLIMainTests::test_log_option_source_ini_val_with_str_default_and_no_arg_val` | `test_log_option_source_ini_val_with_str_default_and_no_arg_val` | à porter |  |
| `BanditCLIMainTests::test_log_option_source_ini_value` | `test_log_option_source_ini_value` | à porter |  |
| `BanditCLIMainTests::test_log_option_source_no_values` | `test_log_option_source_no_values` | à porter |  |
| `BanditCLIMainTests::test_main_baseline_ioerror` | `test_main_baseline_ioerror` | à porter (adapté) | mock IOError → `-b` sur un chemin inexistant/illisible → rc 2 + `Could not open baseline report` |
| `BanditCLIMainTests::test_main_config_unopenable` | `test_main_config_unopenable` | à porter | binaire `bandit -c bandit.yaml test` sans fichier → rc 2 (`Could not read config file.`) |
| `BanditCLIMainTests::test_main_exit_with_no_results` | `test_main_exit_with_no_results` | à porter (adapté) | cible `examples/okay.py` → rc 0 |
| `BanditCLIMainTests::test_main_exit_with_results` | `test_main_exit_with_results` | à porter (adapté) | mock `results_count` → cible réelle avec issue (ex. `examples/os_system.py`) → rc 1 |
| `BanditCLIMainTests::test_main_exit_with_results_and_with_exit_zero_flag` | `test_main_exit_with_results_and_with_exit_zero_flag` | à porter (adapté) | `--exit-zero` sur cible avec issue → rc 0 |
| `BanditCLIMainTests::test_main_handle_ini_options` | `test_main_handle_ini_options` | à porter (adapté) | mock `_get_options_from_ini` → vrai `.bandit` (`--ini`) avec `tests = some_test`/`skips = skip_test` → rc 2 + `No tests would be run, please check the profile.` sur stderr |
| `BanditCLIMainTests::test_main_invalid_config` | `test_main_invalid_config` | à porter | YAML invalide dans `bandit.yaml` → rc 2 (`Error parsing file.`) |
| `BanditCLIMainTests::test_main_invalid_output_format` | `test_main_invalid_output_format` | à porter |  |
| `BanditCLIMainTests::test_main_profile_not_found` | `test_main_profile_not_found` | à porter |  |

## `tests/unit/core/test_blacklisting.py` → `tests/unit_core_blacklisting.rs` ([WP-11](wp/WP-11-unit-core-issue-blacklisting-docs.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `BlacklistingTests::test_report_issue` | `test_report_issue` | porté |  |
| `BlacklistingTests::test_report_issue_defaults` | `test_report_issue_defaults` | porté |  |

## `tests/unit/core/test_config.py` → `tests/unit_core_config.rs` ([WP-08](wp/WP-08-unit-core-config.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `TestConfigCompat::test_bad_yaml` | `test_bad_yaml` | à porter |  |
| `TestConfigCompat::test_blacklist_error` | `test_blacklist_error` | à porter |  |
| `TestConfigCompat::test_converted_blacklist_call_data` | `test_converted_blacklist_call_data` | à porter |  |
| `TestConfigCompat::test_converted_blacklist_call_test` | `test_converted_blacklist_call_test` | à porter |  |
| `TestConfigCompat::test_converted_blacklist_import_data` | `test_converted_blacklist_import_data` | à porter |  |
| `TestConfigCompat::test_converted_blacklist_import_test` | `test_converted_blacklist_import_test` | à porter |  |
| `TestConfigCompat::test_converted_exclude` | `test_converted_exclude` | à porter |  |
| `TestConfigCompat::test_converted_exclude_blacklist` | `test_converted_exclude_blacklist` | à porter |  |
| `TestConfigCompat::test_converted_include` | `test_converted_include` | à porter |  |
| `TestConfigCompat::test_deprecation_message` | `test_deprecation_message` | à porter |  |
| `TestGetOption::test_levels` | `test_levels` | à porter |  |
| `TestGetOption::test_levels_not_exist` | `test_levels_not_exist` | à porter |  |
| `TestGetSetting::test_not_exist` | `test_not_exist` | à porter |  |
| `TestInit::test_file_does_not_exist` | `test_file_does_not_exist` | à porter |  |
| `TestInit::test_settings` | `test_settings` | à porter |  |
| `TestInit::test_yaml_invalid` | `test_yaml_invalid` | à porter |  |
| `TestTomlConfig::test_bad_yaml` | `test_bad_yaml_toml` | à porter | variante TOML (`TestTomlConfig`) |
| `TestTomlConfig::test_blacklist_error` | `test_blacklist_error_toml` | à porter | variante TOML (`TestTomlConfig`) |
| `TestTomlConfig::test_converted_blacklist_call_data` | `test_converted_blacklist_call_data_toml` | à porter | variante TOML (`TestTomlConfig`) |
| `TestTomlConfig::test_converted_blacklist_call_test` | `test_converted_blacklist_call_test_toml` | à porter | variante TOML (`TestTomlConfig`) |
| `TestTomlConfig::test_converted_blacklist_import_data` | `test_converted_blacklist_import_data_toml` | à porter | variante TOML (`TestTomlConfig`) |
| `TestTomlConfig::test_converted_blacklist_import_test` | `test_converted_blacklist_import_test_toml` | à porter | variante TOML (`TestTomlConfig`) |
| `TestTomlConfig::test_converted_exclude` | `test_converted_exclude_toml` | à porter | variante TOML (`TestTomlConfig`) |
| `TestTomlConfig::test_converted_exclude_blacklist` | `test_converted_exclude_blacklist_toml` | à porter | variante TOML (`TestTomlConfig`) |
| `TestTomlConfig::test_converted_include` | `test_converted_include_toml` | à porter | variante TOML (`TestTomlConfig`) |
| `TestTomlConfig::test_deprecation_message` | `test_deprecation_message_toml` | à porter | variante TOML (`TestTomlConfig`) |

## `tests/unit/core/test_context.py` → `tests/unit_core_context.rs` ([WP-09](wp/WP-09-unit-core-context.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `ContextTests::test__get_literal_value` | `test_get_literal_value` | à porter (adapté) | renommé (double underscore refusé par `non_snake_case`) ; `ast::literal::get_literal_value` sur constantes/list/tuple/set/dict/name/bytes (couvert par `literal.rs::literal_values`) |
| `ContextTests::test_call_args` | `test_call_args` | à porter (adapté) | mocks → contexte réel obtenu en parcourant `f(x.spam, 'eggs')` (helper `context_for_call`) |
| `ContextTests::test_call_args_count` | `test_call_args_count` | à porter (adapté) | `None` hors `Call` |
| `ContextTests::test_call_function_name` | `test_call_function_name` | à porter (adapté) |  |
| `ContextTests::test_call_function_name_qual` | `test_call_function_name_qual` | à porter (adapté) |  |
| `ContextTests::test_call_keywords` | `test_call_keywords` | à porter (adapté) | `f(arg1=x.spam, arg2='eggs')` |
| `ContextTests::test_check_call_arg_value` | `test_check_call_arg_value` | à porter (adapté) | `f(spam='eggs')` ; hors `Call` → `None` |
| `ContextTests::test_context_create` | — | non portable | `Context(context_object=Mock)` — le contexte Rust est typé, pas de dict |
| `ContextTests::test_filename` | `test_filename` | à porter (adapté) |  |
| `ContextTests::test_function_def_defaults_qual` | `test_function_def_defaults_qual` | à porter (adapté) | `def f(a=spam.x): ...` |
| `ContextTests::test_get_call_arg_at_position` | `test_get_call_arg_at_position` | à porter (adapté) |  |
| `ContextTests::test_get_lineno_for_call_arg` | `test_get_lineno_for_call_arg` | à porter (adapté) |  |
| `ContextTests::test_is_module_being_imported` | `test_is_module_being_imported` | à porter (adapté) | `import spam` |
| `ContextTests::test_is_module_imported_exact` | `test_is_module_imported_exact` | à porter (adapté) |  |
| `ContextTests::test_is_module_imported_like` | `test_is_module_imported_like` | à porter (adapté) |  |
| `ContextTests::test_node` | `test_node` | à porter (adapté) | `VNode` du contexte |
| `ContextTests::test_repr` | — | non portable | `repr(Context)` non exposé |
| `ContextTests::test_statement` | `test_statement` | à porter (adapté) |  |
| `ContextTests::test_string_val` | `test_string_val` | à porter (adapté) |  |

## `tests/unit/core/test_docs_util.py` → `tests/unit_core_docs_util.rs` ([WP-11](wp/WP-11-unit-core-issue-blacklisting-docs.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `DocsUtilTests::test_import_call_bib` | `test_import_call_bib` | porté |  |
| `DocsUtilTests::test_overwrite_bib_info` | `test_overwrite_bib_info` | porté |  |
| `DocsUtilTests::test_plugin_call_bib` | `test_plugin_call_bib` | porté |  |

## `tests/unit/core/test_issue.py` → `tests/unit_core_issue.rs` ([WP-11](wp/WP-11-unit-core-issue-blacklisting-docs.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `IssueTests::test_get_code` | `test_get_code` | adapté | mock `linecache` → `SourceFile` avec octets de contrôle (couvert par `issue.rs::get_code_with_control_chars`) |
| `IssueTests::test_issue_as_dict` | `test_issue_as_dict` | porté |  |
| `IssueTests::test_issue_create` | `test_issue_create` | porté |  |
| `IssueTests::test_issue_filter_confidence` | `test_issue_filter_confidence` | porté |  |
| `IssueTests::test_issue_filter_severity` | `test_issue_filter_severity` | porté |  |
| `IssueTests::test_issue_str` | `test_issue_str` | porté |  |
| `IssueTests::test_matches_issue` | `test_matches_issue` | porté |  |

## `tests/unit/core/test_manager.py` → `tests/unit_core_manager.rs` ([WP-06](wp/WP-06-unit-core-manager.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `ManagerTests::test_compare_baseline` | `test_compare_baseline` | à porter | déjà couvert par `manager.rs::baseline_helpers` ; porter le miroir |
| `ManagerTests::test_create_manager` | `test_create_manager` | à porter |  |
| `ManagerTests::test_create_manager_with_profile` | `test_create_manager_with_profile` | à porter |  |
| `ManagerTests::test_discover_files_exclude` | `test_discover_files_exclude` | à porter (adapté) | mock `_is_file_included=False` → fichier réel exclu par `-x` |
| `ManagerTests::test_discover_files_exclude_cmdline` | `test_discover_files_exclude_cmdline` | à porter (adapté) | `assert_called_with` → vérifier `excluded_files == [a, b]` et `c` traité |
| `ManagerTests::test_discover_files_exclude_dir` | `test_discover_files_exclude_dir` | à porter (adapté) | 4 formes d'exclusion `./x/*`, `./x/`, `./x`, `y` sur une arborescence réelle |
| `ManagerTests::test_discover_files_exclude_glob` | `test_discover_files_exclude_glob` | à porter |  |
| `ManagerTests::test_discover_files_include` | `test_discover_files_include` | à porter (adapté) | fichier réel `thing` (sans extension) → `enforce_glob=False` → `./thing` inclus |
| `ManagerTests::test_discover_files_recurse_files` | `test_discover_files_recurse_files` | à porter (adapté) | mock `_get_files_from_dir` → répertoire réel contenant `files.py` |
| `ManagerTests::test_discover_files_recurse_skip` | `test_discover_files_recurse_skip` | à porter (adapté) | mock `isdir` → vrai répertoire temporaire |
| `ManagerTests::test_find_candidate_matches` | `test_find_candidate_matches` | à porter | idem |
| `ManagerTests::test_get_files_from_dir` | `test_get_files_from_dir` | à porter (adapté) | mock `os.walk` → vrai tmpdir `/a/{a.py,b.py,c.ww}` |
| `ManagerTests::test_is_file_included` | `test_is_file_included` | à porter | idem (6 cas a–f) |
| `ManagerTests::test_matches_globlist` | `test_matches_globlist` | à porter | déjà couvert par `src/core/discover.rs::glob_list_and_inclusion` ; porter quand même le test miroir |
| `ManagerTests::test_output_results_invalid_format` | `test_output_results_invalid_format` | à porter |  |
| `ManagerTests::test_output_results_valid_format` | `test_output_results_valid_format` | à porter |  |
| `ManagerTests::test_populate_baseline_invalid_json` | `test_populate_baseline_invalid_json` | à porter | `log::with_buffer` pour capturer le warning |
| `ManagerTests::test_populate_baseline_success` | `test_populate_baseline_success` | à porter |  |
| `ManagerTests::test_results_count` | `test_results_count` | à porter |  |
| `ManagerTests::test_run_tests_ioerror` | `test_run_tests_ioerror` | à porter |  |
| `ManagerTests::test_run_tests_keyboardinterrupt` | — | non portable | `KeyboardInterrupt` (SIGINT) → non portable ; comportement Rust = code de sortie 130 par défaut |

## `tests/unit/core/test_meta_ast.py` → `—` ([WP-11](wp/WP-11-unit-core-issue-blacklisting-docs.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `BanditMetaAstTests::test_add_node` | — | non portable | `BanditMetaAst` (debug uniquement) non porté — DEVIATIONS.md #8 |
| `BanditMetaAstTests::test_str` | — | non portable | idem |

## `tests/unit/core/test_test_set.py` → `tests/unit_core_test_set.rs` ([WP-10](wp/WP-10-unit-core-test-set.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `BanditTestSetTests::test_has_defaults` | `test_has_defaults` | à porter (adapté) | registre factice (`B000` sur `Str`, blacklists `B401`/`B302`) → registre réel, mêmes assertions structurelles (cf. WP-10) |
| `BanditTestSetTests::test_profile_blacklist_compat` | `test_profile_blacklist_compat` | à porter (adapté) | registre factice (`B000` sur `Str`, blacklists `B401`/`B302`) → registre réel, mêmes assertions structurelles (cf. WP-10) |
| `BanditTestSetTests::test_profile_exclude_builtin_blacklist` | `test_profile_exclude_builtin_blacklist` | à porter (adapté) | registre factice (`B000` sur `Str`, blacklists `B401`/`B302`) → registre réel, mêmes assertions structurelles (cf. WP-10) |
| `BanditTestSetTests::test_profile_exclude_builtin_blacklist_specific` | `test_profile_exclude_builtin_blacklist_specific` | à porter (adapté) | registre factice (`B000` sur `Str`, blacklists `B401`/`B302`) → registre réel, mêmes assertions structurelles (cf. WP-10) |
| `BanditTestSetTests::test_profile_exclude_id` | `test_profile_exclude_id` | à porter (adapté) | registre factice (`B000` sur `Str`, blacklists `B401`/`B302`) → registre réel, mêmes assertions structurelles (cf. WP-10) |
| `BanditTestSetTests::test_profile_exclude_none` | `test_profile_exclude_none` | à porter (adapté) | registre factice (`B000` sur `Str`, blacklists `B401`/`B302`) → registre réel, mêmes assertions structurelles (cf. WP-10) |
| `BanditTestSetTests::test_profile_filter_blacklist_all` | `test_profile_filter_blacklist_all` | à porter (adapté) | registre factice (`B000` sur `Str`, blacklists `B401`/`B302`) → registre réel, mêmes assertions structurelles (cf. WP-10) |
| `BanditTestSetTests::test_profile_filter_blacklist_include` | `test_profile_filter_blacklist_include` | à porter (adapté) | registre factice (`B000` sur `Str`, blacklists `B401`/`B302`) → registre réel, mêmes assertions structurelles (cf. WP-10) |
| `BanditTestSetTests::test_profile_filter_blacklist_none` | `test_profile_filter_blacklist_none` | à porter (adapté) | registre factice (`B000` sur `Str`, blacklists `B401`/`B302`) → registre réel, mêmes assertions structurelles (cf. WP-10) |
| `BanditTestSetTests::test_profile_filter_blacklist_one` | `test_profile_filter_blacklist_one` | à porter (adapté) | registre factice (`B000` sur `Str`, blacklists `B401`/`B302`) → registre réel, mêmes assertions structurelles (cf. WP-10) |
| `BanditTestSetTests::test_profile_has_builtin_blacklist` | `test_profile_has_builtin_blacklist` | à porter (adapté) | registre factice (`B000` sur `Str`, blacklists `B401`/`B302`) → registre réel, mêmes assertions structurelles (cf. WP-10) |
| `BanditTestSetTests::test_profile_include_id` | `test_profile_include_id` | à porter (adapté) | registre factice (`B000` sur `Str`, blacklists `B401`/`B302`) → registre réel, mêmes assertions structurelles (cf. WP-10) |
| `BanditTestSetTests::test_profile_include_none` | `test_profile_include_none` | à porter (adapté) | registre factice (`B000` sur `Str`, blacklists `B401`/`B302`) → registre réel, mêmes assertions structurelles (cf. WP-10) |

## `tests/unit/core/test_util.py` → `tests/unit_core_util.rs` ([WP-07](wp/WP-07-unit-core-util.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `UtilTests::test_check_ast_node_bad_node` | `test_check_ast_node_bad_node` | porté (adapté) | `NodeKind::parse("Derp")` → `None` |
| `UtilTests::test_check_ast_node_bad_type` | `test_check_ast_node_bad_type` | porté (adapté) | `NodeKind::parse("walk")` → `None` |
| `UtilTests::test_check_ast_node_good` | `test_check_ast_node_good` | porté (adapté) | `NodeKind::parse("Call")` → `Some` |
| `UtilTests::test_deepgetattr` | — | non portable | introspection Python — DEVIATIONS.md #8 |
| `UtilTests::test_escaped_representation_invalid` | `test_escaped_representation_invalid` | porté |  |
| `UtilTests::test_escaped_representation_mixed` | `test_escaped_representation_mixed` | porté |  |
| `UtilTests::test_escaped_representation_simple` | `test_escaped_representation_simple` | porté |  |
| `UtilTests::test_escaped_representation_valid_not_printable` | `test_escaped_representation_valid_not_printable` | porté |  |
| `UtilTests::test_get_call_name1` | `test_get_call_name1` | porté | `ast::qualname::call_name` sur `a.b.c.d(x,y)` (couvert par `qualname.rs::get_call_name`) |
| `UtilTests::test_get_call_name2` | `test_get_call_name2` | porté | alias `a`, `a.b`, `a.b.c.d` |
| `UtilTests::test_get_call_name3` | `test_get_call_name3` | porté | `a.list[0](x,y)` → `attr_qual_name` = `""` |
| `UtilTests::test_get_module_qualname_from_path_abs_missingend` | `test_get_module_qualname_from_path_abs_missingend` | porté |  |
| `UtilTests::test_get_module_qualname_from_path_abs_missingmid` | `test_get_module_qualname_from_path_abs_missingmid` | porté |  |
| `UtilTests::test_get_module_qualname_from_path_abs_syms` | `test_get_module_qualname_from_path_abs_syms` | porté |  |
| `UtilTests::test_get_module_qualname_from_path_abs_typical` | `test_get_module_qualname_from_path_abs_typical` | porté |  |
| `UtilTests::test_get_module_qualname_from_path_dir` | `test_get_module_qualname_from_path_dir` | porté |  |
| `UtilTests::test_get_module_qualname_from_path_invalid_path` | `test_get_module_qualname_from_path_invalid_path` | porté |  |
| `UtilTests::test_get_module_qualname_from_path_rel_missingend` | `test_get_module_qualname_from_path_rel_missingend` | porté |  |
| `UtilTests::test_get_module_qualname_from_path_rel_missingmid` | `test_get_module_qualname_from_path_rel_missingmid` | porté |  |
| `UtilTests::test_get_module_qualname_from_path_rel_syms` | `test_get_module_qualname_from_path_rel_syms` | porté |  |
| `UtilTests::test_get_module_qualname_from_path_rel_typical` | `test_get_module_qualname_from_path_rel_typical` | porté |  |
| `UtilTests::test_get_module_qualname_from_path_sys` | `test_get_module_qualname_from_path_sys` | porté (adapté) | `os.__file__` résolu via `python3 -c "import os; print(os.__file__)"` → `os` |
| `UtilTests::test_get_module_qualname_from_path_with_dot` | `test_get_module_qualname_from_path_with_dot` | porté |  |
| `UtilTests::test_linerange` | `test_linerange` | porté | `jinja2_templating.py` `body[8]` → `[11, 12, 13]` |
| `UtilTests::test_namespace_path_join` | `test_namespace_path_join` | porté |  |
| `UtilTests::test_namespace_path_split` | `test_namespace_path_split` | porté |  |
| `UtilTests::test_parse_ini_file` | `test_parse_ini_file` | porté | `pycompat::configparser` (couvert partiellement par ses tests unitaires) |
| `UtilTests::test_path_for_function` | — | non portable | introspection Python (`get_path_for_function`) — DEVIATIONS.md #8 |
| `UtilTests::test_path_for_function_no_file` | — | non portable | idem |
| `UtilTests::test_path_for_function_no_module` | — | non portable | idem |

## `tests/unit/formatters/test_csv.py` → `tests/unit_formatters_csv.rs` ([WP-13](wp/WP-13-unit-formatters-structured.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `CsvFormatterTests::test_report` | `test_report` | partiel | ex-`csv_report_has_expected_columns` : lire via un parseur CSV (DictReader) et vérifier chaque champ |

## `tests/unit/formatters/test_custom.py` → `tests/unit_formatters_custom.rs` ([WP-13](wp/WP-13-unit-formatters-structured.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `CustomFormatterTests::test_report` | `test_report` | porté | ex-`custom_report_renders_bare_tags` |

## `tests/unit/formatters/test_html.py` → `tests/unit_formatters_html.rs` ([WP-13](wp/WP-13-unit-formatters-structured.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `HtmlFormatterTests::test_escaping` | `test_escaping` | porté | ex-`html_report_escapes_code_only` |
| `HtmlFormatterTests::test_report_contents` | `test_report_contents` | à porter | `span#loc/#nosec`, `div#issue-0/1/2`, classes `issue-sev-*`, `.candidates`/`.candidate`/`.code` (parser HTML `scraper`) |
| `HtmlFormatterTests::test_report_with_skipped` | `test_report_with_skipped` | partiel | ex-`html_report_contains_issue_block` : un seul `div#skipped`, contenu |

## `tests/unit/formatters/test_json.py` → `tests/unit_formatters_json.rs` ([WP-13](wp/WP-13-unit-formatters-structured.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `JsonFormatterTests::test_report` | `test_report` | porté | ex-`json_report_with_baseline_candidates` |

## `tests/unit/formatters/test_sarif.py` → `tests/unit_formatters_sarif.rs` ([WP-13](wp/WP-13-unit-formatters-structured.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `SarifFormatterTests::test_report` | `test_report` | porté | ex-`sarif_report_has_expected_fields` (+ `semanticVersion == VERSION`) |

## `tests/unit/formatters/test_screen.py` → `tests/unit_formatters_screen.rs` ([WP-12](wp/WP-12-unit-formatters-text-screen.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `ScreenFormatterTests::test_no_issues` | `test_no_issues` | à porter | rendre `screen::report` testable (sortie vers `impl Write` au lieu de stdout) |
| `ScreenFormatterTests::test_output_issue` | `test_output_issue` | à porter | idem text + couleurs `COLOR[severity]`/`COLOR["DEFAULT"]` |
| `ScreenFormatterTests::test_report_baseline` | `test_report_baseline` | à porter |  |
| `ScreenFormatterTests::test_report_nobaseline` | `test_report_nobaseline` | à porter | en-têtes colorés `header(...)`, blocs exacts |

## `tests/unit/formatters/test_text.py` → `tests/unit_formatters_text.rs` ([WP-12](wp/WP-12-unit-formatters-text-screen.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `TextFormatterTests::test_no_issues` | `test_no_issues` | porté | ex-`text_report_no_issues` |
| `TextFormatterTests::test_output_issue` | `test_output_issue` | à porter | exposer `text::output_issue_str(issue, indent, show_lineno, show_code, lines)` ; 3 variantes exactes |
| `TextFormatterTests::test_report_baseline` | `test_report_baseline` | à porter | candidats indentés de 10 espaces ; issue à >1 candidat sans code ni lineno |
| `TextFormatterTests::test_report_nobaseline` | `test_report_nobaseline` | partiel | ex-`text_report_with_issue` : ajouter `_totals` forcés (loc 1000, nosec 50, compteurs à 1) et les 20 sous-chaînes exactes |

## `tests/unit/formatters/test_xml.py` → `tests/unit_formatters_xml.rs` ([WP-13](wp/WP-13-unit-formatters-structured.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `XmlFormatterTests::test_report` | `test_report` | partiel | ex-`xml_report_has_testcase_and_error` : parser le XML (`roxmltree`) au lieu de sous-chaînes |

## `tests/unit/formatters/test_yaml.py` → `tests/unit_formatters_yaml.rs` ([WP-13](wp/WP-13-unit-formatters-structured.md))

| Test Python | Test Rust | Statut | Note |
|---|---|---|---|
| `YamlFormatterTests::test_report` | `test_report` | partiel | ex-`yaml_report_roundtrips_same_fields` : relire le YAML (`pycompat::yaml_load`) et vérifier `candidates`/`more_info` comme en JSON |

