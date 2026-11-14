# Determinism formal run attempt — W96

**Outcome: (b) ENVIRONMENTAL FAILURE — no verdict.** The pack ran
against the real binary for the first time and was stopped by this
VM's known llvmpipe wall before the first frame: run A died with
SIGSEGV inside the film render, and the pack — by design — refused
to claim any verdict from a broken run. The determinism TBD item
remains **undischarged for every renderer**; this document records
the attempt, localises the failure, and specifies the two routes
that can discharge it (CI lavapipe, PC RADV).

Run: 2026-11-14, seat W96. Scratch kept at
`~/workspace/.w96-scratch/det-1/` (results file, per-run logs,
the three empty frame dirs) and `~/workspace/.w96-scratch/confirm1*`
(confirmatory single-frame run).

## What was run

```bash
tools/determinism/determinism_check.sh \
    ~/talerender-bin/talerender ~/workspace/.w96-scratch/det-1
```

- Binary: `/home/hatch/talerender-bin/talerender`, sha256
  `7e487d73af642cbbf41a6f84876eaacd0b33dc841c0a68ff9643bee70533fcbd`
  — the CI artifact `talerender-binary` from run 43575835793
  (head `5b0889d`), the same bytes W85/W86 used.
- Film data: the repo's `film/` (the script's default).
- The pack's **capability probe passed**: pointed at an empty data
  dir, the binary died in `load_film_data` as a current binary
  should, so the script proceeded to the renders. The binary under
  test is not stale and does know `--film`.

## The failure, verbatim

`determinism-results.txt` contains only the header block — the
script writes its verdict lines only after all four renders
complete:

```
determinism byte-compare — frames [100, 130) at 1080x1920
date (UTC): 2026-11-14T09:12:52Z
binary: /home/hatch/talerender-bin/talerender
binary sha256: 7e487d73af642cbbf41a6f84876eaacd0b33dc841c0a68ff9643bee70533fcbd
film data dir: /home/hatch/workspace/talerender/film
```

Run A's log (`logs/A-single.log`) in full — the adapter came up
and the film loaded; no frame line follows:

```
renderer: llvmpipe (LLVM 20.1.2, 256 bits) | backend: Vulkan | device type: Cpu | driver: llvmpipe Mesa 25.2.8-0ubuntu0.24.04.4 (LLVM 20.1.2)
film: THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER — 1811 frames @30fps, rendering 100..130 at 1080x1920 (clean PNG frames; decision B)
```

The script's stderr, verbatim:

```
tools/determinism/determinism_check.sh: line 141: 28762 Segmentation fault      ( cd "$OUTROOT" && "$BIN_ABS" --film "$FILM_DIR" --out "$dir" --from "$from" --to "$to" --width 1080 --height 1920 ) > "$log" 2>&1
  render log: renderer: llvmpipe (LLVM 20.1.2, 256 bits) | backend: Vulkan | device type: Cpu | driver: llvmpipe Mesa 25.2.8-0ubuntu0.24.04.4 (LLVM 20.1.2)
  render log: film: THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER — 1811 frames @30fps, rendering 100..130 at 1080x1920 (clean PNG frames; decision B)
REFUSAL: render run 'A-single' exited 139 (log: /home/hatch/workspace/.w96-scratch/det-1/logs/A-single.log). No determinism verdict from a broken run.
```

Localisation:

- **Which run:** A (the first) — B and C never started.
- **Where:** after adapter creation (the `renderer:` line printed)
  and after the film-data load (the `film:` header printed),
  before the first frame write — `A-single/` contains **0 files**.
  Exit **139 = 128 + 11 (SIGSEGV)**, within roughly the first
  ~20 s of the run. This is the stage at which the PBR render
  pipelines are compiled for the first frame.
- **Confirmatory run** (outside the pack, same binary, one frame):
  `talerender --film film --from 100 --to 101` → identical two log
  lines, then `Segmentation fault`, exit 139 after 39 s, 0 frames
  written. Reproducible, and not specific to the pack, the range,
  or the output dir.

## What this means — and what it does not

