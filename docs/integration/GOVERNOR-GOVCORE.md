# Resource governor — GOV-CORE record (GPUFIRST Batch 3, Part 6a)

User ruling 2026-11-18 12:00 +06 (scope addition to Batch 3) +
rider 12:01 (usable custom configuration). Branch `b3/govcore`
on base `e2099c4`. Seat GOV-PAR consumes exactly:
`crate::resource::governor() -> &'static Governor` and
`governor().thread_cap()` — unchanged by the rider.

## Contract surface (src/resource.rs, UNGATED, std only)

- `Governor` + `governor()` (process-wide, initialised once).
- Config chain resolved ONCE in `resolve`:
  CLI flags > env > config file > defaults.
  - Flags (main.rs, validated there, handed in via
    `resource::configure(CliLimits)` before any dispatch):
    `--max-gpu-pct`, `--max-ram-pct`, `--max-threads`.
  - Env: `TALERENDER_MAX_GPU_PCT`, `TALERENDER_MAX_RAM_PCT`,
    `TALERENDER_MAX_THREADS`; kill switch
    `TALERENDER_GOVERNOR=off` (enforcement off, reporting on).
    (The rider's MAX_ names supersede the original brief's
    `TALERENDER_GPU_CAP_PCT`-style names everywhere.)
  - Config file keys `max_gpu_pct` / `max_ram_pct` /
    `max_threads` in the existing hand-parsed talerender.toml:
    `--config <path>` file, else `./talerender.toml`, else
    `$XDG_CONFIG_HOME/talerender/talerender.toml` /
    `~/.config/talerender/talerender.toml`. Missing = never an
    error; malformed governor values = error.
  - Defaults: GPU 60, RAM 60, threads = half the logical
    cores (min 1).
- Validation: percentages are integers 1..=100, threads >= 1;
  invalid in ANY layer → `error: …` + help hint, exit 2
  (flags via main's cli_error; env/file via governor init
  calling the same `crate::cli_error`). Never a panic, never
  a silent ignore. ONE clamp: threads above the logical core
  count clamp to it, printed as `clamp=threads:<req>-><cores>`
  on the governor line.
- Accessors: `enabled()`, `gpu_cap_pct()`, `ram_cap_pct()`,
  `ram_budget_bytes()`, `thread_cap()`, plus derived shares
  and pacing figures. `ram_budget_bytes()` = cap% × total RAM
  from /proc/meminfo MemTotal (mempressure.rs's tested
  parser; NEVER a hardcoded machine). Detection failure →
  documented conservative fallback of 4 GiB total
  (`total_ram_detected=false` on the line).
- Visibility: the effective-limits `governor:` line prints in
  every run — stderr at startup (main, after configure; the
  pinned stdout surfaces stay byte-identical) and stdout
  beside the mem-report block (memreport_print).
- Presets (`--profile safe|balanced|full`): DROPPED per the
  rider's own escape hatch — the existing `--mem-profile`
  already occupies the profile namespace with different
  semantics; a second profile surface would complicate and
  confuse. The three explicit limits are the usable surface.

## Where the RAM budget is enforced

Shares of `ram_budget_bytes()`: paint cache 50%, dyntex pool
10%, buffer pool 5% (resource.rs constants, with rationale).

| Consumer | Enforcement | Site |
|---|---|---|
| Film PaintCache (host RAM, the big one) | Byte budget = min(existing TALERENDER_PAINT_BUDGET_MB budget, governor 50% share); TALERENDER_PAINT_CACHE entry cap remains an additional bound; byte budget wins when smaller. Entry bytes are the real canvas bytes via paintbudget `entry_bytes` (Painted.0.len()). | film.rs `PaintCache::with_backend`; paintbudget.rs `PaintBudget::cap_budget_bytes` |
| DynTexturePool (GPU/shared RAM) | Accounted via the existing gpucore ledger (`ledger_bytes()`, class E). Budget = 10% share, enforced as accounting + a one-time `governor:` over-budget log: slots are live textures for the frame being rendered and cannot be evicted mid-frame without changing pixels; slot count is bounded by concurrent characters in a scene, not by time, so the pool cannot grow across a film. | dyntex.rs `DynTexturePool::new` + `upload` |
| BufferPool (readback staging) | `BufferPool::from_governor()`: budget = min(64 MiB default, governor 5% share); its existing LRU retained-bytes enforcement + single-live-entry rule do the evicting. Honest note: the pool has NO production caller today (gpucore W84 §7.2) — this is the governor-wired constructor for its adopter, not a live path. | bufferpool.rs `from_governor` |
| AtlasPack | No code change: atlas.rs is a pure packing planner with no production caller and allocates nothing; its `total_bytes()` figure is a planning number that would sit inside the same budget picture. Wiring a budget into dead code was judged worse than stating this. | — |
| Frame buffers (film path) | No code change needed: already chunked/streamed — one frame in flight on the PNG path (render → overlays → save → drop), bounded queue on the stream path (mempressure frame_queue_capacity). Per-frame picture at 1080×1920: tight frame 8,294,400 B, padded readback 8,355,840 B (gpucore-pinned figures) — ~0.2% of a default 60%-of-7 GiB budget. | film.rs render loop (existing) |
| Renderer standing targets | Already accounted in the gpucore ledger (220,930,688 B steady at 1080×1920, W84-pinned) and printed via mem-report beside the new governor line; they are fixed-size per run, not a growth consumer, so no budget action applies. | gpucore.rs (existing) |

