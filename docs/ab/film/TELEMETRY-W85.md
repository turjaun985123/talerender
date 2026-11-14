# Film telemetry — independent run at the second paint-cache setting (W85)

Seat W85, 2026-11-14. Purpose: RENDER-READINESS's remaining item 1 asks for
the native film-telemetry gate at **both** paint-cache settings. The lead is
running its own telemetry series at the default setting; this is the
independent run at `TALERENDER_PAINT_CACHE=24` (the memory-tight setting for
this shared VM). The verdict is the stdout **line** — the binary exits 0 on
pass, fail, and error alike, and no exit code is reported here as a verdict.

## Verdict

```
FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)
```

The failure is entirely a **caption-width** failure: canvas overflow is
**none**, continuity is **clean**, and all 7 failures are caption chunks whose
rendered width exceeds the audit's 850 px floor (quoted verbatim below).
These are the same 7 over-floor chunks the wave's Chrome-side caption census
already counts (66 chunks, 7 over the 850 floor) — the native audit and the
census agree on the count; see the width note below for the one place their
numbers differ and why.

## Binary provenance

| | |
|---|---|
| Source | CI artifact `talerender-binary`, repo `turjaun985123/talerender` |
| CI run | **43575835793** (workflow `CI`, conclusion **success**) |
| Head | **5b0889d** (the newest green CI head at provisioning time; ≥ d8fbe83 as briefed) |
| Artifact size | 5,870,442 bytes (zip) |
| Binary | 19,548,480 bytes, sha256 `7e487d73af642cbbf41a6f84876eaacd0b33dc841c0a68ff9643bee70533fcbd` |
| Path used | `~/talerender-bin/talerender` (the stable path from CAPTURE-NATIVE.md) |

The bytes at the stable path were verified, not assumed: a fresh
`gh run download 43575835793 -n talerender-binary` into scratch produced a
byte-identical binary (same sha256) to the one already provisioned there.

**Currency probe:** invoked with `--film-telemetry` and no value, the binary
panics with `missing value for --film-telemetry` (src/main.rs:100) — the mode
is recognised. A stale build answers `unknown argument: --film-telemetry`
instead; this one does not.

## Run

```
TALERENDER_PAINT_CACHE=24 ~/talerender-bin/talerender \
    --film-telemetry /home/hatch/workspace/talerender/film
```

- Started 2026-11-14T08:37:45Z, ended 09:08:11Z — **wall time 30 min 26 s**
  on this shared VM (contended; no speed claim attaches to this number).
- 242 samples at 0.25 s spacing over the 60.46 s film (progress on stderr
  every 20 samples; final progress line `sample 240 (t=59.75)`).
- Observed RSS plateau ≈ 0.84 GB at cache 24 (sampled with `ps` during the
  run, not a formal peak measurement) — vs the 4.6 GB worst case the
  paint-cache override exists to avoid (commit e5860b4).
- Binary exit code 0 (recorded for completeness; it carries no verdict).
- **No `renderer:` line exists in this mode** — the telemetry audit is
  CPU-only (main.rs: "no GPU needed"); no `Renderer` is constructed, so
  there is no renderer to label. Nothing in this run speaks to any GPU path.
- Data provenance: the run loaded `film/onoda.json` + `film/assets/` at
  start. The tree's local HEAD at launch was 743d248 (wave Batch 8, docs/data
  only — no `src/` changes) over the binary's 5b0889d. Two lead commits
  landed while the run was in flight (0f6c033 sky dome / atmosphere —
  `src/` only; ede0e92 — docs only). Neither touches `film/` data, and the
  binary is a fixed CI artifact, so the run is unaffected; whether the
  verdict is re-confirmed at ede0e92 is the lead's call. The 7 failing items
  are caption-strip widths — pure functions of the `film/assets/cap_*.png`
  files and `onoda.json`, which neither commit modifies.

Full stdout preserved in scratch:
`~/workspace/.w85-scratch/telemetry-cache24.stdout.log` (45 lines),
stderr progress in `telemetry-cache24.stderr.log`, launch record in
`telemetry-cache24.status.txt`.

## Section results (from the run's stdout)

- `CANVAS OVERFLOW: none`
- `CONTINUITY: clean (5 named characters tracked)`
- Composition (advisory only, not verdict-bearing): 31 findings —
  caption-zone 0, safe-area 8, thirds 3, framing 2, edge-touch 18; the
  per-item EDGE TOUCH / SAFE AREA / FRAMING warnings and the three
  dead-centre advisories (S01, S10, S12) are in the preserved stdout.
