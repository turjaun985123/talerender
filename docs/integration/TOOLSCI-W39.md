# TOOLSCI-W39 — Batch-3 validators as a set: verification + unified tools-ci wiring

Seat W39, Wave 2 Batch 4. **One new file (this note); nothing else
written, nothing committed.** All runs below are first-hand at HEAD
`df89ad6` (Batch-3 commit `ff52b3a` + the lead's gpucore site-1
commit), CPython 3.12.3 — the same 3.12 the workflow pins. One
honesty line: the working tree carries the lead's *uncommitted*
`src/scenefile.rs` gpucore edit, and `check_shaders.py` reads the
working tree, so its 21/21 below includes that edit; the W30 gate
already established the same verdict at committed states, and
nothing in this wiring depends on the edit. Wall times were taken
with a Python `perf_counter` harness (`/usr/bin/time` and `bc` are
absent on this VM); they are dev-VM numbers, quoted for CI-cost
order of magnitude only.

The three tools are **stdlib-only** (union of imports across all
three: `argparse, json, math, os, re, sys, pathlib`) — the wiring
needs no change to the workflow's pip-install step.

---

## 1. Real-data runs, in sequence, as CI would invoke them

All run from the repo root (the Actions default working directory).

### 1a. Light-rig validator — real film scenes

```
$ python3 tools/format/check_lights.py --film scenes/film
[6 fire basis] PASS — R1: fire point basis 4.6, not the constructor's 5.0 (warnings)
OVERALL: PASS (6/6 checks passed)
exit=0   wall=0.32s
```

All 14 film scenes pass; **zero warnings on real data** (the
fire-basis check is warnings-tier by design and stays silent —
its tier-B heuristic fires only on an exact constructor 5.0).

### 1b. Asset checker — real film, both manifest and driver trees

```
$ python3 tools/format/check_assets.py --root . scenes/film film
check_assets.py — film dir …/scenes/film (manifest)
[2 references] PASS — 26 references walked (manifest film-dir mode, root=…)
[4 portability] PASS — 17 warning(s), report-only
[6 orphans] PASS — 0 orphan(s), report-only
OVERALL: PASS (6/6 checks passed)
check_assets.py — film dir …/film (driver)
[2 references] PASS — 86 references walked (driver film-dir mode)
[4 portability] PASS — 0 warning(s), report-only
[6 orphans] PASS — 0 orphan(s), report-only
OVERALL: PASS (6/6 checks passed)
exit=0   wall=0.78s
```

The 17 portability warnings (manifest layer resolves against
`--root`/CWD — machine-bound by construction, W28 finding F2) are
**report-only by design and must not fail CI**; see §5.

### 1c. WGSL shader checker — whole tree

```
$ python3 tools/ci/check_shaders.py --repo .
check_shaders: 21/21 shaders PASS, 0 FAIL, 0 UNRESOLVED, 0 unresolved site(s)
exit=0   wall=1.61s
```

**Total real-data cost of all three: 2.71 s.**

---

## 2. Fixture suites and negative controls — failures still fail

### 2a. check_lights fixtures

```
$ python3 tools/format/check_lights.py scenes/fixtures/lights/fx-valid.scene.json \
      scenes/fixtures/lights/fx-fire-basis-warn.scene.json
OVERALL: PASS (6/6 checks passed, 1 warning(s))
exit=0   wall=0.09s
```

Each negative fixture, run singly — all exit 1 on exactly the
check its class names (wall for all seven runs: 1.12 s):

```
fx-grade-unknown-key    exit=1  OVERALL: FAIL (5/6 checks passed)
fx-grade-time           exit=1  OVERALL: FAIL (5/6 checks passed)
fx-atmosphere-inline    exit=1  OVERALL: FAIL (5/6 checks passed)
fx-atmosphere-filmlook  exit=1  OVERALL: FAIL (5/6 checks passed)
fx-light-negative       exit=1  OVERALL: FAIL (5/6 checks passed)
fx-light-nan            exit=1  OVERALL: FAIL (5/6 checks passed)
fx-light-colour-range   exit=1  OVERALL: FAIL (5/6 checks passed)
```

### 2b. check_assets fixtures

