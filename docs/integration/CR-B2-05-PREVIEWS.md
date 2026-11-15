# CR-B2-05 — Native library previews (Chrome-replacement Batch 2, Seat 5)

Contract: `CHROME-REPLACEMENT-CONTRACTS.md` §1 row 13 (Library
previews, REPLACE-NATIVE). Deliverables (new files only; nothing
existing edited, nothing committed):

| File | Role |
|---|---|
| `~/workspace/chrome-replacement/batch2/B2-05/generate_previews_native.py` | The staged pipeline: census → classify → scene dispatch → (PC/CI) render + install |
| `~/workspace/chrome-replacement/batch2/B2-05/pending-items-manifest.json` | Machine-readable manifest: all 479 items with preview state + native class + named reason; the 289 blocked pending items listed by name |
| `~/workspace/chrome-replacement/batch2/B2-05/run/` | This seat's executed dry run: manifest copy + 77 generated castsheet scenes under `staging/{pose,mood,look,held}/` (CPU-side proof the dispatch data exists) |
| this note | Counts, method, honest strict-previews status, runbook |

**Standing rule honoured:** previews are a LOCAL functional catalog
only. They were removed from GitHub (commit `5ea42dd`) and the sync
`.gitignore` blocks them. Nothing here uploads, commits, or proposes
pushing a preview anywhere. `--install` in the pipeline writes only
into the local skill tree.

## 1. Inventory — from the filesystem + `library/index.json`, cross-checked against `library_check.py`

Method: parsed `library/index.json` (479 items), stat'ed every
`library/previews/<kind>-<id>.jpg`, replicated `library_check.py`'s
census semantics in the generator, and ran the checker itself:
**`LIBRARY OK (479 items)` — `PREVIEWS: 114 rendered / 74 exempt /
291 pending`. The generator's independent census agrees exactly.**

| kind | items | rendered | pending | exempt | native class (§2) |
|---|---:|---:|---:|---:|---|
| pose | 17 | 17 | 0 | – | READY 17 |
| mood | 19 | 19 | 0 | – | READY 17, G2 2 |
| look | 5 | 5 | 0 | – | READY 5 |
| held | 42 | 36 | 6 | – | G1 36, G1+G6 6 |
| character | 4 | 0 | 4 | – | READY-BY-PROXY 2, G3/G4 2 |
| set | 14 | 12 | 2 | – | NO-STAGING 14 |
| atmosphere | 10 | 6 | 4 | – | NO-STAGING 10 |
| prop3d | 51 | 15 | 36 | – | NO-STAGING 51 |
| shot | 28 | 0 | 28 | – | NO-STAGING 28 |
| motion | 117 | 4 | 113 | – | NO-STAGING 117 |
| clip | 98 | 0 | 98 | – | NO-STAGING 98 |
| sfx / music / tool / doc / recipe / example | 74 | – | – | 74 | EXEMPT 74 |
| **total** | **479** | **114** | **291** | **74** | |

Visual items (owe a preview): **405**. Pending visual: **291**
(atmosphere 4, character 4, clip 98, held 6, motion 113, prop3d 36,
set 2, shot 28 — full by-name list in the manifest).

Other filesystem facts:

- `library/previews/` holds **115 files**: the 114 item previews +
  `castsheet.jpg` (the contact sheet — an orphan by the index's
  naming convention, expected, not an item).
- **Zero** zero-byte previews.
- **Stale — heuristic, labelled as such.** All 114 previews carry
  one batch timestamp (2026-11-12 15:29 UTC). 59 of them predate
  their item's current source file (mtime proxy): all 36 held + 19
  mood (source `core/toon.ts`, touched 2026-11-13) and 4 motion
  (`core/motion.ts`). An mtime is suspicion, not proof of visual
  drift — the manifest records it per item as
  `staleByMtimeHeuristic`, never as a verdict.
