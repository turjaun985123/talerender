# capture_native.mjs — the native capture backend contract

> **STAGED — B2-07 (Chrome-replacement wave, Batch 2).** This file is
> the replacement for `template/tools/CAPTURE-NATIVE.md`, staged at
> `~/workspace/chrome-replacement/batch2/B2-07/template/tools/CAPTURE-NATIVE.md`
> for the lead to land with the Batch 3 default-flip. It describes the
> adapter **as built in Batch 1** (mode dispatch in
> `tools/native_modes/` wired into `capture_native.mjs` at integration
> L6, talerender `172052d`). Every behavioural claim below was read
> from the code named beside it. The shipped v6.0 copy of this
> document describes the pre-Batch-1 adapter (opt-in backend, `cues` /
> `castsheet` refusing as Chrome-only, `sheet` partial) and is stale
> in exactly those places; this rewrite supersedes it at the flip.

`tools/capture_native.mjs` is the skill's capture tool: the same
command words and output discipline the pipeline has always used,
rendered by **TaleRender** (native Rust + wgpu) — **no browser in
the loop**. Since the Chrome-replacement wave it is the default and
only production renderer: there is no second backend to select and
no fallback to fall back to. This document is the adapter's own
reference — inputs, outputs, exit codes, and the rules it refuses
to bend. The backend overview lives in `docs/NATIVE-BACKEND.md`
(skill tree).

## Invocation

```bash
node tools/capture_native.mjs check
node tools/capture_native.mjs video renders/x.mp4
RENDERER=native node tools/capture_native.mjs check   # also accepted
```

- Invoking the adapter **is** the selection. In code
  (`capture_native.mjs`, the `RENDERER_SEL` guard) it runs when
  `RENDERER` is **unset** or `native`, and refuses (exit 2) under
  any other explicit value, so a render's provenance is never
  ambiguous.
- The adapter **never falls back silently** — not on a failed
  gate, not on a refused mode, not on missing data. For the modes
  Batch 1 changed (`castsheet` / `cues` / `preview`) there is no
  other backend to offer at all; the dispatch says so in those
  words (`tools/native_modes/mode_dispatch.mjs`).

## Prerequisites (all checked up front, every miss named)

