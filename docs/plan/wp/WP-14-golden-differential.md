# WP-14 — Corpus « golden » et harnais différentiel automatisé

**Agent** : `banditrs-wp-medium` · **Vague** B (fusion après WP-01) · **Fichiers** : `scripts/gen_golden.sh`, `scripts/diff_against_python.sh`, `tests/golden/**`, `tests/golden.rs`
**Référence** : `PLAN.md` §5 (méthode du différentiel ad hoc), `DEVIATIONS.md`

## Objectif

Aujourd'hui la parité est prouvée par un différentiel manuel qui exige Python. Ce lot fige les sorties
de bandit Python dans le dépôt (**golden files**) et ajoute un test Rust qui les rejoue : la CI vérifie
la parité sans Python, et toute régression de sortie devient un échec de test avec un diff lisible.

Analogie : on photographie une fois le résultat de l'étalon, puis chaque build est comparé à la photo ;
on ne refait la photo (régénération) que quand upstream ou une déviation change, et ce changement est
alors **visible dans le diff du commit**.

## Livrables

1. `scripts/gen_golden.sh` — depuis la racine du dépôt, avec `/home/user/.pyenv-bandit/bin/bandit`, produit :
   - `tests/golden/examples.<fmt>` pour `fmt ∈ {json, yaml, csv, xml, txt, html, sarif, custom}` :
     `bandit -r examples -f <fmt>` (chemins relatifs `examples/…`, stdout seulement) ;
   - `tests/golden/files/<nom>.json` : `bandit examples/<nom> -f json` pour chaque fixture
     (hors `nonsense2.py`, binaire : la sortie contient l'erreur, à inclure quand même si stable) ;
   - normalisation identique des deux côtés (fonction partagée, cf. 3) ;
   - en-tête `tests/golden/README.md` : commit upstream (`1d3053d`), version Python, date, commande.
2. `tests/golden.rs` — pour chaque golden : exécute `env!("CARGO_BIN_EXE_bandit")` avec
   `BANDITRS_PYTHON_COMPAT=3.11`, `current_dir = CARGO_MANIFEST_DIR`, mêmes arguments ; normalise ;
   compare (`pretty_assertions::assert_eq!` pour un diff lisible). Un test par format + un test paramétré
   sur `files/` (boucle avec accumulation des échecs et message listant les fichiers en écart).
3. Normalisation (même liste dans le script `sed` et dans `golden.rs`) :
   `generated_at`/`endTimeUtc`/`Run started:` → `<TS>` ; `readthedocs.io/en/<x>/` → `en/X/` ;
   `semanticVersion`/`version` de l'outil → `<VER>` ; adresses mémoire `0x[0-9a-f]+` → `0x0`
   (DEVIATIONS #5) ; chemins absolus (`custom` utilise `abspath`, SARIF des URIs `file://`) : remplacer
   le préfixe du dépôt par `<ROOT>`.
4. Déviations connues à gérer explicitement :
   - #10/#11 (YAML : pliage double-quoted, ancres `&id001`) — comparer YAML **après** `yaml_load`
     (structure) plutôt qu'octet à octet, ou lister les fichiers exclus de la comparaison brute ;
   - #9 est levée par WP-01 (fusionner WP-01 avant de générer) ;
   - #2 (CSV sans CWE) / #1 (`nosec` virgule) : n'apparaissent pas dans `examples/` — vérifier.
5. `scripts/diff_against_python.sh` finalisé : mode fichier par fichier (JSON) avec **liste blanche** des
   écarts documentés, code de sortie non nul sur écart inattendu, option `--stdlib` pour
   `/usr/lib/python3.11` ; exécution sur la stdlib documentée dans le rapport (résultat attendu : zéro
   diff hors #11).

## Critères d'acceptation

- `cargo test --test golden` vert, sans Python installé (les golden sont committés).
- `scripts/gen_golden.sh && git diff --stat tests/golden` vide après une seconde exécution (stabilité).
- `scripts/diff_against_python.sh examples` et `--stdlib` : rapport zéro diff inattendu.
- Taille du corpus raisonnable (< 2 Mo) ; sinon réduire `files/` aux fixtures ayant des issues.
- Porte de qualité.

## Pièges

- `BANDITRS_PYTHON_COMPAT=3.11` est indispensable (positions de f-strings) tant que la référence est 3.11.
- `custom` et `sarif` contiennent des chemins absolus ; `html` contient l'horodatage dans le corps.
- L'ordre des issues d'un même nœud dépend de l'ordre alphabétique des entry points (`PLAN.md` §6) :
  un golden qui « bouge » sans changement de plugin est un bug d'ordre, pas de contenu.
