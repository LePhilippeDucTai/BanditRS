# Spécification du cœur de bandit (référence pour le portage Rust)

Source : `/home/user/bandit` @ `1d3053d`, `python_requires>=3.10`. Ce document reproduit
l'analyse ligne à ligne du cœur Python (`bandit/core/*.py`). Les références `fichier:ligne`
renvoient au dépôt Python.

Corrections importantes par rapport aux idées reçues :

| Idée reçue | Réalité |
|---|---|
| `visit_AsyncFunctionDef` | **N'existe pas.** `AsyncFunctionDef` suit le chemin générique (`node_visitor.py:218-227`) → tests du type `"AsyncFunctionDef"` (aucun plugin). |
| `context["statement"]` | **Jamais renseigné.** `Context.statement` renvoie toujours `None`. |
| multiprocessing dans `run_tests` | **Aucun** : boucle mono-thread (`manager.py:277`). |
| `constants.CWEMAP`, `TAB`, `SEVERITY_LEVEL`, `CONFIDENCE_LEVEL`, `PROFILE` | N'existent pas (voir §9). |
| setting `plugins_dir` | Pas un vrai setting (seul `plugin_name_pattern` existe). |
| `get_plugin_id` / `check_plugin_id` | L'API réelle est `Manager.get_test_id()` et `Manager.check_id()` (`extension_loader.py:59,102`). |
| Un plugin renvoie une liste d'issues | Non : exactement une `Issue` ou `None`. |
| `ast.Str` / `ast.Bytes` | Seul `ast.Constant` est produit ; `Str`/`Bytes` sont des *étiquettes de type de check*. |

---

## 1. `BanditManager` — `bandit/core/manager.py`

### 1.1 Constructeur (`manager.py:35-73`)

```python
def __init__(self, config, agg_type, debug=False, verbose=False,
             quiet=False, profile=None, ignore_nosec=False)
```
Champs : `debug`, `verbose`, `quiet`, `ignore_nosec`, `b_conf`, `files_list=[]`, `excluded_files=[]`,
`b_ma=BanditMetaAst()`, `skipped=[]`, `results=[]`, `baseline=[]`, `agg_type`, `metrics=Metrics()`,
`b_ts=BanditTestSet(config, profile)`, `scores=[]`. `profile` falsy → `{}`. `agg_type` ∈ {`"file"`, `"vuln"`}
(consulté seulement par json/yaml).

### 1.2 Découverte des fichiers — `discover_files` (`manager.py:200-259`)

```python
excluded_path_globs = self.b_conf.get_option("exclude_dirs") or []   # :212
included_globs      = self.b_conf.get_option("include") or ["*.py"]  # :213
```
- **`include` par défaut** : sans fichier de config, `BanditConfig` pose `include = ["*.py", "*.pyw"]`
  (`config.py:80`). Le repli `or ["*.py"]` ne joue que si un fichier de config est fourni sans clé `include`.
- **`--exclude`** (`excluded_paths`, séparés par des virgules) (`:216-221`) : chaque entrée qui est un
  répertoire existant devient `os.path.join(path, "*")` ; puis ajoutée à `excluded_path_globs`.
  ⚠️ Python mute la liste de la config en place (corrigé dans BanditRS : état possédé).
- Défaut CLI de `--exclude` : `",".join(constants.EXCLUDE)`.

Par cible (`:224-256`) :
- `os.path.isdir(fname)` : si `recursive` → `_get_files_from_dir` (`os.walk`, chaque fichier testé par
  `_is_file_included(path, included_globs, excluded_path_strings, enforce_glob=True)` ; chemins
  `os.path.join(root, filename)` sans préfixe `./`) ; sinon
  `LOG.warning("Skipping directory (%s), use -r flag to scan contents")` et rien n'est ajouté.
- sinon (fichier explicite) : `_is_file_included(..., enforce_glob=False)` — **le glob d'inclusion est
  ignoré** (`bandit foo.txt` scanne le fichier). Si inclus : `fname != "-"` → `os.path.join(".", fname)`
  (préfixe `./` ; un chemin absolu reste absolu) ; `"-"` conservé tel quel. Sinon → `excluded_files`.

`self.files_list = sorted(files_list)`, `self.excluded_files = sorted(excluded_files)` (depuis des `set` :
dédoublonnés, ordre lexicographique).

