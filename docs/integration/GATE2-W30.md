# GATE2-W30 — Wave 2 Batch-3 gate + integration packet

**Seat:** TaleRender Wave 2, W30 (batch-3 gate). **Date:** 2026-11-14.
**Gate window:** 2026-11-14 ~17:03Z (first poll; tree still at
Batch-2 close) → 17:17:34Z (final snapshot).
**Tree state at gate close:** HEAD `11a7a2e`; working tree = 18
untracked Batch-3 paths (§1) + `__pycache__` artefacts (§6.6) +
**one modified tracked file, `src/dof.rs`** — the lead's
uncommitted gpucore migration, in flight as this gate closed
(§6.5). HEAD *moved five times during this gate* — the lead
landed its Batch-1/2 integration in parallel: `2328e6f` (module
registration), `38930fa` (gpucore test fix), `86f0c81`
(PaintBudget + RunLog wiring), `fbf37c0` (render.yml
paint-budget input), `11a7a2e` (lead record: verdicts +
Batch-2 adoption decisions). No lead commit touched a Batch-3
file. Batch-3 seats committed nothing, per the wave model.

**Method.** Deliverables judged on disk only. Every check below
was re-run by this gate against the seats' own fixtures; a
seat's pasted output is quoted only where this gate's run
reproduced it.

---

## 1. Inventory — W21–W29 deliverables

| Seat | Deliverable(s) | Present | Size | Sanity (one line) |
|---|---|---|---|---|
| W21 | `docs/integration/FILM-SPLIT-W21.md` | ✅ | 32,387 B | film.rs blueprint: 16-cluster region map, 10-module target, DAG + 82-name crossing census, 5-item visibility delta, 10-step gated extraction; §6 postscript already anticipates the lead's paint-budget wiring |
| W22 | `docs/integration/SCENE-SPLIT-W22.md` | ✅ | 23,348 B | scene.rs blueprint: clusters A–M, directory-module target preserving every `crate::scene::` path, 13 leaf-first steps, staging-`painters/` decision flagged |
| W23 | `tools/lipsync/build_visemes.py` + `tools/lipsync/fixtures/` (3 files) + `LIPSYNC-W23.md` | ✅ | tool 33,345 B; note 12,668 B | words.json → W15-shaped `talk` clip; dict/number/G2P sources counted separately; no new channel names |
| W24 | `src/locomotion.rs` + `LOCOMOTION-W24.md` | ✅ | 29,849 B / 693 lines; note 13,575 B | Pure gait evaluators, laws L1–L4 as tests; only `use` is `std::f64::consts::PI` (verified) |
| W25 | `src/cameramoves.rs` + `CAMERA-W25.md` | ✅ | 28,392 B / 763 lines; note 7,485 B | Key-set generators in the §14.4 shape; no wgpu (verified, 0 refs); local `Ease` mirrors `anim::Easing`, swap documented |
| W26 | `tools/format/check_lights.py` + `scenes/fixtures/lights/` (9 fixtures) + `LIGHTS-W26.md` | ✅ | validator 25,472 B; note 8,863 B | 6 checks citing R1/R3/R5 + §14.2; two brief-vs-source encoding decisions documented in the note |
| W27 | `tools/film/rerender_plan.py` + `tools/film/fixtures/rerender/` (mini film ×3 + `run_tests.py`) + `RERENDER-W27.md` | ✅ | planner 34,564 B; note 9,975 B | Data-diff → minimal re-render ranges; never deletes/renders/writes; inheritance expansion for dressing causes only |
| W28 | `tools/format/check_assets.py` + `scenes/fixtures/assets/` (53 files, 5 families) + `ASSETS-W28.md` | ✅ | validator 24,116 B (executable); note 22,109 B | Reference census from loader source, incl. the four silent-fallback behaviours made loud as check 5 |
| W29 | `tools/ci/check_shaders.py` | ⚠️ **tool only** | 69,486 B at gate close (62,064 B mid-gate — the seat was still editing it during this gate) | Static WGSL↔Rust structural checker; **its note `docs/integration/SHADERS-W29.md` is MISSING at gate close** — the tool's own docstring references it twice (§3.9, §6.3) |

No seat's *code* is missing. One seat's *note* is: W29.

## 2. Mechanical checks — this gate's own runs

Toolchain: `rustc 1.99.1` (`~/.cargo/bin/rustc`), `--edition
2021`, binaries in `/tmp/gate2-w30/`.

### 2.1 New Rust modules — standalone `rustc --test`