- **Shared frames.** 29 of the 114 rendered files share a byte-hash
  with another item's preview (12 groups), e.g.
  `set-meadow.jpg` ≡ `prop3d-house.jpg`,
  `atmosphere-noon.jpg` ≡ `set-mainStreet.jpg`,
  `look-adult.jpg` ≡ `mood-neutral.jpg`,
  `motion-moodCue.jpg` ≡ `motion-reactCue.jpg`. These are
  scene-shared Chrome-era frames (a prop shown inside its set; an
  atmosphere shown on its set), not per-item isolated renders.
  Recorded per item as `sharedFrame` in the manifest. Whether that
  is acceptable catalog practice is a lead/user call; it is NOT
  something the native pipeline silently reproduces.

## 2. Native classification — what the Batch 1 castsheet machinery can and cannot stage

The pipeline imports its tables (POSES / MOODS / LOOK_PRESETS /
PROP_IDS) from the skill's own `template/tools/castsheet_native.py`
— no re-transcription, so it cannot drift from Batch 1 staging.
Classes (CR-B1-03 gap names reused):

- **READY — 39.** pose 17 + mood 17 + look 5. The castsheet catalog
  stages them today; B1 proved these kinds on lavapipe (look 5/5;
  pose/mood share the identical scene path).
- **READY-BY-PROXY — 2.** `character-adult`, `character-kid`: their
  CharacterDefs in `core/characters.ts` are exactly LOOK_PRESETS
  adult (`{}`, 1.75 m) / kid (`{kid:true}`, 1.35 m). The pipeline
  stages them via the look scene and writes the
  `character-<id>.jpg` name; the mapping is recorded per item.
  These are the **only 2 of the 291 pending items generatable
  today**.
- **BLOCKED-G1 — 36 held.** Native painter takes held props only as
  pre-rendered RGBA plates (scene-format §14.3); **no plate PNG
  exists anywhere in the skill** (the only plate on disk is B1's
  synthetic `book.png` fixture in the evidence dir). All 36 are
  plate-missing — they already have Chrome-era previews, but none
  can be *refreshed* natively. Waits on B2-10's plate generator.
- **BLOCKED-G1+G6 — 6 held.** nest, bandage, dropper, splint,
  berrySprig (props2.ts), butterfly (butterfly.ts): not in the
  castsheet PROPS enumeration at all — never staged by Chrome's
  castsheet either (CR-B1-03 G6) — and plateless. These are 6 of
  the pending 291. Waits on B2-10 + a staging definition.
- **BLOCKED-G2 — 2 mood.** puzzled, deadpan: no native Mood variant
  (17 of 19 in `toonfull.rs`); a native frame would be a neutral
  face under the wrong label. **EXCLUDED from generation until the
  B2-09 port lands** — the generator classifies them, the render
  leg never installs them, even though a whole-kind mood run
  produces files for them. Their Chrome-era previews stay as-is.
- **BLOCKED-G3G4 — 2 character.** finn_fox (ears:fox + tail),
  pip_bird (wing): the defining look fields are dropped by the
  native scene loader — `FullLook` has no appendage fields
  (CR-B1-03 G3/G4). A native frame would be a plain stickman under
  a fox/bird name: faking, so blocked by name.
- **BLOCKED-NO-STAGING — 318** (281 of them pending). set 14,
  atmosphere 10, prop3d 51: the castsheet machinery stages
  *characters only*; no native world-scene preview staging exists.
  shot 28, motion 117, clip 98: camera moves, animation clips and
  evaluator functions — a single still is undefined without a
  staging convention, and none exists (checker's own census:
  previewSheet 0/166 animated-timing items). This seat names them;
  inventing frames for them would be fabrication.
- **EXEMPT — 74.** Non-visual kinds; no preview owed, ever.

## 3. Honest strict-previews status

- **Native-renderable today: 41 of 479** (39 READY + 2
  READY-BY-PROXY) — i.e. 41 of 405 visual items.
- **Waiting on G1: 42** (36 plate-blocked + 6 plate-and-staging).
- **Waiting on G2 (B2-09): 2.** **Waiting on G3/G4: 2.**
- **Waiting on a staging convention that does not exist yet: 318.**
- **Exempt by design: 74.**
- Of the **291 pending**, exactly **2** (character-adult,
  character-kid) can be generated by the pipeline's PC/CI leg
  today; **289 are blocked, every one named with its reason in
  the manifest**.
