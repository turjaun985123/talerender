# TaleRender V2 — Architecture

Source of truth: the tree at HEAD `712a6aa`. Line counts are
`wc -l` at survey time. "Wired" means another module in the tree
calls it on a production path (checked by searching for
`<module>::` outside the module's own file); "registered" means
only that `src/main.rs` carries its `mod` line.

## Module map — every `src/*.rs` at survey (31 files, 32,033 lines incl. WGSL)

> Final-check note: four Batch 2 seat files landed in `src/`
> after this map was surveyed — `lifetimes.rs` (612),
> `memreport.rs` (712), `streammodes.rs` (764),
> `outputmgr.rs` (933) — bringing the tree to 35 `.rs` files.
> They are unregistered (no `mod` lines) and are covered in
> `README.md`'s delivery note, `MEMORY.md` §7, `GPU.md` §4,
> `STREAMING.md` §1, and `MIGRATION.md` B.4 rather than in the
> tables below.

### Entry, errors, utilities

| File | Lines | One line |
|---|---|---|
| `main.rs` | 1,005 | CLI + entry points: parses argv against `const HELP`, owns `Knobs` (res/aa/shadow draft knobs), dispatches to built-in scenes, `--scene-file`, `--film`, `--film-telemetry`, `--telemetry`; registers all 29 modules below. |
| `errors.rs` | 368 | `TaleError`/`TaleResult` — the single structured error type; variants `GpuInit`, `DeviceRequest`, `BufferMap`, `Render`, `ImageIo`, `DataLoad` with `category()` and `exit_code()`. Full reference: `docs/integration/V2-ERROR-REFERENCE.md`. |
| `math.rs` | 297 | In-house column-major `Mat4` math (WGSL/wgpu convention); no external math crate. |
| `runlog.rs` | 327 | Run clock, progress/summary line formats, peak-RSS self-report from `/proc/self/status` `VmHWM`; log-only, never feeds a pixel or a verdict. |
| `observability.rs` | 418 | **V2 (WP12, C6). Registered, not wired.** Pure formatter + printers for one-line `event=<name> k=v …` records (`render.start`, `adapter.selected`, `frame.done`, `mem.pressure`, `output.finalized`); whitespace in values is folded to `_`. No production caller yet. |

### GPU core and post chain

| File | Lines | One line |
|---|---|---|
| `gpucore.rs` | 914 | The single canonical copy of instance/adapter/device creation, offscreen targets, texture→CPU `Readback`, PNG save — plus the VRAM ledger (classes A–G) and the `gpu-mem:` line (see `GPU.md`, `MEMORY.md`). Wired: used by `main`, `ab1`, `dof`, `scenefile`, `film`, `composer`. |
| `composer.rs` | 684 | Post chain machinery: PMREM room-environment capture, UnrealBloom (bright → 5-mip Gaussian → composite), Output and GRADE passes (`PostChain`). |
| `dof.rs` | 685 | M3 Bokeh depth-of-field gate scene (the chart) and the bokeh pass transcription of three.js r170. |
| `rendergraph.rs` | 920 | Render-graph declaration + validation layer (resources, passes, lifetimes, ordering). **Registered, no production caller** — the live post chain is still hard-wired in `ab1`/`composer`/`film`/`scenefile`. |

### Scene model, painters, animation

| File | Lines | One line |
|---|---|---|
| `scene.rs` | 2,979 | Shared scene building blocks: closed geometry vocabulary, the CPU `Painter`, environment texture painters, draw records, deterministic sin-hash, and the in-tree JSON parser (`parse_json`/`Value`) the whole crate uses instead of a JSON dependency. |
| `scenefile.rs` | 2,389 | Scene-format loader/renderer (v1/v2 JSON): direct path (single sRGB pass) or composer path when `post.bokeh` is present. Entry: `run_scene_file`. |
| `toonfull.rs` | 876 | Full `drawToon` port: all 17 moods, walk/run/sit/jump legs, patterns, neckwear, hats, extras, blink/talk/look, head tilt/turn, held-prop plates (stage space 512×1024, canvas 2560×2816). |
| `anim.rs` | 1,052 | Reusable keyframe tracks/clips; `sample(t)` is a pure function of `t`. Its header states it is opt-in; no renderer path calls it (references from `cameramoves.rs` only). |
| `charanim.rs` | 871 | Deterministic character channels feeding `toonfull::FullState` fields. **Partially wired:** `film.rs` calls `charanim::blink_amount` at its staging sites; the remaining channels are not wired into `film`/`scenefile`. |
| `locomotion.rs` | 693 | Procedural gait evaluation (walk/run, phase, foot state). **Wired:** `film.rs::gait_eval` calls `locomotion::eval` / `phase_for_distance` / `painter_phase` (the 2026-11-15 adoption). |
| `cameramoves.rs` | 763 | Named camera-move presets that *generate* scene-format camera key sets. Authoring only; **no production caller**. |
| `ab1.rs` | 1,683 | The §28 A/B reproduction scene (Onoda S01 at t = 1.50 s) rendered through the full production chain; also supplies shared staging used by `film`. |
| `battery.rs` | 153 | `--scene stroke-battery`: the M4 painter stroke-battery cards, pure CPU, from the same `spec.json` the Chrome harness consumes. |

### Film, telemetry

| File | Lines | One line |
|---|---|---|
| `film.rs` | 3,897 | The production film driver: Onoda staging evaluator (`eval_frame`), `PaintCache`, `Renderer` (the ab1 chain parameterised per frame), `run_film` (PNG sequence, resume-by-existing-file) and `run_film_telemetry`. The only film renderer in the tree. |
| `telemetry.rs` | 2,086 | M6 QA telemetry: native `cutouts()` — overflow, screen boxes, continuity/composition/caption audits, film tables; pure CPU, no GPU device is created. |

### Memory (pre-V2 + V2)

| File | Lines | One line |
|---|---|---|
| `paintbudget.rs` | 422 | Paint-cache byte budget + hit/miss/eviction statistics (policy half; storage lives in `film.rs::PaintCache`). Wired via `film.rs` and referenced by `mempressure.rs`. See `MEMORY.md`. |
| `bufferpool.rs` | 655 | Size-classed reuse policy for readback staging buffers, generic over an opaque handle. **No production caller** (gpucore's ledger notes state this explicitly); policy + tests only. |
| `atlas.rs` | 710 | MaxRects atlas packing + UV remap math for character canvases. Pure packing half; **no production caller** — cards are still uploaded as individual textures. |
| `mempressure.rs` | 761 | **V2 (WP1.3, C1). Registered, not wired.** `MemoryProfile`/`PressureLevel`/`MemPressure`: profile resolution, paint-budget/queue-capacity/preload policy, pressure `level()`. See `MEMORY.md`. |
| `dirtyrect.rs` | 845 | **V2 (WP1.2). Registered, not wired.** Dirty-rectangle accumulation/merging, byte-diff producer, full-upload fallback at coverage > 0.60. Pure logic, no wgpu types. See `MEMORY.md`. |

### V2 Batch 1 additions (all registered, none wired — see README)

| File | Lines | One line |
|---|---|---|
| `scheduler.rs` | 562 | WP7 (C5): pure resume-aware frame `plan`, checkpoint file (one index per line), `batches`, cooperative `CancelFlag`, `SchedEvent`. Schedules only; `film.rs`'s loop stays the only renderer. |
| `streamq.rs` | 580 | WP4 queue half (C3): `FramePacket`, bounded `frame_queue` over `sync_channel` (blocking send = backpressure), `PacketPool`, lock-free `StreamStats`. |
| `ffstream.rs` | 655 | WP4 sink half (C3): `FfmpegSink` — spawns ffmpeg rawvideo-`rgba` stdin → libx264 to a `.part.mp4`; `finish()` checks exit code + frame count. See `STREAMING.md`. |
| `config.rs` | 1,061 | WP13 (C8): one resolved `Config` with CLI > env > `talerender.toml` > default precedence. `main.rs` does **not** consult it yet and the new flags are not in `HELP`. See `CONFIG.md`, `CLI.md`. |
| `projectfile.rs` | 681 | WP5 (C4): loader/validator for the `talerender-project/1` interchange envelope (format gate, meta, asset manifest, section counts, `validate_assets`). Section entries are counted, not deep-validated. |
| `assetstream.rs` | 597 | WP8: asset manifest parsing, path+size deduplication, residency *metadata*, missing-asset diagnostics. Loads/uploads/frees nothing. |

### Shaders

15 WGSL files live beside the Rust (`shader_*.wgsl`, incl.
`shader_pbr.wgsl` 490 lines and `shader_bokeh.wgsl` 162).
`tools/ci/check_shaders.py` enforces WGSL↔Rust agreement; it was
updated during the wgpu 30 migration (see `MIGRATION.md`).

### Dependencies

`Cargo.toml` (version `1.0.1`) has exactly three direct
dependencies, exact-pinned: `wgpu = "30.0.2"`,
`pollster = "1.0.1"`, `image = "0.25.10"`. Zero-new-deps is a
standing law (V2-CONTRACTS §1); bounded queues are
`std::sync::mpsc::sync_channel`, JSON is `scene::parse_json`.

## Data flow — script to frames to finish

There is no JavaScript evaluation in the native path. The flow
that exists today:

1. **Authoring data (outside the binary).** The Onoda film's
   staging is ported formula-for-formula into `film.rs`; the data
   it reads (line timings, word anchors, caption chunks) is
   generated into `film/onoda.json` by `film/gen_film_data.py`,
   and 2D plates (props, caption strips, calendar cards) are
   rendered by `film/gen_assets.py` from the project's fonts.
   Text enters as data; the renderer typesets nothing.
   Scene-format projects instead arrive as JSON files consumed
   by `scenefile.rs` (format documented in `docs/scene-format.md`).
   The generalised `talerender-project/1` envelope
   (`projectfile.rs`, C4) exists as a loader but no CLI path
   feeds it yet; its TypeScript exporter
   (`tools/export/export_project.mjs`) landed during this
   wave as an untracked seat file (see `README.md`).
2. **Deterministic evaluation.** A frame is a pure function of
   its index/time: `film::eval_frame(t)` (or the scenefile
   evaluator) produces a `FrameSpec`; characters are painted on
   CPU by `toonfull::paint_toon_full` through `PaintCache`
   (exact-state-key cache; a re-paint is byte-identical).
3. **GPU render.** `gpucore` creates the Vulkan instance/adapter/
   device; the composer chain renders HDR (HalfFloat, MSAA-4× at
   full quality) → Bokeh → Bloom → Output → GRADE, with captions
   composited last; `gpucore::Readback` returns tight RGBA bytes
   (row padding stripped).
4. **Frames out (the only wired output mode).** `run_film` writes
   `frame_%05d.png` into `--out`, skipping files that already
   exist (resume); progress every 25 rendered frames; end-of-run
   lines include the `paint-cache:` stats and the `gpu-mem:` line.
   Single scenes write one PNG via `gpucore::save_png`.
5. **Finish (external, unchanged).** `film/finish.sh FRAMES_DIR
   MIX_WAV OUT_MP4` encodes the PNG sequence with ffmpeg
   (libx264, preset slow, CRF 18, yuv420p, BT.709 tags + x264 VUI
   params), muxes `mix.wav` as AAC 256k, writes `${OUT}.part.mp4`,
   and renames to the final MP4 only on success. See
   `STREAMING.md` for how the unwired streaming modules are
   designed to slot in before this step.

## How the V2 additions wire — or await wiring

| Piece | State at `712a6aa` |
|---|---|
| VRAM ledger + `gpu-mem:` line (Stage 1/WP2 core) | **Wired** (landed pre-V2, `f9429a8`): call sites in `gpucore`, `composer`, `main`, `film`, `scenefile`, `ab1`; enforced by `tools/ci/check_mem_registration.py`. |
| wgpu 30 upgrade | **Wired** — it is the tree's dependency set. See `MIGRATION.md`. |
| `mempressure` (C1), `config` (C8), `scheduler` (C5), `observability` (C6), `projectfile` (C4), `assetstream` (WP8), `dirtyrect` (WP1.2) | **Registered only.** Each file's delivery note specifies the exact lead wiring (mod line — done — plus call sites); the call sites are not in the tree. Behaviour of every existing path is therefore unchanged by their presence. |
| `streamq` + `ffstream` (C3) | **Registered only.** Designed to sit between step 3's readback and step 5's finishing; no render loop constructs a queue or a sink today. |
| Batch 2 seat files: `lifetimes` (WP1.4), `memreport` (C2), `streammodes` (C3), `outputmgr` (WP11), exporter (WP5 TS), `report.py` (WP6), WP9 validator, WP1 LRU spec | **Landing in this wave** — files exist (untracked) at final check but are **not registered in `main.rs`** and not lead-integrated; `memreport.rs` additionally waits on the lead-owned `gpucore::MemSnapshot`, which does not exist yet. See `README.md`'s delivery note for the per-file table. |
| `adapter` (C7), `charstate` (WP10) | **CONTRACT only** — files absent at final check; contracts in `V2-CONTRACTS.md` §3/§5. |
| Pre-V2 policy modules `atlas`, `bufferpool`, `rendergraph`, `cameramoves`, `anim` | In-tree and unit-tested, but still without production callers (their headers say so); they are not V2 regressions — they landed as policy/design halves in Wave 2. |