Both modules are genuinely self-contained (locomotion's only
`use` is std PI; cameramoves has no `use crate::` outside a doc
comment; neither references wgpu):

| Module | Claimed | This gate's run |
|---|---|---|
| `locomotion.rs` | 9 tests | **9 passed, 0 failed** |
| `cameramoves.rs` | 11 tests | **11 passed, 0 failed** |

**Total: 20/20**, matching both seats' claims exactly.

### 2.2 W23 lipsync — fixtures, validator cross-run, determinism

```
$ python3 tools/lipsync/build_visemes.py --self-check
SELF-CHECK: PASS (dict x6, g2p x4, 52 keys, output == expected fixture)   rc=0

$ python3 tools/format/check_clips.py \
    tools/lipsync/fixtures/expected-lipsync.scene.json \
    tools/lipsync/fixtures/onoda-lipsync.demo.scene.json
BOTH files: [1..5] PASS — OVERALL: PASS (5/5 checks passed)   rc=0
```

The compatibility claim is therefore **proven, not asserted**:
W15's validator accepts W23's output shape on both artefacts.
Determinism, first-hand: regenerating the fixture from
`words.json` is byte-identical to the frozen expected file
(sha256 `78892e3c…` both ways); regenerating the Onoda demo
from the project's real `words.json` is byte-identical to the
saved demo (sha256 `9f4b8d88…` both ways — the seat's claimed
hash). Demo census reproduced: **551 keys, 564 segments**.

### 2.3 W26 lights validator — fixtures, tier-B synthetics, real film

All nine fixtures, run individually by this gate — exit code
and failing check number both match LIGHTS-W26's table:

| Fixture | This gate | Note claims |
|---|---|---|
| fx-valid | PASS 6/6, rc 0 | same |
| fx-grade-unknown-key | FAIL check 2, rc 1 | same |
| fx-grade-time | FAIL check 3, rc 1 | same |
| fx-atmosphere-inline | FAIL check 4, rc 1 | same |
| fx-atmosphere-filmlook | FAIL check 4, rc 1 | same |
| fx-light-negative | FAIL check 5, rc 1 | same |
| fx-light-nan | FAIL check 1, rc 1 | same |
| fx-light-colour-range | FAIL check 5, rc 1 | same |
| fx-fire-basis-warn | PASS 6/6, 1 warning, rc 0 | same |

Tier-B synthetics (this gate's own construction: fx-valid
copies with the `fireLevel` marker deleted): intensity 5.0 at
the declared pit → **1 tier-B warning, rc 0**; the S06 evaluated
value 0.6333622932434082 → **no warning, rc 0**. Both as claimed.

Real-film run, re-run by this gate:
`python3 tools/format/check_lights.py --film scenes/film` →
all six checks PASS, `OVERALL: PASS (6/6 checks passed)`,
rc 0 — **byte-identical to the note's verbatim block**.
Negative control: nonexistent file → one-line stderr ERROR,
rc 2.

### 2.4 W28 assets validator — fixtures, CWD cases, real film

Every fixture behaviour in ASSETS-W28 §3 reproduced,
including the exact diagnostic strings:

| Run | This gate |
|---|---|
| valid-scene | PASS 5/5 (check 6 SKIPPED), rc 0 |
| valid-driver | PASS 6/6, the one deliberate orphan `stray.png` reported, rc 0 |
| missing scene | FAIL check 2 — `nope.png` + `gone.json` named, rc 1 |
| missing driver | FAIL check 2 — `cap_001.png` + `sign.png` named, rc 1 |
| wrong-kind scene | FAIL check 3 — PNG magic mismatch with the note's exact first-bytes `7b 22 61 63 74 75 61 6c`, rc 1 |
| wrong-kind driver | FAIL checks 3 + 5 (non-object `plates.json` guard fires with the film.rs:2050 citation), rc 1 |
| input errors ×2 | one-line `ERROR:` each, rc 2 |

CWD cases: from `/tmp` without `--root` → FAIL check 2 with
both references resolved against `/tmp` and both WARN lines,
rc 1; with `--root` at the fixture root → PASS 5/5, rc 0.

**Real-film run, re-run by this gate** (`--root . scenes/film
film`): manifest mode **26 references, 17 warnings, PASS 5/5**;
driver mode **86 references, 0 warnings, 0 orphans, PASS 6/6**;
combined rc 0 — every number identical to ASSETS-W28 §4.

### 2.5 W27 planner — synthetic suite + both real reconstructions

