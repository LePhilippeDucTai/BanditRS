#!/usr/bin/env python3
"""Real-world Python corpus for the BanditRS differential harness (WP-17).

The corpus is a set of PyPI source distributions, pinned by sha256 in
`tests/corpus/manifest.tsv` (committed) and downloaded on demand into
`target/corpus/` (gitignored, `/target` is already in `.gitignore`).

Why sdists and not git clones: a PyPI artifact is immutable once published, so a
sha256 pins the exact bytes for good; if the URL ever disappears the hash still
identifies the file on any mirror. A git checkout, by contrast, drifts with the
branch and needs the whole history.

Subcommands
    seed                rebuild manifest.tsv from the curated list below
    fetch [--tier T]    download + verify + extract into target/corpus/
    verify [--parse]    re-check hashes and counts; --parse also requires every
                        file to parse under the *reference* interpreter
    list [--tier T]     print the manifest
    stats               per-tier totals

Tiers
    smoke      ~8 packages, seconds       — runs in scripts/check.sh
    standard   +15 packages, minutes      — nightly CI / on demand
    full       +12 packages, a campaign
    frontier   deliberately 3.12+-only code: bandit-on-CPython-3.11 cannot parse
               it, BanditRS can. Expected to diverge (DEVIATIONS.md #16); never
               part of a pass/fail gate.
"""

import argparse
import hashlib
import io
import json
import os
import subprocess
import sys
import tarfile
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "tests" / "corpus" / "manifest.tsv"
DEST = Path(os.environ.get("CORPUS_DIR", ROOT / "target" / "corpus"))
COLUMNS = [
    "tier",
    "name",
    "version",
    "url",
    "sha256",
    "upload_time",
    "py_files",
    "py_lines",
    "requires_python",
    "note",
]

# (tier, package, pin, note). A pin of "auto" means "newest release whose
# Requires-Python admits 3.11" — resolved at seed time and then frozen in the
# manifest. An explicit pin is used where a specific release matters: an LTS
# line, or the newest release that still parses under 3.11.
CURATED = [
    # -- smoke: small, security-dense, fast enough for the local quality gate --
    ("smoke", "paramiko", "auto", "B507 host-key policy, B303/B324 hashes, B505 key sizes"),
    ("smoke", "PyYAML", "auto", "B506 yaml.load; BanditRS' YAML emitter run over PyYAML itself"),
    ("smoke", "requests", "auto", "B501 verify=False, B113 missing timeout, B310"),
    ("smoke", "urllib3", "auto", "B501/B502/B503 SSL contexts, B310, B311"),
    ("smoke", "Jinja2", "auto", "B701 autoescape"),
    ("smoke", "Mako", "auto", "B702 mako templates, B102 exec/compile in the engine"),
    ("smoke", "Flask", "auto", "B104 bind-all, B105, B201 debug=True"),
    ("smoke", "Werkzeug", "auto", "B104, B108 /tmp, B303, B311, B102"),
    # -- standard: breadth of plugins, parser scale, real application code --
    ("standard", "Django", "5.2.17", "LTS line (>=3.10): B703 mark_safe, B610/B611, B608, B105"),
    ("standard", "SQLAlchemy", "auto", "B608 string-built SQL, modern typing, very long files"),
    ("standard", "ansible-core", "2.19.13", "newest >=3.11 line: B602-B607 shell/partial paths, B404, B108"),
    ("standard", "celery", "auto", "B301 pickle serializer, B104, B108, B311"),
    ("standard", "Scrapy", "auto", "B320/B410 lxml, B301, B311, B113, B310"),
    ("standard", "pycryptodome", "auto", "B304/B305 ciphers and modes, B413 import Crypto, B505"),
    ("standard", "cryptography", "auto", "B505 key sizes, B324"),
    ("standard", "botocore", "auto", "near-generated code, very large modules: parser scale"),
    ("standard", "boto3", "auto", "thin layer over botocore, kept for realism"),
    ("standard", "pandas", "auto", "B101 asserts at scale, B301 pickle, B307 eval, modern typing"),
    ("standard", "numpy", "2.4.6", "newest >=3.11 release: B101, B102 exec, legacy distutils"),
    ("standard", "redis", "auto", "B113, B105, B311"),
    ("standard", "pymongo", "auto", "B113, B105, B303"),
    ("standard", "pip", "auto", "_vendor/: heterogeneous third-party code, old styles, setup.py"),
    ("standard", "setuptools", "auto", "legacy + vendored + distutils: B102/B307, B404"),
    ("standard", "tornado", "auto", "async throughout (walker stress), B104, B113"),
    # -- full: the campaign corpus --
    ("full", "salt", "auto", "~4500 files, subprocess/crypto/pickle everywhere"),
    ("full", "transformers", "auto", "B614 torch.load, B615 unpinned HF download, B301"),
    ("full", "apache-airflow-core", "auto", "subprocess + SQL + Jinja, B608, B105"),
    ("full", "matplotlib", "auto", "B101, B307, non-ASCII strings, very large files"),
    ("full", "scipy", "auto", "deep nesting, B101, f2py legacy"),
    ("full", "ipython", "auto", "the densest concentration of B102 exec / B307 eval"),
    ("full", "aiohttp", "auto", "async, B113, B104"),
    ("full", "poetry", "auto", "modern typing, subprocess"),
    ("full", "supervisor", "auto", "B104, B108, B411 XML-RPC, odd test fixtures"),
    ("full", "fabric", "auto", "B601/B602/B605"),
    ("full", "pexpect", "auto", "pty and process spawning"),
    ("full", "black", "auto", "tests/data/: walrus, match, PEP 604/646/695, nested f-strings"),
    # -- frontier: expected to diverge, measured separately --
    ("frontier", "Django", "6.1.1", "requires >=3.12: 6 files CPython 3.11 cannot parse"),
    ("frontier", "ansible-core", "2.21.4", "requires >=3.12: 28 files using PEP 695 type params"),
]


