# Determinism byte-compare pack

`determinism_check.sh` settles, with bytes, the film driver's
resume-purity claim — the "determinism byte-compare" item on the
integration README's `TBD-PROOF` list.

## The claim under test

`docs/integration/README.md` says of `--film`:

> Resume: frames already present in the output dir are skipped, so a
> chunked/interrupted run loses nothing; chunking never changes a
> frame (frame(t) is pure — proven by re-rendering a chunk boundary
> twice and byte-comparing).

…while its own Proof section still lists *determinism byte-compare*
as `TBD-PROOF` (REVIEW-W50 finding C4: the TBD is the honest side —
no byte-compare evidence existed in the tree). This pack is the
instrument that turns the parenthetical into evidence — or refutes
it. A second, related claim is tested in the same run: the paint
cache cap (`TALERENDER_PAINT_CACHE`, film.rs) is documented as
pixel-neutral eviction — a smaller cap costs time, never pixels
(PERF.md §4 reads differing results across caps as a determinism
defect; this tests the bytes directly).

## What one run does

Film frames **[100, 130)** at 1080×1920, rendered four times:

| run | invocation | into |
|---|---|---|
| A | `--from 100 --to 130` (default cache) | `A-single/` |
| B | `--from 100 --to 115`, then `--from 115 --to 130` into the **same** dir | `B-chunked/` |
| C | `--from 100 --to 130` with `TALERENDER_PAINT_CACHE=24` | `C-cache24/` |

Then every frame is sha256-compared twice — **A vs B** (chunk
boundary + resume) and **A vs C** (cache cap) — with a per-frame
`IDENTICAL` / `DIFFER` line and a final verdict line:

```
DETERMINISM: PASS            (exit 0)
DETERMINISM: FAIL (n differ) (exit 1; n counts DIFFER lines across both comparisons)
```

Anti-faking discipline, built into the script:

- The driver skips frames that already exist, so all three frame
  dirs must start **empty** — the script refuses (exit 2) if any
  contains files, and asserts each run's own completion line reports
  `0 skipped` and the expected rendered count. A reused dir can
  never "pass" by comparing a run against itself.
- The binary is **probed** before anything renders: pointed at an
  empty data dir, a current binary dies in `load_film_data`
  (`film data: …`); a stale one (e.g. the 13 Nov repo-root build)
  answers `unknown argument: --film`, exit 2, and the script refuses
  with that named. A crashed or incomplete render likewise aborts
  with no verdict (exit 2) — a broken run is not a FAIL, it is no
  evidence.
- The results record the binary's **path + sha256** and the
  `renderer:` line from run A, so the evidence names its renderer:
  a lavapipe PASS and a RADV PASS are separate facts.

Outputs land in the output root: `determinism-results.txt` (the
citable evidence), `logs/` (one log per run), and the three frame
dirs (kept, so any DIFFER can be inspected with `tools/ab/`).

## Running it

Prerequisite: a **current** binary (`--film` era). On this VM there
is none — the repo-root `./talerender` is the stale 13 Nov build and
the script will refuse it by name. Use a CI artifact or run where
the binary was built.

### On the user's PC (RADV — the production renderer)

```bash
cd ~/workspace/talerender
gh run download <ci-run-id> -n talerender-binary -D /tmp/tr-bin   # or any current build
tools/determinism/determinism_check.sh /tmp/tr-bin/talerender ~/det-run-onoda
cat ~/det-run-onoda/determinism-results.txt   # verdict line is the last line
```

The PC is a ~6.7 GiB machine: the renders themselves use the
driver's default cache in runs A/B (that is part of what is being
tested); if memory pressure is a concern on a busy machine, that is
a reason to watch the run, not to pre-shrink the cap — run C is the
tight-cap run. No timing is measured; PERF.md owns performance.

### In CI (lavapipe — the Render workflow's environment)

`render.yml` has no determinism mode; the script needs only a built
binary and the repo checkout, so add it as steps in any job based on
that workflow (same ubuntu-22.04 + lavapipe setup), after the Build
step:

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

Four short renders fit comfortably in the workflow's 120-minute
timeout (no timing claim — the class is "four ≤30-frame renders",
the same class as the film chunks CI already runs). The artifact's
`determinism-results.txt` is labelled lavapipe by its recorded
`renderer:` line; it does not substitute for the PC/RADV run.

## How the verdict discharges the TBD item

When a run prints `DETERMINISM: PASS`, the proof package cites:

1. the run's `determinism-results.txt` — specifically its header
   block (binary sha256 + `renderer:` line), the two
   `section: 30/30 identical (0 differ)` lines, and the final
   `DETERMINISM: PASS` line;
2. the environment it names (CI run id + artifact name, or "user's
   PC, RADV" + date).

With one PASS recorded per renderer of record (CI lavapipe **and**
PC RADV), "determinism byte-compare" comes off the TBD-PROOF list
and the README's resume-purity parenthetical becomes a claim with
evidence behind it. A FAIL instead names every differing frame —
that is a defect report against `frame(t)` purity (or, if only the
cache-cap section differs, against the cache's pixel-neutrality),
and the kept frame dirs + `tools/ab/measure_ab.py` are the next
diagnostic step. Until at least the first PASS exists, the honest
state remains the TBD.
