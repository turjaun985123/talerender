# TaleRender v1.0.1 — patch-release pack (W46)

**Status: PREPARATION ONLY — this pack does not authorise a release.**

Prepared from repo HEAD **`f6616aa3da6b062a6f29da17e4ef9ec7533ee362`**
(`f6616aa`, 2026-11-14). The changelog in §2 is complete for
`3c2c129..f6616aa` (20 commits, verified with `git log` below). Wave 2 is
still running, so the release executor **must** extend the changelog and
re-run every release check against the final release HEAD. Do not tag or
publish from `f6616aa` merely because it is the snapshot used here.

## 1. Why v1.0.1 exists

The published v1.0.0 binary is no longer the only binary calling itself
`talerender 1.0.0`:

| Copy identified by the gap analysis on the user's PC | Size | SHA-256 evidence |
|---|---:|---|
| `./talerender` copy (the v1.0.0 release build) | 19,552,576 B | full digest `1d669857bbda21a9c13d60658a4b4a4593d07bf7c6d4bd7ea88a5b1383771a84` |
| `~/.faceless-shorts/talerender-bin/talerender` (later CLI-era build) | 19,548,480 B | first 16 hex digits `258dce5d56c940de` in the analysis; full digest not recorded there |

Both report version `1.0.0`, but they are different builds with different
behaviour — most visibly, the v1.0.0 release binary predates the
professional CLI and rejects `--help`, while the later build implements
it. That is the ops failure this patch release closes:

- one version bump, `1.0.0 → 1.0.1`;
- one CI-built binary, taken from one identified green CI run;
- one full SHA-256 recorded in the release notes and checked again after
  upload;
- one canonical PC install path, with every other known copy replaced by
  the same bytes (§6).

The v1.0.0 release itself stays published and untouched. Its release is
at <https://github.com/turjaun985123/talerender/releases/tag/v1.0.0>,
targeted at `3c2c129`, with the binary asset digest above (verified via
the GitHub API while preparing this pack).

---

## 2. Changelog: v1.0.0 (`3c2c129`) → preparation HEAD (`f6616aa`)

Verification command used for this section:

```bash
git log --reverse --format='%h %s' 3c2c129..HEAD
git rev-list --count 3c2c129..HEAD   # 20 at f6616aa
```

All 20 commits in the range are accounted for below. Entries are grouped
by what a user/operator can observe; internal-only commits are labelled
as such rather than dressed up as features.

### 2.1 CLI, errors, and film behaviour

- Professional CLI: full `-h`/`--help` flag reference, `-V`/`--version`, and panic-free command-line errors (`error: …` + help hint, exit 2) — `d7bebdd`.
- Film-telemetry exit codes now agree with the authoritative verdict line: 0 = CHECK OK, 1 = CHECK FAILED, 3 = could-not-run; 2 remains CLI-only — `aa10b13`.
- Caption gate fixed by data: the 66-chunk layout was re-chunked to 73 chunks and the regenerated driver data re-ran as **`FILM TELEMETRY: CHECK OK`, exit 0** — `3a36f1f`, `5e90d7e`.
- Structured errors, migration step 1: film loading and image save return `TaleError` with audit exit codes (DataLoad 2, ImageIo 4) instead of panicking — `ba88596`.
- Paint cache gained a byte budget (`TALERENDER_PAINT_BUDGET_MB`, documented in `--help`) with end-of-run cache statistics, and film runs gained elapsed/ETA progress plus a completion summary with peak RSS — `86f0c81`.
- Generic film entry: `--film` now accepts `film.json` as the entry file with `onoda.json` retained as this film's fallback, and the help text was corrected to match the loader — `aadc193`.
- Render dispatch gained an optional `paint_budget_mb` input (empty = binary default) so the forced-eviction byte-probe leg is dispatchable — `fbf37c0`.

**Boundary on the `film.json` line:** `aadc193` closes the filename/help
mismatch for the repo's film-data layout (segment scenes still resolve
from `scenes/film/`, and the Onoda fallback remains). It does **not** by
itself make `--film` a loader for an arbitrary skill project; generic
project support remains the scene-file lane + skill-wiring exporter
route. Release notes must not claim more than this.