**`_is_file_included` (`manager.py:392-418`)** :
```python
if _matches_glob_list(path, included_globs) or not enforce_glob:
    if not _matches_glob_list(path, excluded_path_strings) and not any(
        x in path for x in excluded_path_strings):
        return_value = True
```
Exclusion = glob **ou** sous-chaîne (`--exclude y` exclut `./x/y/z.py`). `_matches_glob_list` =
`fnmatch.fnmatch` sur le chemin complet.

### 1.3 `run_tests` (`manager.py:261-299`)

Mono-thread. `new_files_list = list(self.files_list)`. Barre de progression `rich` si > 50 fichiers et
niveau de log ≤ INFO (cosmétique). Par fichier :
- `fname == "-"` → lecture de stdin en binaire dans un `BytesIO`, tous les `"-"` de `new_files_list`
  renommés `"<stdin>"`, puis `_parse_file("<stdin>", fdata, new_files_list)`.
- sinon `open(fname, "rb")` → `_parse_file(fname, fdata, new_files_list)`.
- `OSError` → `self.skipped.append((fname, e.strerror))` et retrait de `new_files_list`.

Puis `self.files_list = new_files_list` et `self.metrics.aggregate()`.

### 1.4 `_parse_file` (`manager.py:301-344`)

```python
data  = fdata.read()          # bytes
lines = data.splitlines()     # list[bytes]
self.metrics.begin(fname)
self.metrics.count_locs(lines)
nosec_lines = dict()
try:
    fdata.seek(0)
    tokens = tokenize.tokenize(fdata.readline)
    if not self.ignore_nosec:
        for toktype, tokval, (lineno, _), _, _ in tokens:
            if toktype == tokenize.COMMENT:
                nosec_lines[lineno] = _parse_nosec_comment(tokval)
except tokenize.TokenError:
    pass
score = self._execute_ast_visitor(fname, fdata, data, nosec_lines)
```
- `metrics.begin`/`count_locs` **avant** le parsing : un fichier en erreur de syntaxe garde son bloc de
  métriques (`loc`) sans compteurs d'issues.
- Chaque ligne COMMENT a une entrée, valeur `None` pour un commentaire non-nosec ; `None` ≠ ensemble vide.
- `tokenize.TokenError` avalée ; `--ignore-nosec` → `nosec_lines = {}`.
- `ast.parse` reçoit les **bytes** (cookie PEP 263 honoré).

Exceptions (`:325-344`) : `KeyboardInterrupt` → `sys.exit(2)` ; `SyntaxError` →
`skipped.append((fname, "syntax error while parsing AST from file"))` ; toute autre `Exception` →
`LOG.error(...)` + `skipped.append((fname, "exception while scanning file"))` ; le fichier est retiré de
`new_files_list` dans les deux cas.

### 1.5 Analyse des commentaires nosec (`manager.py:27-28, 464-499`)

```python
NOSEC_COMMENT       = re.compile(r"#\s*nosec:?\s*(?P<tests>[^#]+)?#?")
NOSEC_COMMENT_TESTS = re.compile(r"(?:(B\d+|[a-z\d_]+),?)+", re.IGNORECASE)
```
`_parse_nosec_comment(comment)` : `search` (un `# type: ... # nosec B607 # noqa` matche au second `#`) ;
pas de nosec → `None` ; groupe `tests` vide → **ensemble vide** (blanket) ; sinon chaque `group(1)` de
`finditer` est résolu par `_find_test_id_from_nosec_string` : `extman.check_id(m)` → l'id ; sinon
`extman.get_test_id(m)` (nom de plugin/blacklist) → son id ; sinon
`LOG.warning("Test in comment: %s is not a test name or id, ignoring")`.

| commentaire | valeur |
|---|---|
| `# nosec` / `#nosec` / `# nosec  # noqa` | `set()` (blanket) |
| `# nosec: B101` | `{"B101"}` |
| `# nosec B101, B102` / `# nosec B101 B102` | `{"B101","B102"}` |
| `# nosec B101,B102` (sans espace) | `{"B102"}` en Python (bug ; **corrigé** dans BanditRS : les deux ids) |
| `# noqa` | `None` |
| `# type: ... # nosec B607 # noqa: E501` | `{"B607"}` |
| `#nosec (on the line)` | `set()` (chaque mot échoue et produit un warning) |

Les noms sont acceptés : `# nosec: import_subprocess`, `# nosec md5`,
`# nosec subprocess_popen_with_shell_equals_true`.

