# WP-09 — `Context` : tests sur de vrais extraits de code

**Agent** : `banditrs-wp-high` · **Vague** A · **Fichier de test** : `tests/unit_core_context.rs` · **Stubs** : 17 (+2 non portables)
**Python** : `tests/unit/core/test_context.py`, `bandit/core/context.py` · **Rust** : `src/core/context.rs`, `src/ast/walker.rs`

## Objectif

Les tests Python construisent `Context(context_object=dict|Mock)`. Le `Context` Rust est typé et n'existe
que pendant le parcours de l'AST : on obtient donc chaque contexte **en parcourant un vrai extrait**
avec le walker, puis on vérifie la même propriété. Le lot crée le helper de test correspondant.

## Fichiers

- Possédés : `tests/unit_core_context.rs`, `src/core/context.rs`, `src/ast/walker.rs` (+ `src/ast/*` si un
  détail de position est nécessaire).
- Lus : `docs/spec/core.md` §2–3, `src/core/tester.rs` (exemple d'implémentation de `TestRunner`),
  `src/ast/literal.rs` (`get_literal_value`, `PyValue`).

## Helper à ajouter (`src/core/context.rs` ou `src/ast/walker.rs`, `pub`, documenté « support de test »)

```rust
/// Parcourt `source` (nom de fichier `filename`) comme le ferait `BanditNodeVisitor` et appelle `f`
/// avec le `Context` de chaque nœud de type `kind`, dans l'ordre de visite.
pub fn with_contexts(filename: &str, source: &str, kind: NodeKind, f: impl FnMut(&Context<'_, '_>));
```
Implémentation : un `TestRunner` minimal (`wants(k) == (k == kind)`, `run` → `f(ctx)`), `SourceFile::new`,
`parse_module`, `Walker::new(...).process(module)`. Le helper de test côté `tests/` :
`fn first_context<T>(src, kind, f: impl FnOnce(&Context) -> T) -> T` (panique si aucun nœud).

## Tests (adaptés : mock → extrait ; les valeurs attendues restent celles du test Python)

| Test | Extrait / contexte | Attendu |
|---|---|---|
| `test_call_args` | `f(x.spam, 'eggs')`, `Call` | `call_args() == ["spam", "eggs"]` (attribut → `attr`, sinon valeur littérale) |
| `test_call_args_count` | `f('spam', 'eggs')`, `Call` ; puis `'x'`, `Str` | `Some(2)` ; `None` |
| `test_call_function_name` | `spam()`, `Call` ; puis `Str` | `Some("spam")` ; `None` |
| `test_call_function_name_qual` | `spam()`, `Call` ; puis `Str` | `Some("spam")` ; `None` |
| `test_call_keywords` | `f(arg1=x.spam, arg2='eggs')`, `Call` ; puis `Str` | `{"arg1": "spam", "arg2": "eggs"}` ; `None` |
| `test_node` | `spam()`, `Call` | `node().kind() == NodeKind::Call` ; sur `Str` : `node().kind() == Str` |
| `test_string_val` | `'spam'`, `Str` ; puis `f()`, `Call` | `Some("spam")` ; `None` |
| `test_statement` | `x = spam()`, `Call` | l'instruction englobante est un `Assign` (ajouter `statement()` si absent) |
| `test_function_def_defaults_qual` | `import spam\ndef f(a=spam.eggs): pass`, `FunctionDef` ; `def f(): pass` ; `Str` | `["spam.eggs"]` ; `[]` ; `[]` |
| `test_get_literal_value` (renommé) | `get_literal_value` sur `42`, `'spam'`, `['spam', 42]`, `('spam', 42)`, `{'spam', 42}`, `{'spam': 42, 'eggs': 'foo'}`, `spam` (Name), `b'spam'` | int 42 ; str `"spam"` ; list `["spam", 42]` ; tuple `("spam", 42)` ; set `{"spam", 42}` ; dict `{spam: 42, eggs: "foo"}` ; `"spam"` (id du `Name`) ; bytes `b"spam"` — comparer avec `PyValue::py_eq` ; le cas `None` Python (entrée absente) n'a pas d'équivalent typé : l'indiquer dans le doc-commentaire |
| `test_check_call_arg_value` | `f(spam='eggs')`, `Call` | `("spam", "eggs")` → vrai ; `("spam", ["spam","eggs"])` → vrai ; `("spam", "spam")` → faux ; `("spam", <absent>)` → faux ; `("eggs", …)` → faux ; sur `Str` : `None` |
| `test_get_lineno_for_call_arg` | `f(\n    spam=1)`, `Call` | `("spam") == Some(2)` ; `("eggs") == None` |
| `test_get_call_arg_at_position` | `f('spam')`, `Call` ; `f()` ; `Str` | `Some("spam")` / `None` (pos 1) ; `None` ; `None` |
| `test_is_module_being_imported` | `import spam`, `Import` ; puis `Str` | `("spam")` vrai, `("eggs")` faux ; faux |
| `test_is_module_imported_exact` | `import spam\nf()`, `Call` ; `f()` seul | `("spam")` vrai, `("eggs")` faux ; faux |
| `test_is_module_imported_like` | `import os.path\nf()`, `Call` ; `f()` seul | `("os")` vrai, `("bacon")` faux ; faux |
| `test_filename` | `'x'` avec `filename = "spam.py"`, `Str` | `filename() == "spam.py"` |

Non portables : `test_context_create`, `test_repr` (constructeur à partir d'un dict / `repr`).

## Critères d'acceptation

- `cargo test --test unit_core_context` : 17 tests verts.
- `cargo test --all-targets` inchangé ; `bandit --dump-walk examples/nosec.py` identique avant/après
  (le helper ne doit pas modifier le walker de production).
- Porte de qualité ; inventaire mis à jour.

## Pièges

- `Context` porte des lifetimes liées au walker : le helper prend une closure, il ne renvoie pas de `Context`.
- `call_keywords()` et `call_args()` renvoient `Result<_, PyErr>` (erreurs Python reproduites) : `unwrap`
  dans les tests.
- `check_call_arg_value` : lire la signature Rust (valeur unique vs liste de candidats) avant d'écrire
  les cinq assertions ; « absent » ≠ `None` (préambule de `DEVIATIONS.md`).
