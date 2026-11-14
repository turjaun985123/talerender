# GATE2-W20 — Wave 2 Batch-2 gate + integration packet

**Seat:** TaleRender Wave 2, W20 (batch-2 gate). **Date:** 2026-11-14.
**Gate window:** 2026-11-14 ~16:47Z (first sibling file observed)
→ 16:59:54Z (final snapshot).
**Tree state at gate close:** HEAD `ba88596`; working tree = 23
untracked paths (the nine seats' deliverables + `__pycache__`
artefacts, §6.7), **zero modified tracked files**. HEAD *moved
three times during this gate* — the lead landed its own Batch-1
integration in parallel: `aa10b13` (film-telemetry exit codes),
`5e90d7e` (caption driver leg → 73 chunks), `ba88596` (errors
wiring step 1). No lead commit touched a Batch-2 file. Batch-2
seats committed nothing, per the wave model.

**Method.** Deliverables judged on disk only. Every check below
was re-run by this gate against the seats' own fixtures; a seat's
pasted output is quoted only where this gate's run reproduced it.

---

## 1. Inventory — W11–W19 deliverables

| Seat | Deliverable(s) | Present | Size | Sanity (one line) |
|---|---|---|---|---|
| W11 | `src/anim.rs` + `docs/integration/ANIM-W11.md` | ✅ | 37,741 B / 1,052 lines; note 10,243 B | Zero-dependency by design (verified: no `use crate::`, no wgpu/image refs in the file); tracks/clips/easing with bit-stability laws |
| W12 | `src/charanim.rs` + `CHARANIM-W12.md` | ✅ | 34,931 B / 871 lines; note 10,348 B | Blink/idle/gaze/expression channels feeding `toonfull::FullState` fields by name; self-contained (no crate deps) |
| W13 | `src/rendergraph.rs` + `RENDERGRAPH-W13.md` | ✅ | 38,707 B / 917 lines; note 9,048 B | Pure-data graph layer (no wgpu types); fixture = `ab1_full_chain()` transcribed from ab1.rs + composer.rs (§4.3 verifies two passes against source) |
| W14 | `tools/format/check_prefabs.py` + `scenes/fixtures/prefabs/` (13 files) + `FORMAT-EXT-PREFABS-W14.md` | ✅ | validator 33,152 B / 799 lines; note 33,663 B; fixtures: library 1,758 B, valid scene 1,001 B, 11 invalid files (9 classes, 2 as library/scene pairs) | Spec + reference validator + fixtures; Rust loader untouched, stated plainly (§10 of its note) |
| W15 | `tools/format/check_clips.py` + `scenes/fixtures/clips/` (8 files) + `FORMAT-EXT-CLIPS-W15.md` | ✅ | validator 27,702 B / 646 lines; note 23,059 B; fixtures: 1 valid + 7 bad | Clips/tracks data model aligned to W11's names (§4.1); R3 guard enforced by name |
| W16 | `tools/ci/visual_regression.py` + `.github/workflows/visual-regression.yml` + `VISUAL-REGRESSION-W16.md` | ✅ | runner 18,819 B / 422 lines; yml 6,782 B / 151 lines; note 10,679 B | Per-pin drift runner importing the house registry + metrics; dispatch-only workflow |
| W17 | `tools/perf/bench_report.py` + `.github/workflows/bench.yml` + `BENCH-W17.md` | ✅ | parser 27,521 B / 669 lines; yml 7,436 B / 154 lines; note 12,607 B | PERF-protocol report generator with renderer-refusal laws; dispatch-only bench (frames 0..100, two cache legs) |
| W18 | `src/atlas.rs` + `ATLAS-W18.md` | ✅ | 27,999 B / 710 lines; note 14,054 B | MaxRects packer, no wgpu dep; pixel-neutrality *argued*, discharge explicitly deferred to post-wiring probes (§3, W18 verdict) |
| W19 | `src/bufferpool.rs` + `BUFFERPOOL-W19.md` | ✅ | 27,393 B / 655 lines; note 12,436 B | Generic CPU-side pool policy (no wgpu knowledge); budget aligned to W06 (§4.2) |

No seat is MISSING. All nine landed inside the gate window
(first file ~16:47Z, tree settled ~16:54Z, two quiet polls).

## 2. Mechanical checks — this gate's own runs

### 2.1 New Rust modules — standalone `rustc --test` (recipe per GATE2-W10 §2.5)

