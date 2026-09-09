# Écarts délibérés par rapport à bandit (Python)

Politique validée : parité stricte sur tout comportement observé par la suite de tests ; correction des bugs
connus uniquement quand aucun test n'en dépend. Tout le reste (coquilles dans les messages, inversion
`bad_calls`/`bad_imports` de la conversion legacy — testée —, premier match des blacklists, « premier hit » du
nosec sur `linerange`, absent ≠ `None` dans `check_call_arg_value`) est reproduit à l'identique.

1. ~~`# nosec B101,B102` (virgule sans espace) : Python n'ignore que le dernier id ; BanditRS prend tous les
   ids.~~ **Résorbé le 2026-09-09 (J5).** La matrice différentielle CLI (WP-18) a montré que BanditRS
   découpait les jetons d'un commentaire `nosec` sur les espaces et les virgules, là où Python itère
   `NOSEC_COMMENT_TESTS = (?:(B\d+|[a-z\d_]+),?)+` et prend `group(1)` : la ponctuation est ignorée
   (`#nosec (on the line)` donne `on`, `the`, `line`, jamais `(on`) et un groupe répété ne conserve que sa
   **dernière** répétition (`B101,B102` donne le seul jeton `B102`). Le crate `regex` reproduit les deux
   comportements à l'identique ; `src/core/nosec.rs` porte donc désormais la regex telle quelle et il n'y a
   plus d'écart — ni sur les ids retenus, ni sur les avertissements `Test in comment: …` de stderr.
2. Formatter CSV : un test sans CWE (NOTSET) plante en Python (`KeyError: 'link'`) ; BanditRS écrit une colonne vide.
3. Fichier `.bandit` (INI) : `level`, `confidence`, `number` sont convertis en entiers (Python garde des chaînes et
   plante) ; la clé `configfile` est réellement honorée (Python lisait la mauvaise clé de défaut).
4. `discover_files` ne modifie plus la configuration chargée (Python accumulait les exclusions entre appels).
5. Fichiers dépassant la limite de récursion Python (`RecursionError` → « exception while scanning file ») :
   analysés normalement (walker itératif) ; les adresses mémoire dans certains messages (`<ast.List object at
   0x…>` de B202) sont rendues de façon déterministe (`0x0`).
6. Pas de barre de progression `rich` ; `--version` affiche `bandit <version du crate>` ; la ligne
   « running on Python x.y.z » n'est pas émise.
7. Positions des constantes de f-strings : sémantique CPython ≥ 3.12 par défaut (`BANDITRS_PYTHON_COMPAT=3.11`
   pour la sémantique 3.11).
8. Tests Python non portables (introspection : `get_path_for_function`, `deepgetattr`, `check_ast_node`,
   `meta_ast`) : remplacés par des équivalents Rust quand ils existent, sinon omis.