def die(msg):
    print(f"corpus.py: {msg}", file=sys.stderr)
    raise SystemExit(1)


def read_manifest():
    if not MANIFEST.exists():
        die(f"{MANIFEST} not found — run `scripts/corpus.py seed` first")
    rows = []
    for line in MANIFEST.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) != len(COLUMNS):
            die(f"malformed manifest row ({len(parts)} fields): {line[:80]}")
        rows.append(dict(zip(COLUMNS, parts)))
    return rows


def select(rows, tier):
    if tier in (None, "all"):
        return rows
    order = ["smoke", "standard", "full"]
    if tier in order:  # tiers are cumulative; frontier is not
        keep = set(order[: order.index(tier) + 1])
        return [r for r in rows if r["tier"] in keep]
    return [r for r in rows if r["tier"] == tier]


def pypi(name):
    url = f"https://pypi.org/pypi/{name}/json"
    with urllib.request.urlopen(url, timeout=60) as fh:
        return json.load(fh)


def admits_311(spec):
    """True if a Requires-Python specifier admits 3.11 (subset of PEP 440 we need)."""
    if not spec:
        return True
    for clause in spec.split(","):
        clause = clause.strip()
        if not clause:
            continue
        for op in (">=", "<=", "!=", "==", "~=", ">", "<"):
            if clause.startswith(op):
                ver = clause[len(op):].strip().rstrip("*").rstrip(".")
                break
        else:
            continue
        try:
            bound = tuple(int(x) for x in ver.split(".") if x.isdigit())
        except ValueError:
            continue
        if not bound:
            continue
        # patch level is irrelevant to syntax; 99 keeps `>=3.11.4` admissible
        here = (3, 11, 99)[: len(bound)]
        if op == ">=" and not here >= bound:
            return False
        if op == ">" and not here > bound:
            return False
        if op == "<=" and not here <= bound:
            return False
        if op == "<" and not here < bound:
            return False
        if op == "!=" and here == bound:
            return False
        if op in ("==", "~=") and here[: len(bound)] != bound:
            return False
    return True


def sdist_of(data, version):
    for f in data["releases"].get(version, []):
        if f["packagetype"] == "sdist" and not f.get("yanked"):
            return f
    return None