Instructions multi-lignes : la map est indexée par la ligne du commentaire ; la correspondance passe par
`utils.get_nosec` sur `context["linerange"]` (première ligne non-`None` gagne, pas d'union).

### 1.6 Baseline

- `populate_baseline(data)` : `json.loads(data)`, `self.baseline = [issue_from_dict(j) for j in jdata["results"]]` ;
  toute exception → `LOG.warning("Failed to load baseline data: %s")` et `baseline = []`.
- `filter_results(sev, conf)` : `results = [i for i in self.results if i.filter(sev, conf)]` ; sans baseline
  → liste ; sinon `unmatched = _compare_baseline_results(baseline, results)` puis
  **`_find_candidate_matches(unmatched, results)` → `OrderedDict[Issue, list[Issue]]`**. Seuls les formatters
  `@accepts_baseline` (custom, html, json, screen, txt) gèrent les deux formes.
- `_compare_baseline_results` : `[a for a in results if a not in baseline]` (`Issue.__eq__`, sans lignes).
- `_find_candidate_matches` : `OrderedDict` issue → `[i for i in results if unmatched == i]`.
- `get_issue_list(sev_level=LOW, conf_level=LOW)` = `filter_results` ; `results_count` = `len(...)`.

### 1.7 `output_results` (`manager.py:141-198`)

Formatter inconnu → repli `"screen"` si `sys.stdout.isatty() and NO_COLOR unset and TERM != "dumb"`,
sinon `"txt"`. `custom` reçoit `template=`, les autres `lines=`. Toute exception →
`RuntimeError(f"Unable to output report using '{fmt}' formatter: {e}")`.

### 1.8 Codes de sortie (`cli/main.py`)

- `0` : aucun résultat au-dessus des seuils, ou `--exit-zero`.
- `1` : `results_count(sev, conf) > 0`.
- `2` : erreur de config, pas de cible, profil introuvable / include∩exclude non vide, baseline illisible,
  baseline avec un formatter non compatible, aucun test à exécuter (« No tests would be run »), plusieurs
  `.bandit`, `KeyboardInterrupt`.
- Seuils : `RANKING[args.severity - 1]` (compteur `-l`, défaut 1 → `UNDEFINED`), ou `--severity-level
  all|low|medium|high` → 1..4 ; idem confiance.

---

## 2. `BanditNodeVisitor` — `bandit/core/node_visitor.py`

### 2.1 Construction
`BanditNodeVisitor(fname, fdata, metaast, testset, debug, nosec_lines, metrics)` : `scores =
{"SEVERITY": [0]*4, "CONFIDENCE": [0]*4}`, `depth=0`, `imports=set()`, `import_aliases={}`,
`tester=BanditTester(...)`, `namespace = get_module_qualname_from_path(fname)` ou `""` avec
`LOG.warning("Unable to find qualified name for module: %s")`.

### 2.2 Parcours — `generic_visit` (`:238-262`)
Marcheur maison (pas `ast.NodeVisitor`). Pour chaque champ (`ast.iter_fields`) : liste → chaque élément
`ast.AST` reçoit `_bandit_sibling = value[idx+1]` (ou `None` pour le dernier ; peut être `None` si le
slot suivant est `None`) et `_bandit_parent = node` ; puis `pre_visit → visit → generic_visit → post_visit`.
Champ simple → `_bandit_sibling = None`. **Le nœud `Module` racine n'est jamais visité** (ses enfants ont
`_bandit_parent = Module`).

### 2.3 `pre_visit` (`:189-216`) — le contexte
Reconstruit à chaque nœud : `imports` (set partagé), `import_aliases` (dict partagé), `lineno` si
`hasattr(node, "lineno")`, `col_offset` si présent, `end_col_offset` si présent, `node`,
`linerange = utils.linerange(node)`, `filename`, `file_data`. Retourne toujours `True`.
Clés ajoutées ensuite par les visiteurs : `function`, `qualname`, `name`, `call`, `module`, `str`, `bytes`,
`linerange` (surchargée). **`statement` n'existe pas.**

### 2.4 `visit` (`:218-227`)
`visit_<ClassName>` s'il existe (le générique n'est alors PAS appelé), sinon `tester.run_tests(context, name)`.

### 2.5 Les visiteurs
- `visit_ClassDef` : `namespace = namespace_path_join(namespace, node.name)`. **Aucun test.**
- `visit_FunctionDef` : `context["function"]=node` ; `qualname = self.namespace + "." + node.name` (concaténation
  brute) ; `name = qualname.split(".")[-1]` ; push namespace ; `run_tests(context, "FunctionDef")`.