- This is the development VM's documented llvmpipe wall, not a
  determinism result. `render.yml`'s own header records it: *"The
  development VM's llvmpipe JIT segfaults compiling
  shader_pbr.wgsl on its CPU (reproduced on Mesa 23.2/24.0/25.2,
  all binaries back to M3), so PBR-path renders run here [CI]"*.
  This attempt adds Mesa **25.2.8** (this VM's current stack) and
  the current `5b0889d` binary to that list. The film lane is the
  PBR path, so on this VM it cannot produce even one frame;
  W86's black-frame v2+composer result is the same environment
  expressing itself on the loader lane.
- It says **nothing** about `frame(t)` purity, chunk/resume
  byte-identity, or the paint cache's pixel-neutrality — no frame
  bytes exist to compare. It is also not evidence against the
  binary: the identical bytes render the film in CI (the
  1,811-frame render at `381ff47` and every milestone A/B were
  produced on CI lavapipe, Mesa 23.2 / ubuntu-22.04 — a different
  llvmpipe build on a different CPU).
- The pack behaved exactly as designed: probe first, fresh dirs,
  and a hard refusal (exit 2) instead of a verdict when a render
  breaks. Stub-proven behaviour (W67/W78) held against the real
  failure mode.
- Pre-existing partial evidence stands as it was: in CI
  production, overlap frames 1200–1204 were measured
  byte-identical across invocations (W77) — supportive of
  frame(t) purity, but it is not the pack's verdict, does not
  exercise the same-dir resume path or the cache cap, and does
  not discharge the TBD.

**State after this attempt: the pack has still never produced a
real-binary verdict. "Determinism byte-compare" stays on the
TBD-PROOF list.** The discharge criteria (pack README) are
unchanged: cite a results file whose header names the binary
sha256 + `renderer:` line, both `section: 30/30 identical`
lines, and the final `DETERMINISM: PASS` line — one PASS per
renderer of record, CI lavapipe **and** PC RADV.

## Discharge route 1 — CI lavapipe (the full pack)

`render.yml` has no determinism mode, and the pack needs only a
built binary plus the checkout, so the route is the pack README's
own: add two steps to a job on `render.yml`'s environment
(ubuntu-22.04, lavapipe ICD pinned via `VK_ICD_FILENAMES`,
`cargo build --release`), after the Build step:

```yaml
      - name: Determinism byte-compare
        run: |
          tools/determinism/determinism_check.sh \
            ./target/release/talerender "$RUNNER_TEMP/det" \
            --film-dir film
      - name: Upload determinism results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: determinism-${{ github.run_id }}
          path: ${{ runner.temp }}/det
          retention-days: 7
```

This runs the pack itself — A/B/C renders, the same-dir resume,
the `TALERENDER_PAINT_CACHE=24` run, both sha256 sections — and
the uploaded `determinism-<run_id>` artifact's
`determinism-results.txt` is the citable lavapipe record. Four
≤30-frame renders are the same class as the film chunks CI
already runs inside the workflow's 120-minute timeout.

## Discharge route 2 — stock `render.yml` dispatches (partial)

With **no workflow change**, the existing dispatch covers the
cross-invocation half of the claim. Dispatch **Render
(dispatch)** four times (workflow `render.yml`, inputs
`mode`/`from`/`to`; artifact name pattern
`render-film-<from>-<to>-<run_id>`, contents under
`render-out/frames/`):

| dispatch | mode | from | to | gives |
|---|---|---|---|---|
| A | film | 100 | 130 | the pack's run-A range, invocation 1 |
| B1 | film | 100 | 115 | run-B first chunk |
| B2 | film | 115 | 130 | run-B second chunk |
| A2 | film | 100 | 130 | run-A range, invocation 2 |

Download all four artifacts, then sha256-compare
`frame_00100.png` … `frame_00129.png`: A vs (B1 ∪ B2) tests the
chunk boundary across invocations; A vs A2 tests straight
run-to-run repeatability. The lead's probe precedent is the same
pattern at the production boundary: a `[1195,1205)` dispatch
overlapping the 200-frame chunk seam at 1200 (which is where
W77's 1200–1204 byte-identity / 1195–1199 cache-cap divergence
was measured); the analogous overlapping probe at the pack's own
boundary is `from=100 to=130` plus `from=115 to=145`, comparing
the `[115,130)` overlap.

Two honest limits of this route: each dispatch renders into a
fresh `render-out`, so the pack's same-dir skip/resume mechanics
(run B resuming into a partially-filled dir) are **not**
exercised; and `render.yml` exposes no `TALERENDER_PAINT_CACHE`
input, so the cache-cap section (A vs C) cannot run through the
stock workflow at all. Route 2 can therefore support the
frame(t)-purity claim on CI lavapipe, but only route 1 (or the
PC run) discharges the pack item as written.

## Discharge route 3 — the user's PC (RADV, second renderer)

Per `docs/runbook/USER-PC.md` §1–§2, on the PC (Ryzen 5 PRO
5650G, Vega 7, RADV):

```bash
gh run list --repo turjaun985123/talerender --workflow CI --limit 5
gh run download <run-id> --repo turjaun985123/talerender \
    -n talerender-binary -D ~/talerender-bin
chmod +x ~/talerender-bin/talerender
cd ~/workspace/talerender   # checkout carrying film/
tools/determinism/determinism_check.sh \
    ~/talerender-bin/talerender ~/det-run-onoda
cat ~/det-run-onoda/determinism-results.txt
```

Before trusting it, confirm the run's `renderer:` line names
RADV (`device type: IntegratedGpu`, `driver: radv`) — if the
loader picked llvmpipe, pin the RADV ICD with
`VK_ICD_FILENAMES=$(ls /usr/share/vulkan/icd.d/radeon_icd*.json | head -n 1)`
and re-run into a fresh output root (the pack refuses non-fresh
dirs). Runs A/B use the driver's default cache by design — that
is part of what is tested; run C is the tight-cap run. A RADV
PASS is a separate fact from a lavapipe PASS; the TBD item
leaves the list only when both are on record.
