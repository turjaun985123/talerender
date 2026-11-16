# Chrome-Replacement REMEDIATION — contracts (driver, telemetry, decoupling, rulings)

Mission: close the one failure of the Chrome-replacement wave
(final handover 2026-11-15, HEAD `9743850`): the film driver and
telemetry are hardcoded to the Onoda film, so the Batch 3
acceptance proof failed for new films and there was NO flip.
User approval 2026-11-16 09:18 +06: the 4-package remediation
scope AND all 13 B3-07 staging rulings, as a set.

Team: coordinator/integration lead + up to 20 seats in TWO
batches of 10 (parallel ceiling 10; actual counts reported at
every batch close). The lead owns `src/*.rs` core integration,
`Cargo.toml`, workflows, the skill's SKILL.md / template /
scripts integration points, ALL commits in both repos, the flip
commit, and the GitHub sync. Seats write NEW files/modules
only (replacement candidates for existing files are delivered
as complete new files or patch candidates; the lead applies
them). Seats never commit.

## 0. Verified state (2026-11-16, from code at `9743850` — not from records)

- `run_film` (film.rs:3726) and `run_film_telemetry`
  (film.rs:3992) stage the Onoda film from hand-transcribed
  shot functions (`s01`…`s12` in film.rs) driven by `FilmData`
  (film/onoda.json shape: duration, anchors, captions, lines,
  calendarYears, calendarDark, sign). Staging indexes
  `data.anchors["years0"]` etc. BY NAME; telemetry's shot table
  is built from anchor names with hardcoded ids
  (film.rs:3993-3997); both banners print the Onoda title as a
  string literal (film.rs:3754, 4101). A non-Onoda film panics
  (`no entry found for key`) — CR-B3-01, verified.
- The loader DEMANDS `calendarYears` / `calendarDark` / `sign`
  by name from every film (film.rs:126/151). Consumers:
  `calendar_state` (film.rs:1808 — already rules an EMPTY
  calendarYears legal), the calendar overlay + asset loads
  (film.rs:2295-2306: calendar year PNGs, dark plate, sign PNG
  loaded by filename from the data), the s07 staging. Nothing
  else reads them.
- The generic scene path exists and is proven for stills:
  `scenefile::load` parses a scene-format v1/v2 file, folds §15
  via `eval15::process_scene` AT THE FILE'S `time`, walks nodes
  to draws; `run_scene_file` renders ONE frame (direct path, or
  composer path when the file asks for bokeh). Scene files
  carry camera keys evaluated at `time` — so a scene evaluated
  at local time `lt` is the same load with `time := lt`.
- Manifests exist for both layouts and are data: project
  layout `<native>/film.json` (segments[] with id, scene path,
  lines, startS/endS, startFrame/endFrame, set, atmosphere,
  captionChunks; plus title, fps, durationS, totalFrames) with
  film data at `<native>/film/` and scenes at
  `<native>/scenes/film/`; repo layout `scenes/film/film.json`
  with film data at `film/` (paths repo-root-relative). The
  adapter validates the manifest; the binary today ignores it.
- Captions: film path overlays strips post-render from
  `FilmData.captions` (`eval_frame` sets `spec.caption` +
  `caption_scale = sc * caption.fit`, film.rs:2214;
  `overlay_caption` film.rs:3789). Caption strips are PNG
  assets (cap_NNN.png) loaded by `load_assets` via plates.json.
  The Lighthouse Cat bridge HAS strips + plates; its data
  carries `fit: 1.0` assumed (F4) and invented
  calendarYears/calendarDark/sign (F3).
- `telemetry.rs` (M6) computes `cutouts()` over scene data on
  CPU — the generic overflow/continuity source.
- Pre-wave Onoda census: CI run 43797684898 dispatched at
  `9743850` before any remediation landing (frame hashes in the
  job log; saved locally to talerender-evidence on completion).

## 1. Driver selection contract (the load-bearing decision)

`--film` / `--film-telemetry` gain a manifest-driven fork,
decided BEFORE any film data is loaded:

1. Discover the manifest for the film dir `D` (the `--film`
   argument): try `D/../film.json`, then `D/film.json`, then
   `D/../scenes/film/film.json`. Parse it if found.
2. If the manifest carries `"driver": "scene-sequence"` →
   the GENERIC path (§2/§3). Segment scene paths resolve
   against, in order: the manifest's directory, `D/..`, `D/../..`;
   first existing file wins; a segment whose scene resolves
   nowhere is a named load error (fail closed).
