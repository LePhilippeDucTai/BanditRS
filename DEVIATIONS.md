# Écarts délibérés par rapport à bandit (Python)

Politique validée : parité stricte sur tout comportement observé par la suite de tests ; correction des bugs
connus uniquement quand aucun test n'en dépend. Tout le reste (coquilles dans les messages, inversion
`bad_calls`/`bad_imports` de la conversion legacy — testée —, premier match des blacklists, « premier hit » du
nosec sur `linerange`, absent ≠ `None` dans `check_call_arg_value`) est reproduit à l'identique.

1. `# nosec B101,B102` (virgule sans espace) : Python n'ignore que le dernier id ; BanditRS prend tous les ids.
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
