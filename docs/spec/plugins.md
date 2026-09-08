# Spécification des plugins et des blacklists (référence pour le portage Rust)

Reproduction ligne à ligne de `bandit/plugins/*.py` et `bandit/blacklists/*.py` (dépôt Python @ `1d3053d`).
Les noms enregistrés sont ceux de `setup.cfg` ; `Issue.test` est le **nom de la fonction Python**.

## Partie 0 — Mécanique dont dépendent les plugins

- `RANKING_VALUES = {"UNDEFINED": 1, "LOW": 3, "MEDIUM": 5, "HIGH": 10}` ; `bandit.LOW == "LOW"` etc.
- Post-traitement de chaque `Issue` retournée (`tester.py::run_tests`) : `fname`, `fdata`, `lineno` (si `None`),
  `linerange` (si `[]`), `col_offset` (si `-1`), `end_col_offset` (toujours `context.get("end_col_offset", 0)`),
  `test = fonction.__name__`, `test_id` (si `""`). Une exception dans un plugin est journalisée et l'issue perdue.
- Dispatch : `visit_ClassDef` (namespace seulement), `visit_FunctionDef` (tests `"FunctionDef"` ;
  `AsyncFunctionDef` → aucun plugin), `visit_Call`, `visit_Import`/`visit_ImportFrom` (une exécution par
  instruction), `visit_Constant` → `visit_Str`/`visit_Bytes` uniquement si le parent n'est pas un `ast.Expr`
  (docstring) avec `linerange` du parent ; après le parcours, un contexte `"File"` (`lineno=0`,
  `linerange=[0,1]`, `col_offset=0`) exécute les tests `File` (B613).
- `Context` : voir docs/spec/core.md §3 (`call_args` = `attr` pour un `Attribute`, littéral sinon ;
  `_get_literal_value` : bool/None → `"True"`/`"False"`/`"None"` ; `check_call_arg_value` → `None` si absent ;
  `is_module_imported_like` = sous-chaîne).
- Config : `@test.takes_config` nu → section = `__name__` ; `takes_config("x")` → `"x"` ; section absente →
  `gen_config(name)` du module.

## Partie 1 — Plugins (42 fonctions, 31 fichiers)

### B101 `assert_used` — `asserts.py` — checks `Assert`, config `assert_used` = `{"skips": []}`
```
for skip in config.get("skips", []):
    if fnmatch.fnmatch(context.filename, skip): return None
return Issue(LOW, HIGH, cwe=703, text="Use of assert detected. The enclosed code will be removed when compiling to optimised byte code.")
```

### B102 `exec_used` — `exec.py` — `Call`
`if context.call_function_name_qual == "exec": Issue(MEDIUM, HIGH, cwe=78, text="Use of exec detected.")`

### B103 `set_bad_file_permissions` — `general_bad_file_permissions.py` — `Call`
```
if "chmod" in context.call_function_name:            # sous-chaîne sur le nom court
  if context.call_args_count == 2:
    mode = context.get_call_arg_at_position(1)
    if mode is not None and isinstance(mode, int) and _stat_is_dangerous(mode):
        sev = HIGH if (mode & S_IWOTH) else MEDIUM
        filename = context.get_call_arg_at_position(0) or "NOT PARSED"
        return Issue(sev, HIGH, cwe=732, text="Chmod setting a permissive mask %s on file (%s)." % (oct(mode), filename))
_stat_is_dangerous(mode) = mode & S_IWOTH or mode & S_IWGRP or mode & S_IXGRP or mode & S_IXOTH
```
`S_IWOTH=0o002`, `S_IWGRP=0o020`, `S_IXGRP=0o010`, `S_IXOTH=0o001` ; `oct()` → `0o777`. (`True` est converti en
`"True"` par `_get_literal_value`, donc jamais un int ici.)

### B104 `hardcoded_bind_all_interfaces` — `general_bind_all_interfaces.py` — `Str`
`if context.string_val == "0.0.0.0": Issue(MEDIUM, MEDIUM, cwe=605, text="Possible binding to all interfaces.")`

