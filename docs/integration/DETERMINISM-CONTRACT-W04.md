# Deterministic-rendering contract — W04

**Status: contract defined; enforcement job written, not yet run.**
This document states precisely what TaleRender promises about
repeatable output, the mechanisms that uphold the promise, the
evidence on record, and the two runs that formally discharge the
"determinism byte-compare" item on the integration README's
TBD-PROOF list. It is the Wave-2 answer to the audit's §E call
for a dedicated deterministic-rendering contract.

Companion pieces:

- The instrument: `tools/determinism/` (Wave-1 pack, W67;
  currency re-verified W78; local run attempt W96 —
  `docs/integration/DETERMINISM-RUN-W96.md`).
- The CI job: `.github/workflows/determinism.yml` (this seat) —
  W96's discharge route 1, as a standalone dispatch workflow.
- The PC leg: §7 of this document (ready to run; **execution
  pending**).

## 1. The guarantee

> **Same scene data + same time inputs + same binary + same
> renderer ⇒ byte-identical frames.**

Unpacked, every term is a named, checkable thing:

- **Same scene data** — the same film/scene files, byte for byte
  (in practice: the same repo commit). The film validator
  enforces exact-f32 round-trip on the scene data, so "same
  data" cannot silently drift through re-serialisation.
- **Same time inputs** — frame indices (film) and every derived
  `t` are data, computed from the frame index and the data
  alone. Nothing reads the wall clock, an RNG, or the
  environment to decide what a frame contains.
- **Same binary** — identified by sha256 (the pack records it in
  its results file). A different build is a different claim
  (§2).
- **Same renderer** — the adapter + driver the binary prints on
  its `renderer:` line (name, backend, device type, driver
  version string). This is the unit the guarantee is indexed
  by; see §2 for why it cannot be widened.
- **Byte-identical frames** — the output PNG files hash equal
  (sha256), frame for frame.

Two corollaries, both consequences of the same mechanisms (§3),
both explicitly inside the guarantee:

- **Chunking and resume change nothing.** Rendering frames
  [a, b) in one invocation, or [a, m) + [m, b) into the same
  directory with the second run resuming past existing files,
  produces the same bytes. (This is the integration README's
  resume-purity claim; the pack is its test.)
- **The paint-cache cap is pixel-neutral.**
  `TALERENDER_PAINT_CACHE` (default 160) trades time and memory,
  never pixels: a render at cap 24 produces the same bytes as
  one at the default cap.

The guarantee covers the film driver (`--film`) directly — that
is the surface the pack exercises — and the other render modes
by the same mechanisms, without a separate per-mode proof.

## 2. What is NOT promised

- **No cross-vendor / cross-driver bit-identity.** This is a
  physical limit, not a backlog item, and the contract names
  the two known mechanisms:
  - The GRADE grain hash uses `sin()` in fp32; lavapipe and a
    hardware GPU (RADV) will not produce identical hash values
    frame-wide (docs/ab/composer/gaps.md, item 2). The term is
    bounded by construction (amplitude ±0.006 of full scale —
    ≈ 49 dB PSNR ceiling if it decorrelated entirely), and for
    the film its *input* is pinned (§3), but its *evaluation*
    is the GPU's fp32 `sin()`, which is vendor territory.
  - Floating-point rounding differs across shader JITs and
    fp16 paths (the M3 finding: HalfFloat packed-depth storage
    truncates ~1 ulp on llvmpipe; the shipped default stores
    depth packing in Rgba8Unorm, lossless, partly for this
    reason).

  **The approach instead:** each renderer of record keeps its
  own golden frames (per-vendor goldens), and cross-vendor
  comparisons are made in **PSNR bands** — measured and
  reported, never gated as equality. That is the discipline
  every milestone A/B in this repo already follows (figures
  against a differently-rendered reference are calibration,
  with the same-engine cross-renderer band of 15.8–19.3 dB
  from the headless-gl spike as the standing reference point).
- **No identity across binaries or data.** A code change, a
  shader change, or a scene-data change makes a new claim:
  re-render, re-measure, re-pin. The commit census in the
  evidence register (§5) exists so a PASS is always cited
  against the exact tree it was measured on.
- **No timing or performance content.** The pack measures no
  time; the PERF protocol (docs/runbook/PERF.md) owns
  performance numbers, and none are claimed from CI lavapipe
  for the user's hardware.
- **The telemetry verdict law is separate.** `--film-telemetry`
  exits 0 on pass, fail, and error alike; its verdict is its
  printed line. That contract belongs to the telemetry gate
  and is not amended here.

## 3. Mechanisms that uphold the guarantee

1. **Pure frame(t) evaluation.** Animation and scene state are
   a pure function of (t, anchors, seeds). A frame's content
   never depends on previously rendered frames, on where an
   invocation boundary fell, or on how the output directory
   was populated — which is exactly why resume-by-existence
   (the driver skips PNGs already present) is sound, and why
   the pack can test purity by re-rendering.
