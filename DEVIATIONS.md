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
9. `django_mark_safe` (B703) : `DeepAssignation` ne reproduit que les affectations directes en ligne droite
   (`Assign` vers un `Name` simple dans le corps de la fonction/du module courant). La version Python parcourt
   aussi `try`/`with`/`for`/`while` et les affectations par déballage de tuple. Les fixtures qui dépendent de ce
   comportement (`mark_safe_secure.py`, `mark_safe_insecure.py`) sont hors de la suite fonctionnelle de base
   (profil avec `exclude B308`, cf. PLAN.md M4) ; `mark_safe.py` (suite de base) n'exerce que le cas simple.
10. Formatter YAML (`pycompat::yaml_emit`) : le pliage à 80 colonnes (`write_plain`/`write_single_quoted`) est
    reproduit exactement (algorithme de `emitter.py` vérifié empiriquement contre PyYAML 6.0.1), mais les scalaires
    en style double-quoted (texte avec caractères non-ASCII/de contrôle, rare dans les données de bandit) ne sont
    **pas** repliés — une seule ligne physique, sans le découpage `\`-continué de PyYAML. Sans impact sur la suite
    de tests (`tests/formatters.rs` §C.7, valeurs relues, pas de comparaison octet à octet).