### 2.2 Engine modules and architecture

- Foundation modules authored with standalone tests: `errors`, `paintbudget`, `gpucore`, `runlog` — `3a36f1f`.
- Animation foundations authored: animation evaluator (`anim`), character channels (`charanim`), render graph (`rendergraph`), texture atlas (`atlas`), buffer pool (`bufferpool`), plus prefab/clip format-extension specs and validators — `1a62161`.
- First module set registered in the crate and carried by `cargo test` (94 tests at registration): `anim`, `atlas`, `bufferpool`, `charanim`, `gpucore`, `paintbudget`, `rendergraph`, `runlog` — `2328e6f`.
- Test-only fix: the gpucore strip-padding test's payload pattern aliased its own `0xEE` sentinel at x=238; pattern bounded, test intent unchanged — `38930fa`.
- GPU-core migration completed 5/5 — dof, scenefile, ab1, main, and film all create devices/read back through `gpucore`, and the last duplicate `request_adapter` helper was deleted — `df89ad6`, `c420d72`, `9f497c7`, `dc0e555`, `aadc193`.

**Registration caveat the prompt's list gets ahead of:** at `f6616aa`,
`src/locomotion.rs` and `src/cameramoves.rs` exist as standalone,
self-tested modules (authored in `ff52b3a`), but `src/main.rs` contains
**no** `mod locomotion;` or `mod cameramoves;` declaration — they are
not yet registered in the crate. Before the release notes are finalised
the lead must either (a) register them and let CI carry their tests, or
(b) describe them accurately as standalone modules, not registrations.
Do not copy the unqualified phrase "module registrations
(… locomotion/cameramoves)" into the published notes while this stands.

**Adoption caveat for the other modules:** registration means the code
is compiled and its tests run in CI. It does not mean every module is
wired into the film path — per-module call-site adoption remains gated
separately (see `WAVE2-RECORD.md` §9). The published changelog should
say "compiled library modules", not imply new film behaviour.

### 2.3 Data, QA gates, CI, and operator tooling

- Determinism contract + dispatch-only determinism workflow added, and the 14 f021991 film frames re-staged as golden *candidates* (pins deliberately not struck) — `3a36f1f`.
- Formal determinism evidence folded into the record: the CI-lavapipe pack's first dispatch, run `43617370962`, completed **SUCCESS** — `11a7a2e` (workflow from `3a36f1f`).
- Dispatch-only visual-regression workflow (80 dB band) and PC bench harness added — `1a62161`.
- Batch-3 tooling: lip-sync viseme builder (91.1% dictionary coverage on the real Onoda words; output passes the clips validator 5/5), light-rig validator (6/6 over all 14 real scenes), asset checker (112 references, 0 missing), WGSL structural checker (21/21), and an incremental re-render planner proven exact against the real re-chunk (296/1,811 frames) — `ff52b3a`.
- Tools CI now runs the light-rig, asset, and WGSL validators, with `src/**` added to both trigger path lists — `630d82e` (but see the red-run blocker in §5 before calling this "green").
- Batch-4 operator packs: paste-ready scene-format §15 draft, anim-adoption staging tool (63/65 key-carrying records convert; output passes the clips validator 5/5), blink-cost decision evidence, gpucore byte-probe pack, errors step-2 pack, consolidated PC RADV evidence runbook, visual-regression triage, and doc-fix pack — `f6616aa`.

**§15 status flag:** at `f6616aa` the §15 text is a paste-ready draft
(`docs/integration/FORMAT15-DRAFT-W31.md`), not yet folded into the
canonical `docs/scene-format.md`. The release notes may list the draft
as tooling/documentation progress only.

### 2.4 Internal record only (no user-visible change)

- Wave 2 lead record created: mission, laws, stale-item register, caption tension, exit-code contract — `6f00400`.
- Wave 2 verdicts and Batch-2 adoption decisions folded into the record (E6 determinism verdict, caption CHECK OK headline, adapter re-proof) — `11a7a2e`.

### 2.5 User-visible changes not in the original request list

Flagged so the published notes are complete, not just responsive:

1. **Golden candidates were re-staged** (`3a36f1f`) — 14 frames from the
   f021991 render now sit in `docs/golden/frames-f021991/` as pin
   candidates. The pins themselves remain separate human acts.