```
$ python3 tools/film/fixtures/rerender/run_tests.py
PASS identical / caption text / caption split / renumber only /
     scene lights / timing shift ×2 / inheritance ×2 /
     stale tail ×2 / driver captions / driver anchors
PASS REAL scenes re-chunk 3a36f1f: exactly the 7 windows
PASS REAL scenes re-chunk: 10 sampled stills affected
PASS REAL driver re-chunk 5e90d7e: exactly the 7 windows
ALL PASS   rc=0   (17/17 checks)
```

**Agreement against the record (RECHUNK-W01.md), checked:**
the planner's windows — `[201,256) [649,679) [947,990)
[1114,1154) [1321,1362) [1392,1434) [1563,1608)`, 296 of
1,811 frames — are exactly the seven split windows implied by
RECHUNK-W01 §2's piece timings (6.69–8.53 s → frames
201–256, etc.), and the 10 affected A/B stills are exactly
RECHUNK-W01 §6's list (t = 7.50, 8.50, 22.50, 32.50, 37.50,
44.50, 46.50, 47.50, 52.50, 53.50). Both data sides (scenes
`3a36f1f`, driver `5e90d7e`) produce the identical plan.
**Full agreement; no disagreement with the record was found.**

### 2.6 W29 shader checker — over the repo, twice

This gate ran the checker at two moments, against two
different versions of the tool (the seat edited it mid-gate):

| Run | Tool size | Census |
|---|---|---|
| ~17:13Z, HEAD `86f0c81` | 62,064 B | **19/21 PASS, 2 FAIL, 0 UNRESOLVED, rc 1** — `shader_basic.wgsl` visibility FAIL at ab1.rs + film.rs `tex_bgl` sites; `shader_pbr.wgsl` uniform-pair FAIL (frame_bytes assembly 128 B / 352 B vs WGSL 448 B) |
| 17:16Z, HEAD `11a7a2e` (final form) | 69,486 B | **21/21 PASS, 0 FAIL, 0 UNRESOLVED, rc 0** |

Control: the intermediate version run against a `git archive`
snapshot of the batch-launch head `1a62161` printed the same
19/21 census — so the two intermediate FAILs were **not**
caused by the lead's in-window wiring; the seat's final
version resolves both (extractor-side, by the diff in
behaviour; without the note this gate cannot say which side
each fix landed on). The final census is the one the packet
carries (§5.3). **Census-vs-note comparison: impossible —
the note is missing (§6.3).**

## 3. Blueprint review (W21 / W22)

Checked against `src/film.rs` / `src/scene.rs` at **HEAD
`11a7a2e`** (film.rs is 3,723 lines there; W21 read 3,694 —
the +29 is the lead's wiring, below).

### 3.1 W21 film.rs — spot-checks (6 items)

| W21 claim | At HEAD `11a7a2e` | Verdict |
|---|---|---|
| `load_film_data` 63–104 (C1) | fn at **63** | exact |
| `walk_phase` 169 / `walk_bob` 172 (C2) | **169 / 172** | exact |
| `s01` 1395 (C9) | **1395** | exact |
| `eval_frame` 1986–2030 (C9) | **1986** | exact |
| `state_key` 2132–2160, `resolve_chars` 2162 (C11) | **2152 / 2182** (+20) | shifted — lead's PaintBudget wiring (`86f0c81`) inserted the `budget` field + `from_env` construction inside C11, exactly the drift W21's §6 postscript predicted |
| `Renderer::new` 2353, `render` 2823, `run_film` 3454, `run_film_telemetry` 3570 (C13/C15/C16) | **2374 / 2843 / 3474 / 3598** (+21/+20/+20/+28) | shifted — same wiring + RunLog lines in the driver |

Every named item exists, in the claimed cluster, in the
claimed order; the map's own instruction ("re-grep the item
names — the cluster structure, not the line numbers, is the
contract") holds. **DAG: acyclic as drawn.** Edges driver →
{data, staging, chars, render, overlay, paint}, staging →
{motion, sets, data, parent}, chars → {data, parent, sets
helpers}, render → {parent, chars, paint}, overlay →
{chars, parent}, telemetry → {data, staging, chars, paint},
motion a leaf, sets → parent only. The two cycle risks the
blueprint itself names check out in source: `chars` names no
renderer type, and `sets` never calls staging (`JgDyn.extra_std`
is data-in). The §3 census's framing (≈82 crossings, 77
already `pub(crate)`, true delta = the 5 widenings of §4) is
consistent with the greps this gate ran.

**One stale gate text (flagged, §6.1):** W21's G-TEL expects
the verdict line `FILM TELEMETRY: CHECK FAILED (… 7 caption
failures)` with exit 1. That was already superseded *before
this batch* by the lead's `5e90d7e` (CHECK OK, exit 0, at the
73-chunk data). The blueprint's gates must be re-pointed at
the current line before any split step executes.

### 3.2 W22 scene.rs — spot-checks (6 items)

scene.rs is untouched by the lead in this window: **2,979
lines at HEAD, exactly the length W22 read.**

| W22 claim | At HEAD `11a7a2e` | Verdict |
|---|---|---|
| `hex_rgb` 30 (A) | **30** | exact |
| `Geo` 104 / `box_geo` 162 (B) | **104 / 162** | exact |
| `Painter` 475 (C) | **475** | exact |
| `paint_toon` 1016–1322 (D) | **1016** | exact |
| `parse_json` 1575 / `JsonParser` 1586 (G) | **1575 / 1586** | exact |
| `paint_planks` 1923 (H) / `paint_sign` 2873 / `sign_tests` 2903 (M) | **1923 / 2873 / 2903** | exact |

**DAG: acyclic as drawn.** Hard edges raster → colour,
toon → {raster, colour}, textures → {raster, colour},
painters/* → {raster, colour}, geo → math (draw records),
json a leaf. The one edge drawn as *potential* (colour → geo)
is honestly labelled in the note and its removal changes
nothing. The consumer census (§3.1) matches the importer set
this gate knows from the crate (scenefile / ab1 / film /
toonfull / battery / telemetry / dof).

## 4. Per-seat verdicts

| Seat | Verdict | Basis |
|---|---|---|
| W21 | **ACCEPT-WITH-NOTES** | Map verified (§3.1); DAG acyclic; the §6 postscript correctly predicted its own staleness. Notes: step 5's premise is consumed (§6.1) and G-TEL's expected line is stale (§6.1) — both are lead-side updates before execution, not defects in the map |
| W22 | **ACCEPT** | Every spot-check exact at the current head; DAG acyclic; the staging-`painters/` duplication is named with a forced decision at step 13 |
| W23 | **ACCEPT** | Self-check verbatim; W15-validator compatibility proven by this gate's own cross-run (5/5 on both artefacts); both determinism hashes reproduced bit-for-bit |
| W24 | **ACCEPT** | 9/9 tests reproduced; the two pixel-changing adoption decisions (bob phase, arm swing) are flagged for A/B + user sign-off, not taken |
| W25 | **ACCEPT** | 11/11 tests reproduced; honesty note is accurate — presets author data the renderers already evaluate; adoption needs a caller, not renderer work |
| W26 | **ACCEPT** | 9/9 fixtures + both tier-B synthetics + real-film run all reproduced, exit codes included; the one tool-vs-corpus disagreement (s06 `atmospherePreset`) was investigated and resolved as a tool bug with the reasoning recorded |
| W27 | **ACCEPT** | 17/17 reproduced including both real-history reconstructions; exact agreement with RECHUNK-W01 on windows, frame count and the 10 stills |
| W28 | **ACCEPT** | All fixture families, both CWD cases and the real-film run reproduced number-for-number (26 + 86 references, 17 warnings, 0 orphans); findings F3/F4 are genuine CLI-surface observations for the lead |
| W29 | **ACCEPT-WITH-NOTES (deliverable incomplete)** | The tool in its final form is green over the whole repo (21/21, §2.6) and its docstring is a real spec of its checks and limits. But the seat's note — which carries the limits list and the packed-array field-order audit the docstring defers to it — **did not land in the gate window**. The gate cannot verify any census claim of the seat's, because there is none on disk |

**Batch-3 gate overall: PASS-WITH-NOTES** — all nine seats'
code landed; every runnable check this gate could execute is
green: 20/20 module tests, 9/9 + 2/2 lights fixture classes,
all W28 fixture families + CWD cases, 17/17 planner checks,
5/5 clips-validator cross-runs, 21/21 shaders at the tool's
final form. The open threads are one missing note (W29) and
integration steps with named owners (§5), not defects.

## 5. Integration packet for the lead (landing order)

*Merged queue: Batch-3 steps interleaved with the still-open
Batch-1/2 items as this gate understands them from
GATE2-W10 §5, GATE2-W20 §5, WAVE2-RECORD and the lead's own
commits through `11a7a2e`. Conflicts are flagged, not
resolved (§6).*

**Already landed by the lead during Batches 2–3 (context, not
queue):** exit codes (`aa10b13`), errors step 1 (`ba88596`),
caption driver leg + CHECK OK (`5e90d7e`), module
registration incl. Batch-2's five (`2328e6f`), PaintBudget +
RunLog wiring (`86f0c81`), determinism CI leg (E6 run
SUCCESS, per `11a7a2e`). Batch-2 adoption decisions (colour/
step lane, ease bridge, W12 split ratified, atlas probation,
wrap/duration) are recorded in `11a7a2e`.

1. **Central batch-3 commit** (orchestrator): all §1 files.
   **Exclude `__pycache__`** (tools/format, tools/ci — seats'
   and this gate's runs) **and exclude `src/dof.rs`** (the
   lead's uncommitted gpucore migration, §6.5 — it is not
   batch-3 work and must land under the lead's own message).
   **Open call for the orchestrator:** W29's note is missing
   (§6.3) — commit the tool without it, or hold the batch for
   the note; this gate recommends committing the tool (it is
   green and self-documenting) and chasing the note as a
   docs-only follow-up. CI expectations: `ci.yml` green,
   test count unchanged (neither new module is `mod`-declared
   yet); `tools-ci.yml` green (none of the three new
   validators is wired into it yet — steps 3–4).
2. **Register locomotion + cameramoves** (lead; `main.rs`,
   one line each): cargo test picks up **20 new tests**. With
   the cameramoves registration, take W25's documented swap
   (`pub use crate::anim::Easing as Ease;`, deleting the
   local enum) since `anim` is already registered.
3. **W29 checker → tools-ci** (lead): a step running
   `python3 tools/ci/check_shaders.py` (exit 0 at the final
   form, §2.6). Land it *with* the W29 note when the note
   arrives — the note carries the limits list and the
   field-order audit the tool defers to it; CI-ing the tool
   without its limits on record invites over-trust.
4. **W26 + W28 validators → tools-ci** (lead): W26's note
   supplies the exact YAML (film run + fixture pass-pair +
   negative-control loop). W28 takes the analogous shape:
   `--root . scenes/film film` as the pass run, the four
   fixture families as pass/fail pairs. Neither tool needs
   `--no-source`-style caveats.
5. **Format fold-ins, continuing the Batch-2 queue** (lead;
   `docs/scene-format.md`): prefabs (§15) and clips with the
   decisions `11a7a2e` records — *plus* the two Batch-3
   authoring surfaces that need the same treatment before
   any data uses them: W24's `gait` driver block (ranges:
   duty ∈ (0,1), stepLength > 0, runFactor ∈ [0,1] — a
   W15-style validator extension is future work, not built)
   and W23's binding convention (a character node instancing
   a `talk` clip *is* the lipsync adoption signal; no new
   vocabulary). W14's fail-loud reservation rule still binds
   prefab data.
6. **W23 lipsync adoption** (lead; film.rs staging): where a
   character instances a lipsync clip, evaluate its `talk`
   track instead of the `talk(t, …)` oscillator (call sites
   per LIPSYNC-W23 §6). Oscillator shots stay byte-identical.
   Speaker attribution remains the recorded workaround
   (single narrator target) until staging supplies a
   line→character map.
7. **W24 locomotion adoption — gated, per-site** (lead):
   shared-driver sites first (s02's three marchers). The two
   flagged pixel changes (bob phase quarter-step shift; arm
   swing as new motion) go through A/B + user sign-off like
   any deliberate re-baseline; arms carrying story beats or
   props (W24's table) are never overwritten.
8. **W25 cameramoves adoption** (staging tooling, not the
   renderer): a caller that writes `keys_to_json(...)` into
   scene files + a handheld seed convention. No renderer
   change, no re-baseline.
9. **W21 film split, steps 1–4 and 6–10** (lead) — with two
   amendments forced by §6.1: **step 5 is already done**
   (delete it; the wiring it specified landed as `86f0c81`),
   and **G-TEL's expected verdict is now `FILM TELEMETRY:
   CHECK OK`, exit 0**. Re-derive C11-onward line numbers
   from item names per §3.1 (+20/+28 shifts measured).
   Per-step gates otherwise as the blueprint names them
   (G-CI, G-PROBE 195–204 / 795–814 / 1195–1204, G-VAL,
   G-AB).
10. **W22 scene split, steps 1–13** (lead): line numbers
    are exact at the current head (§3.2). Step 13 must land
    the staging-`painters/` decision (delete or mark
    superseded) in the same series. Sequencing vs step 9:
    the two splits touch disjoint files (film split never
    edits scene.rs; scene split preserves every
    `crate::scene::` path film.rs consumes) — they commute,
    but both demand serial CI-green commits, so the lead
    picks one series at a time (§6.4).
11. **Still-open Batch-1/2 queue, unchanged owners:**
    errors migration steps 2+ (remaining `expect()` sites —
    W21's trap T7 names film.rs's two); gpucore entry-point
    migrations (in flight — dof.rs is the current one,
    §6.5); W11 anim adoption per-site (camera keys first,
    gated); W12 charanim adoption (blink paint-cost decision
    first); W13 rendergraph steps 2–3; W18 atlas wiring
    under its probation policy + §7 proof list; W19 buffer
    pool under gpucore's `Readback`; W16 first dispatch
    (expect FAIL on the two stale pins — nobody widens the
    band); W17 first bench dispatch (harness verification
    only); W03 golden pins (human, two separate acts, after
    any re-render settles); bridge repackage + film
    re-render with the A/B footnote for the 10 caption-window
    pairs (RECHUNK-W01 §6 items 3–4); W27's planner becomes
    the standing first step of any future data-change
    re-render, with its §1 anchor-cut widening caveat
    (s05b/s07/s08b, up to ~8 frames).

**Deliberately NOT wired by this batch** (for the record):
locomotion and cameramoves (unregistered); the `gait` block
and lipsync binding in the Rust loader (spec + data tooling
only); all three new validators in tools-ci (steps 3–4);
the shader checker as a gate anywhere (step 3); both splits
(blueprints only — no code moved).

## 6. Conflicts & coordination findings

1. **W21 step 5's premise is consumed — by the lead, during
   this gate.** The blueprint's §6 detailed a paint-budget
   wiring that its postscript warned might already be in
   flight; `86f0c81` landed it (plus RunLog) while the gate
   ran. The blueprint is not wrong — it is *partly executed*.
   The packet (§5.9) deletes step 5 and re-points G-TEL at
   the CHECK OK line (the blueprint's G-TEL still quotes the
   pre-re-chunk CHECK FAILED line — a second, independent
   staleness from `5e90d7e`, which predates the batch).
   **Flagged for the lead to ratify; this gate resolves
   neither.**
2. **No Batch-3 seat contradicted another on a name or a
   number.** The batch's cross-interfaces verified in
   source/output: W23→W15 (validator 5/5 on W23 output,
   `talk` channel pre-existing), W24→W15 (outputs land on
   existing channels; `gait` is a driver block per W15
   §5.3's own ruling), W27→W01/W92 (exact window agreement),
   W26→§14.5 R1/R3/R5 (encoded as checks 6/3/4).
3. **W29's note is missing.** `docs/integration/SHADERS-W29.md`
   is referenced twice by the tool (the packed-array
   field-order audit and the full limits list live there)
   and absent at gate close (last checked 17:17Z). The seat
   was still editing the tool at 17:15Z, so the note may
   land after this snapshot — the orchestrator should
   re-check at commit time before acting on §5.1's open
   call.
4. **The two splits share one seam to watch.** W22
   guarantees `crate::scene::` path stability; W21's split
   *depends* on that guarantee (film.rs's 21-name scene
   import is its largest external edge). If the scene split
   ever broke a path, the film split's re-export proof would
   fail in a confusing place. Packet order (§5.9–5.10)
   keeps them serial for this reason.
5. **Concurrent lead work is not seat work.** HEAD's five
   moves during the window and the uncommitted `src/dof.rs`
   gpucore migration (adapter/instance creation re-pointed
   at `crate::gpucore`, local `make_target` deleted —
   Batch-1 packet item 6 executing) are the lead's. Batch-3
   verdicts judge only the untracked seat files. The
   dof.rs diff must not ride the batch-3 commit (§5.1).
6. **`__pycache__` artefacts** (§5.1): tools/format (seats'
   validator runs), tools/ci (this gate's checker runs),
   plus the pre-existing tools/ab + tools/golden — all
   run by-products; exclude from the central commit.
   (Housekeeping note: a `git archive` snapshot extraction
   during this gate filled the 512 MB `/tmp` tmpfs; the
   census it produced before failing is the §2.6 control,
   and the snapshots were deleted after use.)

*End of gate record. This seat edited nothing but this file
and committed nothing.*
