# WP-13 — Formatters csv / custom / html / json / sarif / xml / yaml

**Agent** : `banditrs-wp-medium` · **Vague** A · **Fichiers de test** : `tests/unit_formatters_{csv,custom,html,json,sarif,xml,yaml}.rs` · **Stubs** : 1 (`html::test_report_contents`) + 4 tests partiels (csv, xml, yaml, html `test_report_with_skipped`)
**Python** : `tests/unit/formatters/test_*.py`, `bandit/formatters/*.py` · **Rust** : `src/formatters/{csv,custom,html,json,sarif,xml,yaml,mod}.rs`, `tests/common/formatters.rs`

## Objectif

Amener les tests des formatters structurés au niveau des tests Python : relire la sortie avec un vrai
parseur (CSV, XML via `roxmltree`, HTML via `scraper`, YAML via `pycompat::yaml_load`) au lieu de
sous-chaînes, et porter `test_report_contents` (candidats, classes CSS, compteurs).

## Fichiers

- Possédés : les 7 fichiers de test, `tests/common/formatters.rs`, `src/formatters/{csv,custom,html,json,sarif,xml,yaml,mod}.rs`,
  `src/pycompat/{csv,html,xml,yaml_emit,json,pyformat,urlquote}.rs`.
- Dépendances déjà déclarées : `roxmltree`, `scraper` (dev-deps).

## Travaux

### `csv::test_report` (partiel)
Lire la sortie comme `csv.DictReader` : en-tête → noms de champs, première ligne → valeurs (pas de
guillemets dans cette fixture ; un splitter `,` suffit, ou un mini-parseur RFC 4180 dans le test).
Asserter : `filename == tmp`, `issue_severity == "MEDIUM"`, `issue_confidence == "MEDIUM"`,
`issue_text == TEXT`, `line_number == "4"`, `line_range == "[4]"`, `test_name == TEST_NAME`,
`more_info` non vide, `col_offset == "8"`, `end_col_offset == "16"`.

### `xml::test_report` (partiel)
`roxmltree::Document::parse` : `testsuite` → `testcase` (`@classname == tmp`, `@name == TEST_NAME`) →
`error` (`@message == TEXT`, `@more_info` présent).

### `yaml::test_report` (partiel)
Relire avec `banditrs::pycompat::yaml_load::safe_load` : `generated_at` présent, `results[0]` :
`filename`, `issue_severity == "MEDIUM"`, `issue_confidence == "MEDIUM"`, `issue_text`,
`line_number == 4`, `line_range == [4]`, `test_name`, `more_info` non vide. **Adaptation** : le test
Python appelle en réalité le formatter JSON (et relit en YAML) — c'est pourquoi il attend `candidates` ;
le vrai formatter YAML n'a pas de branche baseline : ne pas asserter `candidates`, l'expliquer dans le
doc-commentaire.

### `html::test_report_with_skipped` (partiel)
`scraper::Html::parse_document` : exactement un `div#skipped`, dont le texte contient `abc.py` et
`File is bad`.

### `html::test_report_contents` (stub)
Mise en place : `metrics.totals.loc = 1000`, `nosec = 50` ; issues `a` (LOW, `fname = "abc.py"`,
`test = "AAAAAAA"`, `text = "BBBBBBB"`), `b` (MEDIUM), `c` (HIGH), chacune avec un `SourceFile` dont la
ligne de l'issue est `some code` (adaptation du mock `get_code → "some code"`). Le mock
`get_issue_list → {a: [x, y], b: [x], c: [y]}` s'obtient avec la « variante candidats » : résultats
`[a, a2, b, c]` où `a2` a la signature de `a`, et `baseline = [entrée ne correspondant à rien]`
→ `{a: [a, a2], a2: [a, a2], b: [b], c: [c]}` ; les index deviennent `issue-0` = a, `issue-1` = a2,
`issue-2` = b, `issue-3` = c (documenter le décalage).
Assertions (`scraper` + sélecteurs) : `span#loc` == `"1000"`, `span#nosec` == `"50"` ; `issue-0`
contient 1 `div.issue-sev-low`, 1 `div.candidates`, 2 `div.candidate`, 0 `div.code` ; `issue-2` : 1
`div.issue-sev-medium`, 0 `.candidates`, 0 `.candidate`, 1 `div.code` ; `issue-3` : 1 `div.issue-sev-high`,
1 `div.code` ; le premier `.candidate` de `issue-0` et le `.code` de `issue-2` contiennent `some code` ;
le texte de `issue-0` contient `AAAAAAA:`, `BBBBBBB`, `abc.py`, `Line number: 1`. **Adaptation** :
Python force `confidence = "CCCCCCC"` (chaîne libre) ; `Rank` est un enum → garder MEDIUM et asserter
`Medium`/`MEDIUM` selon le template à la place de `CCCCCCC`.
Alternative propre si la variante candidats est trop contraignante : ajouter dans `html.rs` (fichier
possédé) une fonction `report_issues(manager, out, &IssueList)` appelée par `report`.

### Vérifications sans nouveau test
`custom`, `json`, `sarif` : déjà portés ; relire les tests et compléter s'ils divergent du Python
(`json` : `line_range == [line_number]`, `candidates` présents ; `sarif` : `semanticVersion == VERSION`).

## Critères d'acceptation

- `cargo test --test unit_formatters_csv … --test unit_formatters_yaml` : 9 tests verts, 0 ignoré,
  commentaires `PARTIAL (WP-13)` supprimés.
- Différentiel `scripts/diff_against_python.sh examples` sans nouveau diff (tous formats).
- Porte de qualité ; inventaire mis à jour.

## Pièges

- `scraper` : `Selector::parse("div#issue-0 div.candidate")` ; `.text().collect::<String>()`.
- Le HTML n'échappe **que** le code (`test_escaping`) — ne pas « corriger ».
- `IssueList` est un enum : construire la fixture via `Manager` (résultats + baseline), pas à la main,
  sauf à passer par l'alternative `report_issues`.