2. **Two new dispatch-only QA workflows** (determinism, visual
   regression) and a **bench workflow** exist; none runs on ordinary
   pushes, and the visual-regression first dispatch is still expected
   to report the two known stale pins (see §5/§7 — not a v1.0.1 claim).
3. **A new environment knob** (`TALERENDER_PAINT_BUDGET_MB`) and a new
   render-dispatch input (`paint_budget_mb`) are operator-visible
   surface (§2.1).
4. **New Python tools** beyond the validators: lip-sync builder,
   re-render planner, anim staging converter, byte-probe runner,
   blink-cost analyser, VR triage (§2.3).
5. **The old proof MP4 must not be re-attached to v1.0.1.** The v1.0.0
   asset `onoda-native-f021991.mp4` predates the 73-chunk caption data;
   attaching it to v1.0.1 would present a stale-caption film as the new
   release's proof. v1.0.1 is a binary patch release (§4).

---

## 3. Version-bump points

### 3.1 The one functional bump

| File:line | Current | Action |
|---|---|---|
| `Cargo.toml:3` | `version = "1.0.0"` | **Change to `version = "1.0.1"`. This is the only version literal that drives the binary.** |

There is **no tracked `Cargo.lock`** in this repo, so there is no
lockfile version entry to bump. (If a local build has generated one, it
is untracked build state, not a release source.)

### 3.2 Derived — no edit needed, verify after the bump

| File:line | Surface | Why no edit |
|---|---|---|
| `src/main.rs:171` | `println!("talerender {}", env!("CARGO_PKG_VERSION"))` | `--version` output is compiled from the Cargo version; the bump in §3.1 changes it automatically. Post-bump smoke must print exactly `talerender 1.0.1`. |
| `src/main.rs:100-101` | `-h, --help` / `-V, --version` help lines | Documents the flags; contains no version literal. |
| `.github/workflows/bench.yml:80` | `./target/release/talerender --version` in the bench log | Invokes the flag; no literal. Its output becomes 1.0.1 automatically. |
| `docs/runbook/USER-PC.md:174` | `--version` named in the verified-flag list | Flag name only; no literal. |

### 3.3 Literal `1.0.0` audit — historical/minimum references, do NOT rewrite

A tracked-tree `git grep -n '1\.0\.0'` at `f6616aa` returns the
`Cargo.toml` line above plus the following. **None of these is a bump
point.** A global search-and-replace would falsify dated records; the
lead should instead append a dated v1.0.1 release entry to
`WAVE2-RECORD.md` at close-out.

| File:line | Text (abridged) | Disposition |
|---|---|---|
| `docs/integration/DETERMINISM-CONTRACT-W04.md:241` | `--version   # sanity: 1.0.0 or later` | Minimum-version sanity check; 1.0.1 satisfies it. No edit. |
| `docs/runbook/PC-EVIDENCE-W36.md:90` | `--version    # sanity: 1.0.0 or later` | Same minimum check. No edit. |
| `docs/integration/WAVE2-RECORD.md:22` | "version 1.0.0; CI green at launch" | Dated launch-HEAD statement. Keep; append the release row at close. |
| `docs/integration/EXIT-CODES-W02.md:124` | "captured with the v1.0.0 binary (old convention)…" | Historical description of the pre-exit-code binary generation. Keep. |
| `docs/runbook/PERF.md:25` | "`--help` and `--version` (added after v1.0.0)" | Historically true statement. Keep. |
| `docs/runbook/USER-PC.md:13` | "professionalised after v1.0.0; use a binary from the CI run" | Historically true statement. Keep. |
| `docs/integration/DOCFIX-W38.md:111` | "Cargo 1.0.0); structured errors migration step 1…" | Historical changelog prose inside a fix pack. Keep. |
| `docs/integration/DOCFIX-W38.md:381` | "the version (`talerender 1.0.0`)" | Historical description of the CLI commit's output. Keep. |
| `docs/integration/DOCFIX-W38.md:387` | "Cargo version 0.1.0 → 1.0.0 in the same commit" | Historical fact about `d7bebdd`. Keep. |
| `docs/integration/DOCFIX-W38.md:422` | "professionalised after v1.0.0; use a binary from…" | Quoted paste-ready text recording the same history. Keep. |
| `docs/integration/DOCFIX-W38.md:428` | "after v1.0.0); unknown arguments print `error: …`" | Quoted paste-ready text recording the same history. Keep. |
| `docs/integration/DOCFIX-W38.md:579` | `grep -n "added after v1.0.0" docs/runbook/PERF.md` | A verification pattern for the historical wording. Keep. |

