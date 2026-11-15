<!--
DELIVERY NOTE — TaleRender V2, Batch 2, Seat 6 (WP9 production
integration, repo side)

Files created (this seat, NEW files only):
  tools/integration/validate_native_project.py
  docs/integration/V2-WP9-INTEGRATION.md  (this file)

Contract points implemented:
  - V2-CONTRACTS §5 Batch 2 seat 6 / §6: WP9 is repo-side only.
    No skill-tree file was modified; the skill tree was read
    read-only to verify the surface this integration works with.
    Skill-side changes appear ONLY as proposals (§8), awaiting
    the user's separate say-so.
  - Master prompt §13: native stays opt-in, Chrome stays the
    default, Chrome compatibility is not removed or altered.
    Nothing in this seat's deliverables changes a default.
  - Validator mirrors the adapter's own pre-validation
    (capture_native.mjs loadManifest + validateFilmData) plus
    the generic Tall Tale project shape, stdlib-only Python,
    read-only, exit 0 OK / 2 with the FULL problem list.

Wiring the lead must do: NONE for the validator (standalone CLI;
no mod line, no call site, no Cargo change). Optional, at the
lead's/user's discretion: index the tool and this guide from an
existing doc, or add a Tools-CI smoke of `--self-test`.

Tests included: `validate_native_project.py --self-test` builds
three fixture projects in a temp dir (valid / frame-numbering
gap / missing required file) and runs the CLI on each as a
subprocess. RUN BY THE SEAT 2026-11-15: SELF-TEST PASS (3/3) —
valid case exit 0, gap case exit 2 with the gap reported,
missing-file case exit 2 naming script.json. The validator was
also run against the one real bridged project on this machine
(~/faceless-shorts/the-soldier-who-didnt-know): RESULT OK,
exit 0, 1 flagged item (that project's tools/ predates the
adapter — see §8, proposal P1).

Known limitations: the adapter's fixed asset list is
Onoda-bridge-shaped (documented in §3.3 and in the validator);
the guide describes the surface as verified from the adopted
skill tree on 2026-11-15, and any later skill change can drift
it; no RADV/hardware claim is made anywhere in this guide —
every native number cited is CI lavapipe / VM llvmpipe era
evidence recorded in the skill docs.
-->

# WP9 — Tall Tale production integration (as built)

How a Tall Tale project flows through the native (TaleRender)
backend today, what configuration surface actually exists, what
the native path emits, how it fails, and the repo-side validator
that pre-flights a project before any of it runs.

**Standing law (master prompt §13 / §24.16, V2-CONTRACTS §6):
Chrome is the default renderer. The native backend is opt-in,
selected per invocation. This document does not change that,
and nothing in WP9 proposes changing it as a fait accompli —
§8 lists proposals only, each awaiting the user's separate
say-so.**

Sources verified for this guide (read 2026-11-15):
`docs/NATIVE-BACKEND.md`, `template/tools/CAPTURE-NATIVE.md`,
and `template/tools/capture_native.mjs` in the adopted skill
tree (`~/workspace/custom-skills/tall-tale-shorts/`, read-only);
`tools/selftest_native.sh` + `tools/SELFTEST-NATIVE.md` and the
hook block in `scripts/selftest.sh` in the same tree; the one
bridged production project,
`~/faceless-shorts/the-soldier-who-didnt-know/`.

---

## 1. The flow, end to end

```
Tall Tale skill (authoring, unchanged)
  script.json ──► voice/timing ──► data/lines.json, data/words.json
  src/shots.ts  (staging)        data/mix.wav  (voice+music+SFX master)
        │
        │  BRIDGE (once per film, upstream of capture — §2)
        ▼
<project>/native/   film.json manifest + scenes/film/*.scene.json
                    film/onoda.json + film/assets/ (plates, strips)
        │
        │  OPERATOR SELECTS THE BACKEND (per invocation, §3)
        ▼
RENDERER=native node tools/capture_native.mjs <mode>
        │  1. binary discovery (TALERENDER_BIN → project → repo → PATH)
        │  2. pre-validation of manifest + film data (fail closed,
        │     every problem named, before anything renders)
        │  3. GATE for frames/video: talerender --film-telemetry
        │     → stdout verdict line FILM TELEMETRY: CHECK OK/FAILED
        ▼
talerender binary (CI artifact, Rust + wgpu, no browser)
        │  renders frame ranges --film <film-dir> --from/--to
        ▼
<project>/renders/native-frames/frame_<nnnnn>.png   (clean PNG,
        │  integrity-checked cache; resume skips valid frames)
        ▼
FINISH (adapter, mirroring capture.mjs argument-for-argument)
  ffmpeg image2 sequence + data/mix.wav → x264 CRF 18 slow,
  BT.709 tags, AAC 256k, +faststart, <out>.part.mp4 → rename
        ▼
renders/<name>.mp4   (candidate master — see §5 on provenance)
```