## Pressure behaviour

`PaintCache::governor_pressure_shrink` (film.rs), called on a
cache miss before painting: at >= 90% of the byte budget,
evict LRU down to 75% of the budget, floor = ONE live entry
(a single oversize canvas stays resident — paintbudget's
live-entry rule). This is recompute-over-store: an evicted
state repaints byte-identically on next use. The standing
eviction loop (entry cap + byte budget) is unchanged in
semantics and now shares the single `evict_lru` path.
The governor itself never panics and never OOMs: all
arithmetic is saturating/checked, locks recover from poison,
and every failure mode degrades to "paint again, slower".

Log lines (all greppable, all prefixed `governor:`, stderr
for action lines so pinned stdout surfaces are untouched):
- `governor: enabled=… gpu_cap_pct=… …` — effective limits,
  every run (also stdout at render tails).
- `governor: paint-cache budget_bytes=… share_pct=50 of ram_budget_bytes=…` — once per process at cache construction.
- `governor: paint-cache pressure shrink evicted=<n> bytes_before=… bytes_after=… budget_bytes=…` — per shrink event.
- `governor: dyntex pool over budget projected_bytes=… budget_bytes=… (… accounting only)` — once per pool.

## GPU cap — APPROXIMATION, labelled everywhere

A utilisation % cannot be enforced from userspace (wgpu
exposes no utilisation knob, duty-cycle API, or busy
readback). Implemented approximation (resource.rs
`gpu_max_in_flight` / `gpu_pace_gap_ms` /
`pace_gpu_submission`, called before the submits in
gpupaint.rs `replay_gpu` ×2 and film.rs `Renderer::render` ×1):
- max-in-flight derived from the cap: 1 per started 25%
  (60% → 3, 100% → 4). The tree's submit sites are serial —
  each submit is followed by a completion wait — so
  in-flight is structurally 1, always within bound.
- Submission pacing: a minimum inter-submission gap scaled
  by headroom, (100 − pct)/10 ms (60% → 4 ms, 100%/off → 0),
  enforced process-wide between paced submissions.
Code comments, the governor line (`gpu_pacing=approximation`),
--help, and the README all label this an approximation. It is
never claimed as a hard GPU-% guarantee.

## Byte-identity law

Budgets only evict/recompute; pacing only delays; no
governor value reaches a painter, threshold, or verdict.
Governor-on (defaults) vs `TALERENDER_GOVERNOR=off` produce
identical frame hashes by construction. The coordinator's
proof runs are the census dispatches below (default vs
forced-low-cap must hash identically).

## CI plumbing (census.yml)

New optional dispatch inputs `max_ram_pct`, `max_gpu_pct`,
`max_threads` (empty = governor defaults), exported as the
MAX_ env vars on the telemetry + census steps only when
non-empty. All census render + telemetry commands are wrapped
in `/usr/bin/time -v` (the `time` package was added to the
install step) so `Maximum resident set size` lines land in
the log, and `governor:` lines are grepped into the log.

Dispatch commands (coordinator):
- (i) Default-caps full-film run:
  `gh workflow run census.yml --ref b3/govcore`
  (flagged variant: `-f gpupaint=true`)
- (ii) Forced-low-cap pressure run:
  `gh workflow run census.yml --ref b3/govcore -f max_ram_pct=5`
  (expect: governor line with ram_source=env, paint-cache
  budget ≈ 5% × total × 50%, pressure-shrink lines, peak-RSS
  lines, and frame hashes IDENTICAL to run (i)).

## Infeasible-as-specified notes

- Hard GPU-% enforcement: infeasible from userspace on
  wgpu/Vulkan (no utilisation API) — approximation shipped,
  labelled, per the brief's own honesty clause.
- Hard eviction inside DynTexturePool mid-frame: infeasible
  without changing pixels (slots are bound into the frame's
  passes) — accounting + visibility shipped instead, with
  the structural bound (concurrent characters) stated.
- BufferPool/Atlas have no production callers to enforce
  in; see the table — constructor/planning figures wired or
  accounted, not silently dropped.
