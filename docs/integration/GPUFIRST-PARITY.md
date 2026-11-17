# GPUFIRST-PARITY — Batch 2 parity table (GPU vs CPU, executor)

**Status: CI RUN #1 COMPLETE — 77/82 byte-identical, 6 differing px total, determinism 14/14 (see CI results below)
COORDINATOR PUSH.** The parity workflow cannot be dispatched with
`gh workflow run` (404: workflow not on the default branch), so it
carries a `push` trigger on `gpufirst-phase0` (mirroring
`gpufirst-perf.yml`). The coordinator must commit + push BOTH new/
changed files together — the workflow file AND the `src/gpupaint.rs`
dummy-buffer fix below — in one push; that push is run #1. Pushing
the workflow alone would run CI against the unfixed executor and
every replay would panic (see Harness bug). This document will be
completed from the downloaded CI artifact (`gpufirst-parity`):
per-case table rolled up per operation class, determinism results,
adapter line, and `gpu_wall_ms` summary. No CI numbers are claimed
yet; the only numbers below are a clearly-labelled LOCAL smoke.

## Workflow — `.github/workflows/gpufirst-parity.yml` (new, uncommitted)

- Triggers: `workflow_dispatch` + `push` to `gpufirst-phase0`,
  paths-limited to `src/gpupaint*.rs`, `src/gpupaint.wgsl`,
  `src/gpupaint_replay.wgsl`, `src/gputrace.rs`,
  `tools/gpupaint-compare.py`, `docs/ab/stroke-battery/spec.json`,
  and the workflow file itself.
- Setup mirrors `gpufirst-perf.yml`: checkout, stable toolchain,
  rust-cache (`ubuntu2204-gpufirst`), `mold` + `mesa-vulkan-drivers`
  + `vulkan-tools` (lavapipe), plus Pillow/numpy for the compare tool.
- Build: `cargo build --release --features gpupaint`.
- Record: `record-battery` → `parity/cpu-battery` (7 cards / lists),
  `record-states` → `parity/cpu-states` (75 states / lists); a
  prim-kind inventory (`list-inventory.json`) is computed from the
  recorded JSON for the per-operation-class roll-up; CPU `prims.png`
  mode is printed (must be RGBA for the alpha columns to be real).
- Replay: EVERY recorded list (7 + 75) replayed on lavapipe
  (`VK_ICD_FILENAMES` pinned to the lvp ICD); per-list logs in
  `parity/replay-logs/`; failures collected in
  `replay-failures.txt` and fail the Verdict step only after the
  artifact upload (`if: always()`).
- Determinism: named subset of 14 lists spanning battery + states
  classes (prims, toon, face-neutral/laugh/shock, mood-neutral,
  mood-laugh, film-tani-explain, legs-walk-p0,
  plate-holdr-parcel-label, plate-holdl-parcel,
  anchor-lying-lean82, pattern-stripes, fx-sweat-only), each
  replayed TWICE and byte-compared with `cmp` on the PNG pixel
  files; the subset is verified in-workflow against the recorded
  JSON to contain a `head_shade` case and a `composite_plate` case
  (auto-extended from the lists if a named case lacked one), and
  both the `cmp` result and the replay `sha256:` equality are
  recorded (`determinism.csv`) — the SHA-256 implementation was
  proven correct by the FIX-SHA256 seat (test-constant fix only),
  so hash equality is valid evidence, with `cmp` as the
  independent proof.
- Compare: canonical `tools/gpupaint-compare.py` — one battery run
  (46 classes + 7 whole cards) and single-card mode per state
  (75 whole-state metrics). An aggregate step writes
  `parity-summary.json` / `parity-summary.md`: every case (46
  battery classes + 75 states) with identical Y/N, differing px,
  max per-channel |Δ|, BOTH PSNR bases labelled (alpha-only =
  M4-native; RGBA), EDGE/FRINGE/INTERIOR split (boundary =
  fractional-α pixels of the CPU reference only; EDGE Chebyshev
  ≤1, FRINGE =2, INTERIOR >2); roll-ups by battery card and by
  STATES roll-up class (mood-features / film-looks / limbs /
  plates / anchor / patterns-and-looks / transforms) plus an
  overlapping per-prim-kind roll-up; worst-5 overall; adapter
  line; `gpu_wall_ms` min/median/max/mean/total.
- Artifact `gpufirst-parity` is TEXT ONLY (JSON/MD/CSV + logs),
  14-day retention. No rendered images are uploaded anywhere.

## Harness bug found by local smoke — FIX LEFT IN WORKING TREE (uncommitted)

`src/gpupaint.rs::replay_gpu` created ONE 4-byte `gpupaint dummy`
buffer and bound it, in a single dispatch, at binding 2 (pts,
`STORAGE_READ_ONLY`), binding 3 (mask, `STORAGE_READ_WRITE`), and
binding 4 (plate, `STORAGE_READ_ONLY`) whenever a prim had no pts,
no mask, or no plate. wgpu validation rejects that:
`Attempted to use Buffer with 'gpupaint dummy' label with
conflicting usages. Current usage BufferUses(STORAGE_READ_WRITE)
and new usage BufferUses(STORAGE_READ_ONLY). STORAGE_READ_WRITE
is an exclusive usage` — replay panicked on the very first battery
list (`prims`, at its first no-pts/no-mask prim, e.g. fill_ellipse).

Fix (working tree, `git diff src/gpupaint.rs`, +26/−5): three
separate 4-byte dummies — `dummy_pts`, `dummy_mask`, `dummy_plate`
— one per binding, each still registered in the VRAM ledger.
Verified locally after the fix: all 7 battery lists replay clean
(exit 0). The coordinator should commit this fix WITH the
workflow file in the triggering push (the push paths include
`src/gpupaint*.rs`, so one push triggers the run with the fix in).

