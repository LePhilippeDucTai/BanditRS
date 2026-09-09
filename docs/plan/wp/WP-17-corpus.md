# WP-17 — Corpus de code réel, épinglé et reproductible

**Vague** C (jalon J5) · **Fichiers** : `scripts/corpus.py`, `tests/corpus/manifest.tsv`, `tests/corpus/parse-exceptions.txt`

## Objectif

Jusqu'à J4, la parité était prouvée sur `examples/` (94 fixtures écrites *pour tester bandit*) et sur la
stdlib CPython. Aucun de ces deux corpus ne ressemble au code que l'outil rencontre réellement. Ce lot
fournit un corpus de librairies tierces, reproductible à l'octet près et téléchargeable à la demande.

## Choix de conception

- **Sdists PyPI épinglés par sha256**, pas des clones git : un artefact PyPI est immuable une fois publié,
  donc l'empreinte fixe les octets pour de bon et permet de repasser par n'importe quel miroir si l'URL
  disparaît. Un clone git dérive avec sa branche et traîne son historique.
- **Rien n'est committé sauf le manifeste** (`/target` est déjà gitignoré). Le dépôt reste léger, la
  reproductibilité vient de l'empreinte.
- **Quatre tiers** : `smoke` (8 paquets, ~570 fichiers — tient dans la porte de qualité), `standard`
  (+16, ~11 600 fichiers), `full` (campagne), et `frontier`.
- **`frontier` est à part** : des paquets délibérément écrits en syntaxe 3.12+, que le CPython 3.11 de
  l'exécutable de référence ne sait pas analyser. Ils *doivent* diverger ; le tier existe pour chiffrer
  cet écart (DEVIATIONS #16) au lieu de le laisser polluer un verdict de parité.

## Le critère qui porte tout le reste

`scripts/corpus.py verify --parse` exige que **chaque fichier** des tiers de parité soit accepté par
l'`ast` de l'interpréteur de référence. Sans cette garantie, une divergence pourrait toujours s'expliquer
par un écart de version de parseur plutôt que par un défaut de BanditRS ; avec elle, toute divergence est
imputable à BanditRS. Les rares fichiers volontairement invalides livrés par un paquet (fixture de test)
sont listés, avec leur raison, dans `tests/corpus/parse-exceptions.txt` — le différentiel vérifie par
ailleurs que les deux outils les rejettent *identiquement*.

## Critères d'acceptation

- `scripts/corpus.py fetch --tier standard` puis `verify --parse` : 0 échec.
- Le manifeste contient, pour chaque entrée, version, URL, sha256, date de publication, nombre de
  fichiers et de lignes, `Requires-Python`, et la raison de sa présence.
- Une seconde exécution de `fetch` ne modifie pas le manifeste (compteurs stables).
