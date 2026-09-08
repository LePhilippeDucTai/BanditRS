# WP-04 — `bandit-baseline` : dépôts git temporaires et `initialize()`

**Agent** : `banditrs-wp-high` · **Vague** A · **Fichier de test** : `tests/unit_cli_baseline.rs` · **Stubs** : 12
**Python** : `tests/unit/cli/test_baseline.py`, `bandit/cli/baseline.py` · **Rust** : `src/cli/baseline.rs`

## Objectif

Couvrir `bandit-baseline` avec de vrais dépôts git créés par le test (CLI `git`, disponible dans le
`PATH`), et exposer `initialize()` pour les 6 tests qui n'attendent que `(None, None, None)`.

## Fichiers

- Possédés : `tests/unit_cli_baseline.rs`, `src/cli/baseline.rs`.
- Lus : `docs/spec/cli_formatters_tests.md` §A.13, C.5 ; `src/log.rs`.

## API à exposer / refactor de testabilité (`src/cli/baseline.rs`)

```rust
pub const BASELINE_TMP_FILE: &str = "_bandit_baseline_run.json_";
pub const REPORT_BASENAME: &str = "bandit_baseline_result";
pub struct Initialized { pub output_format: String, pub report_fname: String /* + commit info si besoin */ }
/// `initialize()` : `None` ⇔ Python `(None, None, None)`. `cwd` explicite (au lieu de `os.getcwd()`)
/// pour que les tests restent parallèles ; `main` passe `std::env::current_dir()`.
pub fn initialize(cwd: &Path, bandit_args: &[String]) -> Option<Initialized>;
pub fn init_logger();
```
Toutes les commandes `git` reçoivent `-C <cwd>`. `bandit_path()` doit pouvoir être surchargé pour les
tests par la variable d'environnement `BANDITRS_BANDIT_EXE` (documentée dans le doc-commentaire ;
sans impact utilisateur).

## Mise en place commune (helper du fichier de test)

```rust
fn git(repo: &Path, args: &[&str]) -> String;   // `git -C repo args…`, panique si échec
fn init_repo() -> tempfile::TempDir;             // git init -q ; config user.name/email ; commit --allow-empty -m "Initial commit"
fn bin_dir() -> PathBuf;                         // parent de env!("CARGO_BIN_EXE_bandit") → à préfixer au PATH
fn run_baseline(repo: &Path, args: &[&str], env: &[(&str, &str)]) -> (i32, String);  // binaire bandit-baseline
```
Contenu `mktemp.py` pour les fichiers additionnels (`temp_file_contents`), `okay.py` (bénin),
`os_system.py` (malveillant) : lus depuis `examples/`. `config` = le YAML de `test_baseline.py` lignes 15–30
(profil `test` → `start_process_with_a_shell`, `shell_injection` avec `subprocess: []`, `no_shell: []`,
`shell: [os.system]`).

## Tests

| Test | Mise en place / adaptation | Attendu |
|---|---|---|
| `test_bandit_baseline` | repo ; écrire `bandit.yaml` (non suivi) ; 3 branches créées **depuis HEAD courant** (`git checkout -b`) : `benign1` (+`benign_one.py`), `malicious` (+`malicious.py`), `benign2` (+`benign_two.py`) ; après chaque commit : `bandit-baseline -c bandit.yaml -r . -p test` | rc 0, 1, 0 |
| `test_main_non_repo` | tmp sans git | rc 2 |
| `test_main_git_command_failure` | repo + 2 commits ; **adaptation** : `git` factice en tête de PATH (script shell qui délègue au vrai `git` sauf pour la sous-commande utilisée pour lire le commit courant/parent — voir `git(&[...])` dans `baseline.rs` — où il échoue) | rc 2 ; sortie contient `Unable to get current or parent commit` |
| `test_main_no_parent_commit` | repo avec un seul commit | rc 2 ; `Parent commit not available` |
| `test_main_subprocess_error` | repo + 2 commits ; **adaptation** : `BANDITRS_BANDIT_EXE` → script qui écrit sur stdout et `exit 3` | rc 3 |
| `test_init_logger` | `init_logger()` | `log::level() == Level::Info` (format `[%(levelname)7s ] %(message)s`, sortie stdout) |
| `test_initialize_no_repo` | tmp sans git | `None` |
| `test_initialize_git_command_failure` | **adaptation** : `PATH` réduit à un répertoire vide (pas de `git`) | `None` (`Git command not found` / `Git not available`) |
| `test_initialize_dirty_repo` | repo ; `dirty_file.py` écrit et `git add` sans commit | `None` (`Current working directory is dirty and must be resolved`) |
| `test_initialize_existing_report_file` | repo ; `bandit_args = ["-f", "txt", "test"]` ; fichier `bandit_baseline_result.txt` présent | `None` |
| `test_initialize_with_output_argument` | repo ; `bandit_args = ["-o", "bandit_baseline_result"]` | `None` |
| `test_initialize_existing_temp_file` | repo ; fichier `_bandit_baseline_run.json_` présent | `None` |

Pour `test_bandit_baseline`, le `PATH` doit contenir `bin_dir()` (ou `BANDITRS_BANDIT_EXE` pointer sur
`CARGO_BIN_EXE_bandit`) pour que le `bandit` invoqué soit celui du build. `bandit.yaml` reste non suivi :
`is_dirty` ignore les fichiers non suivis (`--untracked-files=no`), comme `repo.is_dirty()` en Python.

## Critères d'acceptation

- `cargo test --test unit_cli_baseline` : 12 tests verts, parallèles (aucun `set_current_dir`, chaque
  test son `TempDir`, `PATH` passé par `Command::env`, pas par `std::env::set_var`).
- Comportement CLI inchangé : vérification manuelle `bandit-baseline --help` et un run sur un dépôt
  jetable (voir `PLAN.md` M8).
- Porte de qualité ; inventaire mis à jour.

## Pièges

- `git commit` exige `user.name`/`user.email` : les fixer dans le repo temporaire (`git -C repo config`).
- Le premier commit Python est `index.commit("Initial commit")` sur un index vide → `--allow-empty`.
- `bandit-baseline` réinitialise le working tree (`reset --hard`) : ne jamais l'exécuter dans le dépôt
  BanditRS lui-même.