Toolchain: `rustc 1.99.1` (`~/.cargo/bin/rustc`), `--edition 2021`,
binaries in `/tmp/gate2-w20/`. All five modules are genuinely
self-contained — grep confirms **no `use crate::`/`super::`, no
wgpu, no image references in any of them** — so no stand-ins or
CI-pending cases arose this batch (contrast Batch 1's gpucore):

| Module | Claimed | This gate's run |
|---|---|---|
| `anim.rs` | 20 tests | **20 passed, 0 failed**; 0 warnings |
| `charanim.rs` | 12 tests | **12 passed, 0 failed**; 0 warnings |
| `rendergraph.rs` | 8 tests | **8 passed, 0 failed**; 0 warnings |
| `atlas.rs` | 11 tests | **11 passed, 0 failed**; 0 warnings |
| `bufferpool.rs` | 14 tests | **14 passed, 0 failed**; exactly the one dead-code warning its note discloses (`from_env`/`budget_bytes` unused pre-wiring) |

**Total: 65/65**, matching every seat's claim exactly.

### 2.2 Workflows — python yaml-parse + structure

Both parse OK (`yaml.safe_load`); both are **dispatch-only**
(`workflow_dispatch`), single job, `runs-on: ubuntu-22.04`,
11 steps each — `bench.yml` ("Bench (dispatch)") and
`visual-regression.yml` ("Visual Regression (dispatch)"), as
their notes describe. Parse-level only: no workflow runner
exists on this VM, so both remain **dispatch-unproven** (both
seats state this themselves; see the packet §5.9–5.10).

### 2.3 W14 prefabs validator — full fixture sweep (this gate)

```
valid (library + valid.scene.json)     rc=0  OVERALL: PASS (6/6 checks passed)
invalid-unknown-prefab                 rc=1  nodes[0]: unknown prefab 'ghost-fence' (in scope: marker-post, post-pair)
invalid-missing-param                  rc=1  nodes[0]: missing required param 'height' for prefab 'marker-post'
invalid-extra-param                    rc=1  nodes[0].params.weight: prefab 'marker-post' declares no param 'weight' (declared: capColor, capY, height, postColor, postY)
invalid-param-type                     rc=1  nodes[0].params.height: value 'tall' does not satisfy declared type 'number' of prefab 'marker-post' (got string)
invalid-override-forbidden             rc=1  nodes[0].overrides.cap.type: override touches forbidden key 'type' (…)
invalid-override-transform             rc=1  nodes[0].overrides.cap.transform: override touches forbidden key 'transform' (…)
invalid-override-unknown-pid           rc=1  nodes[0].overrides.roof: prefab 'marker-post' has no pid 'roof' in its direct scope (declared: cap)
invalid-cycle (bad library)            rc=1  prefab cycle: loop-a -> loop-b -> loop-a — expansion would not terminate (spec §6)
invalid-slot-undeclared (bad library)  rc=1  prefabs.broken-post (…).node.geometry.size[1]: $param 'widht' is not declared by prefab 'broken-post' (declared: height)
nonexistent scene                      rc=2  ERROR: check_prefabs.py: scene: cannot read /nonexistent.scene.json (No such file or directory)
```

**10/10 behaviours reproduced**, every message byte-identical
to FORMAT-EXT-PREFABS-W14 §9's pasted outputs.

### 2.4 W15 clips validator — full fixture sweep (this gate)

```
valid-clips.scene.json        rc=0  OVERALL: PASS (5/5 checks passed)
bad-unknown-channel           rc=1  [2 channels]  unknown channel 'camera.zoom' — camera channels are: eye, fovY, target
bad-unsorted-keys             rc=1  [3 keys]      key times out of order — 1.3 follows 2.6 (keys[1])
bad-duplicate-key-time        rc=1  [3 keys]      duplicate key time 2.6 (already at keys[1])
bad-key-beyond-duration       rc=1  [3 keys]      2.5 is beyond the clip duration 2.0 (clip 'bad-range')
bad-unknown-easing            rc=1  [3 keys]      unknown easing 'bounce' — the engine's curves are: linear, smoother, smootherstep, step, easeOutCubic, eo, easeInOutCubic, eio, easeOutBack, eob
bad-clip-cycle                rc=1  [4 references] reference cycle: cycle-a -> cycle-b -> cycle-a · cycle-self -> cycle-self (both reported)
bad-grade-time-r3             rc=1  [2 channels]  channel 'post.grade.time' is not animatable: R3 pins the GRADE grain time to 0 (…)
scenes/film/s01.scene.json    rc=2  ERROR: … no 'clips' block and no clip instances — nothing to validate
```

**8/8 reproduced**: each bad fixture fails *exactly one* check —
the designed one — with the note's verbatim message.

### 2.5 W16 visual-regression runner — re-run against its own synthetic fixtures (`/tmp/vr-w16/`)

| Run | This gate's result |
|---|---|
| Byte-identical renders (synthetic pins) | `MATCH` ×2 (hashes `20f8d94f37c1`, `7189bb1d23c9`), `VISUAL REGRESSION: PASS (2 pins, 0 drift)`, **exit 0** — identical to the note's Run 1, incl. "16 UNPINNED (of 18)" |
| Noisy pair (`ref/render-dof-noisy`) | `DRIFT`, PSNR **25.13 dB** → FAIL, **exit 1**. Same class/verdict path as the note's σ=10 run (28.13 dB on its own fabrication — a different pair, not a discrepancy) |
| Tiny pair (`ref/render-dof-tiny`) | `WITHIN-BAND`, PSNR **81.76 dB**, max Δ = 1 → PASS, **exit 0**. Reproduces the note's Run-2 class (its 1-LSB pair measured 84.77 dB; that exact pair does not survive in /tmp) |
| Absent render file | `VISUAL REGRESSION: FAIL (1 pins, 1 drift)`, **exit 1** (MISSING-RENDER) — as the note's Run 4 |
| Real repo state, references fed as their own renders | `MATCH` ×2 (hashes `957d5573497e`, `d7bd71a02a3d`), `Counts: 2 MATCH · 14 UNPINNED · 2 NOT-RENDERED (of 18 golden items)`, PASS, **exit 0** — byte-identical to the note's Run 5 (identity sanity, not render evidence — the note says so too) |
| Unknown item id / no `--render` | one `ERROR:` line each, **no verdict line, exit 2** — as the note's Run 6 |

### 2.6 W17 bench parser — re-run against its own synthetic sets (`/tmp/w17-test/`)

| Set | This gate's result |
|---|---|
| A — CI-style lavapipe | Report rc=0; renderer line + **software banner** present; §3 rows `108.20 s · 921600 KiB · 1.082 s/f` (cap 160) and `119.00 · 512000 · 1.190` (cap 24), extrapolation `—` for both — **identical to BENCH-W17 §3** |
| B — PC-style RADV | rc=0; ab1 warm median **2.89** (runs 2.90, 2.85, 2.95, 2.88), peak 406000 KiB; S01 median **1.69**; §6 rows native `24.60 s · 0.273 s/f · 13,171 f/h` and Chrome `13.50 · 0.150 · 24,000` — **identical to the note** |
| Mixed renderers | **exit 2**, refusal naming both renderer strings verbatim (llvmpipe + RADV Vega 7) |
| No renderer line | **exit 2**, refusal naming the PERF §1 remedy (copy the adapter line into `perf/env.txt`) |

Every number in these sets is the seat's fabrication, as its
note declares; what this gate verified is that the parser
produces the claimed tables and refusals from them, first-hand.

## 3. Per-seat verdicts

| Seat | Verdict | Basis |
|---|---|---|
| W11 | **ACCEPT** | 20/20 tests reproduced; zero-dependency claim verified in source; the f32-monotonicity caveat (smootherstep band ≤2e-6) is measured and disclosed, not hidden |
| W12 | **ACCEPT-WITH-NOTES** | 12/12 reproduced. Note: charanim is *independent* of W11's tracks — closed-form channel functions, no `anim` usage (§4.4). Lip-sync honestly out of scope with its data requirements named (phonemes, driven `talk`, speaker attribution) |
| W13 | **ACCEPT** | 8/8 reproduced; fixture spot-verified against composer.rs source (§4.3): composite→horiz[0] and the LoadOp::Load RMW add both exact |
| W14 | **ACCEPT** | Validator 10/10 fixture behaviours reproduced byte-for-byte; spec's honesty section (§10) is precise about the loader not implementing any of it |
| W15 | **ACCEPT-WITH-NOTES** | Validator 8/8 reproduced byte-for-byte; easing/wrap names exactly match W11 (§4.1). Notes: the ease-direction bridge is not restated in its §7 (§4.1), and colour/step channels have no W11 lane (§4.1 — unflagged by either seat) |
| W16 | **ACCEPT-WITH-NOTES** | Runner fully reproduced incl. the real-repo identity run. Notes: the note's specific 84.77 dB WITHIN-BAND pair does not survive in /tmp (class reproduced at 81.76 dB via the tiny pair); the workflow is dispatch-unproven and its expected first dispatch is a FAIL on the two stale pins — debt, not a job defect |
| W17 | **ACCEPT** | Parser outputs + both refusals reproduced verbatim on all four synthetic sets; bench.yml parse-checked only (no runner on this VM — disclosed by the seat) |
| W18 | **ACCEPT-WITH-NOTES** | 11/11 reproduced; determinism of the packer itself is test-pinned. Note: pixel-neutrality is *argued by construction*, not proven — the seat's own §7 makes post-wiring byte-probes + 60-pair A/B the discharge, and §5/§6 bound the win honestly (VRAM *costs* 1.55×/state; the win is upload traffic + churn, gated by the probation admission policy) |
| W19 | **ACCEPT** | 14/14 reproduced incl. the disclosed warning; budget philosophy aligned with W06 (§4.2) with one deliberate, documented divergence (0-semantics) |

**Batch-2 gate overall: PASS** — all nine seats landed, and
every runnable check this gate could execute is green:
65/65 module tests, 18/18 validator fixture behaviours,
6/6 visual-regression scenario groups, 4/4 bench-parser sets.
Nothing in this batch is wired into the crate yet — by design;
the open threads are integration steps with named owners (§5),
not defects.

## 4. Cross-seat coherence

### 4.1 W15 ↔ W11 (anim) — names exact; two bridges + one unflagged gap

- **Easing names: EXACT MATCH, verified in source.** anim.rs
  `Easing::from_name` (anim.rs:101) accepts `linear`, `step`,
  `smootherstep`, `smoother`, `easeOutCubic`, `eo`,
  `easeInOutCubic`, `eio`, `easeOutBack`, `eob` — precisely the
  ten spellings in W15's validator list and table (canonical
  `smoother` + alias `smootherstep`; camelCase canonicals +
  short aliases). Key field `value` matches `Key { t, value,
  ease }` (anim.rs:308); wrap modes match `Wrap::{Clamp, Loop,
  PingPong}` (anim.rs:166) ↔ clipTime `"clamp"/"loop"/
  "ping-pong"`. (W15's line citations — :98/:163/:305 — land on
  the items' doc comments, ~3 lines above the items; harmless
  drift from aligning mid-seat.)