The Chrome path (`node tools/capture.mjs …`) is the same shape
with a browser in place of the binary and JPEG-transported
frames in place of clean PNGs. It is untouched by the existence
of the native path: nothing routes a capture to the adapter
automatically.

Modes parity (verified from `CAPTURE-NATIVE.md`):

| Mode | Native | Notes |
|---|---|---|
| `check` | ✓ | Runs the film telemetry audit; prints `CHECK OK` / `CHECK FAILED`. The audit is CPU-only — no GPU needed. |
| `frames <dir> "<t,…>"` | ✓ | Gate first, then clean PNGs staged as `t<time>.png` (Chrome stages `t<time>.jpg`). Times snap to the 1/fps grid; a moved time is printed. |
| `video <out.mp4> [start end] [--draft]` | ✓ | Gate first, full/range render, then the finish in §6. `--draft`: veryfast CRF 23, AAC 192k. |
| `sheet <out.jpg> [step] [cols] [--guides]` | Partial | Tiles native frames at `capture.mjs`'s own settings (`-q:v 2` samples, `-q:v 3` sheet). No gate runs in this mode. The **golden-sheet procedure stays Chrome**; the native reference set is the native golden pack, never a sheet. |
| `cues`, `castsheet`, `preview` | ✗ refuse | Chrome-only by nature (they read/serve the project page). The adapter prints why plus the Chrome command, and exits non-zero — it never falls back silently (§5). |

## 2. The bridge — the prerequisite that is not capture

The native renderer does not read `src/shots.ts`. A project is
captured natively only after its staging has been transcribed
into film data, once, upstream of capture:

- **Manifest** — `native/film.json` (adapter also accepts
  `native/scenes/film/film.json`), format `talerender-film`
  v1: `fps`, `durationS`, `totalFrames`, `resolution`, and a
  `segments[]` table (`id`, `scene`, `startFrame`, `endFrame`,
  plus staging metadata). The proven manifest (the Onoda
  short) carries 14 segments tiling 1,811 frames at 30 fps,
  1080×1920, 60.357 s.
- **Segment scenes** — one scene-format v2 file per segment at
  the path the manifest names (`native/scenes/film/…`).