3. Otherwise → the LEGACY path (today's code staging),
   unchanged. The Onoda manifests carry no `driver` field, so
   Onoda stays on the staging that produced every established
   reference — the census gate (§5) is protected by
   construction, and still proven by census.
4. Legacy-entry guard: the legacy path's staging consumes a
   fixed set of Onoda anchor names. At legacy entry
   (run_film + run_film_telemetry), if the loaded data's
   anchor set does not contain the required names, return a
   NAMED error — "film data is not staged by the legacy
   driver; bridge it with driver: scene-sequence (see
   REMEDIATION-CONTRACTS §1)" — never a panic. This converts
   the B3-01 failure class into a readable defect for any
   future mis-bridged film.
5. `build_bridge.py` emits `"driver": "scene-sequence"` for
   films it bridges from scene data. The Onoda repo film is
   NOT re-bridged and gains no driver field.

## 2. Package 1 — generic scene-sequence film driver (`src/filmseq.rs`)

- New module `filmseq` (seat-written; lead wires `mod` +
  the §1 fork in film.rs entry points).
- API the seat builds against (lead landing L0, behaviour-
  preserving refactor of scenefile.rs, gated by CI + golden
  verify + byte probes BEFORE Batch 1 spawns):
  - `scenefile::load_root(root: Value, label: &str) -> Result<Loaded, String>`
    (the body of today's `load`, minus file read/parse;
    `load(path)` keeps its exact behaviour via load_root).
  - `scenefile::render_loaded_pixels(w, h, scene, knobs)
    -> TaleResult<Vec<u8>>` (L0 as built: the two render
    paths gained a `SceneOut` sink — File keeps today's
    exact behaviour/prints; Pixels returns RGBA in memory).
    Device/pipelines are built per call in v1; a persistent
    SceneRenderer (device + pipeline reuse across frames)
    is a NAMED FOLLOW-UP, not a gate input — it changes no
    pixel, and the proof budget (§5) fits without it.
- Driver behaviour: for frame n in [from, to): t = n/fps;
  find the segment owning n (manifest frame ranges tile
  [0, totalFrames)); lt = t - segment.startS; clone the
  segment's parsed scene root, set `time` to lt (f32 law:
  the value written is the f32 nearest lt), `load_root`,
  render. Post-render, apply the film-level overlays the
  scene path does not own, in the film path's order: caption
  overlay from FilmData (`caption_at(data, t)` mirrors
  eval_frame's selection; scale = sc * fit as in film.rs —
  the seat extracts the exact rule from eval_frame) using
  `load_assets` strips. NO calendar overlay, NO fade unless
  the scene data itself carries them (scene post/grade is
  the scene path's own).
- Streaming (WP4 OutputMode::Stream) on the generic path:
  support it via the same FullContractSink contract the
  legacy stream branch uses, or fail loudly naming it —
  the seat's charter decides from the adapter's actual use
  (the adapter's video mode is disk-frames + finish; stream
  is exercised only by --film direct runs). Silence is not
  an option; a named limitation is.
- Banner: `film: <manifest title> — <totalFrames> frames
  @<fps>fps, rendering <from>..<to> at <W>x<H> (clean PNG
  frames; decision B)` — the manifest's title, never a
  literal. Progress/completion lines mirror run_film's
  shapes (the adapter and bench tooling parse none of them,
  but humans and logs do).
- Determinism: same scene root + same lt → same Loaded →
  same pixels (the scene path's existing property); the
  driver adds no nondeterminism (no wall-clock, no HashMap
  iteration into draw order).

## 3. Package 2 — generic telemetry (`src/filmtelemetry.rs`)

- Invoked from the §1 fork in `run_film_telemetry` when the
  manifest selects the generic driver. CPU-only, like the
  legacy gate.
- Output contract: the SAME verdict line —
  `FILM TELEMETRY: CHECK OK` / `FILM TELEMETRY: CHECK FAILED`
  — and a report whose sections mirror the legacy report
  (duration/fps/frames header with the manifest's title;
  shot table; caption audit; overflow; continuity) so
  `check_mode.mjs`'s parsing and the adapter's verdict
  handling work unchanged. Any line check_mode requires
  that the generic report cannot produce from data is a
  named finding in the seat's note, not a silent omission.
- Shot table: from manifest segments (id, startS, endS,
  lines, caption chunk ranges) — no anchor names, no
  hardcoded ids.
- Caption audit: from FilmData captions (text, s, e, fit,
  strip) with the legacy audit's floors (rendered width
  ≤ 850 px, fit ≥ 0.68 hard; display-time advisory) —
  widths computed by the same estimator the legacy audit
  uses (the seat extracts it; if it lives in film.rs
  staging, it moves to a shared pub(crate) home).
- Overflow/continuity: sampled at 0.25 s like the legacy
  audit; per sample, the owning segment's scene is loaded
  at lt and `telemetry::cutouts()` (or the draw bounds the
  seat evidences as equivalent) yields the character
  rectangles the overflow/continuity rules consume.
  Continuity = look signature stability across samples of
  a segment, per the legacy rule's data analogue.

## 4. Package 3 — loader decoupling (film.rs + bridge + validator)

- `load_film_data`: `calendarYears` absent → empty vec
  (already legal per calendar_state's ruling); `calendarDark`
  absent → empty string; `sign` absent → empty string.
  Present-but-wrong-type still fails BY NAME (era pairing
  fails closed on malformed data; only ABSENCE becomes
  optional).
- `load_assets`: calendar year PNGs load only for declared
  years; the dark plate and sign PNG load only when their
  data keys are non-empty (assets fields become
  Option/empty-aware at their consumers: the calendar
  overlay runs only when a CalendarState exists; the
  Renderer::new sign texture falls back to a 1×1 transparent
  texture when no sign is declared — the sign plane is only
  drawn by s07 staging, which only runs when calendar data
  exists). Onoda (all keys present) is byte-identical by
  construction; the census proves it.
- `build_bridge.py`: stops INVENTING calendar/sign for
  films whose scenes never use them — the keys are emitted
  only when the film's data actually declares them; the
  emitted manifest gains `"driver": "scene-sequence"`.
- `validate_native_project.py`: a fresh bridge without
  calendar/sign is NOT "STALE BRIDGE"; staleness remains
  MANIFEST-SHA / generation drift only.
- The Lighthouse Cat project is re-bridged under the new
  rules (Batch 2): no invented 31-year calendar, no harbour
  sign declaration, driver field present, measured caption
  fit (Package 4). Its original bridge is preserved in the
  B3-01 seat directory for diffing.

## 5. Acceptance gate (both legs; no partial credit)

(a) ONODA CENSUS: post-remediation census at the final
    talerender HEAD — 1,811/1,811 frame hashes identical to
    the pre-wave census (run 43797684898, at `9743850`).
    Any differing frame must be defect-attributed and
    reported BEFORE proceeding; an unexplained diff fails
    the gate.
(b) LIGHTHOUSE CAT PROOF: CI proof run mirroring
    43750140530 (binary built from the final HEAD, Chrome
    scrubbed + exec-logging shim) running the SHIPPED
    SKILL's adapter on the re-bridged Lighthouse Cat
    project: check / frames / sheet / video legs exit 0,
    `FILM TELEMETRY: CHECK OK` from the GENERIC telemetry,
    `CHROME-EXEC-LOG: EMPTY`, draft MP4 produced.
    check also runs on the Onoda fixture in the same run
    (legacy path regression).

ONLY IF BOTH LEGS PASS — the flip (staged package B3-10,
refreshed with this wave's evidence):
- Binary provisioned from the flip-time HEAD's CI artifact
  (era checks re-run against it — the B3-10 staged binary
  is stale by construction now).
- SKILL.md 7.0 stamp + CHANGELOG v7.0 entry (rationale:
  renderer replacement is breaking) + castsheet
  fallback-clearing patch + README stamp, as ONE separate
  flip commit in the skill tree, LAST.
- Standing sync: verify `gh api user` = `turjaun985123`
  FIRST (abort + report if not), then
  `~/workspace/tall-tale-upgrade/sync-skill-to-github.sh`;
  previews stay excluded per .gitignore; report the sync
  commit on turjaun985123/tall-tale-shorts.
If either leg fails: NO flip, NO bump, NO sync — report
the failure with evidence.

## 6. The 13 rulings — implementation map (approved text = DESIGN.md §5 seat recommendations)

| Ruling | Lands as |
|---|---|
| R-W1 sets staged by builder's default call | world staging generator config (Batch 1 seat R1-07) |
| R-W2 SET-CAM eye [0,2.6,10.5] fovY 44 / PROP-CAM fovY 42, az 35°, el 20° | generator constants, as approved |
| R-W3 prop3d alone at origin on REF ground disc | generator |
| R-W4 scale figure OFF prop3d/atmosphere, ON for sets | generator |
| R-W5 port painters: siding, brick, dirt, sand, facade — no substitutions | talerender/skill texture registry port (seat R1-05 investigates the registry from code, ports all five) |
| R-W6 text props via generalised held-plate path | plate pipeline tooling (seat R1-06) |
| R-A1 poster τ = D/2 | poster generator (seat R1-08) |
| R-A2 root-motion staged in place | poster generator |
| R-A3 shot posters on REF-STAGE + base figure | poster generator |
| R-A4 previewSheet produced alongside, NOT gating | poster generator + checker semantics unchanged for sheets |
| R-A5 one canonical probe-fixture set | fixture set authored once (seat R1-08), reused |
| R-A6 exempt 53 T4 items per-item with reasons | library_check.py `previewExempt` mechanism + the 53 named exemptions from ACCOUNTING.json (seat R1-09); strict-previews then reports honestly |
| R-A7 camera-roll format amendment (optional `roll` on camera + keys, Z-rotation after look-at) | scenefile patch candidate (seat R1-04) + scene-format.md amendment; additive — absent roll = today's bytes (probe-gated) |

Expected effect (B3-07 accounting): 242/318 previews
generatable; 73 blocked by name (20 T3 + 53 T4-exempt).
strict-previews state is REPORTED as built — generatable
is not generated; renders ride CI/PC legs.

## 7. Package 4 — bridge docs + caption-fit + era-pairing

- Bridge docs: SKILL.md §5 step 4 gains the followable
  bridge procedure (skeleton export invocation, --anchors,
  --assets-dir rasterisation duty, the data declarations a
  film actually needs post-decoupling) — drafted by seat
  R2-01 from build_bridge.py's real interface, landed by
  the lead. F8's standard: a fresh user can bridge from
  the skill alone.
- Caption-fit MEASURED: the font measurer the audit uses
  is run over the Lighthouse Cat's 43 chunks (and the
  Onoda 73 for reference); build_bridge writes measured
  fit values, never assumed 1.0 (seat R1-10 measures +
  bridge patch candidate; lead lands).
- Era-pairing: SKILL.md §0's symptom list gains the
  current-binary panic case (exit 101 on unstageable
  data) — post-remediation its named meaning is the §1.4
  legacy-entry guard error; era_hint.mjs classification
  extended to match (seat R2-02). F2/F9 closed by §1.4 +
  this text.

## 8. Gates per landing (standing, from the parent contracts §3)

CI + Tools CI read at STEP level (quota-red upload step
named, never read as code failure; cancelled ≠ failed);
`FILM TELEMETRY: CHECK OK`; determinism byte-probes intact;
golden verify 17 OK after any scenefile change; skill
`LIBRARY OK` (479 items) after any skill change; no
rendered photos/frames to GitHub (evidence local in
~/workspace/talerender-evidence/, hashes/metrics as text);
small CI artifacts only; final encode stays x264 CRF 18;
era pairing fails closed. ALL proof is lavapipe/CI — the
first RADV confirmation of the Chrome-free skill rides
the user's PC session, and the handover says so plainly.

## 9. Batch plan

- **L0 (lead, before Batch 1):** this contract + the
  scenefile API extraction (§2) — CI/golden/probe gated.
- **Batch 1 (10 seats):** R1-01 filmseq driver; R1-02
  generic telemetry; R1-03 loader decoupling patch
  candidate + validator/bridge patch candidates; R1-04
  camera roll (R-A7); R1-05 texture painters (R-W5);
  R1-06 text-plate generalisation (R-W6); R1-07 world
  staging generator (R-W1..W4); R1-08 poster generator +
  probe fixtures (R-A1..A5); R1-09 checker exemption
  (R-A6); R1-10 caption-fit measurement (Pkg 4).
- **Batch 2 (10 seats):** R2-01 bridge docs draft (Pkg 4);
  R2-02 era-hint 101 + §0 text draft; R2-03 Lighthouse Cat
  re-bridge; R2-04 census operations (pre/post + diff);
  R2-05 CI proof fixture + workflow patch for the LC film;
  R2-06 flip package refresh; R2-07 failure-mode re-tests;
  R2-08 grep audit (both repos, classified); R2-09
  strict-previews + library verification as built;
  R2-10 selftest + doctor/validator regression run.
- **Lead integration** after each batch; acceptance (§5);
  flip + sync only on a full pass.