- `visit_Call` : `context["call"]=node` ; `qualname = get_call_name(node, import_aliases)` ; `name` = dernier
  segment ; `run_tests(context, "Call")`.
- `visit_Import` : pour chaque alias : si `asname` → `import_aliases[asname] = name` ; `imports.add(name)` ;
  `context["module"] = name` (le dernier gagne). Un seul `run_tests(context, "Import")`.
- `visit_ImportFrom` : `module is None` → `return self.visit_Import(node)` (tests `"Import"`). Sinon par alias :
  `import_aliases[asname or name] = module + "." + name` ; `imports.add(module + "." + name)` ;
  `context["module"] = module` ; `context["name"] = name` (dernier gagne). Un `run_tests(context, "ImportFrom")`.
- `visit_Constant` : `str` → `visit_Str`, `bytes` → `visit_Bytes`, sinon rien.
- `visit_Str` / `visit_Bytes` : `context["str"|"bytes"] = node.value` ; **seulement si le parent n'est pas un
  `ast.Expr`** (docstring) : `context["linerange"] = linerange(node._bandit_parent)` (celle du **parent**) et
  `run_tests(context, "Str"|"Bytes")`.

### 2.6 `post_visit` (`:229-236`)
`depth -= 1` ; si `FunctionDef` ou `ClassDef` : `namespace = namespace_path_split(namespace)[0]`
(`AsyncFunctionDef` : ni push ni pop).

### 2.7 `process` (`:278-297`)
```python
f_ast = ast.parse(data)          # aucun flag
self.generic_visit(f_ast)
self.context = {"file_data": self.fdata, "filename": self.fname, "lineno": 0, "linerange": [0, 1], "col_offset": 0}
self.update_scores(self.tester.run_tests(self.context, "File"))
```
Le contexte `"File"` n'a ni `node`, ni `imports`, ni `end_col_offset` (B613 uniquement).

---

## 3. `Context` — `bandit/core/context.py`

| membre | sémantique exacte |
|---|---|
| `call_args` | `[]` sans `call` ; par argument positionnel : **`arg.attr` si le nœud a un attribut `attr`** (`ast.Attribute` → dernier nom), sinon `_get_literal_value(arg)`. |
| `call_args_count` | `len(call.args)` ou `None`. |
| `call_function_name` | `ctx.get("name")`. |
| `call_function_name_qual` | `ctx.get("qualname")`. |
| `call_keywords` | `None` sans `call` ; dict `{kw.arg: kw.value.attr if hasattr(value,"attr") else _get_literal_value(kw.value)}` (`**kwargs` → clé `None`). |
| `node`, `string_val` (`str`), `bytes_val` (`bytes`), `filename`, `file_data`, `import_aliases` | accès directs. |
| `string_val_as_escaped_bytes` | `string_val.encode("unicode_escape")` ; sinon `escaped_bytes_representation(bytes_val)` ; sinon `None`. |
| `statement` | toujours `None`. |
| `function_def_defaults_qual` | `[get_qual_attr(d, import_aliases) for d in node.args.defaults]` si présents. |

**`_get_literal_value(literal)`** : `Constant` : bool → `str(value)` (`"True"`/`"False"`), `None` →
`"None"`, sinon valeur brute (int, float, complex, str, bytes, Ellipsis) ; `List` → liste récursive ;
`Tuple` → tuple ; `Set` → set (élément non hachable → `TypeError`) ; `Dict` → `dict(zip(keys, values))`
(**nœuds AST bruts**) ; `Name` → `id` ; sinon `None`.

Méthodes :
- `get_call_arg_value(name)` : `call_keywords.get(name)` (implicitement `None`).
- `check_call_arg_value(name, values=None)` : valeur `None` → `None` ; sinon `values` non-liste emballé dans
  une liste ; `True` si `arg_value == val` pour un des `values`, sinon `False`.
- `get_lineno_for_call_arg(name)` : `key.value.lineno` du keyword correspondant, sinon `None`.
- `get_call_arg_at_position(n)` : `if max_args and n < max_args: getattr(arg, "attr", None) or _get_literal_value(arg)`, sinon `None`.
- `is_module_being_imported(m)` : `ctx.get("module") == m`.
- `is_module_imported_exact(m)` : `m in imports`.
- `is_module_imported_like(m)` : `any(m in imp for imp in imports)` (**sous-chaîne**).