- **Ease attachment direction DIFFERS — bridge exists, W15
  doesn't restate it.** anim keys carry easing-to-*next* (the
  key a segment *leaves*); W15 clip keys carry the §14.4
  convention (the key a segment *ends at*). W11 §4.1 documents
  the exact loader mapping for §14.4 keys (format key[i+1].ease
  → anim key[i].ease); clips generalised §14.4, so the same
  mapping applies — but FORMAT-EXT-CLIPS §7's flags cover only
  wrap/duration placement. The lead must apply the §4.1 rule in
  the clips loader too; nothing contradicts, one paragraph is
  missing.
- **Duration/wrap placement differs — disclosed by W15 (§7
  flag 1).** anim: per-`Track` duration + wrap. W15: duration
  on the *clip* (the validator's beyond-duration check needs
  one bound), wrap on the *instance* (`clipTime.loop`). Loader
  bridges mechanically; lead to ratify.
- **The unflagged gap: colour and step channels have no anim
  lane.** anim's `AnimValue` covers `f32`, `[f32; 3]`, `Quat`.
  W15's kinds S → f32 ✓ and V → vec3 ✓, but **C** (colour hex —
  needs the loader's hex→linear conversion before a vec3 track
  can carry it, per W15 §4's own colour law) and **Step**
  (`mood`/`legs` enum strings — anim has no string/enum track,
  and step values must not route through interpolation) have
  no specified evaluator path in either seat's document. Small,
  mechanical, and currently owned by nobody — the lead should
  assign it with the clips fold-in (§5.3).