9. `django_mark_safe` (B703) : `DeepAssignation` est désormais porté intégralement — `try`/`with`/`for`/`while`/
   `ExceptHandler` et le déballage de tuple (`a, b = ...`) sont tous parcourus, comme côté Python. La limitation
   d'origine (affectations directes en ligne droite seulement) n'existe plus. Il subsiste deux divergences,
   volontaires, où BanditRS est plus strict que Python parce qu'il ne reproduit pas deux bugs de
   `django_xss.py` :
   - Cas d'une fonction imbriquée qui prend la variable en paramètre puis la réaffecte dans son corps :
     `is_assigned`, branche `FunctionDef`, fait `for name in node.args.args: if isinstance(name, ast.Name):
     ... return assigned` — or `args.args` contient des `ast.arg`, jamais des `ast.Name` ; la condition est
     donc **toujours fausse** et Python ne court-circuite jamais sur un paramètre de fonction imbriquée : il
     descend systématiquement dans le corps de la fonction, y trouve l'affectation censée être « sûre » et ne
     remonte pas d'issue (0 B703). BanditRS teste réellement l'appartenance à `parameters.args` (`is_param`) et
     court-circuite correctement, donc considère la variable non résolue et remonte l'issue (1 B703).
   - Cas d'un déballage de tuple imbriqué (`a, (b, c) = ...`) : côté Python, `is_assigned` fait
     `for name in target.elts: if name.id == ...` sans vérifier que chaque élément est un `ast.Name` ; un
     élément qui est lui-même un `ast.Tuple` n'a pas d'attribut `.id`, ce qui lève une `AttributeError` — avalée
     par le testeur (`bandit/core/tester.py`), qui journalise l'exception et n'émet aucune issue pour ce nœud
     (0 B703). BanditRS traite un élément non-`Name` de façon défensive (retourne « non assigné ») au lieu de
     paniquer, et l'analyse se poursuit normalement jusqu'à remonter l'issue (1 B703).
   Aucun test de la suite ni aucune fixture (`mark_safe.py`, `mark_safe_insecure.py`, `mark_safe_secure.py`) n'exerce
   ces deux cas ; ils ne sont visibles qu'en construisant un fichier ad hoc et en comparant à l'exécutable Python
   de référence.
10. Formatter YAML (`pycompat::yaml_emit`) : le pliage à 80 colonnes (`write_plain`/`write_single_quoted`) est
    reproduit exactement (algorithme de `emitter.py` vérifié empiriquement contre PyYAML 6.0.1), mais les scalaires
    en style double-quoted (texte avec caractères non-ASCII/de contrôle, rare dans les données de bandit) ne sont
    **pas** repliés — une seule ligne physique, sans le découpage `\`-continué de PyYAML. Sans impact sur la suite
    de tests (`tests/formatters.rs` §C.7, valeurs relues, pas de comparaison octet à octet).
11. Formatter YAML : PyYAML déduplique par **identité d'objet Python** (`id()`) — quand deux `Issue` du même nœud
    partagent la même liste `linerange` par référence (observé quand plusieurs plugins déclenchent sur le même
    nœud), il émet une ancre/alias YAML (`&id001` / `*id001`) au lieu de répéter la valeur. BanditRS n'a pas de
    notion d'identité d'objet partagée (`LineRange` est `Copy`) et répète toujours la valeur littérale — le contenu
    relu est identique, seule la représentation texte diffère (visible dans `scripts/diff_against_python.sh` sur
    `wildcard-injection.py`/`partial_path_process.py`/`nosec.py`, jamais dans un test unitaire).
12. URLs de documentation (`more_info`) : `DOCS_VERSION` vaut `"latest"` (`src/lib.rs`), donc BanditRS émet
    `https://bandit.readthedocs.io/en/latest/plugins/…`. Python construit la même URL à partir de la version
    **installée** du paquet (`https://bandit.readthedocs.io/en/0.0.1.dev49/…` avec la référence de ce dépôt).
    Choix délibéré (PLAN.md §1) : la version du crate Rust n'a pas de page readthedocs correspondante, et
    pointer une version figée périmerait les liens. Visible sur toute sortie contenant `more_info` (json, csv,
    xml, yaml, html, txt, sarif) ; le harnais différentiel normalise ce champ. Aucun test de la suite ne
    dépend du numéro de version dans l'URL.
13. `Context::statement()` (`src/core/context.rs`) : en Python, `Context.statement` lit
    `self._context.get("statement")`, une clé que `node_visitor.py` n'écrit jamais — la propriété vaut donc
    toujours `None` en production (aucun plugin ne la lit non plus) ; le test Python ne fait que vérifier le
    getter du dict avec une valeur simulée. Le `Context` Rust n'a pas de dict non typé à simuler ; WP-09
    (`docs/plan/wp/WP-09-unit-core-context.md`) donne donc à `statement()` une vraie implémentation — le plus
    proche ancêtre de type `Stmt` — pour que la propriété soit testable avec un extrait réel. Sans impact
    observable : `statement()` n'est utilisé par aucun plugin ni aucune sortie.
