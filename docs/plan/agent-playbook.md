# Playbook d'un sous-agent BanditRS (un lot = un agent)

Ce document est la procédure que suit un sous-agent (Sonnet 5, effort fixé par son agent
`.claude/agents/banditrs-wp-*.md`) pour livrer **un** lot de travail (WP). Il est volontairement
prescriptif : la valeur du parallélisme vient de ce que tous les lots respectent le même contrat.

## 1. Orientation (10 minutes, pas plus)

Lire dans l'ordre, sans rien modifier :

1. `docs/plan/wp/WP-xx-*.md` — **ta fiche** : objectif, fichiers possédés, tests à porter, valeurs attendues.
2. `docs/plan/README.md` §2 (règles d'or) et §5 (propriété des fichiers).
3. `docs/plan/test-inventory.md`, section de ton fichier Python : statut de chaque test.
4. Le **fichier de test Python** de ton lot dans `/home/user/bandit/tests/...` — c'est la vérité des
   valeurs attendues ; ta fiche en est un condensé.
5. Le **code Python testé** (`/home/user/bandit/bandit/...`) et son équivalent Rust (les fichiers que tu
   possèdes). `docs/spec/*.md` et `PLAN.md` §6 (pièges) en complément.

Environnement : Rust stable (`cargo`), référence Python dans `/home/user/.pyenv-bandit`
(`bin/bandit`, `bin/python` ; `bin/python -m pytest /home/user/bandit/tests/... -q` pour rejouer un
test Python), fixtures dans `examples/` (copie de `bandit/examples`).

## 2. Mise en place

```bash
cd <ton worktree>                      # l'orchestrateur t'a lancé avec isolation: worktree
git checkout -b wp/WP-xx-<slug>        # slug = celui du nom de ta fiche
cargo test --all-targets 2>&1 | tail -3   # doit être vert avant de commencer
scripts/wp_status.sh | grep WP-xx      # tes stubs restants
```

## 3. Boucle TDD, stub par stub

Pour **chaque** stub de ton fichier miroir (ordre : du plus simple au plus structurant) :

1. **Rouge.** Retire `#[ignore = ...]`, remplace `unimplemented!` par le port du test Python : mêmes
   valeurs, mêmes chaînes (copiées, pas retapées), même nom de fonction. Lance
   `cargo test --test <fichier> <nom_du_test>` : il doit échouer *pour la raison attendue* (assertion,
   pas erreur de compilation due à une API absente — si l'API manque, c'est l'étape 2).
2. **API manquante ?** Si le test a besoin d'un `pub fn`/champ qui n'existe pas : ajoute-le **dans un
   fichier que tu possèdes**, minimal, nommé comme l'attribut Python (`get_options_from_ini`,
   `output_issue_str`…), documenté par un doc-commentaire qui cite la fonction Python. Si le fichier
   n'est pas à toi : n'y touche pas, note le besoin pour ton rapport (§6) et passe au stub suivant.
3. **Vert.** Implémente ou corrige jusqu'au vert. Comportement Python bizarre mais testé → on le
   reproduit (avec un commentaire `// upstream:` qui cite la ligne Python). Comportement Python bizarre
   *non* testé et manifestement bogué → on peut corriger, à condition d'ajouter l'entrée `DEVIATIONS.md`.
4. **Différentiel** si ta modification peut changer une sortie observable (plugin, formatter, config,
   positions) :
   ```bash
   cargo build --release
   diff <(/home/user/.pyenv-bandit/bin/bandit examples/<f>.py -f json 2>/dev/null | python3 -m json.tool) \
        <(BANDITRS_PYTHON_COMPAT=3.11 target/release/bandit examples/<f>.py -f json 2>/dev/null | python3 -m json.tool)
   ```
   Un diff non couvert par `DEVIATIONS.md` est un bug à corriger avant de continuer.
5. **Commit** : `git commit -am "WP-xx: port test_<nom> (+ <API ajoutée>)"`. Un commit par test ou
   par petit groupe de tests cohérent ; pas de commit « WIP » cassé.

Tests « adaptés » (mocks Python → fixtures réelles) : la fiche donne l'adaptation ; le doc-commentaire
du test Rust doit expliquer en une phrase ce qui a été adapté et pourquoi l'assertion reste équivalente.

Tests « partiels » (déjà présents, à renforcer) : ajouter les assertions manquantes listées dans le
commentaire `PARTIAL (WP-xx)` du test, puis supprimer ce commentaire.

## 4. Porte de qualité (avant le rapport, obligatoire)

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings      # zéro warning
cargo test --all-targets                       # zéro échec ; tes stubs ne sont plus ignorés
scripts/wp_status.sh | grep -E 'WP-xx|TOTAL'   # tes stubs : 0 (ou justifiés dans le rapport)
git status --short                             # uniquement des fichiers que tu possèdes
```

Puis mettre à jour **tes lignes** de `docs/plan/test-inventory.md` (colonne Statut → « porté », note
courte si adaptation) et, si tu as créé un écart, `DEVIATIONS.md` (numéro suivant, même style).

Interdits : modifier `Cargo.toml`/`Cargo.lock`, `src/lib.rs`, `PLAN.md`, `docs/plan/README.md`, les
fichiers d'un autre lot ; ajouter un `#[ignore]` ; désactiver, affaiblir ou supprimer un test existant ;
mettre un nom de modèle ou d'outil d'IA dans un commit, un commentaire ou un fichier.

## 5. Niveaux d'effort (ce que l'orchestrateur attend)

| Agent | Quand | Attitude attendue |
|---|---|---|
| `banditrs-wp-low` | ports directs, API déjà exposée | exécuter la fiche à la lettre, pas de refactor |
| `banditrs-wp-medium` | ports volumineux, parseurs, scripts | même chose, plus la lecture systématique du code Python correspondant avant chaque test |
| `banditrs-wp-high` | testabilité à créer, sémantique à compléter (`DeepAssignation`, conversion legacy, helper de contexte) | concevoir l'API minimale *avant* de coder, vérifier empiriquement contre Python, tenir la parité bit à bit |

## 6. Rapport de fin (message final, format fixe)

```
WP-xx <slug> — TERMINÉ | PARTIEL
Branche : wp/WP-xx-<slug>   HEAD : <sha>   Commits : <n>
Stubs activés : <k>/<n>  (restants : <liste ou aucun>)
Tests partiels renforcés : <liste ou aucun>
API ajoutée (pub) : <fichier::fn/champ> — <une ligne chacune>
Écarts : DEVIATIONS.md #<n> <titre> | aucun
Différentiel : <fixtures comparées> — zéro diff | diffs couverts par #<n>
Porte de qualité : fmt OK / clippy OK / test OK (<nb> tests)
Besoins hors propriété : <fichier — API demandée — pour quel test> | aucun
Points d'attention pour la fusion : <conflits probables, décisions à valider> | aucun
```

Un rapport « PARTIEL » est acceptable (commits intermédiaires poussés sur la branche) à condition de
lister précisément ce qui reste et pourquoi.

## 7. Prompt de lancement (utilisé par l'orchestrateur / le skill `banditrs-dispatch`)

```
Tu es l'agent du lot WP-xx de BanditRS (réécriture Rust de bandit, dépôt courant).
Ta fiche : docs/plan/wp/WP-xx-<slug>.md. Procédure : docs/plan/agent-playbook.md (à suivre
intégralement, y compris la porte de qualité §4 et le rapport §6).
Règles : docs/plan/README.md §2 et §5 — tu ne modifies que les fichiers possédés par WP-xx.
Référence Python : /home/user/bandit (code) et /home/user/.pyenv-bandit (exécutable).
Travaille dans ce worktree sur la branche wp/WP-xx-<slug> (crée-la). Commits petits, préfixés
"WP-xx:". Termine par le rapport de fin au format exact du playbook §6.
```