2. **Exact f32 paint-cache keys** (fix `cabcc30`, E0600
   follow-up `f021991`). The paint cache's `state_key` once
   quantised continuous channels to 2 decimal places; the
   painter is continuous in those channels, so distinct states
   collided into one cache entry and five frames at a chunk
   boundary came out dependent on invocation history (§5, E2).
   Keys are now the exact f32 bit patterns. With keys exact, a
   cache miss repaints from the same pure state a hit would
   have returned — eviction is pixel-neutral by construction,
   at any cap (§1, second corollary).
3. **Pinned grain time** (ruling R3, docs/scene-format.md
   §14.5). `post.grade.time` is **0** for the whole film — no
   path passes film time into the GRADE grain shader. (Wave 1
   found seven scene files carrying film-time grade values —
   every one exactly its file's evaluation instant — and
   zeroed them; the validator-era data now carries 0
   everywhere.) Grain therefore sees fixed inputs only.
4. **Seeded scatter, no RNG state.** The engine's only
   randomness is the deterministic sin-hash `rnd(i)` from the
   skill's lib.ts/world.ts, reproduced exactly in
   `src/scene.rs` ("all randomness from the f64 sin-hash rnd()
   — no RNG state"). No system entropy, no clock, no
   iteration-order dependence enters a painted pixel.
5. **The f32 data law** (scene-format spec §11). Transcribed
   staging carries exact f32 values, so the data side of
   "same inputs" is enforceable by the film validator rather
   than assumed.
6. **Clean, lossless output** (decision (B), recorded at
   `801f85c`). Native frames are clean PNGs — no lossy
   generation sits in the output path whose encoder state
   could vary; PNG bytes for identical pixels are stable.

## 4. The instrument and the verdict discipline

The pack (`tools/determinism/determinism_check.sh`) renders
film frames **[100, 130)** at 1080×1920 four times:

| run | invocation | tests |
|---|---|---|
| A | `--from 100 --to 130`, default cache | the reference bytes |
| B | `--from 100 --to 115`, then `--from 115 --to 130` into the **same** dir | chunk boundary + resume |
| C | `--from 100 --to 130`, `TALERENDER_PAINT_CACHE=24` | cache-cap pixel-neutrality |

then sha256-compares every frame, A vs B and A vs C, and ends
`determinism-results.txt` with exactly one verdict line:

```
DETERMINISM: PASS              (script exit 0)
DETERMINISM: FAIL (n differ)   (script exit 1)
```

A broken run is neither: the script **refuses** (exit 2) —
stale binary (its capability probe names it), non-fresh output
dirs (a reused dir could "pass" by comparing a run against
itself), a crashed or short render, a missing `renderer:` line.
Refusal means *no evidence*, never a soft pass. The results
file records the binary's path + sha256 and run A's
`renderer:` line, so every verdict names the exact binary and
renderer it belongs to.

**Renderers of record** — a PASS is per renderer, and one
never stands in for the other:

1. **CI lavapipe** — ubuntu-22.04, Mesa 23.2, the `lvp` ICD
   pinned via `VK_ICD_FILENAMES` (the render.yml environment).
2. **PC RADV** — the user's PC: Ryzen 5 PRO 5650G, Radeon
   Vega 7 iGPU (gfx90c), RADV driver.

**Discharge rule.** The TBD-PROOF item clears when one pack
PASS per renderer of record is on record, cited as in §6.
Probe overlaps and partial routes (§5, E4) support the claim
but do not discharge it: only the pack exercises the same-dir
resume and the cache-cap section.

## 5. Evidence register

| # | Evidence | Renderer | State |
|---|---|---|---|
| E1 | Pack written (W67) and **stub-proven both directions**: identical stub bytes → PASS/exit 0; one flipped byte (frame 117) → FAIL naming it in both sections, exit 1; stale stub → REFUSAL, exit 2, no artifacts. Currency re-verified against source at `3781eb2` (W78): no drift. | — (instrument) | Instrument proven; no real frames |
| E2 | **Defect found by measurement** (W77, CI production renders at `381ff47`): overlap frames 1200–1204 byte-identical across invocations, but frames **1195–1199 differ at 72.34 dB** (1,787 px, 86% at \|Δ\|=1) — past the 160-entry cache cap. | CI lavapipe | Pre-fix; the measurement that exposed the bug |
| E3 | **Root cause + fix**: `state_key` quantised continuous channels to 2 dp → key collisions (lead; the wave's cache-eviction theory falsified — the pattern repeated digit-for-digit across independent fleets, which only the collision explains). Fix `cabcc30` (exact f32 bit-pattern keys), E0600 follow-up `f021991`. | — | Fix landed and pushed |
| E4 | **Lead probes at the final head `f021991`** (render.yml dispatch overlaps): frames 1195–1204 **10/10 byte-identical**, 795–814 **20/20**, and the v1 range 195–204 **10/10** — the previously failing boundary now passes, post-fix. | CI lavapipe | Strong support; **not** a pack verdict (no same-dir resume, no cache-24 run) |
| E5 | **W96 local pack attempt**: run A SIGSEGV before frame 1 — this VM's llvmpipe JIT wall (Mesa 25.2.8; render.yml's header documents it). The pack refused (exit 2) and claimed nothing. Record: DETERMINISM-RUN-W96.md. | VM llvmpipe | Environmental failure; evidence for nothing, recorded honestly |
| E6 | **Formal pack PASS, CI lavapipe** — via `.github/workflows/determinism.yml` (this seat). | CI lavapipe | **Pending**: workflow written; awaiting commit + first dispatch |
| E7 | **Formal pack PASS, PC RADV** — procedure in §7. | PC RADV | **Pending execution** on the user's PC |

Read plainly: the guarantee in §1 is *supported* — the one
measured violation (E2) was root-caused to a mechanism (E3)
that no longer exists, and the same boundary measures clean
after the fix (E4). It is not yet *discharged*: the formal
pack verdict (E6, E7) is the remaining step, and this contract
does not let the probes impersonate it.

## 6. How a PASS gets recorded

For each renderer of record, the citable record is:

1. the run's `determinism-results.txt` — its header block
   (binary sha256 + `renderer:` line), both
   `section: 30/30 identical (0 differ)` lines, and the final
   `DETERMINISM: PASS` line;
2. where it ran: for CI, the workflow run id + the
   `determinism-<run_id>` artifact name; for the PC, "user's
   PC, RADV" + the date.

When a PASS lands, append its row to §5 (date, renderer,
binary sha256, run id / artifact) — the register is the
record; the integration README's TBD list is then updated by
its owner with a pointer here.

## 7. The RADV leg — ready-to-run procedure (PENDING EXECUTION)

*This leg cannot run from the development VM (no GPU; its
llvmpipe segfaults on the film lane — E5). It runs on the
user's PC, in one sitting, ~four short renders. Nothing below
has been executed yet; it is labelled pending until the
results file exists.*

On the PC (Ryzen 5 PRO 5650G, Vega 7, RADV), in a checkout of
`turjaun985123/talerender` that contains `film/`:

```bash
# 1. Provision a current CI binary (any green CI run at or
#    after the CLI release; the results file records its
#    sha256 either way)
gh run list --repo turjaun985123/talerender --workflow CI --limit 5
gh run download <run-id> --repo turjaun985123/talerender \
    -n talerender-binary -D ~/talerender-bin
chmod +x ~/talerender-bin/talerender
~/talerender-bin/talerender --version   # sanity: 1.0.0 or later

# 2. Run the pack into a FRESH output root (it refuses non-fresh
#    dirs). Note its exit code as it finishes:
#    0 = PASS, 1 = FAIL, 2 = refusal (no verdict)
cd ~/workspace/talerender
tools/determinism/determinism_check.sh \
    ~/talerender-bin/talerender ~/det-run-onoda

# 3. Read the verdict — the last line of the results file
cat ~/det-run-onoda/determinism-results.txt
```

**Checks before trusting the result:**

- The results file's `renderer (run A):` line must name RADV —
  `device type: IntegratedGpu`, `driver: radv`. If it names
  llvmpipe, the wrong ICD was picked: pin RADV and re-run into
  a **fresh** output root:
  `VK_ICD_FILENAMES=$(ls /usr/share/vulkan/icd.d/radeon_icd*.json | head -n 1)`
- Runs A/B use the driver's default paint-cache cap **by
  design** — that is part of what is being tested; run C is
  the tight-cap run. The PC has ~6.7 GiB RAM: watch the run,
  don't pre-shrink the cap (PERF.md §4 owns any memory
  observations; this run claims no timings).
