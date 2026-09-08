# CLI, formatters et suite de tests de bandit (référence pour le portage Rust)

Dépôt Python @ `1d3053d`. `bandit.__version__` vient des métadonnées du paquet (dans le venv de test :
`0.0.1.dev49`) ; BanditRS utilise `DOCS_VERSION = "latest"` pour les URLs de documentation (normalisé par le
harnais différentiel). `bandit.__author__ = "PyCQA"`.

# Partie A — CLI

## A.1 Entry points
`bandit = bandit.cli.main:main`, `bandit-config-generator = bandit.cli.config_generator:main`,
`bandit-baseline = bandit.cli.baseline:main`. Formatters (ordre) : `csv, json, txt, xml, html, sarif, screen, yaml, custom`.

## A.3 `_init_logger(log_level=INFO, log_format=None)`
Handlers du logger racine vidés ; format `log_format` ou `"[%(module)s]\t%(levelname)s\t%(message)s"` ;
`logging.captureWarnings(True)` ; handler sur **stderr** ; `LOG.debug("logging initialized")`.

## A.4 `_get_options_from_ini(ini_path, target)`
- `ini_path` fourni → ce fichier. Sinon `os.walk` de chaque cible, `fnmatch.filter(filenames, ".bandit")` ;
  plusieurs → `LOG.error("Multiple .bandit files found - scan separately or choose one with --ini\n\t%s", ", ".join(files))`
  puis `sys.exit(2)` ; un seul → `LOG.info("Found project level .bandit file: %s", f)`.
- `utils.parse_ini_file(ini_file)` → `dict(config.items("bandit"))` ou `None` (warning).