1. **The `talerender` binary.** Discovery order (code:
   `findBinary()`): `TALERENDER_BIN` (explicit; a bad path is a
   hard error, never second-guessed) → `<project>/native/talerender`
   → `~/workspace/talerender/talerender` (repo checkout) → `PATH`.
   The binary is the `talerender-binary` artifact from a green CI
   run of `turjaun985123/talerender`; there is no local build.
   Every run of a film mode prints the binary path and how it was
   found.

   **Provision it explicitly — this is the recommended setup.**
   Download the artifact from a green CI run, put it at a stable
   path outside any project, and point `TALERENDER_BIN` at it:

   ```bash
   gh run list --repo turjaun985123/talerender --workflow CI --limit 5
   gh run download <run-id> --repo turjaun985123/talerender \
       -n talerender-binary -D ~/talerender-bin
   chmod +x ~/talerender-bin/talerender
   export TALERENDER_BIN=~/talerender-bin/talerender
   ```

   (Dropping the binary at `<project>/native/talerender` also
   works — that is discovery's second slot.) Do **not** rely on
   the repo-checkout fallback: that path holds whatever build
   happens to sit in the checkout, discovery selects it silently,
   and in every local run of the integration wave it was a stale
   pre-gate build. **Verify currency before a capture** — current
   builds know the `--film-telemetry` mode the gate depends on; a
   stale one does not:

   ```bash
   "$TALERENDER_BIN" --film-telemetry 2>&1 | grep -q 'unknown argument: --film-telemetry' \
     && echo 'STALE — predates the film gate; re-provision from CI' \
     || echo 'OK — the film-telemetry mode is recognised'
   ```
2. **A Vulkan driver** for the render modes (`check` needs none —
   the telemetry audit is CPU-only). RADV on the user's PC;
   lavapipe/llvmpipe proves correctness and supports no speed
   claim.
3. **ffmpeg** on `PATH` (finishing + sheet/castsheet assembly).
4. **The project's film data** (next section). No film data, no
   capture — the adapter refuses rather than guessing, names the
   bridge step as the remedy, and never improvises data from
   `src/shots.ts` at capture time.

### Era pairing — binary and bridge data must match

The binary and a project's film data are two halves of one
bridge, and they must come from **compatible repo states**:
provision the binary from a CI run at or after the repo state
the bridge data was generated and validated against (the binary's
loaders and the data's format evolve together in the repo; the
adapter's manifest validation cannot see a binary that predates
the modes it calls). A mismatched — older — binary does not
render wrong frames; it renders nothing. The symptom is exact:
the gate's telemetry call answers
`unknown argument: --film-telemetry` (exit 2), no verdict line
is printed, and per the verdict rule below a missing verdict is
a **failed** gate — `frames`/`video` refuse before a single
frame is rendered. Safe, but blocked: re-provision the binary
(prerequisite 1) and re-run.

Since Batch 1 (CR-B1-10), the no-verdict gate failure prints this
symptom **and** the `TALERENDER_BIN` provisioning remedy inline,
via `tools/native_modes/era_hint.mjs` — classified from positive
evidence only (exit 2, or the binary's own
`unknown argument: --film-telemetry` text). A no-verdict run with
other evidence (exit 3 = film data unreadable, a signal, a crash)
is reported as such, with the stale check offered as one named
possibility, never asserted.

## Film data layout (project side)

```
<project>/native/                  (NATIVE_DIR overrides)
  film.json                        the manifest — OR scenes/film/film.json
  scenes/film/<segment>.scene.json one per manifest segment
  film/                            (NATIVE_FILM_DIR overrides)
    onoda.json                     timing, word anchors, caption chunks
    assets/  plates.json, plate/calendar/sign PNGs, caption strips
```

- The **manifest** (`format: "talerender-film"`, version 1) is
  validated before anything runs (code: `loadManifest()`):
  `fps` must equal `FPS`; segment frame ranges must tile
  `[0, totalFrames)` exactly; `totalFrames` must equal
  `ceil(durationS × fps)` (the binary's own frame rule); every
  segment's scene file must exist. Manifest problems are
  collected and reported together, and the binary-data validation
  below still runs when the manifest stage fails — one fault no
  longer hides another behind it (pre-validation accumulates
  across stages in `main()`).
- The **binary data** is pre-validated by
  `tools/native_modes/filmdata_validate.mjs` (Batch 1, CR-B1-06 —
  the P4 generalisation). The required-asset list is **derived
  from the data itself**, never hardcoded to one film:
  `assets/plates.json` plus every plate it declares (manifest
  shape `plates[].file`, defaulting to `<name>.png`; the legacy
  flat shape is also honoured), the film data's own calendar/sign
  declarations (`calendarYears[]` → `calendar-<year>.png` each,
  `calendarDark`, `sign` — absent fields require nothing), and
  every caption strip named in `captions[].strip`. Duration /
  lines / anchors / captions presence is checked, and the
  manifest↔data duration must agree (same frame count), so the
  two halves of a bridge can't drift apart. A broken bridge fails
  here with the full missing list, not as a panic mid-capture.
  The data file is named `onoda.json` because that is the name
  the current binary reads.
- Bridging a project is upstream work, done once, before capture
  — see `docs/NATIVE-BACKEND.md` ("The bridge") for the two
  routes (`build_bridge.py` for a project's own data,
  `package_bridge.py` for film data the repo already holds) and
  the honest manual remainder.

Rendered frames cache as clean PNGs in
`<project>/renders/native-frames/frame_<nnnnn>.png`
(`NATIVE_FRAMES_DIR` overrides). They are shared by all modes and
valid frames are never deleted by the adapter. The one exception:
a cached frame that fails the integrity check (code: `pngCheck()`
— bad PNG signature, wrong dimensions, or truncated: no closing
IEND chunk, the mark of a hard kill mid-write) is deleted and
re-rendered, never trusted by existence alone.

## Modes

| Mode | Inputs | Outputs | Gate |
|---|---|---|---|
| `check` | film data | the full parity report — duration, renderer provenance, shot list, captions, CAST, audit, corrected caption audit — then `CHECK OK` / `CHECK FAILED` (`tools/native_modes/check_mode.mjs`) | — (it *is* the gate) |
| `frames <dir> "<t,…>"` | times in seconds | `<dir>/t<time>.png` — clean PNG at each time | telemetry first |
| `sheet <out.jpg> [step] [cols] [--guides]` | sampling step, columns | a tiled contact sheet (preview-grade JPEG) + the telemetry audit printed after tiling, report-only (`tools/native_modes/sheet_mode.mjs`) | none |
| `video <out.mp4> [start end] [--draft]` | optional range | the finished master at `<out>` + a `<out>.render.json` provenance sidecar | telemetry first |
| `castsheet [outdir] [kind]` | cast data | native cast sheet (`tools/native_modes/castsheet_mode.mjs`); items whose held prop has no native plate yet are skipped **by name**, never faked | — |
| `cues` | — | **routing only**: prints the `tools/cues_native.py` emitter route + the state of `data/cues.json`, exit 1 always — cues are authored data, and this adapter never produces them itself | — |
| `preview` | — | **retired**: it only ever served the project page for a browser; the native review deliverables are `sheet` / `frames` / `video`, exit 1 | — |

The three changed modes are dispatched **before** binary discovery
and manifest validation (`main()` → `isChangedMode()` in
`tools/native_modes/mode_dispatch.mjs`): none of them needs the
film manifest. `castsheet` receives the discovered binary when
discovery succeeds; its own failure names the missing prerequisite
when there is none.

### check — the parity report

`check` runs `talerender --film-telemetry <film-dir> --tables`
(code: `tools/native_modes/check_mode.mjs`, CR-B1-04) and prints
the full report shape the pipeline's QA gate has always printed:
a `duration <d> s` line, a `renderer:` provenance line (the native
equivalent — binary path + discovery route; there is no WebGL
renderer string on this path), the shot list reformatted from the
tables' SHOT TABLE, the `captions:` list, the `CAST:` line, the
binary's audit lines byte-for-byte (CANVAS OVERFLOW, EDGE TOUCH,
CONTINUITY, CAPTION ZONE, SAFE AREA, FRAMING, COMPOSITION, CAPTION
AUDIT/WIDTH/FIT), and the final `CHECK OK` / `CHECK FAILED`.

Three behaviours are deliberate and disclosed in the code:

- **Shot-line substitution, labelled.** The trailing field of
  each shot line is the shot's set/atmosphere from the tables,
  because the free-text shot `note` exists only in the authored
  staging and is not in the film data. The line says so
  (`tables dressing; OP.shots() note is page-side only`).
- **Authored-caption correction.** The binary constructs every
  film caption as auto-chunked, so its authored count is an
  artefact on any project whose `script.json` authors `caps`.
  When `script.json` is present, `check` recomputes authored
  lines from `lines[].caps` and replaces those lines; without
  `script.json` it passes the binary's lines through and says so.
- **CAP_CHECK semantics.** Caption WIDTH/FIT failures count
  toward the verdict **only** under `CAP_CHECK=strict`; otherwise
  their FAIL tag is downgraded to WARN in the report. The verdict
  is re-derived from the binary's own FAILED counts under that
  rule (the binary's telemetry itself counts caption failures
  unconditionally).

Exit codes for `check`: 0 = `CHECK OK`; 1 = `CHECK FAILED`;
3 = telemetry could not run at all (no verdict line — stale
binary, unreadable data). The 3 is the binary's own
EXIT-CODES-W02 code, surfaced instead of collapsed to 1.

### The gate (frames / video)

Both modes run `talerender --film-telemetry <film-dir>` before
rendering a single frame (code: `runTelemetry()` +
`gateOrStop()`). **There is no flag to render past a failed
gate**, and a partial frame directory is never presented as a
finished capture (it stays on disk for resume — frame(t) is
pure, so re-running after a fix skips what already rendered).

> **Verdict parsing — read this before touching the code.** The
> verdict lives in a stdout line: `FILM TELEMETRY: CHECK OK` /
> `FILM TELEMETRY: CHECK FAILED (…)`. That line is identical and
> authoritative under **both** binary generations; the exit code
> is a cross-check, never a replacement. *Old* binaries exit **0**
> on pass, on fail, *and* on error. *New* binaries (spec:
> `docs/integration/EXIT-CODES-W02.md` in the TaleRender repo)
> agree with the line: **0** = CHECK OK, **1** = CHECK FAILED,
> **3** = telemetry could not run (reason on stderr), **2** = CLI
> error. The adapter echoes the full output, then decides: FAILED
> is tested **before** OK; an OK line passes only with exit 0; a
> FAILED line fails the gate under either generation's code
> (0 or 1); a line/exit-code **disagreement**, and a **missing**
> verdict line (stale binary, crash, bad data), fail the capture
> closed — never a pass. The missing-verdict failure text comes
> from `tools/native_modes/era_hint.mjs` (see Era pairing).

### frames

Each requested time maps to the frame on the 1/fps grid
(`n = round(t × fps)`); when the staged time differs from the
requested one, the mapping is printed. Rendering goes through the
driver's own resume (ranges bridge gaps of ≤ 8 frames; already
rendered frames are skipped — after passing the integrity check
above; an unusable cached frame is re-rendered, not skipped
past). After rendering, each requested frame is verified present
and valid before it is copied out, the staged count is checked
against the request, and every staged copy is verified to exist.
Output naming is `t<time>.png` (e.g. `t1.20.png`) — clean PNG,
the native frame format.

