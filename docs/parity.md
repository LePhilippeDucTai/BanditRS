<!-- The methodology behind README §5. The README keeps the results; this file keeps the how. -->

# How parity is proven

Relying on a single mode of proof means relying on its blind spots. BanditRS layers four of them, sharing no
assumptions: one reads the specification, one queries the living oracle, one freezes the past, and one takes
the tool out of the laboratory.

> **Analogy.** A new scale can be checked in four ways: by re-reading its manual (the test suite), by
> comparing it against a reference scale (the differential), by re-weighing every morning the same reference
> weight kept in the drawer (the golden corpus), and by weighing the crates the warehouse actually ships
> (real third-party code). The third is the only one that still works when the reference scale is no longer
> in the room; the fourth is the only one that weighs what a customer will.

## 1. The Python test suite, ported test by test

The **273 tests** in `bandit/tests/` served as the acceptance specification. The porting rule: *one Python
test ⇒ one Rust test with the same name, in a mirror file*.

| | Count |
|---|---:|
| Reference Python tests | 273 |
| → ported as-is | 225 |
| → adapted (Python mocks replaced by real fixtures) | 38 |
| → not portable (pure Python introspection: `deepgetattr`, `meta_ast`…) | 10 |
| Rust tests **without** a Python counterpart (`argparse` emulation, and the CLI matrix replay of [§5](#5-real-third-party-code-and-the-whole-command-line-surface)) | 4 |
| **Total Rust tests** (67 unit + 287 integration) | **354** |
| Remaining `#[ignore]` tests | **0** |

The line-by-line inventory is in [`docs/plan/test-inventory.md`](plan/test-inventory.md); the 10
non-portable ones are justified one by one ([deviation #8](../README.md#6-remaining-differences-deliberate)).

## 2. Functional coverage: 75 test IDs out of 75

The set of test IDs is compared against the enumeration of the Python extension loader:

```console
$ diff <(python -c "…extension_loader.MANAGER…") <(grep -ohE 'B[0-9]{3}' src/core/registry.rs src/core/blacklist.rs | sort -u)
$ echo $?
0
```

<details>
<summary><b>The 75 IDs (42 plugins + 33 blacklist entries)</b></summary>

| Family | IDs |
|---|---|
| **B1xx** — miscellaneous | `B101` `B102` `B103` `B104` `B105` `B106` `B107` `B108` `B110` `B112` `B113` |
| **B2xx** — application | `B201` `B202` |
| **B3xx** — call blacklist | `B301` `B302` `B303` `B304` `B305` `B306` `B307` `B308` `B310` `B311` `B312` `B313` `B314` `B315` `B316` `B317` `B318` `B319` `B321` `B323` `B324` |
| **B4xx** — import blacklist | `B401` `B402` `B403` `B404` `B405` `B406` `B407` `B408` `B409` `B411` `B412` `B413` `B415` |
| **B5xx** — crypto & certificates | `B501` `B502` `B503` `B504` `B505` `B506` `B507` `B508` `B509` |
| **B6xx** — injection | `B601` `B602` `B603` `B604` `B605` `B606` `B607` `B608` `B609` `B610` `B611` `B612` `B613` `B614` `B615` |
| **B7xx** — templating | `B701` `B702` `B703` `B704` |

</details>

The same goes for the CLI surface: the **27 long options** of `bandit --help` are exactly those of BanditRS,
including the mutual exclusions (`-v`/`-q` rejected with the same exit code `2`).

`bandit-baseline`'s `argparse` emulation is locked down byte-for-byte by a dedicated test — the only Rust test
without a Python counterpart (`argparse` belongs to the standard library, so bandit's suite does not test it,
and that is precisely why a discrepancy could slip in there):

| Invocation | Python | BanditRS |
|---|---|---|
| `bandit-baseline --help` / `-h` | help on stdout, exit `0` | **identical** |
| `bandit-baseline` (no target) | `usage:` + `bandit-baseline: error: the following arguments are required: targets` on stderr, exit `2` | **identical** |
| `bandit-baseline -f bogus .` | `usage:` + `bandit-baseline: error: argument -f: invalid choice: 'bogus' …`, exit `2` | **identical** |

## 3. Differential against Python bandit

[`scripts/diff_against_python.sh`](../scripts/diff_against_python.sh) runs both binaries file by file,
normalizes the volatile fields, and **fails** if a file diverges for a reason that is not on the allow-list in
`DEVIATIONS.md`.

| Corpus | Files | Identical JSON reports | Unexpected divergences |
|---|---:|---:|---:|
| `examples/` (bandit fixtures) | 94 | **94** | **0** |
| `/usr/lib/python3.11` (CPython stdlib) | 672 | **672** | **0** |
| `bandit/` (bandit's own source code) | 69 | **69** | **0** |
| AST walk traces (order of visited nodes) | 91 | **91** | **0** |
| Aggregate reports over `examples/`, 8 formats | 8 | **7** | 1 — `yaml`, PyYAML anchors (see [README §2](../README.md#2-is-the-output-really-identical)) |

```console
$ scripts/diff_against_python.sh examples
walk traces: identical=91 different=0
json identical: examples          txt identical: examples
csv identical: examples           xml identical: examples
custom identical: examples        sarif identical: examples
html identical: examples          yaml DIFF: examples (PyYAML anchors — DEVIATIONS #11)
file-by-file JSON: identical=94 whitelisted=0 unexpected=0 total=94
diff_against_python.sh: zero unexpected diff

$ scripts/diff_against_python.sh --stdlib
file-by-file JSON: identical=672 whitelisted=0 unexpected=0 total=672
diff_against_python.sh: zero unexpected diff
```

Among the fields this normalization neutralizes, one case is worth showing, because the divergence is on
**Python's side**:

```diff
# examples/tarfile_extractall.py, issue B202
-"issue_text": "… members were properly validated {'Other': <ast.List object at 0x7fc651c4e170>})."
+"issue_text": "… members were properly validated {'Other': <ast.List object at 0x0>})."
```

That message interpolates the `repr()` of an AST node, which contains the Python object's **memory address**:
bandit's output is therefore not reproducible from one run to the next, nor comparable across two machines.
BanditRS renders `0x0` to produce deterministic output
([deviation #5](../README.md#6-remaining-differences-deliberate)).

## 4. Golden corpus: parity without Python

The two modes above assume a Python interpreter and an installed `bandit`. So that parity stays verifiable
*ad vitam* — in CI, on a bare machine, five years from now — the reference outputs are committed:

```
tests/golden/          96 files, 3.7 MB
├── examples.{json,txt,csv,xml,yaml,html,sarif,custom}   aggregate reports over all of examples/
├── files/                                               86 JSON reports, one per fixture
└── normalize.rs                                         the normalization, in a single place
```

```bash
cargo test --test golden       # 9 tests, replays the whole corpus, no Python required
```

This test is what turns parity from a *one-off observation* into a **regression invariant**: any change to the
engine that would move an issue, a column or a byte of a report makes the suite fail.

## 5. Real third-party code, and the whole command-line surface

The three modes above share a blind spot: `examples/`, the stdlib and bandit's own source are all code that
was *written to test bandit* or that bandit's authors already ran. And all three compare a single thing — the
`-f json` report on stdout, from the default invocation. A drop-in replacement has to match more than that.

**Real libraries.** [`tests/corpus/manifest.tsv`](../tests/corpus/manifest.tsv) pins 38 PyPI source
distributions by sha256 (they are never committed; `scripts/corpus.py fetch` downloads and verifies them).
Both tools scan each package with `bandit -r <pkg> -f json -q`, and *identical* means the same issues in the
same order, with the same file, line, column range, severity, confidence and message; the same `errors[]`;
and the same `metrics` block — which also proves both discovered the same files with the same line counts.

The tiers are cumulative — `smoke` is what CI runs on every push, `full` is the whole thing:

| Tier | Packages | Files | Lines | Issues | Identical |
|---|---:|---:|---:|---:|---|
| `smoke` (requests, Flask, Jinja2, urllib3…) | 8 | 572 | 176,043 | 7,796 | **8/8** |
| `standard` (+ Django, numpy, pandas, SQLAlchemy, ansible…) | 24 | 11,624 | 3,640,954 | 73,577 | **24/24** |
| `full` (+ transformers, salt, scipy, matplotlib, airflow…) | 36 | 21,536 | 7,170,763 | 130,730 | **36/36** |

Three of those packages ship files CPython refuses on purpose — salt's Jinja scaffolding named
`{{module_name}}.py`, pexpect's Python 2 helpers, black's `tests/data/` museum of pathological syntax. Both
tools reject them, and the `errors[]` comparison checks they reject them *identically*, which is the
interesting half.

**The whole option surface.** [`scripts/cli_matrix.py`](../scripts/cli_matrix.py) runs **88 invocations**
through both binaries and compares **stdout, stderr *and* the exit code** — the two streams and the one
number a CI job actually branches on. It covers the severity and confidence thresholds, test selection and
skipping, profiles and config files, all nine formatters, `--msg-template`, `-o`, context lines,
aggregation, `-q`/`-v`/`-d`, `--exit-zero`, `--ignore-nosec`, target discovery (recursive or not, excludes,
globs, symlinks, missing paths, empty directories, non-UTF-8 and unparsable files), `.bandit` handling,
every usage error that exits 2, baselines, and the `bandit-baseline` / `bandit-config-generator` binaries.

Result: **85/88 identical**, the 3 remaining being deviations [#10, #18, #19](../README.md#6-remaining-differences-deliberate).
On its first pass the matrix found **six real divergences**, all fixed — the one worth naming is that
`-ll`/`-lll`/`-ii`/`-iii` filtered one level too low, because argparse's `action="count"` increments *from*
`default=1`; `bandit -ll` is the common CI invocation, so it was silently reporting the wrong set of issues.
Nothing else in this repository observed it: the Python suite does not test its own argument parser.

```bash
cargo test --test cli_matrix     # replays the 88 recorded invocations, no Python required
```

**What it exercises.** The corpus triggers **60 of the 75 test ids**; the other 15 (telnetlib, the XML
family, SNMP…) are code no maintained library still writes, and are covered by the `examples/` golden
corpus instead — so **0 ids go unexercised**. Parity on code that never wakes a plugin would prove little,
which is why [`docs/drop-in-parity.md`](drop-in-parity.md) names them one by one rather than averaging
them away. That report is *generated* by `scripts/parity_report.py` from the differential's own JSON output,
never edited by hand.
