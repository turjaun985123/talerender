# TaleRender v1.0.1 — patch-release pack, REFRESH (W89)

**Status: PREPARATION ONLY — this pack does not authorise a release.**

Prepared from repo HEAD **`c440d90`** (2026-11-15). This pack refreshes
Batch 5's `RELEASE-101-W46.md` (snapshot `f6616aa`) to the current tree:
it extends W46's changelog by the 28 commits in `f6616aa..c440d90`,
re-verifies the version-bump mechanics and the `1.0.0` literal census,
and updates the release procedure with the gates that exist now but did
not at W46's snapshot (above all the knobs gate). W46's §1 (why v1.0.1
exists: two divergent builds both reporting `talerender 1.0.0`) is
unchanged and is not repeated; W46's PC-cleanup command blocks (§6) are
carried forward in §6 below. The release still needs the **user's
explicit go** and must ship from the **final wave HEAD**, not from this
snapshot — the executor extends the changelog over the remaining delta
and re-proves every gate in §4 on the bump commit's SHA.

## 1. What changed since W46's snapshot

W46's §5 listed four hard blockers. Their state at `c440d90`:

| W46 blocker | State now |
|---|---|
| Tools CI red on the snapshot SHA (absolute-path asset step) | **FIXED** — classification fix `617ea4e`, gitignored-fixture-PNG fix `865c471`, and the later ffmpeg-declaration fix `f4f6ccf`; Tools CI green at `f4f6ccf` and `b2e46d2` (record fold-in 2). Must still be re-proven on the bump SHA. |
| `WAVE2-RECORD.md` incomplete | **Mostly closed** — fold-in 1 (`b6ee9fb`) and fold-in 2 (`cca4e6e`) landed; the close-out fold-in covering Batches 8–10 and the final lead landings is still owed at the freeze. |
| Version still 1.0.0 | **Unchanged** — `Cargo.toml:3` still `version = "1.0.0"` (re-verified, §3). The bump is Step 1 of the procedure, by design. |
| User's explicit go | **Still outstanding.** Nothing in this pack substitutes for it. |

Two W46 caveats also moved: the §15 text is no longer a draft (folded
into `docs/scene-format.md` at `4dd6530`, with the loader reservation
guard), and the golden pins W46 treated as unpinned candidates have
since been **struck** (Acts 1–2 + a disclosed corrective, §2.3). One
W46 caveat has **not** moved: `src/locomotion.rs` and
`src/cameramoves.rs` are still **not registered** in the crate —
`src/main.rs` at `c440d90` declares `mod anim;` (line 19) and
`mod charanim;` (line 23) but no `mod locomotion;` /
`mod cameramoves;`. The published notes must keep describing them as
standalone modules, or the lead registers them before the freeze.

## 2. Changelog extension: `f6616aa` → `c440d90`

Verification commands used for this section:

```bash
git log --reverse --format='%h %s' f6616aa..HEAD   # 28 commits
git rev-list --count f6616aa..HEAD                 # 28
git rev-list --count 3c2c129..HEAD                 # 48 (v1.0.0 -> here)
```

All 28 commits are accounted for below, grouped by what a user/operator
can observe. W46's §2 (the first 20 commits, `3c2c129..f6616aa`) stands
as written and is not repeated; the published changelog is W46 §2 +
this §2.

### 2.1 Features

- **Quality knobs — F4 IMPLEMENTATION COMPLETE.** `--res`, `--aa`,
  `--shadow`, and the combined `--draft` flag, all validated before any
  GPU work (bad values are clean CLI errors, exit 2). Defaults are the
  identity: a no-flag run is byte-identical to the pre-knob binary's
  output (the identity rests on exact arithmetic — round/floor/even-mask
  sizing that returns the base size untouched at `--res 1`, and
  power-of-two shadow arithmetic that is bit-identical at 2048). Any
  off-default knob stamps `[DRAFT … — NOT FOR DELIVERY]` on the run's
  header/progress/completion lines, so a draft render can never be
  mistaken for a master — `ea98458` (phase 1: parse + sizing + draft
  tagging), `9ad2177` (phase 2: `--aa` MSAA plumbing across all four
  scene passes, depth twins included), `76b922c` (phase 3: `--shadow`
  target sizing in film + ab1, shader fed only through the uniform).
  The dispatch-only **knobs-gate workflow** runs the acceptance legs
  (identity / draft smoke / validation) and passes only on the exact
  `KNOBS GATE: PASS` line — `2017e86`.
