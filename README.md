# TaleRender

A native Rust + [wgpu](https://wgpu.rs) rendering engine for **Tall Tale Shorts** —
a Three.js-compatible compiler and GPU renderer that replaces the
Chrome/Chromium capture pipeline with a genuinely native one.

**No Chrome. No Chromium. No headless browser. No CDP. No screenshots of a web page.**

TaleRender is a specialised **2D character animation + 3D environment** production
engine. It preserves the Tall Tale creative system — expressive camera-facing 2D
characters inside dimensional 3D environments with cinematic cameras — and never
replaces the 2D characters with modelled 3D characters.

## Status

All milestones are complete, and production integration is proven on a
complete short. (This table was stale at M1 for a long time; it now
reflects the record in `docs/` and the goal ledger.)

| Milestone | Scope | State |
|---|---|---|
| M0 | Repository audit of Tall Tale Shorts (API / browser / shader inventory) | **Complete** — 6 audit findings + ADR-001 + compatibility contract + risk register (see docs/) |
| M1 | Native wgpu prototype: offscreen render → PNG, headless | **Complete** — real 3D scene (perspective camera, lit cube, billboarded character card) rendered in CI |
| §28 | Real-scene A/B: Onoda S01 t=1.50 vs the Chrome original | **Complete** — PSNR 20.152 dB |
| M2 | Scene format v1 + JSON loader | **Complete** (2a7ff89) — ab1-as-data 95.17 dB vs hand-authored, dof-chart-as-data 84.49 dB, hand-authored modes pixel-identical |
| M3 | Bokeh depth-of-field parity gate | **PASS** (b0de44e) — blur width within 0.05 px at all 10 chart depths; full-frame PSNR 47.38 dB |
| M4 | Painter stroke battery (Skia parity, contract §2) | **PASS 46/46** (819ee16) — full character alpha PSNR 47.88 dB |
| M5 | Composer completion (shadows, PMREM lighting, bloom, GRADE) | **PASS** (79aff03) — ablation on Onoda S01: 21.36 → 21.42 → 23.850 dB |
| M6 | QA telemetry: native cutouts() vs Chrome (contract §3) | **PASS — exact agreement** (9a8cc79); JPEG q0.93 decision resolved: native frames stay clean (user decision (B), 801f85c) |
| Production integration | Scene format v2, film driver, native telemetry gate, FFmpeg finishing, skill capture backend | **Complete, with named remainders** — the full Onoda short (14 segments, 1,811 frames @ 1080×1920, 30 fps) rendered natively in CI; final-head A/B vs Chrome: **mean 23.02 dB** (60 pairs); telemetry gate at `FILM TELEMETRY: CHECK OK` since the
Wave-2 caption re-chunk (`5e90d7e`; the earlier waiver
ruling is superseded); finished MP4 proof delivered. Full record: [docs/integration/FINAL-REPORT.md](docs/integration/FINAL-REPORT.md), proof package in [docs/ab/film/](docs/ab/film/) |

Architecture record: [docs/audit/](docs/audit/) (M0 findings) · [docs/ADR-001-talerender-architecture.md](docs/ADR-001-talerender-architecture.md) · [docs/compatibility-contract.md](docs/compatibility-contract.md) · [docs/risk-register.md](docs/risk-register.md)

What exists today is the complete native pipeline: a JSON scene format
(v2) with a real character node, the full composer chain, a film driver
that renders a complete short under a telemetry gate, and FFmpeg
finishing — proven end to end on "THE SOLDIER WHO DIDN'T KNOW THE WAR
WAS OVER". Remaining, by design: GPU proof on the target PC (RADV —
every number so far is CI lavapipe), and the decision on adopting the
native backend into the shipped skill (a verified work copy exists;
Chrome remains the default). The golden-frame pins were struck in
Wave 2 (`916a109`, `166873c`).

## Honest rendering notes

- CI runners have no GPU. CI renders through **lavapipe** (Mesa's software
  Vulkan driver), and the binary prints the adapter it actually used, e.g.
  `llvmpipe`. Software frames prove correctness; performance numbers only
  count when measured on the target hardware (Ryzen 5 PRO 5650G, Radeon
  Vega 7 iGPU, RADV/Vulkan).
- No quality reduction is ever applied silently. Anything that lowers output
  quality is an explicit, reported configuration choice.

## Build & run

```bash
cargo build --release
./target/release/talerender --width 1080 --height 1920 --out frame.png
```