- **Binary film data** — `native/film/onoda.json` (the filename
  is the current binary's, whatever the film): `duration`,
  `fps`, `frames`, timed `lines[]`, word `anchors{}`, and
  `captions[]` chunks each naming a `strip` PNG. Generated from
  the project's `data/lines.json` / `data/words.json` /
  `script.json` by the same rules as the engine's timing /
  captions code.
- **Assets** — `native/film/assets/`: `plates.json` (held-prop
  plate anchors) + the plate PNGs it keys, calendar plates,
  sign art, and every caption strip named in the data —
  rasterised once from the staging's own drawing recipes and
  the project's own fonts.

For an existing pre-native project, the repo-side bridge
packager (`tools/bridge/package_bridge.py`, stdlib Python)
assembles exactly this layout under `<project>/native/` from
film data the TaleRender repo already holds, self-verifies it
check-for-check against the adapter's pre-validation, and
writes a `MANIFEST-SHA` (sha256 of every packaged file). Its
honest scope, per `docs/NATIVE-BACKEND.md`: it packages an
existing bridge; it does not *create* film data for a new
project. Transcription remains per-film work. Audio needs no
bridge — the finish muxes `data/mix.wav` unchanged.

Prerequisite order (operator): **binary first** (current CI
artifact, §3.2), **bridge second** (packager self-verification
green), **`RENDERER=native` last**, and `check` before any
render mode.

## 3. Backend configuration surface — what exists today

### 3.1 Selection

| Fact | Behaviour (verified in `capture_native.mjs`) |
|---|---|
| Pipeline default | `tools/capture.mjs` (Chrome). Nothing auto-routes. |
| Selecting native | Invoke the adapter: `RENDERER=native node tools/capture_native.mjs <mode>`. |
| `RENDERER` unset, adapter invoked directly | Adapter runs natively — invoking it *is* the selection. |
| `RENDERER=<anything else>` (e.g. `chrome`) | Adapter **refuses, exit 2** — a render's provenance is never ambiguous. |
| Fallback to Chrome | Never silent, never automatic. It is an operator decision: re-run via `capture.mjs`. |

### 3.2 Environment variables the native path reads

Verified from `capture_native.mjs` (adapter) and
`tools/selftest_native.sh` (smoke test):

| Variable | Read by | Meaning |
|---|---|---|
| `RENDERER` | adapter, selftest hook | Backend selection, per §3.1. The selftest hook requires exactly `native`. |
| `TALERENDER_BIN` | adapter, selftest | Explicit binary path. Wins over discovery and is never second-guessed — a bad path is a hard error. |
| `NATIVE_DIR` | adapter | Film-data root (default `<project>/native`). |
| `NATIVE_FILM_DIR` | adapter | The binary's data dir (default `<NATIVE_DIR>/film`, else `<NATIVE_DIR>`); an explicit value without `onoda.json` is a hard error. |
| `NATIVE_FRAMES_DIR` | adapter | PNG frame cache (default `<project>/renders/native-frames`). |
| `FPS` | adapter | Must equal the manifest's `fps` (30 today); invalid values rejected exactly as in `capture.mjs`. |
| `TALERENDER_DOF_DEPTH_FORMAT` | binary (passed through) | Bokeh depth storage; default is lossless `Rgba8Unorm`; `halffloat` restores the exact BokehPass configuration on a proven driver. |
| `TALERENDER_SCENE` | selftest only | Explicit ab1 scene-fixture path. |
| `TALERENDER_REPO` | selftest only | Repo checkout for the fixture search. |
| `SELFTEST_NATIVE_TIMEOUT` | selftest only | Render-leg timeout, default 900 s. |

Explicitly **no-ops on the native path** (Chrome-path variables,
listed in `CAPTURE-NATIVE.md`): `CHROME`, `CHROME_FLAGS`,
`SOFTWARE_GL`, `QS`, `FRAME_TRANSPORT`. There is no native
equivalent of `QS` canvas scaling — the binary renders at the
manifest's resolution and frame rate only.

**Binary discovery order** (adapter, every run prints the path
and how it was found): `TALERENDER_BIN` → `<project>/native/
talerender` → `~/workspace/talerender/talerender` (repo
checkout) → `PATH`. The binary is the `talerender-binary`
artifact from a green CI run of `turjaun985123/talerender`;
there is no local build. Era pairing matters: binary and
bridge data must come from compatible repo states, and the
repo-checkout fallback silently selected a stale build in
every local integration-wave run — provision explicitly via
`TALERENDER_BIN` and verify currency (a stale binary answers
the gate call with `unknown argument: --film-telemetry`).

**Also required at runtime:** a Vulkan driver for the render
modes (RADV on the user's PC is the intended production
driver; lavapipe/llvmpipe proves correctness and supports no
speed claim) and `ffmpeg` on `PATH` for finishing and sheet
assembly, as on the Chrome path.

### 3.3 What the adapter validates before anything runs

`loadManifest`: format/version, `fps` == `FPS`,
resolution present, segments tile `[0, totalFrames)` exactly,
`totalFrames == ceil(durationS × fps)`, every segment's scene
file exists. `validateFilmData`: `onoda.json` duration /
lines / anchors / captions present, manifest↔data duration
agreement (same frame count), `assets/plates.json`, the five
hold plates, the four calendar plates + dark plate + sign,
and every caption strip named in the data. Problems from all
stages are accumulated and reported together; a manifest fault
no longer masks a film-data fault. The calendar/sign names are
the adapter's current hardcoded list — Onoda-bridge-shaped,
and the one place where today's "universal" surface is still
film-specific (see §8, P4).

### 3.4 The selftest hook (skill side, as adopted)

`scripts/selftest.sh` invokes `tools/selftest_native.sh` once,
immediately before its final `SELFTEST PASSED` verdict, guarded
by `RENDERER=native`. The smoke script additionally requires
`TALERENDER_BIN`; otherwise it prints one SKIP line and exits
0 — an operator who never selects native can never be failed
or slowed by it. Its legs: `--telemetry ab1` → CHECK OK /
exit 0; `ab1-variant` (deliberate overflow) → CHECK FAILED /
exit 1 (the gate proven in both directions); `ab1-file` over
the scene-file fixture → CHECK OK; and a `--scene-file` render
asserted to be a true PNG at exactly 1080×1920 (the clean-frame
contract, decision (B), checked byte-for-byte). Exit codes:
0 PASS/SKIP, 1 FAIL, 3 ENV-BLOCKED (an environment that can
produce no verdict — e.g. llvmpipe's known PBR-path failure on
driver-less VMs — is reported as blocked, never as a backend
failure). WP9 changes nothing here; the validator in §4 is a
repo-side complement, not a replacement.

## 4. The repo-side validator (this seat's tool)

`tools/integration/validate_native_project.py` — Python
stdlib only, read-only, no renderer invoked:

```bash
python3 tools/integration/validate_native_project.py <project-dir>
python3 tools/integration/validate_native_project.py <project-dir> \
    --frames <dir> [--expect-frames N]
python3 tools/integration/validate_native_project.py --self-test
```

Exit **0** = valid (flagged warnings may still print); exit
**2** = invalid, with the **full** problem list — every problem
found, never just the first. (`--self-test` exits 1 if a
fixture case misbehaves, i.e. the tool itself is broken.)

What it checks, and why:

- **Generic project shape** (the part the adapter does not
  look at): `script.json`, `data/lines.json`, `data/words.json`
  present and parsing; `src/shots.ts` present; script lines
  non-empty; timed lines monotonic with `end > start`; word
  timings ordered; **script↔timing line-count agreement**
  (drift between `script.json` and `data/lines.json` is a
  bridge-staleness symptom).
- **Audio, present-or-flagged:** a missing `data/mix.wav` is
  printed as a loud `FLAGGED: AUDIO:` item, not a failure —
  the adapter's video mode legitimately renders SILENT without
  one, so the validator's job is to make sure nobody ships
  that silence by accident. A present-but-tiny (< 44-byte)
  `mix.wav` *is* a problem.
- **Bridge, mirroring the adapter:** manifest format/version,
  fps (including an `FPS`-env mismatch, which the adapter
  would refuse), resolution, exact segment tiling,
  `totalFrames == ceil(durationS × fps)`, segment `frames`
  fields, every scene file present *and parsing*, manifest↔
  `onoda.json` frame-count agreement, `onoda.json`
  duration/lines/anchors/captions shape, segment→line
  references in range, `plates.json` parsing with every keyed
  plate present, the adapter's fixed calendar/sign assets,
  and every caption strip named in the data present on disk.
  Cross-check the adapter cannot make: `data/lines.json`
  duration vs manifest `durationS` (> 0.25 s apart = problem).
- **Adapter presence, flagged:** a project whose `tools/`
  predates adoption has no `capture_native.mjs` (the proven
  Onoda project is exactly this case) — flagged with the
  remedy, not failed; its Chrome path is unaffected.
- **Rendered output (`--frames`):** both PNG namings the
  native path produces — cache frames `frame_<nnnnn>.png` and
  staged frames `t<time>.png`. For cache frames: count vs
  expected (`--expect-frames`, else the manifest's
  `totalFrames`; a partial/ranged cache is validated by
  passing its range length), **numbering continuity with every
  gap index named**, no indices at/ beyond `totalFrames`,
  5-digit zero-padding flagged (ffmpeg's image2 sequence
  depends on it), and per-frame structural integrity identical
  to the adapter's `pngCheck` (signature, IHDR dimensions vs
  the manifest, closing IEND — the truncation catch). Staged
  `t<time>.png` files get the same integrity/dimension check.

