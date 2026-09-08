# WP-06 — `BanditManager` : découverte de fichiers, baseline, sorties

**Agent** : `banditrs-wp-high` · **Vague** A · **Fichier de test** : `tests/unit_core_manager.rs` · **Stubs** : 20 (+1 non portable)
**Python** : `tests/unit/core/test_manager.py`, `bandit/core/manager.py` · **Rust** : `src/core/manager.rs`, `src/core/discover.rs`

## Objectif

Porter les 20 tests du manager. Les tests Python mockent `os.walk`, `os.path.isdir`,
`_get_files_from_dir`, `_is_file_included` ; on les remplace par de vrais répertoires temporaires
(`tempfile::TempDir`) et des chemins **absolus** (pas de `set_current_dir`, les tests sont parallèles).

## Fichiers

- Possédés : `tests/unit_core_manager.rs`, `src/core/manager.rs`, `src/core/discover.rs`.
- Lus : `docs/spec/core.md` §1, `src/core/issue.rs` (`BaselineIssue::from_dict` accepte `line_number: "n"`
  tel quel — les champs sont des `serde_json::Value`), `src/formatters/mod.rs` (`output_results` : un
  format inconnu retombe sur le format par défaut).

## Fixture (`_get_issue_instance`)

`Issue::new(sev, conf, Cwe::MULTIPLE_BINDS, "Test issue", "code.py", "bandit_plugin", <test_id>, 1)`
(`sev`/`conf` = MEDIUM par défaut). `Manager` : `Manager::new(BanditConfig::default(), AggType::File, TestSet::new(...))`.

## Tests

| Test | Mise en place / adaptation | Attendu |
|---|---|---|
| `test_create_manager` | manager par défaut | `debug == false`, `verbose == false`, `agg_type == AggType::File` (dériver `PartialEq` si absent) |
| `test_create_manager_with_profile` | `Profile { include: {"any_other_function_with_shell_equals_true", "assert_used"} }` (noms, comme en Python) | mêmes assertions |
| `test_matches_globlist` | `discover::matches_glob_list("test", &["*tes*"])` / `["*fes*"]` | `true` / `false` |
| `test_is_file_included` | 6 cas `(path, globs, excluded, enforce_glob)` : a `("a.py",["*.py"],[],true)`, b `("a.dd",["*.py"],[],false)`, c `("a.py",["*.py"],["a.py"],true)`, d `("a.dd",["*.py"],[],true)`, e `("x_a.py",["*.py"],["x_*.py"],true)`, f `("x.py",["*.py"],["x_*.py"],true)` | `true, true, false, false, false, true` |
| `test_get_files_from_dir` | **adaptation** : `<tmp>/a/{a.py,b.py,c.ww}` réels ; `get_files_from_dir(<tmp>, ["*.py"], None)` | `files == [<tmp>/a/a.py, <tmp>/a/b.py]`, `excluded == [<tmp>/a/c.ww]` (triés) |
| `test_populate_baseline_success` | JSON du test Python (avec `"line_number": "n"`, `"line_range": "n-m"`, cwe 605) | `mgr.baseline == vec![BaselineIssue::from_dict(&dict).unwrap()]` |
| `test_populate_baseline_invalid_json` | `{"data": "bad"}` sous `log::with_buffer` | `baseline` vide **et** une entrée de niveau `Warning` capturée |
| `test_results_count` | `results` = issues LOW/LOW, MEDIUM/MEDIUM, HIGH/HIGH | `[results_count(L,L), (M,M), (H,H)] == [3, 2, 1]` |
| `test_output_results_invalid_format` | `output_results(&mgr, 5, Low, Low, &mut Output::file(<tmp>/_temp_output), "invalid", None)` | `Ok` et le fichier existe (retombée sur le format par défaut) |
| `test_output_results_valid_format` | idem avec `"txt"` et `_temp_output.txt` | fichier existe |
| `test_discover_files_recurse_skip` | **adaptation** : cible = répertoire réel ; `discover_files(&[dir], false, None)` | `files_list == []`, `excluded_files == []` |
| `test_discover_files_recurse_files` | **adaptation** : `<tmp>/files.py` + `<tmp>/excluded.ww` ; récursif | `files_list == [<tmp>/files.py]`, `excluded_files == [<tmp>/excluded.ww]` |
| `test_discover_files_exclude` | **adaptation** : cible fichier `<tmp>/thing.py`, `excluded_paths = Some("<tmp>/thing.py")` | `files_list == []`, `excluded_files == ["<tmp>/thing.py"]` (chemin tel que fourni) |
| `test_discover_files_exclude_dir` | **adaptation** : arborescence `<tmp>/x/y.py` et `<tmp>/x/y/z.py` ; 4 appels : `(["<tmp>/x/y.py"], "<tmp>/x/*")`, `(…, "<tmp>/x/")`, `(…, "<tmp>/x")`, `(["<tmp>/x/y/z.py"], "y")` (sous-chaîne) | à chaque fois `files_list == []` et `excluded_files == [la cible]` |
| `test_discover_files_exclude_cmdline` | cibles `["a", "b", "c"]` (inexistantes), `excluded_paths = "a,b"` | `excluded_files == ["a", "b"]`, `files_list == ["./c"]` (adaptation de `assert_called_with(..., enforce_glob=False)`) |
| `test_discover_files_exclude_glob` | cibles `["a.py", "test_a.py", "test.py"]` (inexistantes), `"test_*.py"` | `files_list == ["./a.py", "./test.py"]`, `excluded_files == ["test_a.py"]` |
| `test_discover_files_include` | cible `["thing"]` (inexistante, pas de glob imposé) | `files_list == ["./thing"]`, `excluded_files == []` |
| `test_run_tests_ioerror` | `files_list = [<tmp>/no_such_file.py]` ; `run_tests()` | `skipped` contient une entrée dont le nom est ce chemin |
| `test_compare_baseline` | a (`file1.py`), b (`file2.py`), c (`file1.py`, HIGH) | `compare_baseline_results(&[a,b], &[a,b,c]) == [c]` ; `(&[a,b,c], &[a,b,c]) == []` ; `(&[a,b,c], &[a,b]) == []` |
| `test_find_candidate_matches` | a, b identiques ; c (`file1.py`) | `{a: [a,b]}` pour `([a],[a,b])` ; `{a: [a]}` pour `([a],[a,c])` ; `{a: []}` pour `([a],[c])` ; `{a: [a,b], b: [a,b]}` pour `([a,b],[a,b,c])` (type de retour : voir `manager.rs::baseline_helpers`) |

`test_run_tests_keyboardinterrupt` : non portable (SIGINT), documenté.

## Critères d'acceptation

- `cargo test --test unit_core_manager` : 20 tests verts, parallèles.
- `cargo test --test functional` et `--test runtime` inchangés (aucune régression de `discover_files`).
- Porte de qualité ; inventaire mis à jour.

## Pièges

- `discover_files` Python préfixe `./` les **fichiers relatifs** existants ou non, mais laisse tel quel un
  chemin exclu ; reproduire exactement ce que fait `manager.py:200-259` (déjà porté — vérifier avec ces tests).
- Les ensembles Python (`{…}`) deviennent des `Vec` triés côté Rust : comparer après tri.
- Le warning de `populate_baseline` n'est émis que si le niveau global le permet : régler
  `log::set_level(Level::Warning)` dans le bloc `with_buffer`, sous un `Mutex` local au fichier.
