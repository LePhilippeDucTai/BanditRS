---
name: banditrs-dev
description: >
  Continue the BanditRS implementation (the pure-Rust rewrite of the Python `bandit` security
  linter, at /home/user/banditrs) from wherever the last session left off, and at the end of the
  session merge the work into main and push directly. Use this whenever the user asks to
  "continue banditrs", "continue l'implémentation", work on any BanditRS module/plugin/formatter/
  CLI piece, port more of bandit to Rust, fix a differential/parity bug against Python bandit, or
  asks to wrap up, merge, and push a BanditRS session — even if they don't name the skill
  explicitly. If the working directory is /home/user/banditrs (or the user mentions BanditRS,
  bandit-rs, or "the Rust bandit rewrite"), this skill almost certainly applies.
---

# BanditRS development sessions

BanditRS is a from-scratch, behavior-exact Rust port of `bandit` (the Python security linter). It
is far enough along that "continuing the implementation" mostly means: read the handoff doc, port
or fix one thing at a time, verify each increment against the real Python tool, and leave the
handoff doc accurate for whoever (or whatever session) picks it up next. This skill is that loop,
plus the end-of-session merge-and-push the user wants every time.

Do not treat this as a green-field coding task where you improvise architecture — the design,
conventions, and even the exact wording of known bugs to reproduce are already settled and written
down. Your job is to extend the existing pattern, not invent a new one.

## 1. Orient yourself

Read, in order:

1. **`PLAN.md`** — the canonical handoff document. §3 says what's done and validated, §5
   ("Prochaines étapes détaillées") says what to do next and in what order, §6 lists known
   pitfalls worth re-reading before you re-discover them the hard way, §7 has ready-to-paste
   commands. Trust §5 over your own guess about priorities — it reflects decisions already made
   with the user across many prior sessions.
2. **`DEVIATIONS.md`** — every place BanditRS *deliberately* differs from Python bandit, with the
   reason. If you find an output mismatch, check here first: it may be a known, accepted deviation
   rather than a bug to fix. If you introduce a new deliberate deviation, add an entry here in the
   same style (numbered, one paragraph, cites what test or behavior it affects) — don't leave it
   undocumented, or the next session will "fix" it back into a mismatch.
3. **`docs/spec/*.md`** (`core.md`, `plugins.md`, `cli_formatters_tests.md`) — a summarized spec of
   bandit's behavior, organized to mirror the Rust module layout. Useful for orientation, but it is
   a *summary*: when actually porting a module, the Python source is the source of truth (next
   point), not the summary.

## 2. Before porting or fixing any module

