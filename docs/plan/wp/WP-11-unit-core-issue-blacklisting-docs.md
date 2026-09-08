# WP-11 — `Issue`/`Cwe`, `blacklisting.report_issue`, `docs_utils.get_url`

**Agent** : `banditrs-wp-low` · **Vague** A · **Fichiers de test** : `tests/unit_core_issue.rs` (7), `tests/unit_core_blacklisting.rs` (2), `tests/unit_core_docs_util.rs` (3) · **Stubs** : 12 (+2 non portables `meta_ast`)
**Python** : `tests/unit/core/test_{issue,blacklisting,docs_util,meta_ast}.py` · **Rust** : `src/core/issue.rs`, `src/core/docs_utils.rs`, `src/core/metrics.rs`

## Objectif

Ports directs : les API existent (`Issue::new`, `as_dict`, `filter`, `same_signature`, `get_code`,
`BlacklistEntry::report_issue`, `docs_utils::get_url`). Des tests unitaires équivalents existent déjà
dans `src/core/issue.rs` ; les tests miroirs les **doublent** avec les noms Python (traçabilité de
l'inventaire) — ne pas supprimer les tests `src/`.

## Fichiers

- Possédés : les trois fichiers de test, `src/core/issue.rs`, `src/core/docs_utils.rs`, `src/core/metrics.rs`.
- Lus : `src/core/blacklist.rs` (`BlacklistEntry`, champs `pub` — ne pas éditer, WP-10).

## Fixture `_get_issue_instance`

`Issue::new(sev, conf, Cwe::MULTIPLE_BINDS, "Test issue", "code.py", "bandit_plugin", "B999", 1)` puis
`col_offset = 8`, `end_col_offset = 16` (`sev`/`conf` MEDIUM par défaut).

## Tests

| Test | Attendu |
|---|---|
| `test_issue_create` | construction OK (type `Issue`) |
| `test_issue_str` | `issue.to_string() == "Issue: 'Test issue' from B999:bandit_plugin: CWE: CWE-605 (https://cwe.mitre.org/data/definitions/605.html), Severity: MEDIUM Confidence: MEDIUM at code.py:1:8"` |
| `test_issue_as_dict` | `as_dict(false, -1)` : `filename == "code.py"`, `test_name == "bandit_plugin"`, `test_id == "B999"`, `issue_severity == "MEDIUM"`, `issue_cwe == {"id": 605, "link": "https://cwe.mitre.org/data/definitions/605.html"}`, `issue_confidence == "MEDIUM"`, `issue_text == "Test issue"`, `line_number == 1`, `line_range == []` (linerange vide), `col_offset == 8`, `end_col_offset == 16`, pas de clé `code` |
| `test_issue_filter_severity` | issues (LOW, MEDIUM, HIGH) × confiance HIGH ; pour chaque niveau `l` et issue `i` : `i.filter(l, Rank::Undefined) == (rank(i.severity) >= rank(l))` |
| `test_issue_filter_confidence` | symétrique avec `filter(Rank::Undefined, l)` |
| `test_matches_issue` | `same_signature` : a == a, a == f (clone) ; ≠ b (HIGH), ≠ c (conf LOW), ≠ d (`text = "ABCD"`), ≠ e (`fname = "file1.py"`), ≠ g (`test = "ZZZZ"`) ; == h (`lineno = 12345`, ignoré) |
| `test_get_code` | **adapté** : `SourceFile::new("code.py", "\u{8}0\n")` (octets de contrôle) attaché à l'issue (`source`), `get_code(-1, false)` ne panique pas et contient `"0"` |
| `test_report_issue` (blacklisting) | `BlacklistEntry { name: "x", id: "B000", cwe: Cwe::NOTSET, qualnames: [], message: "test {name}", level: "HIGH" }.report_issue("name")` → draft : `test_id == "B000"`, sévérité `High`, `cwe.as_dict() == {}`, confiance `High`, texte `"test name"` |
| `test_report_issue_defaults` | entrée sans id/level explicites — telle que la produirait la conversion legacy : `id: "LEGACY"`, `level: ""` (chaîne vide → `Rank::parse` échoue → MEDIUM, comme `data.get("level", "MEDIUM")`) → `test_id == "LEGACY"`, sévérité `Medium`, cwe `{}`, confiance `High`, texte `"test name"` — préciser l'adaptation dans le doc-commentaire ; si WP-08 introduit un constructeur pour les entrées legacy, l'utiliser |
| `test_overwrite_bib_info` | `get_url("B304") == get_url("B305") == base_url() + "blacklists/blacklist_calls.html#b304-b305-ciphers-and-modes"` |
| `test_plugin_call_bib` | `get_url("B101") == base_url() + "plugins/b101_assert_used.html"` |
| `test_import_call_bib` | `get_url("B413") == base_url() + "blacklists/blacklist_imports.html#b413-import-pycrypto"` |

`base_url()` = `https://bandit.readthedocs.io/en/latest/` (`DOCS_VERSION`, cf. `PLAN.md` §1).
`test_meta_ast.py` (2 tests) : `BanditMetaAst` est un outil de debug non porté — inventaire + `DEVIATIONS.md` #8.

## Critères d'acceptation

- `cargo test --test unit_core_issue --test unit_core_blacklisting --test unit_core_docs_util` : 12 verts.
- Porte de qualité ; inventaire mis à jour.

## Pièges

- `Cwe::NOTSET` s'affiche `{}` dans `as_dict` mais `CWE-0`? — vérifier `issue.py` (`Cwe.__str__`) et le
  test `report_issue_matches_python` de `blacklist.rs` avant d'asserter sur l'affichage.
- `as_dict` Python n'inclut `code` que si `with_code=True`.
