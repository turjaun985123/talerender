# GPUFIRST-PERF — Phase 0 performance shape: CPU paint vs GPU replay

**Seat:** PERF (GPU-first Phase 0). **Repo HEAD verified:** `bc12a5c2bbb33d93416f25eaebbe8fec8c4eb678` (`bc12a5c`). This seat does not commit; the coordinator integrates.

**Code delivered:** `src/gpupaint_time.rs` (new) + two flags in `src/main.rs` (`--gpupaint-time [--gpupaint-no-gpu]`, wired unconditionally — the harness is measurement-only and renders nothing) + `.github/workflows/gpufirst-perf.yml` (workflow_dispatch; builds default, tests, builds `--features gpupaint`, runs the harness on lavapipe, uploads the log). With `--features gpupaint` the harness additionally records each state's **display-list composition** through the TRACE seat's `gputrace` recorder (prim counts by class, `GPUFIRST-TIME COMPOSITION` lines). The STATES seat's canonical spread lives in `src/gpupaint_states.rs::state_spread()`; this harness's spread (below) was authored independently against the same axes (registry looks, all 19 moods, legs/poses, channel extremes, look branches) because the module wiring was still landing while this seat ran — the coordinator can point `run()` at `crate::gpupaint::state_spread()` with a one-line change once the duplicate `mod gpupaint` declarations settle.

---

## 0. What was actually measured — and what was not

Three different things are measured; they must not be conflated.

1. **CPU paint (REAL).** `toonfull::paint_toon_full` wall time per state, median of 9 timed repetitions after 1 warmup, `std::time::Instant`. This is the production painter, unmodified (default-features build; the trace hooks compile out).
2. **GPU replay (PROXY — read this before quoting).** At this head the GPUX executor's `replay` subcommand is a loud stub (exit 3, "GPU executor not yet integrated") and **no GPU SDF/display-list replay exists yet**. What this harness times on the GPU side is the texture path the film renderer already uses for character cards: upload of the painted 2560×2816 canvas (`queue.write_texture`, timed separately) + a full-canvas textured-quad render, submit→complete (median of 9), plus wgpu **timestamp-query** ms when the adapter exposes `TIMESTAMP_QUERY` (labelled per line: `wall_median_ms` is host submit→complete; `gpu_timestamp_median_ms` is GPU-side pass time). This proxy is the *floor* of any GPU-first design that keeps painted canvases and the *pipeline shape* (dispatch, upload, full-canvas raster, timestamp plumbing) a true prim replay will reuse. It is **not** the cost of evaluating the toon's primitives on the GPU, and no number in the GPU column may be quoted as "GPU paint time".
3. **F_gpu (PROXY, INTEG's named requirement).** The per-paint GPU fixed cost: an empty submit (dispatch alone) and a replay whose source is an all-transparent canvas (dispatch + full-canvas raster, zero prim content), same median-of-9 protocol. `F_gpu := empty-replay median`. The marginal cost of prim content in any future executor is then `(per-list replay) − F_gpu`, and INTEG's break-even inequality should consume F_gpu in exactly this form.

> ⚠ **SOFTWARE RENDERING — LAVAPIPE ONLY.** Every GPU-side number in this note was produced under lavapipe/llvmpipe, where the "GPU" rasterises on the same CPU cores as everything else. These numbers **bound the algorithm's work and prove the pipeline shape; they do NOT predict RADV timings** on the user's Vega 7. The RADV leg comes later on the user's PC; the harness is adapter-agnostic and runs there unchanged (`./talerender --gpupaint-time`, no ICD pinning — wgpu picks the hardware adapter). **Portability caveats:** (i) the harness requests `wgpu::Limits::default()` because downlevel limits cap textures at 2048 and the canvas is 2560×2816 — same constraint as the production paths; (ii) timestamp queries are requested only when the adapter advertises `TIMESTAMP_QUERY` (RADV does — the user's PC mem-report already shows `gpu-timestamps: available`); when absent, the GPU-side column prints `na` and only wall times stand; (iii) no surface/window is ever created, so headless PC runs need no display server.

**Execution provenance.** Numbers below come from a local run of the exact CI binary configuration on this seat's VM (1.5 shared vCPU, ~6 GiB — heavily throttled; absolute ms are therefore *pessimistic* relative to a CI runner and say nothing about the user's PC): log `~/workspace/talerender-evidence/gpufirst-phase0/perf-local-vm-run1.log` (adapter line and full per-state lines in the log). The CI reproduction is `.github/workflows/gpufirst-perf.yml` (dispatch after the coordinator pushes this branch); its run id and log excerpt join the evidence dir when harvested — until then, treat the CI cell as **pending**, not missing: the workflow file, the grep checks (`GPUFIRST-TIME SUMMARY`, `GPUFIRST-TIME F-GPU`), and the artifact name (`gpufirst-perf-log`) are all in-tree.