### B105 / B106 / B107 — `general_hardcoded_password.py`
```python
RE_WORDS = "(pas+wo?r?d|pass(phrase)?|pwd|token|secrete?)"
RE_CANDIDATES = re.compile("(^{0}$|_{0}_|^{0}_|_{0}$)".format(RE_WORDS), re.IGNORECASE)   # utilisé avec .search()
_report(value, lineno=None) -> Issue(LOW, MEDIUM, cwe=259, text=f"Possible hardcoded password: '{value}'", lineno=lineno)
```
**B105 `hardcoded_password_string`** — `Str` ; `node = context.node` ; selon `node._bandit_parent` :
1. `ast.Assign` → pour chaque `targ` de `parent.targets` : `Name` et `RE_CANDIDATES.search(targ.id)` →
   `_report(node.value)` ; `Attribute` et `search(targ.attr)` → `_report(node.value)`.
2. `elif ast.Dict and node in parent.keys and RE_CANDIDATES.search(node.value)` → `pos = parent.keys.index(node)` ;
   `value_node = parent.values[pos]` ; si `Constant` → `_report(value_node.value)`.
3. `elif ast.Subscript and RE_CANDIDATES.search(node.value)` → `assign = parent._bandit_parent` ; si `Assign` et
   `assign.value` est un `Constant` str → `_report(assign.value.value)`.
4. `elif ast.Index ...` (legacy < 3.9, jamais atteint avec ruff).
5. `elif ast.Compare` → `comp = parent` ; `comp.left` `Name` avec `search(comp.left.id)` et `comp.comparators[0]`
   `Constant` str → `_report(comparators[0].value)` ; idem `Attribute` avec `comp.left.attr`.
Une seule issue ou `None`. (Branches 2–4 : regex sur la chaîne elle-même.)

**B106 `hardcoded_password_funcarg`** — `Call` :
```
for kw in context.node.keywords:
    if isinstance(kw.value, ast.Constant) and isinstance(kw.value.value, str) and RE_CANDIDATES.search(kw.arg):
        return _report(kw.value.value, lineno=kw.value.lineno)
```
(`kw.arg is None` pour `**kwargs` avec une valeur str constante → `TypeError` → aucune issue.)

**B107 `hardcoded_password_default`** — `FunctionDef` :
```
defs = [None] * (len(node.args.args) - len(node.args.defaults)); defs.extend(node.args.defaults)
for key, val in zip(node.args.args, defs):
    if isinstance(key, (ast.Name, ast.arg)):
        if val is None or (isinstance(val, ast.Constant) and val.value is None): continue
        if isinstance(val, ast.Constant) and isinstance(val.value, str) and RE_CANDIDATES.search(key.arg):
            return _report(val.value)
```
Seuls les `args` positionnels (pas kwonly/posonly) ; note : `node.args.defaults` couvre posonly + args, d'où un
décalage possible (reproduire littéralement, `[None] * n` négatif → liste vide).

### B108 `hardcoded_tmp_directory` — `general_hardcoded_tmp.py` — `Str`, config `hardcoded_tmp_directory` = `{"tmp_dirs": ["/tmp", "/var/tmp", "/dev/shm"]}`
```
tmp_dirs = config["tmp_dirs"] if (config is not None and "tmp_dirs" in config) else ["/tmp","/var/tmp","/dev/shm"]
if any(context.string_val.startswith(s) for s in tmp_dirs): Issue(MEDIUM, MEDIUM, cwe=377, text="Probable insecure usage of temp file/directory.")
```

### B110 `try_except_pass` — `ExceptHandler`, config `try_except_pass` = `{"check_typed_exception": False}`
```
node = context.node
if len(node.body) == 1:
    if (not config["check_typed_exception"]) and node.type is not None and getattr(node.type, "id", None) != "Exception": return
    if isinstance(node.body[0], ast.Pass): Issue(LOW, HIGH, cwe=703, text="Try, Except, Pass detected.")
```
### B112 `try_except_continue` — idem avec `ast.Continue`, texte `"Try, Except, Continue detected."`, config `try_except_continue`.

