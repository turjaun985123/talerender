# GPUFIRST Batch 3 — FILMPATH seat: film-path integration behind the flag

Branch `b3/filmpath` on base `01e3ea8` (gpufirst-phase0 tip).
Scope: wire the existing gpupaint executor into the film render
path behind the Batch 3 flag. No default flipped, no telemetry
code touched, no DESIGN-CARRY port touched, dizzy untouched.

## 1. The selection contract (ONE place)

Module `src/paint_backend.rs` (not feature-gated):

```rust
pub(crate) enum PaintBackend { Cpu, GpuPaint }
pub(crate) fn paint_backend() -> PaintBackend
pub(crate) fn gpupaint_enabled() -> bool   // == (paint_backend() == GpuPaint)
```

Rule: `GpuPaint` iff the binary was built `--features gpupaint`
AND env `TALERENDER_GPUPAINT` is exactly `"1"`. Every other
configuration (default build; env unset / `"0"` / `"true"` /
`""`) is `Cpu` — the CPU painter, byte-for-byte as before.
In feature builds the same predicate is re-exported as
`gpupaint::enabled()` (a delegation, not a second env read).
The telemetry seat should consume `crate::paint_backend::paint_backend()`.

`film::PaintCache::new_for_film()` calls `paint_backend()` ONCE
and pins the result for the cache's lifetime; only the two film
render paths (`run_film`, `run_film_stream`) use it.
`PaintCache::new()` — the telemetry constructor — stays
CPU-always: film telemetry is a CPU-only mode (W84 ruling D1)
and must not grow a GPU device because an env var is set.
When the GPU backend is selected, `new_for_film` prints one
evidence line: `paint-backend: gpupaint (feature gpupaint,
TALERENDER_GPUPAINT=1)`. CPU runs print nothing extra.

## 2. Wiring

`PaintCache::get_or_paint` (the single character-paint entry,
shared by render and telemetry via `resolve_chars`) now paints
a miss through `PaintCache::paint_miss`, which dispatches on
the pinned backend:

- `Cpu` → `toonfull::paint_toon_full` (unchanged call).
- `GpuPaint` → `gpupaint::paint_state_gpu(look, st)`.

`paint_state_gpu` (src/gpupaint.rs) returns the CPU painter's
exact tuple shape `(px, w, h, hand_l, hand_r)` — film's
`Painted` — so `resolve_chars`, the card composite, the B-1
canvas-anchor compensation and the rifle hand mapping consume
it unchanged. Hands/w/h are carried from the traced CPU paint
(they are a pure function of the same look+state; the GPU
replay does not recompute them).

How it paints: the only executor at this head is trace+replay.
The CPU paint runs inside a `gputrace` window (the
`record-states` path, verbatim), `prepare()` + `replay_gpu()`
replay the list on the GPU, and the read-back pixels replace
the CPU pixels. COST HONESTY: the CPU paint still runs (to
trace) and `replay_gpu` creates a fresh instance/adapter/device
per call, exactly as the `--gpu-paint replay` harness does.
This is integration, not the perf endgame; a persistent
executor is a later seat and must not change a pixel.

## 3. PaintCache semantics

Preserved, not replaced. The cache stores finished `Painted`
canvases keyed by the SAME exact `state_key` (all FullState
fields + hold_l/hold_r NAMES — F-V1 coverage unchanged) for
either backend; budget/LRU/eviction code is untouched, so a
hit costs no paint on either backend and eviction remains a
pure time cost. The GPU path therefore replays once per
distinct state, not once per frame. One accounting note:
`paint_state_gpu` wraps the replay in `gpucore::mem_scope()`,
so the film's end-of-run `gpu-mem:` line keeps its standing
meaning; the ledger's peak watermark is NOT restored by
design, so `peak_accounted_bytes` may sit higher in flagged
runs (accounting only, not a pixel).

## 4. Failure behaviour

No silent CPU fallback. If the GPU paint fails (empty trace,
prepare error, no adapter, readback error, size mismatch),
`paint_state_gpu` returns `Err` and `paint_miss` panics with
`gpupaint film paint failed (TALERENDER_GPUPAINT=1, no CPU
fallback by parity law): …`. A flagged census therefore either
contains GPU pixels or fails visibly — never unmarked CPU
pixels (parity law).

## 5. Flagged census

`.github/workflows/census.yml` gained a `workflow_dispatch`
boolean input `gpupaint` (default `false` = today's exact
behaviour). `true` → build `--features gpupaint`; the env
`TALERENDER_GPUPAINT=1` is scoped to the census render step
only (`0` otherwise); the telemetry step is never flagged.
The chunk logs' grep now also surfaces the `paint-backend:`
line as per-chunk backend evidence.

```sh
# unflagged (baseline)
gh workflow run census.yml --ref b3/filmpath -f gpupaint=false
# flagged
gh workflow run census.yml --ref b3/filmpath -f gpupaint=true
# flagged again (determinism pair with the previous run)
gh workflow run census.yml --ref b3/filmpath -f gpupaint=true
```

Diff the `== sha256 census-* ==` blocks across the job logs.
Caveat: each flagged chunk render still runs the CPU paint to
trace (see §2), so flagged census wall-time is NOT a GPU-perf
measurement.

## 6. Assumptions about the gpupaint executor API

- `prepare(&DisplayList)` and `replay_gpu(&list, &preps,
  &allocs)` were private, harness-shaped functions; this seat
  added `paint_state_gpu` in the same module to wrap them
  rather than changing their signatures.
- `replay_gpu` is self-contained per call (own instance /
  adapter / device / pipelines) and deterministic (Batch 2:
  221/221 byte-identical) — relied on, not re-proven here.
- `gputrace` windows are thread-local; the film paint path is
  single-threaded per cache (Rc-based), so tracing inside
  `get_or_paint` cannot interleave with another paint.
- The traced canvas is always 2560x2816 (`paint_toon_full`'s
  fixed canvas), matching `record-states`.