---

## 4. `BanditTester` — `bandit/core/tester.py`

`run_tests(raw_context, checktype)` : pour chaque test de `testset.get_tests(checktype)` (`name = test.__name__`,
`context = Context(copy.copy(raw_context))`, appel `test(context)` ou `test(context, test._config)`).

**Si `result is not None`** (ordre exact) :
1. `nosec_tests_to_skip = _get_nosecs_from_contexts(temp_context, test_result=result)` — **avant** le
   défaut de `lineno` (`nosec_lines.get(result.lineno)` ne compte que si le plugin a fixé `lineno`).
2. `result.fname = context["filename"]` ; 3. `result.fdata = context["file_data"]` ;
4. `if result.lineno is None: result.lineno = context["lineno"]` ;
5. `if result.linerange == []: result.linerange = context["linerange"]` ;
6. `if result.col_offset == -1: result.col_offset = context["col_offset"]` ;
7. `result.end_col_offset = context.get("end_col_offset", 0)` (**toujours écrasé**) ;
8. `result.test = name` ; 9. `if result.test_id == "": result.test_id = test._test_id` ;
10. porte nosec : `None` → rapporter ; ensemble vide → `metrics.note_nosec()` et `continue` ; `test_id` dedans
    → `metrics.note_skipped_test()` et `continue` ; sinon rapporter ;
11. `results.append(result)` ; `scores["SEVERITY"][RANKING.index(sev)] += RANKING_VALUES[sev]` (idem confiance).

**Si `result is None`** : recalcul de `nosec_tests_to_skip` ; si non vide et `test._test_id` dedans →
`LOG.warning("nosec encountered (Bxxx), but no failed test on file <f>:<lineno>")`.

Exception dans un test → `report_error` (`LOG.error("Bandit internal error running: <test> on file <f> at
line <n>: <err><traceback>")`), relancée si `debug`.

`_get_nosecs_from_contexts` : `base = nosec_lines.get(result.lineno)` si résultat ; `ctx_tests =
utils.get_nosec(nosec_lines, context)` (premier non-`None` sur `linerange`) ; les deux `None` → `None` ;
sinon union des non-`None`.

---

## 5. `BanditTestSet`, `test_properties`, `extension_loader`

- `checks(*args)` : `func._checks` ; `"File"` accepté tel quel ; les autres noms validés par
  `utils.check_ast_node`. `takes_config` : nu → `func.__name__` ; `takes_config("x")` → `"x"`. `test_id(id)`.
  `accepts_baseline`.
- `extension_loader.MANAGER` : `formatters_mgr`/`formatter_names` ; `plugins` (ordre des entry points
  `setup.cfg`), `plugins_by_id`, `plugins_by_name` ; `blacklist` = `{node_type: [conf...]}` fusionné depuis
  `calls`/`imports` (`gen_blacklist()`), `blacklist_by_id`, `blacklist_by_name` ; `builtin = ["B001"]` ;
  `get_test_id(name)` : plugin → id ; sinon blacklist → id ; sinon `None` ; `check_id(t)` :
  `t in plugins_by_id or t in blacklist_by_id or t in builtin` ; `validate_profile` : warning
  `"Unknown test found in profile: X"` pour les ids inconnus, `ValueError(f"Non-exclusive include/exclude
  test sets: {union}")` si intersection.
- Donnée de blacklist : `{"name", "id", "cwe", "message", "qualnames", "level"}` (`build_conf_dict`, level
  défaut `"MEDIUM"`).
- `BanditTestSet(config, profile)` : `filtering = _get_filter(config, profile)` ; `plugins = [p for p in
  extman.plugins if p._test_id in filtering]` + `_load_builtins(filtering, profile)` ; `_load_tests`.

