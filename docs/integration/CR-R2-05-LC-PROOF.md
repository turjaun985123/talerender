# R2-05 — CI acceptance proof for "The Lighthouse Cat's Last Shift" (LC)

Seat R2-05, Chrome-Replacement REMEDIATION wave, Batch 2. Acceptance
spec: `REMEDIATION-CONTRACTS.md` §5(b) (talerender `6da8ca7`). Nothing
here is committed; nothing in the repo was edited. All deliverables
are patch candidates in this staging dir.

## Deliverables

| File | What it is |
|---|---|
| `proof-lc.yml` | NEW workflow file candidate → lead lands it as `.github/workflows/proof-lc.yml` in `turjaun985123/talerender`. Implements the §5(b) legs on top of the model harness's scrub+shim. YAML-parse verified locally. |
| `assemble_lc_fixture.sh` | Fixture assembly: `pack` builds the repo fixture from R2-03's re-bridge + the LC source project; `check` validates a packed fixture against the §1/§2/§4 shape. Tested locally (see "Local verification"). |
| `proof-lc-vs-proof.diff` | `diff -u` of the candidate against the current `.github/workflows/proof.yml` @ HEAD `01a941a` (558 lines) — for the lead's review; the per-change rationale is §3 below. |
| `NOTE.md` | This file: dispatch, expectations, interface assumptions, risks. |

## 1. The model run — what I actually read (run 43750140530)

