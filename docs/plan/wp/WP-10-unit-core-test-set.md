# WP-10 — `BanditTestSet` : profils, filtrage des blacklists (registre réel)

**Agent** : `banditrs-wp-high` · **Vague** A · **Fichier de test** : `tests/unit_core_test_set.rs` · **Stubs** : 13
**Python** : `tests/unit/core/test_test_set.py`, `bandit/core/test_set.py` · **Rust** : `src/core/test_set.rs`, `src/core/blacklist.rs`, `src/core/registry.rs`

## Objectif

Les tests Python injectent un registre factice (un plugin `B000` sur `Str` + deux blacklists `B401`
telnet et `B302` marshal). Le registre Rust est statique ; on rejoue donc **les mêmes assertions
structurelles avec le registre réel**, en calculant les valeurs de référence depuis le registre plutôt
qu'en les codant en dur (le test reste vrai quand un plugin est ajouté).

Correspondances : `B000`/`Str` → `B105` (`hardcoded_password_string`, vérifie `Str`) ; `B401` → même id
(telnetlib, présent dans `Import` et `ImportFrom`) ; `B302` → même id (marshal, présent dans `Call`).

## Fichiers

- Possédés : `tests/unit_core_test_set.rs`, `src/core/test_set.rs`, `src/core/blacklist.rs`, `src/core/registry.rs`.
- Lus : `docs/spec/core.md` §5, `src/core/config.rs` (`Profile`).

## Helpers (fichier de test)

```rust
fn ts(profile: Profile) -> TestSet;                       // TestSet::new(&BanditConfig::default(), &profile)
fn n_plugins(kind: NodeKind) -> usize;                     // registry::PLUGINS filtrés par `checks.contains(&kind)`
fn n_blacklist(ts: &TestSet, kind: NodeKind) -> usize;     // TestRef::Blacklist dans get_tests(kind)
fn table_len(ts: &TestSet, kind) -> usize;                 // ts.blacklist.as_ref().map(|t| t.<kind>.len())
```
`BlacklistTable` expose `call`, `import`, `import_from` (ajouter des accesseurs `pub` si les champs ne le
sont pas — fichier possédé).

## Tests

| Test | Profil | Attendu |
|---|---|---|
| `test_has_defaults` | défaut | `get_tests(Str).len() == n_plugins(Str)` et `> 0` |
| `test_profile_include_id` | `include: {"B105"}` | `get_tests(Str).len() == 1` ; `get_tests(Call)` sans plugin (seulement, éventuellement, aucun `Blacklist` car `B001` absent de l'include) |
| `test_profile_exclude_id` | `exclude: {"B105"}` | `get_tests(Str).len() == n_plugins(Str) - 1` |
| `test_profile_include_none` | `include: {}` | identique au défaut |
| `test_profile_exclude_none` | `exclude: {}` | identique au défaut |
| `test_profile_has_builtin_blacklist` | défaut | `n_blacklist(Import) == n_blacklist(ImportFrom) == n_blacklist(Call) == 1`, et c'est le **dernier** test de chaque liste |
| `test_profile_exclude_builtin_blacklist` | `exclude: {"B001"}` | `0` blacklist sur les trois kinds ; `ts.blacklist.is_none()` |
| `test_profile_exclude_builtin_blacklist_specific` | `exclude` = **tous** les ids de `blacklist::all_entries()` | même résultat que ci-dessus |
| `test_profile_filter_blacklist_none` | défaut | `table_len(Import) == nombre d'entrées statiques Import`, idem `ImportFrom`, `Call` (comparer à `BlacklistTable::builtin()`) |
| `test_profile_filter_blacklist_one` | `exclude: {"B401"}` | `table_len(Import)`/`(ImportFrom)` = builtin − 1 ; `table_len(Call)` inchangé |
| `test_profile_filter_blacklist_include` | `include: {"B001", "B401"}` | `table_len(Import) == table_len(ImportFrom) == 1` ; `table_len(Call) == 0` et `n_blacklist(Call) == 0` |
| `test_profile_filter_blacklist_all` | `exclude` = tous les ids blacklist | `0` test `Blacklist` partout (`blacklist.is_none()`) |
| `test_profile_blacklist_compat` | `Profile { include: {"B001"}, blacklist: Some({"Call": [entrée marshal]}) }` (entrée : `BlacklistEntry { name: "marshal", id: "B302", cwe: Cwe::DESERIALIZATION_OF_UNTRUSTED_DATA, qualnames: ["marshal.load", "marshal.loads"], message: "Deserialization with the marshal module is possibly dangerous.", level: "MEDIUM" }`) | `table_len(Call) == 1` ; `table_len(Import) == table_len(ImportFrom) == 0` ; `n_blacklist(Import) == n_blacklist(ImportFrom) == 0`, `n_blacklist(Call) == 1` |

Chaque test Rust cite dans son doc-commentaire l'adaptation (« registre factice B000/Str → B105 »).

## Critères d'acceptation

- `cargo test --test unit_core_test_set` : 13 tests verts.
- `cargo test --test functional` inchangé (le filtrage des profils est exercé par `test_django_xss_*` de WP-01).
- Porte de qualité ; inventaire mis à jour.

## Pièges

- `B001` dans `include` **sans** autre id de blacklist ⇒ toutes les blacklists ; avec un id ⇒ seulement
  celui-là (règle `_get_filter`, déjà implémentée).
- Une table dont un kind est vide ne reçoit pas de test `Blacklist` pour ce kind (« pointless »).
- Ne pas coder en dur des effectifs de plugins : ils changent avec upstream.
