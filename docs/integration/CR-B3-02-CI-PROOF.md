# CR-B3-02 — CI render-proof harness (Chrome-replacement wave, Batch 3, Seat 02)

**Status: HARNESS BUILT + LOCALLY VALIDATED; CI RUN NOT YET EXECUTED —
staged for the lead to commit, push, and dispatch (exact commands in
§5). No CI run id exists yet, and none is claimed.**

**Scope adjustment (lead notice, 2026-11-16 00:37 +06): the NEW-film
leg of the original charter is BLOCKED — see §0. The harness is
unchanged and is proven against the Onoda data, which is the only
film the current binary's film driver can serve.**

---

## 0. BLOCKER — the binary's film driver is hardcoded to Onoda (new-film leg BLOCKED)

Lead-verified in the talerender source at HEAD, and independently
evidenced by seat B3-01's runs on its new proof film ("The Lighthouse
Cat's Last Shift", 39.05 s, adapter-valid, validator-clean bridge —
`~/workspace/chrome-replacement/batch3/B3-01/FRICTION.md` F1/F3, logs
in `batch3/B3-01/logs/`):

- `src/film.rs` staging indexes the film data's anchors **by Onoda's
  names** (`data.anchors["years0"]` at film.rs:1523,
  `["never2"]`/`["surrender2"]` at 1594-1595, telemetry's shot table
  from `["thought4"]`/`["decades6"]`/`["refused7"]` + hardcoded shot
  ids at 3993-3997), indexes `lines[]` at Onoda line indices, and the
  loader requires `calendarYears`/`calendarDark`/`sign` from **every**
  film.
- B3-01's bridged new film therefore panics the gate, verbatim from
  its `logs/check1.log` (same panic in its frames/sheet/video and
  direct-telemetry logs):

  ```
  thread 'main' (16975) panicked at src/film.rs:3726:55:
  no entry found for key
  check_mode: film telemetry produced no verdict line — treated as a FAILED gate
  CHECK FAILED   (adapter exit 3)
  ```

- Worst symptom (B3-01 `logs/sheet.log`): fed a non-Onoda bridge, the
  driver announced **the wrong film** — "THE SOLDIER WHO DIDN'T KNOW
  THE WAR WAS OVER — 1172 frames @30fps" (the new film's frame count,
  Onoda's staging) — before crashing on an Onoda anchor lookup.

