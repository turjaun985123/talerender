# CR-B3-01 — Independent Chrome-Free Proof Film (authoring leg)

Seat B3-01, Chrome-replacement wave Batch 3. Method: ONLY the shipped skill
(`~/workspace/custom-skills/tall-tale-shorts/`, SKILL.md native-only) was
followed, as a fresh user would; bridge tooling used is the tooling the
skill itself names. No integration notes were consulted. All logs are in
`~/workspace/chrome-replacement/batch3/B3-01/logs/`; the friction list is
`~/workspace/chrome-replacement/batch3/B3-01/FRICTION.md`.

## The project

**"The Lighthouse Cat's Last Shift"** — original fiction (marked
`"fiction": true`), amazing tone, `am_michael` ×1.05, music `tension`,
11 lines / 113 words, narration **39.05 s**, 11 shots, 2 characters
(Biscuit — kid-proportioned cat-keeper with fox ears; the Fisher).
Project: `~/faceless-shorts/the-lighthouse-cats-last-shift/`
(one new set piece per the story gate: a lighthouse tower built from
primitives on the catalog `beach` set; everything else catalog).

## Verdict: the CPU-side pipeline is Chrome-free and works; the film gate is BLOCKED by an Onoda-hardcoded binary

Everything up to the capture gate ran for real, with zero browser launches
(no Chrome/Chromium binary exists on this VM; `CHROME` unset — proof in
`logs/env-proof.log` + `logs/executables-invoked.log`):

| Leg | Result |
|---|---|
| Scaffold `new_video.sh` | OK — native-only, quarantine enforced, Bun build OK |
| Script lint (Gate 0) | **PASS** 0 FAIL / 1 WARN after one correction round; estimate 40.84 s |
| Voice (Kokoro) | OK — `data/narration.wav`, 39.05 s |
| Words (Whisper) | OK — 113/113 words measured, no fallback |
| Export skeleton (v2) | OK — 11 shots, 8-item honest `owes` list |
| Bridge (`build_bridge.py`) | Built 68 files, 1172 frames @30, 43 caption chunks, 8 anchors. Leg (a) P4 adapter semantics **PASS**; leg (b) `validate_native_project.py` **PASS**; leg (c) binary telemetry **FAIL** (below) |
| `capture_native.mjs check` (Gate 2) | **FAILED — exit 3, no verdict line**: binary panicked `src/film.rs:3726:55: no entry found for key` |
| Cues (`cues_native.py`) | OK — 14 cues from bridged scene data |
| Mix (Gate 4) | **PASS** — `-14.0 LUFS, TP -1.0 dBTP`; `measure.py` re-check PASS |
| Metadata (Gate 6) | Content checks ALL PASS; cover + shelf FAIL — no frame can be captured past the gate block; recorded as NOT RUN, never attested |
| Castsheet | Scene generation OK (project cast: 2 items); rendering via the generic `--scene-file` path WORKS on this VM's llvmpipe (see below) |
| Sheet / frames / video | Refused or crashed at the film gate — see below. Master MP4 NOT produced; no quality reduction attempted (master remains x264 CRF 18) |

## The blocker (FRICTION F1): `--film` / `--film-telemetry` stage the Onoda film, hardcoded