Self-test (run by the seat, 2026-11-15): three fixture
projects built in a temp dir — valid, frame-numbering gap
(frames 0, 1, 3), missing `script.json` — each run through the
CLI as a subprocess: **SELF-TEST: PASS (3/3)** (exit 0 / exit 2
with the gap reported / exit 2 naming the file). Also run
against the real bridged project
(`~/faceless-shorts/the-soldier-who-didnt-know`): **RESULT:
OK, exit 0** — 30 fps, 1,811 frames, 14 segments, 12 lines,
73 caption strips — with the single expected flag for its
pre-adoption `tools/` (§8, P1).

## 5. Metadata the native path emits

There is no render-metadata sidecar file today; the metadata
surface is stdout lines, the bridge's own documents, and the
output discipline. Precisely:

- **Adapter banner (every run):** binary path + discovery
  route (`via TALERENDER_BIN` / project native dir / repo
  checkout / PATH), the manifest path, and the film line —
  title, duration, frame count, fps, resolution, film-data
  dir. This banner is the render's provenance record; combined
  with the selection rule (§3.1) it is why a native render
  cannot be mislabelled as a Chrome one.
- **Gate verdict:** the binary's `FILM TELEMETRY: CHECK OK` /
  `CHECK FAILED (…)` stdout line, echoed in full by the
  adapter, followed by the adapter's own `CHECK OK` /
  `CHECK FAILED` line in `check` mode.
