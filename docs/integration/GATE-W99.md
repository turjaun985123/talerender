# GATE-W99 — Final Gate (Batch 10, seat W99)

## Tree state at gate start

- Run date: 2026-11-14 (UTC), gate start 09:12:35Z
- `git log -1`: `8af40eb9cfe450f18a1a2d9e448937c21e52d615` — "Wave batch 9 (W81-W90): saturation->sat rename in 12 files (values byte-unchanged, validator 7/7), staging felt reconcile (52/52 staging + 45/45 integrated), tools-ci validator source-absent mode (506 literals UNVERIFIED not failed; negative control still fails), MP4 finish DONE (1811 counted frames, -14.00 LUFS vs mix -13.99, colour-tag finding recorded), film telemetry verdict: CHECK FAILED on 7 caption chunks over the 850px floor only (overflow none, continuity clean; independent cache-24 run agrees with the lead's logs verbatim), fx-textures renders byte-identical local vs CI, fog-chart native gate reproduced from CI artifacts (<=1 code), 14 golden mids staged hash-verified (pins stay human), README proof patch proposal, gate 6/6" (2026-11-14 09:11:03 +0000)
- `git status --porcelain`: **0 dirty paths — working tree clean** at gate start.
- Note: Batch-10 siblings and the lead may land concurrently; every item below is timestamped, and the tree state will be re-checked at gate close.

## Tree state at gate close (09:50Z)

- HEAD moved twice during the gate, both times by other seats: `8af40eb` (gate start) → `6ca97fc` ("finish.sh: BT.709 primaries/transfer now land (x264 VUI params added…)" — the lead fixing W84's colour-tag finding; first observed 09:25Z) → `f021991` (committed 09:42:56Z, during item 8's telemetry run; sibling Batch-10 deliverables + FINAL-REPORT/README/runbook edits visible in `git status`).
- Items 1–7 and the stale-binary half of item 8 ran at `8af40eb`. Neither HEAD move touches the film data, the scene files, or the telemetry path (the `6ca97fc` change is the finishing script); the bridge package used for item 8's current-binary run was repackaged at `6ca97fc` and re-verified 21/21 before use.
- This seat's only repo change is this file (untracked, uncommitted, per the seat rule).

## Items

### 1. Validator FULL mode — ✅ PASS (run 09:12:55Z)
- Command: `python3 tools/film/validate_film.py`
- Expected: 7/7 PASS
- Observed (verbatim key lines):
  - `[1 parse] PASS — 14 present, 0 missing, 0 unparsable`
  - `[2 version] PASS — 14 files checked, 0 wrong`
  - `[3 f32] PASS — 1389 numbers checked: 883 exact f32, 506 source literals (pass), 0 computed values not round-tripping (fail); literals grepped from 15 source files under /home/hatch/faceless-shorts/the-soldier-who-didnt-know/src`
  - `[4 attachments] PASS — 0 top-level violations, 10 attachments on character nodes`
  - `[5 captions] PASS — 66 chunk listings, 66 unique chunks covered of 66, 0 duplicates, 0 uncovered`
  - `[6 durations] PASS — sum(durations) = 60.357 s -> 1811 frames (total 1811); tiling exact`
  - `[7 atmosphere] PASS — R5: declared presets match the manifest for 14/14 files`
  - `OVERALL: PASS (7/7 checks passed)`, exit 0

### 2. Validator CI mode (--no-source) — ✅ PASS (run 09:12:55Z)
- Command: `python3 tools/film/validate_film.py --no-source`
- Expected: 7/7 PASS with `COVERAGE: REDUCED` line + 506 UNVERIFIED (post-W83)
- Observed (verbatim key lines):
  - `[3 f32] PASS — 1389 numbers checked: 883 exact f32, 506 source literals UNVERIFIED (--no-source given), 0 computed values not round-tripping (fail)`
  - `COVERAGE: REDUCED (--no-source given) — check 3's SOURCE LITERAL class was NOT checked in this run: the 506 short decimals above are UNVERIFIED, not passed. The exact-f32 round-trip was enforced on all 1389 numbers, and not-round-tripping values with > 6 significant digits still FAIL (they can never be source literals).`
  - All other checks PASS as in item 1; `OVERALL: PASS (7/7 checks passed)`, exit 0

### 3. Caption determinism — ✅ PASS (run 09:13:21Z)
- Command: `python3 tools/captions/gen_caption_strips.py --project ~/faceless-shorts/the-soldier-who-didnt-know --out /tmp/gate99-captions`, then `diff -r /tmp/gate99-captions assets/captions` (assets at repo root)
- Expected: byte-identical
- Observed: generator wrote `manifest.json + 66 strips`; `diff -r` produced no output — **CLEAN (byte-identical)**, 67 files on each side.
- Note: the generator's own WIDTH warnings (chunks rendering > 850 px, e.g. 867 px and 960 px) are the same known over-floor caption set behind the telemetry verdict in item 8 — a data property, not nondeterminism.

