# Tools CI — `.github/workflows/tools-ci.yml`

The data-side gates, automated. Until this workflow the wave's
Python toolchain ran only by hand on the dev VM; now every push or
PR that touches the trees the tools read re-runs them, and a
failure is red in the same place the Rust CI is.

House style follows `ci.yml` / `render.yml`: `ubuntu-22.04` pin
(one runner image for the whole repo), per-ref concurrency with
cancel-in-progress, `actions/checkout@v4`, named steps, comments
wherever a choice isn't obvious. Python is pinned to the stack the
tools are proven under on the dev VM — CPython 3.12
(`setup-python@v5`), `pillow==10.2.0`, `numpy==1.26.4` — because
the caption check is a byte comparison and its renderer stack must
not drift.

## The steps, and what each proves

| Step | Command (essence) | Proves | Failure signal |
|---|---|---|---|
| Film data validator (7/7) | `validate_film.py --root .` | The film manifest, all 14 segment files, atmospheres and caption ownership still satisfy the validator's seven checks (tiling, f32 exactness, R5 atmospheres, …) | Exit 1 = a check failed, 2 = unreadable input; the full report prints either way. Exit 0 *is* the 7/7 verdict by the tool's construction |
| Caption strips determinism | `gen_caption_strips.py --project … --out $RUNNER_TEMP/captions`, then `diff -r` vs `assets/captions` | The committed strips + manifest are exactly what the generator produces from the source project — no hand-edited or stale strip can survive | Any byte differs → `diff` names the file, step fails |
| Native golden pack verify | `make_golden.py` (verify mode) | No pinned golden reference has drifted from its file | Exit 1 iff any item is STALE. MISSING (the 14 film mid-frames, until the film renders) does **not** fail |
| Bridge packager self-verification | `package_bridge.py --repo . --project $RUNNER_TEMP/bridge-project` | The repo's film data still packages into the skill adapter's `native/` layout and passes the adapter-contract checks | Tool exit non-zero → step prints the captured log and fails; the summary line must read `VERIFICATION PASSED — 21/21 checks` |
| Scene JSON parse sweep | inline Python, `json.loads` over `scenes/**/*.json` (32 files today) | No scene data file is truncated or corrupt JSON | Exit 1, each invalid file named. Scenefile-*independent* by design — schema belongs to the Rust loader and `ci.yml`'s build |

Every step fails the job on failure, with the tool's own output in
the log (the bridge step captures to a file precisely so its words
appear on failure as well as success).

## Trigger paths — rationale

`push` + `pull_request`, path-filtered to exactly the trees a step
reads, plus the workflow file itself (so edits to it are exercised
on their own PR) and `workflow_dispatch` for manual runs:

- `tools/**` — the tools themselves, their docs, the golden pins.
- `scenes/**` — validator + JSON sweep inputs.
- `assets/captions/**` — the determinism check's committed side.
- `film/**` — the bridge packager's data side (`film/onoda.json`,
  `film/assets/`). *(Beyond the brief's core list, deliberately:
  a film-data edit can break packaging, so it must trigger the
  step that would catch it.)*
- `docs/golden/**`, `docs/ab/**` — the golden pack's reference
  files (`docs/golden/frames/*.png`, the four milestone
  `native*.png`): re-rendering one changes bytes a pin covers.
  *(Same reasoning as `film/**`.)*
- `painters/**` is **not** a trigger: the painters are staged Rust
  modules — Rust is `ci.yml`'s job, and their test evidence is the
  scratch-harness run recorded in `painters/TEST-RUN.md`. No step
  here reads them.

## Decisions the tools forced

