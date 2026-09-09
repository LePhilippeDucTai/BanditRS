# WP-19 — Rapport de preuve et couverture réelle

**Vague** C (jalon J5) · **Fichiers** : `scripts/diff_corpus.py`, `scripts/parity_report.py`, `docs/drop-in-parity.md`

## Objectif

Produire l'artefact que l'on montre quand on écrit « drop-in replacement » : des chiffres reproductibles,
pas un adjectif.

## Forme du différentiel : agrégat d'abord, par fichier seulement en cas d'écart

`scripts/diff_against_python.sh` forke les deux binaires **une fois par fichier**. Sur 11 600 fichiers,
à ~0,2 s de démarrage Python par fichier, cela coûterait plus de 30 minutes rien qu'en `fork`/`exec`.
`scripts/diff_corpus.py` inverse la logique : une exécution `-r <paquet> -f json` par outil et par paquet
(quelques dizaines de forks), puis une comparaison structurée — `results[]` dans l'ordre, `errors[]`, et
le bloc `metrics`, qui prouve au passage que les deux outils ont découvert exactement le même ensemble de
fichiers avec les mêmes compteurs de lignes. La comparaison par fichier n'est utilisée qu'en bissection,
à l'intérieur d'un paquet divergent, pour nommer le coupable.

## La question qu'il est tentant de sauter

Une parité sur du code qui ne réveille aucun plugin ne prouve presque rien. Le rapport chiffre donc
**combien des 75 identifiants le corpus déclenche réellement**, et surtout nomme ceux qu'il ne touche
jamais, en disant pour chacun s'il est couvert ailleurs (corpus golden `examples/`) ou s'il reste un
angle mort. La liste des identifiants chargés est demandée au binaire lui-même (`--help`), pas
re-dérivée de la source, pour qu'elle ne puisse pas dériver.

## Critères d'acceptation

- `scripts/diff_corpus.py --tier standard` : 0 divergence inattendue, code de sortie 0.
- `docs/drop-in-parity.md` généré, avec le tableau par paquet, l'accord de la matrice CLI, la couverture
  des identifiants (angles morts nommés) et le tier `frontier` chiffré à part.