`README.md` at `f6616aa` contains no `1.0.0` literal and no `--version`
mention (the DOCFIX-W38 pack's proposed README text has not been
applied), so the README is not a bump point. If the lead applies that
pack before the freeze, re-run the grep and re-audit before tagging.

Created by the procedure itself (not source files): the git tag /
GitHub release **`v1.0.1`** and the release title **`TaleRender v1.0.1`**.

---

## 4. Release procedure (mirrors the v1.0.0 method)

v1.0.0 method, for reference: the binary asset was the
`talerender-binary` artifact of a green CI run (`43613715321`) — CI
uploads it from `target/release/talerender` (`.github/workflows/ci.yml`,
"Upload binary artifact" step) — downloaded locally, verified as a
working binary, its SHA-256 recorded in the notes, then uploaded to a
directly-published (non-draft, non-prerelease) GitHub release. v1.0.1
follows the same shape, with the stronger verification commands below.
**No local `cargo build` is a release source** — builds happen in CI
(repo law; this VM has no cargo in PATH).

### Step 0 — Freeze and authorisation gate

Do not start until **every** item in §5 is green/complete, including the
user's explicit go. Then:

```bash
cd ~/workspace/talerender
git fetch origin
git rev-parse HEAD                 # note the pre-bump feature-freeze SHA
git status --porcelain             # no tracked modifications may remain
```

Work from a clean checkout/worktree for the release steps: concurrent
wave work leaves untracked artefacts (e.g. `__pycache__`) in the shared
tree, and none of them may leak into the bump commit.

### Step 1 — Version bump commit (lead only)

```bash
# Cargo.toml:3 only:
#   version = "1.0.0"  ->  version = "1.0.1"
git add Cargo.toml
git commit -m "Release: bump version to 1.0.1"
git push origin main
RELEASE_SHA=$(git rev-parse HEAD)  # the bump commit IS the release target
```

### Step 2 — Extend the changelog to the release target

```bash
git log --reverse --format='%h %s' 3c2c129.."$RELEASE_SHA"
git log --reverse --format='%h %s' f6616aa.."$RELEASE_SHA"   # the delta this pack has not seen
```

Fold every commit in the second list into the release notes (grouped as
in §2), and record the final count. If the delta includes a revert of
the caption re-chunk (user veto at wave close), the §2.1 caption entry
and the telemetry expectation in Step 4 change with it — follow the
final tree, not this pack.

### Step 3 — Prove CI + Tools CI green on the exact release SHA

```bash
REPO=turjaun985123/talerender
CI_RUN_ID=$(gh run list --repo "$REPO" --commit "$RELEASE_SHA" \
  --workflow ci.yml --limit 1 --json databaseId --jq '.[0].databaseId')
TOOLS_RUN_ID=$(gh run list --repo "$REPO" --commit "$RELEASE_SHA" \
  --workflow tools-ci.yml --limit 1 --json databaseId --jq '.[0].databaseId')

test "$(gh run view "$CI_RUN_ID"    --repo "$REPO" --json conclusion --jq .conclusion)" = "success"
test "$(gh run view "$TOOLS_RUN_ID" --repo "$REPO" --json conclusion --jq .conclusion)" = "success"
```

A cancelled run is not a success; a success on any other SHA is not
evidence for this release. If either lookup returns a run whose
`headSha` is not `$RELEASE_SHA`, stop.

### Step 4 — Download the CI binary and verify it BEFORE upload