- **Caption determinism is environment-gated — loudly, never
  silently.** The generator's inputs (`script.json`,
  `data/words.json`, `data/lines.json`, `fonts/Inter-Black.otf`)
  live in the film's project dir, **not in this repo**, so a
  GitHub-hosted runner has nothing to regenerate from. The step
  resolves the project from the `CAPTIONS_PROJECT` repository
  variable if set, else the tool's own default
  (`~/faceless-shorts/the-soldier-who-didnt-know`). Project absent
  and var unset → `::warning::` annotation ("NOT verified in this
  run") + exit 0: the absence is on the record, not a silent pass.
  Var set but pointing at nothing → `::error::` + exit 1 (a
  configuration mistake is not an environment fact). Where the
  project exists — the dev VM, a self-hosted runner — the full
  regen + byte-diff runs and fails on any diff. To make the check
  run on GitHub-hosted runners, the source inputs (or a vendored
  captions bundle) would have to enter the repo; that is a content
  decision for the integrator, flagged here, not made by this
  workflow.
- **Two tools write into the repo by default; CI contains both.**
  `gen_caption_strips.py` defaults `--out` to `assets/captions/`
  — the step always passes `--out "$RUNNER_TEMP/captions"`.
  `make_golden.py --pin` rewrites `tools/golden/golden-hashes.json`
  — the workflow runs verify mode only; no run block contains
  `--pin` (pinning is a deliberate human re-baseline act, per
  `docs/golden/NATIVE-GOLDEN.md`).
- **The bridge packager needs its target dir to exist** (it exits
  2, "create it first", otherwise) — the step `mkdir -p`s
  `$RUNNER_TEMP/bridge-project` first. It also refuses to
  overwrite a populated `native/` without `--force`; a fresh temp
  dir makes that moot, and `--force` is never passed.
- **The 21/21 count is asserted, not just exit 0.** If a future
  tool change adds or drops a verification check, this step turns
  red until a human updates the expected count here on purpose —
  the same deliberate-re-baseline discipline as the golden pins.
- **No pip cache.** The two pinned wheels install in seconds;
  `setup-python`'s pip caching wants a requirements file this repo
  doesn't keep, and inventing one for the cache key wasn't worth
  a second source of version truth — the pins live in the
  workflow, once.

## What it deliberately does NOT do

- **No Rust.** No cargo build/test, no painter compilation —
  `ci.yml` owns the crate; the painters join it when the lead
  wires them into `src/`.
- **No renders.** No binary, no lavapipe, no frames — `render.yml`
  owns dispatch renders; golden mid-frames stay MISSING here
  until a render supplies them.
- **No pinning, no writes to the repo.** Every step is read-only
  against the checkout; all outputs land in `$RUNNER_TEMP`. The
  checkout is byte-identical after a green run.
- **No A/B measurement.** `tools/ab/measure_ab.py` needs rendered
  frames (a render artifact), so it belongs with `render.yml`'s
  outputs, not this suite.
- **No schedule.** Data gates guard changes; a cron would only
  re-prove an unchanged tree.

## Local validation (seat W56, before first push)

The YAML was parsed with PyYAML and structurally linted (25
assertions: name, trigger set, push/PR path parity, every trigger
path present, `painters/**` absent, concurrency, runner pin,
timeout, unique step names, action pins, pip pins, no `--pin` in
any run block, every invoked tool file exists) — all pass. Every
step's commands were then executed against this repo:

- validator → `OVERALL: PASS (7/7 checks passed)`, exit 0
- golden verify → `4 OK / 0 STALE / 14 MISSING of 18`, exit 0
- bridge → `VERIFICATION PASSED — 21/21 checks`, exit 0 (after
  the `mkdir` the tool requires)
- JSON sweep → 32 files, 0 invalid, exit 0
- captions (project present on the dev VM) → regenerated 66
  strips + manifest into a temp dir, `diff -r` against
  `assets/captions` **clean — byte-identical**, exit 0; the
  step script's other two branches were exercised from the YAML
  text itself: no-project → `::warning::` + SKIPPED + exit 0;
  `CAPTIONS_PROJECT` set-but-wrong → `::error::` + exit 1

First real run happens on the next push that touches a trigger
path; a green run there is the workflow's own proof.