### B113 `request_without_timeout` — `Call`
```
HTTP_VERBS = {"get","options","head","post","put","patch","delete"}; HTTPX_ATTRS = {"request","stream","Client","AsyncClient"} | HTTP_VERBS
qualname = context.call_function_name_qual.split(".")[0]
if qualname == "requests" and name in HTTP_VERBS and context.check_call_arg_value("timeout") is None:
    Issue(MEDIUM, LOW, cwe=400, text=f"Call to {qualname} without timeout")
if ((qualname == "requests" and name in HTTP_VERBS) or (qualname == "httpx" and name in HTTPX_ATTRS)) and context.check_call_arg_value("timeout", "None"):
    Issue(MEDIUM, LOW, cwe=400, text=f"Call to {qualname} with timeout set to None")
```

### B201 `flask_debug_true` — `app_debug.py` — `Call`
```
if context.is_module_imported_like("flask") and context.call_function_name_qual.endswith(".run") and context.check_call_arg_value("debug", "True"):
    Issue(HIGH, MEDIUM, cwe=94, text="A Flask app appears to be run with debug=True, which exposes the Werkzeug debugger and allows the execution of arbitrary code.", lineno=context.get_lineno_for_call_arg("debug"))
```

### B202 `tarfile_unsafe_members` — `Call` (CWE 22)
```
if context.is_module_imported_exact("tarfile") and "extractall" in context.call_function_name:
    if "filter" in context.call_keywords and is_filter_data(context): return None      # kw filter == Constant "data"
    if "members" in context.call_keywords:
        members = get_members_value(context)   # Call → {"Function": arg.func.id} (AttributeError si func n'est pas un Name) ; Name → {"Other": arg.id} ; sinon {"Other": arg (nœud)}
        return LOW/LOW si "Function" in members else MEDIUM/MEDIUM
    return HIGH/HIGH
```
Textes (`{members}` = `str(dict)`, parenthèse parasite d'origine) :
- LOW : `"Usage of tarfile.extractall(members=function(tarfile)). Make sure your function properly discards dangerous members {members})."`
- MEDIUM : `"Found tarfile.extractall(members=?) but couldn't identify the type of members. Check if the members were properly validated {members})."`
- HIGH : `"tarfile.extractall used without any validation. Please check and discard dangerous members."`

### B501 `request_with_no_cert_validation` — `Call`
Mêmes ensembles que B113 ; `if ((qualname=="requests" and name in HTTP_VERBS) or (qualname=="httpx" and name in HTTPX_ATTRS)) and context.check_call_arg_value("verify", "False")` →
`Issue(HIGH, HIGH, cwe=295, text=f"Call to {qualname} with verify=False disabling SSL certificate checks, security issue.", lineno=context.get_lineno_for_call_arg("verify"))`.

### B502 / B503 / B504 — `insecure_ssl_tls.py`
Config `ssl_with_bad_version` = `{"bad_protocol_versions": ["PROTOCOL_SSLv2","SSLv2_METHOD","SSLv23_METHOD","PROTOCOL_SSLv3","PROTOCOL_TLSv1","SSLv3_METHOD","TLSv1_METHOD","PROTOCOL_TLSv1_1","TLSv1_1_METHOD"]}`.

**B502 `ssl_with_bad_version`** — `Call`, CWE 327 :
```
if qual == "ssl.wrap_socket":
    if check_call_arg_value("ssl_version", bad): HIGH/HIGH "ssl.wrap_socket call with insecure SSL/TLS protocol version identified, security issue." lineno=get_lineno_for_call_arg("ssl_version")
elif qual == "pyOpenSSL.SSL.Context":
    if check_call_arg_value("method", bad): HIGH/HIGH "SSL.Context call with insecure SSL/TLS protocol version identified, security issue." lineno=get_lineno_for_call_arg("method")
elif qual not in ("ssl.wrap_socket", "pyOpenSSL.SSL.Context"):
    if check_call_arg_value("method", bad) or check_call_arg_value("ssl_version", bad):
        lineno = get_lineno_for_call_arg("method") or get_lineno_for_call_arg("ssl_version")
        MEDIUM/MEDIUM "Function call with insecure SSL/TLS protocol identified, possible security issue."
```
**B503 `ssl_with_bad_defaults`** — `FunctionDef`, config `ssl_with_bad_version` :
`for default in context.function_def_defaults_qual: if default.split(".")[-1] in bad: Issue(MEDIUM, MEDIUM, cwe=327, text="Function definition identified with insecure SSL/TLS protocol version by default, possible security issue.")`

**B504 `ssl_with_no_version`** — `Call`, sans config :
`if qual == "ssl.wrap_socket" and context.check_call_arg_value("ssl_version") is None: Issue(LOW, MEDIUM, cwe=327, text="ssl.wrap_socket call with no SSL/TLS protocol version specified, the default SSLv23 could be insecure, possible security issue.", lineno=get_lineno_for_call_arg("ssl_version"))`

### B505 `weak_cryptographic_key` — `Call`, config `weak_cryptographic_key` (CWE 326, confiance HIGH)
Défauts : `weak_key_size_dsa_high 1024, dsa_medium 2048, rsa_high 1024, rsa_medium 2048, ec_high 160, ec_medium 224`.
```
_classify_key_size(config, key_type, key_size):
    if isinstance(key_size, str): return None
    for size, level in [(cfg[f"weak_key_size_{key_type.lower()}_high"], HIGH), (cfg[..._medium], MEDIUM)]:
        if key_size < size: return Issue(level, HIGH, cwe=326, text="%s key sizes below %d bits are considered breakable. " % (key_type, size))   # espace final
weak_cryptographic_key = _cryptography_io(...) or _pycrypto(...)
```
cryptography.io : `cryptography.hazmat.primitives.asymmetric.{dsa,rsa,ec}.generate_private_key` → DSA/RSA/EC ;
positions `{"DSA": 0, "RSA": 1, "EC": 0}` ; DSA/RSA : `key_size = get_call_arg_value("key_size") or get_call_arg_at_position(pos) or 2048` ;
EC : `curve = get_call_arg_value("curve") or (len(call_args) > 0 and call_args[0])` ; `key_size = curve_key_sizes[curve] if curve in curve_key_sizes else 224` avec
`{"SECT571K1":571,"SECT571R1":570,"SECP521R1":521,"BrainpoolP512R1":512,"SECT409K1":409,"SECT409R1":409,"BrainpoolP384R1":384,"SECP384R1":384,"SECT283K1":283,"SECT283R1":283,"BrainpoolP256R1":256,"SECP256K1":256,"SECP256R1":256,"SECT233K1":233,"SECT233R1":233,"SECP224R1":224,"SECP192R1":192,"SECT163K1":163,"SECT163R2":163}`.
pycrypto : `Crypto.PublicKey.{DSA,RSA}.generate`, `Cryptodome.PublicKey.{DSA,RSA}.generate` ; `key_size = get_call_arg_value("bits") or get_call_arg_at_position(0) or 2048`.

### B506 `yaml_load` — `Call`
```
imported = context.is_module_imported_exact("yaml"); qualname = context.call_function_name_qual
if not imported and isinstance(qualname, str): return
parts = qualname.split("."); func = parts[-1]
if all(["yaml" in parts, func == "load", not check_call_arg_value("Loader","SafeLoader"), not check_call_arg_value("Loader","CSafeLoader"),
        not get_call_arg_at_position(1) == "SafeLoader", not get_call_arg_at_position(1) == "CSafeLoader"]):
    Issue(MEDIUM, HIGH, cwe=20, text="Use of unsafe yaml load. Allows instantiation of arbitrary objects. Consider yaml.safe_load().", lineno=context.node.lineno)
```

### B507 `ssh_no_host_key_verification` — `Call`
`if is_module_imported_like("paramiko") and call_function_name == "set_missing_host_key_policy" and node.args:` ; `a = args[0]` ; `val = a.attr | a.id | a.func.attr | a.func.id | None` ; `if val in ["AutoAddPolicy", "WarningPolicy"]: Issue(HIGH, MEDIUM, cwe=295, text="Paramiko call with policy set to automatically trust the unknown host key.", lineno=get_lineno_for_call_arg("set_missing_host_key_policy"))` (→ lineno du nœud).

### B508 `snmp_insecure_version_check` / B509 `snmp_crypto_check` — `Call`, CWE 319, MEDIUM/HIGH
- B508 : `qual == "pysnmp.hlapi.CommunityData"` et (`check_call_arg_value("mpModel", 0)` ou `..., 1`) → `"The use of SNMPv1 and SNMPv2 is insecure. You should use SNMPv3 if able."`, `lineno=get_lineno_for_call_arg("CommunityData")`.
- B509 : `qual == "pysnmp.hlapi.UsmUserData" and call_args_count < 3` → `"You should not use SNMPv3 without encryption. noAuthNoPriv & authNoPriv is insecure"`.

### B601 `paramiko_calls` — `Call`
`if is_module_imported_like("paramiko") and call_function_name in ["exec_command"]: Issue(MEDIUM, MEDIUM, cwe=78, text="Possible shell injection via Paramiko call, check inputs are properly sanitized.")`

### B602–B607 — `injection_shell.py`, config `shell_injection` (CWE 78)
Défauts : voir `src/core/plugin_config.rs` (`subprocess`, `shell`, `no_shell`). `full_path_match = re.compile(r"^(?:[A-Za-z](?=\:)|[\\\/\.])")`.
```
_evaluate_shell_call(context): LOW si node.args[0] est un Constant str, sinon HIGH   # pas de niveau MEDIUM
has_shell(context): si "shell" in call_keywords: pour le keyword shell : Constant numérique (int/float/complex/bool) → bool(val) ; List → bool(elts) ; Dict → bool(keys) ;
                    Name in ["False","None"] → False ; autre Constant → val.value (ex. "" falsy) ; sinon True
```
- **B602** `subprocess_popen_with_shell_equals_true` : `if config and qual in config["subprocess"] and has_shell and len(call_args) > 0` → LOW/HIGH `"subprocess call with shell=True seems safe, but may be changed in the future, consider rewriting without shell"` ou HIGH/HIGH `"subprocess call with shell=True identified, security issue."` ; `lineno=get_lineno_for_call_arg("shell")`.
- **B603** `subprocess_without_shell_equals_true` : `if config and qual in config["subprocess"] and not has_shell` → LOW/HIGH `"subprocess call - check for execution of untrusted input."`, `lineno=get_lineno_for_call_arg("shell")`.
- **B604** `any_other_function_with_shell_equals_true` : `if config and qual not in config["subprocess"] and has_shell` → MEDIUM/LOW `"Function call with shell=True parameter identified, possible security issue."`, `lineno=get_lineno_for_call_arg("shell")`.
- **B605** `start_process_with_a_shell` : `if config and qual in config["shell"] and len(call_args) > 0` → LOW/HIGH `"Starting a process with a shell: Seems safe, but may be changed in the future, consider rewriting without shell"` ou HIGH/HIGH `"Starting a process with a shell, possible injection detected, security issue."`.
- **B606** `start_process_with_no_shell` : `if config and qual in config["no_shell"]` → LOW/MEDIUM `"Starting a process without a shell."`.
- **B607** `start_process_with_partial_path` : `if config and len(call_args)` et `qual` dans subprocess/shell/no_shell : `node = args[0]` ; si `List` non vide → `elts[0]` ; si `Constant` str et `not full_path_match.match(value)` → LOW/HIGH `"Starting a process with a partial executable path"`.

### B608 `hardcoded_sql_expressions` — `injection_sql.py` — `Str`
```python
SIMPLE_SQL_RE = re.compile(r"(select\s.*from\s|delete\s+from\s|insert\s+into\s.*values[\s(]|update\s.*set\s)", re.IGNORECASE | re.DOTALL)
def _evaluate_ast(node):
    wrapper=None; statement=""; str_replace=False
    if parent est BinOp: out = utils.concat_string(node, node._bandit_parent); wrapper = out[0]._bandit_parent; statement = out[1]
    elif parent est Attribute et parent.attr in ("format","replace"): statement = node.value; wrapper = parent._bandit_parent._bandit_parent; str_replace = (attr == "replace")
    elif parent est JoinedStr: substrings = constantes str du JoinedStr ; si node == substrings[0]: statement = "".join(valeurs) ; wrapper = parent._bandit_parent
    if isinstance(wrapper, ast.Call): return (get_called_name(wrapper) in ["execute","executemany"], statement, str_replace)
    return (False, statement, str_replace)
execute_call, statement, str_replace = _evaluate_ast(context.node)
if SIMPLE_SQL_RE.search(statement): Issue(MEDIUM, MEDIUM if (execute_call and not str_replace) else LOW, cwe=89, text="Possible SQL injection vector through string-based query construction.")
```

### B609 `linux_commands_wildcard_injection` — `Call`, config `shell_injection`
```
if not ("shell" in config and "subprocess" in config): return
if qual in config["shell"] or (qual in config["subprocess"] and check_call_arg_value("shell", "True")):
   if call_args_count >= 1:
      arg = get_call_arg_at_position(0); argument_string = "".join(f" {li}" for li in arg) si list, arg si str, "" sinon
      if argument_string != "": for f in ["chown","chmod","tar","rsync"]: if f in argument_string and "*" in argument_string:
          Issue(HIGH, MEDIUM, cwe=155, text="Possible wildcard injection in call: %s" % qual, lineno=get_lineno_for_call_arg("shell"))
```

### B610 `django_extra_used` / B611 `django_rawsql_used` — `django_sql_injection.py` — `Call`, CWE 89, MEDIUM/MEDIUM
- B610 : `if call_function_name == "extra"` : `kwargs = {kw.arg: kw.value}` + positionnels `select, where, params, tables, order_by, select_params` ; `insecure` si `where`/`tables` n'est pas une `List` de `Constant` str, ou si `select` n'est pas un `Dict` dont toutes les clés puis toutes les valeurs sont des `Constant` str → `"Use of extra potential SQL attack vector."`.
- B611 : `if is_module_imported_like("django.db.models") and call_function_name == "RawSQL"` : `sql = args[0] if args else kwargs["sql"]` (KeyError sinon) ; si pas un `Constant` str → `"Use of RawSQL potential SQL attack vector."`.

### B612 `logging_config_insecure_listen` — `Call`
`if qual == "logging.config.listen" and "verify" not in call_keywords: Issue(MEDIUM, HIGH, cwe=94, text="Use of insecure logging.config.listen detected.")`

### B613 `trojansource` — **`File`**
```
BIDI_CHARACTERS = ("‪","‫","‬","‭","‮","⁦","⁧","⁨","⁩","‏")
for lineno, line in enumerate(decoded_source.splitlines(), start=1):   # str.splitlines
    for char in BIDI_CHARACTERS:
        try: col_offset = line.index(char) + 1     # index en caractères
        except ValueError: continue
        text = "A Python source file contains bidirectional control characters (%r)." % char   # ex. '‮'
        issue = Issue(HIGH, MEDIUM, cwe=838, text=text, lineno=lineno, col_offset=col_offset); issue.linerange = [lineno]; return issue
```

### B614 `pytorch_load` — `Call`
`if not is_module_imported_exact("torch") and isinstance(qual, str): return` ; `if qual in {"torch.load", "torch.serialization.load"}` : `w = get_call_arg_value("weights_only")` ; `if w == "True" or w is True: return` ; `Issue(MEDIUM, HIGH, cwe=502, text="Use of unsafe PyTorch load", lineno=get_lineno_for_call_arg("load"))`.

### B615 `huggingface_unsafe_download` — `Call`
```
if not any(is_module_imported_like(m) for m in ["transformers","datasets","huggingface_hub"]): return
parts = qual.split("."); func = parts[-1]
unsafe = {"from_pretrained":["transformers"], "load_dataset":["datasets"], "hf_hub_download":["huggingface_hub"], "snapshot_download":["huggingface_hub"], "repository_id":["huggingface_hub"]}
if func not in unsafe or not any(m in parts for m in unsafe[func]): return
for kw in node.keywords: if kw.arg in ("revision","commit_id") and not isinstance(kw.value, ast.Constant): return
rev = get_call_arg_value("revision") or get_call_arg_value("commit_id")
if rev is not None and isinstance(rev, str): s = str(rev).strip("\"'"); if len(s) >= 7 and all(c in hexdigits for c in s): return
first = get_call_arg_at_position(0); if first and isinstance(first, str) and first.startswith(("./","/","../")): return
Issue(MEDIUM, HIGH, cwe=494, text=f"Unsafe Hugging Face Hub download without revision pinning in {func}()", lineno=get_lineno_for_call_arg(func))
```

### B701 `jinja2_autoescape_false` — `Call`, CWE 94
`parts = qual.split(".")` ; `if "jinja2" in parts and parts[-1] == "Environment"` : parcours `ast.walk` (BFS) du nœud d'appel ; premier `keyword` `autoescape` : valeur `False` (`id == "False"` ou `value is False`) → HIGH/HIGH `"Using jinja2 templates with autoescape=False is dangerous and can lead to XSS. Use autoescape=True or use the select_autoescape function to mitigate XSS vulnerabilities."` ; valeur `True` ou appel `select_autoescape` → `None` ; autre → HIGH/MEDIUM `"Using jinja2 templates with autoescape=False is dangerous and can lead to XSS. Ensure autoescape=True or use the select_autoescape function to mitigate XSS vulnerabilities."` ; aucun keyword `autoescape` → HIGH/HIGH `"By default, jinja2 sets autoescape to False. Consider using autoescape=True or use the select_autoescape function to mitigate XSS vulnerabilities."`.

### B702 `use_of_mako_templates` — `Call`
`if "mako" in parts and parts[-1] == "Template"` → MEDIUM/HIGH cwe=80 `"Mako templates allow HTML/JS rendering by default and are inherently open to XSS attacks. Ensure variables in all templates are properly sanitized via the 'n', 'h' or 'x' flags (depending on context). For example, to HTML escape the variable 'data' do ${ data |h }."`

### B703 `django_mark_safe` — `django_xss.py` — `Call`
`if is_module_imported_like("django.utils.safestring") and call_function_name in ["mark_safe","SafeText","SafeUnicode","SafeString","SafeBytes"]` : `xss = node.args[0]` (IndexError sans argument) ; si pas un `Constant` str → `check_risk(node)` :
- `Name` → remonter les parents jusqu'à `Module`/`FunctionDef` ; si `FunctionDef` et le nom est un paramètre (`parent.args.args`) → `is_param` (reste insecure) ; sinon `secure = evaluate_var(xss, parent, node.lineno)`.
- `Call` → `secure = evaluate_call(xss, parent)`.
- `BinOp` avec `op` `Mod` et `left` `Constant` str → `secure = evaluate_call(transform2call(xss), parent)` (`"fmt" % x` → `Call(func=Attribute(left, "format"), args=elts du tuple ou [right])`).
- `if not secure: Issue(MEDIUM, HIGH, cwe=80, text="Potential XSS on mark_safe function.")`.
`evaluate_call(call, parent)` : uniquement `Call` dont `func` est `Attribute(Constant str, "format")` sans keywords ; chaque argument : `Constant` str → sûr ; `Name` → `evaluate_var` ; `Call` → `evaluate_call` ; `Starred` de List/Tuple → ses éléments ajoutés ; sinon insecure ; `secure = num_secure == len(args)`.
`evaluate_var(name, parent, until)` : `False` si paramètre de la fonction ; sinon `DeepAssignation(name).is_assigned(node)` sur `parent.body` (nœuds de `lineno < until`) : affectation à `Constant` str → sûr ; `Name` → récursif ; `Call` → `evaluate_call` ; liste/tuple → tous sûrs ; sinon insecure (arrêt).
`DeepAssignation.is_assigned` : `Expr` → `.value` ; `FunctionDef` → `False` si le nom est un paramètre sinon corps ; `With` → `optional_vars.id == name` → le `With` ; corps sinon ; `Try` → body+handlers+orelse+finalbody ; `ExceptHandler` → body ; `If`/`For`/`While` → body+orelse ; `AugAssign` → `value` si cible `Name` ; `Assign` → `targets[0]` `Name` → `value` ; `Tuple` cible avec `Tuple` valeur → élément positionnel.

### B704 `markupsafe_markup_xss` — `Call`, config `markupsafe_xss` = `{"extend_markup_names": [], "allowed_calls": []}`
```
if qual not in ("markupsafe.Markup","flask.Markup") and qual not in config.get("extend_markup_names", []): return
args = node.args; if not args or isinstance(args[0], ast.Constant): return
allowed = config.get("allowed_calls", []); if allowed and isinstance(args[0], ast.Call) and get_call_name(args[0], import_aliases) in allowed: return
Issue(MEDIUM, HIGH, cwe=79, text=f"Potential XSS with ``{qual}`` detected. Do not use ``{context.call_function_name}`` on untrusted data.")
```

### B324 `hashlib` — `hashlib_insecure_functions.py` — `Call` (nom de fonction **`hashlib`**, entry point `hashlib_insecure_functions`)
```
WEAK_HASHES = ("md4","md5","sha","sha1"); WEAK_CRYPT_HASHES = ("METHOD_CRYPT","METHOD_MD5","METHOD_BLOWFISH")
parts = qual.split("."); func = parts[-1]
if "hashlib" in parts: _hashlib_func(func) elif "crypt" in parts and func in ("crypt","mksalt"): _crypt_crypt(func)
_hashlib_func: keywords = call_keywords
   func in WEAK_HASHES and keywords.get("usedforsecurity","True") == "True" → HIGH/HIGH cwe=327 f"Use of weak {func.upper()} hash for security. Consider usedforsecurity=False" lineno=node.lineno
   func == "new": name = call_args[0] if call_args else keywords.get("name") ; str et name.lower() in WEAK_HASHES et usedforsecurity "True" → même issue avec name.upper()
_crypt_crypt: "crypt" → name = args[1] if len(args) > 1 else keywords.get("salt") ; "mksalt" → args[0] if args else keywords.get("method") ; str in WEAK_CRYPT_HASHES → MEDIUM/HIGH cwe=327 f"Use of insecure crypt.{name.upper()} hash function." lineno=node.lineno
```

## Partie 2 — Blacklists

Données verbatim dans `src/core/blacklist.rs` (`BLACKLIST_CALLS` B301–B323, `BLACKLIST_IMPORTS` B401–B415).
Règles d'appariement : documentation de module de `src/core/blacklist.rs`.

Points d'attention :
1. B602/B605 n'ont que deux niveaux (LOW/HIGH), pas de `_has_special_characters`.
2. `context.statement` n'existe pas.
3. Les tests `Str` ignorent les docstrings (parent `ast.Expr`) et prennent la `linerange` du parent.
4. `get_lineno_for_call_arg(x)` avec un nom non-keyword (B507/B508/B509/B614/B615) → `None` → lineno du nœud.
5. `True`/`False`/`None` littéraux → chaînes `"True"`/`"False"`/`"None"`.
6. `is_module_imported_like` = sous-chaîne ; `is_module_imported_exact` = appartenance.
7. Blacklist appels = égalité exacte ; imports = `startswith`.
8. `message.replace("{name}", name)` (toutes les occurrences).
9. `visit_Import`/`visit_ImportFrom` : une exécution par instruction → un seul alias signalé.
10. `random.randrange` apparaît deux fois dans B311 (inoffensif).
11. Tests `File` (B613) : `lineno=0`, `linerange=[0,1]`, `col_offset=0`.