```bash
RELEASE_DIR=$(mktemp -d ~/workspace/.release-v101.XXXXXX)
gh run download "$CI_RUN_ID" --repo "$REPO" \
  -n talerender-binary -D "$RELEASE_DIR/artifact"
BIN="$RELEASE_DIR/artifact/talerender"
chmod +x "$BIN"

file "$BIN"                        # must be an x86-64 ELF executable
stat -c '%n %s bytes' "$BIN"
RELEASE_BIN_SHA256=$(sha256sum "$BIN" | awk '{print $1}')
echo "$RELEASE_BIN_SHA256"         # goes into the notes verbatim (full 64 hex)

# Version smoke — the whole point of the patch release:
test "$("$BIN" --version)" = "talerender 1.0.1"

# Help smoke:
"$BIN" --help > "$RELEASE_DIR/help.txt"
grep -q -- '--film <DIR>'                 "$RELEASE_DIR/help.txt"
grep -q -- '--film-telemetry <DIR>'       "$RELEASE_DIR/help.txt"
grep -q -- 'TALERENDER_PAINT_BUDGET_MB'   "$RELEASE_DIR/help.txt"
grep -q -- 'EXIT CODES:'                  "$RELEASE_DIR/help.txt"

# Bad-input smoke — must be a clean CLI error, exit 2, no panic:
set +e
"$BIN" --release-smoke-bad-flag >"$RELEASE_DIR/badflag.out" 2>"$RELEASE_DIR/badflag.err"
BAD_RC=$?
set -e
test "$BAD_RC" -eq 2
grep -q '^error: ' "$RELEASE_DIR/badflag.err"
! grep -qi 'panic' "$RELEASE_DIR/badflag.err"

# Telemetry smoke on the real film data (CPU-only; run from the repo
# root of the RELEASE_SHA checkout so `film/` is the current data):
set +e
"$BIN" --film-telemetry film > "$RELEASE_DIR/telemetry.txt" 2>"$RELEASE_DIR/telemetry.err"
TEL_RC=$?
set -e
grep -Fx 'FILM TELEMETRY: CHECK OK' "$RELEASE_DIR/telemetry.txt"
test "$TEL_RC" -eq 0
```

If the telemetry line is not exactly `FILM TELEMETRY: CHECK OK` with
exit 0, stop — do not publish a patch release whose headline gate has
regressed, and do not "fix" the expectation in the notes.

### Step 5 — Release notes (shape mirrors v1.0.0)

Write `$RELEASE_DIR/notes.md` from this skeleton, replacing every
`<…>` from the verified values above and the final-tree status:

```markdown
Patch release. TaleRender is a native Rust + wgpu renderer for Tall
Tale Shorts. v1.0.1 exists to end a version-identity failure: two
different builds were both reporting `talerender 1.0.0`. There is now
exactly one v1.0.1 binary, built by CI from HEAD `<RELEASE_SHA>`.

## What changes since v1.0.0

- Professional CLI: `-h`/`--help` with the full flag reference,
  `-V`/`--version`, and panic-free CLI errors (exit 2). The published
  v1.0.0 binary predates all of this — it rejects `--help`.
- `--film-telemetry` exit codes now agree with the verdict line
  (0 OK / 1 CHECK FAILED / 3 could-not-run); the line stays
  authoritative.
- Caption gate is green by data: the film's captions were re-chunked
  66 → 73 and the gate prints `FILM TELEMETRY: CHECK OK` (exit 0).
- `--film` accepts `film.json` as the generic entry file
  (`onoda.json` remains as this film's fallback); help text matches
  the loader.
- Structured errors (step 1) for film loading and image save;
  paint-cache byte budget (`TALERENDER_PAINT_BUDGET_MB`) with cache
  statistics; film progress with elapsed/ETA and a peak-RSS summary.
- Architecture: shared GPU core across all five call sites;
  compiled library modules for animation, character channels, render
  graph, atlas, buffer pool, paint budget, run log and structured
  errors <plus locomotion/camera moves ONLY if registered by the
  final HEAD — see pack §2.2>.
- QA/CI: determinism contract + dispatch job (CI-lavapipe leg PASS),
  visual-regression and bench workflows, and Tools CI validators
  (light rig, assets, WGSL structure).

## Asset

- `talerender` — CI-built binary (ubuntu-22.04, CI run `<CI_RUN_ID>`,
  from HEAD `<RELEASE_SHA>`); sha256 `<RELEASE_BIN_SHA256>`.
  Verify after download:
  `echo "<RELEASE_BIN_SHA256>  talerender" | sha256sum -c -`
  and `talerender --version` → `talerender 1.0.1`.
  Requires a Vulkan driver (hardware RADV/ANV or software lavapipe).
  No display server needed.

No MP4 is attached to this patch release: the v1.0.0 proof film
predates the caption re-chunk and is not re-issued as v1.0.1 proof.

## Known limits (named, not hidden)

- <Final-tree status, copied from the completed WAVE2-RECORD —
  e.g. RADV proof state, golden-pin state, skill-adoption state.
  Do not copy v1.0.0's limit text forward unchecked.>

Full record: `docs/integration/WAVE2-RECORD.md`
```

