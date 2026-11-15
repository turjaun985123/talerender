# Bench software record — CI lavapipe (V2 WP6 tooling proof)

> ⚠ **SOFTWARE RESULTS — NOT HARDWARE.** This record was
> produced on GitHub's CI lavapipe (llvmpipe, a CPU software
> Vulkan device). Its numbers must never be quoted as
> hardware performance (PERF.md honesty rule). It proves the
> WP6 tooling end-to-end: per-frame timing emission →
> bench_lab record → report.py comparison. **GPU rows stay
> PENDING-PC** until the same commands run on the user's
> PC (RADV) per PC-SESSION-W98.

- Source run: bench workflow `43696086097` at HEAD
  `81cf0f4` (2026-11-15). The run's conclusion was failure
  SOLELY at the two artifact-upload steps (GitHub artifact
  quota); every bench leg succeeded, and this record is
  transcribed from the job log, which the lab leg `cat`s
  deliberately. The record JSON itself was in the failed
  upload — its content is reproduced by the report table
  below plus bench_lab's own `.md` (same run).
- Leg: `ci-lavapipe-cap160` — film frames 0..100, default
  paint cache (cap 160), `TALERENDER_FRAME_TIMING=1`.

| label | adapter_class | frames | total_s | frame_ms mean | p50 | p95 | p99 | peak_rss_kb | binary sha256 (prefix) | git head |
|---|---|---|---|---|---|---|---|---|---|---|
| ci-lavapipe-cap160 | lavapipe-software | 100 | 104.85 | 1039.33 | 1087.97 | 1123.89 | 1133.97 | 1409432 | `800ec1d77062` | 81cf0f488872357ff74e9ed327f525322ac0ea28 |

- Adapter line (from the same run family, CI smoke):
  `adapter: llvmpipe (LLVM 15.0.7, 256 bits) | backend:
  Vulkan | device type: Cpu | driver: llvmpipe Mesa
  23.2.1-1ubuntu3.1~22.04.4 (LLVM 15.0.7) (software)`.
- Cache stats for the configuration live in the leg log
  (the `paint-cache:` completion line); the record's
  per-frame sha256 map is the determinism provenance —
  the same head's probe run verified the standard ranges
  byte-identical to the d101770 baseline (40/40).
- Reproduce on the PC: `python3 tools/perf/lab/bench_lab.py
  run --binary <talerender> --out bench-out/lab --label
  pc-radv-cap160 --frames 0..100 --film film` then
  `python3 tools/perf/lab/report.py bench-out/lab/*.json`
  — the report will carry the `hardware-or-other` label
  verbatim and drop the PENDING-PC line only in favour of
  "unverified hardware label" (validation wave owns the
  verification itself).