- **Scene-format §15 folded in + reservation guard.** The prefabs/clips
  extension text is now part of the canonical `docs/scene-format.md`,
  and both loaders (scene-file and film) hard-error on reserved §15
  keys with the spec's named messages instead of silently ignoring
  them — `4dd6530`. The §15 conformance checker is wired into Tools CI
  alongside the unknown-field ratchet and the sheet suite — `8b9ac36`.
- **F2 migration COMPLETE — film data is generic.** The film driver no
  longer hardcodes this film's assets: the two formerly silent
  fallbacks in asset loading now fail loudly, naming the plate/caption
  — `a2ec80c`; the film data itself declares its calendar years, dark
  plate, and sign — `20a81a7`; and the loader now *demands* those
  declarations (no Onoda defaults inside a generic loader), with
  `plates.json` promoted to a manifest whose entries are the plate set
  — `2c41313`. Frame impact: zero, by transcription, probe-gated.
- **Blink adoption.** The film's scheduled blink channel is now the
  character-animation module's smooth blink, composed as
  `max(channel, authored beats)` so the 7 authored word-beat blinks
  survive structurally (25 beat character-frames, 0 changed); 386
  character-frames / 345 film frames change deliberately, as censused
  before landing — `5a9383e`.
- **Structured errors, step 2.** The GPU core's adapter/device/readback
  entry points return `TaleError` end to end (no-adapter and device
  failures exit 3 with their message texts preserved verbatim); the
  panic surface in the five entry files is now only the documented
  invariants — `edb058f`. (The telemetry exit-code agreement itself
  landed earlier, at `aa10b13`, and is already in W46's §2.1.)

### 2.2 Fixes

- **Tools CI incident, root causes fixed.** (i) The asset checker's
  existence test failed the film manifest's absolute machine paths on
  any machine but the author's; absolute-flagged references are now
  classified as portability warnings, while relative missing references
  still fail — `617ea4e`. (ii) The checker's fixture PNGs had been
  silently swallowed by `.gitignore` since Batch 3 (green locally only
  because the files exist on this disk); negations added and 55 fixture
  PNGs committed — `865c471`. (iii) The sheet suite failed on hosted
  runners because the image ships no ffmpeg; the workflow now declares
  the dependency — `f4f6ccf`.
- **Errors step-2 build break fixed.** The signature conversion missed
  the two `Ok(...)` construction wraps in `film.rs`; CI caught it, the
  wraps were restored — `aa92cee`.
- **Golden Act 1 corrective** — see §2.3.
- **ab1 delimiter fix.** The knobs phase-2 attachment replacement left
  a dangling closer in `ab1.rs` (four probe dispatches failed at
  Build); one line removed, no logic change — `c440d90` (this pack's
  snapshot HEAD).

### 2.3 Golden pins — struck, corrected, verified

- **Act 1:** the 14 film first-pins struck (11 mids from the f021991
  candidates, 3 in-window mids from the post-re-chunk render) —
  `916a109`.
- **Act 2:** the ab1-composer pin re-struck from a fresh render at the
  integration head (`d7bd71a0` → `60c32c1a`, 23.98 dB vs Chrome) —
  `166873c`.
- **Corrective (disclosed):** the first Act-1 commit had pinned
  superseded pre-atmosphere-fix bytes for the 11 out-of-window mids —
  a missed copy step in the pin staging. The visual-regression canary
  caught it on its first measured run; the corrective re-pinned exactly
  those 10 lines from the candidate bytes and the pack re-verifies
  18 OK / 0 STALE / 0 MISSING — `2755ff3`. Post-corrective regression
  runs match everything except the s28 item below.
- **s28 is in neither act.** The historical §28 first-light pin was
  deliberately not re-struck (the re-pin packet recommended against
  it); visual regression therefore stays red on s28 alone until the
  user rules on that pin. v1.0.1's notes must state this, not smooth
  it over.

### 2.4 Record corrections (named, not hidden)

A release note that hid these would repeat them; they are part of the
wave's record and are summarised here for the notes' "record" line.

- **Adapter re-proof correction** (fold-in 1, `b6ee9fb`): the record's
  earlier "re-proof PASS" row was corrected — the run behind it had
  been SIGKILLed under contention and its fixture mixed 66-chunk data
  with 73-chunk strips. The corrected fixture was rebuilt and the
  correction later **closed** by an independent 8/8 re-proof
  (fold-in 2, `cca4e6e`).
- **Act-2 provenance cite corrected** (fold-in 2, `cca4e6e`): the
  Act-2 commit message cited its fresh render's CI run at `9f497c7`;
  the run's actual head was `dc0e555`. The record row now carries the
  correction (the commit message itself is immutable).
- **Knobs phase-1 record error** (`4af54c5`): during Batch 7/8 the
  record relayed a knobs phase-1 landing that **had not happened** —
  verification against the code proved the absence airtight (no Knobs
  struct, no flags, help unchanged), the record was corrected to the
  true state (spec + patch packs complete, implementation not
  started), and the real three-phase landing followed from those packs
  (`ea98458`/`9ad2177`/`76b922c`, §2.1). The 540×960 / 22.35 dB figure
  attached to the false claim is void.
- **Truncated spec restored** (`33c5b2a`): the committed knobs spec
  had lost its tail in transit (it ended mid-§4.2 with a literal
  truncation marker); the tail was spliced back from the provenance-
  tagged restoration pack, with 6 items registered as unrecoverable
  rather than reinvented.
- **Doc-fix queue applied** (`c7aa648`): the refreshed doc-fix pack
  was applied wholesale (final-report addendum, results discharge
  note, fog closure note, film + root README corrections, PERF
  discharge notes) — superseding the older pack, of which nothing had
  ever been applied.

### 2.5 Internal record only (no user-visible change)

- Batch central commits: Batch 5 (gap-fix packs) `d3def63`, Batch 6
  (adoption preparation) `3a3a427`, Batch 7 (verification) `b2e46d2`,
  Batch 8 (close-out preparation) `021ca01`.
- Record fold-ins: `b6ee9fb` (§2.4), `cca4e6e` (§2.4).

## 3. Version-bump mechanics — re-verified at `c440d90`

### 3.1 The one functional bump

| File:line | Current | Action |
|---|---|---|
| `Cargo.toml:3` | `version = "1.0.0"` | **Change to `version = "1.0.1"`. Still the only version literal that drives the binary.** |

There is still **no tracked `Cargo.lock`** in this repo, so there is
no lockfile version entry to bump.

### 3.2 Derived — no edit needed, verify after the bump

| File:line | Surface | Why no edit |
|---|---|---|
| `src/main.rs:274` | `println!("talerender {}", env!("CARGO_PKG_VERSION"))` (guard at :273) | `--version` output is compiled from the Cargo version (W46 cited :171; the line moved as `main.rs` grew). Post-bump smoke must print exactly `talerender 1.0.1`. |
| `src/main.rs:115` | `-V, --version` help line | Documents the flag; contains no version literal. |
| `.github/workflows/bench.yml:80` | `./target/release/talerender --version` in the bench log | Invokes the flag (same line as at W46's snapshot); output becomes 1.0.1 automatically. |
| `docs/runbook/USER-PC.md` | `--version` named in the verified-flag guidance (:8, :174 area) | Flag name only; no literal. |

### 3.3 Literal `1.0.0` census — re-run at `c440d90`

`git grep -n '1\.0\.0'` over the tracked tree returns the `Cargo.toml`
bump point (§3.1) plus the 28 lines below (the two release packs,
W46 and this one, are excluded — they are dated preparation documents
whose `1.0.0` mentions are their subject, not version claims).
**Exactly one line requires a release-time decision** (README, first
row); the other 27 are historical or minimum-version references and
must **not** be rewritten — a global search-and-replace would falsify
dated records.

| File:line | Text (abridged) | Disposition |
|---|---|---|
| `README.md:65` | "`--version` for the version (`talerender 1.0.0`)" | **NEW since W46** (the README had no `1.0.0` literal at `f6616aa`; the doc-fix application `c7aa648` added this CLI pointer). Present-tense user documentation of the *current* `--version` output, not a dated record. **Recommend: update to `1.0.1` in the bump commit itself** (one extra line, same commit), so the shipped tree's own README matches the shipped binary. If the lead rules it out of the bump commit, it must be fixed in a same-day doc commit and the discrepancy named in the release notes. |
| `docs/runbook/PC-EVIDENCE-W36.md:90` | `--version    # sanity: 1.0.0 or later` | Minimum-version sanity check; 1.0.1 satisfies it. No edit. |
| `docs/integration/DETERMINISM-CONTRACT-W04.md:241` | `--version   # sanity: 1.0.0 or later` | Same minimum check. No edit. |
| `docs/integration/E7-READINESS-W78.md:167` | `--version sanity "1.0.0 or later"` | Same minimum check (new doc since W46). No edit. |
| `docs/runbook/PERF.md:26` | "`--help` and `--version` (added after v1.0.0)" | Historically true statement. Keep. |
| `docs/runbook/USER-PC.md:13` | "professionalised after v1.0.0; use a binary from the CI run" | Historically true statement. Keep. |
| `docs/integration/WAVE2-RECORD.md:22` | "version 1.0.0; CI green at launch" | Dated launch-HEAD statement. Keep; the release row is appended at close (§4, Step 7). |
| `docs/integration/WAVE2-RECORD.md:229` | "on that mixed fixture the OLD (v1.0.0) binary prints `CHECK OK`…" | Historical, inside the corrected adapter row (§2.4). Keep. |
| `docs/integration/EXIT-CODES-W02.md:124` | "captured with the v1.0.0 binary (old convention)…" | Historical description of the pre-exit-code binary generation. Keep. |
| `docs/integration/GAPFIX-MATRIX-W49.md:5,21,41,52,135,136` | gap analysis "probed against the v1.0.0 release binary" (6 lines) | Historical analysis basis. Keep all six. |
| `docs/integration/BENCH-FIRST-W64.md:36` | "`--version` → `talerender 1.0.0`" (bench run at `aa92cee`) | Dated run evidence. Keep. |
| `docs/integration/DOCFIX-W38.md:111,381,387,422,428,579` | historical changelog prose + quoted paste-ready text (6 lines, same lines as W46 found) | Historical. Keep all six. |
| `docs/integration/DOCFIX2-W67.md:145,319` | the same historical prose carried into the refreshed pack | Historical. Keep both. |
| `docs/integration/FINAL-REPORT.md:1021` | the same sentence as applied to the final report's addendum | Historical changelog prose. Keep. |
| `docs/integration/GATE2-W50.md:202,214` | range notation `v1.0.0..HEAD`; "v1.0.0 proof MP4" | Git-range notation + historical reference. Keep both. |
| `docs/integration/GATE2-W80.md:395` | "the v1.0.0 proof MP4 is NOT re-attached…" | Historical reference inside the Batch-8 gate. Keep. |

Created by the procedure itself (not source files): the git tag /
GitHub release **`v1.0.1`** and the release title **`TaleRender v1.0.1`**.

## 4. Release procedure (mirrors v1.0.0, gates updated)

v1.0.0 method, unchanged: the binary asset is the `talerender-binary`
artifact of a green CI run — CI uploads it from
`target/release/talerender` (`.github/workflows/ci.yml`, "Upload
binary artifact" step) — downloaded locally, verified as a working
binary, its SHA-256 recorded in the notes, then uploaded to a
directly-published (non-draft, non-prerelease) GitHub release.
**No local `cargo build` is a release source** — builds happen in CI
(repo law; this VM has no cargo in PATH).

### Step 0 — Freeze and authorisation gate

Do not start until **every** item in §5 is green/complete, including
the user's explicit go. Then:

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
# Plus, per §3.3's recommendation, the one present-tense README line:
# README.md:65  (`talerender 1.0.0`)  ->  (`talerender 1.0.1`)
git add Cargo.toml README.md
git commit -m "Release: bump version to 1.0.1"
git push origin main
RELEASE_SHA=$(git rev-parse HEAD)  # the bump commit IS the release target
```

### Step 2 — Extend the changelog to the release target

```bash
git log --reverse --format='%h %s' 3c2c129.."$RELEASE_SHA"
git log --reverse --format='%h %s' c440d90.."$RELEASE_SHA"   # the delta this pack has not seen
```

Fold every commit in the second list into the release notes (grouped
as in W46 §2 + this pack's §2), and record the final count (48 at this
snapshot). If the delta includes a revert of the caption re-chunk
(user veto at wave close), the caption entries and the telemetry
expectation in Step 3 change with it — follow the final tree, not
this pack.

### Step 3 — Prove every gate ON the exact release SHA

A cancelled run is not a success; a success on any other SHA is not
evidence for this release. For every run below, confirm its `headSha`
is `$RELEASE_SHA` before reading its conclusion.

**(a) CI and (b) Tools CI — push-triggered, must be SUCCESS:**

```bash
REPO=turjaun985123/talerender
CI_RUN_ID=$(gh run list --repo "$REPO" --commit "$RELEASE_SHA" \
  --workflow ci.yml --limit 1 --json databaseId --jq '.[0].databaseId')
TOOLS_RUN_ID=$(gh run list --repo "$REPO" --commit "$RELEASE_SHA" \
  --workflow tools-ci.yml --limit 1 --json databaseId --jq '.[0].databaseId')

test "$(gh run view "$CI_RUN_ID"    --repo "$REPO" --json conclusion --jq .conclusion)" = "success"
test "$(gh run view "$TOOLS_RUN_ID" --repo "$REPO" --json conclusion --jq .conclusion)" = "success"
```

**(c) Knobs gate — dispatch-only; passes only on the exact PASS line:**

```bash
gh workflow run knobs.yml --repo "$REPO" --ref main   # main == $RELEASE_SHA after Step 1's push
# wait for the newest run of "Knobs gate (dispatch)", verify headSha == $RELEASE_SHA,
# then:
KNOBS_RUN_ID=$(gh run list --repo "$REPO" --workflow knobs.yml --limit 1 \
  --json databaseId --jq '.[0].databaseId')
test "$(gh run view "$KNOBS_RUN_ID" --repo "$REPO" --json conclusion --jq .conclusion)" = "success"
gh run view "$KNOBS_RUN_ID" --repo "$REPO" --log | grep -Fx 'KNOBS GATE: PASS'
```

(The workflow itself fails the job without the exact line; the grep
is the executor's independent read of the evidence.)

**(d) Determinism — dispatch-only; the CI-lavapipe leg must PASS:**

```bash
gh workflow run determinism.yml --repo "$REPO" --ref main
DET_RUN_ID=$(gh run list --repo "$REPO" --workflow determinism.yml --limit 1 \
  --json databaseId --jq '.[0].databaseId')
test "$(gh run view "$DET_RUN_ID" --repo "$REPO" --json conclusion --jq .conclusion)" = "success"
```

The workflow enforces the exact line itself (it errors unless
`determinism-results.txt` contains a line exactly `DETERMINISM: PASS`),
so a success conclusion *is* the PASS evidence. This discharges the
contract's CI leg (E6) on the release SHA; the RADV leg (E7) remains a
PC-session item and is not a release gate (§5).

**(e) Golden verify — 18 OK on the release checkout:**

```bash
# in the clean RELEASE_SHA checkout:
python3 tools/golden/make_golden.py        # verify mode is the default
# expected tail: 18 OK / 0 STALE / 0 MISSING
```

**(f) Telemetry — the headline gate, from the downloaded binary:**
executed as part of Step 4's smoke (exact line
`FILM TELEMETRY: CHECK OK`, exit 0). If that line is not produced, stop
— do not publish a patch release whose headline gate has regressed,
and do not "fix" the expectation in the notes.

### Step 4 — Download the CI binary and verify it BEFORE upload

Identical to W46's Step 4 (artifact download into a fresh
`~/workspace/.release-v101.*` dir, `file`/`stat`, full SHA-256 recorded,
`--version` smoke = exactly `talerender 1.0.1`, bad-flag smoke = clean
`error: …`, exit 2, no panic, and the telemetry smoke of Step 3(f)),
with **one addition** — the knobs are now real flags and the shipped
help must name them:

```bash
grep -q -- '--res'     "$RELEASE_DIR/help.txt"
grep -q -- '--aa'      "$RELEASE_DIR/help.txt"
grep -q -- '--shadow'  "$RELEASE_DIR/help.txt"
grep -q -- '--draft'   "$RELEASE_DIR/help.txt"
```

### Step 5 — Release notes (shape mirrors v1.0.0)

W46's Step 5 skeleton stands; refresh its "What changes" list from
W46 §2 + this pack's §2 (add: quality knobs with draft tagging; §15
folded in with the loader reservation guard; F2 generic film data;
blink adoption; structured errors step 2; golden pins struck with the
disclosed corrective), and its "Known limits" from the **completed**
`WAVE2-RECORD.md` — expected to name at least: RADV proof still owed
(all native numbers are CI lavapipe), s28 pin disposition still open,
skill adoption still the user's pending decision, and the
locomotion/cameramoves modules still unregistered if that stands at
the freeze. Do not copy v1.0.0's limit text forward unchecked.

### Step 6 — Publish (only on the user's explicit go)

Identical to W46's Step 6: existence guard (neither the release nor a
same-named tag may already exist — stop if so), then
`gh release create v1.0.1 "$BIN" --target "$RELEASE_SHA" --title
"TaleRender v1.0.1" --notes-file "$RELEASE_DIR/notes.md"`, published
directly (not draft, not prerelease), mirroring v1.0.0.

### Step 7 — Verify the published release independently

Identical to W46's Step 7: tag resolves to `$RELEASE_SHA`; release is
published, not draft/prerelease; GitHub's recorded asset digest equals
the pre-upload SHA-256; a fresh download re-checks (`sha256sum -c`,
`--version` = `talerender 1.0.1`). Finally, append the release row
(tag, target SHA, CI run id, Tools CI run id, knobs-gate run id,
determinism run id, full binary SHA-256, release URL) to
`WAVE2-RECORD.md` §7/§8 as part of the wave close-out.

## 5. What must be true before release — status at `c440d90`

| # | Requirement | Evidence at `c440d90` | Status |
|---|---|---|---|
| 1 | **User's explicit go** for the v1.0.1 release, given at wave close | No release go is recorded; the decision briefs (caption veto, s28, locomotion) are prepared for the same close conversation | ❌ NOT MET |
| 2 | Final HEAD frozen (all wave batches + fixes landed; nothing mid-flight) | Batch 9 is running; Batch 10 + the fixes-pass remainder + close-out remain | ❌ NOT MET |
| 3 | Version bumped to 1.0.1 | `Cargo.toml:3` still `version = "1.0.0"` | ❌ NOT MET |
| 4 | All six §4 Step-3 gates green **on the bump SHA** | By construction unprovable until Step 1; snapshot-era evidence: CI + Tools CI SUCCESS at `ea98458` and Tools CI at `f4f6ccf`/`b2e46d2`; knobs-gate acceptance dispatches in flight at `c440d90` (W81 verifying independently); golden pack verified 18 OK / 0 STALE / 0 MISSING at the pin acts and correctives; telemetry `CHECK OK`, exit 0 since `5e90d7e` | ⏳ RE-PROVE AT RELEASE |
| 5 | `WAVE2-RECORD.md` complete through the final HEAD | Fold-ins 1–2 landed (`b6ee9fb`, `cca4e6e`); Batches 8–10, the knobs landings, and the release row itself still to fold | ❌ NOT MET |
| 6 | Changelog extended to the final HEAD | W46 §2 (20 commits) + this §2 (28) = 48 at the snapshot; the executor extends over the Step-2 delta | ⚠️ MET FOR SNAPSHOT ONLY |
| 7 | Caption disposition final | Re-chunk landed, gate CHECK OK; the user's wave-close veto is still open (brief ready; a veto stales 3 in-window pins and changes the tree) | ⏳ PENDING CLOSE DECISION |
| 8 | s28 disposition | Deliberately unpinned in both acts (§2.3); a user decision (re-pin / retire / leave red), not a release blocker, but the notes must state the outcome | ⏳ PENDING CLOSE DECISION |
| 9 | Release-notes claims match the final tree | Locomotion/cameramoves registration caveat (§1) unresolved at the snapshot; notes wording must follow the freeze tree | ❌ NOT MET |

**Not prerequisites** (do not invent blockers, but the notes must
state their final status honestly): PC RADV evidence (E7), and skill
adoption of the native backend. Neither was made a condition of this
patch release; v1.0.1 must not claim either as done unless the
completed `WAVE2-RECORD.md` says so.

**Standing prohibitions (carried forward from W46, still in force):**

- **Do NOT re-attach the v1.0.0 proof MP4.** `onoda-native-f021991.mp4`
  predates the 73-chunk caption re-chunk; attaching it to v1.0.1 would
  present a stale-caption film as the new release's proof. v1.0.1 is a
  binary-only patch release.
- **The v1.0.0 release stays published and untouched.** v1.0.1 is a
  new tag on the bump commit, not a re-issue.
- **No local build is a release source.** The asset comes from the
  green CI run on the bump SHA (Step 4), full SHA-256 recorded before
  upload and re-verified after (Step 7).

## 6. PC cleanup — one canonical v1.0.1 binary

W46's §6 is carried forward **unchanged** — the adapter's discovery
order and the provisioning route it documents have not moved since
`f6616aa` — and is incorporated by reference: inventory (§6.2),
canonical install (§6.3), replace-every-copy (§6.4), end-state
verification (§6.5) of `docs/integration/RELEASE-101-W46.md`. The
essentials, restated so this pack can be executed standalone:

- **Canonical path: `~/talerender-bin/talerender`**, selected by
  **`export TALERENDER_BIN=~/talerender-bin/talerender`** (the
  adapter's `findBinary()` honours the env var first; replacing stray
  copies without setting it is not enough, because discovery could
  still pick a project or checkout copy first).
- Install only from the published v1.0.1 release, verified against the
  **full 64-hex SHA-256 from the release notes** — never a prefix —
  and smoke-checked: `--version` prints exactly `talerender 1.0.1`.
- Every other known copy (the hub copies and the repo-checkout
  fallback) is replaced with the **same bytes**; backups take a
  `.pre-v1.0.1` suffix — a different filename, so no discovery rule
  can ever select a backup by accident. Backups are the rollback until
  a native capture has run successfully against the new binary.
- Cleanup is complete only when: the canonical digest matches the
  release notes, every surviving copy reports `talerender 1.0.1`, a
  hub-wide digest sweep finds no divergent copy, and `TALERENDER_BIN`
  points at the canonical path.

## 7. Go / no-go summary (for the user, verbatim)

As of this snapshot the honest answer is **no-go — yet**: nothing in
the code blocks v1.0.1 (the knobs work is finished and gate-wired, the
film-data migration is complete, the golden pins are struck and
verified, and CI and Tools CI are green at recent heads), but the
release is deliberately gated on four things that only exist at the
wave's close — your explicit go, the wave freeze (Batches 9–10 and the
close-out record still to land), the one-line version bump itself, and
all six acceptance gates re-proven on that bump commit's exact SHA,
including the knobs gate's `KNOBS GATE: PASS` line and telemetry's
`FILM TELEMETRY: CHECK OK`. The moment the wave closes and you say go,
this pack is a mechanical procedure: bump, re-prove, publish one
CI-built binary as v1.0.1, and clean the PC down to a single canonical
copy — with the caption veto and the s28 pin decided in the same
conversation so the release notes can state both outcomes truthfully.

## 8. Pack provenance

- Base pack read in full: `docs/integration/RELEASE-101-W46.md`
  (snapshot `f6616aa`); its §2 changelog, §4 verification commands, §5
  blocker analysis, and §6 cleanup are the substrate this refresh
  extends rather than replaces.
- Changelog range re-derived first-hand: `git log f6616aa..HEAD` at
  `c440d90` (28 commits; 48 in `3c2c129..HEAD`), with per-commit
  subjects read in full — including the knobs phase commits, the F2
  phases, both golden acts + corrective, fold-ins 1–2, and the
  `4af54c5` record correction.
- Version surfaces re-derived first-hand: `Cargo.toml:3`;
  `src/main.rs` (`mod` census :19/:23, `--version` guard :273,
  `env!("CARGO_PKG_VERSION")` print :274, help `-V` line :115);
  `.github/workflows/bench.yml:80`; `git grep -n '1\.0\.0'` census
  (§3.3 — 29 tracked occurrences outside the two release packs:
  1 bump, 1 decision (README), 27 keep).
- Gate shapes verified against the workflows themselves:
  `.github/workflows/knobs.yml` ("Knobs gate (dispatch)"; job passes
  only on the exact `KNOBS GATE: PASS` line, per `2017e86`),
  `.github/workflows/determinism.yml` (dispatch; errors unless the
  results file contains the exact line `DETERMINISM: PASS`),
  `tools/golden/make_golden.py` (verify is the default mode).
- Live CI state sampled at preparation: knobs-gate dispatches in
  progress at `c440d90`; CI/Tools CI conclusions for the snapshot HEAD
  itself had not landed at sampling time and are therefore **not**
  claimed anywhere in this pack — §4 requires them on the bump SHA
  regardless.
- Nothing in this pack was committed, tagged, built, or published by
  its author. The release remains the lead/orchestrator's act, gated
  on §5 — including the user's explicit go.
