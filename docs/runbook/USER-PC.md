# Runbook — the user's PC (RADV, production renders)

For: the user's Linux PC — **AMD Ryzen 5 PRO 5650G, Radeon Vega 7
iGPU, RADV Vulkan driver, Ubuntu**. This is where production film
renders happen: the same binary CI builds, with RADV replacing
CI's lavapipe. Everything below uses only flags the binary
actually has (checked against `src/main.rs`). Note: **there is
no `--help`** — an unknown argument prints `unknown argument: …`
and exits with code 2.

## 1. Get the binary

CI builds on `ubuntu-22.04` (glibc 2.35) on purpose, so the
artifact runs on this PC as-is. Download it from the latest
green CI run of `turjaun985123/talerender`:

```bash
gh run list --repo turjaun985123/talerender --workflow CI --limit 5
gh run download <run-id> --repo turjaun985123/talerender \
    -n talerender-binary -D ~/talerender-bin
chmod +x ~/talerender-bin/talerender
```

(`talerender-frame` is the other CI artifact — just the M1 smoke
frame, not needed here.) Work from a checkout of the repo: the
film data (`film/`), scene files (`scenes/`) and finishing script
(`film/finish.sh`) all live in the tree, and commands below are
run from the repo root.

## 2. Confirm the GPU is the one rendering

The binary prints its adapter before every render, e.g.:

```
renderer: AMD Radeon … | backend: Vulkan | device type: IntegratedGpu | driver: radv …
```

(scene-file mode prints the same line prefixed `adapter:`.)
**That line is the ground truth.** If it names llvmpipe /
Lavapipe / "Software", the loader picked the software driver —
the render would still work but on CPU. Check what Vulkan sees:

```bash
vulkaninfo --summary        # look for the Vega 7 / RADV device
ls /usr/share/vulkan/icd.d/ # radeon_icd*.json is the RADV ICD
```

Pin the RADV ICD if the wrong one wins:

```bash
export VK_ICD_FILENAMES=$(ls /usr/share/vulkan/icd.d/radeon_icd*.json | head -n 1)
# (newer Vulkan loaders also accept VK_DRIVER_FILES with the same value)
```

Re-run and confirm the startup line names RADV before starting
a long render.

## 3. Smoke test — one scene file

```bash
./talerender-bin/talerender --scene-file scenes/character-v2.scene.json --out /tmp/scene.png
```

Renders the format-v2 S01 staging (character node + light rig)
at the default 1080×1920 to a single clean PNG. `--width` /
`--height` override the size if ever needed.

## 4. The telemetry gate (no GPU needed)

```bash
./talerender-bin/talerender --film-telemetry film
```

Pure CPU: the M6 audit over the whole film at 0.25 s sampling.
The verdict is the printed line — expect:

```
FILM TELEMETRY: CHECK OK
```

A failure prints `FILM TELEMETRY: CHECK FAILED (…)` and itemises
the overflow groups / continuity violations / caption failures.
Judge this command by its printed line, not its exit code.

## 5. Render the film

```bash
./talerender-bin/talerender --film film --out frames/
```

- 1,811 frames at 1080×1920 @ 30 fps, written as
  `frames/frame_00000.png` … `frames/frame_01810.png`
  (frame *n* is t = n/30 s). Progress prints every 25 frames.
- **Chunking:** `--from N --to M` renders frames [N, M);
  `--to 0` (the default) means through the end.
- **Resume is automatic:** a frame whose file already exists is
  skipped, so an interrupted run loses nothing and chunk
  boundaries never change a frame (frame(t) is a pure function).
  To redo one frame, delete its PNG and re-run.
- Disk: budget roughly **2–6 GB** for the full PNG set.

## 6. Finish — encode + mux the project's mix

```bash
film/finish.sh frames ~/faceless-shorts/the-soldier-who-didnt-know/data/mix.wav onoda-native.mp4
```

`film/finish.sh FRAMES_DIR MIX_WAV OUT_MP4` applies the
pipeline's own finishing settings: x264 **slow CRF 18**,
yuv420p, BT.709 tags, the project's existing `data/mix.wav`
muxed unchanged as AAC 192k, faststart — written to
`OUT_MP4.part.mp4` first and renamed on success, so a failed
finish never leaves a plausible file. Verify:

```bash
ffprobe onoda-native.mp4   # 1080×1920, 30 fps, h264 + AAC, ≈60.46 s
```

Loudness should measure ≈ **−14.0 LUFS** — the audio is the
project's own master, muxed, not remixed.

## 7. Environment variables (the few that exist)

- `TALERENDER_DOF_DEPTH_FORMAT=halffloat` — restores three.js'
  HalfFloat bokeh-depth storage. **Leave it unset on RADV:**
  the default Rgba8Unorm packing is lossless and is the proven
  path; RADV fp16 rounding is unverified (carried from M3).
- `CAP_CHECK=strict` — makes the `--telemetry` caption audit
  fail instead of warn. Telemetry modes only; not needed for
  the film gate in §4.

## 8. Troubleshooting

- **Startup line says llvmpipe/Lavapipe** → wrong ICD; pin
  `VK_ICD_FILENAMES` to the radeon JSON (§2) and re-check.
- **`no wgpu adapter found — a Vulkan driver (hardware or
  lavapipe) is required`** → no usable Vulkan driver at all.
  Check `vulkaninfo --summary`; the RADV driver comes with
  `mesa-vulkan-drivers`.
- **`GLIBC_2.xx not found`** → that binary did not come from
  this repo's CI (which builds on ubuntu-22.04 for exactly this
  reason). Re-download the `talerender-binary` artifact (§1).
- **Killed mid-film / out of memory** → just re-run the same
  command; resume skips finished frames. Background: painted
  character canvases are ~29 MB each and the paint cache is
  FIFO-capped at 160 entries (worst ≈4.6 GB, but a single
  shot's live working set is far smaller; eviction costs time,
  never a pixel). On this ~6.7 GiB machine, rendering in
  `--from/--to` chunks keeps any one session bounded.
- **A frame looks corrupt after a hard kill** → a PNG being
  written when the process died can exist but be truncated,
  and resume skips by existence. Delete that frame's file and
  re-run.
- **`unknown argument: …`, exit 2** → the flag doesn't exist.
  The full set is: `--width --height --out --scene
  --scene-file --caption --post-stage --telemetry --captions
  --cutouts-out --film --film-telemetry --from --to`.