### sheet

The sheet tiles sampled native frames with the retired page
capture's exact tile filter (including its `--guides` grid +
caption-zone overlay), one preview JPEG per sample
(ffmpeg `-q:v 2`), the tiled sheet at `-q:v 3`. Timestamp labels
are burned into a **fixed 250×86 black box** (drawbox fill +
drawtext inside it — the box no longer drifts with the
timestamp's digit count) with a **sans-first** font search under
the project's `fonts/` (Inter / *Sans* / DejaVu Sans Bold system
font before any other project font; unlabelled, with a warning,
when no font exists). After tiling it runs the binary's film
telemetry and prints the audit line classes (CANVAS OVERFLOW,
EDGE TOUCH, CAPTION ZONE, SAFE AREA, FRAMING, COMPOSITION) —
report-only, sampled at telemetry's 0.25 s (a finer superset of
the sheet's own step samples, disclosed on the output line). An
audit that cannot run prints
`CANVAS OVERFLOW: audit unavailable (<reason>)` and exits 1: a
sheet never certifies an audit it did not run. The sheet is a
review deliverable, not a golden — the native reference set is
the native golden pack (see `docs/NATIVE-BACKEND.md`).

### video — finishing parity

Rendering: `talerender --film <film-dir> --out <frames-dir>
--from <n0> --to <n0+n>` (resume built in, cached frames
integrity-checked first), then a completeness check that every
frame of the range exists and is a valid PNG at the manifest's
dimensions.