### Step 6 — Publish (only on the user's explicit go)

```bash
# Guard: neither the release nor a same-named tag may already exist.
if gh release view v1.0.1 --repo "$REPO" >/dev/null 2>&1; then
  echo "v1.0.1 already exists — STOP"; exit 1
fi

gh release create v1.0.1 "$BIN" --repo "$REPO" \
  --target "$RELEASE_SHA" \
  --title "TaleRender v1.0.1" \
  --notes-file "$RELEASE_DIR/notes.md"
```

Published directly (not draft, not prerelease), mirroring v1.0.0.

### Step 7 — Verify the published release independently

```bash
# The tag resolves to the release commit:
test "$(gh api "repos/$REPO/commits/v1.0.1" --jq .sha)" = "$RELEASE_SHA"

# Published, not draft/prerelease:
gh release view v1.0.1 --repo "$REPO" \
  --json tagName,isDraft,isPrerelease,url \
  --jq -e '.isDraft == false and .isPrerelease == false'

# GitHub's recorded asset digest equals the pre-upload digest:
REMOTE_DIGEST=$(gh api "repos/$REPO/releases/tags/v1.0.1" \
  --jq '.assets[] | select(.name == "talerender") | .digest')
test "$REMOTE_DIGEST" = "sha256:$RELEASE_BIN_SHA256"

# Fresh download, fresh check:
VERIFY_DIR=$(mktemp -d ~/workspace/.release-v101-verify.XXXXXX)
gh release download v1.0.1 --repo "$REPO" --pattern talerender -D "$VERIFY_DIR"
echo "$RELEASE_BIN_SHA256  $VERIFY_DIR/talerender" | sha256sum -c -
test "$("$VERIFY_DIR/talerender" --version)" = "talerender 1.0.1"
```

Finally, append the release row (tag, target SHA, CI run id, Tools CI
run id, full binary SHA-256, release URL) to `WAVE2-RECORD.md` §7/§8 as
part of the wave close-out.

---

## 5. What must be true before release — status at preparation time

| # | Requirement | Evidence at `f6616aa` | Status |
|---|---|---|---|
| 1 | **User's explicit go** for the v1.0.1 release, given at wave close | This pack was ordered as preparation; no release go is recorded | ❌ NOT MET |
| 2 | Final HEAD frozen (all wave batches + fixes landed; nothing mid-flight) | Snapshot is mid-Wave-2 (Batch 4 the last central commit) | ❌ NOT MET |
| 3 | Version bumped to 1.0.1 | `Cargo.toml:3` still `version = "1.0.0"` | ❌ NOT MET |
| 4 | CI green on the exact release SHA | At the snapshot SHA: CI run `43622186461` = **SUCCESS** — but this must be re-proven on the bump commit's SHA | ⚠️ MET FOR SNAPSHOT ONLY |
| 5 | Tools CI green on the exact release SHA | At the snapshot SHA: Tools CI run `43622186445` = **FAILURE** (see below) | ❌ NOT MET |
| 6 | `WAVE2-RECORD.md` complete through the final HEAD | §7 verdict log ends at `86f0c81`; §8 ledger still shows Batch 3 as "seats running" and Batches 4–10 unfilled; the gpucore 5/5 migration, tools-ci wiring, and Batch 3/4 verdicts are absent | ❌ NOT MET |
| 7 | Changelog extended to the final HEAD | §2 covers `3c2c129..f6616aa` (20 commits); later commits unseen | ❌ NOT MET |
| 8 | Caption disposition final | Re-chunk landed and the gate is CHECK OK, but the record keeps the user's wave-close veto open; a veto would change the tree and §2 | ⏳ PENDING CLOSE DECISION |
| 9 | Release-notes claims match the final tree | §2.2's locomotion/cameramoves registration caveat is unresolved at the snapshot | ❌ NOT MET |

