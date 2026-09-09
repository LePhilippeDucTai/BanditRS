# WP-18 — Matrice différentielle CLI (stdout, stderr, code de sortie)

**Vague** C (jalon J5) · **Fichiers** : `scripts/cli_matrix.py`, `tests/cli_matrix/**`, `tests/cli_matrix.rs`

## Objectif

C'est le cœur de la revendication « drop-in replacement ». Tous les harnais antérieurs comparaient une
seule chose : le rapport `-f json` sur stdout. Or ce sur quoi un remplacement se juge, c'est aussi le
**code de sortie** sur lequel un job de CI branche, le **stderr** qu'un développeur lit, et le
comportement de **toutes** les options — pas seulement de l'invocation par défaut.

## Livrables

1. `tests/cli_matrix/cases.tsv` — 88 invocations : seuils `-l`/`-i` et `--severity-level`/
   `--confidence-level`, sélection `-t`/`-s`, profils et fichiers de config, les neuf formatters,
   `--msg-template`, `-o` (le contenu du fichier écrit est comparé, pas seulement stdout), `-n`, `-a`,
   `-q`/`-v`/`-d`, `--exit-zero`, `--ignore-nosec`, découverte des cibles (récursive ou non, exclusions,
   globs, lien symbolique, chemin inexistant, répertoire vide, fichier non-UTF-8, fichier au parsing
   invalide), `.bandit`, toutes les erreurs d'usage qui sortent en 2, baselines, et les binaires frères
   `bandit-baseline` / `bandit-config-generator`.
2. `tests/cli_matrix/workspace/` — l'arborescence de fixtures, recopiée dans un répertoire temporaire par
   cas, de sorte que **tous les chemins de sortie sont relatifs** et n'ont pas besoin d'être normalisés.
3. `scripts/cli_matrix.py` — `gen` (enregistre le golden depuis la référence), `diff` (compare les deux
   outils maintenant), `run <id>` (les deux sorties côte à côte, pour investiguer).
4. `tests/cli_matrix.rs` — rejoue le golden contre les binaires Rust, **sans Python installé**, donc
   gratuitement dans `cargo test`.

## Deux pièges du harnais, pour mémoire

- `subprocess(text=True)` applique la traduction universelle des fins de ligne et réécrit les `\r\n` que
  le module `csv` de Python émet : la comparaison doit se faire sur des octets.
- `bandit-baseline` appelle `bandit` via le PATH ; chaque outil doit trouver **son** binaire en premier,
  sinon on compare un outil à lui-même.

## Critères d'acceptation

- `scripts/cli_matrix.py diff` : 0 divergence inattendue. Chaque divergence attendue est indexée sur un
  numéro de `DEVIATIONS.md` dans le dictionnaire `EXPECTED`, et son golden est enregistré depuis
  BanditRS (le rejeu continue donc de détecter une régression Rust).
- `cargo test --test cli_matrix` vert sans Python.
