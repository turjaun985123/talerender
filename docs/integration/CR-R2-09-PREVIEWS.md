# R2-09 — Previews leg: unblocking pass, generation at scale, CI render plan, strict-previews accounting

Seat: Chrome-replacement REMEDIATION wave, Batch 2, R2-09.
Verified starting state (this seat, 2026-11-16): `scripts/library_check.py
--strict-previews` FAILs — **114 rendered / 74 kind-exempt / 53
exempt-with-reason / 238 pending** (479 items). All work is NEW files in
this directory; the shipped skill was never edited, nothing committed,
nothing installed into `library/previews/`.

**Headline, honestly: the unblocking pass unblocks ZERO items outright —
and that is the finding.** The five R-W5 painters (brick/siding/dirt/sand/
facade) really are in the binary (talerender `67fd9ef`,
`src/scenefile.rs::resolve_texture` arms at lines 488–500, verified), but
every one of the 11 `[R-W5-PAINTER]` prop items carries a *second* named
blocker that the single dependency string was masking. What the pass does
is make the tables tell the truth per item, fix the validator that would
have rejected the painters anyway, and stage everything that is
genuinely stageable today (23 scenes, all CPU-validated).

## 1. Unblocking pass (charter §1)

Deliverables: `patched/world_stage.py` + `patched/validate_stage.py`
(staged replacements), `patched/*.patch` (unified diffs vs the shipped
scripts), `patched/make_patch.py` (the asserted exact-string edit script
that builds them), `classification-diff.json` (before/after over all 75
world items), `run/before/` + `run/after/` (full generator runs).

**Status changes: 0. Dependency/reason changes: 20 entries** (12 prop3d +
8 sets). Per-item verdicts for the 11:

| item | painter leg | true remaining blocker (now the recorded dependency) |
|---|---|---|
| prop3d-rails | dirt **RESOLVED** | `TRANSCRIPTION`: run-length builder (len=200, no R-W3 centring rule; PROP-CAM would sit ~130 m out) + 285-instance InstancedMesh ties |
| prop3d-pond | sand **RESOLVED** | `GEOMETRY + TRANSCRIPTION + MATERIAL`: bank/sparkle are RingGeometry (no ring kind in the format), translucent water, shade() colours, icosahedron flower |
| prop3d-fireplace | brick **RESOLVED** | `R-W5-PAINTER-BEYOND-FIVE`: flame-card painter + unplaced PointLight |
| prop3d-barn | siding **RESOLVED** | `GEOMETRY + PAINTER-BEYOND-FIVE`: gambrel ExtrudeGeometry gable + own `barndoor` canvas painter |
| prop3d-house (reason only; primary code was already GEOMETRY) | siding/brick **RESOLVED** | gable ExtrudeGeometry is now recorded as the *sole* blocker |
| prop3d-palm, fern, ferns, cloudDeck, fallenLog, rug, trailMarker | **not resolved** — palmfrond / fern-card / clouddeck / log-end / rug-motif / chevron painters are *beyond* the R-W5 five | unchanged code `R-W5-PAINTER`, reason now explicitly records "NOT resolved by 67fd9ef" |

Sets (8): `mainStreet, cityStreet, farm, desert, beach, mountain,
fieldCamp, pondBank` — dependency *notes* now record the resolved painter
legs (facade/brick/siding/dirt/sand as applicable). Status stays
SHELL-ONLY: every set carries other named blockers (FORMAT/GEOMETRY/
R-W6-TEXT/transcription). No set scene changed geometrically — the
before/after scene diff is exactly the convention banner + these notes.

**Validator defect found and fixed (staged):** the shipped
`scripts/validate_stage.py` `GENERATORS` set never learned the five
painters, so any scene using them *fails CPU validation while the binary
loads it fine*. Proven both directions with a probe scene
(brick+facade textures): shipped validator `0/1 valid`, patched `1/1
valid`. The staged `validate_stage.py` adds exactly
`brick, siding, dirt, sand, facade` with the evidence cited inline.

## 2. Generation at scale (charter §2) — all runs in `run/`

