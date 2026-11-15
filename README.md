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

## Target hardware

Ubuntu Linux first. Primary target: Ryzen 5 PRO 5650G with integrated
Radeon graphics and ~6.7 GiB system RAM — the engine is designed for modest
hardware: bounded memory, reused buffers, one frame at a time.

## Relationship to Tall Tale Shorts

The existing skill (SKILL.md / CHARTER.md / STYLE.md, the library catalog, the
audio + caption + FFmpeg finishing pipeline) is **not** modified by this
project. TaleRender replaces only the *rendering/capture* stage, behind a
compatibility contract established by the Milestone 0 audit.
