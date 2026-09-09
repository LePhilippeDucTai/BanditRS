<div align="center">

# BanditRS

**The Python security scanner [`bandit`](https://github.com/PyCQA/bandit), rewritten in pure Rust.**
Same tests, same options, same outputs, same exit codes — **17× to 87× faster**,
with **57% less memory**.

[![CI](https://github.com/LePhilippeDucTai/BanditRS/actions/workflows/ci.yml/badge.svg)](https://github.com/LePhilippeDucTai/BanditRS/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.96%2B-B7410E?logo=rust&logoColor=white)](rust-toolchain.toml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-351%20✓%20(0%20ignored)-success)](#5-how-parity-is-proven)
[![Parity](https://img.shields.io/badge/parity-75%2F75%20tests%20B1xx–B7xx-success)](#52-functional-coverage-75-test-ids-out-of-75)
[![Differential](https://img.shields.io/badge/differential-835%20files%20%C2%B7%200%20unexpected%20diff-success)](#53-differential-against-python-bandit)
[![Version](https://img.shields.io/badge/version-0.2.0-informational)](Cargo.toml)

</div>

---

## Install

Fastest way to try it: `uvx` fetches, builds and runs BanditRS in one shot, installing nothing
permanent. Here it scans the current directory:

```bash
uvx --from git+https://github.com/LePhilippeDucTai/BanditRS bandit -r .
```

To keep it around, install it as a tool — `bandit`, `banditrs`, `bandit-baseline` and
`bandit-config-generator` all land on your `PATH`:

```bash
uv tool install git+https://github.com/LePhilippeDucTai/BanditRS
bandit -r .
```

Both commands build the Rust crate from source, so they need `rustc` ≥ 1.96 and take a couple of
minutes the first time. To skip compiling altogether, grab the prebuilt wheel for your platform from
the [latest release](https://github.com/LePhilippeDucTai/BanditRS/releases) — no Rust toolchain
involved, and it works with every supported Python:

```bash
pip install https://github.com/LePhilippeDucTai/BanditRS/releases/download/v0.2.0/<wheel>
uvx --from <same-wheel-url> bandit -r .     # or run it on the fly, still without Rust
```

Platform wheel names, coexisting with the PyCQA `bandit` command, `pre-commit`, and building from
source: [§7](#7-usage-and-installation-details).

---

## Contents

1. [In one minute](#1-in-one-minute)
2. [Side-by-side output comparison](#2-side-by-side-output-comparison)
3. [Benchmarks](#3-benchmarks)
4. [Why it's faster](#4-why-its-faster)
5. [How parity is proven](#5-how-parity-is-proven)
6. [Remaining differences](#6-remaining-differences-deliberate)
7. [Usage and installation details](#7-usage-and-installation-details)
8. [Architecture](#8-architecture)
9. [Development](#9-development)
10. [License and credits](#10-license-and-credits)

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
| Executables | `bandit`, `bandit-baseline`, `bandit-config-generator` | **the same 3** |
| Exit codes | `0` / `1` (issues) / `2` (usage error) | **identical** |
| `# nosec`, `.bandit`, `pyproject.toml`, profiles, baseline | ✅ | ✅ |
| Parallelism | no (one file after another) | yes (rayon, one file per task) |
| Scanning the CPython 3.11 stdlib (672 files, 307k lines) | 17.0 s — 75 MiB | **0.20 s — 33 MiB** |

---

## 2. Side-by-side output comparison

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

*(Real outputs, wrapped and elided with `…` so they fit in two columns; the full `diff` follows.)*

<table>
<tr><th width="50%"><code>bandit -q app.py</code> (Python)</th><th width="50%"><code>bandit -q app.py</code> (BanditRS)</th></tr>
<tr valign="top"><td>

```
Run started:2026-09-09 09:53:52.239965+00:00

Test results:
>> Issue: [B404:blacklist] Consider possible security
   implications associated with the subprocess module.
   Severity: Low   Confidence: High
   CWE: CWE-78 (https://cwe.mitre.org/…/78.html)
   More Info: …/en/0.0.1.dev49/blacklists/…#b404-…
   Location: ./app.py:1:0
1	import subprocess
2	import yaml
3	

------------------------------------------------
>> Issue: [B506:yaml_load] Use of unsafe yaml load.
   Allows instantiation of arbitrary objects.
   Consider yaml.safe_load().
   Severity: Medium   Confidence: High
   CWE: CWE-20 (https://cwe.mitre.org/…/20.html)
   More Info: …/en/0.0.1.dev49/plugins/b506_yaml_load…
   Location: ./app.py:6:11
5	def load(path):
6	    return yaml.load(open(path).read())
7	
…
Total issues (by severity):
	Undefined: 0
	Low: 2
	Medium: 1
	High: 1
```

</td><td>

```
Run started:2026-09-09 09:53:52.273576+00:00

Test results:
>> Issue: [B404:blacklist] Consider possible security
   implications associated with the subprocess module.
   Severity: Low   Confidence: High
   CWE: CWE-78 (https://cwe.mitre.org/…/78.html)
   More Info: …/en/latest/blacklists/…#b404-…
   Location: ./app.py:1:0
1	import subprocess
2	import yaml
3	

------------------------------------------------
>> Issue: [B506:yaml_load] Use of unsafe yaml load.
   Allows instantiation of arbitrary objects.
   Consider yaml.safe_load().
   Severity: Medium   Confidence: High
   CWE: CWE-20 (https://cwe.mitre.org/…/20.html)
   More Info: …/en/latest/plugins/b506_yaml_load…
   Location: ./app.py:6:11
5	def load(path):
6	    return yaml.load(open(path).read())
7	
…
Total issues (by severity):
	Undefined: 0
	Low: 2
	Medium: 1
	High: 1
```

</td></tr>
</table>

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

### 2.1 The eight report formats, across all of `examples/`

The same exercise at the scale of the full fixture corpus (94 files), after neutralizing the volatile fields
only (timestamp, tool version in the doc URL). Eight formats out of nine: `screen` is the same rendering as
`txt` enriched with ANSI escape sequences, and is covered by `tests/unit_formatters_screen.rs` rather than by
this comparison.

```bash
for fmt in json txt csv xml yaml html sarif custom; do
  bandit-python -r -q -f $fmt examples | normalise > py.$fmt
  banditrs      -r -q -f $fmt examples | normalise > rs.$fmt
  diff py.$fmt rs.$fmt
done
```

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

### 2.2 On real code: the `bandit` repository itself

```console
$ # 69 files, 8,737 lines of code — bandit's own source, scanned by both tools
$ diff <(bandit-python -r -q -f json bandit/ | normalise) \
       <(banditrs      -r -q -f json bandit/ | normalise)
$ echo $?
0
```

**Identical** JSON report — same single issue found, same per-file metrics, same ordering.

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
| `/usr/lib/python3.11` (CPython stdlib) | 672 | 307,504 | 17.02 s | **0.196 s** | **86.9×** |
| `examples/` (bandit fixtures) | 94 | 9,534 | 6.497 s | **0.341 s** | **19.0×** |
| `examples/long_set.py` (the largest, 65 KiB) | 1 | 7,279 | 6.247 s | **0.359 s** | **17.4×** |
| `examples/subprocess_shell.py` (a typical file) | 1 | 60 | 0.197 s | **0.006 s** | **32.3×** |

Cost per line on the stdlib: **55 µs/line** on the Python side against **0.64 µs/line** on the Rust side —
i.e. 18 kloc/s against 1.57 Mloc/s.

> **Why is `examples/` "only" 19× when the stdlib is 87×?**
> Because `examples/` is dominated by a single pathological file: `long_set.py` accounts for 6.25 s out of
> the corpus's 6.50 s on the Python side. Measuring `examples/` therefore essentially means measuring *one*
> file — and parallelism does not help on a single file. The 19× is the **pure sequential** gain; the 87× on
> the stdlib is that same gain multiplied by parallelism. Both numbers are consistent, see §3.4.

### 3.3 Latency and memory

| Metric | Python | BanditRS | Delta |
|---|---:|---:|---|
| Latency on a 60-line file (the editor / `pre-commit` case) | 197 ms | **6 ms** | −97% |
| Peak RSS, stdlib scan | 75.5 MiB | **32.7 MiB** | −57% |
| Deployment size | Python venv + ~8 dependencies | **one 5.1 MB binary** | — |

The 6 ms single-file latency is the most concrete day-to-day argument: under 10 ms, the analysis becomes fast
enough to run **on every save** in an editor, which the ~200 ms of Python interpreter startup rules out.

### 3.4 Breaking the gain down: engine × parallelism

The overall factor mixes two independent effects. They can be separated by throttling rayon:

| `RAYON_NUM_THREADS` | Time (stdlib) | Speedup vs Python | Parallel speedup | Efficiency |
|---:|---:|---:|---:|---:|
| 1 | 0.591 s | 28.8× | 1.00× | — |
| 2 | 0.331 s | 51.4× | 1.79× | 89% |
| 4 | 0.176 s | 96.7× | 3.36× | 84% |

> This series is a separate campaign from the one in §3.2 (median of 3 instead of 7), hence the 0.176 s here
> against 0.196 s there: run-to-run variance on this shared container is on the order of ±10%. What matters
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

### 3.5 Non-regression guardrail

Gains are worthless if they erode silently. `benches/e2e.rs` (criterion) measures 7 hot spots, and
`scripts/bench_regression.sh` fails if any of them degrades by more than **+10%** relative to the committed
baseline:

```bash
cargo bench --bench e2e            # HTML report in target/criterion/
scripts/bench_regression.sh        # comparison against the baseline, +10% threshold
```

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
`rayon` simply hands out one file per task. That is what the factor 3.36 in §3.4 measures — a factor the
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

Relying on a single mode of proof means relying on its blind spots. BanditRS layers three of them, sharing no
assumptions: one reads the specification, one queries the living oracle, one freezes the past.

> **Analogy.** A new scale can be checked in three ways: by re-reading its manual (the test suite), by
> comparing it against a reference scale (the differential), and by re-weighing every morning the same
> reference weight kept in the drawer (the golden corpus). The third is the only one that still works when
> the reference scale is no longer in the room.

### 5.1 The Python test suite, ported test by test

The **273 tests** in `bandit/tests/` served as the acceptance specification. The porting rule: *one Python
test ⇒ one Rust test with the same name, in a mirror file*.

| | Count |
|---|---:|
| Reference Python tests | 273 |
| → ported as-is | 225 |
| → adapted (Python mocks replaced by real fixtures) | 38 |
| → not portable (pure Python introspection: `deepgetattr`, `meta_ast`…) | 10 |
| Rust tests **without** a Python counterpart (`argparse` emulation — see [§5.2](#52-functional-coverage-75-test-ids-out-of-75)) | 1 |
| **Total Rust tests** (67 unit + 284 integration) | **351** |
| Remaining `#[ignore]` tests | **0** |

The line-by-line inventory is in [`docs/plan/test-inventory.md`](docs/plan/test-inventory.md); the 10
non-portable ones are justified one by one ([deviation #8](#6-remaining-differences-deliberate)).

### 5.2 Functional coverage: 75 test IDs out of 75

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

### 5.3 Differential against Python bandit

[`scripts/diff_against_python.sh`](scripts/diff_against_python.sh) runs both binaries file by file,
normalizes the volatile fields, and **fails** if a file diverges for a reason that is not on the allow-list in
`DEVIATIONS.md`.

| Corpus | Files | Identical JSON reports | Unexpected divergences |
|---|---:|---:|---:|
| `examples/` (bandit fixtures) | 94 | **94** | **0** |
| `/usr/lib/python3.11` (CPython stdlib) | 672 | **672** | **0** |
| `bandit/` (bandit's own source code) | 69 | **69** | **0** |
| AST walk traces (order of visited nodes) | 91 | **91** | **0** |
| Aggregate reports over `examples/`, 8 formats | 8 | **7** | 1 — `yaml`, PyYAML anchors (see [§2.1](#21-the-eight-report-formats-across-all-of-examples)) |

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
([deviation #5](#6-remaining-differences-deliberate)).

### 5.4 Golden corpus: parity without Python

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

---

## 6. Remaining differences (deliberate)

The policy is explicit: **strict parity on everything the test suite observes**; bandit's known bugs are fixed
only when no test depends on them. The typos in messages, the `bad_calls`/`bad_imports` inversion in *legacy*
configuration conversion, the first-match ordering of blacklists, the "first hit" behaviour of `nosec` over a
`linerange`, the absent ≠ `None` distinction in `check_call_arg_value`: all of it is reproduced
**identically**, bugs included.

Here are the 15 deliberate deviations. The [`DEVIATIONS.md`](DEVIATIONS.md) file gives the full rationale for
each, backed by a test case.

| # | Deviation | Visible where | Nature |
|---:|---|---|---|
| 1 | `# nosec B101,B102` (comma without a space): Python only ignores the **last** id, BanditRS takes them all | files with multi-id `nosec` | 🐞 bug fixed |
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
| 14 | The "Using command line arg for selected targets" log is only emitted if the `targets` key is in the `.bandit` file | `-v --ini` | 🎨 cosmetic |
| 15 | `bandit-baseline`: "Got current commit: `master`" instead of "`<sha> master`" | `bandit-baseline` | 🎨 cosmetic |

**None of these deviations changes which issue is reported, at which line, with which severity** — with the
assumed exception of #9 (two `B703` edge cases that no upstream fixture exercises) and #1 (a `nosec` syntax
that Python clearly mishandles).

---

## 7. Usage and installation details

### Installing

BanditRS ships as a Python package, so it drops into any environment that already
has `pip`, `uv` or `pipx` — no Rust knowledge required to *use* it. The quick
commands are at the [top of this README](#install); here is the full matrix.

| Method | Command | Needs a Rust toolchain? |
| --- | --- | --- |
| **Prebuilt wheel** (recommended) | `pip install https://github.com/LePhilippeDucTai/BanditRS/releases/download/v0.2.0/<wheel>` | **No** |
| **Prebuilt wheel, no install** | `uvx --from <same-wheel-url> bandit -r .` | **No** |
| **uv, on the fly** | `uvx --from git+https://github.com/LePhilippeDucTai/BanditRS bandit -r .` | **Yes** |
| **uv, as a tool** | `uv tool install git+https://github.com/LePhilippeDucTai/BanditRS` | **Yes** |
| **From git** | `pip install git+https://github.com/LePhilippeDucTai/BanditRS` | **Yes** |
| **From source** | `cargo build --release` | Yes |

> **Why `git+` needs Rust.** `pip install git+…` and `uvx --from git+…` always
> build from source: the tool clones the repository and builds a wheel from the
> checkout on the spot. No prebuilt artifact is involved, so `rustc` ≥ 1.96 and
> `cargo` must be present, and the build takes a couple of minutes. That is
> inherent to the `git+` syntax, not a limitation of this package. The prebuilt
> wheels attached to each
> [release](https://github.com/LePhilippeDucTai/BanditRS/releases) exist precisely
> to avoid it — pick the one matching your platform from the table below.

> **Why `--from`.** The distribution is named `banditrs`, but the command you
> usually want is `bandit`. `uvx` assumes the two match, so plain `uvx bandit`
> would look for a *different* project; `--from` names the package explicitly and
> lets you pick any of the four commands it ships. `uvx --from git+… banditrs -r .`
> runs the same program under the non-conflicting alias.

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

Installing gives you four commands:

```
bandit                     drop-in replacement for the PyCQA command
banditrs                   identical alias (see below)
bandit-baseline            scan against a git baseline
bandit-config-generator    emit a configuration profile
```

#### Coexisting with PyCQA bandit

The `bandit` command is a deliberate drop-in replacement, which means it occupies
the same file name as the PyCQA package. If both are installed in the same
environment, the last one installed wins — silently. The `banditrs` alias is
there for exactly that case: it is byte-for-byte the same program under a name
nothing else claims, so you can compare the two side by side, or migrate
gradually, without uninstalling anything.

### Usage

Every `bandit` invocation works as-is:

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

The two companion tools too:

```bash
bandit-config-generator --show-defaults > bandit.yaml   # generate a full profile
bandit-baseline -r my_project/                          # diff against the parent commit (requires git)
```

### From Python

The `banditrs` module is a thin locator/launcher around the same executables —
the analysis is never reimplemented in Python:

```python
import banditrs

banditrs.find_bandit_bin()                  # -> PosixPath('.../bin/bandit')
result = banditrs.run(["-r", "src/", "-f", "json"], capture_output=True, text=True)
result.returncode                           # non-zero when issues are found
```

`python -m banditrs …` is equivalent to calling `bandit` directly.

### Replacing bandit in a CI pipeline

```yaml
# .pre-commit-config.yaml — before
- repo: https://github.com/PyCQA/bandit
  rev: <tag>
  hooks: [{ id: bandit, args: ["-r", "src/"] }]

# after: same invocation, same output, same exit code
- repo: https://github.com/LePhilippeDucTai/BanditRS
  rev: v0.2.0
  hooks: [{ id: banditrs, args: ["-r", "src/"] }]
```

pre-commit builds the hook in its own isolated environment, so this route
compiles from source and needs a Rust toolchain on the machine running the hook.

### Building from source

```bash
rustup update stable          # rustc ≥ 1.96 required
git clone https://github.com/LePhilippeDucTai/BanditRS && cd BanditRS
cargo build --release
# → target/release/{bandit, banditrs, bandit-baseline, bandit-config-generator}
```

### Development tooling

```bash
bandit --dump-walk examples/nosec.py     # AST walk trace (node order, contexts)
BANDITRS_PYTHON_COMPAT=3.11 bandit …     # f-string positions the CPython 3.11 way
```

---

## 8. Architecture

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

---

## 9. Development

### Quality gate

```bash
scripts/check.sh          # build + 351 tests + rustfmt + clippy -D warnings + benchmark compilation
scripts/check.sh fast     # without the benchmarks (development loop)
```

`scripts/check.sh` reproduces the jobs of the GitHub Actions workflow in `.github/workflows/ci.yml`.
`scripts/check.sh parity` adds the differential against the reference Python bandit (the CLI matrix and
the smoke tier of the real-world corpus); it needs the network and the reference binary, which is why it
is not in the default run. Exit `0` = the equivalent of a green CI.

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

### Internal documentation

| File | Contents |
|---|---|
| [`PLAN.md`](PLAN.md) | progress status, decisions, milestones, architecture, pitfalls encountered |
| [`DEVIATIONS.md`](DEVIATIONS.md) | the 15 deliberate deviations, justified one by one |
| [`docs/spec/core.md`](docs/spec/core.md) | the exact semantics of the Python core (read before porting anything) |
| [`docs/spec/plugins.md`](docs/spec/plugins.md) | the 42 plugins + blacklists: messages, regexes, defaults verbatim |
| [`docs/spec/cli_formatters_tests.md`](docs/spec/cli_formatters_tests.md) | CLI, formatters, test suite as specification |
| [`docs/plan/benchmarks.md`](docs/plan/benchmarks.md) | measurement protocol, thresholds, results, `callgrind` profile |
| [`docs/plan/test-inventory.md`](docs/plan/test-inventory.md) | Python test ↔ Rust test mapping, line by line |
| [`docs/plan/README.md`](docs/plan/README.md) | the parallel development plan (16 work packages), kept for the record |

---

## 10. License and credits

Apache-2.0 — see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).

BanditRS is an independent reimplementation of [**bandit**](https://github.com/PyCQA/bandit) (PyCQA), from
which it takes the entirety of its semantics, its messages, the `examples/` fixtures and the test
specifications. All credit for the design of the security rules goes to the original project.

The Python parser comes from [**ruff**](https://github.com/astral-sh/ruff) (Astral).