- Exit 2 is a refusal, not a FAIL: read its `REFUSAL:` line,
  fix the named cause, re-run into a fresh root.

**Expected artefact:** `~/det-run-onoda/` — `determinism-results.txt`
(the citable record, ending `DETERMINISM: PASS`), `logs/` (one
log per run), and the `A-single/` / `B-chunked/` / `C-cache24/`
frame dirs (kept until the result is recorded, so any DIFFER
can be inspected with `tools/ab/`).

**Where the result gets recorded:** send `determinism-results.txt`
back; the orchestrator appends the E7 row to §5 of this
document (date, binary sha256, both section lines) and cites
it in the wave close-out. With E6 also on record, the
TBD-PROOF item is discharged per §4.

## 8. Maintenance — what makes a new claim

Any change to painter arithmetic, the cache keys, the grain
inputs, the scatter seeds, the scene/film data, or the GPU
stack (wgpu/Mesa/driver) voids the register's PASSes for the
affected renderer: dispatch `determinism.yml` again (CI leg)
and re-run §7 (PC leg) before citing determinism for the new
tree. A FAIL is a defect report, not a nuisance: it names
every differing frame, and the kept frame dirs +
`tools/ab/measure_ab.py` are the diagnostic path. Never answer
a FAIL by widening the comparison — the contract's tolerance
lives in the cross-vendor PSNR bands (§2), never inside a
single renderer's byte-compare.