### 4.2 W19 ↔ W06 — budget philosophy aligned

Verified against both modules/notes: byte accounting (never
entry counts as the risk measure); a stated small fraction of
the target PC's ~6.7 GiB usable RAM, derived from the real
working set (W06: 2 GiB ceiling ≈ 30% for the paint cache;
W19: 64 MiB default — `TALERENDER_BUFFER_POOL_MB`, constant
verified in bufferpool.rs — ≈ 1% of RAM, ~3% of W06's ceiling,
sized to the one-buffer-in-flight readback reality); the same
env semantics shape (unset/unparseable → derived default,
`N` → N MiB, greppable `stats_line()` in the same house style);
eviction byte-driven. **One deliberate divergence, documented
in BUFFERPOOL-W19 §3:** `=0` disables *enforcement* in W06 (the
cache exists independently) but disables *the pool* in W19 (a
pool has no existence apart from its budget). No conflict.

### 4.3 W13 fixture ↔ composer.rs — two passes spot-checked against source

1. **`bloom_composite` writes `bloom_h0` (reused horiz[0]).**
   composer.rs `PostChain::bloom`: `// Composite into horiz[0].`
   — the composite pass draws into `horiz[0]`'s view reading all
   five `vert` views. Fixture pass 16: reads `bloom_v0..v4`,
   writes `bloom_h0`. **Exact.**