**`_get_filter`** :
```python
inc = set(profile.get("include", []));  exc = set(profile.get("exclude", []))
all_blacklist_tests = {t["id"] for tests in extman.blacklist.values() for t in tests}
if "B001" in inc:
    if not inc.intersection(all_blacklist_tests): inc.update(all_blacklist_tests)
    inc.discard("B001")
if "B001" in exc: (symétrique)
filtered = inc if inc else (set(plugins_by_id) | set(builtin) | all_blacklist_tests)
return filtered - exc
```
**`_load_builtins`** : si `profile["blacklist"]` existe (config legacy) l'utiliser ; sinon `{node_type: [t
for t in tests if t["id"] in filtering]}` en supprimant les types vides ; rien → pas de test B001 ; sinon
`blacklist._test_id = "B001"`, `_checks = blacklist.keys()`, `_config = blacklist`.
**`_load_tests`** : pour chaque plugin avec `_takes_config` : `cfg = config.get_option(name)` ; si `None` →
`module.gen_config(name)` ; `plugin._config = cfg`. Puis `tests[check].append(plugin)` par type.
`get_tests(checktype)` : `tests.get(checktype) or []`.

Profil (`cli/main.py:111-122`) : `-p NAME` → `config["profiles"][NAME]` (peut porter `blacklist`) ; sinon
`{"include": set(config["tests"] or []), "exclude": set(config["skips"] or [])}` ; `-t`/`-s` ajoutés.

---

## 6. `BanditConfig` — `bandit/core/config.py`

- Sans fichier : `_config = {"plugin_name_pattern": "*.py", "include": ["*.py", "*.pyw"]}`.
- Avec fichier : `open(config_file, "rb")` ; `OSError` → `ConfigError("Could not read config file.", path)` ;
  suffixe `.toml` → `tomllib.load(f).get("tool", {}).get("bandit", {})` (`TOMLDecodeError` →
  `ConfigError("Error parsing file.")`) ; sinon `yaml.safe_load(f)` (`YAMLError` → même erreur) ; puis
  `validate(config_file)` ; puis `if not isinstance(_config, dict): ConfigError("Error parsing file.")` ; puis
  `convert_legacy_config()` ; `_init_settings()`. Un fichier chargé **remplace** tout (pas de fusion des défauts).
- `ConfigError.message = f"{config_file} : {message}"` ; `ProfileNotFound(config_file, profile)` →
  `"Unable to find profile (P) in config file: C"`.
- `get_option("a.b")` : descente niveau par niveau ; `None` si un niveau manque ou est falsy.
- `get_setting(name)` : seul `plugin_name_pattern` (défaut `"*.py"`, surchargé par `get_option`).
- `convert_legacy_config` : `updated_profiles = convert_names_to_ids()` ; `bad_calls, bad_imports =
  convert_legacy_blacklist_data()` ; si profils → `convert_legacy_blacklist_tests(updated_profiles, bad_calls,
  bad_imports)` alors que la signature est `(profiles, bad_imports, bad_calls)` ⚠️ **inversion conservée**
  (les tests `test_config.py` en dépendent).
- `convert_names_to_ids` : `include`/`exclude` mappés par `get_test_id(i) or i` en sets.
- `convert_legacy_blacklist_data` : `blacklist_calls.bad_name_sets` → dicts avec `name = key`, `message =
  message.replace("{func}", "{name}")` ; `blacklist_imports.bad_import_sets` → `name = key`,
  `message.replace("{module}", "{name}")`, `imports` → `qualnames`. Log `"Legacy blacklist data found in
  config, overriding data plugins"` si non vide.
- `convert_legacy_blacklist_tests` : par profil, `blacklist_calls` ∈ include et ∉ exclude →
  `blacklist["Call"] += bad_calls` ; `blacklist_imports` → `blacklist["Import"|"ImportFrom"|"Call"] +=
  bad_imports` ; les noms legacy `blacklist_calls`, `blacklist_imports`, `blacklist_import_func` sont remplacés
  par `B001` dans les deux sets ; si `B001` dans les deux, retiré de `exclude`. Résultat dans `profile["blacklist"]`.
- `validate(path)` : si `profiles` présent → legacy ; pour chaque profil, une référence à
  `blacklist_imports`/`blacklist_import_func`/`blacklist_calls` sans bloc de données correspondant →
  `ConfigError("Config file has an include or exclude reference to legacy test '{0}' but no configuration data
  for it. Configuration data is required for this test. Please consider switching to the new config file
  format, the tool 'bandit-config-generator' can help you with this.")` ; enfin warning
  `"Config file '%s' contains deprecated legacy config data. Please consider upgrading to the new config
  format. The tool 'bandit-config-generator' can help you with this. Support for legacy configs will be
  removed in a future bandit version."` (argument = chemin, `""` dans les tests).

---

## 7. `Issue` et `Cwe` — `bandit/core/issue.py`