- `library_check.py --strict-previews` therefore **still fails**
  (291 pending) and will keep failing until G1 lands *and* a
  staging convention for world/shot/motion/clip items is built
  and ruled on. The generator's own `--strict` mirrors this:
  verified exit 1 on this tree. Nobody should report
  strict-previews as green.

## 4. What was actually run here (CPU-side only)

This VM has **no Vulkan ICD** (`/usr/share/vulkan/icd.d` absent, no
`/dev/dri`) — per the wave state, the replacement VM lost lavapipe.
All render output from this host would be **UNVERIFIED-ENV**, so no
render was attempted. Executed and verified:

1. `library_check.py --verbose` → LIBRARY OK, census as §1.
2. `generate_previews_native.py` (dry run) → exit 0; census +
   classification as above; Batch 1 generator dispatched per kind
   producing **77 scene files** (pose 17, mood 19, look 5, held 36
   — held scenes carry `plateMissing`, exactly as B1 designed).
3. `--strict` → exit 1, naming 289 blocked pending items.
4. `--render` → **exit 3, refused**: "no Vulkan ICD on this host
   (UNVERIFIED-ENV)" — fail-closed, mirroring the Batch 1 adapter
   smokes on this VM.

## 5. Runbook — PC / CI leg (where renders can run)

Preconditions: talerender binary discoverable (`TALERENDER_BIN` or
repo checkout — the mode's own discovery), a Vulkan ICD (user's PC:
RADV/Vega 7; CI: lavapipe, labelled software), ffmpeg, python3,
node. After B2-09 lands, re-run: puzzled/deadpan move G2 → READY
automatically (the classifier keys off the imported MOODS table's
gap list — update `G2_MOODS` in the generator when B2-09's port is
verified). After B2-10 lands, point `--plates-dir` at its output:
held items with a plate flip G1 → READY automatically.

```bash
SKILL=~/workspace/custom-skills/tall-tale-shorts
PIPE=~/workspace/chrome-replacement/batch2/B2-05/generate_previews_native.py

# 1. census + manifest + dry-run dispatch (any host)
python3 $PIPE --skill $SKILL --out ~/previews-run
#    expect: "479 items | states {'rendered': 114, 'pending': 291,
#             'exempt': 74}" and a manifest whose pendingByClass is
#             {BLOCKED-NO-STAGING: 281, READY-BY-PROXY: 2,
#              BLOCKED-G3G4: 2, BLOCKED-G1+G6: 6}

# 2. render the READY classes (PC/CI only; refuses without an ICD)
python3 $PIPE --skill $SKILL --out ~/previews-run --render
#    expect under ~/previews-run/renders/: pose-*.jpg x17,
#    mood-*.jpg x17 installable (19 produced; puzzled/deadpan are
#    generated by the kind run but EXCLUDED — see step 3),
#    look-*.jpg x5, castsheet.jpg; held renders nothing and the
#    mode names all 36 plate-skips.

# 3. install into the LOCAL catalog (never anywhere else)
python3 $PIPE --skill $SKILL --out ~/previews-run --render --install
#    copies ONLY READY items: 17 pose + 17 mood + 5 look +
#    character-adult.jpg / character-kid.jpg (from the look
#    renders) = 41 files. mood-puzzled.jpg / mood-deadpan.jpg are
#    never installed (G2 exclusion is in the code, with a log line).

# 4. verify
python3 $SKILL/scripts/library_check.py --verbose
#    expect: LIBRARY OK; PREVIEWS rendered rises 114 -> 116
#    (the 2 character previews are new; the other 39 refresh
#    existing files). --strict-previews STILL FAILS (289 pending)
#    — that is the honest state until G1 + staging land.
```

Expected end state of a successful PC/CI run today: **116 rendered
/ 74 exempt / 289 pending**, 41 previews natively produced, every
remaining pending item still named in the regenerated manifest.
Any run claiming more than that on today's machinery has faked a
frame somewhere — most likely a prop-less held figure, a neutral
"puzzled", or an appendage-less Finn/Pip.