```
$ python3 tools/format/check_assets.py scenes/fixtures/assets/valid-scene/mini.scene.json
OVERALL: PASS (5/5 checks passed)     exit=0
$ python3 tools/format/check_assets.py scenes/fixtures/assets/valid-driver
OVERALL: PASS (6/6 checks passed)     exit=0
$ python3 tools/format/check_assets.py scenes/fixtures/assets/missing
OVERALL: FAIL (5/6 checks passed)     exit=1
$ python3 tools/format/check_assets.py scenes/fixtures/assets/wrong-kind
OVERALL: FAIL (4/6 checks passed)     exit=1
$ python3 tools/format/check_assets.py /nonexistent/target
ERROR: check_assets.py: no such file or directory: /nonexistent/target
                                      exit=2
```
(wall for the fixture runs: 0.51 s)

**Operator trap, recorded so CI doesn't fall into it:** the
valid-scene target is the *file* `mini.scene.json`, not its
directory. Passing the directory exits 2 (input error) — correct
behaviour, wrong invocation. The YAML in §4 names the file.

**Check counts are mode-dependent** — 5/5 for a lone scene file
(orphan scan SKIPPED), 6/6 for film dirs. CI must assert exit
codes and per-target OVERALL lines, never one global count
string, for this tool. (check_lights is 6/6 in every mode seen.)

### 2c. check_assets CWD pair (the root rule, proven both ways)

```
$ cd /tmp && python3 …/tools/format/check_assets.py …/scenes/fixtures/assets/cwd/scenes/film
OVERALL: FAIL (4/5 checks passed)     exit=1   (references resolved against --root /tmp — not found)
$ python3 tools/format/check_assets.py --root scenes/fixtures/assets/cwd \
      scenes/fixtures/assets/cwd/scenes/film
OVERALL: PASS (5/5 checks passed)     exit=0
```

### 2d. check_shaders negative controls (scratch copy, real tree untouched)

W29 shipped no in-repo fixtures; its controls were scratch-copy
mutations. Reproduced the same way — `src/` copied to
`/tmp/w39-neg/`, tool run with `--repo .` there:

```
baseline                                   21/21 PASS            exit=0
@binding(1)→(9) in src/shader_bokeh.wgsl   FAIL src/shader_bokeh.wgsl
                                           site FAIL: src/ab1.rs:1060 (run_ab1)
                                           20/21 PASS, 1 FAIL    exit=1
fn fs_main renamed in shader_bokeh.wgsl    FAIL (same site)      exit=1
mutations reverted                         21/21 PASS            exit=0
```

The checker bites, names the shader and the Rust call site, and
UNRESOLVED (a helper shape it cannot interpret) also exits 1 —
coverage loss can never pass silently.

---

## 3. CWD sensitivity — what the CI steps must pin

