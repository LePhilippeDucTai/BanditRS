# WP-12 — Formatters `text` et `screen` : chaînes exactes, baseline, testabilité de `screen`

**Agent** : `banditrs-wp-medium` · **Vague** A · **Fichiers de test** : `tests/unit_formatters_text.rs` (2 stubs + 1 partiel), `tests/unit_formatters_screen.rs` (4 stubs)
**Python** : `tests/unit/formatters/test_{text,screen}.py`, `bandit/formatters/{text,screen}.py` · **Rust** : `src/formatters/text.rs`, `src/formatters/screen.rs`

## Objectif

Exposer `_output_issue_str` dans les deux formatters, rendre `screen` testable (écriture dans un
`Write` au lieu de `print`), et porter les 6 tests restants + renforcer `test_report_nobaseline` (text).

## Fichiers

- Possédés : les deux fichiers de test, `src/formatters/text.rs`, `src/formatters/screen.rs`.
- Lus : `tests/common/formatters.rs` (fixture, **propriété de WP-13** — helpers supplémentaires dans tes
  fichiers de test), `docs/spec/cli_formatters_tests.md` §B.5, B.6, C.7 ; `src/core/metrics.rs`
  (`FileMetrics` champs `pub`, `Scores::note`).

## API à exposer

```rust
// text.rs et screen.rs
pub fn output_issue_str(issue: &Issue, indent: &str, show_lineno: bool, show_code: bool, lines: i64) -> String;
// screen.rs
pub const COLOR: ...;                       // DEFAULT, HEADER, LOW, MEDIUM, HIGH (déjà présents ? les rendre pub)
pub fn header(text: &str) -> String;        // COLOR["HEADER"] + text + COLOR["DEFAULT"]
pub fn report_to(manager: &Manager, out: &mut dyn Write, sev: Rank, conf: Rank, lines: i64) -> io::Result<()>;
// `report(...)` existant = `report_to(..., stdout)` + le hint « not written to file »
```

## Tests `text`

| Test | Mise en place | Attendu |
|---|---|---|
| `test_output_issue` | issue de la fixture Python (`fname = "code.py"`, `test = "bandit_plugin"`, `lineno = 1`, `test_id` vide, col 0) avec un `SourceFile` attaché ; `indent = "CCCCCCC"` | (1) `output_issue_str(&i, indent, true, true, -1)` == 5 lignes exactes ci-dessous **+** chaque ligne de `i.get_code(-1, true)` (séparée sur `\n`) préfixée de `indent` (adaptation du mock `"DDDDDDD"`) ; (2) `show_code = false` → les 5 lignes seulement ; (3) `show_lineno = false` → ligne Location `"{indent}   Location: code.py::"` |
| `test_report_nobaseline` (partiel) | `metrics.totals = FileMetrics { loc: 1000, nosec: 50, skipped_tests: 0, issues: Some([[1,1,1,1],[1,1,1,1]]) }` ; `scores = [Scores::note(Undefined, Undefined)]` (poids 1 → `SEVERITY: 1, CONFIDENCE: 1`) ; `files_list = ["binding.py"]` ; `skipped = [("abc.py","File is bad")]` ; `excluded_files = ["def.py"]` ; `verbose = true` ; 2 issues a, b ; `lines = 5` | les 20 sous-chaînes de `test_text.py` lignes 144–165 **et** la sortie contient `output_issue_str(&a, "", true, true, 5)` et idem pour b (adaptation de `assert_has_calls`) |
| `test_report_baseline` | résultats `[a, b1, b2]` (a unique ; b1, b2 même signature), `baseline = [entrée ne correspondant à rien]` → `get_issue_list` est la variante candidats `{a: [a], b1: [b1, b2], b2: [b1, b2]}` ; `lines = 5` | la sortie contient `output_issue_str(&a, "", true, true, 5)` ; `output_issue_str(&b1, "", false, false, -1)` ; `"-- Candidate Issues --"` ; `output_issue_str(&b1, "          ", true, true, 5)` et `output_issue_str(&b2, "          ", true, true, 5)` (indent = 10 espaces) |

Lignes exactes de `output_issue_str` (text) :
```
{indent}>> Issue: [{test_id}:{test}] {text}
{indent}   Severity: Medium   Confidence: Medium
{indent}   CWE: CWE-605 (https://cwe.mitre.org/data/definitions/605.html)
{indent}   More Info: {docs_utils::get_url(test_id)}
{indent}   Location: code.py:1:0
```

## Tests `screen`

| Test | Attendu |
|---|---|
| `test_output_issue` | comme text, mais première ligne `"{indent}{COLOR[MEDIUM]}>> Issue: [...] ..."` et ligne Location suivie de `COLOR[DEFAULT]` |
| `test_no_issues` | `report_to` sur un manager vide, `lines = 5` → contient `"No issues identified."` |
| `test_report_nobaseline` | même mise en place que text (`_totals` = `{loc: 1000, nosec: 50}` + compteurs à 1) ; contient `"Run started"`, `header("Files in scope (1):")`, `"\n\tbinding.py (score: {SEVERITY: 1, CONFIDENCE: 1})"`, `header("Files excluded (1):") + "\n\tdef.py"`, `"Total lines of code: 1000\n\tTotal lines skipped (#nosec): 50"`, `"Total issues (by severity):\n\t\tUndefined: 1\n\t\tLow: 1\n\t\tMedium: 1\n\t\tHigh: 1"`, idem `(by confidence)`, `header("Files skipped (1):") + "\n\tabc.py (File is bad)"` ; plus les deux `output_issue_str(&x, "", true, true, 5)` |
| `test_report_baseline` | comme text (candidats indentés de 10 espaces) |

## Critères d'acceptation

- `cargo test --test unit_formatters_text --test unit_formatters_screen` : 4 + 5 tests verts, 0 ignoré.
- `bandit examples/nosec.py` (txt et screen) inchangé : différentiel `-f txt` sur `examples/` à zéro diff ;
  `-f screen` comparé à l'œil (Python n'écrit screen que sur un TTY).
- Porte de qualité ; inventaire mis à jour ; commentaire `PARTIAL (WP-12)` supprimé.

## Pièges

- Python `report` de `text` n'écrit rien si `manager.quiet` et aucun résultat ; garder ce comportement.
- `screen` n'imprime pas la ligne « Total potential issues skipped… ».
- Les couleurs sont des séquences ANSI littérales (`\x1b[95m`…) : les tester par égalité de chaînes, pas
  via un terminal.
