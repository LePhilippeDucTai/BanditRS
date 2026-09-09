# WP-20 — Benchmarks sur code réel (extension de WP-15)

**Vague** C (jalon J5) · **Fichiers** : `scripts/bench_corpus.py`, `benches/e2e.rs`, `docs/plan/benchmarks.md` (§7)

## Objectif

`examples/` (94 fichiers minuscules écrits pour déclencher tous les plugins) et la stdlib (inhabituellement
pauvre en findings) sont non représentatifs, dans deux directions opposées. Mesurer sur du vrai code
applicatif, et produire les quatre mesures que le protocole de `benchmarks.md` nomme mais qu'aucun script
ne produisait.

## Livrables

1. `scripts/bench_corpus.py` — vitesse par paquet et agrégée, même protocole que `bench_vs_python.sh`
   (médiane de N après une chauffe), plus :
   - **mémoire de pointe dans le harnais** : `/usr/bin/time -v` est absent de la machine de référence, donc
     `getrusage(RUSAGE_CHILDREN)` avec un enfant dédié par mesure — un enfant partagé rapporterait le
     maximum courant de *tous* les enfants déjà moissonnés ;
   - **passage à l'échelle des threads** (`RAYON_NUM_THREADS` 1/2/4) avec l'ajustement d'Amdahl ;
   - **démarrage à froid en distribution** (p50/p90/max), pas en médiane : c'est le cas d'usage éditeur et
     pre-commit, celui où le démarrage domine ;
   - **coût des neuf formatters** sur un gros rapport (seuls `json` et `sarif`, sur `examples/`, l'étaient).
2. `benches/e2e.rs` — deux benchs criterion sur le corpus, à *skip silencieux* quand il n'est pas
   téléchargé (même motif que `scan_stdlib_3_11`) : un paquet dense en findings, et le plus gros fichier
   quasi généré de `botocore` (isole le débit parseur/walker du coût des plugins).
3. `docs/plan/benchmarks.md` §7 — résultats, et réconciliation des deux chiffres stdlib contradictoires
   committés jusqu'ici (49,9× en §5, 86,9× dans le README).

## Critères d'acceptation

- Le tableau §7 est rempli avec la machine et les conditions écrites.
- `cargo bench --no-run` compile sur une machine sans corpus.