`Cwe` : constantes (0 NOTSET, 20, 22, 78, 79, 80, 89, 94, 155, 259, 284, 295, 319, 326, 327, 330, 377, 400,
494, 502, 605, 703, 732, 838) ; `MITRE_URL_PATTERN = "https://cwe.mitre.org/data/definitions/%s.html"` ;
`link()` `""` si NOTSET ; `__str__` `"CWE-%i (%s)"` ou `""` ; `as_dict()` `{"id", "link"}` ou `{}` ;
`from_dict` → `int(data["id"])` ou NOTSET ; égalité sur `id`.

`Issue(severity, cwe=0, confidence="UNDEFINED", text="", ident=None, lineno=None, test_id="", col_offset=-1,
end_col_offset=0)` ; `fname=""`, `fdata=None`, `test=""`, `linerange=[]`. Sentinelles utilisées par le tester.
- `__str__` : `"Issue: '<text>' from <test_id>:<ident or test>: CWE: <cwe>, Severity: <sev> Confidence: <conf> at <fname>:<lineno>:<col_offset>"`.
- `__eq__` : `text, severity, cwe, confidence, fname, test, test_id` (pas les lignes) ; `__hash__ = id(self)`.
- `filter(sev, conf)` : `RANKING.index(self.severity) >= RANKING.index(sev)` et idem confiance.
- `get_code(max_lines=3, tabbed=False)` :
  ```python
  max_lines = max(max_lines, 1)
  lmin = max(1, self.lineno - max_lines // 2)
  lmax = lmin + len(self.linerange) + max_lines - 1
  tmplt = "%i\t%s" if tabbed else "%i %s"
  for line in range(lmin, lmax):          # lmax EXCLUSIF
      text = fdata.readline() if fname == "<stdin>" else linecache.getline(fname, line)
      if isinstance(text, bytes): text = text.decode("utf-8")
      if not len(text): break
      lines.append(tmplt % (line, text))
  return "".join(lines)
  ```
  (`linecache.getline` renvoie la ligne **avec** son `\n` ; newlines universelles ; `<stdin>` = octets bruts).
- `as_dict(with_code=True, max_lines=3)` : `filename, test_name, test_id, issue_severity, issue_cwe, issue_confidence,
  issue_text, line_number, line_range, col_offset, end_col_offset` (+ `code`).
- `from_dict` : requiert `code, filename, issue_severity, issue_cwe, issue_confidence, issue_text, test_name,
  test_id, line_number, line_range` (KeyError sinon) ; `col_offset`/`end_col_offset` défaut 0.

---

## 8. `utils.py`