def resolve(name, pin):
    data = pypi(name)
    if pin != "auto":
        f = sdist_of(data, pin)
        if f is None:
            die(f"{name} {pin}: no sdist for that version")
        # the *file's* Requires-Python, not `info`'s — `info` describes the
        # latest release, which is exactly what an explicit pin is avoiding
        return pin, f, f.get("requires_python") or ""
    best = None
    for version, files in data["releases"].items():
        f = next((x for x in files if x["packagetype"] == "sdist" and not x.get("yanked")), None)
        if f is None or not admits_311(f.get("requires_python")):
            continue
        parts = version.split(".")
        if not all(p.isdigit() for p in parts[:3]) or not parts[0].isdigit():
            continue  # skip pre-releases and non-numeric schemes
        key = tuple(int(p) for p in parts if p.isdigit())
        if best is None or key > best[0]:
            best = (key, version, f)
    if best is None:
        die(f"{name}: no sdist admitting Python 3.11")
    return best[1], best[2], best[2].get("requires_python") or ""


def cmd_seed(args):
    rows = []
    for tier, name, pin, note in CURATED:
        print(f"resolving {name} ({pin})…", file=sys.stderr)
        version, f, req = resolve(name, pin)
        rows.append(
            {
                "tier": tier,
                "name": name,
                "version": version,
                "url": f["url"],
                "sha256": f["digests"]["sha256"],
                "upload_time": f.get("upload_time_iso_8601", f.get("upload_time", "")),
                "py_files": "-",
                "py_lines": "-",
                "requires_python": req or "-",
                "note": note,
            }
        )
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    with MANIFEST.open("w") as fh:
        fh.write(
            "# Real-world corpus for the differential harness (WP-17). Generated by\n"
            "# `scripts/corpus.py seed`; py_files/py_lines are filled by `fetch`.\n"
            "# Artifacts are pinned by sha256: PyPI files are immutable, so the hash\n"
            "# identifies the exact bytes on any mirror even if the URL rots.\n"
            "#" + "\t".join(COLUMNS) + "\n"
        )
        for r in rows:
            fh.write("\t".join(r[c] for c in COLUMNS) + "\n")
    print(f"wrote {MANIFEST} ({len(rows)} entries)")


def target_dir(row):
    return DEST / f"{row['name']}-{row['version']}"


def download(row):
    blob = urllib.request.urlopen(row["url"], timeout=600).read()
    got = hashlib.sha256(blob).hexdigest()
    if got != row["sha256"]:
        die(f"{row['name']} {row['version']}: sha256 mismatch\n  expected {row['sha256']}\n  got      {got}")
    return blob


def extract(blob, dest, url):
    dest.mkdir(parents=True, exist_ok=True)
    if url.endswith(".zip"):
        zipfile.ZipFile(io.BytesIO(blob)).extractall(dest)
    else:
        tarfile.open(fileobj=io.BytesIO(blob)).extractall(dest, filter="data")


def count(dest):
    files = lines = 0
    for p in dest.rglob("*.py"):
        if not p.is_file():
            continue
        files += 1
        lines += p.read_bytes().count(b"\n")
    return files, lines


def write_manifest(rows):
    head = [l for l in MANIFEST.read_text().splitlines() if l.startswith("#")]
    with MANIFEST.open("w") as fh:
        fh.write("\n".join(head) + "\n")
        for r in rows:
            fh.write("\t".join(r[c] for c in COLUMNS) + "\n")


def cmd_fetch(args):
    rows = read_manifest()
    todo = select(rows, args.tier)
    for row in todo:
        dest = target_dir(row)
        if dest.exists() and not args.force:
            print(f"have  {row['name']} {row['version']}")
        else:
            print(f"fetch {row['name']} {row['version']} …", flush=True)
            extract(download(row), dest, row["url"])
        files, lines = count(dest)
        if row["py_files"] in ("-", ""):
            row["py_files"], row["py_lines"] = str(files), str(lines)
        elif (str(files), str(lines)) != (row["py_files"], row["py_lines"]):
            die(
                f"{row['name']} {row['version']}: extracted tree does not match the manifest "
                f"({files} files / {lines} lines, expected {row['py_files']} / {row['py_lines']})"
            )
    write_manifest(rows)
    print(f"corpus in {DEST} ({len(todo)} package(s))")


