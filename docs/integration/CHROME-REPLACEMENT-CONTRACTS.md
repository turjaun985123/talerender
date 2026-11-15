# Chrome Replacement — Phase 0 contracts (mode matrix, ownership, acceptance)

Mission: replace Chrome completely in Tall Tale Shorts v6 (user
order 2026-11-15 18:53 +06: "Assign 30 agents to completely
replace chrome from tall tale shorts v6"). This order SUPERSEDES
the opt-in ruling of 2026-11-15 morning: the goal is that no step
of making a film with the skill launches Chrome — the native
renderer becomes the default and only production renderer.

Team: coordinator/integration lead + 30 seats in three batches
of 10 (parallel ceiling 10; actual counts reported at every
batch close). Repos: this repo (HEAD `0dec530` at Phase 0) and
the shipped skill `~/workspace/custom-skills/tall-tale-shorts/`
(v6.0; the skill tree is not a git repo — it publishes through
the standing sync script at the end).

## 0. Verified state (2026-11-15, from code — not from stale docs)

- Adapter (skill `template/tools/capture_native.mjs`, adopted
  this morning): modes `check` / `frames` / `video` / `sheet`
  (partial: tiles native frames, runs no gate); **refuses**
  `cues` / `castsheet` / `preview` (exit 1, fallback text).
  Binary discovery `TALERENDER_BIN` → project → repo checkout →
  PATH; era pairing fails closed (a stale binary answers
  `unknown argument: --film-telemetry`, no verdict line = failed
  gate). Finishing mirrors `capture.mjs` argument-for-argument
  (x264 CRF 18 slow, BT.709, AAC 256k, `.part` discipline).
- Film data layout: `<project>/native/film.json` (manifest
  `talerender-film` v1) + `scenes/film/*.scene.json` +
  `film/onoda.json` (timing/anchors/captions; the filename is
  the binary's current one) + `film/assets/` (plates, calendar/
  sign plates, caption strips). The adapter's `validateFilmData`
  HARDCODES the Onoda asset list (WP9 finding, proposal P4).
- Bridge: `tools/bridge/package_bridge.py` packages film data
  the repo already holds into a project layout and
  self-verifies. It does NOT create film data for a new project
  (WP9 proposal P5 — the known gap). §15 evaluator: scene-format
  §15 (prefabs §15.2–15.5, clips §15.6–15.8) is folded into the
  format + checkers (`tools/format/check_clips.py`,
  `check_prefabs.py`, `stage_clips.py`); the loader's step-2
  CONSUMPTION of clips/prefabs is not implemented (guard-only).
  The film.rs s12 code-side retirement rides step 2.
- V2 modules landed but unwired (V2-HANDOVER): WP4 streaming
  (`streamq`/`ffstream`/`streammodes` — `--film` resolves the
  mode and fails loudly on Stream; no session in `run_film`),
  WP7 `scheduler` (planning only), WP8 `assetstream` (no
  production caller), WP11 `outputmgr` (not wired), WP2
  `memreport` (modes do not print it), WP1.2 `dirtyrect`
  (inert — `upload_dyn` creates a fresh texture per frame),
  WP1.3 `mempressure` (partially wired), WP5 `projectfile.rs` +
  `tools/export/export_project.mjs` (C4 interchange, emit-side
  validated; not yet a staging transcription path).
- Cues (F6 design, `CUES-W45.md`): `capture.mjs cues` evaluates
  `OP.cues()` in the Chrome page → `data/cues.json`, consumed by
  `tools/mix.py` (with `tools/audio_recipes.py` `build_cues` as
  the standard compiler on top). The authored cues are DATA:
  scene files already carry `shot.cues` transcribed and
  audit-verified against `shots.ts` (Onoda: 9 cues / 14
  segments). Nothing in the native path emits `cues.json`.
- Castsheet (`template/src/castsheet.ts` + `castsheet.html`):
  stages one frame per pose/mood/look/held prop →
  `<kind>-<id>.jpg` + labelled `castsheet.jpg`. It is ALSO the
  library-preview generator: `library/previews/<kind>-<id>.jpg`
  is "the name castsheet writes" (`scripts/library_check.py`).
- Skill Chrome touchpoints (grep-verified): `scripts/setup.sh`
  (Chrome is a hard requirement, lines 31–32), `scripts/
  selftest.sh` (runs `capture.mjs` check/cues/sheet/video +
  the RENDERER-guarded native hook), `template/tools/
  capture.mjs`, `template/tools/capture.ts` (Bun port, same
  Chrome dependency), `template/tools/perf_probe.mjs` (+
  `tests/28/perf_probe.test.mjs`), SKILL.md / README.md /
  STYLE.md / CHANGELOG.md, `docs/NATIVE-BACKEND*.md`,
  `tools/selftest_native.sh` + `tools/SELFTEST-NATIVE.md`.
- Library: 479 items, `LIBRARY OK`; previews live in
  `library/previews/` (excluded from the GitHub sync by
  `.gitignore` — functional catalog, local by the no-photos
  rule).
- Binary provisioning: CI artifact `talerender-binary`. The
  artifact store hit its quota during V2 (uploads failing since
  ~04:40 UTC 2026-11-15; newest downloadable binary = `967fa37`,
  the v1.0.1 bump). Phase 0 deleted the stale artifact backlog
  (~1.98 GB of superseded run byproducts; verdicts are recorded
  in docs, frame evidence is local in
  `~/workspace/talerender-evidence/`) to restore uploads.
- This VM has Vulkan ICDs including lavapipe (`lvp_icd.json`);
  local native renders are possible (slow, correctness-only).
  There is no `/dev/dri` — no RADV here, ever. All proof in
  this wave is lavapipe/CI; the first RADV confirmation of the
  Chrome-free skill rides the user's PC session
  (PC-SESSION-W98). The handover must state this plainly.

## 1. Mode matrix

Every `capture.mjs` mode and every Chrome touchpoint in the
skill, classified. REPLACE-NATIVE = build the native
equivalent. NOT-CHROME = the step never needed a browser;
route it correctly. EXCEPTION = irreducible in this wave;
named, evidenced, reported to the user — never silently kept.

| # | Item | Class | Disposition |
|---|------|-------|-------------|
| 1 | `check` | REPLACE-NATIVE | Exists (adapter check = film telemetry audit). Batch 1 verifies itemised parity vs the Chrome check's outputs (overflow / continuity / caption audit / composition warnings — the warnings derive from cutouts telemetry, proven field-identical in M6; any gap is closed or named). |
| 2 | `frames` | REPLACE-NATIVE | Exists. PNG output instead of JPEG is contract decision (B), documented, not a gap. |
| 3 | `video` | REPLACE-NATIVE | Exists (finishing parity argument-for-argument). Batch 1 wires the WP4 streaming session into `run_film` so the driver path is first-class; the adapter's disk-frames path already satisfies the mode contract. |
| 4 | `sheet` | REPLACE-NATIVE | Exists as partial. Batch 1 verifies tile-filter / label / `--guides` parity. The "golden-sheet procedure stays Chrome" clause is RETIRED by this mission: the review-sheet procedure becomes native; the golden reference is the talerender native golden pack (local-only images, hashes as text), never a sheet. |
| 5 | `cues` | NOT-CHROME (routing) | Cues are authored data + word timings; page evaluation was an extraction convenience. Batch 1 builds a skill-side emitter that writes `data/cues.json` from the project's native scene data (+ clip events, per the seat's evidence) in the exact shape `mix.py` / `build_cues` consume. Parity fixture: the Onoda project's existing Chrome-produced `data/cues.json`. An unbridged project fails loudly naming the bridge step — never a Chrome fallback. |
| 6 | `castsheet` | REPLACE-NATIVE | Batch 1 builds it: cast staging as scene data, binary/adapter rendering per item, labelled `castsheet.jpg` tiling. Doubles as the library-preview generator (Batch 2). |
| 7 | `preview` | NOT-CHROME (retired) | Verified in code: `capture.mjs preview` only starts a static server and prints a URL; it launches no browser (the page-open branch is skipped for this mode). It is not a production step. The mode retires with `capture.mjs`; the client-review deliverable becomes native sheet + frames + video (Batch 2 docs). |
| 8 | `capture.ts` | REPLACE-NATIVE (with #1–7) | Bun port of `capture.mjs`, same Chrome dependency; quarantined/removed with it in Batch 2. |
| 9 | `perf_probe.mjs` (+ its test) | REPLACE-NATIVE | A Chrome-page draw-cost probe. The native equivalent is the WP6 bench lab / perf tooling in this repo. Template copy retires in Batch 2; docs point at the bench lab. |
| 10 | `scripts/setup.sh` Chrome check | REPLACE-NATIVE | Requirement becomes: talerender binary discoverable + a Vulkan ICD present (lavapipe acceptable, labelled as software) + ffmpeg. Chrome is neither required nor checked. |
| 11 | `scripts/selftest.sh` | REPLACE-NATIVE | Batch 2: the native path becomes the test's subject end-to-end (check → cues → sheet → video on the template project, bridged by Batch 1 tooling); the RENDERER guard is inverted/removed — native is what is being tested. |
| 12 | `scripts/new_video.sh` | REPLACE-NATIVE | Batch 2: scaffolds the native-first template; printed flow commands are the native ones. |
| 13 | Library previews | REPLACE-NATIVE | Batch 2 generates pending previews with the Batch 1 castsheet machinery; `library_check.py` stays the validator (its `--strict-previews` state is reported honestly). |
| 14 | Golden-sheet procedure (SKILL.md §6, qa-gates) | REPLACE-NATIVE | Re-based: review via native sheet; golden reference = native golden pack. Chrome golden comparisons are retired, not preserved. |
| 15 | SKILL.md / README / STYLE / modes docs | REPLACE-NATIVE | Batch 2 rewrite of every capture instruction to the native flow. Historical wave docs in `docs/` stay as history and are classified as such in the final grep audit. |
| 16 | `tools/selftest_native.sh` + hook | REPLACE-NATIVE (absorbed) | Once native is the default, the separate opt-in smoke is redundant: Batch 2 folds its legs into the native selftest flow and retires the guard (lead decision at integration, documented). |
| 17 | Existing projects' `tools/` (pre-adoption) | REPLACE-NATIVE | WP9 proposal P1: Batch 2 ships the retrofit step (adapter + bridge into a pre-adoption project); Batch 3 runs it on the Onoda project. |
| 18 | `qa_visual.py` / `qa-pack.json` | Verify in Batch 2 | Expected backend-agnostic (reads images); if it invokes capture, it is routed native. |
| 19 | Project build (`bun run build`) | NOT-CHROME | TypeScript compilation; no browser. Unchanged. |
| 20 | `voice.py` / `words.py` / `mix.py` / metadata | NOT-CHROME | Kokoro / Whisper / numpy pipeline; never used Chrome. Unchanged. |

No row is classified EXCEPTION at Phase 0. Any row that proves
impossible during the wave is reclassified EXCEPTION with
evidence and reported — it is never quietly left on Chrome.

## 2. Ownership

- **Lead (coordinator)** owns: this repo's `src/*.rs` core
  integration, `Cargo.toml`, workflows; the skill's `SKILL.md`,
  `scripts/selftest.sh`, `scripts/setup.sh`, template structure
  decisions, `CHANGELOG.md` + version stamps; ALL commits in
  this repo; skill file integration; the default-flip commit;
  the GitHub sync.
- **Seats** write NEW files/modules only, at the paths their
  charter names (replacement candidates for existing tool files
  are delivered as complete new files; the lead swaps them in
  at integration). Seats never commit, in either repo.
- Central integration per landing, with the gates of §3.

## 3. Gates (per landing)

- This repo: CI + Tools CI verdicts read at the step level
  (build/test steps are the verdict; the artifact-upload step
  is quota-red and is named as such, never read as a failure of
  the code). Cancelled (concurrency supersession) ≠ failed.
- `FILM TELEMETRY: CHECK OK` at every render-path landing;
  determinism byte-probes intact; the Onoda film's frames
  byte-identical unless a change is defect-attributed and
  reported (the `state_key` fix precedent, `b1355b1`).
- Skill: `LIBRARY OK` (479 items) after any skill change.
- Standing laws: no rendered photos/frames to GitHub (evidence
  local in `~/workspace/talerender-evidence/`, hashes/metrics
  as text); small CI artifacts only; no silent quality
  reduction (final encode stays x264 CRF 18); no fabricated
  results; era pairing fails closed everywhere.

## 4. Acceptance (the definition of done for this mission)

1. Every matrix row (§1) lands in its classification, or is
   reclassified with evidence (EXCEPTIONs named to the user).
2. **Independent Chrome-free proof** (Batch 3): a fresh seat,
   given only the shipped skill, produces a complete NEW short
   with Chrome made unreachable (`CHROME` unset, no Chrome
   binary in `PATH`, bundled-Chrome path absent) — the film
   completes check → cues → frames → sheet → castsheet → video
   → mix → metadata with **zero Chrome launches**, proven by
   the environment log + an exec trace, not by assertion.
3. The Onoda project re-runs through the skill's native path
   end-to-end (after the P1 retrofit), its frames matching the
   established native reference (byte-identical or any delta
   defect-attributed).
4. Failure modes are tested and fail loudly with remedies:
   missing binary, stale binary (era mismatch), unbridged
   project.
5. **Default flip**: a separate, final commit — only if 2–4
   pass. If the proof fails, there is no flip; the failure is
   reported instead.
6. Skill version bump (7.0 recommended — a renderer
   replacement is breaking; rationale recorded in CHANGELOG)
   and the standing per-version GitHub sync
   (`~/workspace/tall-tale-upgrade/sync-skill-to-github.sh`;
   it self-aborts unless the active gh account is
   `turjaun985123` — verified `turjaun985123` at Phase 0; the
   sync's preview exclusion stays as it is).
7. Handover: per-item statuses (REPLACED /
   NOT-CHROME-BY-DESIGN / EXCEPTION / INCOMPLETE), the matrix
   as built, actual batch counts, the proof film's report, the
   flip commit, the version + sync result, and a grep audit of
   every remaining Chrome reference in either repo, classified.

## 5. Batch plan

- **Batch 1 — native completeness** (this repo heavy): §15
  step-2 evaluator + s12 retirement evidence; WP4 streaming
  session (+ WP7 plan consumption); native castsheet; sheet +
  check parity; cues emitter; bridge builder + P4
  generalisation; TS export hardening; WP2 print wiring +
  WP1.2 activation; WP8/WP11 production callers; adapter
  mode-matrix implementation + P3 metadata sidecar.
- **Batch 2 — skill migration** (skill heavy): template
  native-first; SKILL.md rewrite; selftest native end-to-end;
  setup native; library previews; doctor/validation (P2);
  docs set; CHANGELOG + 7.0 preparation; P1 retrofit tooling;
  error surfaces.
- **Batch 3 — proof + hardening**: the independent Chrome-free
  proof; Onoda re-run; failure-mode tests; parity spot-checks;
  hardening seats (bridge edge cases, captions parity,
  castsheet over catalog kinds, fresh-scaffold selftest, docs
  sweep, grep audit); the flip commit LAST; the sync.