| fonction | sémantique |
|---|---|
| `_get_attr_qual_name(node, aliases)` | `Name` → `aliases[id]` ou `id` ; `Attribute` → récursion sur `.value`, `"." + attr`, puis **re-test du nom composé dans `aliases`** ; sinon `""` (`x[0].bar` → `".bar"`, `a.list[0](x)` → `""`). |
| `get_call_name(node, aliases)` | `func` `Name` → id résolu ; `Attribute` → `_get_attr_qual_name` ; sinon `""`. |
| `get_func_name(node)` | `node.name`. |
| `get_qual_attr(node, aliases)` | `Attribute` seulement : `prefix = aliases.get(value.id, value.id)`, `""` si `value` n'est pas un `Name` ; `f"{prefix}.{attr}"` ; sinon `""`. |
| `get_module_qualname_from_path(path)` | `head, tail = os.path.split(path)` ; vides → `InvalidModulePath('Invalid python file path: "P" Missing path or file name')` ; `qname = [splitext(tail)[0]]` ; tant que `head not in ["/", ".", ""]` : si `isfile(head + "/__init__.py")` → `head, tail = split(head)`, `qname.insert(0, tail)` ; sinon stop ; `".".join(qname)`. Pas de résolution des symlinks. |
| `namespace_path_join(base, name)` | `f"{base}.{name}"`. |
| `namespace_path_split(path)` | `tuple(path.rsplit(".", 1))`. |
| `escaped_bytes_representation(b)` | `b.decode("unicode_escape").encode("unicode_escape")`. |
| `calc_linerange(node)` | memoïsé ; min/max des `lineno` du nœud et de **tous** ses descendants (lignes de début) ; sentinelles `9999999999`/`-1`. |
| `linerange(node)` | `hasattr(node, "lineno")` → `list(range(lineno, end_lineno + 1))` ; sinon (Module, arguments, comprehension, withitem, match_case…) : `body`/`orelse`/`handlers`/`finalbody` vidés temporairement, min/max via `calc_linerange` sur les enfants restants, `(0, 1)` si `max == -1` ; puis contournement bug 16806 : si `node._bandit_sibling` a un `lineno` et `sibling.lineno - min > 1` → `range(min, sibling.lineno)`. |
| `concat_string(node, stop=None)` | remonte `_bandit_parent` tant que `BinOp` ; collecte les feuilles gauche/droite (récursif, en s'arrêtant à `stop`) ; renvoie `(racine, " ".join(constantes str))` (**jointes par des espaces**). |
| `get_called_name(node)` | `func.attr` (Attribute) / `func.id` (Name) / `""`. |
| `parse_ini_file(f_loc)` | `configparser`, `dict(config.items("bandit"))` ; erreur/section absente → warning `"Unable to parse config file %s or missing [bandit] section"` et `None`. |
| `check_ast_node(name)` | Python ≥ 3.12 accepte `"Num"`, `"Str"`, `"Ellipsis"`, `"NameConstant"`, `"Bytes"` ; sinon `getattr(ast, name)` doit être une sous-classe de `ast.AST` ; sinon `TypeError(f"Error: {name} is not a valid node type in AST")`. |
| `get_nosec(nosec_lines, context)` | `for lineno in context["linerange"]: if nosec_lines.get(lineno) is not None: return it` → sinon `None`. |

---

## 9. `constants.py`
```python
plugin_name_pattern = "*.py"
RANKING = ["UNDEFINED", "LOW", "MEDIUM", "HIGH"]
RANKING_VALUES = {"UNDEFINED": 1, "LOW": 3, "MEDIUM": 5, "HIGH": 10}
CRITERIA = [("SEVERITY", "UNDEFINED"), ("CONFIDENCE", "UNDEFINED")]
CONFIDENCE_DEFAULT = "UNDEFINED"
FALSE_VALUES = [None, False, "False", 0, 0.0, 0j, "", (), [], {}]
log_format_string = "[%(module)s]\t%(levelname)s\t%(message)s"
EXCLUDE = (".svn", "CVS", ".bzr", ".hg", ".git", "__pycache__", ".tox", ".eggs", "*.egg")
```

## 10. `metrics.py`
`data["_totals"] = {"loc": 0, "nosec": 0, "skipped_tests": 0}` puis `for rank in RANKING: for criteria in
CRITERIA: totals[f"{criteria}.{rank}"] = 0` (clés **entrelacées** SEVERITY.UNDEFINED, CONFIDENCE.UNDEFINED, …).
`begin(fname)` crée `{"loc", "nosec", "skipped_tests"}` ; `note_nosec`, `note_skipped_test` ;
`count_locs(lines)` : `sum(bool(tmp) and not tmp.startswith(b"#") for tmp in (l.strip() for l in lines))` ;
`count_issues(scores)` : `SEVERITY.*` puis `CONFIDENCE.*` = `score // RANKING_VALUES[rank]` ;
`aggregate()` : `Counter` sur tous les blocs (y compris `_totals`) → `data["_totals"]`.

## 11. `blacklisting.py`
```python
def report_issue(check, name):
    return Issue(severity=check.get("level", "MEDIUM"), confidence="HIGH",
                 cwe=check.get("cwe", Cwe.NOTSET),
                 text=check["message"].replace("{name}", name),
                 ident=name, test_id=check.get("id", "LEGACY"))
```
`blacklist(context, config)` : voir `src/core/blacklist.rs` (documentation de module) pour les règles
d'appariement (appels : égalité exacte, cas `__import__`/`importlib.import_module` ; imports : `startswith`
sur `prefix + alias.name` ; premier match gagne).

## 12. `docs_utils.py`
`get_url(bid)` : base `https://bandit.readthedocs.io/en/{version}/` ; plugin →
`plugins/{bid.lower()}_{func.__name__}.html` ; blacklist → `blacklists/blacklist_{calls|imports}.html#{id}-{name}`
(`_` → `-`, tout en minuscules ; `B304`/`B305` → `b304-b305` + `ciphers-and-modes` ; `B313`–`B320` →
`b313-b320`) ; sinon la base.

## Gestion des versions de Python
`ast.parse(data)` sans flag sur des **bytes** ; `end_lineno`/`end_col_offset` requis (≥ 3.8) ; seuls des
`ast.Constant` sont produits ; en CPython 3.11 les constantes d'un `JoinedStr` portent la plage de toute la
chaîne (3.12+ : plages réelles des segments, fusion des segments adjacents).
