# WP-16 — Intégration continue : validation, job différentiel, tableau de bord

> ## ⛔ LOT SUSPENDU (2026-09-09)
>
> **Décision de l'utilisateur : aucune CI hébergée, aucun coût — la validation se fait uniquement en local.**
> Le workflow créé en J0 a été désactivé (renommé `.github/workflows/ci.yml.disabled` ; GitHub ne lit que
> `*.yml`/`*.yaml`, le fichier est donc inerte et aucune exécution n'est déclenchée).
>
> **Le remplaçant est `scripts/check.sh`**, qui reproduit les trois jobs à l'identique sur la machine de dév :
> build de toutes les cibles + suite complète + `wp_status.sh --check`, puis rustfmt + clippy `-D warnings`,
> puis compilation des benchs. Sortie 0 = équivalent d'une CI verte. `scripts/check.sh fast` saute les benchs.
>
> Ne pas reprendre ce lot sans accord explicite de l'utilisateur. Pour réactiver le workflow :
> `git mv .github/workflows/ci.yml.disabled .github/workflows/ci.yml`.
>
> Le reste de cette fiche est conservé tel quel pour le jour où la question se reposera.

**Agent** : `banditrs-wp-low` · **Vague** B (job golden après WP-14) · **Fichiers** : `.github/workflows/*.yml`, `README.md` (badge uniquement)
**Référence** : `.github/workflows/ci.yml` (créé en J0 : jobs `test`, `lint`, `bench`)

## Objectif

S'assurer que la CI créée en J0 passe réellement sur GitHub, puis l'étendre : job `golden` (WP-14),
job `differential` planifié (régénère les golden avec Python et signale une dérive), et faire de
`scripts/wp_status.sh --check` un job bloquant une fois J1 atteint.

## Livrables

1. Validation : pousser la branche, lire le résultat des trois jobs (outils GitHub disponibles dans la
   session orchestrateur — le rapport indique l'URL du run et corrige les erreurs de workflow
   (`--locked` exige un `Cargo.lock` à jour ; `Swatinem/rust-cache` ; toolchain `stable` vs
   `rust-toolchain.toml`).
2. Job `golden` : `cargo test --test golden` (après fusion de WP-14) — dans le job `test` ou séparé.
3. Job `differential` (`schedule: cron` hebdomadaire + `workflow_dispatch`) : Python 3.11, `pip install
   "bandit[toml,yaml,sarif] @ git+https://github.com/PyCQA/bandit@1d3053d"` + `sarif_om
   jschema-to-python`, `scripts/gen_golden.sh`, puis `git diff --exit-code tests/golden` (dérive =
   échec avec le diff en artefact).
4. Job `wp-status` : `scripts/wp_status.sh` (informatif) ; passer à `--check` dans le même commit que la
   clôture de J1 (orchestrateur).
5. Badge CI dans `README.md` (ligne 1) ; pas d'autre modification du README.
6. Optionnel si rapide : `cargo audit` (job non bloquant, `continue-on-error: true`).

## Critères d'acceptation

- Les jobs `test`, `lint`, `bench` sont verts sur la branche d'intégration.
- Le workflow `differential` s'exécute avec succès en `workflow_dispatch` (une fois).
- Aucun secret requis ; aucun job ne dépasse 15 min.

## Pièges

- `cargo bench --no-run --locked` compile `criterion` (long) : garder le cache Rust.
- Les golden supposent `BANDITRS_PYTHON_COMPAT=3.11` et Python 3.11 exactement (positions de f-strings).
