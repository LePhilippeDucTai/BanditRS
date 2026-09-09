# WP-16 — Intégration continue : validation, job différentiel, tableau de bord

> ## ✅ LOT REPRIS (2026-09-09)
>
> **Décision précédente (levée) :** « aucune CI hébergée, aucun coût — validation uniquement en local ».
> Le workflow avait été neutralisé en `.github/workflows/ci.yml.disabled`.
>
> **Ce qui a changé :** la question du coût a été tranchée sur pièces. `LePhilippeDucTai/BanditRS`
> est un dépôt **public**, et la facturation GitHub Actions ne s'applique pas aux dépôts publics
> utilisant des runners *standard* : minutes illimitées et gratuites, y compris sur les runners
> ARM64 (`ubuntu-24.04-arm`, `windows-11-arm`) et macOS. Seuls les *larger runners* sont facturés
> même pour un dépôt public — aucun job n'en déclare. Les assets de Releases ne sont pas facturés
> non plus. Aucun secret n'est requis (le `GITHUB_TOKEN` automatique suffit).
>
> **Seule réserve :** si le dépôt passait en privé, la facturation démarrerait (2 000 min/mois
> offertes, coefficient ×2 Windows et ×10 macOS).
>
> `.github/workflows/ci.yml` est donc réactivé et étendu d'un job `python` (construction du wheel
> + fumée des quatre commandes + sdist installable). `.github/workflows/release.yml` s'y ajoute :
> six wheels (Linux/macOS/Windows × x86-64/ARM64) plus un sdist, attachés à chaque tag `v*`.
>
> `scripts/check.sh` reste la porte de qualité locale et gagne le mode `python` (qui vérifie en plus
> la **parité de sortie entre le binaire du wheel et celui construit par cargo**, ce que la CI ne
> fait pas).

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
