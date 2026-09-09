# WP-21 — Intégration : porte de qualité, CI, documentation

**Vague** C (jalon J5) · **Fichiers** : `scripts/check.sh`, `.github/workflows/ci.yml`, `docs/plan/README.md`, `PLAN.md`, `README.md`, `DEVIATIONS.md`

## Livrables

1. **`scripts/check.sh parity`** — corpus `smoke` + matrice CLI face à la référence. Le mode par défaut
   n'appelle **pas** le réseau : la matrice et le golden sont rejoués par `cargo test`, sans Python.
2. **CI** — un job `parity` (PR et push) qui installe le bandit de référence *au commit exact* dont le
   golden a été enregistré (`1d3053d`) et joue la matrice CLI puis le tier `smoke` ; un job
   `parity-nightly` hebdomadaire pour le tier `standard` et pour le **contrôle de dérive amont**
   (régénérer le golden, `git diff --exit-code tests/golden`) — prévu par WP-16 §3 et jamais construit
   jusqu'ici.
3. **Dérive documentaire** — `PLAN.md`, `docs/plan/README.md` et `README.md` décrivaient tous les trois la
   CI comme désactivée et logée dans `ci.yml.disabled` ; elle est active dans `ci.yml` depuis la mise en
   place du wheel multi-plateforme.
4. **`DEVIATIONS.md`** — les écarts trouvés par la matrice, et la résorption de ceux qui n'en étaient plus.

## Critères d'acceptation

- `scripts/check.sh` (sans réseau) et `scripts/check.sh parity` (avec) verts.
- Aucun document ne décrit plus la CI comme désactivée.
