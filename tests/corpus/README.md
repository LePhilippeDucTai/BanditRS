# Real-world corpus

`manifest.tsv` pins a set of PyPI source distributions by sha256. The sources
themselves are **not** committed: `scripts/corpus.py fetch` downloads them into
`target/corpus/` (gitignored) and refuses anything whose hash does not match.

```
scripts/corpus.py list   --tier standard    # what is in it, and why each entry is there
scripts/corpus.py fetch  --tier standard    # download + verify + extract (~200 MB)
scripts/corpus.py verify --parse            # integrity, and the precondition below
scripts/corpus.py stats                     # per-tier totals
```

## Why sdists and not git clones

A PyPI artifact is immutable once published, so a sha256 fixes the exact bytes
for good, and identifies the same file on any mirror if the URL ever rots. A git
checkout drifts with its branch and drags its history along.

## Tiers

| tier | what it is for |
|---|---|
| `smoke` | 8 packages, ~570 files. Small enough for `scripts/check.sh parity` and the per-push CI job. |
| `standard` | +16 packages, ~11 600 files. The weekly CI run and the figures in `docs/drop-in-parity.md`. |
| `full` | +12 packages, ~21 500 files in total. A campaign, run by hand. |
| `frontier` | Packages requiring Python 3.12. Not a parity target — it measures what the parser's target version changes (DEVIATIONS.md #16). |

The first three are cumulative: `--tier standard` includes `smoke`.

## The precondition that makes the whole thing mean something

`verify --parse` requires every file in the parity tiers to be accepted by the
**reference interpreter's own** `ast`. Without it, a divergence could always be
waved away as a parser-version artefact. With it, any divergence the differential
finds is BanditRS's.

A handful of files are rejected on purpose — a package shipping a deliberately
invalid fixture, a Jinja template with a `.py` name, Python 2 helper scripts.
Those are listed with their reason in `parse-exceptions.txt`. They are not
excluded from the scan: the differential still checks that **both tools reject
them identically**, via the `errors[]` comparison.

## Choosing a version

`scripts/corpus.py seed` resolves a `"auto"` pin to the newest release whose
`Requires-Python` admits 3.11, then freezes it in the manifest. Where a specific
release matters — an LTS line, or the newest one that still parses under 3.11 —
the pin is written out explicitly in the `CURATED` list, with the reason.