---

## 1. CPU paint cost per state

Protocol: 77 states (11 registry rests, 19 moods, 10 leg states, 5 film poses, 12 channel extremes incl. the B-1 lying-lean re-anchor case, 19 look branches: 3 patterns, 11 hats, 3 neckwear, all-extras, kid rest/sit). Per state: median of 9, 1 warmup. Coverage = non-transparent pixels of the painted canvas (the work proxy; every state paints onto the same 2560×2816 = 7,208,960 px canvas, of which a standing figure covers ≈ 540k px ≈ 7.5%).

**Run 1 COMPLETE (CPU side):** `perf-local-vm-run1.log` — full spread, `SUMMARY cpu-state-ms n=77 min=1693.590 median=3378.190 p95=5075.786 max=7523.446 mean=3435.385` (per-state lines in the log are the record). Run 1's GPU phase died on a harness bug (timestamp buffer usage flags, wgpu validation) **after** the adapter line printed (`llvmpipe (LLVM 20.1.2) … Mesa 25.2.8 … (software)`, `timestamp_query_available=true`); the bug is fixed (two-buffer resolve→staging readback) and the GPU phase was re-run standalone as run 3 (`perf-local-vm-gpu-run3.log`, `--gpupaint-gpu-only`) — see §2. The CI dispatch of `gpufirst-perf.yml` reproduces everything on an unthrottled runner.

| statistic (CPU ms/state, this VM) | value |
|---|---|
| min | 1,693.6 |
| median | 3,378.2 |
| p95 | 5,075.8 |
| max | 7,523.4 (registry09-rest, Tani look) |