The Python reference lives at **`/home/user/bandit`** (a plain read-only clone, not a package
install). Before writing or fixing the Rust equivalent of any module, read the corresponding
Python file there — the spec docs compress away edge cases, exact string formatting, and the
occasional upstream bug that a test or the differential harness depends on byte-for-byte. This
project's stated policy (see `PLAN.md` §1) is strict parity on anything the test suite or
differential harness observes, including known upstream *bugs* — fix a bug only when you've
checked nothing depends on the buggy behavior. When in doubt, reproduce Python's behavior exactly
and leave a comment explaining the quirk (see existing code for the style — e.g.
`src/formatters/sarif.rs`'s comments on `line_range[1]` indexing and negative-index wraparound).

If you need to verify something empirically rather than by reading source (e.g., "does PyYAML fold
this string?", "what order does stevedore load plugins in?"), just run it: `/home/user/bandit` and
a working Python 3 are both available, and this project's git history includes real examples of
bugs found exactly this way (plugin load order, SARIF snippet indexing) — don't guess when you can
check in ten seconds.

## 3. The reference Python venv (for differential testing)

`/home/user/.pyenv-bandit` is a venv with bandit installed editable from `/home/user/bandit`, used
to compare BanditRS's actual output against real bandit. If it's missing (fresh container), recreate it:

```bash
python3 -m venv /home/user/.pyenv-bandit
/home/user/.pyenv-bandit/bin/pip install -e "/home/user/bandit[toml,yaml,sarif]"
# sarif extras have needed an extra manual install before:
/home/user/.pyenv-bandit/bin/pip install sarif_om jschema-to-python
```

Use it whenever a change could affect observable output (a plugin, a formatter, position/line
handling, config loading). A quick single-file check:

```bash
diff <(/home/user/.pyenv-bandit/bin/bandit examples/foo.py -f json 2>/dev/null | python3 -m json.tool) \
     <(BANDITRS_PYTHON_COMPAT=3.11 target/debug/bandit examples/foo.py -f json 2>/dev/null | python3 -m json.tool)
```

`BANDITRS_PYTHON_COMPAT=3.11` matters: the reference venv runs Python 3.11, and BanditRS defaults
to CPython ≥3.12 f-string position semantics (see `PLAN.md` §1). Without that env var you'll see
spurious diffs on any fixture with f-strings and chase a bug that isn't one.

For a broader sweep across every fixture and format (worth doing before ending a session that
touched a plugin or formatter), loop over `examples/*.py` × the output formats and diff each
against the reference venv, normalizing the known-volatile fields (`generated_at`, doc-version
URLs, the crate/package version string) — see `PLAN.md` §5 for the exact `norm()` sed pattern used
last time and the list of diffs that are expected (covered by `DEVIATIONS.md`). Anything *not*
covered by an existing deviation entry is a real bug: fix it before considering the increment done.

## 4. How to work

Small, verified increments — this is how the whole codebase got built and it's why the
differential harness stays green:

1. Pick the next item from `PLAN.md` §5 (or whatever the user asked for).
2. Read the Python source for that piece (§2 above).
3. Implement it in Rust, matching existing module style (check a sibling file in the same
   directory for conventions before inventing your own).
4. `cargo build --lib` (fast feedback), then the relevant test subset, then `cargo test
   --all-targets` once the piece is plausibly done.
5. If it affects output, spot-check with the differential command from §3.
6. Commit. Prefer several small, well-described commits over one large one — it makes the
   `git log` itself useful documentation of what happened, and it means a build break is easy to
   bisect.

Don't batch up a large amount of unverified work and try to debug it all at once at the end —
every prior session in this project's history that did small verified steps found and fixed real
bugs quickly (wrong plugin order, SARIF indexing, YAML folding edge cases); the failure mode to
avoid is discovering three unrelated bugs simultaneously after an hour of uncompiled changes.

## 5. Ending the session: validate, then merge and push to main

The user has asked, every time, for the session's work to end up merged into `main` and pushed —
no PR, no review step, that's the deliberate workflow for this repo. Do this automatically at the
end of a work session (or immediately if the user explicitly says to wrap up / merge / push), but
earn it first: a push to `main` should never carry a red build.

```bash
cd /home/user/banditrs
git status                       # see what's uncommitted before doing anything destructive-adjacent

cargo build --all-targets
cargo test --all-targets         # every suite must show 0 failed
cargo clippy --all-targets -- -D warnings   # must be clean
cargo fmt --all --check          # if this fails, run `cargo fmt --all` and re-verify build+test
```

If anything here is red, fix it (or, if it's pre-existing and out of scope, tell the user and stop
— don't merge red).

Once green:

1. **Update `PLAN.md`** to reflect what actually happened this session — the §3 status table and
   §5 "next steps" are the whole point of this document; if they go stale, the next session (or
   the next `banditrs-dev` invocation) starts from a false picture. Move finished items from §5
   into §3, adjust the milestone table, and leave §5 pointing at the real next step. Do this
   *before* the final commit so it's part of the session's history, not a follow-up.
2. **Commit** any remaining uncommitted work on the current feature branch (check `git branch
   --show-current` — this project has used branches like
   `claude/banditrs-implementation-<id>`; use whatever branch is checked out, don't assume the name).
3. **Push the feature branch**: `git push -u origin <branch>`.
4. **Merge into main**:
   ```bash
   git fetch origin main
   git merge-base --is-ancestor origin/main <branch> && echo "fast-forward OK"
   ```
   - If that prints "fast-forward OK" (main hasn't diverged — the common case, since this repo's
     workflow keeps all work on one feature branch merged straight through): `git checkout main`,
     `git merge --ff-only <branch>`, `git push origin main`, then `git checkout <branch>` to leave
     the working directory as it was.
   - If it's *not* a fast-forward because `<branch>` is simply behind `main` with unrelated commits
     you don't recognize (someone/something else pushed to main), **stop and tell the user** what
     you found rather than merging blind — this is the one case where "push to main automatically"
     doesn't apply, because you'd be guessing at how to reconcile someone else's work.
   - If `main` is behind but nothing on it conflicts with your branch (a plain divergence you can
     resolve), a regular `git merge <branch>` (into main) is fine — but never `--force` anything,
     and never rewrite history that's already on `origin/main`.
5. Double-check the push actually landed: `git rev-parse HEAD` on `main` should equal what you
   pushed, and `git log --oneline -1 origin/main` (after a `git fetch`) should match. This project
   has, in practice, had a second stale local checkout drift out of sync with what got pushed from
   a different working copy — if you know of other local clones of this repo, it's worth a quick
   `git fetch && git status` in them too, but don't go hunting for clones that were never mentioned.

Tell the user what got merged (a one- or two-line summary of the commits) and that it's live on
`origin/main`, not just "done."