| pipeline | command (skill = `~/workspace/custom-skills/tall-tale-shorts`) | result |
|---|---|---|
| world (patched generator) | `patched/world_stage.py --skill $SKILL --accounting $B307/ACCOUNTING.json --out run/after` | **GENERATED 21** (atmosphere 10/10, prop3d 11/51) · **SHELL-ONLY 14** (sets, labelled, never installable) · **BLOCKED 40** (each named) |
| world validation | `patched/validate_stage.py run/after/world` | **35/35 valid, 0 fail, 0 unverified-f32** |
| posters | `template/tools/poster_stage.py --out run/posters` | **GENERATED 2** (clip-arms_crossed_final, clip-poseToClip) · **RECORD-ONLY 164** · **BLOCKED 77** (T3 14 + T4 53 + HARVEST 10) |
| poster validation | `template/tools/validate_posters.py run/posters` | **scenes 2/2, records 166/166 valid** (2 poster scenes each carry 3 short-decimal unverified-f32 notes — the validator's source-literal class, reported not hidden) |
| text plates (R-W6) | `template/tools/text_plates.py template/tools/text_plate_specs --out run/textplates` | **7 plates for 6 items** (signpost ×2, poster, platformScale, mailSack, train, chickCrate), sha256 in `run/textplates/build-report.json` |
| orchestrator staging | `scripts/generate_previews_native.py --skill $SKILL --out run/orchestrator` | exit 0; staging produced for castsheet kinds + world; env honestly `UNVERIFIED-ENV (no Vulkan ICD found)`; `--render` correctly refuses on this host |

Rulings as implemented were verified in the generators' own constants
(R-W1 default calls table; R-W2 SET-CAM `[0,2.6,10.5]` fovY 44 / PROP-CAM
fovY 42 az 35 el 20 margin 1.25; R-W3 REF ground disc; R-W4 figure OFF
prop/atmosphere ON sets; R-A1 τ=D/2; R-A2 in-place; R-A3 REF-STAGE + base
figure; R-A4 sheets produced in records, not gating; R-A5 canonical
fixtures — poster_stage asserts fixture coverage of all 77 T2-PROBE
items). Per-kind generated+valid / shell-only / blocked counts are the
table above; every blocked item's named reason is in
`run/after/manifest.json`, `run/posters/MANIFEST.json`, and
`accounting.json` (all 479 items, line-by-line).

Two orchestrator traps the lead must know:
- `generate_previews_native.py classify()` is **stale**: it still
  returns `BLOCKED-NO-STAGING` for all 318 world/animated items and its
  `--render` leg renders castsheet kinds only. World/poster scenes have
  no orchestrator render/install path — the CI workflow (§3) renders
  scene files directly. Patching the orchestrator is future lead work.
- Its held classification depends on `--plates-dir`: with the default
  (`<out>/plates`) the 36 held items read BLOCKED-G1; with the skill's
  real plates (`template/tools/prop_plates`, 36 PNGs present) they are
  READY — castsheet-renderable total is **77** (READY 75 + proxy 2).

## 3. CI render plan (charter §3) — `ci/previews-render.yml`

**Repo choice: `turjaun985123/talerender`.** Justification: the binary is
talerender's — that repo already builds it in CI (mold + rust-cache) and
already carries the proven lavapipe setup (`proof.yml`:
mesa-vulkan-drivers + ICD resolution, mirrored verbatim). The skill repo
has no Rust CI, and previews must never be committed anywhere near it.
The workflow checks the skill out read-only (staging source), generates
+ validates scenes in-job, renders with `./talerender --scene-file …
--width 1080 --height 1920` on lavapipe, converts PNG→JPG in-job
(`ffmpeg -q:v 2`), and uploads **JPG-only** artifacts.

- Inputs: `batch` ∈ world | posters | castsheet | all; `skill_ref`.
- Prerequisite: talerender repo secret **`SKILL_READ_TOKEN`** (PAT with
  read access to the private skill repo; the default GITHUB_TOKEN cannot
  cross repos). Absent secret ⇒ checkout fails loudly, by design.
- SHELL-ONLY / RECORD-ONLY / BLOCKED items are never rendered (the
  render loops filter on `status == GENERATED`; the castsheet leg
  excludes `castsheet.jpg` and the two G2 moods from harvest).
- **Artifact discipline:** one artifact per batch per run
  (`previews-<batch>`), retention 7 days, each with
  `harvest-manifest.json` (name/bytes/sha256). Expected sizes (existing
  previews measure 113,150 B mean): world 21 JPG ≈ **2.4 MB**, posters
  2 JPG ≈ **0.25 MB**, castsheet 77 JPG ≈ **8.7 MB**, all ≈ **11.4 MB**
  per run. Full 242-item end-state ≈ 27 MB — no chunking needed; PNGs
  (~2–4 MB each) never leave the runner.

## 4. Strict-previews accounting (charter §4) — `accounting.json`

Reconciliation (all figures computed from the manifests + library state,
not asserted):

- Library: 479 = 114 rendered + 74 kind-exempt + 53 exempt-with-reason +
  **238 pending**. Cross-check: 238 = 281 (B3-07 pending) − 53 (T4, now
  exempt-with-reason) + 10 (pending outside the 318: character ×4, held ×6). ✓
- B3-07 target: 242/318 generatable; blocked 23 T3 + 53 T4. On the
  *pending* basis the blocked floor is **20** (3 T3 items — house,
  platformScale, stars — already hold Chrome-era previews).
- **As-built projection:** CI renders of the 23 staged GENERATED scenes
  fill **12 pending** (atmosphere dusk/fog/golden/storm;
  prop3d bookshelfWall/cubbies/desk/sandbags/shelf/toolbox;
  clip arms_crossed_final/poseToClip) and refresh 11 rendered. Castsheet
  proxy fills 2 more (character adult/kid). **Projected pending: 224.**
- **Gap 224 → 20 = 204, every item named in `accounting.json`:**
  poster RECORD-ONLY pending **160** (§15 evaluation seat) · T2-HARVEST
  **10** (one-time builder evaluation) · world T2 prop ports **24**
  (per-item blockers as corrected in §1, incl. R-W6 plate consumption —
  plates exist for the 6 text props, their geometry transcription does
  not) · SHELL-ONLY sets pending **2** (plus 12 rendered sets awaiting
  the same transcription for native refresh) · non-B3-07 blocked **8**
  (finn_fox/pip_bird G3G4 appendage + 6 held G1+G6 staging definitions).
- **Extra staleness found:** `shot-dutch` sits inside the T3 floor, but
  its blocker (no camera roll) is **resolved** — `roll_deg` is in
  talerender's `scenefile.rs` (Batch-1 commit `093f40d`, R-A7).
  `poster_stage.py` still tier-gates on stale B3-07 tiers. Re-tiering is
  a lead/accounting act (not done here); after re-tier dutch joins the
  RECORD-ONLY queue and the honest floor becomes 19.
- No placeholder JPGs were created; no new exemptions proposed; anything
  not generatable carries its named reason in `accounting.json`.

## 5. Exact commands for the lead

```bash
# (a) Land the staged replacements (review the .patch files first)
cp ~/workspace/chrome-replacement/remediation/batch2/R2-09/patched/world_stage.py \
   ~/workspace/custom-skills/tall-tale-shorts/scripts/world_stage.py
cp ~/workspace/chrome-replacement/remediation/batch2/R2-09/patched/validate_stage.py \
   ~/workspace/custom-skills/tall-tale-shorts/scripts/validate_stage.py

# (b) Install the CI workflow + set its secret (PAT: read on the skill repo)
cp ~/workspace/chrome-replacement/remediation/batch2/R2-09/ci/previews-render.yml \
   ~/workspace/talerender/.github/workflows/previews-render.yml
#   commit + push talerender, then in the talerender repo settings add
#   secret SKILL_READ_TOKEN (or: gh secret set SKILL_READ_TOKEN --repo turjaun985123/talerender)

# (c) Dispatch renders (one batch at a time keeps artifacts small)
gh workflow run previews-render.yml --repo turjaun985123/talerender -f batch=world
gh workflow run previews-render.yml --repo turjaun985123/talerender -f batch=posters
gh workflow run previews-render.yml --repo turjaun985123/talerender -f batch=castsheet
gh run watch --repo turjaun985123/talerender

# (d) Harvest + verify + install LOCALLY (previews never go to GitHub)
RUN=$(gh run list --repo turjaun985123/talerender --workflow previews-render.yml --limit 1 --json databaseId -q '.[0].databaseId')
gh run download "$RUN" --repo turjaun985123/talerender --dir /tmp/previews-harvest
python3 - <<'EOF'
import hashlib, json
from pathlib import Path
for man in Path('/tmp/previews-harvest').rglob('harvest-manifest.json'):
    d = json.loads(man.read_text())
    for f in d['files']:
        p = man.parent / f['file']
        assert p.stat().st_size == f['bytes'] and hashlib.sha256(p.read_bytes()).hexdigest() == f['sha256'], p
    print(man.parent, 'verified', d['count'], 'JPGs')
EOF
# install ONLY the fills (world/poster GENERATED names) + castsheet output,
# eyeballing each JPG first; filenames already follow <kind>-<id>.jpg:
cp /tmp/previews-harvest/previews-world/*.jpg  ~/workspace/custom-skills/tall-tale-shorts/library/previews/
cp /tmp/previews-harvest/previews-posters/*.jpg ~/workspace/custom-skills/tall-tale-shorts/library/previews/
# castsheet batch refreshes pose/mood/look/held + fills character-adult/kid:
cp /tmp/previews-harvest/previews-castsheet/*.jpg ~/workspace/custom-skills/tall-tale-shorts/library/previews/

# (e) Verify the projection (expect pending 238 -> 224)
python3 ~/workspace/custom-skills/tall-tale-shorts/scripts/library_check.py --strict-previews
```

## 6. Honest limits

- No pixel render was attempted or is claimed (no Vulkan ICD on this
  VM; `generate_previews_native.py --render` refuses, correctly). All
  scenes are CPU-validated only — UNVERIFIED-ENV for renders.
- The unblocking pass changes classifications and reasons only; it does
  not transcribe any new builder. The 24 pending T2 prop ports, 14 set
  transcriptions, 160 poster evaluations and 10 harvests remain real
  future seats, sized in `accounting.json`.
- `run/orchestrator/` was produced with the *shipped* world_stage (the
  orchestrator imports it from the skill); after step (a) above, its
  world staging output equals `run/after/`.
