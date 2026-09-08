# WP-07 — `bandit.core.utils` : qualname de module, noms d'appel, linerange, ini

**Agent** : `banditrs-wp-medium` · **Vague** A · **Fichier de test** : `tests/unit_core_util.rs` · **Stubs** : 26 (+4 non portables)
**Python** : `tests/unit/core/test_util.py`, `bandit/core/utils.py` · **Rust** : `src/core/utils.rs`, `src/pycompat/configparser.rs`, `src/ast/{qualname,linerange,vnode}.rs` (lecture)

## Objectif

Port mécanique de 26 tests. Les fonctions existent déjà (`utils.rs`, `ast::qualname`, `ast::linerange`,
`pycompat::configparser`, `NodeKind::parse`) ; le travail est surtout la fixture d'arborescence avec
liens symboliques et les chemins relatifs.

## Fichiers

- Possédés : `tests/unit_core_util.rs`, `src/core/utils.rs`, `src/pycompat/configparser.rs`.
- Lus : `src/ast/qualname.rs` (`call_name`, `attr_qual_name`, type `Aliases`), `src/ast/linerange.rs`,
  `src/ast/vnode.rs` (`NodeKind::parse`), `src/source/parse.rs` (`parse_module`).

## Fixture `_setup_get_module_qualname_from_path` (dans le fichier de test)

Créer sous un `TempDir` :
```
good/__init__.py good/a/__init__.py good/a/b/__init__.py good/a/b/c/__init__.py good/a/b/c/test_typical.py
missingmid/__init__.py (pas de missingmid/a/__init__.py) missingmid/a/b/__init__.py missingmid/a/b/c/__init__.py missingmid/a/b/c/test_missingmid.py
missingend/__init__.py missingend/a/b/__init__.py (pas de missingend/a/b/c/__init__.py) missingend/a/b/c/test_missingend.py
syms/__init__.py syms/a/__init__.py  +  symlink syms/a/bsym → good/a/b   (std::os::unix::fs::symlink)
```
`reltempdir` = chemin relatif du `TempDir` depuis `std::env::current_dir()` (helper local `relpath(from, to)`
à écrire : composants `..` puis suffixe — `os.path.relpath`). Les tests ne changent **jamais** de cwd.

## Tests

| Test | Appel | Attendu |
|---|---|---|
| `test_get_module_qualname_from_path_abs_typical` | `<tmp>/good/a/b/c/test_typical.py` | `"good.a.b.c.test_typical"` |
| `test_get_module_qualname_from_path_with_dot` | `"./__init__.py"` (le fichier n'a pas besoin d'exister — vérifier `utils.py`) | `"__init__"` |
| `test_get_module_qualname_from_path_abs_missingmid` | `<tmp>/missingmid/a/b/c/test_missingmid.py` | `"b.c.test_missingmid"` |
| `test_get_module_qualname_from_path_abs_missingend` | `<tmp>/missingend/a/b/c/test_missingend.py` | `"test_missingend"` |
| `test_get_module_qualname_from_path_abs_syms` | `<tmp>/syms/a/bsym/c/test_typical.py` | `"syms.a.bsym.c.test_typical"` |
| `..._rel_typical` / `..._rel_missingmid` / `..._rel_missingend` / `..._rel_syms` | mêmes chemins via `reltempdir` | mêmes résultats |
| `test_get_module_qualname_from_path_sys` | **adapté** : `/usr/lib/python3.11/os.py` (si absent : chercher `os.py` via `python3 -c "import os;print(os.__file__)"`) | `"os"` |
| `test_get_module_qualname_from_path_invalid_path` | `"/a/b/c/d/e.py"` | `"e"` |
| `test_get_module_qualname_from_path_dir` | `"/tmp/"` | `Err(InvalidModulePath)` |
| `test_namespace_path_join` | `("base1.base2", "name")` | `"base1.base2.name"` |
| `test_namespace_path_split` | `"base1.base2.name"` | `("base1.base2", Some("name"))` |
| `test_get_call_name1` | parser `a.b.c.d(x,y)` ; `call_name(call, &Aliases::default())` | `"a.b.c.d"` |
| `test_get_call_name2` | alias `{"a": "alias.x.y"}` ; `{"a.b": "alias.x.y"}` ; `{"a.b.c.d": "alias.x.y"}` | `"alias.x.y.b.c.d"` ; `"alias.x.y.c.d"` ; `"alias.x.y"` |
| `test_get_call_name3` | `a.list[0](x,y)` ; `attr_qual_name(func, &aliases)` | `""` |
| `test_linerange` | parser `examples/jinja2_templating.py` ; `linerange` de `body[8]` | `[11, 12, 13]` (3 lignes) |
| `test_escaped_representation_simple` | `escaped_bytes_representation(b"ascii")` | `b"ascii"` |
| `test_escaped_representation_valid_not_printable` | `b"\\u0000"` (7 octets : backslash, u, 0, 0, 0, 0) | `b"\\x00"` |
| `test_escaped_representation_invalid` | `b"\\uffff"` | `b"\\uffff"` (inchangé) |
| `test_escaped_representation_mixed` | `b"ascii\\u0000\\uffff"` | `b"ascii\\x00\\uffff"` |
| `test_parse_ini_file` | fichier `[bandit]\nexclude=/abc,/def` puis `[Blabla]\nsomething=something` | `Some({"exclude": "/abc,/def"})` puis `None` |
| `test_check_ast_node_good` | **adapté** : `NodeKind::parse("Call")` | `Some(NodeKind::Call)` |
| `test_check_ast_node_bad_node` | `NodeKind::parse("Derp")` | `None` |
| `test_check_ast_node_bad_type` | `NodeKind::parse("walk")` | `None` |

Non portables (documentés) : `test_path_for_function*` (3) et `test_deepgetattr` (introspection Python).

## Critères d'acceptation

- `cargo test --test unit_core_util` : 26 tests verts, parallèles.
- Porte de qualité ; inventaire mis à jour.

## Pièges

- `get_module_qualname_from_path` Python résout les symlinks ? Non : il remonte les répertoires tant qu'un
  `__init__.py` existe (`os.path.isfile`), sans `realpath` — d'où `syms.a.bsym.c.test_typical`. Vérifier
  que le port Rust ne canonicalise pas.
- `linerange` Python retourne `range(start, end+1)` → comparer `LineRange::to_vec()`.
- `escaped_bytes_representation` prend et rend des **octets** (`\\u0000` est la séquence littérale de 6
  caractères, pas le caractère U+0000).