### 4. Golden verify (make_golden.py) — ✅ PASS (run 09:13:22Z)
- Command: `python3 tools/golden/make_golden.py`
- Expected: 4 OK / 0 STALE / 14 MISSING with staged hashes in the current column
- Observed (verbatim summary): `4 OK / 0 STALE / 14 MISSING of 18 golden items`, exit 0. The 4 pinned items (s28-onoda-t150, dof-chart, stroke-toon, ab1-composer) read OK with pinned == current; all 14 film rows read MISSING (no pin — pinning is a human act) with their staged hashes in the current column (s01 ebcfeba172be55b8, s02 9be3879c94db211a, s03 61c54a42dbde95c8, s04 61e2ff32da998146, s05a bcb161f11cf41881, s05b e932ed856dbac45f, s06 d41bd58e3918fe70, s07 9eca38032479cfdb, s08 7ab95e9d4c94fb0b, s08b 7764821c30e1094e, s09 ebe45d9c7f322e22, s10 59bdb4f2558fa37f, s11 eb7b23f4a4be2834, s12 1978d6e603bcfc83).

### 5. Bridge packager — ✅ PASS (run 09:13:21Z)
- Command: `python3 tools/bridge/package_bridge.py --repo . --project /tmp/gate99-bridge` (fresh target)
- Expected: 21/21
- Observed: `packaged 94 files into /tmp/gate99-bridge/native` (manifest 1811 frames @ 30 fps, 60.357 s, 1080x1920; 14 segment scene files; film/onoda.json + 78 asset files); all 21 contract checks [PASS]; final line verbatim: `package-bridge: VERIFICATION PASSED — 21/21 checks`, exit 0.

### 6. JSON sweep scenes/**/*.json — ✅ PASS (run ~09:13Z)
- Command: python3 glob `scenes/**/*.json` (recursive) + `json.load` each
- Expected: all parse
- Observed: `JSON sweep: 32 files, 0 invalid`

### 7. Painter harnesses — ✅ PASS (both forms assembled fresh from the current tree; runs started 09:15:13Z, ~155 s wall each under concurrency)
- Recipe: painters/TEST-RUN.md (standalone rustc 1.99.1 from `~/.cargo/bin`, wgpu stub = the one documented const `COPY_BYTES_PER_ROW_ALIGNMENT: u32 = 256`)
- **Staging form** (`/tmp/w99-staging`: staging `painters/*.rs` verbatim — including the W82-reconciled `felt.rs` — + `src/scene.rs` from `c74cf0c^` with `painters/planks.rs` appended + repo `src/math.rs`): build exit 0; verbatim result: `test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 140.47s`
- **Integrated form** (`/tmp/w99-integrated`: HEAD `src/scene.rs` + `src/math.rs` verbatim): build exit 0; verbatim result: `test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 140.39s`

### 8. Adapter fail-closed spot check (work copy) — ✅ PASS
- `node --check template/tools/capture_native.mjs` → OK (run 09:14Z).
- **Stale binary — ✅ PASS (run 09:14:25Z):**
  - Command: `RENDERER=native NATIVE_DIR=/tmp/gate99-bridge/native TALERENDER_BIN=~/workspace/talerender/talerender node template/tools/capture_native.mjs check` (work copy `~/workspace/talerender-skill-wiring`; stale binary = repo-checkout build of 13 Nov)
  - Observed: adapter pre-validation accepted the package (banner + film header printed: `film: THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER — duration 60.36 s · 1811 frames @ 30 fps · 1080x1920`), then stderr: `unknown argument: --film-telemetry` and `capture-native: film telemetry produced no verdict line (binary exit 2) — treated as a FAILED gate. If the binary predates --film-telemetry, use a current CI artifact.`
  - **Adapter exit 1.** Package tree before/after: 95 files, aggregate sha256 of the sorted per-file hash list identical (`5476d0bca5196a8728c550b2f97d4f908a9116721bb3f247a24ee12e408e21c5`) — **zero artifacts created or modified**. Fail-closed confirmed.