- **Frame mapping:** in `frames` mode, every requested time
  that snapped to a different grid time is printed
  (`t=<t>s → frame <n> (… staged at <t'>s)`); `sheet` prints
  its sample count/step and its preview-grade disclaimer;
  `video` prints the output path, frame count, audio state
  (`with data/mix.wav` or `SILENT: no data/mix.wav`), encode
  wall time, and the line `renderer: native (TaleRender) —
  clean PNG frames, no JPEG generation (contract
  decision (B))`.
- **Bridge documents:** the manifest itself (title, fps,
  duration, totalFrames, resolution, per-segment set /
  atmosphere / film-look / caption-chunk ranges, and an
  `audio` block recording the mix file, its duration and its
  −14.0 LUFS loudness as *muxed unchanged*), `onoda.json`
  (per-line timings, anchors, per-chunk caption width/fit),
  and the packager's `MANIFEST-SHA` (sha256 of every packaged
  bridge file).
- **Frames:** clean PNGs, `frame_<nnnnn>.png`, at the
  manifest's dimensions — no JPEG generation anywhere in the
  chain (decision (B)); the native reference set is the
  TaleRender repo's native golden pack, not the Chrome golden
  sheet, and the two are never compared against each other.

## 6. Error handling — fail closed, out loud, no silent fallback

- **Gate failure stops capture.** `frames`/`video` run the
  telemetry audit first. The verdict *line* is authoritative
  under both binary generations (old binaries exit 0 on pass,
  fail, and error alike; new binaries agree with the line:
  0 OK / 1 FAILED / 3 could-not-run / 2 CLI error). The
  adapter passes only on an OK line with exit 0; a FAILED
  line, a line/exit disagreement, or **no verdict line at
  all** (stale binary, crash, bad data) fails the capture
  closed. There is no flag to render past a failed gate, and
  a partial frame directory is never presented as finished —
  it stays on disk for resume (frame(t) is pure; re-running
  skips integrity-valid frames and re-renders unusable ones,
  which are deleted on sight, never trusted by existence).
- **Adapter exit codes:** 0 success (for `check`, the gate
  printed CHECK OK); 1 capture failed (gate failed, mode
  refused, prerequisite/film-data item missing or invalid —
  named, binary/ffmpeg non-zero); 2 usage/selection error
  (unknown mode, bad sheet arguments, explicit non-native
  `RENDERER`).
- **Missing prerequisites refuse, named:** no binary, no
  Vulkan device (render modes), no film data — the adapter
  says which and stops. It never improvises film data from
  `shots.ts` at capture time.