Finishing is the pipeline's standing video encode — the same
encoder argument sets the skill has always shipped:

- master: `libx264 -preset slow -crf 18 -threads 4`, BT.709
  primaries/transfer/matrix tags *and* the `x264-params` triple,
  `tv` range, AAC 256k @ 48 kHz;
- `--draft`: `veryfast`, CRF 23, AAC 192k;
- audio: `-ss <start> -t <range>` on `data/mix.wav` when present
  (silent lavfi otherwise, and the summary line says `SILENT`);
- `-shortest`, `+faststart`, write to `<out>.part.mp4`, rename
  onto `<out>` only on a zero ffmpeg exit; a failed encode
  removes the `.part` and fails loudly.

The frame source is an `image2` sequence of clean PNGs from
disk. That is the backend's reason to exist — contract decision
(B): native masters carry no q0.93 JPEG generation (the retired
page capture transported frames as JPEG; nothing in this path
does).

One ranged-render note: native frames live on the 1/fps grid, so
a `[start, end]` render begins at frame `round(start × fps)` —
up to half a frame (≤ 17 ms at 30 fps) from the requested start.
Full-film renders (start 0) are unaffected.

A successful video render also writes a provenance sidecar,
`<out>.render.json` (`talerender-render-meta` v1, via
`tools/native_modes/render_meta.mjs`): backend, binary path /
via / `--version` output, manifest path + sha256, frame range,
encode settings, gate verdict. Fields the run did not supply
are `null`, never invented. It is written only after the
`.part` → final rename succeeds — a failed encode writes no
sidecar.

### castsheet

`castsheet [outdir] [kind]` (code:
`tools/native_modes/castsheet_mode.mjs` +
`tools/castsheet_native.py`, CR-B1-03) stages one frame per
pose / mood / look / held prop — or, with kind `cast`, one per
character of the **project's** cast — with no browser:

1. The generator (stdlib-only Python) emits one scene-format v2
   file per item plus a cast manifest into
   `<outdir>/.native-castsheet/` (the auditable staging data).
   The catalog enumeration is the retired page's own: 77 items
   (17 poses, 19 moods, 5 look presets, 36 held props).
2. Each scene is rendered with the binary's `--scene-file` mode
   to a clean PNG at 1080×1920.
3. Each PNG is converted with ffmpeg to
   `<outdir>/<kind>-<id>.jpg` (`-q:v 2`) — the same
   `<kind>-<id>.jpg` names `library/previews/` uses — a labelled
   copy (92 px black bar, `"<kind> — <id>"`) is made at tile
   grade, and the labelled tiles are assembled into
   `<outdir>/castsheet.jpg` with the retired mode's exact tile
   filter (`scale=270:480`, `tile=4xROWS`, padding 4, `-q:v 3`).