Heaviest states (run `CPU-HEAVIEST` lines): registry09-rest 7,523 ms and registry08-rest 7,471 ms — both the **Tani look** (vest pattern + tie + glasses + mustache: the registry's maximal branch stack), coverage 556,620 px; then sit-deep 5,517 ms (deep sit drop), registry07-rest 5,076 ms (scarf look), registry10-rest 4,984 ms, mood-Laugh 4,928 ms, jump 4,710 ms. **Load-noise caveat:** this VM is shared and throttled (1.5 vCPU across all Phase-0 seats); identical Tani instances at different spread positions measured 2,886–7,523 ms across the run, so cross-state ratios carry that noise and the CI run is the citable comparison. The ordering claims that survive: the maximal-branch look tops the table, deep-sit is the heaviest leg state, and no state is cheap in absolute terms on this class of throttled CPU — the *shape* (seconds-scale CPU paint per state on constrained cores) is the finding the PC/CI numbers will rescale, not reorder.

Battery cards (run 1, median of 9): prims **800 ms**, toon **1,780 ms**, face-neutral 1,455 / face-determined 1,401 / face-angry 1,445 / face-laugh 1,613 / face-shock 1,663 ms.

Headline shape:

- The distribution is **wide**: the cheapest states are the sparse registry rests; the heaviest are the states the brief predicted — but the mechanism is worth naming precisely. Cost does **not** track coverage (all standing states cover ≈ 540k px, yet medians differ by >2× between registry rests alone). What tracks cost is *which painter branches run*: the Tani look (vest pattern + tie + glasses + mustache) is the heaviest registry rest; masked fills (`polygon_mask` is a full-canvas 7.2M-cell scan, and every pattern/shade/highlight fill pays one) and plate-free pattern work dominate. In other words: **per-state CPU cost is a prim-composition cost, not a pixel-coverage cost** — which is precisely why the per-class split in §4 matters and why a GPU executor's win will be prim-shaped, not canvas-shaped.
- Battery cards (same protocol): `prims` (the M4 primitive card), `toon`, and the five `face-<mood>` cards are in the log as `GPUFIRST-TIME CPU battery=*` lines. The prims card is a *sparse* card (strokes on a mostly empty 2560×2816 canvas) and is correspondingly cheap next to a full `paint_toon_full`; the toon/face cards use the M4 five-mood painter (`scene::paint_toon`), a subset of the production painter, and are cheaper than production states for the same reason.

## 2. GPU replay cost per state (lavapipe, proxy)

Same table shape as §1, from the log's `GPU-REPLAY-PROXY` lines + `SUMMARY gpu-replay-wall-ms / gpu-upload-ms / gpu-timestamp-ms` lines:

**Measured (run 3, `perf-local-vm-gpu-run3.log`, exit 0, all 77 states; adapter `llvmpipe (LLVM 20.1.2) … Mesa 25.2.8 … (software)`; timestamp queries available and used):**

| quantity (lavapipe, this VM) | min | median | p95 | max |
|---|---|---|---|---|
| GPU replay wall, submit→complete (ms/state) | 90.8 | **104.0** | 115.6 | 123.3 |
| GPU timestamp, pass time (ms/state) | 89.1 | **103.0** | 112.5 | 120.5 |
| Canvas upload 2560×2816 (ms/state) | 11.5 | **12.1** | 16.6 | 28.8 |
| **F_gpu** — empty submit (dispatch alone) | — | **0.025** | — | — |
| **F_gpu** — empty-canvas replay (dispatch + full-canvas raster, zero prims) | — | **102.8** | 258.4 | — |

Three structural findings, all lavapipe-shaped and labelled as such:

1. **The proxy's cost is ~100% fixed.** Per-state replay (104.0 ms) ≈ F_gpu empty replay (102.8 ms) ≈ GPU timestamp (103.0 ms): the painted content of the texture changes the cost by ~1 ms. On lavapipe a full-canvas 7.2M-fragment pass *is* the cost, regardless of prims — which is exactly the F_gpu form INTEG's inequality needs: on this adapter, `X ≈ F_gpu + ~0`. On RADV the same decomposition must be re-measured (the fixed raster will collapse by orders of magnitude; the *prim-marginal* term, invisible here, is what will remain) — the harness prints all three lines so the PC run produces the RADV decomposition with no code change.
2. **Wall ≈ timestamp** (104.0 vs 103.0 ms): submit/complete overhead on this path is ~1 ms; the pipeline shape is not host-bound.
3. **Upload is real but second-order here:** 12.1 ms median to move 28.8 MB — small next to lavapipe's raster, but on RADV, where the raster term collapses, a per-paint 28.8 MB upload could become the *dominant* per-paint cost. That is a concrete, measured argument for the born-on-GPU painter over any CPU-paint + upload hybrid.

Same-VM shape comparison (both sides throttled identically, so the *ratio* is the content): CPU paint median 3,378 ms/state vs proxy replay+upload ≈ 116 ms/state — a ~29× gap even with the "GPU" being the same CPU cores running llvmpipe. This does **not** mean GPU paint is 29× faster (the proxy does not paint); it means the full-canvas raster + upload *envelope* inside which a real GPU painter must fit is ~29× cheaper than the CPU painter on the same constrained hardware — the headroom the executor has to spend on prim evaluation before it stops winning.

Reading: on lavapipe the resident-texture replay wall time is dominated by llvmpipe rasterising 7.2M fragments on CPU cores — it is the *same order* as the CPU paint itself, which is the expected software-renderer result and exactly why these numbers cannot be extrapolated to RADV. The two transferable facts are structural: (i) **upload is a first-class cost** — moving 28.8 MB per canvas is not free even before any prim is evaluated, and any GPU-first design that still uploads whole canvases per paint pays it per paint; a true GPU painter avoids the upload entirely (the canvas is born on the GPU), which is an argument *for* the GPU-painter design and *against* a hybrid that paints on CPU and uploads; (ii) **F_gpu** (empty replay) vs per-state replay bounds the fixed/variable split of the proxy: the fixed part is the full-canvas raster itself, i.e. on real hardware the floor per paint is one 7.2M-fragment pass plus dispatch — small, but multiplied by Profile B's paint count (§3) it becomes the term that decides the design.

## 3. Film economics

### 3.0 Inputs (provenance-locked)

- **FREQ seat's measured census totals (ONLY source used for Profile A):** 1,811 frames → **3,612 character-paint requests, 1,000 hits, 2,612 misses** (chunk-summed; the cache resets per chunk, so chunk-summed misses *overstate* single-process paints), hit rate **27.7%**, **2,042 evictions, 100% byte-budget**. Single-process distinct states **D ∈ [562, 2,612]**, hence single-process paints ∈ [D, 2,612]. (The figures in INTEG §8.3 — hits=1204/misses=611 and its derived "5.9× larger" sampled saving — do **not** match the measured logs, are flagged by the coordinator as an unresolved citation discrepancy, and are **not used anywhere in this note**.)
- **Per-state CPU paint cost:** §1's median, written `C` ms/state below. Per-state GPU paint cost (future executor, RADV): written `X` ms/state — **unknown, never measured on RADV**; all GPU-first totals below are stated as functions of X, with F_gpu's lavapipe proxy as the only measured GPU anchor.
- **Census chunk elapsed totals:** the CI bench of record (run 43623743480, frames 0..100, lavapipe) rendered 100 frames in 185.32 s wall with 89 paints in that window — i.e. paint *count* is known per window but the logs cannot separate paint *time* from raster time on a software renderer (BENCH-FIRST-W64 §3.2). The paint share below is therefore computed as `paints × C / total`, with the total taken from the same class of run, and labelled an estimate wherever the two sides come from different machines.

### 3.1 Profile A — staged poses (measured profile)

Paints executed per film render: **2,612** (chunk-summed upper bound, cache resets per chunk) down to **D ∈ [562, 2,612]** single-process (a single process reuses across chunk boundaries, so its paints = distinct states actually evicted-and-repainted, bounded below by D and above by the chunk-summed 2,612).

- **Total CPU paint time per film** = paints × C. At the chunk-summed bound: `2,612 × C`. Illustrations: at C = 50 ms → 131 s of pure paint per film; at C = 500 ms → 21.8 min; at run 1's median C = 3,378 ms → ≈ 2.45 h (throttled-VM illustration only — it prices this VM, not any target machine). On any machine where C stays in the tens-of-ms range or worse, paint is the dominant single line item of the render. (Insert the CI/PC C when quoting; the formula is the deliverable.)
- **Paint share:** in the CI bench window (frames 0..100: 89 paints, 185.32 s wall, lavapipe), paint share = `89 × C_CI / 185.32 s`. On lavapipe both sides are CPU work and the share is meaningful as a *work* share even though neither side predicts RADV wall time.
- **GPU-first projection (RANGE, assumptions stated):** film paint time becomes `paints × X + (cache/GPU-residency effects)`. GPU-first beats CPU paint per state iff `X + F_gpu_marginal < C`, where the F_gpu term is already inside any honest X (a per-paint GPU cost quoted without its dispatch+raster fixed cost is not a per-paint cost). **Breakeven X:** GPU-first stops mattering when `paints × (C − X)` falls below the *rest* of the frame pipeline's per-film time — i.e. when paint is no longer the bottleneck. Concretely, with 1,811 frames and a non-paint per-frame cost of `R` ms (3D scene + composer + encode), paint's share after GPU-first is `paints×X / (1811×R + paints×X)`; at paints = 2,612 and X = 5 ms, paint is 13.1 s/film — below any plausible `1811×R` — so **any X ≲ 5 ms/state makes paint a solved problem under Profile A, and further per-state optimisation buys nothing**. The interesting threshold is the other direction: X must merely beat C by enough to cover integration complexity; even X = C/2 halves a five-to-twenty-minute line item.

### 3.2 Profile B — per-frame sampled poses (MODELLED, not measured)

Native projects are now authored with per-frame sampled character poses (filmseq per-frame root clone + `time`; clips are rejected by the loader, scenefile.rs ~574). Consequences, per the coordinator's update and FREQ's Profile B:

- The generic path has **NO cache** — `scenefile.rs:752` paints unconditionally, **0% hit rate by construction**. Every character-frame paints: **≈ 3,622 paints per film** (FREQ's Profile B count; the staged profile's 3,612 requests become ~3,622 unconditional paints — the small delta is the generic path's staging, FREQ's number, used as given).
- **Saving ratio, parametrically** (coordinator's form): with Δ = C − X (per-state saving, ms),
  - staged saving = `staged_paints × Δ`, `staged_paints ∈ [562, 2,612]`;
  - sampled saving ≈ `3,622 × Δ`.
  The sampled saving is therefore **1.39× the staged chunk-summed bound and up to 6.4× the staged single-process floor** — stated as a range from the measured bounds, not a point ratio (and emphatically not INTEG §8.3's disowned 5.9× point claim, which rested on the mismatched 611-miss figure).
- **Breakeven under B:** with paints at ~every character-frame, even a modest Δ compounds over 3,622 paints (Δ = 10 ms → 36 s/film; Δ = 100 ms → 6 min/film). Conversely the F_gpu fixed cost is now paid 3,622 times with no cache to amortise it: GPU-first wins under B iff `X (= F_gpu + marginal prim cost) < C`, full stop — there is no hit rate to hide behind. This makes the F_gpu measurement (§0.3) *more* decision-relevant under B, not less: if F_gpu on RADV were ever to approach C, the entire GPU-first thesis fails under B while still succeeding under A.
- **Does the cache still earn its complexity under B? (Analysis.)** Largely **no — as a paint cache**. Under B the working set (near-unique state per character-frame) exceeds any feasible residency by construction: at 28.8 MB/card and ~74-card residency under the 2 GiB byte budget, a cache can only hold ~2% of a film's sampled states, and sampled neighbours differ by construction (per-frame sampling is *designed* to not repeat bit-exact states — the exact-key law means near-identical poses still miss). What survives is a much thinner device: a **within-shot repeated-pose memo** (holds, cycles, and blink/talk plateaus do repeat bit-exact values inside a shot) — i.e. the cache degenerates from a film-level economy to a *latency device for repeated poses within a shot*, valuable only where the staging actually repeats states. Recommendation (analysis, for the flip decision): under B, keep the exact-key memo with a *small* residency (its hit rate will be whatever within-shot repetition gives, likely ≪ Profile A's 27.7%), and spend the complexity budget on making the miss path — the GPU paint itself — fast, because under B the miss path *is* the product.

### 3.3 Cache interplay under faster paint (analysis)

At 28.8 MB/card and ~74-card residency under 2 GiB: faster paint **does** change optimal cache policy, in the direction of *smaller* residency. The cache exists to avoid repaint cost `C`; its price is resident bytes (28.8 MB × entries) plus eviction churn (Profile A already shows 2,042 evictions, 100% byte-budget-driven — the cache is memory-bound, not count-bound). When paint cost falls from C to X, the break-even reuse distance shrinks proportionally: an entry is worth holding only if it is re-hit before eviction with probability ≳ (holding cost)/(X saved). At X ≈ C/10, entries must be ~10× more likely to re-hit to justify the same bytes — under Profile A's measured reuse (27.7% hit rate at 74-card residency) a GPU-first cache at the same residency would be mostly dead bytes. The byte budget, not the entry cap, should therefore be *re-derived from X* at flip time (start at ~24 cards ≈ 0.69 GiB, the telemetry configuration, and measure), rather than carried over from the CPU era's 2 GiB. Under Profile B this collapses to §3.2's conclusion: minimal memo, maximal miss-path speed.

## 4. Per-prim-class CPU cost split

Two independent legs, both in the log:

1. **Synthetic single-prim micro-costs** (`GPUFIRST-TIME PRIM` lines, run 1): each Painter class looped 200× (20 warmup) on one production-size canvas, µs/prim: fill_polygon-bean113 **2,499,295 µs**, stroke_polyline-long113 **563,413 µs**, stroke_ellipse-head **388,908 µs**, fill_ellipse_masked-head **194,874 µs**, polygon_mask-bean113 **187,606 µs**, fill_rect-fullcanvas **123,067 µs**, fill_ellipse-head 6,823 µs, stroke_polyline-limb3 5,754 µs, fill_ellipse-small 87 µs. Labelled SYNTHETIC with a hard caveat: these looped figures were measured late in a long throttled run and the largest classes exceed what the same operations can cost inside a full paint (a whole state's median is 3,378 ms, which bounds the sum of its ~dozens of prims) — so the loop numbers are **not** additive per-prim prices. What survives is the *ordering*: polygon/ellipse-outline classes with large bboxes and per-pixel distance tests dominate; small filled ellipses are ~free; full-canvas scans (mask, fill_rect) sit in between. A citable per-class split needs the CI re-run of this section on an unthrottled runner; treat run 1's PRIM lines as ordering evidence only.
2. **Display-list composition × micro-cost** (feature build): `COMPOSITION` lines give each state's prim count by class; multiplying by leg 1's µs/prim gives an *estimated* per-class split per state (estimate, not a traced timing — the painter's real per-prim costs vary with bbox and coverage in ways a looped synthetic cannot see). The aggregate `COMPOSITION-TOTAL` line is the spread-level composition the GPUX executor should expect: strokes and ellipses by count, masked fills and masks by cost.

If the feature build's composition lines are absent from a given log (default-features run), leg 2 is **skipped for that run** and leg 1 + the INVB seat's structural per-paint prim census (`GPUFIRST-INVB-INVENTORY.md`) are the split of record — say so when quoting.

## 5. Limits of this note

- No RADV number appears anywhere; every GPU figure is lavapipe and labelled. No CI run id is claimed for the perf workflow until the coordinator's dispatch lands; the local VM run is labelled with its throttled hardware wherever its absolute ms are used, and the economics are stated parametrically so CI/PC numbers drop in without re-deriving anything.
- Medians with sample counts throughout (n=9 per state/card, n=200 for prim micro-costs); no single-run figures.
- The GPU column measures a texture-replay proxy, not the future SDF executor; when the executor lands, this harness's CPU table and F_gpu definition remain the comparison frame, and the executor's per-list timings replace the proxy column.
