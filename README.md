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

| Milestone | Scope | State |
|---|---|---|
| M0 | Repository audit of Tall Tale Shorts (API / browser / shader inventory) | In progress |
| M1 | Native wgpu prototype: offscreen render → PNG, headless | **Started** — smoke frame |
| M2 | Core Three.js compatibility (Scene/Object3D/Mesh/Camera/…) | Not started |
| M3 | One real Tall Tale scene migrated + visual A/B vs Chrome | Not started |
| M4 | Animation & character parity | Not started |
| M5 | Cinematic quality (shadows, bloom, DOF, grading) | Not started |
| M6 | Native production capture + FFmpeg integration | Not started |
| M7 | Full project integration | Not started |
| M8 | Production hardening & release | Not started |

What exists today is the Milestone 1 smoke frame: wgpu initialises on the Vulkan
backend, renders one 1080×1920 frame to an offscreen texture and writes a PNG.
The shader draws a simple test scene (sky, sun, ground, character card). It is
scaffolding that proves the pipeline end to end — it is **not** a migrated
Tall Tale scene, and it does not claim visual parity with anything yet.

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