- **Finishing discipline:** encode writes `<out>.part.mp4`
  and renames onto `<out>` only on a zero ffmpeg exit; a
  failed encode removes the `.part` and fails loudly. A
  killed encode therefore never leaves a truncated master
  under the final name — the same `.part` law as Chrome.

## 7. Finishing parity (why a master means the same thing)

The adapter's `video` finish mirrors `capture.mjs`'s video
encode argument-for-argument (not the repo's standalone
`film/finish.sh`, whose flags differ in detail): master
`libx264 -preset slow -crf 18 -threads 4`, BT.709
primaries/transfer/matrix tags *and* the `x264-params`
triple, `tv` range, AAC 256k @ 48 kHz; `--draft` veryfast
CRF 23 / AAC 192k; audio `-ss/-t` sliced from
`data/mix.wav` when present (silent lavfi otherwise, and the
summary says SILENT); `-shortest`, `+faststart`. The one
intended difference is the frame source: an image2 sequence
of clean PNGs instead of JPEGs piped from a page. One
ranged-render note: a `[start, end]` native render begins at
frame `round(start × fps)` — up to half a frame (≤ 17 ms at
30 fps) from the requested start, where Chrome evaluates at
the exact offset. Full-film renders are unaffected.

## 8. PROPOSALS — skill-side improvements (NOT made)

> Everything in this section is a **proposal only**. Per
> V2-CONTRACTS §6 the skill tree is out of WP9's write scope;
> none of these changes has been made, and each awaits the
> user's separate say-so before anyone touches the skill.

- **P1 — Retrofit the adapter into pre-adoption projects.**
  The one proven bridged project still has a pre-adoption
  `tools/` (no `capture_native.mjs` in the project folder —
  the validator flags exactly this). Proposal: a documented,
  one-time copy step (template `tools/capture_native.mjs` +
  `tools/CAPTURE-NATIVE.md` into the project) offered when a
  project gains a `native/` bridge, so the native path is
  invocable from the project it belongs to.
- **P2 — Offer this validator as an optional skill
  pre-flight.** Proposal: the skill doc *may* gain one line
  pointing operators at the repo-side validator before a
  first native capture of a newly bridged project. The
  validator stays repo-side and stdlib-only; the skill would
  reference, not vendor, it.
- **P3 — A machine-readable render-metadata sidecar.**
  Today provenance is stdout (§5). Proposal: the adapter
  *could* additionally write `<out>.render.json` (backend,
  binary provenance, manifest hash, frame range, encode
  settings, gate verdict) beside each `video` output. This
  changes adapter behaviour and output layout — user
  decision, not a WP9 act.
- **P4 — Generalise the bridge's fixed asset list.** The
  adapter's `validateFilmData` hardcodes the Onoda calendar/
  sign plates. Proposal (upstream of any second bridged
  film): drive the asset requirement from `plates.json` /
  manifest data instead, in the adapter and this validator
  together, so a non-Onoda bridge is not failed for lacking
  Onoda props.
- **P5 — A general project→bridge converter** remains the
  known future work already recorded in
  `docs/NATIVE-BACKEND.md`; WP9 repeats it here only to keep
  the integration picture complete. Until it exists, an
  unbridged project is a Chrome project, and both the
  adapter and the validator say so plainly.

## 9. Known limits (native path, as documented in the skill)

- One bridged film so far; every limit of that transcription
  is a limit of the path until a second film exercises more
  of the engine.
- Caption chunk widths are measured with PIL against the
  project's own font, not Skia's metrics (≤ 0.15 px difference
  on the bridged film, no audit verdict flipped; the limit is
  structural — only an over-wide chunk's fit can differ).
- Captions, the calendar overlay and film fades are
  CPU-composited after readback, as they are 2D canvas
  overlays in the staging too.
- No RADV evidence yet: every native number on record is CI
  lavapipe or VM llvmpipe. No performance claim exists, and
  this guide makes none.
- Native-vs-Chrome A/B numbers carry the known one-directional
  bias of the references' q0.93 JPEG generation (decision
  (B)); they are reported with that floor stated, never gated
  as pixel equality.