2. **`bloom_add` is an RMW onto the read buffer.** Source: the
   add pass binds the composite view and draws onto `target`
   with `wgpu::LoadOp::Load` ("Additive blend over the read
   buffer"); in the ab1 chain `target` is the bokeh output.
   Fixture pass 17: reads `bloom_h0` + `bokeh_out`, writes
   `bokeh_out` (RMW). **Exact** — and the fixture's version-
   binding semantics exist precisely because a naive edge set
   false-cycles on this pass (RENDERGRAPH-W13, semantics note).
   (Bonus, same source read: the H/V cascade shape — H0 ←
   bright, V ← H, H ← previous V — matches fixture rows 6–15,
   and `bloom_bright` reads the chain input.)

### 4.4 W12 ↔ W11 — independent, not layered (ratify, don't repair)

W11 §4.3 maps poseTrack adoption as "W12's lane" and supplies
the exact anim-track encoding. W12 instead implements blink /
idle / gaze / expression as closed-form pure functions of
`(id, t)` and never cites anim.rs. The designs are
complementary, not competing: charanim's channels are
procedural laws (hashed blink schedules, breathing), while
W15 clips will keyframe *authored* character channels
(mood/headTurn/arm tracks) through W11's tracks. No file or
name conflicts. The lead should ratify this split at fold-in
so a future seat doesn't "fix" one into the other.

### 4.5 W18 ↔ W06 — complementary budgets, one shared machine

atlas's `stats_line()` deliberately mirrors paintbudget's
house style; the two govern different copies of the same
canvases (W06: CPU paint cache; W18: GPU pages, *additional*
memory). W18 §5 states the consequence itself: both budgets
must be counted together on the 6.7 GiB PC. No conflict; the
combined envelope belongs in the lead's wiring gate (§5.7).

## 5. Integration packet for the lead (landing order)

1. **Central batch-2 commit** (orchestrator): all §1 files.
   **Exclude the `__pycache__` directories** (tools/format —
   the seats' validator runs; tools/ab + tools/golden — created
   by this gate's W16 re-runs importing the house tools). CI
   expectations: `ci.yml` green, test count unchanged (no
   module is `mod`-declared yet); `tools-ci.yml` green (the new
   validators are not yet wired into it — W15 §7(d) is the
   wiring step, not a commit blocker).
2. **Module registration** (lead; `main.rs`, one line each):
   `mod anim; mod charanim; mod rendergraph; mod atlas;
   mod bufferpool;` → cargo test picks up the **65 new tests**.
   Expected residue: bufferpool's `from_env`/`budget_bytes`
   dead-code warning persists until step 8 consumes them
   (same pre-wiring pattern as paintbudget in Batch 1).
3. **Format fold-ins** (lead; `docs/scene-format.md`): W14 as
   §15 (prefabs), W15 next (clips). For clips, record the
   §4.1 decisions *as it lands*: the ease-direction bridge
   (W11 §4.1's rule), wrap/duration placement ratification,
   the colour/step lane assignment, and keep-or-drop `uses`
   layering (W15 §7 flags 1–5 are the checklist). **W14's §7
   rule binds:** the fail-loud reservation guard (named
   "prefab extension not supported" errors on the reserved
   keys, in scenefile.rs *and* film.rs's own node walk) lands
   with or before any prefab *data* — until then
   `check_prefabs.py` is the only consumer and no production
   file may use prefabs.
4. **W11 adoption, per-site and gated** (lead, ANIM-W11 §4):
   scenefile camera keys first — §28 anchor + film A/B, since
   the f64→f32 evaluation can differ ~1 ulp (disclosed, not
   silent); then film camK clips; poseTrack per §4.4's split.
5. **W12 adoption** (lead): film.rs — decide the blink paint
   cost first (`blink_amount_q` ≤8 levels vs ~7 paints/event
   against `state_key`, CHARANIM-W12's cost note), then
   channels behind A/B; scenefile path opt-in via the `"anim"`
   node block, default off = byte-identical for existing files.
6. **W13 migration steps 2–3** (lead, RENDERGRAPH-W13):
   declare + validate beside the existing recording code
   (ab1 uses `ab1_full_chain()` verbatim), then order from the
   graph — byte-identical by construction, since the fixture
   pins derived order == source order. Step 4 (transient
   allocation from lifetimes) is a **later seat with its own
   gate**, not part of this landing.
7. **W18 atlas wiring** (lead) — only with the §6 probation
   admission policy (slot on *second* paint-cache hit, lazy
   pages to `max_pages`, no mid-film eviction) and the §7
   proof list as the gate: byte-probes on frames 1195–1204 +
   795–814 atlased-vs-Dyn, 60-pair A/B with card-edge/silhouette
   inspection, and the traffic claim measured on the user's PC
   (RADV) — never asserted from this VM. Do not wire
   pack-everything.
8. **W19 pool wiring** (lead, BUFFERPOOL-W19 §4/§8): under
   gpucore's `Readback` (record/collect), film's `Renderer`
   owns the pool; the other six sites stay unpooled by design.
   Gate: byte-compare probes 1195–1204 + 795–814 at default,
   `TALERENDER_BUFFER_POOL_MB=0`, and `=1` (forced oversize
   path); telemetry verdict line unchanged.
9. **W16 first dispatch** (lead/orchestrator): expect **FAIL**
   on `ab1-composer` + `s28-onoda-t150` (pre-`c00897d` stale
   pins — the golden pack's own EXPECTED STALE record), with
   `dof-chart` as the clean control that distinguishes "job
   works" from "job broken". Nobody widens the 80 dB band or
   whitelists a pin to make it green; the pins are re-struck
   by the human re-baseline act, separately.
10. **W17 first bench dispatch**: harness verification only.
    Its numbers are lavapipe and carry the software banner —
    indicative, never quotable (PERF honesty rule). The PC
    RADV run remains the protocol's own pending item.

**Deliberately NOT wired by this batch** (for the record):
all five Rust modules (unregistered); prefabs and clips in
the Rust loader (spec + Python reference only); the atlas GPU
side and the buffer pool at any readback site; render-graph
*execution* (the graph validates/orders, it does not draw);
both new workflows (never dispatched); `check_clips.py` in
tools-ci (step 3/§7(d) of W15's adoption list).

## 6. Conflicts & coordination findings

1. **No seat contradicted another on a name or a number.**
   The batch's one exact-name interface (W15↔W11 easings) is
   a perfect match in source, not just in prose (§4.1).
2. **The colour/step evaluator lane is unowned** (§4.1) —
   the batch's only genuine gap. Assign at the clips fold-in.
3. **W15's §7 omits the ease-direction bridge** that W11 §4.1
   already solved for the identical §14.4 case — a missing
   paragraph, not a disagreement (§4.1).
4. **W12/W11 layering expectation** (§4.4): ratify the split;
   do not let a future seat merge them by assumption.
5. **Waiver tension — status changed during this gate.** The
   lead's `5e90d7e` (caption driver leg) landed while the gate
   ran; its commit message records the telemetry re-run at the
   73-chunk data as **`FILM TELEMETRY: CHECK OK`, exit 0** —
   the Phase-1 headline gate, lead-reported in the commit
   (this gate did not re-run telemetry: no binary at the
   current head exists on this VM, and builds are CI-only).
   The user's veto-at-close option over the re-chunk stands
   as the record prescribes; nothing in Batch 2 touches it.
6. **Concurrent lead work is not seat work.** HEAD's three
   moves during the window (`aa10b13`, `5e90d7e`, `ba88596`)
   are the lead's Batch-1 packet items 2/3/7 landing; Batch-2
   verdicts above judge only the untracked seat files.
7. **`__pycache__` artefacts** (§5.1): three directories,
   all run by-products — exclude from the central commit.

*End of gate record. This seat edited nothing but this file
and committed nothing.*
