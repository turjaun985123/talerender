# Runbook — CI routes (GitHub Actions)

Repo: `turjaun985123/talerender`. Three workflows:
`.github/workflows/ci.yml` (build + test + smoke render on every
push), `.github/workflows/render.yml` (manual dispatch renders
— the PBR-path workhorse) and `.github/workflows/tools-ci.yml`
(the Python toolchain gates — film validator, captions, golden,
bridge — on pushes/PRs touching the data trees; it builds no
Rust and renders nothing; contract in `tools/CI-WORKFLOW.md`).
Builds happen **only** here; neither
the dev VM nor the user's PC compiles Rust.

## Division of labour

| Where | What runs there |
|---|---|
| Dev VM | Code + film/scene data authoring; CPU-only modes (`--telemetry`, `--film-telemetry`); m1 / dof paths. Its llvmpipe JIT **segfaults compiling `shader_pbr.wgsl`** (reproduced on Mesa 23.2 / 24.0 / 25.2), so PBR renders never run there. |
| CI (this page) | Every build + test; PBR stills and film frame ranges on **lavapipe** (ubuntu-22.04, Mesa 23.2) — the environment the milestone A/B proofs were produced on. |
| User's PC | Production film renders on **RADV** (Vega 7) + finishing — see `USER-PC.md`. |

## ci.yml — on every push / PR / manual dispatch

- **Runner pinned to `ubuntu-22.04`** (glibc 2.35) deliberately:
  the binary artifact is downloaded and run on older machines;
  a binary built on the newest image demands a newer glibc and
  won't start there.
- **Concurrency:** runs are grouped per ref with
  `cancel-in-progress` — pushing a fix cancels the stale run.
  A *cancelled* run is not a failure; check the newest run.
- **Cache:** Swatinem/rust-cache with `cache-on-failure: true`
  (even a failed compile warms the next round), key segment
  `ubuntu2204`. `CARGO_INCREMENTAL=0`; mold linker via
  `RUSTFLAGS` in env so build/test/render share one fingerprint.
- **Steps:** checkout → Rust stable → cache → install mold +
  mesa-vulkan-drivers + vulkan-tools → `cargo build --release`
  → `cargo test --release` → M1 smoke render under lavapipe
  (`--width 1080 --height 1920 --out frame.png`; the lavapipe
  ICD path is globbed, `lvp_icd*.json`, never hardcoded).
- **Artifacts:** `talerender-frame` (the smoke `frame.png`) and
  `talerender-binary` (`target/release/talerender`) — the
  binary is how scene work iterates locally without burning a
  CI round per visual fix.

Download a binary artifact:

```bash
gh run list --repo turjaun985123/talerender --workflow CI --limit 5
gh run download <run-id> --repo turjaun985123/talerender \
    -n talerender-binary -D /tmp/tr
```

## render.yml — "Render (dispatch)", manual only

Dispatch it from the Actions tab, or:

```bash
gh workflow run render.yml --repo turjaun985123/talerender \
    -f mode=film -f from=0 -f to=120
gh run watch --repo turjaun985123/talerender
```

**Inputs** (names/defaults exactly as the workflow declares):

| Input | Type | Default | Meaning |
|---|---|---|---|
| `mode` | choice: `ab1`, `dof-chart`, `film`, `scene-file` | `ab1` | What to render |
| `from` | string | `0` | film: first frame (inclusive) |
| `to` | string | `0` | film: last frame (exclusive; 0 = end) |
| `scene` | string | `scenes/character-v2.scene.json` | scene-file: path to the scene JSON |

**What each mode runs** (after the same build steps as ci.yml):

```bash
ab1:        talerender --scene ab1 --out render-out/ab1.png
dof-chart:  talerender --scene dof-chart --out render-out/dof-chart.png
film:       talerender --film film --out render-out/frames --from <from> --to <to>
scene-file: talerender --scene-file <scene> --out render-out/scene.png
```

- `ab1` is the M5 A/B still — the regression oracle for the
  PBR path. `scene-file` is how format-v2 scene data (e.g. the
  wave's `scenes/film/*.scene.json`) gets rendered for proof.
- **Timeout:** 120 minutes per job — size film ranges
  accordingly; frames are clean PNGs, chunked by `from`/`to`.
- **Artifact:** everything under `render-out/` is uploaded as
  `render-<mode>-<from>-<to>-<run_id>` with **7-day retention**
  — download promptly:

```bash
gh run download <run-id> --repo turjaun985123/talerender \
    -n render-film-0-120-<run_id> -D ./render-out
```

## Reading verdicts

- Report a CI result from the run's own conclusion, the moment
  it's checked — a cancelled run (superseded push) is neither
  green nor red.
- A red `cargo test` or a failed smoke render blocks the
  binary artifact for that run; use the last green run's
  artifact instead and say so.