**The Tools CI failure, precisely (run `43622186445`, step "Asset
references (film manifest + driver)"):** the newly wired step runs
`python3 tools/format/check_assets.py --root . scenes/film film` on a
hosted runner, where the film manifest's absolute project references —
`project`, `audio.projectPath`, and `audio.file` under
`/home/hatch/faceless-shorts/the-soldier-who-didnt-know` — do not exist,
so the references check fails (manifest side `OVERALL: FAIL (4/5)`) and
the following steps are skipped. The same command passes on the dev VM,
where that project tree exists (W39's local verification). This is a
wiring/environment mismatch the lead must resolve — by scoping the
checker/step or by running that leg where the project exists — **not**
by re-running until it happens to pass, and not by deleting the step
silently. Until Tools CI is green on the release SHA, there is no
release.

**Not prerequisites** (do not invent blockers, but the notes must state
their final status honestly): PC RADV evidence, the golden pin acts,
and skill adoption. None was made a condition of this patch release;
v1.0.1 must not claim any of them as done unless the completed
`WAVE2-RECORD.md` says so.

---

## 6. PC cleanup — one canonical v1.0.1 binary

For the user's PC, after v1.0.1 is published. Goal state: the canonical
binary is installed once, every other known copy carries the **same
bytes**, and the skill adapter is configured to use the canonical path
explicitly.

### 6.1 Which path is canonical, and why

The skill-wiring adapter (`template/tools/capture_native.mjs`,
`findBinary()`) resolves the binary in this order:

1. **`TALERENDER_BIN`** — explicit; wins outright, and a bad explicit
   path is a hard error (never second-guessed);
2. `<project>/native/talerender`;
3. `~/workspace/talerender/talerender` (repo-checkout fallback — the
   work-copy operator doc, `template/tools/CAPTURE-NATIVE.md`, warns
   this path "holds whatever build happens to sit in the checkout" and
   silently selected a stale build in every local wave run);
4. `PATH`.

The work copy's documented provisioning route
(`template/tools/CAPTURE-NATIVE.md` §"Prerequisites", and
`docs/NATIVE-BACKEND.md`) is therefore the canonical setup:

- **Canonical path: `~/talerender-bin/talerender`**
- **Selected by: `export TALERENDER_BIN=~/talerender-bin/talerender`**

Replacing stray copies without setting `TALERENDER_BIN` is not enough:
discovery could still pick a project or checkout copy first.

### 6.2 Inventory first (see what exists before touching anything)

```bash
# Known locations from the gap analysis / tasking:
ls -l ~/.faceless-shorts/talerender-bin/talerender \
      ~/.faceless-shorts/talerender \
      ~/talerender-bin/talerender 2>/dev/null

# Every exact-name copy under the skill hub (catches per-project
# <project>/native/talerender slots too):
find ~/.faceless-shorts -type f -name talerender -print

# Anything PATH would resolve:
type -a talerender || true
```

### 6.3 Install the canonical binary from the published release

Use the full SHA-256 from the v1.0.1 release notes — never a prefix.

```bash
RELEASE_SHA256='<full 64-hex sha256 from the v1.0.1 release notes>'
CANON="$HOME/talerender-bin/talerender"
STAGE="$HOME/Downloads/talerender-v1.0.1"
mkdir -p "$HOME/talerender-bin" "$STAGE"

gh release download v1.0.1 --repo turjaun985123/talerender \
  --pattern talerender --dir "$STAGE"
cd "$STAGE"
echo "$RELEASE_SHA256  talerender" | sha256sum -c -     # must say: OK
install -m 0755 "$STAGE/talerender" "$CANON"

export TALERENDER_BIN="$CANON"
test "$("$TALERENDER_BIN" --version)" = "talerender 1.0.1"
```

Make the selection persistent for capture shells (add once):

```bash
grep -qxF 'export TALERENDER_BIN="$HOME/talerender-bin/talerender"' ~/.bashrc \
  || echo 'export TALERENDER_BIN="$HOME/talerender-bin/talerender"' >> ~/.bashrc
```

### 6.4 Replace every other known copy with the same bytes

Backups get a `.pre-v1.0.1` suffix — a different filename, so no
discovery rule can ever select a backup by accident.

```bash
# The two gap-analysis locations + the adapter's checkout fallback:
for p in \
  "$HOME/.faceless-shorts/talerender-bin/talerender" \
  "$HOME/.faceless-shorts/talerender" \
  "$HOME/workspace/talerender/talerender"
do
  if [ -f "$p" ]; then
    cp -a "$p" "$p.pre-v1.0.1"
    install -m 0755 "$CANON" "$p"     # also fixes the old non-executable copy
    echo "replaced: $p (backup at $p.pre-v1.0.1)"
  fi
done

# Any remaining exact-name copies under the hub (project native slots):
while IFS= read -r -d '' p; do
  cp -a "$p" "$p.pre-v1.0.1"
  install -m 0755 "$CANON" "$p"
  echo "replaced: $p"
done < <(find "$HOME/.faceless-shorts" -type f -name talerender -print0)
```

If `type -a talerender` in §6.2 showed a PATH copy outside these
locations, apply the same backup + `install` to that path as well — or
remove it; do not leave a fourth, divergent binary on PATH.

### 6.5 Verify the end state

```bash
# Canonical:
sha256sum "$CANON"                     # first field == $RELEASE_SHA256
"$CANON" --version                      # talerender 1.0.1

# Every surviving copy under the hub is the same bytes — this find
# must print NOTHING:
find "$HOME/.faceless-shorts" -type f -name talerender -exec sha256sum {} + \
  | grep -v "^$RELEASE_SHA256 "

# Every replaced location reports the new version:
for p in \
  "$HOME/.faceless-shorts/talerender-bin/talerender" \
  "$HOME/.faceless-shorts/talerender" \
  "$HOME/workspace/talerender/talerender"
do
  [ -f "$p" ] && { printf '%s -> ' "$p"; "$p" --version; }
done

# The adapter's explicit selection is live in this shell:
echo "$TALERENDER_BIN"                  # /home/<user>/talerender-bin/talerender
```

Cleanup is complete only when: the canonical digest matches the
release notes, `--version` prints `talerender 1.0.1` at every surviving
location, the hub-wide digest sweep prints nothing, and
`TALERENDER_BIN` points at the canonical path. The `.pre-v1.0.1`
backups may be deleted once a native capture has run successfully
against the new binary; until then they are the rollback.

---

## 7. Pack provenance

- Changelog range re-derived first-hand: `git log 3c2c129..HEAD` at
  `f6616aa` (20 commits); per-commit subjects and the gpucore/film.json
  diff (`aadc193`) checked against the tree.
- Version surfaces re-derived first-hand: `git grep -n '1\.0\.0'`,
  `Cargo.toml`, `src/main.rs:100-101,171`, `.github/workflows/ci.yml`
  (binary-artifact step), `.github/workflows/bench.yml:80`.
- v1.0.0 facts checked against the live release (`gh release view
  v1.0.0`) and its asset digest (GitHub API): binary
  `sha256:1d669857bbda21a9c13d60658a4b4a4593d07bf7c6d4bd7ea88a5b1383771a84`,
  19,552,576 B, target `3c2c129`, published, not draft/prerelease.
- CI status at the snapshot checked live: CI `43622186461` SUCCESS;
  Tools CI `43622186445` FAILURE with the failed step's log read
  (§5). `WAVE2-RECORD.md` completeness assessed from its §7/§8 at the
  snapshot.
- Adapter conventions read from the skill-wiring work copy:
  `template/tools/capture_native.mjs` (`findBinary`, lines 15-18,
  68-85) and `template/tools/CAPTURE-NATIVE.md` (prerequisites +
  provisioning commands).
- Nothing in this pack was committed, tagged, built, or published by
  its author. The release remains the lead/orchestrator's act, gated
  on §5 — including the user's explicit go.