Kinds are `pose` | `mood` | `look` | `held` | `cast` (empty =
the whole catalog); an unknown kind fails loudly. **Held props
enter only as pre-rendered plates** (scene-format §14.3): an
item with no plate PNG is **skipped and named** in the summary —
never silently dropped, never faked — and a `held` run that
renders nothing at all fails, naming the missing plates step.
Known gaps are recorded in the staging data, not hidden: moods
`puzzled` / `deadpan` have no native painter variant yet and are
flagged `nativeMoodFallback` in the manifest; look fields the
native character node does not consume are carried in the
manifest as dropped, per item. See `docs/NATIVE-BACKEND.md`
(Known limits).

### cues — routing, not a render mode

`cues` is **not** an adapter render mode. Cues are authored
data, and in this pipeline they are emitted from the project's
bridge/scene data by the skill-side emitter — never extracted
from a page. The adapter's `cues` dispatch
(`tools/native_modes/mode_dispatch.mjs`) prints the routing
resolution — including whether an existing `data/cues.json` is
already newer than the scene data, as far as timestamps can
tell — plus the exact command, and exits 1 **always**: a caller
scripting `capture_native.mjs cues && mix.py` must not proceed
on a cues file this run did not produce.

```bash
python3 tools/cues_native.py            # from the project folder
```

That writes `data/cues.json` in the exact shape `tools/mix.py`
and `tools/audio_recipes.py build_cues` consume (byte-identical
to the retired page extraction on the parity fixture — CR-B1-05).
Clip events are **not** cues: events found in scene data are
written to a `clip-events.json` sidecar in `build_cues`' input
shape, never merged into `cues.json` (`mix.py` would skip their
names silently). An unbridged project fails in the emitter,
naming the bridge step — this adapter does not improvise cues
and offers no other route. Emitter exit codes: 0 = written;
2 = bad input / unbridged / contract violation; 3 =
`--strict-events` tripped (cues.json still written).

### preview — retired

`preview` only ever started a static server and printed a URL
for a browser to open; it launched no capture and was never a
production step. It is retired. The dispatch says so and names
the native review deliverables that replace it — `sheet`,
`frames`, `video` — and exits 1.

## Environment

| Variable | Meaning |
|---|---|
| `RENDERER` | Backend selection guard. This adapter runs when the value is `native` **or unset** — invoking the adapter is itself the selection; any other explicit value refuses here (exit 2). |
| `TALERENDER_BIN` | Path to the talerender binary; wins over discovery, never second-guessed. |
| `NATIVE_DIR` | Film-data root (default `<project>/native`). |
| `NATIVE_FILM_DIR` | The binary's data dir (default `<NATIVE_DIR>/film`, else `<NATIVE_DIR>`); an explicit value without `onoda.json` is a hard error, never second-guessed. |
| `NATIVE_FRAMES_DIR` | PNG frame cache (default `<project>/renders/native-frames`). |
| `FPS` | Must equal the manifest's fps (the binary renders at the manifest rate; today 30). Invalid values are rejected up front. |
| `CAP_CHECK` | `check` mode only: caption WIDTH/FIT failures count toward the verdict only when this is `strict`. |
| `TALERENDER_DOF_DEPTH_FORMAT` | Passed through to the binary (bokeh depth storage; see `docs/NATIVE-BACKEND.md`). |

The retired page capture's variables (`CHROME`, `CHROME_FLAGS`,
`SOFTWARE_GL`, `QS`, `FRAME_TRANSPORT`) have no effect here —
there is no page and no browser for them to configure.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success — for `check`, the gate printed `CHECK OK`; for `castsheet`, the sheet was written (skips, if any, are named in its summary). |
| 1 | Capture failed: telemetry gate failed, a prerequisite or film-data item is missing/invalid (named), the binary or ffmpeg exited non-zero, a `sheet` audit could not run, or a `castsheet` prerequisite is missing. `cues` (routing answer) and `preview` (retired mode) exit 1 under this clause by design — neither is a usage error. `check` also exits 3 when telemetry could not run at all (no verdict — see check). |
| 2 | Usage / selection error: unknown mode, bad `sheet` step/cols, bad `castsheet` kind, or an explicit non-native `RENDERER` selection. |

## Known limits of this adapter

- The binary renders at the manifest's frame rate only (30 fps
  today) and at the manifest's resolution; there is no canvas
  scaling knob on this path.
- `frames`/`sheet` times snap to the frame grid (printed when
  they move).
- The telemetry verdict depends on the binary's stdout wording
  above; if a future binary changes that line, this adapter
  fails closed (no verdict = failed gate) until updated.
- `castsheet` held props are plate-bound (see castsheet): the
  catalog's held items without a plate are skipped by name until
  plates exist for them.