## Local validation (this VM — NOT the CI verdict)

- `cargo check --features gpupaint` (PATH `~/.cargo/bin`,
  `CARGO_TARGET_DIR=~/workspace/talerender-target`): clean
  (268 pre-existing warnings, 0 errors), before and after the fix.
- Workflow YAML parses (`yaml.safe_load` OK).
- Local smoke, DEBUG build on this VM's lavapipe (llvmpipe Mesa
  25.2.8, LLVM 20.1.2 — CI ubuntu-22.04 Mesa will differ, and the
  CI build is release, so timings/results may differ in detail):
  adapter `llvmpipe (LLVM 20.1.2, 256 bits) | backend: Vulkan |
  device type: Cpu | driver: llvmpipe Mesa 25.2.8-0ubuntu0.24.04.4
  (LLVM 20.1.2) (software)`; battery replays 21–34 prims each,
  `gpu_wall_ms` 1390–2079 (debug-build numbers, meaningless for
  performance). Battery compare (local, debug): prims, face-neutral,
  face-determined, face-angry, face-laugh, face-shock all
  100.0000% identical / 0 differing px; `toon` 1 differing px
  (INTERIOR, max ΔR/G/B/A = 14/14/12/0, alpha PSNR inf, RGBA PSNR
  95.438 dB), localised by the compare tool to class
  `toon-head-outline` (99.99971%). Evidence:
  `~/workspace/talerender-evidence/gpufirst-phase0/parity-local-smoke/`
  (`battery-local-debug.{json,md}`, `replay-summary.txt`).
  A 1-px INTERIOR RGB-only difference on the in-situ head outline
  is consistent with a 1-ulp-class SDF/blend rounding flip
  (ellipse_sd division chain / FMA contraction are the candidate
  mechanisms from the GPUX risk list) — to be confirmed or refuted
  by the CI release run, not asserted from this smoke.
  States path also smoke-validated locally: `mood-neutral`
  (21 prims, incl. 1 `head_shade`) replayed GPU-vs-CPU
  100.0000% identical / 0 differing px, and replayed twice with
  `cmp` IDENTICAL and equal `sha256:`
  (`e8a0c941b4cf3a6db09df908e0f56be1f10bd75b997e4be459971e6f6817dc9c`
  both runs) — the workflow's single-card compare mode and its
  determinism method both work as written. `mood-laugh`'s list
  contains `masked_blend` + `head_shade` as expected.
  `composite_plate` path also validated locally (after the local
  debug `record-states` reached it — 47/75 states in 600 s before
  a smoke-imposed timeout, a debug-build speed artifact, not a
  harness failure): `plate-holdr-parcel-label`'s list contains
  exactly 1 `composite_plate` + 1 `head_shade` (22 prims total);
  its GPU replay is 100.0000% identical / 0 differing px vs CPU,
  and a second replay is `cmp` IDENTICAL with equal `sha256:`
  (`faa254310668b2505688e62d601e8754d761cbe648f3300142ba9792895f5662`
  both runs).

## CI results — run #1 (2026-11-17)

Run **43945477118**, triggered by commit `37a22f3` ("GPUFIRST: parity
workflow + executor dummy-buffer validation fix"). **Verdict:
SUCCESS** — the `parity` job passed in 6m39s; `replay-failures.txt`
empty, `parity-summary.json` written. Artifact `gpufirst-parity`
(text only, 14-day retention).

Adapter: `llvmpipe (LLVM 15.0.7, 256 bits) | backend: Vulkan |
device type: Cpu | driver: llvmpipe Mesa 23.2.1-1ubuntu3.1~22.04.4
(LLVM 15.0.7) (software)` — CI lavapipe, as designed; no RADV
numbers are claimed from this run.

GPU wall time over 82 cases: min 766 ms, median 1,050 ms,
max 1,452 ms, total 89.5 s.

### Headline

**77 of 82 cases byte-identical; 6 differing pixels in total
across the whole run.** Determinism subset: **14/14 identical**
(`cmp` identical + sha256 equal on every row). Per-class roll-ups
were clean except: battery `toon` 1/3, states `limbs` 7/8, states
`transforms` 4/6 — every miss is one of the five cases below.

### The five non-identical cases (the complete list)

| case | diff px | max Δ (RGBA) | RGBA PSNR | zone | reading |
|---|---:|---|---:|---|---|
| legs-walk-p3 | 2 | 0/0/0/16 | 95.64 dB | EDGE | alpha-only, on the silhouette edge |
| toon-head-outline | 1 | 14/14/12/0 | 82.23 dB | INTERIOR | the toon head-outline pixel |
| toon-card | 1 | 14/14/12/0 | 95.44 dB | INTERIOR | same pixel family as toon-head-outline |
| xf-campitch-s01 | 1 | 13/13/12/0 | 95.90 dB | INTERIOR | single pixel under camera pitch |
| xf-headtilt-pos6-think | 1 | 13/13/12/0 | 95.90 dB | INTERIOR | single pixel under head tilt |

The local-smoke toon head-outline diff **did reproduce at release
precision**, with identical numbers (1 px, max Δ 14/14/12/0, RGBA
PSNR 95.438 dB on toon-card) — it is a stable, real 1-px interior
difference, not a smoke artifact. Consistent with the known risk
list (fan_sd acos at cap tips / hypot→sqrt ulp flips moving one
coverage pixel); no case shows a structural or multi-pixel
divergence, and FRINGE is 0 everywhere.

Full per-case table, per-prim-kind roll-ups and determinism rows:
the run's `parity-summary.md` in the `gpufirst-parity` artifact.