Run `./target/release/talerender --help` for the full flag
reference (all modes, defaults, and exit codes) and
`--version` for the version (`talerender 1.0.1`). Bad
invocations print `error: …` plus a help hint and exit 2 —
no panics on bad input.

Requirements: a Vulkan driver — hardware (RADV/ANV/…) or software
(`mesa-vulkan-drivers` / lavapipe on Ubuntu). No display server is needed.

## Resource governor

Every run is governed by resource limits (src/resource.rs) so the
renderer stays memory-safe on a modest shared-RAM machine and never
hangs the PC: roughly **RAM ≤ 60%** and **GPU ≤ 60%** by default,
with the caps user-configurable four ways, in precedence order
**CLI flag > env var > config file > built-in default**:

| Limit | Flag | Env var | Config key | Default |
|---|---|---|---|---|
| GPU cap % | `--max-gpu-pct` | `TALERENDER_MAX_GPU_PCT` | `max_gpu_pct` | 60 |
| RAM cap % | `--max-ram-pct` | `TALERENDER_MAX_RAM_PCT` | `max_ram_pct` | 60 |
| Thread cap | `--max-threads` | `TALERENDER_MAX_THREADS` | `max_threads` | half the logical cores (min 1) |

The config file is the same hand-parsed `key = value`
talerender.toml as the rest of the CLI: `--config <path>` when
given, else `./talerender.toml`, else
`~/.config/talerender/talerender.toml`. A missing file is never an
error; an invalid value in any layer is (`error: …`, exit 2 — never
a panic, never silently ignored). `TALERENDER_GOVERNOR=off`
disables enforcement; the limits are still resolved and printed.
A thread request above the core count is clamped to it, and the
clamp is printed.

The **effective** limits print on a greppable `governor:` line in
every run (stderr at startup, and beside the mem-report block at
render tails), naming each value, its source layer, the derived
RAM budget in bytes, and any clamp. Example:

```bash
./target/release/talerender --max-ram-pct 25 --max-threads 3 --film film --out frames
# governor: enabled=true gpu_cap_pct=60 gpu_source=default ram_cap_pct=25 ram_source=cli ram_budget_bytes=… …
```

What the caps do:

- **RAM (directly enforced).** The RAM budget is the cap % of
  total RAM read from `/proc/meminfo` (a documented 4 GiB
  fallback if detection fails — never a hardcoded machine).
  Shares of it bound the big consumers: the film paint cache
  gets 50% (byte budget; the `TALERENDER_PAINT_CACHE` entry cap
  still applies on top), the dyntex card-texture pool 10%
  (accounted + logged; live slots are never evicted mid-frame),
  the readback buffer pool 5%. Under pressure the paint cache
  shrinks by LRU eviction to a one-entry floor — eviction only
  ever costs a re-paint, never a pixel. On an integrated GPU
  this one budget covers GPU allocations too: same shared pool.
  Frame buffers on the film path are already streamed one frame
  at a time (~8 MB per 1080×1920 frame) and sit inside the same
  budget picture.
- **GPU (approximation — labelled as such everywhere).** A GPU
  utilisation % cannot be enforced from userspace: wgpu exposes
  no utilisation knob. The cap instead drives submission pacing
  (a minimum gap between queue submissions, scaled by the
  headroom under the cap) and a derived max-in-flight figure
  the serial submit sites structurally satisfy. This throttles
  the GPU's duty cycle only roughly — it is **not** a hard
  GPU-% guarantee, and no log line claims one.
- **Threads (directly enforced by consumers).** The governor
  resolves and reports the cap; parallel telemetry consumes
  `governor().thread_cap()`.

The governor changes speed and memory only — never pixels,
never verdicts. Governor on at default caps and
`TALERENDER_GOVERNOR=off` produce identical frame hashes by
construction (budgets only evict/recompute; pacing only delays).

## Target hardware

Ubuntu Linux first. Primary target: Ryzen 5 PRO 5650G with integrated
Radeon graphics and ~6.7 GiB system RAM — the engine is designed for modest
hardware: bounded memory, reused buffers, one frame at a time.

## Relationship to Tall Tale Shorts

The existing skill (SKILL.md / CHARTER.md / STYLE.md, the library catalog, the
audio + caption + FFmpeg finishing pipeline) is **not** modified by this
project. TaleRender replaces only the *rendering/capture* stage, behind a
compatibility contract established by the Milestone 0 audit.