**Consequence for this seat (per the lead's adjusted deliverable):**
the harness below remains the proving instrument, but it can only
prove frames/video for a film the driver serves — today, Onoda. The
fixture is therefore the retrofitted Onoda bridge (§1), not as a
convenience fallback but as the only servable film. The new-film leg
stays **BLOCKED until a generic scene-sequence film path exists** in
the binary. Per instructions, no workaround was attempted: no fake
anchors, no invented calendar data in any fixture.

Staging root (everything the lead needs, nothing committed by this
seat — per charter the lead commits):

```
~/workspace/chrome-replacement/batch3/B3-02/
  .github/workflows/proof.yml     -> repo .github/workflows/proof.yml
  proof/                          -> repo proof/ (126 files, ~7.4 MB)
  run-evidence/                   local validation logs (NOT for the repo)
```

Repo state at build time: `talerender` main `63cc816` ("CR-B2-04 +
CR-B2-08 seat notes…") — matches the seat charter. Skill state:
`~/workspace/custom-skills/tall-tale-shorts/` post-Batch-2 (native-first
template, adapter wired at Batch-1 integration L6).

---

## 1. Fixture decision — the final run will use the Onoda retrofit

Seat B3-01 (new proof project) was authoring in parallel; its staging
directory did not exist when this harness was built, so per the
charter's fallback the harness was built against the **retrofitted
Onoda bridge**. The lead's blocker notice (§0) then made that choice
definitive rather than provisional: B3-01's film now exists and is
bridge-valid, but the current binary cannot serve it, so Onoda is
the only fixture this harness can prove end-to-end today:

- Source: `~/workspace/chrome-replacement/batch2/B2-08/onoda-copy/`
  (CR-B2-08 P1 retrofit; re-bridged from the repo's current film data).
- Film: "THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER" — 60.357 s,
  1811 frames @ 30 fps, 1080×1920, 14 segments, 73 caption chunks.
- Copied verbatim into `proof/fixture/project/`: `native/` (manifest,
  14 segment scenes, `film/onoda.json`, 85 asset files), `script.json`,
  `fonts/`, `data/{cues,lines,words}.json`, `data/words.txt`.
  Key-file sha256, source = staged: `film.json` `18ad444c…`,
  `onoda.json` `1634013d…`, `script.json` `1572c1aa…`.
- Prior verdict on this exact data: B2-08's own adapter check recorded
  **CHECK OK, exit 0** (`batch2/B2-08/evidence/adapter-check.log`).

Two deliberate, lossless transformations (full detail in
`proof/README.md` in the staging tree):

1. `data/mix.wav` (12 MB) is stored as `data/mix.flac` (2.1 MB,
   lossless). The workflow decodes it back and asserts the decoded
   PCM md5 equals `data/mix.pcm-md5`
   (`1d1b69d007d096f92abe3b9f38bb9366`). Locally verified: **match**.
   `data/narration.wav` excluded (a mix stem; no adapter mode reads it).
2. The fixture carries **no `tools/`** — the workflow installs the
   adapter bundle, so the adapter under proof is the shipped skill's
   by construction, not the fixture project's older copy.

**Swap path for B3-01:** replace `proof/fixture/project/` contents.
The workflow derives every fixture-specific value from the fixture's
own `native/film.json` (frame times at 5/35/65/95 % of duration;
sheet = adapter defaults; castsheet = catalog `look`; video =
full-range) — no workflow edit needed.

## 2. Adapter bundle — provenance

`proof/adapter/tools/` = the shipped skill's adapter subset, copied
from `~/workspace/custom-skills/tall-tale-shorts/template/tools/`
(`capture_native.mjs`, `CAPTURE-NATIVE.md`, `castsheet_native.py`,
`cues_native.py`, `native_modes/` × 7). Verified **byte-identical**
to the skill tree by sha256 on all 11 files (full table +
re-verification snippet in `proof/adapter/PROVENANCE.md`; e.g.
`capture_native.mjs` `5d7c3500…`, `check_mode.mjs` `5d9d486c…`).
A bundle is necessary because the workflow runs in the talerender
repo and the skill repo is not yet synced with the Batch-2 template.

## 3. The workflow (`proof.yml`) — what it does

Manual dispatch only (`workflow_dispatch`, optional `fixture` input
defaulting to `proof/fixture/project`). One job, `ubuntu-22.04`
(repo house pin), `timeout-minutes: 240`, build fingerprint identical
to `ci.yml` (mold via `RUSTFLAGS`, `CARGO_INCREMENTAL=0`,
rust-cache key `ubuntu2204`). **No `upload-artifact` steps anywhere**
(quota) — all evidence is printed to the job log. Steps:

1. Checkout / Rust toolchain / rust-cache / Node 22 (adapter is
   Node-builtins-only; no npm install).
2. Install mold + `mesa-vulkan-drivers` (lavapipe; ICD resolved by
   glob, the `ci.yml` trap) + ffmpeg; export `VK_ICD_FILENAMES`.
3. `cargo build --release`; the binary under proof is built from the
   same checkout that carries the fixture (era pairing by
   construction); exported as `TALERENDER_BIN`.
4. **Chrome scrub + shim:** `apt-get remove` google-chrome/chromium
   packages, delete any `/usr/bin` browser binaries, blank `CHROME`
   and `PUPPETEER_EXECUTABLE_PATH`, then create `$RUNNER_TEMP/
   chrome-shim/` with executable wrappers for `google-chrome`,
   `google-chrome-stable`, `chromium`, `chromium-browser`, `chrome`,
   `headless_shell`, `chrome-headless-shell`. Each wrapper appends a
   timestamped `INVOKED:` line to `$CHROME_EXEC_LOG` (truncated empty
   at creation) and exits 127. The shim dir is prepended via
   `$GITHUB_PATH`.
5. **Verify the scrub:** every browser name must resolve to the shim
   or to nothing; any real binary still reachable fails the job.
6. **Assemble:** fixture + adapter bundle → `$RUNNER_TEMP/proof-project`
   (renders never touch the repo tree), FLAC→WAV restore + PCM-md5
   assertion, tree listing, `node --check` on the adapter.
7. **Adapter legs** (each prints its exit code; the step fails on
   non-zero): `check` (also greps the log for `^CHECK OK`) →
   `frames review/frames "<derived times>"` (sizes + sha256 printed)
   → `sheet review/all.jpg` (adapter defaults step 0.5 / cols 9;
   size + sha256) → `castsheet review/cast look` (per-item JPGs +
   `castsheet.jpg` size + sha256) → `video renders/proof-draft.mp4
   --draft` (full range; size + sha256, the `.render.json` provenance
   sidecar printed, and an **ffprobe summary**: codec_name,
   codec_type, width, height per stream + format duration/size).
8. **Verdict (`if: always()`):** prints
   `CHROME-EXEC-LOG: EMPTY` only if the log has zero bytes; otherwise
   prints its contents and fails the job.

Success criteria as they will appear in the job log: five
`ADAPTER <mode> exit: 0` lines, `CHECK OK: present in adapter
output`, sheet/castsheet file listings with sizes, the ffprobe
summary (expected shape: h264 video 1080×1920 + aac audio, duration
≈ 60.36 s), and `CHROME-EXEC-LOG: EMPTY`.

**Renderer honesty:** the runner's Vulkan is Mesa **lavapipe
(software)**. This proof is correctness-only; it supports no speed
claim. First RADV confirmation still rides the user's PC session,
as with every prior native gate on this wave.

### A trap found and disarmed: `.gitignore`

The repo-root `.gitignore` ignores `*.png` globally (exceptions only
for `film/assets/`, `scenes/fixtures/`, `tools/**/fixtures/`). The
fixture's 84 PNGs (caption strips, plates) are **functional inputs
the binary reads** — a naive `git add` would silently drop them and
the committed bridge would be incomplete. `proof/.gitignore`
therefore re-includes `!**/*.png` under `proof/` (safe: the workflow
renders into `RUNNER_TEMP`, never into `proof/`, so no rendered
evidence can sneak back). Verified in a scratch repo replicating the
root `.gitignore`: `git check-ignore` exit 1 (not ignored) for a
fixture PNG, exit 0 (still ignored) for a root `frame.png`, and
`git add -n proof` lists **all 126 files** including all 84 PNGs.
The lead's checklist in `proof/README.md` repeats the check.

## 4. Local validation performed on this VM (no Vulkan ICD here)

| Check | Result |
|---|---|
| Workflow YAML parses (PyYAML); 15 steps in charter order | PASS |
| All 11 `run:` scripts pass `bash -n` after YAML dedent | PASS |
| Shim mechanism, functional test: names resolve to shim; invocation exits 127 and appends a timestamped log line; untouched log stays empty | PASS (log in staging `run-evidence/shim-negative-control.log`) |
| Adapter bundle vs skill tree, sha256 × 11 | PASS — byte-identical |
| Fixture key files vs B2-08 source, sha256 × 3 | PASS — identical |
| FLAC→WAV restore, PCM md5 vs recorded value | PASS — `1d1b69d0…` match |
| `.gitignore` behaviour (scratch repo) | PASS — see §3 |
| Adapter `node --check` + `native_modes` syntax; Python tools compile | PASS |
| Adapter **check** leg, assembled exactly as the workflow assembles it, binary `~/talerender-bin/talerender` pinned via `TALERENDER_BIN` | **ATTEMPT 1 KILLED by an inter-seat incident — see below; direct-binary re-run of the same staged fixture: `FILM TELEMETRY: CHECK OK`, exit 0** |

The check-leg attempt: the adapter assembled, discovered the binary,
and loaded the manifest correctly (`duration 60.36 s · 1811 frames ·
1080x1920`), then the telemetry child printed progress to
`sample 20 (t=4.75)` and died **without a verdict line and without
stderr**, so `check_mode` reported (verbatim):

```
film-telemetry: sample 20 (t=4.75)
check_mode: film telemetry produced no verdict line — treated as a FAILED gate (see stderr above)
CHECK FAILED
ADAPTER_CHECK_EXIT:3
```

Exit 3 is the adapter's "telemetry could not run" code — **not** a
data verdict (a data verdict is CHECK OK / CHECK FAILED with exit
0/1, and this same data holds B2-08's CHECK OK on this VM).
**Cause, established after the fact by the lead's incident notice
(2026-11-16 00:37 +06): seat B3-01 killed processes by name
`talerender` at ~18:25 UTC and caught this run's telemetry child
(PID 13217) — no files were touched.** The death was an inter-seat
incident on a contended VM (load average 15.03 at the time, a second
telemetry run and a full `cargo build --release` concurrent), not
evidence about the fixture, the adapter, or the harness — and it is
precisely why this proof was chartered to ride CI rather than this
VM. A direct-binary telemetry re-run of the staged fixture
(`--film-telemetry <assembled-fixture>/native/film --tables`, same
pinned binary) was launched after the notice and **completed:
`FILM TELEMETRY: CHECK OK`, exit 0** (29 min wall / ~8.3 min CPU
under the same contention; full log in staging
`run-evidence/telemetry-direct-rerun.log`). So the staged fixture
copy itself — assembled exactly as the workflow assembles it —
passes the binary's full telemetry gate, corroborating B2-08's
adapter-level CHECK OK on the identical source data. (The binary's
raw `CAPTION AUDIT … 0/12 lines authored` line is expected in a
direct run; the adapter's `check_mode` corrects the authored count
from `script.json` to 12/12, as in B2-08's log. The verdict line is
identical either way.)

The render legs (frames / sheet / castsheet / video) were **not**
attempted locally: this VM has no Vulkan ICD at all, so they are
env-blocked by design (adapter fails closed naming it). They exist
only in the CI run.

## 5. Run instructions for the lead (this seat does not commit)

```bash
cd ~/workspace/talerender
cp ~/workspace/chrome-replacement/batch3/B3-02/.github/workflows/proof.yml .github/workflows/proof.yml
cp -r ~/workspace/chrome-replacement/batch3/B3-02/proof proof
git add .github/workflows/proof.yml proof
git status --short          # expect 126 files under proof/, INCLUDING fixture PNGs
git check-ignore proof/fixture/project/native/film/assets/cap_000.png \
  && echo "STILL IGNORED — do not commit" || echo "PNG not ignored — good"
git commit -m "CR-B3-02: CI render-proof harness (Chrome-free native render, lavapipe)"
git push
gh workflow run proof.yml --repo turjaun985123/talerender
gh run watch --repo turjaun985123/talerender
```

Dispatch could not be done from this seat: `gh workflow list` on
`turjaun985123/talerender` shows the 10 existing workflows and no
Proof workflow — GitHub requires the workflow file on the default
branch before `workflow_dispatch` exists, and this seat is barred
from committing. **Run id: none yet.** Once the lead dispatches,
read the verdict at step level (the artifact-upload quota issue does
not exist here — this workflow uploads nothing): the five
`ADAPTER … exit:` lines, `CHECK OK`, the ffprobe summary, and the
final `CHROME-EXEC-LOG: EMPTY` line are the whole verdict.

## 6. Known limits / expectations for the CI run

- **Duration:** full-film draft = 1811 lavapipe frames (~1 s/frame
  software record from the V2 bench) + four telemetry passes +
  sheet's 121 samples (frame cache shared with video). Expect tens
  of minutes; the 240 min timeout is headroom, not an estimate.
- **Disk:** the PNG frame cache for 1811 frames at 1080×1920 lives
  in `RUNNER_TEMP`; hosted runners have room, but if a future
  fixture grows, watch this first.
- **Castsheet scope:** kind `look` only (5 items, no held-plate
  dependency) — the charter's "one kind". The `held` kind remains
  plate-gated (G1) and is out of scope for this proof.
- If any leg fails in CI, the failing step's own log is the report:
  the adapter names its missing prerequisite or gate verdict, and
  the shim verdict step still runs (`if: always()`).