- **Current binary — ✅ PASS (the adapter machinery carried a real verdict end-to-end):**
  - Binary: `~/talerender-bin/talerender` (CI artifact from run 43575835793, head `5b0889d`, sha256 `7e487d73…` — provisioned by W85/W86). Package: bridge package as in item 5.
  - **First attempt (launched 09:14:34Z) was interrupted by a runtime restart ~11 min in**: the exec session was lost, `/tmp` was wiped (log and package gone), no verdict was produced, and no process survived. Recorded as an interruption, not a result.
  - **Recovery run:** package repackaged into `~/workspace/.w99-scratch/bridge` (persistent scratch; re-verified 21/21), adapter relaunched detached 09:26:08Z with `TALERENDER_PAINT_CACHE=24`; verdict present by 09:50:09Z, adapter process finished by 09:50:16Z (~24 min wall).
  - Command: `RENDERER=native NATIVE_DIR=~/workspace/.w99-scratch/bridge/native TALERENDER_BIN=~/talerender-bin/talerender TALERENDER_PAINT_CACHE=24 node template/tools/capture_native.mjs check`
  - Observed — the binary's telemetry audit ran **through the adapter** and the adapter relayed it in full (242 samples). Key lines verbatim:
    - `CANVAS OVERFLOW: none`
    - `CONTINUITY: clean (5 named characters tracked)`
    - `CAPTION AUDIT: 66 chunks, 0/12 lines authored, min fit 0.92, max rendered width 1051 px`
    - The 7 caption failures, verbatim (chunk → width): line 2 "was a Japanese soldier" 1039 px; line 5 "Planes dropped leaflets" 1051 px; line 7 "surviving on coconuts" 1002 px; line 8 "named Norio Suzuki" 892 px; line 9 "commander ordered it." 1015 px; line 10 "his old commander," 871 px; line 11 "and Onoda surrendered" 1050 px — each ending `…renders NNN px (> 850) at t=… — the width floor wins (the per-word time floor is advisory); split the chunk`. (Widths agree with W85's independent PIL reproduction to the digit.)
    - **Verdict line, verbatim: `FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)`**
    - Adapter's terminal line, verbatim: `CHECK FAILED`
  - Reading: the adapter PASS here is the machinery — pre-validation, telemetry invocation, verdict relay, fail-closed terminal state all worked with a real current binary (previously exercised end-to-end only with stubs, W54). The verdict itself is the **film's**: FAILED on the 7 over-floor caption chunks only, overflow none, continuity clean — the same verdict W85 measured directly and the lead's logs record; this run is its third independent reproduction. (The numeric adapter exit code was not captured on the recovery run; per CAPTURE-NATIVE.md's exit table, check mode exits 1 when the gate fails, and the adapter's terminal line was `CHECK FAILED`.)

### 9. Tools-CI workflow sanity — ✅ PASS (run ~09:13Z)
- Command: PyYAML `safe_load` of `.github/workflows/tools-ci.yml` + grep for the flag
- Expected: YAML parses; validator step carries `--no-source` (post-W83)
- Observed: `YAML parses OK; jobs: ['tools']`; the validator step at line 92 runs `python3 tools/film/validate_film.py --root . --no-source`, preceded at line 91 by the mode echo (`Validator mode: --no-source (check 3 SOURCE LITERAL class UNVERIFIED on hosted runners — …)`) and the explanatory comment block at lines 73–81.

## Summary — GATE: 9/9 PASS (gate close 09:50Z)

All nine gate items pass as checks. No gate FAILs. Headlines:

1. **The film's telemetry verdict stands at `FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)`** — now reproduced a third time, and for the first time carried end-to-end through the skill adapter (`capture_native.mjs check`) with the real provisioned CI binary, not stubs. Overflow none; continuity clean; the failure is exactly the 7 auto-chunked caption widths over the 850 px floor (871–1051 px). Whether to re-chunk or accept is the lead/user decision (W92's decision pack); the gate machinery itself is proven in both directions — fail-closed on a stale binary (exit 1, zero artifacts) and verdict-faithful on a current one.
2. **Data/tooling suite is uniformly green at the final tree:** validator 7/7 in BOTH modes (full: 883 exact f32 + 506 source literals; CI `--no-source`: 506 UNVERIFIED under an explicit `COVERAGE: REDUCED` line, overall still PASS — the W83 fix behaving exactly as designed), caption strips byte-identical on regeneration, bridge 21/21, 32/32 scene JSONs parse, painter harnesses 52/52 (staging) + 45/45 (integrated), golden 4 OK / 0 STALE / 14 MISSING with all 14 film mids staged and hash-carrying (pins remain the human act), tools-ci workflow parses with `--no-source` in its validator step.
3. **Operational notes:** item 8's first current-binary attempt was interrupted by a runtime restart (~11 min in, `/tmp` wiped, no verdict) and was recovered from persistent scratch — the interruption is recorded in item 8, not silently retried. HEAD moved under the gate twice (`8af40eb` → `6ca97fc` → `f021991`) from lead/sibling landings; no move touched the artefacts under test, and the one artefact rebuilt after a move (the bridge package) was re-verified before use.