def parse_check(dest, python):
    """Every file must parse under the reference interpreter.

    This is the load-bearing precondition of the whole differential: if a file
    is rejected by the reference interpreter's own parser, any divergence on it
    says something about parser versions, not about BanditRS.
    """
    script = (
        "import ast,sys\n"
        "bad=0\n"
        "for p in sys.argv[1:]:\n"
        "    try: ast.parse(open(p,'rb').read())\n"
        "    except SyntaxError as e: bad+=1; print(f'{p}: {e.msg}')\n"
        "    except Exception as e: bad+=1; print(f'{p}: {type(e).__name__}')\n"
        "sys.exit(1 if bad else 0)\n"
    )
    paths = [str(p) for p in dest.rglob("*.py") if p.is_file()]
    bad = []
    for i in range(0, len(paths), 400):  # keep the argv under ARG_MAX
        r = subprocess.run([python, "-c", script, *paths[i : i + 400]], capture_output=True, text=True)
        if r.returncode:
            bad.extend(r.stdout.strip().splitlines())
    return bad


def cmd_verify(args):
    rows = select(read_manifest(), args.tier)
    failures = 0
    for row in rows:
        dest = target_dir(row)
        if not dest.exists():
            print(f"MISSING {row['name']} {row['version']} — run `corpus.py fetch`")
            failures += 1
            continue
        files, lines = count(dest)
        if (str(files), str(lines)) != (row["py_files"], row["py_lines"]):
            print(f"COUNT   {row['name']}: {files}/{lines} vs manifest {row['py_files']}/{row['py_lines']}")
            failures += 1
            continue
        msg = f"ok      {row['name']} {row['version']} ({files} files, {lines} lines)"
        if args.parse:
            bad = parse_check(dest, args.python)
            if bad:
                tag = "expected" if row["tier"] == "frontier" else "FAIL"
                print(f"{tag:<7} {row['name']}: {len(bad)} file(s) rejected by {args.python}")
                for b in bad[:5]:
                    print(f"          {b}")
                if row["tier"] != "frontier":
                    failures += 1
                continue
            if row["tier"] == "frontier":
                print(f"NOTE    {row['name']}: frontier entry parses cleanly — it no longer proves anything")
            msg += ", all parse"
        print(msg)
    if failures:
        print(f"corpus.py: {failures} failure(s)", file=sys.stderr)
        return 1
    print(f"corpus.py: {len(rows)} package(s) verified")
    return 0


def cmd_list(args):
    for r in select(read_manifest(), args.tier):
        print(f"{r['tier']:<9} {r['name']:<22} {r['version']:<12} {r['py_files']:>6} files  {r['note']}")
    return 0


def cmd_stats(args):
    rows = read_manifest()
    print(f"{'tier':<10} {'packages':>8} {'py files':>10} {'py lines':>10}")
    for tier in ("smoke", "standard", "full", "frontier"):
        sel = [r for r in rows if r["tier"] == tier]
        f = sum(int(r["py_files"]) for r in sel if r["py_files"].isdigit())
        l = sum(int(r["py_lines"]) for r in sel if r["py_lines"].isdigit())
        print(f"{tier:<10} {len(sel):>8} {f:>10} {l:>10}")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    tier = dict(choices=["smoke", "standard", "full", "frontier", "all"], default="smoke")
    sub.add_parser("seed").set_defaults(fn=cmd_seed)
    p = sub.add_parser("fetch")
    p.add_argument("--tier", **tier)
    p.add_argument("--force", action="store_true")
    p.set_defaults(fn=cmd_fetch)
    p = sub.add_parser("verify")
    p.add_argument("--tier", **tier)
    p.add_argument("--parse", action="store_true")
    p.add_argument("--python", default=os.environ.get("PY_REF", "/home/user/.pyenv-bandit/bin/python"))
    p.set_defaults(fn=cmd_verify)
    p = sub.add_parser("list")
    p.add_argument("--tier", **tier)
    p.set_defaults(fn=cmd_list)
    sub.add_parser("stats").set_defaults(fn=cmd_stats)
    args = ap.parse_args()
    raise SystemExit(args.fn(args) or 0)


if __name__ == "__main__":
    main()