- `CAPTION AUDIT: 66 chunks, 0/12 lines authored, min fit 0.92, max rendered width 1051 px`
- `CAPTIONS: estimate-grade (auto-chunked) on lines 1–12 — author caps in script.json for a master`

## The 7 caption failures (verbatim)

```
CAPTION WIDTH FAIL: line 2 chunk "was a Japanese soldier" renders 1039 px (> 850) at t=6.69s — the width floor wins (the per-word time floor is advisory); split the chunk
CAPTION WIDTH FAIL: line 5 chunk "Planes dropped leaflets" renders 1051 px (> 850) at t=21.61s — the width floor wins (the per-word time floor is advisory); split the chunk
CAPTION WIDTH FAIL: line 7 chunk "surviving on coconuts" renders 1002 px (> 850) at t=31.55s — the width floor wins (the per-word time floor is advisory); split the chunk
CAPTION WIDTH FAIL: line 8 chunk "named Norio Suzuki" renders 892 px (> 850) at t=37.13s — the width floor wins (the per-word time floor is advisory); split the chunk
CAPTION WIDTH FAIL: line 9 chunk "commander ordered it." renders 1015 px (> 850) at t=44.01s — the width floor wins (the per-word time floor is advisory); split the chunk
CAPTION WIDTH FAIL: line 10 chunk "his old commander," renders 871 px (> 850) at t=46.39s — the width floor wins (the per-word time floor is advisory); split the chunk
CAPTION WIDTH FAIL: line 11 chunk "and Onoda surrendered" renders 1050 px (> 850) at t=52.07s — the width floor wins (the per-word time floor is advisory); split the chunk
```

## Independent width reproduction

The audit's rendered width is the ink bbox (alpha > 32) of the strip PNG the
driver loads (`film/assets/cap_*.png`, via `load_assets`). Re-measured
independently with PIL under the same rule, the seven widest strips are:

| strip | ink width (px) | audit's chunk |
|---|---|---|
| cap_026.png | 1051 | line 5 "Planes dropped leaflets" |
| cap_059.png | 1050 | line 11 "and Onoda surrendered" |
| cap_014.png | 1039 | line 2 "was a Japanese soldier" |
| cap_051.png | 1015 | line 9 "commander ordered it." |
| cap_040.png | 1002 | line 7 "surviving on coconuts" |
| cap_044.png | 892 | line 8 "named Norio Suzuki" |
| cap_053.png | 871 | line 10 "his old commander," |

All 66 strips measured: max 1051 px, **exactly 7 over 850** — the audit's
numbers reproduce digit-for-digit from the data files.

**Width note vs the wave census:** the Batch-1 caption census reported
"max renderedW 960 px" for the same 66 chunks. That figure came from the
scenes-side caption generator measuring its own strip set; the telemetry
audit measures the driver-side `film/assets/` strips. The over-floor
**count (7) is identical** in both; the widths differ because the two strip
sets are different files. No contradiction in the verdict-relevant fact:
7 chunks exceed the 850 px floor under either measurement.

## Agreement with the lead's own runs

No telemetry verdict line has been committed anywhere in `docs/` as of this
writing (RESULTS.md still records the verdict for head 381ff47 as "not
run"). The lead's local run logs in `/tmp` (its scratch, uncommitted) —
`/tmp/telemetry4.log` (2026-11-14 08:08 UTC) and `/tmp/telemetry5.log`
(08:42 UTC) — both end with the **identical verdict line**:

```
FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)
```

and telemetry5's seven CAPTION WIDTH FAIL lines match this run's verbatim —
same chunks, widths, and times. The lead's logs do not record which binary
or cache setting they used (the log head is progress lines), so the
agreement is stated at the level the evidence supports: **two independent
series, one of them at cache 24, produce the identical verdict with
identical per-field detail.** If the lead's series ran at the default cap
(160), the two cache settings agree — which is the comparison
RENDER-READINESS item 1 exists to make — but that premise is the lead's to
confirm.

## What this does and does not settle

- The telemetry gate's painter side is clean at cache 24: zero overflow
  groups, zero continuity violations over 242 samples of the whole film.
- The gate as a whole reads **FAILED**, on caption widths alone. The fix
  the audit itself prescribes is data-side (split the 7 chunks, or author
  caps in script.json — the audit notes all 12 lines are estimate-grade
  auto-chunked). Whether the production gate accepts the film with these
  7 known over-floor chunks is a lead/user decision, not this seat's.
- This file is the first in-tree record of a real (non-stub) film-telemetry
  verdict line.