14. ~~`apply_ini_options` (`src/cli/main.rs`) ne journalise « Using command line arg for selected targets »
    que si la clé `targets` est présente dans le `.bandit` (INI).~~ **Résorbé le 2026-09-09 (J5).** Le cas
    `ini_explicit` de la matrice CLI (WP-18) l'observe. `parser.get_default("targets")` vaut `None` pour un
    `nargs="*"`, donc `_log_option_source` prend sa branche « défaut non défini » : le message est émis dès
    que des cibles sont passées en ligne de commande, et la valeur de l'ini n'est consultée que s'il n'y en a
    aucune. Porté tel quel.
15. ~~`src/cli/baseline.rs::name_rev()` rend `master` là où Python (`commit.name_rev`) rend `<sha> master`
    dans le message « Got current/parent commit: … ».~~ **Résorbé le 2026-09-09 (J5).** Le cas
    `baseline_bin_run` de la matrice CLI l'observe. `commit.name_rev` de GitPython est `git name-rev <sha>`,
    dont la sortie est « <sha> <nom> » ; le `--name-only` du portage Rust supprimait la moitié `<sha>`.

## Écarts constatés au jalon J5 (matrice CLI et corpus réel)

16. **Plancher de syntaxe.** BanditRS analyse avec `ruff_python_parser` (syntaxe Python 3.13) ; bandit Python
    analyse avec l'`ast` de l'interpréteur hôte, soit 3.11 pour l'exécutable de référence de ce dépôt. Sur un
    fichier écrit en syntaxe 3.12+ (PEP 695 `type X = …`, paramètres de type génériques), Python remonte une
    `SyntaxError` dans `errors[]` et n'analyse rien, là où BanditRS analyse le fichier normalement et peut
    donc remonter des issues supplémentaires. Mesuré : `django 6.1.1` (6 fichiers sur 2 907),
    `ansible-core 2.21.4` (28 sur 1 802). Ce n'est pas un défaut de parité mais une différence de plancher :
    le tier `frontier` du corpus (`tests/corpus/manifest.tsv`) l'isole et le chiffre, et les tiers de parité
    sont épinglés à des versions dont `scripts/corpus.py verify --parse` garantit qu'elles passent sous 3.11.
17. **Ordre des identifiants dans « profile include/exclude tests ».** `_log_info` joint un `set` Python, dont
    l'ordre d'itération dépend du hachage randomisé des chaînes : cinq exécutions identiques du même
    `bandit -c cfg/profiles.yml -p ShellInjection` ont donné trois ordres différents. BanditRS itère une
    collection déterministe. C'est Python qui n'est pas reproductible ici ; le harnais trie les deux côtés
    avant comparaison (`scripts/cli_matrix.py`).
18. **Sortie de `-d` (debug).** Python journalise, pour chaque nœud visité, le dictionnaire `Context` complet,
    y compris le `repr` des objets `ast` CPython (`<ast.Name object at 0x…>`) — 17 514 lignes contre 38 sur
    l'espace de travail de la matrice. BanditRS n'a pas de graphe d'objets Python à imprimer ; reproduire ce
    dump octet à octet supposerait d'émuler le `repr` de chaque type de nœud CPython et la disposition exacte
    du dictionnaire, pour une sortie qui est une aide au débogage et non une interface exploitable. Les
    lignes de journal `INFO`/`WARNING`/`ERROR`, elles, sont identiques (cas `debug` de la matrice).
19. **Texte des erreurs de parsing YAML.** Une configuration YAML invalide produit le message de PyYAML
    (« while parsing a flow sequence / expected ',' or ']' », avec ligne et colonne) ; BanditRS produit celui
    de `saphyr-parser` (« illegal placement of ':' indicator at byte … »). Même comportement (erreur
    journalisée par `[config] ERROR`, code de sortie 2), seul le libellé de l'analyseur diffère.