Verified first-hand via `gh run view` / `gh api .../jobs` / `--log`
on 2026-11-16 (no claim below comes from a record I didn't read):

- Run `43750140530`, workflow "Proof (Chrome-free native render)",
  `workflow_dispatch` @ headSha `148dcab`, conclusion **success**;
  single job `proof` in **55m45s** (created 19:02:34Z, updated
  19:58:22Z, 2026-11-15).
- Step outcomes (all success) and durations: Checkout/toolchain/
  cache/Node ≈ 30 s; Install mold+lavapipe+ffmpeg ≈ 60 s; Build
  ≈ 27 s (cache-warm); Scrub + Verify ≈ instant (the verify step
  prints `scrub verified: no real chrome/chromium reachable`);
  Assemble ≈ instant, with `mix.wav PCM md5 — expected/actual:
  1d1b69d007d096f92abe3b9f38bb9366` (match); **check 19:04:31→19:07:30
  (≈3 min)** — log carries adapter `CHECK OK` and
  `ADAPTER check exit: 0`; **frames →19:10:38 (≈3 min)**, log carries
  `FILM TELEMETRY: CHECK OK`; **sheet →19:18:43 (≈8 min)**;
  **castsheet ≈3 s**; **video 19:18:46→19:58:19 (≈39.5 min)** — log
  carries `FILM TELEMETRY: CHECK OK`, MP4 9,989,356 B,
  sha256 `322284ac93e3144c…`, ffprobe `codec_name=h264`,
  `codec_name=aac`, `duration=60.367000` (manifest durationS is
  60.357; audio master 60.457 — mux/container padding is real and
  is why the LC probe uses a tolerance, §3); final step prints
  **`CHROME-EXEC-LOG: EMPTY`**.
- The harness landed in `80a7dad` (proof.yml + `proof/` tree:
  Onoda fixture + byte-identical skill adapter bundle) and
  `148dcab` (scrub extended to `chromedriver` after run
  `43749554380`'s verify step caught it in `/usr/bin` — removal
  list, shim list and verify list all extended). The candidate
  copies the post-`148dcab` scrub verbatim.

## 2. Design: a new workflow, not a patch to proof.yml

`proof.yml` stays byte-untouched: it is the proven Onoda harness
and the standing regression reference, and §5(b) wants the Onoda
check *in the LC run* — which the candidate does itself (leg 0).
A separate file also gets a separate concurrency group
(`proof-lc-<ref>`), so an LC dispatch can never cancel an Onoda
proof in flight. Everything else is reuse: build fingerprint
(`CARGO_INCREMENTAL: 0`, mold `RUSTFLAGS`, `Swatinem/rust-cache`
key `ubuntu2204`), `ubuntu-22.04`, Node 22, lavapipe ICD
resolution, the scrub+shim, the verify step, the adapter-leg body
shape (`set +e` → `tee` log → `PIPESTATUS[0]` → print exit →
hashes only), and the `if: always()` verdict step are copied from
`proof.yml` @ `148dcab`.

Leg order in the candidate:

0. **Onoda regression check** (adapter `check` on
   `proof/fixture/project`) — runs first: a legacy-path regression
   fails the run in ≈3 min instead of after the LC video leg.
1. **LC preflight** (new, seconds): manifest `driver ==
   "scene-sequence"`, LC title, segment frame ranges tile
   `[0, totalFrames)`, every segment scene resolves, film data
   carries **no** `calendarYears`/`calendarDark`/`sign` keys
   (contracts §4 — a pre-decoupling bridge fails here, loudly,
   before any render).
2. **LC check** — plus the generic-telemetry assertions (§4).
3. **LC frames** — times derived from the LC manifest's own
   `durationS` (5/35/65/95 %), exactly as the model derives them
   from the Onoda manifest; asserts exactly 4 PNGs.
4. **LC sheet** — adapter defaults, as the model.
5. **LC video `--draft`** (full range) + MP4 probe assertions (§4).
6. **LC castsheet — OPTIONAL, `continue-on-error`, NON-GATING**
   (see §3).
7. **Verdict** — per-leg outcome table (job log + `$GITHUB_STEP_SUMMARY`)
   + `CHROME-EXEC-LOG: EMPTY` assertion, `if: always()`.

## 3. Every change vs proof.yml, and why

(Full text: `proof-lc-vs-proof.diff`.)

1. **New file / name / concurrency group** — §2 above.
2. **Inputs replace the single `fixture` input**: `lc_fixture`
   (default `proof/fixture-lc/project`), `onoda_fixture` (default
   `proof/fixture/project` — the existing fixture, unchanged),
   `lc_expected_duration_s` (default `39.05`),
   `lc_duration_tolerance_s` (default `0.60`), `upload_mp4`
   (default `false`). This is the parameterisation the lead wires
   at dispatch: if R2-03's fixture lands somewhere else, only the
   dispatch inputs change, not the file.
3. **Assemble step generalised to both projects** via a shell
   function; the FLAC→WAV decode + PCM-md5 verify is the model's
   block, applied per fixture *when the fixture carries the FLAC
   convention*; a plain `data/mix.wav` fixture is accepted as-is
   (the proof README's documented swap option). Neither convention
   present = fail.
4. **LC preflight step added** — §2 leg 1. No counterpart in the
   model (the Onoda fixture needs no driver assertion; it is the
   legacy path by design).
5. **Onoda check leg added** — required by §5(b)'s last sentence.
   Body mirrors the model's check step.
6. **LC check/frames/sheet/video legs** = the model's four leg
   bodies with project/output names changed (`lc-proof-draft.mp4`)
   and assertion greps added (§4). Frame times, sheet defaults
   and the `--draft` full-range video call are unchanged in shape.
7. **Castsheet: REMOVED from the gate; kept as an optional
   non-gating step.** §5(b) lists check/frames/sheet/video for LC
   — castsheet is not an acceptance leg (the model included it
   because the Onoda fixture's `look` catalog was already proven).
   LC castsheet also has a known unrelated defect (B3-01 F5:
   `castsheet_native.py` drops the `ears` look field). Gating the
   Chrome-replacement decision on it would be wrong; dropping it
   entirely would lose the signal. It runs, its exit code is
   printed and tabled as INFORMATIONAL, and `continue-on-error`
   keeps it out of the verdict.
8. **No artifact uploads by default; one optional upload added.**
   The model uploads nothing (repo artifact quota is exhausted;
   upload steps are the standing conclusion-red). The candidate
   keeps that discipline — hashes/logs/ffprobe in the job log —
   and adds a single opt-in `upload-artifact` of **only** the LC
   draft MP4 (`upload_mp4: true`), `continue-on-error` so a quota
   refusal cannot red a passed proof. Frames/sheets are never
   uploaded (also the user's no-rendered-photos-to-GitHub rule's
   spirit: evidence stays hashes + one MP4 max).
9. **Verdict step extended** with the per-leg outcome table
   (step `id:`s + `${{ steps.*.outcome }}`), still `if: always()`,
   still ending on the chrome-log assertion.
10. **Header comment rewritten** to state the §5(b) purpose, the
    deviations, and the fixture contract. `timeout-minutes: 240`
    kept (headroom, as the model).

## 4. Assertions (deliverable 3 — where each lives)

- **Generic-telemetry CHECK OK**: LC check leg greps the adapter
  output for `^CHECK OK` (adapter line) and reports the binary's
  `^FILM TELEMETRY: CHECK OK`; the frames and video legs re-grep
  `FILM TELEMETRY: CHECK OK` in their own logs (both re-run the
  telemetry gate — in the model run that line appears in the
  frames and video logs, verified above).
- **Generic, not legacy**: the telemetry report header is the
  discriminator — legacy prints a literal title
  (`FILM TELEMETRY — THE SOLDIER WHO DIDN'T KNOW THE WAR WAS
  OVER`, film.rs), generic prints the manifest title
  (`FILM TELEMETRY — {title} (native, 0.25 s sampling)`,
  filmtelemetry.rs). The LC check leg asserts the header/title
  names the Lighthouse Cat and asserts the Onoda title appears
  **nowhere** in the LC check output. (If the adapter's check
  output does not surface the raw header line, the title grep is
  skipped with a note and the binary-verdict + exit-code
  assertions still stand — see Risks.)
- **CHROME-EXEC-LOG emptiness**: verdict step, `if: always()`,
  prints the log and fails if non-empty — model's step, verbatim
  semantics.
- **MP4 existence/format**: video leg asserts the file exists,
  prints size + sha256 + the `.render.json` sidecar, and ffprobes
  it: requires an `h264` stream and an `aac` stream, and
  `|duration − 39.05| ≤ 0.60 s` (defaults; both are dispatch
  inputs). Tolerance rationale: model MP4 probed 60.367 s vs
  60.357 s manifest / 60.457 s audio; LC's audio master is
  39.149 s vs the 39.049 s film, so the muxed draft may sit
  anywhere in ≈39.05–39.16 s; 0.60 s covers that with margin
  while still catching a wrong-film/wrong-range render (the
  next-nearest plausible durations are a single segment ≈3–5 s
  or the Onoda 60 s).
- **Per-leg exit codes as step results**: every leg keeps the
  model's `ADAPTER … exit: N` print, carries a step `id`, and the
  verdict step tables `${{ steps.<id>.outcome }}` into the log
  and the GitHub step summary. A non-zero leg fails its step
  (except castsheet, by design), so the run conclusion itself is
  the gate.

## 5. Fixture assembly (deliverable 2)

`assemble_lc_fixture.sh pack --bridge <R2-03 output> --project
~/faceless-shorts/the-lighthouse-cats-last-shift --dest <staging>`
produces exactly the Onoda fixture's member set, in LC terms:

- `native/` copied verbatim from the bridge (manifest `film.json`,
  `film/` data + assets, `scenes/film/*.scene.json`, plus any
  MANIFEST-SHA/README the bridge carries);
- `script.json`, `fonts/` (3 font files + OFL texts), and
  `data/{cues,lines,words}.json` + `data/words.txt` from the
  source project;
- audio: `data/mix.wav` (7,516,686 B, 39.149 s) encoded losslessly
  to `data/mix.flac` (test pack: 1,561,438 B) + `data/mix.pcm-md5`
  written from the ffmpeg hash muxer, in the exact format the
  workflow's assemble step verifies;
- `tools/` deliberately omitted (workflow installs
  `proof/adapter/tools/`); `data/narration.wav` deliberately
  omitted (mix stem; no adapter mode reads it) — both mirror the
  Onoda fixture's documented calls.
- `pack` ends by running `check` on its own output, so a bad pack
  fails at assembly time, not in CI.

`check` enforces the §1/§2/§4 shape: driver field, LC title,
tiling, scene resolution, no invented calendar/sign keys, member
completeness, no `tools/`, FLAC/PCM round-trip. It also WARNs
(not fails) if every caption `fit` is exactly 1.0 — the
measured-fit tell (R1-10/R2-03).

**Local verification (this VM, CPU-only — no rendering):**
- `bash -n` clean; `proof-lc.yml` YAML-parses clean.
- `check` against the CURRENT (B3-01-era) LC bridge **fails
  correctly**, naming all four defects: driver `None`,
  `calendarYears`, `calendarDark`, `sign` present (+ `tools/`
  present in the live project). This is the pre-remediation
  bridge; the checker discriminating it is the point.
- `pack` against a synthetic re-bridge (driver injected,
  calendar/sign keys stripped, in /tmp) **passes end-to-end**:
  `FIXTURE CHECK OK`, `FLAC round-trip OK (PCM md5
  dd07095db34dbfd7db0fe5c3017cf749)`, fixture 4.0 MB total.
  The synthetic bridge was a test double only — the real pack
  must run on R2-03's actual output.

**Landing the fixture (lead):** run `pack` with `--dest
~/workspace/talerender/proof/fixture-lc/project`, then — the
proof README's trap — `git check-ignore
proof/fixture-lc/project/native/film/assets/cap_000.png` must say
NOT ignored: the repo-root `.gitignore` ignores `*.png` and
`proof/.gitignore` re-includes only the Onoda fixture's paths.
The lead must extend `proof/.gitignore` (or add
`proof/fixture-lc/.gitignore`) to re-include the LC fixture PNGs,
or the bridge assets will silently not commit. This is a landing
step I cannot do from staging (LAW: no repo edits).

## 6. Dispatch (exact commands for the lead)

Preconditions: R2-03's re-bridge landed; fixture packed + checked
(§5) and committed at `proof/fixture-lc/project`; `proof-lc.yml`
landed at `.github/workflows/proof-lc.yml`; `gh api user` =
`turjaun985123`; dispatch from the FINAL talerender HEAD (the
binary under proof is built from the dispatch ref's checkout).

```bash
# default fixture paths (fixture committed at the default location):
gh workflow run proof-lc.yml --repo turjaun985123/talerender --ref main

# if the fixture landed elsewhere, or the proof must run on the
# branch carrying the fixture before it merges:
gh workflow run proof-lc.yml --repo turjaun985123/talerender --ref <branch> \
  -f lc_fixture=proof/fixture-lc/project \
  -f onoda_fixture=proof/fixture/project

# watch + read the verdict at step level (never just the conclusion):
gh run watch --repo turjaun985123/talerender
gh run view <run-id> --repo turjaun985123/talerender
gh run view <run-id> --repo turjaun985123/talerender --log \
  | grep -E "ADAPTER|CHECK OK|FILM TELEMETRY|CHROME-EXEC-LOG|ASSERT"
```

**Expected duration**: model run 55m45s for Onoda (1,811 frames).
LC is 1,172 frames (0.65×) / 39.05 s (0.65×): scaling the model's
per-leg times gives check ≈3 min (CPU telemetry, similar),
frames ≈2–3 min, sheet ≈5 min (78 tiles vs 121), video ≈26 min,
plus ≈2.5 min setup/build (cache-warm) and the ≈3 min Onoda
check — **expect ≈40 min, plan for 30–50 min**; `timeout-minutes`
stays 240 as headroom. A cold Rust cache adds build time (the
model's build was 27 s cache-warm; ci.yml-class cold builds on
this repo run longer — if the cache is cold, add ~10–20 min).

## 7. R2-03 interface assumptions (made explicit, per charter)

R2-03 was still running when this seat designed (its staging dir
was empty at design time). The candidate is built against the
CONTRACT layout, with every assumption parameterised:

1. **Bridge layout** = contracts §1/§2 project layout:
   `native/film.json` (manifest) + `native/film/` (film data,
   file still named `onoda.json` — the adapter's `findFilmDir`
   looks for that literal name) + `native/scenes/film/`. The
   assembly script additionally tolerates the bridge arriving as
   a bare native dir, a project dir containing `native/`, or a
   `bridge-native/` dir (the B3-01 shape).
2. **Manifest carries `"driver": "scene-sequence"`** (§1.5:
   `build_bridge.py` emits it post-R1-03). Preflight fails the
   run without it — deliberately: a missing driver field means
   the binary takes the LEGACY path and the LC check would panic
   or mis-stage (the B3-01 failure), which must read as a bridge
   defect, not a renderer verdict.
3. **Film data omits `calendarYears`/`calendarDark`/`sign`**
   entirely (§4 decoupling) — preflight asserts absence. If
   R2-03 instead emits them as empty (`[]`/`""`), the preflight's
   key-presence test will false-fail: **the lead should relax
   that one test to "absent OR empty" if R2-03's validator-clean
   output carries empty keys.** This is the single most likely
   interface mismatch.
4. **Manifest numbers**: title "The Lighthouse Cat's Last
   Shift", `durationS` 39.049, `totalFrames` 1172, `fps` 30,
   11 segments s01–s11, 43 captions (B3-01 bridge values; the
   re-bridge may re-chunk captions idx 0/3/40 per the Batch-1
   integration note, changing chunk count but not duration or
   frame count). Duration assertions read `lc_expected_duration_s`
   as a dispatch input precisely so a manifest correction needs
   no workflow edit.
5. **Held plates declared**: Batch-1 integration left generic
   telemetry failing closed at s03 (`hold plate texture
   'lens-cloth' not declared`). Assumption: R2-03's re-bridge
   declares the plates (lens-cloth, oil-can assets already exist
   in the bridge) in `plates.json`. If it doesn't, the LC check
   leg fails with that named error — a correct gate outcome,
   attributable to the bridge, and the preflight cannot catch it
   from staging (plate declaration semantics live in the binary).
6. **Adapter bundle unchanged**: the proof still exercises
   `proof/adapter/tools/` (byte-identical to the shipped skill's
   `capture_native.mjs`, sha256 `5d7c3500…` on both sides,
   re-verified at design time). If the skill adapter changes
   before the flip, the lead re-stages the bundle and updates
   `proof/adapter/PROVENANCE.md` — the LC workflow needs no edit.
7. **Fixture committed in-repo**, like Onoda's (not fetched from
   another repo/branch at run time). The `lc_fixture` input +
   `--ref` dispatch cover the "lands on a branch first" case;
   a fixture that lives OUTSIDE the talerender repo is not
   supported by this candidate (would need a second checkout
   step — lead's call if R2-03 lands there).

## 8. Risks / what remains unverified until the lead's dispatch

- **Nothing here has rendered.** This VM has no Vulkan ICD; the
  candidate workflow, the preflight's Python, and the assertion
  greps are static-verified only (YAML parse, `bash -n`, the
  assembly script's two local test runs). The first real
  execution IS the acceptance run — by design (same as the
  model harness, whose first run caught the chromedriver gap).
- **Check-output surface**: the adapter's `check` prints its own
  reformatted report; whether the raw `FILM TELEMETRY — <title>`
  header line survives into the check log depends on
  check_mode's reformatting (it filters the header line out of
  its audit lines but the binary's stderr/stdout interleave via
  `tee` may still carry it). The candidate degrades gracefully
  (note, not fail) if the header is absent from the check log,
  and asserts the binary verdict line where the model run
  provably shows it (frames/video logs). If the lead wants the
  generic-path proof to be airtight regardless, add a direct
  binary leg — `"$TALERENDER_BIN" --film-telemetry
  "$LC_PROJECT/native/film" --tables | grep "FILM TELEMETRY —
  The Lighthouse Cat"` — as a 2-minute step before LC check;
  omitted here only to keep the proof adapter-shaped like the
  model and §5(b)'s wording ("running the SHIPPED SKILL's
  adapter"). Recommended if the first dispatch shows the header
  missing from the check log.
- **Caption re-chunking (idx 0/3/40)** in R2-03's re-bridge
  could in principle move a caption audit result; the gate
  handles it (CHECK OK or a named caption failure), but a
  failure there is a bridge/content defect, not a harness one.
- **Sheet leg cost variance**: sheet renders every 0.5 s tile
  through the film driver; on the generic path each tile is a
  scene load + render with per-call device setup (contracts §2
  names the persistent-renderer follow-up as not-yet-done). If
  per-frame overhead on the generic path is much worse than the
  legacy path's, sheet/video could exceed the scaled estimates
  — the 240 min timeout absorbs this, but the lead should not
  read a long video leg as a hang before ~90 min.
- **Artifact quota**: default path uploads nothing, so the
  quota cannot red this run. The optional MP4 upload is
  `continue-on-error` for the same reason.
- **`.gitignore` PNG trap** for the LC fixture (§5) — the one
  landing step most likely to silently corrupt the fixture;
  it is the lead's, and it is checkable in seconds.

## 9. Seat-law compliance

No commits; no repo files edited (the repo's `proof.yml` was
only read; the diff file was generated into staging); no results
fabricated — every claim about run 43750140530 is from `gh`
output read in this session (§1), and every LC number is from
the B3-01 bridge / source project files read in this session.
Local tests used a synthetic bridge in /tmp and are labelled as
such.
