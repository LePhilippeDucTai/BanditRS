---
name: banditrs-dispatch
description: >
  Orchestrate the parallel BanditRS work packages: launch a wave of `banditrs-wp-*` subagents (one per
  WP from docs/plan/README.md §4), review and merge their branches in order, re-run the quality gate,
  update PLAN.md / test-inventory.md, and push. Use when the user asks to "lance la vague A", "dispatch
  les WP", "lance WP-06 et WP-08", "fusionne les lots", "où en sont les WP", or any request to run the
  BanditRS plan with subagents. For single-threaded continuation work use `banditrs-dev` instead.
---

# Orchestrer les lots BanditRS

Tu es l'**orchestrateur** (maître d'œuvre) du plan `docs/plan/README.md`. Tu ne portes pas les tests
toi-même : tu lances des sous-agents, tu réceptionnes, tu fusionnes, tu vérifies, tu tiens les documents
d'état. Lis d'abord `docs/plan/README.md` (§4 dispatch, §5 propriété, §6 protocole) et
`docs/plan/agent-playbook.md` §7 (prompt de lancement).

## 1. Avant de lancer

```bash
cd /home/user/banditrs
git status --short                 # propre
git branch --show-current          # branche d'intégration (claude/banditrs-implementation-* ou main)
cargo test --all-targets 2>&1 | grep -E 'test result|FAILED'   # tout vert
scripts/wp_status.sh               # stubs restants par WP
```
Décide la vague (README §4) : A = WP-01…13 (priorité absolue : TDD), B = WP-14, 15, 16. Si le nombre
d'agents simultanés doit rester modeste : A1 = WP-02, 05, 07, 11, 12, 13 puis A2 = WP-01, 03, 04, 06,
08, 09, 10. N'inclus pas un lot déjà terminé (0 stub restant et inventaire à « porté »).

## 2. Lancer une vague

Un appel `Agent` **par lot, tous dans le même message** (exécution concurrente), en arrière-plan :
- `subagent_type` : `banditrs-wp-low` / `banditrs-wp-medium` / `banditrs-wp-high` selon la colonne
  « Agent » du tableau §4 ;
- `isolation: "worktree"` (déjà dans la définition de l'agent, le repasser ne coûte rien) ;
- `prompt` : le texte du playbook §7 avec `WP-xx` et `<slug>` substitués (le slug est le nom de la fiche
  sans le préfixe `WP-xx-` ni `.md`) ;
- `description` : `WP-xx <slug>`.

Pendant l'exécution, ne travaille pas sur les fichiers des lots lancés. Tu peux préparer la vague
suivante ou mettre à jour `PLAN.md`.

## 3. Réceptionner un lot

À la notification de fin, lis le rapport (format playbook §6). Si le format n'est pas respecté ou si
« Stubs activés » est incomplet sans justification, renvoie l'agent (`SendMessage`) avec la demande
précise. Sinon, lance la revue : `Agent(subagent_type: "banditrs-reviewer", prompt: "Revois la branche
wp/WP-xx-<slug> (lot WP-xx)")`. Un verdict « À CORRIGER » repart vers l'agent du lot avec les actions
demandées.

## 4. Fusionner (ordre : low → medium → high, un lot à la fois)

```bash
git merge --no-ff wp/WP-xx-<slug> -m "Merge WP-xx <slug>"
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --all-targets
scripts/wp_status.sh
```
- Conflit trivial (lignes voisines de `test-inventory.md`, `DEVIATIONS.md`) : résous-le toi-même.
- Conflit de code : `git merge --abort`, puis demande à l'agent du lot de rebaser sa branche sur
  l'intégration (`SendMessage`) — ne résous pas à sa place une logique que tu n'as pas écrite.
- Rouge après fusion : idem, renvoi avec la sortie de la commande en échec. Jamais de fusion rouge
  « à corriger plus tard », jamais de `--force`.
- Si un lot touche un fichier hors propriété sans l'avoir signalé : refus, retour à l'agent.

Après chaque fusion réussie : commit de fusion présent, `scripts/wp_status.sh` montre la baisse.

## 5. Clore une vague

1. Différentiel complet : `cargo build --release && scripts/diff_against_python.sh examples` — tout diff
   non couvert par `DEVIATIONS.md` est un bug : ouvre un lot de correction (agent `high`) avant de continuer.
2. `PLAN.md` §3 : mettre à jour le tableau des jalons (J1/J2/J3) et les effectifs de tests ; §5 doit
   pointer vers `docs/plan/README.md`.
3. `docs/plan/test-inventory.md` : synthèse (compteurs) cohérente avec `scripts/wp_status.sh`.
4. Si J1 est atteint (0 stub) : passer le job `wp-status` de la CI en `--check` (fiche WP-16).
5. Commit, push de la branche d'intégration (`git push -u origin <branche>`), puis la procédure de fin de
   session du skill `banditrs-dev` §5 (validation complète puis fusion dans `main`, telle que l'utilisateur
   l'a demandée pour ce dépôt).

## 6. Suivi

« Où en sont les WP ? » → `scripts/wp_status.sh` + liste des branches `git branch --list 'wp/*'` +
état des agents en cours ; réponds en trois lignes : stubs restants par lot, lots fusionnés, blocages.
