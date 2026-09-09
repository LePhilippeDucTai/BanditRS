# Corpus golden (WP-14)

Fige les sorties de la référence Python (`bandit 0.0.1.dev49`, dépôt upstream
`1d3053d`) pour `examples/` : `tests/golden/examples.<fmt>` (huit
formats, `bandit -r examples -f <fmt>`) et `tests/golden/files/<nom>.json`
(`bandit examples/<nom> -f json`, un par fixture).

Régénéré le 2026-09-09 avec :

```
PY_BANDIT=/home/user/.pyenv-bandit/bin/bandit scripts/gen_golden.sh
```

`cargo test --test golden` rejoue ce corpus contre le binaire Rust
(`BANDITRS_PYTHON_COMPAT=3.11`) après la même normalisation
(`tests/golden/normalize.rs`) : aucune installation Python n'est requise
pour ce test. Ne régénérer que si upstream ou une entrée de `DEVIATIONS.md`
change ; le diff du commit doit alors montrer précisément ce qui change.
