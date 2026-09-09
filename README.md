<div align="center">

# BanditRS

**The Python security scanner [`bandit`](https://github.com/PyCQA/bandit), rewritten in pure Rust.**
Same tests, same options, same outputs, same exit codes — **17× to 86× faster**,
with **57% less memory**.

[![CI](https://github.com/LePhilippeDucTai/BanditRS/actions/workflows/ci.yml/badge.svg)](https://github.com/LePhilippeDucTai/BanditRS/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.96%2B-B7410E?logo=rust&logoColor=white)](rust-toolchain.toml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-354%20✓%20(0%20ignored)-success)](#5-how-parity-is-proven)
[![Parity](https://img.shields.io/badge/parity-75%2F75%20tests%20B1xx–B7xx-success)](#5-how-parity-is-proven)
[![Differential](https://img.shields.io/badge/differential-21k%20files%20of%20real%20code%20%C2%B7%200%20unexpected%20diff-success)](#5-how-parity-is-proven)
[![PyPI](https://img.shields.io/pypi/v/banditrs.svg)](https://pypi.org/project/banditrs/)

</div>

---

## Install

BanditRS is on [PyPI](https://pypi.org/project/banditrs/) as prebuilt wheels for six
platforms — no Rust toolchain needed, nothing to compile. Fastest way to try it, nothing
installed permanently:

```bash
uvx banditrs -r .
```

To keep it around — `bandit`, `banditrs`, `bandit-baseline` and `bandit-config-generator` all
land on your `PATH`:

```bash
uv tool install banditrs
bandit -r .
```

`pip` and `pipx` work the same way:

```bash
pip install banditrs      # or: pipx install banditrs
bandit -r .
```

Platform wheel names and building from source: [§7](#7-installation-details).

---

## Usage

Every `bandit` invocation works as-is — same options, same output, same exit codes:

```bash
bandit -r my_project/                          # recursive scan
bandit -r my_project/ -f json -o report.json   # JSON report
bandit -r my_project/ -ll -ii                  # severity ≥ MEDIUM, confidence ≥ MEDIUM
bandit -r my_project/ -s B101,B601             # skip tests
bandit -r my_project/ -t B602                  # run a single test
bandit -r my_project/ -c bandit.yaml           # configuration profile
bandit -r my_project/ -b baseline.json         # show only new issues
bandit -r my_project/ --exit-zero              # never fail (report mode)
bandit --help                                  # all 27 options
```

Installing gives you four commands:

```
bandit                     drop-in replacement for the PyCQA command
banditrs                   identical alias (see below)
bandit-baseline            scan against a git baseline
bandit-config-generator    emit a configuration profile
```

```bash
bandit-config-generator --show-defaults > bandit.yaml   # generate a full profile
bandit-baseline -r my_project/                          # diff against the parent commit (requires git)
```

**Coexisting with PyCQA bandit.** The `bandit` command deliberately occupies the same file name as
the PyCQA package, so if both are installed in one environment the last one installed wins — silently.
The `banditrs` alias is byte-for-byte the same program under a name nothing else claims: use it to
compare the two side by side, or to migrate gradually, without uninstalling anything.

**In `pre-commit`.** Same invocation, same output, same exit code:

```yaml
# before
- repo: https://github.com/PyCQA/bandit
  rev: <tag>
  hooks: [{ id: bandit, args: ["-r", "src/"] }]

# after
- repo: https://github.com/LePhilippeDucTai/BanditRS
  rev: v0.2.1
  hooks: [{ id: banditrs, args: ["-r", "src/"] }]
```

pre-commit builds the hook in its own isolated environment, so this route compiles from source and
needs a Rust toolchain (`rustc` ≥ 1.96) on the machine running the hook.

**From Python.** The `banditrs` module is a thin launcher around the same executables — the analysis
is never reimplemented in Python. `python -m banditrs …` is equivalent to calling `bandit` directly:

```python
import banditrs

result = banditrs.run(["-r", "src/", "-f", "json"], capture_output=True, text=True)
result.returncode                           # non-zero when issues are found
```

---

## Contents

1. [In one minute](#1-in-one-minute)
2. [Is the output really identical?](#2-is-the-output-really-identical)
3. [Benchmarks](#3-benchmarks)
4. [Why it's faster](#4-why-its-faster)
5. [How parity is proven](#5-how-parity-is-proven)
6. [Remaining differences](#6-remaining-differences-deliberate)
7. [Installation details](#7-installation-details)
8. [Architecture and development](#8-architecture-and-development)
9. [License and credits](#9-license-and-credits)

---

## 1. In one minute

`bandit` walks the AST of a Python file and runs ~75 security tests (command injection, unsafe
deserialization, weak crypto, hardcoded passwords, vulnerable XML…). BanditRS does the same thing,
using the Python parser from [ruff](https://github.com/astral-sh/ruff) and per-file parallel analysis.

The goal is **not** "a linter inspired by bandit": it is a **drop-in replacement**. You can substitute the
binary in a `pre-commit` hook, a CI pipeline or a `Makefile` without changing a single option, and get
byte-for-byte the same output (modulo the exceptions documented in [§6](#6-remaining-differences-deliberate)).

| | `bandit` (Python) | **BanditRS** |
|---|---|---|
| Language / runtime | Python ≥ 3.10 + dependencies (PyYAML, stevedore, rich, GitPython…) | Rust, one native binary, zero runtime |
| Test IDs | 75 (42 plugins + 33 blacklists) | **the same 75** |
| Long command-line options | 27 | **the same 27** |
| Output formats | 9 (`csv`, `custom`, `html`, `json`, `sarif`, `screen`, `txt`, `xml`, `yaml`) | **the same 9** |
| Executables | `bandit`, `bandit-baseline`, `bandit-config-generator` | **the same 3**, plus the `banditrs` alias |
| Exit codes | `0` / `1` (issues) / `2` (usage error) | **identical** |
| `# nosec`, `.bandit`, `pyproject.toml`, profiles, baseline | ✅ | ✅ |
| Parallelism | no (one file after another) | yes (rayon, one file per task) |
| Scanning the CPython 3.11 stdlib (672 files, 307k lines) | 18.4 s — 75 MiB | **0.21 s — 33 MiB** |

---

## 2. Is the output really identical?

The decisive test for a drop-in replacement is not "does it find the same bugs", it is
"is the `diff` empty". Here is the answer, on a deliberately faulty file:

```python
# app.py
import subprocess
import yaml


def load(path):
    return yaml.load(open(path).read())


def run(user_input):
    subprocess.call("grep " + user_input, shell=True)


PASSWORD = "hunter2"
```

The `diff -u` of the two full outputs contains only **ten changed lines** — five pairs, four of which are
exactly the same pattern:

```diff
@@ timestamp (volatile on both sides) @@
-Run started:2026-09-09 09:53:52.239965+00:00
+Run started:2026-09-09 09:53:52.273576+00:00
@@ version in the documentation URL (deviation #12) @@
-   More Info: …/en/0.0.1.dev49/blacklists/blacklist_imports.html#b404-import-subprocess
+   More Info: …/en/latest/blacklists/blacklist_imports.html#b404-import-subprocess
-   More Info: …/en/0.0.1.dev49/plugins/b506_yaml_load.html
+   More Info: …/en/latest/plugins/b506_yaml_load.html
-   More Info: …/en/0.0.1.dev49/plugins/b602_subprocess_popen_with_shell_equals_true.html
+   More Info: …/en/latest/plugins/b602_subprocess_popen_with_shell_equals_true.html
-   More Info: …/en/0.0.1.dev49/plugins/b105_hardcoded_password_string.html
+   More Info: …/en/latest/plugins/b105_hardcoded_password_string.html
```

Everything else — issue ordering, severities, confidences, CWEs, **line:column** positions, code snippets,
tabs, separator widths, closing metrics, exit code — is byte-for-byte identical. The only two differences are
the timestamp (volatile by nature) and the version in the documentation URL
([deviation #12](#6-remaining-differences-deliberate), a deliberate choice).

The same exercise over the full fixture corpus (`examples/`, 94 files), in every report format, after
neutralizing the volatile fields only (timestamp, tool version in the doc URL). `screen` is `txt` plus
ANSI escapes and is covered by a unit test instead:

| Format | Report lines | Diverging lines | Verdict |
|---|---:|---:|---|
| `json` | 13,293 | **0** | identical |
| `txt` | 6,073 | **0** | identical |
| `csv` | 598 | **0** | identical |
| `xml` | 1,792 | **0** | identical |
| `html` | 13,915 | **0** | identical |
| `custom` | 597 | **0** | identical |
| `sarif` | 23,973 | 4 | the tool's own `"version"`/`"semanticVersion"` (`0.0.1.dev49` vs `0.2.0`) |
| `yaml` | 11,589 | 164 | PyYAML anchors/aliases (`&id001`/`*id001`) — [deviation #11](#6-remaining-differences-deliberate); content identical when re-read |

Six formats out of eight are **strictly identical**. The other two diverge only in their *representation*,
never in the data: SARIF announces its own version (which is the correct behaviour for two different tools),
and YAML forgoes a PyYAML optimization based on Python object identity (`id()`), which Rust structurally does
not have.

On real code the answer is the same: scanning bandit's own repository (69 files, 8,737 lines) with
both tools gives an **identical** JSON report — same single issue, same per-file metrics, same ordering.

---

## 3. Benchmarks

### 3.1 Protocol

No number in this section is an estimate. Everything is reproducible:

```bash
cargo build --release
scripts/bench_vs_python.sh -n 7                                       # corpus throughput
scripts/bench_vs_python.sh -n 15 examples/subprocess_shell.py examples/long_set.py
```

- **Machine**: 4 vCPU container (Intel Xeon @ 2.10 GHz), 15 GiB RAM, Linux 6.18.44-fc-v24.
- **Reference**: `bandit 0.0.1.dev49` (PyCQA repository @ `1d3053d`), CPython 3.11.15.
- **Candidate**: `banditrs 0.2.0`, `release` profile (fat LTO, `codegen-units = 1`).
- **Measurement**: median of N runs after one warm-up run, wall-clock time, `-f json -q` output redirected to
  `/dev/null` (we measure the analysis, not the terminal).
- **Memory**: `ru_maxrss` from `RUSAGE_CHILDREN`, one dedicated measuring process per reading.

The full protocol, the non-regression thresholds and the `callgrind` profile are in
[`docs/plan/benchmarks.md`](docs/plan/benchmarks.md).

### 3.2 Throughput

| Target | Files | Lines | `bandit` (Python) | **BanditRS** | Factor |
|---|---:|---:|---:|---:|---:|
| 36 real libraries ([§5.5](#5-how-parity-is-proven)) | 21,536 | 7,170,763 | 506.9 s | **8.3 s** | **61.0×** |
| `/usr/lib/python3.11` (CPython stdlib) | 672 | 307,504 | 18.40 s | **0.214 s** | **86.0×** |
| `examples/` (bandit fixtures) | 94 | 9,534 | 6.497 s | **0.341 s** | **19.0×** |
| `examples/long_set.py` (the largest, 65 KiB) | 1 | 7,279 | 6.247 s | **0.359 s** | **17.4×** |
| `examples/subprocess_shell.py` (a typical file) | 1 | 60 | 0.197 s | **0.006 s** | **32.3×** |

Cost per line on the stdlib: **60 µs/line** on the Python side against **0.70 µs/line** on the Rust side —
i.e. 17 kloc/s against 1.44 Mloc/s.

> **Why is `examples/` "only" 19× when the stdlib is 86×?**
> Because `examples/` is dominated by a single pathological file: `long_set.py` accounts for 6.25 s out of
> the corpus's 6.50 s on the Python side. Measuring `examples/` therefore essentially means measuring *one*
> file — and parallelism does not help on a single file. The 19× is the **pure sequential** gain; the 86× on
> the stdlib is that same gain multiplied by parallelism. Both numbers are consistent, see §3.5.

### 3.3 Per-library breakdown

§3.2 aggregates 21,536 files into one number. To check the gain isn't an artifact of one favorable
package dominating the average, here it is broken down **library by library**, on the eight
real-world packages of the `smoke` tier of [`tests/corpus/manifest.tsv`](tests/corpus/manifest.tsv) —
the same pinned sdists the differential test suite ([§5.5](#5-how-parity-is-proven))
runs both tools against, so a library that appears here is also proven byte-for-byte identical:

```bash
scripts/bench_corpus.py -n 5 --tier smoke      # target/bench/corpus.md
```

| Library | Files | Lines | `bandit` (Python) | **BanditRS** | Factor |
|---|---:|---:|---:|---:|---:|
| `Werkzeug 3.1.8` | 138 | 35,090 | 2.389 s | **0.055 s** | **43.4×** |
| `urllib3 2.7.0` | 81 | 32,241 | 2.511 s | **0.070 s** | **35.6×** |
| `Flask 3.1.3` | 83 | 17,889 | 1.212 s | **0.028 s** | **43.4×** |
| `Jinja2 3.1.6` | 52 | 22,755 | 1.793 s | **0.044 s** | **40.8×** |
| `paramiko 5.0.0` | 65 | 26,972 | 1.837 s | **0.042 s** | **44.0×** |
| `Mako 1.4.1` | 71 | 20,199 | 1.232 s | **0.025 s** | **48.6×** |
| `PyYAML 6.0.3` | 47 | 9,371 | 0.861 s | **0.016 s** | **53.7×** |
| `requests 2.34.2` | 35 | 11,526 | 0.942 s | **0.031 s** | **30.6×** |
| **8 libraries, total** | **572** | **176,043** | **12.78 s** | **0.31 s** | **41.1×** |

No outlier: every one of the eight libraries lands between **30.6×** and **53.7×**, a tighter band than
the 19×–86× spread of §3.2 because these are same-sized, ordinary application code rather than the
extremes (one pathological file, or the finding-poor stdlib). Extending to the full 36-package corpus
of [§5.5](#5-how-parity-is-proven) keeps the same shape: **88.0×**
on the best package (`transformers`) down to **37.6×** on the worst (`fabric`) — the aggregate 61.0×
of §3.2 is a genuine middle, not an average dragged up by one library.

### 3.4 Latency and memory

| Metric | Python | BanditRS | Delta |
|---|---:|---:|---|
| Latency on a 60-line file (the editor / `pre-commit` case) | 197 ms | **6 ms** | −97% |
| Peak RSS, stdlib scan | 75.5 MiB | **32.7 MiB** | −57% |
| Deployment size | Python venv + ~8 dependencies | **one 5.1 MB binary** | — |

The 6 ms single-file latency is the most concrete day-to-day argument: under 10 ms, the analysis becomes fast
enough to run **on every save** in an editor, which the ~200 ms of Python interpreter startup rules out.

### 3.5 Breaking the gain down: engine × parallelism

The overall factor mixes two independent effects. They can be separated by throttling rayon:

| `RAYON_NUM_THREADS` | Time (stdlib) | Speedup vs Python | Parallel speedup | Efficiency |
|---:|---:|---:|---:|---:|
| 1 | 0.591 s | 28.8× | 1.00× | — |
| 2 | 0.331 s | 51.4× | 1.79× | 89% |
| 4 | 0.176 s | 96.7× | 3.36× | 84% |

> This series is a separate campaign from the one in §3.2 (median of 3 instead of 7), hence the 0.176 s here
> against 0.214 s there: run-to-run variance on this shared container is on the order of ±10%. What matters
> here is not the absolute value but the **ratio between rows**, all drawn from the same series.

The observed factor therefore factorizes cleanly:

$$
S_{\text{total}} \;=\; \underbrace{28.8}_{\text{sequential engine}} \;\times\; \underbrace{3.36}_{\text{4 cores}} \;\approx\; 97
$$

Feeding the 4-core speedup into Amdahl's law recovers the parallelizable fraction of the work:

$$
S(p) = \frac{1}{(1-f) + \frac{f}{p}} \quad\Longrightarrow\quad
f = \frac{1 - 1/S(4)}{1 - 1/4} = \frac{1 - 0.2976}{0.75} \approx \mathbf{0.94}
$$

**94% of the work is parallelizable**; the remaining 6% is the filesystem walk, the serialization of the final
report and the aggregation of metrics — all intrinsically sequential. The model therefore predicts a ceiling
of $1/0.06 \approx 16.7\times$ for the parallel component alone: beyond a dozen cores, adding cores will bring
almost nothing more, and the factor will stay governed by the sequential gain and by disk I/O. This is an
extrapolation of a model from a single measurement point, to be confirmed on a machine with more cores.

---

## 4. Why it's faster

Three causes, in decreasing order of importance. None of them is "Rust is fast": they are architectural
choices that the language merely makes possible.

**1. The hot loop is no longer interpreted.** `bandit` visits each AST node and, for each node, calls a list
of plugin functions looked up in a Python dictionary, building a `Context` object wrapping a dict every time.
The dominant cost is not the analysis, it is *dispatch*: hundreds of millions of interpreted dict lookups and
function calls. On the Rust side the walker is a `match` on an `enum` of nodes, plugins are `fn`s in a static
table, and the `Context` is a typed struct borrowed from the tree — zero allocation per node.

> **Analogy.** Python hands each node to a switchboard operator who looks up the right party in a directory,
> picks up the phone, and forwards the call. Rust wired the switchboard once and for all at compile time:
> each node lands directly on the right extension.

**2. The parser.** CPython's `ast.parse` is written in C and is fast, but it materializes one Python object
per node (with a header, a reference count, an attribute dict). The `ruff_python_parser` / `ruff_python_ast`
crates produce a compact tree of `enum`s, with no indirection and no garbage collector.

**3. Parallelism.** Bandit's natural unit of work is the file, and files are independent. Rust has no GIL:
`rayon` simply hands out one file per task. That is what the factor 3.36 in §3.5 measures — a factor the
Python version cannot obtain without going multi-*process* (and therefore paying for a full interpreter per
core, which would also explain its RSS).

**An honest counter-example.** The `callgrind` profile ([`benchmarks.md` §6](docs/plan/benchmarks.md)) shows
that 51% of the current instruction budget goes into two functions that parity forces us to keep naive:
`NosecLines::for_range` (30.8%, a line-by-line sweep of `# nosec` comments, quadratic in nesting depth) and
the `trojansource` plugin (20.4%, ten search passes per line). Fixing them would probably buy another 25–30%
— but any optimization that would change issue ordering, a position or an output string is forbidden without
a validated entry in `DEVIATIONS.md`. **Parity outranks speed.**

---

## 5. How parity is proven

Relying on a single mode of proof means relying on its blind spots. BanditRS layers four of them,
sharing no assumptions — one reads the specification, one queries the living oracle, one freezes the
past, and one takes the tool out of the laboratory:

| Mode | What is compared | Scale | Result |
|---|---|---:|---|
| **Ported test suite** | bandit's own tests, one Python test ⇒ one Rust test with the same name | 273 Python tests → 354 Rust tests | 263 ported, 10 not portable ([#8](#6-remaining-differences-deliberate)), **0 ignored** |
| **Functional coverage** | test IDs and CLI options, enumerated from both tools | 75 IDs, 27 options | **75/75**, **27/27** |
| **Differential** | JSON report per file, both tools live, volatile fields normalized | `examples/` + stdlib + bandit's source: 835 files, 91 AST walk traces | **0 unexpected divergences** |
| **Golden corpus** | reports recorded from the reference, replayed without Python (`cargo test --test golden`) | 96 files, 3.7 MB | regression invariant: any moved byte fails CI |
| **Real third-party code** | 36 PyPI packages pinned by sha256, `bandit -r` on each | 21,536 files, 7.2 M lines, 130,730 issues | **36/36 identical**, `errors[]` included |
| **Whole option surface** | 88 CLI invocations: stdout, stderr **and** exit code | 88 | **85/88**, the 3 others are deviations [#10, #18, #19](#6-remaining-differences-deliberate) |

Between them these exercise **all 75 test IDs**. Two findings are worth naming: the CLI matrix caught
`-ll`/`-ii` filtering one level too low (bandit's suite never tests its own argument parser), and one
message in bandit's output interpolates a Python memory address, so the *reference* is not
reproducible between two runs — BanditRS renders `0x0` instead ([#5](#6-remaining-differences-deliberate)).

The method, the per-mode numbers and the commands to reproduce them: [`docs/parity.md`](docs/parity.md).
The package-by-package evidence, generated from the differential's own output and never hand-edited:
[`docs/drop-in-parity.md`](docs/drop-in-parity.md).

---

## 6. Remaining differences (deliberate)

The policy is explicit: **strict parity on everything the test suite observes**; bandit's known bugs are fixed
only when no test depends on them. The typos in messages, the `bad_calls`/`bad_imports` inversion in *legacy*
configuration conversion, the first-match ordering of blacklists, the "first hit" behaviour of `nosec` over a
`linerange`, the absent ≠ `None` distinction in `check_call_arg_value`: all of it is reproduced
**identically**, bugs included.

Here are the 16 deliberate deviations that remain. The [`DEVIATIONS.md`](DEVIATIONS.md) file gives the full
rationale for each, backed by a test case; it is append-only, so its numbering is stable and three of its
nineteen entries (#1, #14, #15) are struck through — the CLI matrix of [§5](#5-how-parity-is-proven)
showed they were divergences BanditRS could simply stop having, and they were resolved rather than justified.

| # | Deviation | Visible where | Nature |
|---:|---|---|---|
| 2 | CSV formatter with no CWE (`NOTSET`): Python crashes (`KeyError: 'link'`), BanditRS writes an empty column | `-f csv` | 🐞 bug fixed |
| 3 | `.bandit` (INI): `level`/`confidence`/`number` converted to integers; the `configfile` key actually honoured | `--ini` | 🐞 bug fixed |
| 4 | `discover_files` no longer mutates the loaded configuration (Python accumulated exclusions across calls) | repeated calls | 🐞 bug fixed |
| 5 | Memory addresses (`<ast.List object at 0x…>`) rendered as `0x0`; no `RecursionError` (iterative walker) | `B202`, deeply nested files | 🎯 determinism |
| 6 | No `rich` progress bar; `--version` shows the crate version; no `running on Python x.y.z` line | console output | 🎨 cosmetic |
| 7 | Positions of f-string constants: CPython ≥ 3.12 semantics by default | `-f json`, columns | ⚙️ `BANDITRS_PYTHON_COMPAT=3.11` |
| 8 | 10 Python introspection tests (`deepgetattr`, `meta_ast`…) with no Rust equivalent | test suite | 📐 not portable |
| 9 | `B703` / `DeepAssignation`: two edge cases where BanditRS is **stricter** (it does not reproduce two bugs in `django_xss.py`) | ad-hoc files only | 🐞 bug fixed |
| 10 | YAML: double-quoted (non-ASCII) scalars are not wrapped at 80 columns | `-f yaml`, rare | 🎨 cosmetic |
| 11 | YAML: no `&id001`/`*id001` anchors/aliases (PyYAML deduplicates by Python object `id()`) | `-f yaml` | 🎨 cosmetic |
| 12 | `more_info` points at `…/en/latest/…` instead of the installed package version | all formats | 🎯 deliberate choice |
| 13 | `Context::statement()` has a real implementation (in Python the key is never written, so the property is always `None`) | no plugin reads it | 📐 testability |
| 16 | Parser target version: set by `BANDITRS_PYTHON_COMPAT`, not by a host interpreter — align it with the reference and there is no gap at all, 3.12-only packages included | files using syntax the reference rejects | ⚙️ configuration |
| 17 | The ids in "profile include/exclude tests" come out in a stable order; Python joins a `set`, whose order changes between runs of the *same* command | `-v -p <profile>` | 🎯 determinism |
| 18 | `-d` does not dump the `Context` dict node by node (Python prints `<ast.Name object at 0x…>` reprs — 17,514 lines against 38); the `INFO`/`WARNING`/`ERROR` log lines are identical | `-d` | 🎨 cosmetic |
| 19 | An invalid YAML config reports `saphyr-parser`'s wording instead of PyYAML's — same behaviour, same exit code 2, different parser message | broken `-c` file | 🎨 cosmetic |

**None of these deviations changes which issue is reported, at which line, with which severity** — with the
assumed exception of #9 (two `B703` edge cases that no upstream fixture exercises) and #16, which is not a
gap but a setting: with the parser target aligned to the reference interpreter, the reports match on real
code down to `errors[]` ([§5](#5-how-parity-is-proven)).

---

## 7. Installation details

BanditRS ships as a Python package, so it drops into any environment that already
has `pip`, `uv` or `pipx` — no Rust knowledge required to *use* it. The quick
commands are at the [top of this README](#install); here is the full matrix.

| Method | Command | Needs a Rust toolchain? |
| --- | --- | --- |
| **uv, on the fly** (recommended) | `uvx banditrs -r .` | **No** |
| **uv, as a tool** | `uv tool install banditrs` | **No** |
| **pip** | `pip install banditrs` | **No** |
| **pipx** | `pipx install banditrs` | **No** |
| **A specific version or platform** | `pip install https://github.com/LePhilippeDucTai/BanditRS/releases/download/v0.2.1/<wheel>` | **No** |
| **From git (unreleased/dev)** | `pip install git+https://github.com/LePhilippeDucTai/BanditRS` | Yes |
| **From source** | `cargo build --release` | Yes |

> **Why `git+` needs Rust.** `pip install git+…` always builds from source: the
> tool clones the repository and builds a wheel from the checkout on the spot.
> No prebuilt artifact is involved, so `rustc` ≥ 1.96 and `cargo` must be
> present, and the build takes a couple of minutes. That is inherent to the
> `git+` syntax, not a limitation of this package — use it only to try an
> unreleased commit; every tagged release ships as a prebuilt wheel on PyPI,
> which is what the methods above install.

> **Why `uvx banditrs` needs no `--from`.** The distribution is named
> `banditrs`, and so is one of the four commands it installs (an alias for
> `bandit` — see [Usage](#usage)), so `uvx` finds it directly. To run the `bandit` name
> itself without installing anything, name the package explicitly:
> `uvx --from banditrs bandit -r .` — plain `uvx bandit` would instead look for
> the unrelated PyCQA `bandit` package on PyPI.

Every release publishes one wheel per platform. Because the wheels contain native
executables that do not link against libpython, each one is tagged
`py3-none-<platform>` and works with **every** supported Python version:

| Platform | Architecture | Wheel tag |
| --- | --- | --- |
| Linux (glibc ≥ 2.17) | x86-64 | `manylinux2014_x86_64` |
| Linux (glibc ≥ 2.17) | ARM64 | `manylinux2014_aarch64` |
| macOS | Intel | `macosx_*_x86_64` |
| macOS | Apple Silicon | `macosx_*_arm64` |
| Windows | x86-64 | `win_amd64` |
| Windows | ARM64 | `win_arm64` |

### Building from source

```bash
rustup update stable          # rustc ≥ 1.96 required
git clone https://github.com/LePhilippeDucTai/BanditRS && cd BanditRS
cargo build --release
# → target/release/{bandit, banditrs, bandit-baseline, bandit-config-generator}
```

---

## 8. Architecture and development

This is not a line-by-line transcription: it is an idiomatic redesign (no dynamic dictionaries, no mutable
global state, per-file parallelism) under an identical-output constraint.

```
src/
├── ast/         2,586 l.  ruff AST walk: walker, linerange, qualname, positions,
│                          literals, f-strings, debug trace
├── core/        4,654 l.  the engine: config, profiles, plugin registry, blacklists, tester,
│                          context, nosec, file discovery, metrics, parallel scan
├── plugins/     2,399 l.  the 42 plugins (32 files), messages and thresholds verbatim
├── formatters/  1,464 l.  the 9 output formats
├── cli/         1,579 l.  compatible argparse, bandit-baseline, bandit-config-generator
├── pycompat/    2,587 l.  exact emulation of the Python standard library wherever the output
│                          depends on it: PyYAML (80-column wrapping), configparser, format(),
│                          repr() of literals, encodings
└── source/        253 l.  file reading/decoding, line index, snippets
```

| | Python (`bandit`) | Rust (BanditRS) |
|---|---:|---:|
| Source code | 11,234 lines | 15,961 lines |
| Tests | 5,053 lines | 5,725 lines |

The `pycompat/` module (2,587 lines, 16% of the code) deserves a word: it implements no security analysis at
all. It exists only because bandit's output exposes implementation details of the Python standard library —
PyYAML's line-wrapping algorithm, `str.format()` syntax, the `repr()` of literals, `configparser` semantics.
**This is the real price of byte-for-byte parity**, and it is higher than that of the analysis engine itself.

### Dependencies

`ruff_python_{parser,ast}` · `ruff_{text_size,source_file}` (parser) · `rayon` (parallelism) · `regex` ·
`rustc-hash` · `indexmap` · `serde`/`serde_json` · `toml` · `saphyr-parser` · `encoding_rs` · `typed-arena`.

### Quality gate

```bash
scripts/check.sh          # build + 354 tests + rustfmt + clippy -D warnings + benchmark compilation
scripts/check.sh fast     # without the benchmarks (development loop)
```

`scripts/check.sh` reproduces the jobs of the GitHub Actions workflow in `.github/workflows/ci.yml`.
`scripts/check.sh parity` adds the differential against the reference Python bandit (the CLI matrix and
the smoke tier of the real-world corpus); it needs the network and the reference binary, which is why it
is not in the default run. Exit `0` = the equivalent of a green CI.

### Non-regression guardrail

Gains are worthless if they erode silently. `benches/e2e.rs` (criterion) measures 7 hot spots, and
`scripts/bench_regression.sh` fails if any of them degrades by more than **+10%** relative to the
committed baseline:

```bash
cargo bench --bench e2e            # HTML report in target/criterion/
scripts/bench_regression.sh        # comparison against the baseline, +10% threshold
```

### Checking parity yourself

Two of these need no Python at all — they replay output recorded from the reference:

```bash
cargo test --test golden                      # the golden corpus: examples/ in nine formats
cargo test --test cli_matrix                  # 88 CLI invocations: stdout, stderr AND exit code
```

The rest run the reference bandit alongside BanditRS and compare:

```bash
scripts/cli_matrix.py diff                    # the CLI matrix, live
scripts/corpus.py fetch --tier standard       # 24 real libraries, pinned by sha256 (~200 MB)
scripts/corpus.py verify --parse              # every file parses under the reference interpreter
scripts/diff_corpus.py --tier standard        # differential over 11 624 files of real code
scripts/diff_against_python.sh examples       # the original per-file differential
scripts/gen_golden.sh                         # regenerate the golden corpus
scripts/bench_corpus.py -n 5                  # speed, memory, thread scaling, cold start
```

They expect a reference Python `bandit`; by default `/home/user/.pyenv-bandit/bin/bandit`, overridable
through the `PY_BANDIT` variable. `scripts/check.sh parity` runs the fast subset of all of this.

**What that produces:** [`docs/drop-in-parity.md`](docs/drop-in-parity.md) — the evidence behind the
"drop-in replacement" claim, generated by `scripts/parity_report.py`, never edited by hand.

### Debugging aids

```bash
bandit --dump-walk examples/nosec.py     # AST walk trace (node order, contexts)
BANDITRS_PYTHON_COMPAT=3.11 bandit …     # f-string positions the CPython 3.11 way
```

### Internal documentation

| File | Contents |
|---|---|
| [`PLAN.md`](PLAN.md) | progress status, decisions, milestones, architecture, pitfalls encountered |
| [`DEVIATIONS.md`](DEVIATIONS.md) | the 16 remaining deliberate deviations, justified one by one (append-only: 19 entries, 3 struck through as resolved) |
| [`docs/spec/core.md`](docs/spec/core.md) | the exact semantics of the Python core (read before porting anything) |
| [`docs/spec/plugins.md`](docs/spec/plugins.md) | the 42 plugins + blacklists: messages, regexes, defaults verbatim |
| [`docs/spec/cli_formatters_tests.md`](docs/spec/cli_formatters_tests.md) | CLI, formatters, test suite as specification |
| [`docs/plan/benchmarks.md`](docs/plan/benchmarks.md) | measurement protocol, thresholds, results, `callgrind` profile |
| [`docs/plan/test-inventory.md`](docs/plan/test-inventory.md) | Python test ↔ Rust test mapping, line by line |
| [`docs/drop-in-parity.md`](docs/drop-in-parity.md) | the drop-in evidence, package by package and invocation by invocation — **generated**, never hand-edited |
| [`tests/corpus/README.md`](tests/corpus/README.md) | the real-world corpus: tiers, why sdists are pinned by sha256, why `verify --parse` is the precondition |
| [`docs/plan/README.md`](docs/plan/README.md) | the parallel development plan (21 work packages), kept for the record |

---

## 9. License and credits

Apache-2.0 — see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).

BanditRS is an independent reimplementation of [**bandit**](https://github.com/PyCQA/bandit) (PyCQA), from
which it takes the entirety of its semantics, its messages, the `examples/` fixtures and the test
specifications. All credit for the design of the security rules goes to the original project.

The Python parser comes from [**ruff**](https://github.com/astral-sh/ruff) (Astral).