Clés lues dans `[bandit]` (via `_log_option_source`) : `configfile` (dest `config_file`), `exclude`
(`excluded_paths`), `skips`, `tests`, `targets` (split sur `,`), `recursive`, `aggregate` (`agg_type`),
`number` (`context_lines`, `int(x or 0) or None`), `profile`, `level` (`severity`), `confidence`, `format`
(`output_format`), `msg-template`, `output` (`output_file`), `verbose`, `debug`, `quiet`, `ignore-nosec`,
`baseline`. (Python : valeurs string ; BanditRS convertit `level`/`confidence`/`number` en entiers —
DEVIATIONS.md #3.)

## A.6 `_log_option_source(default_val, arg_val, ini_val, option_name)`
```python
if default_val is None:
    if arg_val: LOG.info("Using command line arg for %s", name); return arg_val
    elif ini_val: LOG.info("Using ini file for %s", name); return ini_val
    else: return None
elif default_val == arg_val: return ini_val if ini_val else arg_val
else: return arg_val
```
Priorité CLI > ini > défaut.

## A.7 `_get_profile(config, profile_name, config_path)`
`profile_name` → `config.get_option("profiles") or {}` puis `profiles.get(name)` ; `None` →
`ProfileNotFound(config_path, name)` ; sinon `profile["include"] = set(config.get_option("tests") or [])`,
`profile["exclude"] = set(config.get_option("skips") or [])`.

## A.8 `_log_info`
`LOG.info("profile include tests: %s", ",".join(include) or "None")`, idem exclude,
`"cli include tests: %s"` (`args.tests`), `"cli exclude tests: %s"` (`args.skips`).

## A.9 Arguments (`ArgumentParser(description="Bandit - a Python source code security analyzer", RawDescriptionHelpFormatter)`)

| Flags | dest | action | défaut | choices | aide |
|---|---|---|---|---|---|
| `targets` (positionnel, `nargs="*"`) | targets | store | — | — | `source file(s) or directory(s) to be tested` |
| `-r`, `--recursive` | recursive | store_true | False | — | `find and process files in subdirectories` |
| `-a`, `--aggregate` | agg_type | store | `"file"` | `file`, `vuln` | `aggregate output by vulnerability (default) or by filename` |
| `-n`, `--number` | context_lines | store int | 3 | — | `maximum number of code lines to output for each issue` |
| `-c`, `--configfile` | config_file | store | None | — | `optional config file to use for selecting plugins and overriding defaults` |
| `-p`, `--profile` | profile | store | None | — | `profile to use (defaults to executing all tests)` |
| `-t`, `--tests` | tests | store | None | — | `comma-separated list of test IDs to run` |
| `-s`, `--skip` | skips | store | None | — | `comma-separated list of test IDs to skip` |
| `-l`, `--level` ‡ | severity | count | 1 | — | `report only issues of a given severity level or higher (-l for LOW, -ll for MEDIUM, -lll for HIGH)` |
| `--severity-level` ‡ | severity_string | store | — | `all`, `low`, `medium`, `high` | `report only issues of a given severity level or higher. "all" and "low" are likely to produce the same results, but it is possible for rules to be undefined which will not be listed in "low".` |
| `-i`, `--confidence` † | confidence | count | 1 | — | `report only issues of a given confidence level or higher (-i for LOW, -ii for MEDIUM, -iii for HIGH)` |
| `--confidence-level` † | confidence_string | store | — | idem | (même texte avec « confidence ») |
| `-f`, `--format` | output_format | store | `screen`/`txt` (tty) | `csv, custom, html, json, sarif, screen, txt, xml, yaml` (triés) | `specify output format` |
| `--msg-template` | msg_template | store | None | — | `specify output message template (only usable with --format custom), see CUSTOM FORMAT section for list of available values` |
| `-o`, `--output` | output_file | store, `nargs="?"`, `FileType("w", encoding="utf-8")` | stdout | — | `write report to filename` |
| `-v`, `--verbose` ★ | verbose | store_true | False | — | `output extra information like excluded and included files` |
| `-d`, `--debug` | debug | store_true | False | — | `turn on debug mode` |
| `-q`, `--quiet`, `--silent` ★ | quiet | store_true | False | — | `only show output in the case of an error` |
| `--ignore-nosec` | ignore_nosec | store_true | False | — | `do not skip lines with # nosec comments` |
| `-x`, `--exclude` | excluded_paths | store | `",".join(EXCLUDE)` | — | `comma-separated list of paths (glob patterns supported) to exclude from scan (note that these are in addition to the excluded paths provided in the config file) (default: .svn,CVS,.bzr,.hg,.git,__pycache__,.tox,.eggs,*.egg)` |
| `-b`, `--baseline` | baseline | store | None | — | `path of a baseline report to compare against (only JSON-formatted files are accepted)` |
| `--ini` | ini_path | store | None | — | `path to a .bandit file that supplies command line arguments` |
| `--exit-zero` | exit_zero | store_true | False | — | `exit with 0, even with results found` |
| `--version` | — | version | — | — | `f"%(prog)s {version}\n  python version = {sys.version}"` |

‡ / † / ★ : trois groupes mutuellement exclusifs. `set_defaults(debug=False, verbose=False, quiet=False, ignore_nosec=False)`.
Format par défaut : `"screen" if sys.stdout.isatty() and NO_COLOR unset and TERM != "dumb" else "txt"`.

## A.10 Épilogue de l'aide
`plugin_list = "\n\t".join(sorted(set(f"{id}\t{name}" pour les plugins et les blacklists)))` ; `epilog = dedent(texte) + f"\t{plugin_list}"` avec le texte :
```

CUSTOM FORMATTING
-----------------

Available tags:

    {abspath}, {relpath}, {line}, {col}, {test_id},
    {severity}, {msg}, {confidence}, {range}

Example usage:

    Default template:
    bandit -r examples/ --format custom --msg-template \
    "{abspath}:{line}: {test_id}[bandit]: {severity}: {msg}"

    Provides same output as:
    bandit -r examples/ --format custom

    Tags can also be formatted in python string.format() style:
    bandit -r examples/ --format custom --msg-template \
    "{relpath:20.20s}: {line:03}: {test_id:^8}: DEFECT: {msg:>20}"

    See python documentation for more information about formatting style:
    https://docs.python.org/3/library/string.html

The following tests were discovered and loaded:
-----------------------------------------------
```
(newline initial et newline + indentation finaux inclus). La table id → nom est dans `src/core/registry.rs` et `src/core/blacklist.rs`.

## A.11 `main()` pas à pas
1. `debug = DEBUG if "-d" in sys.argv or "--debug" in sys.argv else INFO` ; `_init_logger(debug)`.
2. `extension_mgr = MANAGER` ; `baseline_formatters = [json, txt, html, screen, custom]`.
3. Parser (§A.9), épilogue (§A.10), `args = parser.parse_args()`.
4. `output_format != "custom" and msg_template is not None` → `parser.error("--msg-template can only be used with --format=custom")` (exit 2).
5. `--severity-level` : `all→1, low→2, medium→3, high→4` → `args.severity` ; idem confiance.
6. `ini_options = _get_options_from_ini(args.ini_path, args.targets)` ; si présent, résolution de chaque option (§A.4).
7. `b_conf = BanditConfig(config_file=args.config_file)` ; `ConfigError` → `LOG.error(e)`, exit 2.
8. `if not args.targets: parser.print_usage(); sys.exit(2)`.
9. `b_conf.get_option("log_format")` → `_init_logger(DEBUG, log_format=...)`. `args.quiet` → `_init_logger(WARN)`.
10. `profile = _get_profile(...)` ; `_log_info` ; `profile["include"].update(args.tests.split(","))` ; idem skips ; `extension_mgr.validate_profile(profile)` ; `ProfileNotFound`/`ValueError` → `LOG.error(e)`, exit 2.
11. `b_mgr = BanditManager(b_conf, args.agg_type, args.debug, profile=profile, verbose=args.verbose, quiet=args.quiet, ignore_nosec=args.ignore_nosec)`.
12. `args.baseline` : `open(args.baseline)` → `populate_baseline` ; `OSError` → `LOG.warning("Could not open baseline report: %s", path)`, exit 2 ; format hors `baseline_formatters` → `LOG.warning("Baseline must be used with one of the following formats: " + str(list))`, exit 2.
13. `output_format != "json"` : `LOG.info("using config: %s", config_file)` si défini ; `LOG.info("running on Python %d.%d.%d", ...)`.
14. `b_mgr.discover_files(args.targets, args.recursive, args.excluded_paths)`.
15. `if not b_mgr.b_ts.tests: LOG.error("No tests would be run, please check the profile."); sys.exit(2)`.
16. `b_mgr.run_tests()` ; `LOG.debug(b_ma)`, `LOG.debug(metrics)`.
17. `sev_level = RANKING[args.severity - 1]` ; `conf_level = RANKING[args.confidence - 1]`.
18. `b_mgr.output_results(args.context_lines, sev_level, conf_level, args.output_file, args.output_format, args.msg_template)`.
19. `sys.exit(1)` si `results_count(sev, conf) > 0 and not args.exit_zero` sinon `sys.exit(0)`.

Détails : `-o` ouvert/tronqué au parsing (`nargs="?"` sans `const` → `None` pour un `-o` nu) ; `xml` réouvre le fichier en binaire ; les formatters loggent « … written to file » si `fileobj.name != "<stdout>"`.

## A.13 `bandit-baseline` (`cli/baseline.py`)
Globales : `bandit_args = sys.argv[1:]` (capturés à l'import, donc `-f` est aussi transmis à `bandit`), `baseline_tmp_file = "_bandit_baseline_run.json_"`, `default_output_format = "terminal"`, `report_basename = "bandit_baseline_result"`, `valid_baseline_formats = ["txt", "html", "json"]`.
- `init_logger()` : niveau INFO, format `"[%(levelname)7s ] %(message)s"`, handler sur **stdout**.
- `initialize()` : parser `description="Bandit Baseline - Generates Bandit results compared to a baseline"`, epilog `"Additional Bandit arguments such as severity filtering (-ll) can be added and will be passed to Bandit."` ; `targets` (`nargs="+"`), `-f` (`output_format`, défaut `"terminal"`, choices txt/html/json) ; `parse_known_args()`. Étapes : format `terminal` → `LOG.info("No output format specified, using %s", "terminal")` ; `report_fname = f"{report_basename}.{fmt}"` ; git absent → `LOG.error("Git not available, reinstall with baseline extra")` et `(None, None, None)` ; `git.Repo(os.getcwd())` : `InvalidGitRepositoryError` → `"Bandit baseline must be called from a git project root"` ; `GitCommandNotFound` → `"Git command not found"` ; `repo.is_dirty()` → `"Current working directory is dirty and must be resolved"` ; `fmt != terminal and exists(report_fname)` → `"File %s already exists, aborting"` ; `exists(baseline_tmp_file)` → `"Temporary file %s needs to be removed prior to running"` ; `"-o" in bandit_args` → `"Bandit baseline must not be called with the -o option"` ; retourne `(fmt, repo, report_fname)` ou `(None, None, None)`.
- `main()` : `init_logger()` ; `initialize()` ; pas de repo → exit 2 ; `commit = repo.commit()` ; `LOG.info("Got current commit: [%s]", commit.name_rev)` ; `parent = commit.parents[0]` ; `LOG.info("Got parent commit: [%s]", ...)` ; `GitCommandError` → `"Unable to get current or parent commit"` exit 2 ; `IndexError` → `"Parent commit not available"` exit 2 ; `output_type = ["-f","txt"] if terminal else ["-o", report_fname]` ; dans un tmpdir : étape 1 `"Getting Bandit baseline results"` sur le parent : `bandit` + `bandit_args + ["-f","json","-o", tmpfile]` ; étape 2 `"Comparing Bandit results to baseline"` sur le commit courant : `bandit_args + ["-b", tmpfile] + output_type` ; chaque étape : `repo.head.reset(commit=..., working_tree=True)`, `subprocess.check_output(["bandit"] + args)` ; `CalledProcessError` → `output = e.output`, `rc = e.returncode` ; `rc not in [0, 1]` → `LOG.error("Error running command: %s\nOutput: %s\n", bandit_args, output)` ; fin : `repo.head.reset(commit=current_commit, working_tree=True)` ; `terminal` → `print(output)` sinon `LOG.info("Successfully wrote %s", report_fname)` ; `sys.exit(rc)` du dernier run.

## A.14 `bandit-config-generator` (`cli/config_generator.py`)
Template (newline initial inclus) :
```

### Bandit config file generated from:
# '{cli}'

### This config may optionally select a subset of tests to run or skip by
### filling out the 'tests' and 'skips' lists given below. If no tests are
### specified for inclusion then it is assumed all tests are desired. The skips
### set will remove specific tests from the include set. This can be controlled
### using the -t/-s CLI options. Note that the same test ID should not appear
### in both 'tests' and 'skips', this would be nonsensical and is detected by
### Bandit at runtime.

# Available tests:
{test_list}

# (optional) list included test IDs here, eg '[B101, B406]':
{test}

# (optional) list skipped test IDs here, eg '[B101, B406]':
{skip}

### (optional) plugin settings - some test plugins require configuration data
### that may be given here, per-plugin. All bandit test plugins have a built in
### set of sensible defaults and these will be used if no configuration is
### provided. It is not necessary to provide settings for every (or any) plugin
### if the defaults are acceptable.

{settings}
```
`init_logger()` : INFO, `"[%(levelname)5s]: %(message)s"`, stdout. Arguments : `--show-defaults` (store_true, `show the default settings values for each plugin but do not output a profile`), `-o/--out` (`output_file`, `output file to save profile`), `-t/--tests` (`list of test names to run`), `-s/--skip` (`list of test names to skip`) ; `RawTextHelpFormatter` ; description :
```
Bandit Config Generator

    This tool is used to generate an optional profile.  The profile may be used
    to include or skip tests and override values for plugins.

    When used to store an output profile, this tool will output a template that
    includes all plugins and their default settings.  Any settings which aren't
    being overridden can be safely removed from the profile and default values
    will be used.  Bandit will prefer settings from the profile over the built
    in values.
```
Ni `-o` ni `--show-defaults` → `print_help()` + exit 1. `get_config_settings()` : pour chaque plugin avec `_takes_config` et un `gen_config` → `config[plugin.name] = gen_config(name)` ; `yaml.safe_dump(config, default_flow_style=False)`. `main()` : `--show-defaults` → `print(yaml)` ; `-o` : fichier existant → `LOG.error("File %s already exists, exiting")` exit 2 ; ids inconnus → `RuntimeError("unknown ID in skips: X")`/`"unknown ID in tests: X"` (capturée → `LOG.error("Error: %s", e)`) ; `test_list = sorted(["# {id} : {name}" ...] plugins + blacklists)` ; `contents = template.format(cli=" ".join(sys.argv), settings=yaml, test_list="\n".join(test_list), skip="skips: " + str(skips) if skips else "skips:", test=...)` ; `OSError` → `"Unable to open %s for writing"` ; succès → `LOG.info("Successfully wrote profile: %s")` ; retourne 0.

# Partie B — Formatters
Signature `report(manager, fileobj, sev_level, conf_level, lines=-1)` (`custom` : `template=None`). `@accepts_baseline` : json, txt, html, screen, custom.

`Issue.as_dict` : `filename, test_name, test_id, issue_severity, issue_cwe ({id, link} ou {}), issue_confidence, issue_text, line_number, line_range, col_offset, end_col_offset` (+ `code`). `more_info` ajouté par les formatters via `docs_utils.get_url`.

## B.1 JSON
```python
machine_output = {"results": [], "errors": [{"filename": f, "reason": r} for f, r in manager.get_skipped()]}
results = manager.get_issue_list(sev_level, conf_level); baseline = not isinstance(results, list)
if baseline: pour chaque issue : d = as_dict(max_lines=lines) ; d["more_info"] = get_url(test_id) ; si len(candidats) > 1 : d["candidates"] = [c.as_dict(max_lines=lines) ...]
else: collector = [as_dict(max_lines=lines)] + more_info
machine_output["results"] = sorted(collector, key=itemgetter("test_name" if agg_type == "vuln" else "filename"))
machine_output["metrics"] = manager.metrics.data
machine_output["generated_at"] = now(utc).strftime("%Y-%m-%dT%H:%M:%SZ")
json.dumps(machine_output, sort_keys=True, indent=2, separators=(",", ": "))   # pas de newline final
```
Log `"JSON output written to file: %s"`.

## B.2 YAML
Même collecte sans branche baseline ; `result["code"] = code.replace("\n", "\\n")` ; `generated_at` ; `yaml.safe_dump(machine_output, fileobj, default_flow_style=False)` (clés triées, block style, largeur 80). Log `"YAML output written to file: %s"`.

## B.3 CSV
`fieldnames = ["filename","test_name","test_id","issue_severity","issue_confidence","issue_cwe","issue_text","line_number","col_offset","end_col_offset","line_range","more_info"]` ; `DictWriter(extrasaction="ignore")` ; `r["issue_cwe"] = r["issue_cwe"]["link"]` (KeyError si NOTSET → **corrigé** : vide) ; `line_range` = repr de liste (`[4]`) ; dialecte excel (CRLF). Log `"CSV output written to file: %s"`.

## B.4 XML
`<testsuite name="bandit" tests="N">` ; par issue `<testcase classname={fname} name={test}>` + `<error more_info={url} type={severity} message={text}>` avec texte `"Test ID: %s Severity: %s Confidence: %s\nCWE: %s\n%s\nLocation %s:%s" % (test_id, severity, confidence, cwe, text, fname, lineno)` ; `tree.write(fileobj, encoding="utf-8", xml_declaration=True)` → `<?xml version='1.0' encoding='utf-8'?>\n` ; stdout binaire / fichier réouvert en `wb`. Log `"XML output written to file: %s"`.

## B.5 Text
`get_verbose_details` : `"Files in scope (%i):"` + `"\t%s (score: {SEVERITY: %i, CONFIDENCE: %i})"` (sommes des vecteurs) + `"Files excluded (%i):"` + `"\t%s"`.
`get_metrics` : `"\nRun metrics:"`, `"\tTotal issues (by severity):"`, `"\t\tUndefined: N"`, `"\t\tLow: N"`, `"\t\tMedium: N"`, `"\t\tHigh: N"`, `"\tTotal issues (by confidence):"` + idem.
`_output_issue_str(issue, indent, show_lineno=True, show_code=True, lines=-1)` (joint par `\n`) :
```
{indent}>> Issue: [{test_id}:{test}] {text}
{indent}   Severity: {severity.capitalize()}   Confidence: {confidence.capitalize()}
{indent}   CWE: {str(cwe)}
{indent}   More Info: {url}
{indent}   Location: {fname}:{lineno if show_lineno else ""}:{col_offset if show_lineno else ""}
```
puis si `show_code` : `[indent + line for line in issue.get_code(lines, True).split("\n")]`.
`get_results` : `"\tNo issues identified."` si vide ; sinon par issue : bloc complet (ou, baseline avec > 1 candidat : issue sans lineno/code, `"\n-- Candidate Issues --"`, chaque candidat avec `indent = " " * 10` et `lines`, suivis de `"\n"`), puis `"-" * 50` ; joints par `\n`.
`report` (seulement si `not manager.quiet or manager.results_count(sev, conf)`), joints par `\n` + `"\n"` final :
```
Run started:{datetime.now(utc)}
[verbose details]
\nTest results:
{results}
\nCode scanned:
\tTotal lines of code: {loc}
\tTotal lines skipped (#nosec): {nosec}
\tTotal potential issues skipped due to specifically being disabled (e.g., #nosec BXXX): {skipped_tests}
{metrics}
Files skipped ({n}):
\t{fname} ({reason})
```
Log `"Text output written to file: %s"`.

## B.6 Screen
`COLOR = {"DEFAULT": "\033[0m", "HEADER": "\033[95m", "LOW": "\033[94m", "MEDIUM": "\033[93m", "HIGH": "\033[91m"}` ; `header(text, *args) = HEADER + text % args + DEFAULT` sur `Run started:%s`, `Files in scope (%i):`, `Files excluded (%i):`, `\nTest results:`, `\nCode scanned:`, `\nRun metrics:`, `Files skipped (%i):` ; première ligne d'issue `"%s%s>> Issue: [%s:%s] %s" % (indent, COLOR[severity], ...)` ; ligne Location suivie de `COLOR["DEFAULT"]` ; pas de ligne « Total potential issues skipped » ; `print("\n".join(bits))` sur stdout ; si `-o` : `LOG.info("Screen formatter output was not written to file: %s, consider '-f txt'", name)`.

## B.7 HTML
Templates verbatim de `bandit/formatters/html.py:171-323` (`header_block`, `report_block`, `issue_block`, `code_block`, `candidate_block`, `candidate_issue`, `skipped_block`, `metrics_block`) — à recopier depuis le fichier Python. Règles : `skipped_str = "".join(f"{fname} <b>reason:</b> {reason}<br>")` ; par issue : sans baseline ou 1 candidat → `code_block.format(code=html.escape(get_code(lines, True).strip("\n").lstrip(" ")))` sinon candidats ; `issue_class = f"issue-sev-{severity.lower()}"` ; seul le code est échappé. Log `"HTML output written to file: %s"`.

## B.8 Custom
Template par défaut `"{abspath}:{line}: {test_id}[bandit]: {severity}: {msg}"` ; tags : `abspath` (`os.path.abspath(fname)`), `relpath`, `line`, `col`, `end_col`, `test_id`, `severity`, `msg`, `confidence`, `range` (liste), `cwe` (`str(cwe)`). `string.Formatter().parse` ; validation `vformat(template, (), SafeMapper(line=0))` (`ValueError` → `LOG.error("Template is not in valid format: %s", e.args[0])`, exit 2) ; aucun tag → `LOG.error("No tags were found in the template. Are you missing '{}'?")`, exit 2 ; tags inconnus → `LOG.warning("Tag '%s' was not recognized and will be skipped, did you mean to use '%s'?", tag, similar)` (`similar` = max de `len(set(tag) & set(known))`, ordre trié) et émis comme texte littéral ; template reconstruit (`{`→`{{`, `}`→`}}` dans les littéraux ; champs `{name[:spec][!conv]}`) + `"\n"` ; par issue `format(**SafeMapper(...))`. Log `"Result written to file: %s"`.

## B.9 SARIF
`SCHEMA_URI = "https://json.schemastore.org/sarif-2.1.0.json"`, `SCHEMA_VER = "2.1.0"`, `TS_FORMAT = "%Y-%m-%dT%H:%M:%SZ"`. `SarifLog(schema_uri, version, runs=[Run(tool=Tool(driver=ToolComponent(name="Bandit", organization="PyCQA", semantic_version=version, version=version)), invocations=[Invocation(end_time_utc, execution_successful=True, tool_configuration_notifications=[Notification(level="error", message={text: reason}, locations=[physical artifactLocation uri=to_uri(fname)])])], properties={"metrics": metrics.data}, results=[Result(rule_id, rule_index, message={text}, level=HIGH→"error"/MEDIUM→"warning" (omis, défaut)/LOW→"note", locations=[physical{artifactLocation{uri}, region{startLine=line_range[0], endLine=line_range[1] si len>1 sinon [0], startColumn=col+1, endColumn=end_col+1, snippet{text=ligne}}, contextRegion{startLine, endLine, snippet}}], properties={issue_confidence, issue_severity})])])` ; règles `ReportingDescriptor(id, name=test_name, help_uri=url, properties={"tags": ["security", f"external/cwe/cwe-{id}"], "precision": confidence.lower()})` dédoublonnées ; `parse_code(code)` : split sur `\n`, dernier élément vide retiré, chaque ligne `"<n> <texte>"` → `texte + "\n"` (dernier sans `\n` si le code n'en avait pas) ; `to_uri` : absolu → `PurePath.as_uri()`, relatif → `urllib.parse.quote(as_posix())` ; sérialisation `jschema_to_python.to_json` (camelCase, `None` et défauts omis). Ordre de clés canonique : voir le docstring de `bandit/formatters/sarif.py:19-123`. Log `"SARIF output written to file: %s"`.

# Partie C — Tests

## C.1 `tests/functional/test_functional.py`
setUp : `BanditConfig()`, `BanditManager(b_conf, "file")`, `b_ts = BanditTestSet(config=b_conf)`. `run_example(script, ignore_nosec=False)` : `path = os.path.join(os.getcwd(), "examples", script)` ; `discover_files([path], True)` ; `run_tests()`. `check_example` : `scores = []` ; run ; `result[type][rank] = score[type][idx] // RANKING_VALUES[rank]` ; `assertDictEqual(expect, result)`. `check_metrics` : `Metrics()` neuf ; compare `_totals[k]` et `_totals[f"{criteria}.{rank}"]`.
Table complète des comptes attendus : `tests/functional.rs` (colonnes SEVERITY puis CONFIDENCE en ordre UNDEFINED/LOW/MEDIUM/HIGH). Cas particuliers :
- `test_nonsense` : `1 == len(b_mgr.skipped)`.
- `test_multiline_sql_statements` : aussi `check_metrics("sql_multiline_statements.py", {"nosec": 7, "skipped_tests": 8})`.
- `test_metric_gathering` : `skip.py` → `{"nosec": 2, "loc": 7, "issues": {"CONFIDENCE": {"HIGH": 5}, "SEVERITY": {"LOW": 5}}}` ; `imports.py` → `{"nosec": 0, "loc": 4, "issues": {"CONFIDENCE": {"HIGH": 2}, "SEVERITY": {"LOW": 2}}}`.
- `test_multiline_code` (`multiline_statement.py`) : 0 skipped, 1 fichier finissant par `multiline_statement.py` ; 3 issues : `lineno 1, linerange [1]`, code contient `subprocess` ; `lineno 5, linerange [3,4,5,6]`, `shell=True` ; `lineno 11, linerange [8..13]`, `shell=True`.
- `test_code_line_numbers` (`binding.py`) : `get_code().splitlines()` → lignes `"%i " % (lineno-1)`, `lineno`, `lineno+1`.
- `test_baseline_filter` : baseline JSON `{"results": [{"filename": f"{cwd}/examples/flask_debug.py", "issue_confidence": "MEDIUM", "issue_severity": "HIGH", "issue_cwe": {"id": 94, "link": ".../94.html"}, "issue_text": "A Flask app appears to be run with debug=True, which exposes the Werkzeug debugger and allows the execution of arbitrary code.", "line_number": 10, "col_offset": 0, "line_range": [10], "test_name": "flask_debug_true", "test_id": "B201", "code": "..."}]}` → `1 == len(baseline)` et `get_issue_list() == {}`.
- `test_asserts` / `test_try_except_*` mutent `plugin._config` ; `test_django_xss_*` et `test_markupsafe_*` utilisent un `BanditTestSet` avec profil/config.

## C.2 `test_runtime.py` : voir `tests/runtime.rs` (commandes, codes, sous-chaînes).

## C.3 `tests/functional/test_baseline.py`
Constantes : `"Total lines of code: 12"`, `"Total lines of code: 9"`, `"Total lines skipped (#nosec): 0"`, `"Total lines skipped (#nosec): 3"`, `"Files skipped (0):"`, `"No issues identified."`, `"Issue: [B317:blacklist]"`, `"Issue: [B506:yaml_load]"`, `"Issue: [B602:subprocess_popen_with_shell_equals_true]"`, candidats `"subprocess.Popen('/bin/ls *', shell=True)"`, `"... # nosec"`, `"y = yaml.load(temp_str)"`, `"... # nosec"`, `"xml.sax.make_parser()"`, `"... # nosec"`.
`_create_baseline(paired)` : tmpdir ; copie `examples/<value>` → `<tmp>/<key>` ; `bandit -r [--ignore-nosec] -f json -o <tmp>/baseline_report.json <tmp>` ; puis copie `examples/<key>` par-dessus ; `_run_bandit_baseline` : `bandit -r [--ignore-nosec] -b <baseline> <dir>`.

| Test | baseline ← contenu | flag | rc baseline | rc run | attendus |
|---|---|---|---|---|---|
| no_new_candidates | new_candidates-all.py ← new_candidates-all.py | — | 1 | 0 | code 12, nosec 3, skipped 0, no issues |
| no_existing_no_new_candidates | okay.py ← okay.py | — | 0 | 0 | code 1, nosec 0, skipped 0, no issues |
| no_existing_with_new_candidates | new_candidates-all.py ← new_candidates-none.py | — | 0 | 1 | code 12, nosec 3, B317/B506/B602, candidats 1/3/5 |
| existing_and_new_candidates | new_candidates-all.py ← new_candidates-some.py | — | 1 | 1 | code 12, nosec 3, B317/B506, candidats 3/5 |
| no_new_candidates_include_nosec | all ← all | `--ignore-nosec` | 1 | 0 | code 12, nosec 0, no issues |
| new_candidates_include_nosec_only_nosecs | new_candidates-nosec.py ← new_candidates-none.py | `--ignore-nosec` | 0 | 1 | code 9, nosec 0, B317/B506/B602, candidats 2/4/6 |
| new_candidates_include_nosec_new_nosecs | all ← none | `--ignore-nosec` | 0 | 1 | code 12, nosec 0, B317/B506/B602, candidats 1–6 |

## C.4 `tests/unit/cli/test_main.py`
`test_init_logger` (handlers non vides, niveau INFO/DEBUG) ; `_get_options_from_ini` : `(None, [])` → `None` ; `(tmpdir, [])` → `None` ; `(None, [tmpdir])` → `None` ; deux `.bandit` → exit 2 ; `_log_option_source` : `(None|"default", "file", "vuln")` → `"file"` ; `(None, None, "vuln")` → `"vuln"` ; `("file", "file", "vuln")` → `"vuln"` ; `(None, None, None)` → `None` ; config illisible/invalide → exit 2 ; ini `{"exclude": "/tmp", "skips": "skip_test", "tests": "some_test"}` → exit 2 et `LOG.error("No tests would be run, please check the profile.")` ; `-p bad` → exit 2 et `"Unable to find profile (bad) in config file: bandit.yaml"` ; baseline illisible → 2 ; `-b base.json -f csv` → 2 ; résultats → 1 ; sans résultats → 0 ; `--exit-zero` avec résultats → 0.

## C.5 `tests/unit/cli/test_baseline.py`
`test_bandit_baseline` : dépôt git temporaire avec `bandit.yaml` (profil `test` : `include: [start_process_with_a_shell]`) ; branches `benign1` (`benign_one.py` = okay.py → rc 0), `malicious` (+ `malicious.py` = os_system.py → rc 1), `benign2` (tout → rc 0) ; commande `bandit-baseline -c bandit.yaml -r . -p test`. Autres : non-repo → exit 2 ; échec git → 2 ; commit sans parent → 2 ; `subprocess` retourne 3 → exit 3 ; `initialize()` → `(None, None, None)` pour : non-repo, git absent, dépôt dirty, rapport existant (`-f txt` + `bandit_baseline_result.txt`), `-o` dans les args, temp file existant.

## C.6 `tests/unit/cli/test_config_generator.py`
Sans argument → SystemExit ; `--show-defaults` → `show_defaults` True ; `--out dummyfile` → `output_file == "dummyfile"` ; `get_config_settings()` == `yaml.safe_dump({plugin.name: gen_config(...)}, default_flow_style=False)` ; `main()` avec `--show-defaults` → 0.

## C.7 Formatters (`tests/unit/formatters/*`)
setUp commun : `BanditConfig()`, `BanditManager(conf, "file")`, tmpfile, issue `fname=tmp`, `lineno=4`, `linerange=[4]`, `test="hardcoded_bind_all_interfaces"`, texte `"Possible binding to all interfaces."`, ajoutée à `manager.results`.
- json : `get_issue_list` = `OrderedDict([(issue, [c1, c2])])` → `generated_at` non nul ; `results[0]` : filename, issue_severity, issue_confidence, issue_text, `line_number == 4`, `line_range == [4]`, `test_name`, `"candidates" in results[0]`, `more_info` non nul. yaml : mêmes assertions (le test appelle en fait `b_json.report` et charge en YAML).
- csv : `DictReader` → `line_number == "4"`, `line_range == "[4]"`, `col_offset == "8"`, `end_col_offset == "16"`, `more_info` non nul.
- xml : `testsuite.testcase["@classname"] == tmp`, `error["@message"] == text`, `testcase["@name"] == test`, `error["@more_info"]` non nul.
- custom : `col_offset=30`, `end_col_offset=38`, template `"{line},{col},{end_col},{severity},{msg}"` → `"4","30","38",severity,text`.
- text : `test_output_issue` (get_code → `"DDDDDDD"`, indent `"CCCCCCC"`, comparaison exacte des 5 lignes ± code ± lineno) ; `test_no_issues` (`"No issues identified."`) ; `test_report_nobaseline` (verbose, `files_list=["binding.py"]`, `scores=[{"SEVERITY":[0,0,0,1],"CONFIDENCE":[0,0,0,1]}]`, `skipped=[("abc.py","File is bad")]`, `excluded_files=["def.py"]`, `_totals={"loc":1000,"nosec":50,"skipped_tests":0}` + 8 compteurs à 1) → sous-chaînes `"Run started"`, `"Files in scope (1)"`, `"binding.py (score: "`, `"CONFIDENCE: 1"`, `"SEVERITY: 1"`, `"CWE: CWE-605 (https://cwe.mitre.org/data/definitions/605.html)"`, `"Files excluded (1):"`, `"def.py"`, `"Undefined: 1"`, `"Low: 1"`, `"Medium: 1"`, `"High: 1"`, `"Total lines skipped "`, `"(#nosec): 50"`, `"Total potential issues skipped due to specifically being "`, `"disabled (e.g., #nosec BXXX): 0"`, `"Total issues (by severity)"`, `"Total issues (by confidence)"`, `"Files skipped (1)"`, `"abc.py (File is bad)"` ; `test_report_baseline` (candidats indentés de 10 espaces, issue avec > 1 candidat sans code ni lineno).
- screen : idem avec couleurs ; `_totals={"loc":1000,"nosec":50}` ; chaînes `header("Files in scope (1):")`, `"\n\tbinding.py (score: {SEVERITY: 1, CONFIDENCE: 1})"`, `header("Files excluded (1):") + "\n\tdef.py"`, `"Total lines of code: 1000\n\tTotal lines skipped (#nosec): 50"`, `"Total issues (by severity):\n\t\tUndefined: 1\n\t\tLow: 1\n\t\tMedium: 1\n\t\tHigh: 1"`, idem confidence, `header("Files skipped (1):") + "\n\tabc.py (File is bad)"`.
- html : `div#skipped` unique avec `"abc.py"`/`"File is bad"` ; `span#loc == "1000"`, `span#nosec == "50"` ; `div#issue-0/1/2` ; classes `issue-sev-low/medium/high` ; issue-0 : 1 `.candidates`, 2 `.candidate`, 0 `.code` ; issue-1/2 : 1 `.code` ; `"some code"` ; `"AAAAAAA:"`, `"BBBBBBB"`, `"CCCCCCC"`, `"abc.py"`, `"Line number: 1"` ; `test_escaping` : `"<tag in code>"` absent tel quel.
- sarif : `$schema`, `version`, `driver.name == "Bandit"`, `organization == "PyCQA"`, `semanticVersion == version`, `rules[0].id == "B104"`, `name == "hardcoded_bind_all_interfaces"`, tags `"security"` et `"external/cwe/cwe-605"`, `precision == "medium"`, `invocations[0].executionSuccessful`, `endTimeUtc` non nul, `result.get("level") is None`, `message.text == issue.text`, `region.startLine == 4 == endLine`, `tmp in artifactLocation.uri`.

## C.8–C.16 Tests unitaires du cœur
- manager : `_matches_glob_list`, `_is_file_included` (6 cas), `_get_files_from_dir` (`os.walk` mocké → `exc == {"/a/c.ww"}`, `inc == {"/a/a.py","/a/b.py"}`), `populate_baseline` (succès / JSON invalide → `[]` + warning), `results_count` (LOW/MEDIUM/HIGH → `[3, 2, 1]`), format invalide → fichier quand même produit, `discover_files` (non récursif → listes vides ; récursif mocké ; exclusions `./x/*`, `./x/`, `./x`, `y` → `excluded_files == ["./x/y.py"]`/`["./x/y/z.py"]` ; `excluded_paths="a,b"` → dernier appel `("c", ["*.py","*.pyw"], ["a","b"], enforce_glob=False)` ; glob `test_*.py` → `files_list == ["./a.py","./test.py"]`, `excluded_files == ["test_a.py"]`), `run_tests` (fichier inexistant → dans `skipped`), `_compare_baseline_results`, `_find_candidate_matches`.
- util : `get_module_qualname_from_path` (voir `src/core/utils.rs`), `namespace_path_*`, `get_call_name` 1–3, `linerange` (`./examples/jinja2_templating.py`, `tree.body[8]` → `[11, 12, 13]`), `escaped_bytes_representation`, `parse_ini_file` (`"[bandit]\nexclude=/abc,/def"` → `{"exclude": "/abc,/def"}` ; `"[Blabla]\nsomething=something"` → `None`), `check_ast_node` (`"Call"` ok, `"Derp"`/`"walk"` → TypeError).
- issue : `__str__` exact, `as_dict`, `filter`, égalité, `get_code` sur octets invalides.
- config : settings, fichier inexistant → `ConfigError` contenant le chemin, YAML invalide, `get_option` pointé, legacy YAML/TOML (`test_1 == {"blacklist": {}, "exclude": set(), "include": {"B101","B604"}}` ; `test_4["exclude"] == {"B101"}` ; `test_2["blacklist"] == {"Call": [{"qualnames": ["telnetlib"], "level": "HIGH", "message": "{name} is considered insecure.", "name": "telnet"}]}` ; `test_3["blacklist"]["Call"|"Import"|"ImportFrom"] == [{"message": "{name} library appears to be in use.", "name": "pickle", "qualnames": ["pickle.loads"]}]` ; `include == {"B001"}` ; `test_5["exclude"] == {"B001"}` ; message de dépréciation ; erreurs `blacklist_*` ; `"[]"` → `"Error parsing file."`).
- test_set : registre factice (plugin `B000` sur `Str` + blacklists `telnet/B401/HIGH/["telnetlib"]` et `marshal/B302/["marshal.load","marshal.loads"]`) : defaults → 1 test `Str` ; include/exclude `B000` ; `B001` builtin sur Import/ImportFrom/Call ; exclude `B001` → 0 ; exclude `B302,B401` → 0 ; filtrage des entrées (`_config` de longueur 2/1) ; `include ["B001","B401"]` → 1 ; profil `{"include": ["B001"], "blacklist": {"Call": [marshal]}}` → seulement `Call`.
- blacklisting : `report_issue({"level":"HIGH","message":"test {name}","id":"B000"}, "name")` → `B000`, `HIGH`, cwe `{}`, confiance `HIGH`, `"test name"` ; défauts → `LEGACY`, `MEDIUM`.
- docs_util : `B304 == B305 == base + "blacklists/blacklist_calls.html#b304-b305-ciphers-and-modes"`, `B101 → plugins/b101_assert_used.html`, `B413 → blacklists/blacklist_imports.html#b413-import-pycrypto`.

# Partie D — Fichiers d'exemples
96 fichiers (`examples/` copié verbatim). `__init__.py` et `init-py-test/__init__.py` vides (qualname des modules) ; `nonsense.py` = `test(hi` (SyntaxError) ; `nonsense2.py` = 40 octets gzip (SyntaxError) ; `trojansource.py` UTF-8 avec U+202E/U+2066/U+2069 (1 issue HIGH/MEDIUM) ; `trojansource_latin1.py` ISO-8859-1 avec cookie (0 issue) ; `long_set.py` 65 Ko ; `skip.py` (`loc 7`, `nosec 2`, 5 issues) ; `nosec.py` et `multiline_statement.py` : voir docs/spec/core.md §1.5 et `tests/functional.rs`.