| Tool | CWD behaviour | CI requirement |
|---|---|---|
| `check_lights.py` | Atmospheres vocabulary resolves relative to the tool's own path (W26); runs correctly from any CWD. | None strictly; still run from repo root for uniformity. |
| `check_assets.py` | **Sensitive by design.** Manifest `segments[].scene`/`sourceData` resolve against `--root`, whose default is the caller's CWD. From the wrong CWD the real film FAILS (that's the loud behaviour the cwd fixture proves). | Step must run at the checkout root **and** pass `--root .` explicitly — never rely on the default. The cwd fixture pair (§2c) is wired as a step so a future "helpful" default change turns red. |
| `check_shaders.py` | `--repo .` is interpreted from the CWD. | Run at the checkout root with `--repo .` explicit (W29's shape). |

GitHub Actions steps start at the checkout root, so the pin is:
no `working-directory:` overrides on these steps, explicit
`--root .` / `--repo .` flags as written in §4.

---

## 4. Paste-ready wiring for `.github/workflows/tools-ci.yml`

Two edits: **(A)** add `src/**` to the trigger paths (see
conflict C1 — without this the shader step never runs on the
changes it exists to catch), **(B)** append the steps after the
existing final step ("Scene JSON parse sweep"). Existing steps
are untouched; style matches the file (comment block above each
step, 6-space step indent, plain `run:`).

### (A) Trigger paths — add one line to BOTH path lists

```yaml
    paths:
      - "tools/**"
      - "scenes/**"
      - "src/**"
      - "assets/captions/**"
      - "film/**"
      - "docs/golden/**"
      - "docs/ab/**"
      - ".github/workflows/tools-ci.yml"
```
(and identically under `pull_request:`)

### (B) Steps to append

```yaml
      # The light rig is data now (scene-format §14): grade keys,
      # the R3 grain-time pin, R5 preset-named atmospheres and the
      # R1 fire basis are enforced here, mechanically, instead of
      # by review. Warnings (fire basis) print but do not fail;
      # exit 1 is a failed check, 2 an input error — both fail
      # the step. No project tree needed: the atmospheres
      # vocabulary resolves relative to the tool's own path.
      - name: Light-rig validator (film scenes)
        run: python3 tools/format/check_lights.py --film scenes/film

      # The fixtures are the validator's own proof: the valid and
      # warnings-only scenes must pass, and every negative fixture
      # must fail — a validator that stops biting turns this red.
      - name: Light-rig validator (fixtures + negative controls)
        run: |
          python3 tools/format/check_lights.py \
            scenes/fixtures/lights/fx-valid.scene.json \
            scenes/fixtures/lights/fx-fire-basis-warn.scene.json
          for f in fx-grade-unknown-key fx-grade-time fx-atmosphere-inline \
                   fx-atmosphere-filmlook fx-light-negative fx-light-nan \
                   fx-light-colour-range; do
            if python3 tools/format/check_lights.py \
                 scenes/fixtures/lights/$f.scene.json; then
              echo "::error::$f unexpectedly PASSED"; exit 1
            fi
          done

      # Every asset reference in the film manifest and the driver
      # data resolves, with the right kind of bytes behind it.
      # --root is EXPLICIT: manifest references resolve against
      # it, and its default is the caller's CWD — a step that
      # relied on the default would pass or fail by accident of
      # where it ran. Portability warnings (17 at HEAD, the
      # manifest layer is root-relative by construction) and the
      # orphan report print but never change the exit code;
      # exit 1 (missing / wrong-kind / a loader silent-fallback
      # made loud) and 2 (input error) fail the step.
      - name: Asset references (film manifest + driver)
        run: python3 tools/format/check_assets.py --root . scenes/film film

      # Fixtures pin the tool's contract, including the root
      # rule itself: the cwd fixture must FAIL from a foreign
      # CWD without --root and PASS with it. Check counts are
      # mode-dependent (5/5 lone scene, 6/6 film dir), so this
      # step asserts exit codes, not a count string. The
      # valid-scene target is the scene FILE, not its directory
      # (the directory is an input error, exit 2).
      - name: Asset checker (fixtures + CWD root rule)
        run: |
          python3 tools/format/check_assets.py scenes/fixtures/assets/valid-scene/mini.scene.json
          python3 tools/format/check_assets.py scenes/fixtures/assets/valid-driver
          for t in missing wrong-kind; do
            if python3 tools/format/check_assets.py scenes/fixtures/assets/$t; then
              echo "::error::assets fixture $t unexpectedly PASSED"; exit 1
            fi
          done
          if (cd "$RUNNER_TEMP" && python3 "$GITHUB_WORKSPACE/tools/format/check_assets.py" \
                "$GITHUB_WORKSPACE/scenes/fixtures/assets/cwd/scenes/film"); then
            echo "::error::cwd fixture unexpectedly PASSED without --root"; exit 1
          fi
          python3 tools/format/check_assets.py --root scenes/fixtures/assets/cwd \
            scenes/fixtures/assets/cwd/scenes/film

      # WGSL ↔ Rust structural agreement: entry points, binding
      # numbers/kinds against the Rust bind-group layouts, and
      # hand-packed uniform sizes. Exit 1 covers FAIL and
      # UNRESOLVED alike — a refactor that changes a helper's
      # shape drops coverage loudly instead of silently. Reads
      # src/**, which is why src/** is in this workflow's trigger
      # paths. If a legitimate new helper shape appears, extend
      # the interpreter in check_shaders.py; never weaken a
      # verdict.
      - name: WGSL shader structural check
        run: python3 tools/ci/check_shaders.py --repo .
```

### Expected CI output lines (verified at `df89ad6`)

| Step | Expected headline(s) |
|---|---|
| Light-rig validator (film scenes) | `OVERALL: PASS (6/6 checks passed)` |
| Light-rig validator (fixtures + negative controls) | `OVERALL: PASS (6/6 checks passed, 1 warning(s))`, then 7× `OVERALL: FAIL (5/6 checks passed)` inside the loop (expected; the step fails only if one *passes*) |
| Asset references (film manifest + driver) | 2× `OVERALL: PASS (6/6 checks passed)`; `[4 portability] PASS — 17 warning(s), report-only` on the manifest target |
| Asset checker (fixtures + CWD root rule) | `OVERALL: PASS (5/5…)` (valid-scene, cwd-with-root), `OVERALL: PASS (6/6…)` (valid-driver), `OVERALL: FAIL…` for missing/wrong-kind/cwd-without-root (expected; asserted) |
| WGSL shader structural check | `check_shaders: 21/21 shaders PASS, 0 FAIL, 0 UNRESOLVED, 0 unresolved site(s)` |

Added CI cost at HEAD: ≈ 4.4 s of tool time total (2.71 s real
data + ≈ 1.7 s fixtures) plus the `src/**` trigger making this
job also run on Rust-only pushes — the job is checkout + seconds
of Python, so the cost is a runner minute, not a queue problem.

---

## 5. Failure semantics — what blocks, what only prints

| Tool | Exit | Meaning | CI effect |
|---|---|---|---|
| check_lights | 0 | All checks pass. Fire-basis **warnings may be present** (fixtures step shows 1) and do not fail. | pass |
| check_lights | 1 | A check failed (bad grade key, `grade.time ≠ 0`, inline atmosphere, out-of-range light…). | **blocks** |
| check_lights | 2 | Input error (unreadable/missing file). | **blocks** (wiring error) |
| check_assets | 0 | All references resolve with correct kinds. **Portability warnings (check 4) and orphans (check 6) are report-only and never move this code** — the real film passes *with* 17 warnings. | pass |
| check_assets | 1 | Missing reference, wrong-kind bytes, or a loader silent-fallback guard tripped. | **blocks** |
| check_assets | 2 | Input error (bad target, incl. the directory-instead-of-file trap). | **blocks** (wiring error) |
| check_shaders | 0 | Every shader PASS, zero UNRESOLVED. | pass |
| check_shaders | 1 | A shader FAIL **or** any site UNRESOLVED (coverage dropped). One code for both, deliberately (W29): never split or whitelist it. | **blocks** |

House pattern is consistent across all three (0 pass / 1 check
failure / 2 input error, matching `validate_film.py`), so the
steps gate on the exit code as-is — no output scraping needed.

---

## 6. Conflicts and gaps between the three seats' hookup shapes

- **C1 — W29's trigger claim is wrong against the current
  workflow (the one real conflict).** SHADERS-W29 §9 says
  tools-ci "already runs on `src/**`-touching changes" and
  suggests relying on "the workflow's existing push/PR paths".
  It does not: the path lists at `df89ad6` are `tools/**`,
  `scenes/**`, `assets/captions/**`, `film/**`, `docs/golden/**`,
  `docs/ab/**` and the workflow file — **no `src/**`**, while
  check_shaders' entire input is `src/**`. Wiring the step
  without edit (A) would produce a gate that runs on data pushes
  but *never on the shader/Rust changes it exists to catch*.
  Resolution proposed (not silently picked): add `src/**` to
  both lists, as in §4(A). Side effect, stated openly: tools-ci
  will then also run on Rust-only pushes.
- **C2 — W28 shipped no hookup shape.** ASSETS-W28 documents the
  tool, its run command and its exit semantics but contains no
  CI YAML at all (unlike W26's three steps and W29's one). The
  two assets steps in §4 are derived from the note's documented
  real run (`--root . scenes/film film`) and fixture record —
  the lead should treat them as this seat's proposal, not as
  W28's.
- **C3 — Step-shape divergence, no contradiction.** W26 proposes
  three steps, W29 one, W28 none. §4 keeps W26's granularity
  (folding its fixture + negative steps into one to keep the
  job readable) and W29's single step verbatim in behaviour.
  W26's negative loop relies on the `if cmd; then` exemption
  from Actions' default `bash -e` — standard, and the same
  pattern is reused for the assets negatives.
- **C4 — No global check-count assertion is safe for
  check_assets** (5/5 vs 6/6 by target mode, §2b). The existing
  workflow *does* assert a count string for the bridge packager
  ("21/21 checks"); do not copy that pattern onto the assets
  steps. For check_shaders the 21/21 line is the tool's own
  summary and is safe to quote in expectations, but the step
  still gates on exit code alone.
- **C5 — check_shaders negative controls are deliberately NOT
  wired.** They require mutating a scratch copy of the tree
  (§2d); W29 did not propose CI wiring for them, and this seat
  does not invent it. The control is re-run by hand (or by a
  future seat) when the checker changes; the CI gate is the
  real-tree run plus the UNRESOLVED-fails rule.