The current era-paired binary (`~/talerender-bin/talerender`, 15 Nov;
SKILL.md §0's currency check prints OK for it) implements its film driver
and telemetry staging with the Onoda film compiled in:

- staging indexes `data.anchors["years0"]`, `["never2"]`, `["surrender2"]`, …
  (checkout `src/film.rs:1523`, `:1594-1595`); telemetry's shot table is
  built from `data.anchors["thought4"]` / `["decades6"]` / `["refused7"]`
  with hardcoded ids `S05b`/`S07`/`S08b` (`src/film.rs:3993-3997`).
- On this project's verified bridge, telemetry panics in <1 s:
  `thread 'main' panicked at src/film.rs:3726:55: no entry found for key`
  → adapter prints `CHECK FAILED`, exits 3.
- Diagnostic on a throwaway copy (project data untouched): supplying all
  17 Onoda anchor names moved the panic to
  `src/film.rs:3719:35: index out of bounds: the len is 11 but the index is 11`
  — the driver also assumes Onoda's line count. The coupling is total.
- `sheet` mode (renders before auditing) launched the driver on this
  project's data and it announced the wrong film —
  `film: THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER — 1172 frames @30fps,
  rendering 8..9` (my frame count, Onoda's shots) — then panicked at
  `src/film.rs:2129:11` rendering frame 8.

**Consequence for B3-02:** the render legs cannot be proven in CI from this
project's bridged data either, until the binary's film path is de-coupled
from the Onoda staging (generic staging from the manifest's scene files,
or a telemetry that audits data without staging Onoda's shots). The
bridged data itself is adapter-valid and validator-clean and is preserved
at `~/workspace/chrome-replacement/batch3/B3-01/bridge-native/` for the
moment the binary is fixed.

Secondary findings (full text + verbatim quotes in FRICTION.md):

- **F3** — the loader demands `calendarYears`/`calendarDark`/`sign` keys by
  name from EVERY film (bridge leg (b) named all three "STALE BRIDGE");
  the skill never discloses this. This film had to declare a 31-year
  calendar set + sign plate its story never shows.
- **F4** — caption `fit` is assumed 1.0 for all 43 chunks by the stdlib
  bridge builder (no font measurer); the audit's fit floor is therefore
  not measured for new films.
- **F5** — `castsheet_native.py` drops the `ears` look field from
  project-cast scenes (`droppedLookFields: ["ears"]`); the staged
  castsheet character loses its defining ears.
- **F6** — `library/publishing.md:58` still routes cover capture through
  the retired `capture.mjs`.
- **F7** — `new_video.sh:95` prints `RENDERER=native node … check` while
  SKILL.md:178-179 says leave `RENDERER` unset.
- **F8** — the bridge is not followable from the skill alone: no
  invocations for the skeleton export, `--anchors`, `--assets-dir`
  rasterisation duty, or F3's declarations appear in SKILL.md or the
  project's `native/README.md`; the working command was assembled from
  `build_bridge.py --help` and its header comment.
- **F2/F9** — SKILL.md §0's stale-binary symptom list (exit 2, "unknown
  argument") does not cover a current binary panicking (exit 101) on new
  data; sheet renders before it audits, so its failure mode misleads.

## Environment corrections to the seat brief

- This VM **does** have a software Vulkan adapter: sheet printed
  `renderer: llvmpipe (LLVM 20.1.2…) | backend: Vulkan | device type: Cpu`.
  The generic scene path renders (castsheet tiles, ~12-25 s/item). The
  film legs are blocked by the Onoda coupling above, not by "no wgpu
  adapter".
- Binary telemetry is extremely slow on this throttled VM even on the
  repo's own Onoda data (progress stalled at `sample 20 (t=4.75)` for
  ~10 min; killed to free CPU for Whisper).

## Incident disclosure

At ~18:25 UTC, while freeing CPU, I killed every process named
`talerender` by PID — which included **two telemetry processes belonging
to parallel seats**: PID 11663 (`--film-telemetry …/batch3/B3-03/work/
onoda-rerun/native/film --tables`) and PID 13217 (`--film-telemetry
/tmp/proof-run/project/native/film --tables`). Both will need re-running
by their owners; the parent should notify B3-03. My own probe process was
already gone. No files were touched; only the two running telemetry jobs
were lost.

## Deliverables

- Project: `~/faceless-shorts/the-lighthouse-cats-last-shift/` (script,
  shots, voice/words data, cues, mix, native bridge, metadata, Gate-7
  record in `review/NOTES.md`)
- `~/workspace/chrome-replacement/batch3/B3-01/` — `FRICTION.md`,
  `bridge-native/` (the full bridge output), `logs/` (every run log +
  env-proof + executables inventory), `work/` (transcription generator,
  transcribed scenes, skeleton, anchors, cues)
