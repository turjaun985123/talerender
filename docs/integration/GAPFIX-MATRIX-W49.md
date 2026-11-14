# GAPFIX-MATRIX-W49 — traceability matrix for the gap-analysis order

**Seat:** W49, Wave 2 Batch 5 (gap-fix batch). **Order traced:** the user's
2026-11-14 23:24 +06 "also do this fixes" on the external *TaleRender ×
tall-tale-shorts* gap analysis (rev 2, probed against the **v1.0.0 release
binary** on the user's PC). **Snapshot:** HEAD `f6616aa` (Batch 4 central
commit), taken while Batch 5 seats W41–W48 are in flight — no Batch-5
artefact is in the tree yet, so every QUEUED row cites the seat assignment,
not a file. CI at snapshot: main CI `43622186461` **SUCCESS**; tools-ci
`43622186445` **FAILURE** (see §D2 — a live item, not a footnote).
*Postscript, same evening:* while this matrix was being written, the
Batch-5 seats' files began landing in the working tree (uncommitted:
FILMJSON-W41, KNOBS-W43, SHEET-W44 + `tools/film/make_sheet.py`,
CUES-W45, EXPORTER-W47, SELFTEST-NATIVE-W48 +
`tools/ci/frame_hash_compare.py`). The QUEUED statuses stand — the work
is in flight and lands formally at the batch close — but "no artefact
in the tree" is no longer literally true and is corrected here rather
than silently edited above.

**Status legend:** CLOSED (done, evidence cited) · STALE (true of the
v1.0.0 binary, not of HEAD — do not re-fix) · QUEUED (accepted; queue
location named) · OPEN (true, not queued anywhere — the rows that matter)
· NOT-ADOPTED (rejected or replaced by a different contract, reason given).

**Method:** every row was checked against git log, source at HEAD, or a
named doc verified against source. The orchestrator's triage (F1–F6 +
companions) was treated as a hypothesis and verified in §A before use.

---

## A. The triage, verified

| Triage item | Verdict on the triage itself |
|---|---|
| F1 `--film` reads `onoda.json`, help says `film.json` → landed `aadc193` | **CONFIRMED.** `aadc193` diff inspected: `load_film_data` (src/film.rs) now prefers `film.json`, falls back to `onoda.json`; `--help` film/caption text corrected in the same commit (also closes W28's F3/F4 wording findings). Scope limit the triage did not state: F1 fixed the *entry filename only* — see G1 for what `--film` still hardcodes. |
| F2 film driver demands Onoda assets unconditionally → W41 | **CONFIRMED still true at HEAD.** `load_assets` (src/film.rs, ~L2053) unconditionally loads `calendar-{1945,1955,1965,1974}.png`, `calendar-dark.png`, `sign.png`, and — whenever `plates.json` parses as an object — all five plates (`paper-map`, `leaflet`, `book-open`, `book-closed`, `coconut`). QUEUED: **W41**. |
| F3 unknown JSON fields silently ignored → W42 | **CONFIRMED still true.** Both loaders consume via `Value::get` throughout (src/scenefile.rs, src/film.rs `load_film_data`); no unknown-key check exists anywhere in `src/`. QUEUED: **W42** (strictness *report* first; making it a hard error is a format decision — §E). |
| F4 no `--res`/`--aa`/`--shadow` knobs → W43 | **CONFIRMED.** `--help` at HEAD lists only `--width`/`--height`; film path hardcodes MSAA-4 (`hdr_msaa` sample_count 4) and a 2048² shadow target (src/film.rs, `aadc193` diff). QUEUED: **W43** (draft-only knobs; defaults stay full quality — the user's no-quality-loss law). |
| F5 no native sheet mode → W44 | **CONFIRMED with a nuance:** the *binary* has no sheet mode; the skill work-copy adapter already has a partial sheet (tiles from native frames, no audit — `capture_native.mjs` L326–373 in `~/workspace/talerender-skill-wiring/`). QUEUED: **W44** (native sheet). |
| F6 cues export stays skill-side, design only → W45 | **CONFIRMED as the standing position.** The work-copy adapter refuses `cues` with a documented rationale (cues are baked into `data/mix.wav`, which the native finish muxes unchanged — `capture_native.mjs` L417). QUEUED: **W45** (design only; no native cues emitter this wave). |
| R0 "two binaries" → W46 v1.0.1 pack, user-confirmed at close | **CONFIRMED as plan.** Cargo.toml still says `1.0.0` at HEAD; no v1.0.1 exists. QUEUED: **W46** + user decision (§E). One sub-item the triage missed: bit-reproducibility — §D5. |
| §7 exporter architecture → W47 | **CONFIRMED.** The analysis's own recommended route (Chrome stays authoring runtime, exporter emits scene/film data) matches the wave's skill-wiring direction. QUEUED: **W47**. |
| G19 selftest → W48 | **CONFIRMED.** QUEUED: **W48**. Note the related live failure in §D2. |

---

## B. Gaps G1–G19

| # | Status | Evidence at HEAD `f6616aa` / queue |
|---|---|---|
| G1 No generic film input | **PARTLY CLOSED, remainder QUEUED (W41) + OPEN residue** | Entry filename CLOSED by `aadc193` (film.json preferred). But the driver remains a single-film program: the shot table is 14 Rust fn-pointers in `eval_frame` keyed to hardcoded anchor names (`thought4`, `decades6`, `coconuts6`…), fps is a literal 30.0 (`frames_total = duration * 30.0`; `t = n / 30.0`), and the title string `"THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER"` is hardcoded in both `run_film` and `run_film_telemetry`. Unconditional assets → W41. A *generic* `--film` (data-driven shots) is not queued in Batch 5 — it depends on the W47 exporter producing driver-consumable data; see R1. |
| G2 Scene format undocumented + hostile | **STALE (documentation + diagnostics), QUEUED residue (W42)** | The doc the v1.0.0 `--help` cited now exists and is substantial: `docs/scene-format.md`, 555 lines, §§1–14.5 (skeleton, camera, lighting, nodes incl. material §4.1, transforms, textures, post, f32 law, format v2). Load-path errors are `Result<_, String>` converted to `TaleError` (errors step 1, `ba88596`) — not exit-101 panics. The analysis's material-schema probe confusion does not reproduce against the documented §4.1. Residue: unknown fields still silently ignored → W42; §15 (prefabs/clips) exists only as the W31 draft, fold-in is the lead's, queued in the Batch-4 packet. |
| G3 No text rasterizer | **NOT-ADOPTED (mechanism) — strips contract stands** | No font stack in the binary, by design: the standing contract is pre-rendered caption strips composited post-grade (user-level ruling recorded in MEMORY 2026-11-14; the film driver header states "the renderer typesets nothing"). Strips for the reference pipeline are rendered by `film/gen_assets.py` (PIL + the project's own Inter fonts) — not by Chrome. The *outcome* for the reference film is CLOSED (G4 row). A native rasterizer is not queued and should not be re-proposed. |
| G4 No word-timing / chunking parity | **PARTLY CLOSED (reference film), QUEUED for generic (W47), one OPEN sub-item** | `film/gen_film_data.py` transcribes `buildCaptions()` over the project's real `script.json` + `data/lines.json` + `data/words.json` (Wd() anchors, caps overrides, hold rule), and Wave 2's re-chunk (`3a36f1f` + `5e90d7e`) made the gate print `FILM TELEMETRY: CHECK OK` on the 73-chunk layout. **OPEN sub-item the triage missed:** the transcription covers only *authored* `caps` — it raises `SystemExit("this film's script always sets caps; autoChunks not transcribed")`, so the auto-chunk rule (short function word rides the next, ≤ 11 chars) has no native-side equivalent. Any auto-chunked project (e.g. headless-chicken) cannot be data-generated today. Recommend: fold into W47's exporter scope or a Batch-6 seat. |
| G5 No audio path / no cues | **QUEUED (W45, design only) + NOT-ADOPTED (native cues this wave)** | The binary still emits PNG frames only and no `cues.json`. The pipeline position (adapter, work copy): cues are Chrome-extracted once and baked into `data/mix.wav`; the native finish muxes that WAV unchanged, so the native *render* path does not consume cues at all. A fully Chrome-free authoring flow would still need cues — that is exactly what W45 designs. `mix.py` itself is untouched, per the analysis's own non-goal. |
| G6 No shot model / duration source | **QUEUED (W47) + NOT-ADOPTED (native shot model)** | At HEAD: duration enters via `film.json`'s `duration` key (for the reference film, `gen_film_data.py` derives it from `lines.json`); shots are code, not data (G1). The wave's architecture — following the analysis §7 — is an *exporter* that emits driver data, not a native `shots.ts` interpreter or native shot model. W47 owns the exporter design; until it lands, R2/R6-style parity for new projects is unproven. |
| G7 `check` has no equivalent | **PARTLY STALE, remainder OPEN — not queued** | STALE half: "always exits 0" is false since `aa10b13` — film-telemetry exits 0 (CHECK OK) / 1 (CHECK FAILED) / 3 (could-not-run), verdict line verbatim and authoritative; the adapter was re-proven against the real binary in both directions (WAVE2-RECORD §7). The native gate prints overflow, edge-touch, continuity, composition and the caption audit (src/telemetry.rs `print_audit`/`print_continuity`/`print_composition`/`print_caption_audit`). **OPEN remainder:** film telemetry prints **no shot table, no full caption table, and no cast list** (the shot list and caption infos are internal; only summaries + failure lines print), and it runs only on the film driver's own data. No Batch-5 seat owns this. Recommend: lead fixes pass or a Batch-6 seat; it is the last piece of R6. |
| G8 No cast/continuity source | **STALE for the reference film; generic case QUEUED (W47)** | The native gate computes `sig` itself: `look_signature()` (src/telemetry.rs, transcribed from cast.ts) over the driver's own look registry (`tel_look`/`look_registry`, src/film.rs), tracked per instance id (`INSTANCE_IDS`). Continuity is not fed external sigs. For arbitrary projects the cast/look source is exporter-supplied data — W47. |
| G9 Characters: painter exists, nothing feeds it, state static | **STALE (film driver), QUEUED (generic + scene-file lane adoption)** | "Static until proven otherwise" is disproven for the film driver: per-frame state is evaluated in code — `pose_track`, `walk_phase`/`walk_bob`, `blink_of`, mood/arm/leg channels — the Wave-1 film *is* the proof (A/B mean 23.02 dB). Two honest residues: (1) the scene-file lane's character node (§14.3) carries one look/state per file; the data-driven channels (`src/anim.rs`, `src/charanim.rs`) are registered modules with **zero call sites** at HEAD, and `src/locomotion.rs` / `src/cameramoves.rs` are not yet registered as modules at all — adoption is Phase-4 lead work, A/B-gated (WAVE2-RECORD §4). (2) Generic per-cutout spec export → W47. |
| G10 No set kit / atmospheres | **PARTLY STALE, remainder QUEUED (W47)** | The format grew a set vocabulary in Wave 1: `scenes/atmospheres.json` + `scenes/ATMOSPHERES.md` (preset-named atmospheres, ruling R5), `scenes/sets/`, generator node kinds in the loader (scatter §4.3, code-ref §4.4, registry painters), and the reference film's sets are fully built in the driver. What does not exist is a TS `world.ts`/`props.ts` *exporter* — W47. Porting the kit natively was the analysis's expensive option and is NOT-ADOPTED in favour of export. |
| G11 No camera rigs / motion vocabulary | **PARTLY STALE, remainder QUEUED (W47 / lead adoption)** | Format §14.4 camera keys exist with the house easing law (smootherstep by default; `"ease": "linear"` on the destination key) and are consumed by `eval_camera_keys` (src/scenefile.rs). `src/cameramoves.rs` (W25: dolly/pan-tilt/orbit/tracking/seeded handheld, 11/11 tests) exists but is unregistered and unconsumed at HEAD. The analysis's cheaper fix — exporter samples rigs into keys — is the W47 route; W32's staging tool already converts 63/65 key-carrying records (Batch 4, `f6616aa`). |
| G12 Quality knobs fixed | **QUEUED (W43)** | See §A/F4. `--width`/`--height` exist (absolute res, not a scale); no `--res`, `--aa`, `--shadow`, or `--draft` at HEAD. W43 owns all four, draft-only, defaults unchanged. |
| G13 Per-shot look not demonstrated | **STALE in substance; transitions NOT-ADOPTED (no contract)** | Per-shot look exists two ways: format-side, each scene file carries its own `post` block (§9/§14: grade keys sat/warm/exposure/time + vignette/grain, bokeh, bloom — enforced by W26's validator, 6/6 over all 14 real scenes); driver-side, each shot function sets its own `FrameSpec` post. The ab1 `exposure = 1.0` the analysis cites is the §28 reference scene's fixed staging, not a pipeline limit. Cross-shot look *transitions* (blended grades) exist in neither engine's contract — Chrome grades per shot discretely too — so none is queued; flagged here so the row is a decision, not an oversight. |
| G14 No frame fade | **STALE — CLOSED in the film driver** | Found by direct source read (the triage missed this one): `eval_frame` computes `fade_factor = clamp(t/0.25).min(clamp((duration−t)/0.4))` — fade-in 0.25 s, fade-out 0.4 s, exactly the `main.ts` values — and `run_film` applies it via `apply_fade` after the shot draw, before captions (main.ts's order). The analysis's "[inferred from code]" was wrong for the film path. (The single-scene `--scene-file` lane has no film fade — correct, it renders one frame.) |
| G15 No sheet / frames / preview | **MIXED: frames CLOSED, sheet QUEUED (W44), preview OPEN (by design)** | Frames: `--film --from/--to` renders PNG ranges with resume, and the work-copy adapter exposes frames/sheet/video modes over them. Sheet: binary mode → W44; adapter partial sheet exists (work copy). Preview: no native equivalent and none queued — the adapter refuses it as Chrome-only (a live page is inherently the browser's). The analysis itself rates preview "nice-to-have" (§5 row 14). Recommend: leave refused-by-design; revisit only if the user asks. |
| G16 No deliverable stage | **CLOSED for the reference pipeline (one deviation, §D3)** | `film/finish.sh` wraps the PNGs with the skill's finishing settings: x264 `-preset slow -crf 18`, all three BT.709 tags **plus** the x264 VUI params that make them land (`6ca97fc`), AAC, `+faststart`, `.part` discipline, `set -euo pipefail` (fails loudly). Proven end-to-end: the delivered proof MP4 (1,811 frames, 60.457 s, −14.00 LUFS, all-bt709 probe). It muxes the project's existing `data/mix.wav` unchanged — ffmpeg is wrapped, not reimplemented, as the analysis prescribes. Generic projects inherit this once W47 supplies frames. |
| G17 Operational readiness | **MIXED — see R0/R9 rows** | Source in VCS: STALE-as-claimed (repo `turjaun985123/talerender`, this matrix is written inside it). Panics: CLI surface CLOSED (`d7bebdd`, exit 2, no panics on bad input); load paths CLOSED (step 1, `ba88596`); GPU-init/readback `expect()`s remain — collapsed by the gpucore migration to gpucore's own sites (`src/gpucore.rs`: adapter L80, device L103, map L295, save L331–332) — QUEUED as errors step 2, re-keyed in GATE2-W40, owner lead fixes pass. `--help` accuracy: F1/F3/F4 wording fixed (`aadc193`); one residue in §D4. Two-binaries/version: QUEUED W46 + user decision. Self-test: G19. |
| G18 GPU portability | **OPEN (evidence, not code) — owner: user's-PC session** | RADV execution is proven (the analysis itself ran the binary on RADV; every native frame of the reference film is reproducible on CI lavapipe, and determinism E6 PASSed on CI lavapipe, run `43617370962`). What has never been measured: a lavapipe-vs-Chrome-SwiftShader *timing* comparison, and any RADV perf numbers (PERF.md tables blank by design). The instruments exist — bench harness (W17), byte-probe pack (W34), consolidated PC evidence runbook (W36) — but the run itself can only happen on the user's PC. Not a Batch-5 seat; carried on the wave-close checklist (§E). |
| G19 No self-test | **QUEUED (W48)** | The shipped skill's `selftest.sh` proves the Chrome path only; the work copy carries a native selftest with SKIP/PASS/FAIL/ENV-BLOCKED classes (Wave 1). W48 owns the native-path selftest extension pack (incl. a Chrome-vs-native frame comparison for one project, per R10). Related live evidence: §D2. |

---

## C. Requirements R0–R10

| # | Status | Evidence / queue |
|---|---|---|
| R0 Source and build (blocking) | **PARTLY CLOSED; QUEUED (W46) + OPEN sub-item (§D5)** | Source in version control: CLOSED (private repo, full history, this wave's 100+ commits). One canonical binary + behaviour-changing version: QUEUED — W46's v1.0.1 pack (version bump from HEAD, single CI-built asset, checksum), released only on the user's go (§E). The analysis's accept test — *build from source twice → identical sha256* — is **not met and not queued**: there is no `Cargo.lock` in the repo (CI generates it), so bit-reproducible builds are unproven. §D5. |
| R1 Generic film input | **QUEUED (W41 + W47); F1 slice CLOSED** | `film.json` is now the entry (`aadc193`). "No `onoda*` present" acceptance is not yet reachable: unconditional assets (W41) and the code-baked shot table/title/fps (G1) mean a second project still cannot run. The generic route is W47's exporter emitting driver data; whether the driver itself becomes data-driven is a post-W47 lead decision — flagged OPEN as a decision, not lost. |
| R2 Timeline from the skill's data | **PARTLY CLOSED (reference film); QUEUED (W47); auto-chunk OPEN (G4)** | For Onoda, the timeline *is* computed from `script.json`/`lines.json`/`words.json` by `gen_film_data.py` — no hand-typed timings. The binary itself reads only the generated `film.json`; the analysis's accept test (telemetry caption table identical to `capture check` on headless-chicken) cannot pass today: auto-chunks are untranscribed (G4) and telemetry prints no full caption table (G7). Both halves have owners only via W47/G7-recommendation — the chicken half is honestly OPEN. |
| R3 Captions rendered | **NOT-ADOPTED as specified; outcome CLOSED for the reference film** | The requirement's mechanism (native rasterizer) is rejected under the strips contract (G3). The outcome bars are met for Onoda by the strip pipeline: chunks measured from the strips' own ink, gate `CHECK OK` at the 73-chunk layout (`5e90d7e`), pop 0.86→1 over 0.1 s implemented in `eval_frame`, fit floor enforced by the audit (split, never shrink). The literal accept test (frame PSNR ≥ 40 dB vs the Chrome frame at a chunk start) has never been run as stated — the film A/B (mean 23.02 dB) is whole-frame and not caption-isolated. Flagged honestly: if the user wants the letter of R3, it needs a caption-isolated A/B — not queued. |
| R4 Characters | **CLOSED for the reference film; QUEUED for generic (W47 + lead adoption)** | Native painter + per-frame state (G9) proven on the reference film: projected boxes and overflow are the telemetry gate's own checks (`CANVAS OVERFLOW: none`, continuity clean, CHECK OK). The ±2 px silhouette accept test for an arbitrary two-character scene awaits exporter-fed specs (W47) and the anim/charanim adoption (lead, A/B-gated). |
| R5 Sets | **CLOSED for the reference film; QUEUED for generic (W47)** | `the-soldier-who-didnt-know` renders end-to-end natively (1,811 frames, A/B 23.02 dB mean vs Chrome). `headless-chicken-v4fixed` does not — it is blocked on the same exporter + auto-chunk pair (W47, G4). The analysis's ≥ 38 dB sequence bar for chicken is therefore OPEN until W47 lands; Onoda's 23.02 dB is the *calibration* number (headless-gl band 15.8–19.3 dB), not a claim against that bar. |
| R6 QA gate parity | **PARTLY CLOSED; remainder OPEN (G7)** | Exit-code parity CLOSED (`aa10b13`); overflow/edge/continuity/composition/caption-audit parity CLOSED for the film gate (src/telemetry.rs is a deliberate transcription of capture.ts's printers). Deliberate-break acceptance (hide an actor → reported, non-zero exit) holds for the classes the gate prints. Missing: shot table, full caption table, cast list (G7) — OPEN, unqueued, recommended to the lead fixes pass. Boot-error class is Chrome-specific; its native analogue (load failure → exit 3, no verdict line) exists. |
| R7 Review loop | **QUEUED (W43 + W44); preview OPEN-by-design (G15)** | `--res`/`--aa`/`--shadow`/`--draft` → W43. `--sheet` → W44 (adapter partial sheet already exists in the work copy; the analysis's accept shape — 61-frame contact sheet, fast — is the W44 target). Frames mode CLOSED (G15). Preview stays Chrome (adapter-refused, documented). |
| R8 Audio + deliverable | **CLOSED (pipeline) + QUEUED (cues design, W45)** | Deliverable half CLOSED via `film/finish.sh` (G16) with one deviation (§D3). Cues half: `mix.py` unchanged and fed by Chrome-extracted cues baked into `mix.wav` today; the native-cues question is W45's design seat. The R8 accept test (ffprobe shape of the deliverable) is already proven on the proof MP4. |
| R9 Ops | **QUEUED (W42 + W46 + lead errors step 2)** | Meaningful errors: CLI + load paths done; GPU/readback step 2 re-keyed and owned by the lead (G17). Unknown fields as errors: W42 reports first; the hard-error call is a format decision for the close (§E note). `--help` matches reality: F1-fixed, one residue (§D4). Canonical binary: W46 + user go. |
| R10 Selftest | **QUEUED (W48)** | See G19. W48's pack must make `selftest.sh` pass with the native renderer selected, including the one-project frame-hash comparison against Chrome. Until then the native path has gate evidence (telemetry, validators, CI) but not selftest evidence. |

---

## D. Items the triage missed

| # | Item | Status / evidence |
|---|---|---|
| D1 | **G14 frame fade is already implemented** (in/out 0.25 s / 0.4 s in `eval_frame` + `apply_fade`). The analysis inferred its absence; the triage repeated the gap list without checking. | CLOSED — see G14. No seat needed. |
| D2 | **tools-ci is RED at HEAD.** The W39 wiring (`630d82e`) added the asset checker to tools-ci; it passes locally but fails on the runner: run `43622033396` (wiring commit) and `43622186445` (HEAD `f6616aa`) both FAILURE — manifest-mode `check_assets.py` reports `OVERALL: FAIL (4/5)` while driver mode passes 6/6 in the same run. The manifest carries absolute `/home/hatch/...` paths (`project`, `audio.projectPath`) that exist on this VM — which is why W39's local verification passed — and not on the runner. Main CI is green (`43622186461`), so this is a tools-ci-only red. | **OPEN — owner: lead fixes pass.** Fix shapes: make the manifest-mode existence check treat authored absolute paths as portability warnings (the checker's own check-4 philosophy), or give tools-ci a fixture manifest. Do not "fix" by deleting the step — it caught a real portability fact. |
| D3 | **AAC bitrate deviation in the deliverable.** `film/finish.sh` encodes `-b:a 192k`; the skill master's setting per the analysis (§2.1/G16) is AAC 256k. The delivered proof MP4 therefore differs from the skill master in exactly one encode parameter. | **OPEN (micro) — owner: lead.** Either align finish.sh to 256k or record the deviation as deliberate in MP4-EVIDENCE; silent drift in a "settings verbatim" claim is the failure mode, not the 64k itself. |
| D4 | **`--help` still over-promises scene resolution.** Post-F1 help says film "segment scenes resolve from scenes/film/" — but `src/film.rs` contains no `scenes/film` reference at all: the driver evaluates its shots in code; `scenes/film/` is the manifest/validator convention (tools/film/validate_film.py). The sentence describes the wrong component. | **OPEN (micro) — owner: lead** (one-line help correction, same class as the F3/F4 fixes in `aadc193`). |
| D5 | **No `Cargo.lock` → R0's reproducibility accept test unmet.** The repo carries no lockfile; CI generates one per run. Two source builds are therefore not guaranteed identical, and "build twice → identical sha256" has never been demonstrated. | **OPEN — owner: lead.** Recommend committing the CI-generated lockfile (zero-deps law makes it small) or explicitly documenting nondeterminism in the v1.0.1 pack (W46) so the user decides with the fact in view. |
| D6 | **Auto-chunk transcription gap** in `gen_film_data.py` (authored caps only; `SystemExit` on auto-chunked scripts). | OPEN — see G4. Recommend folding into W47's scope. |
| D7 | **Unregistered/unconsumed animation modules.** `src/locomotion.rs` and `src/cameramoves.rs` are not in `src/main.rs`'s mod list at HEAD; `anim`/`charanim` are registered but have zero call sites. A reader of the tree can easily over-claim "animation system landed". | QUEUED (lead adoption, Phase 4, A/B-gated) — recorded here as an honesty guard for the close-out report. |
| D8 | **Telemetry table outputs** (shot table / full caption table / cast list) absent from film telemetry. | OPEN — see G7/R6. Recommend lead fixes pass or Batch 6. |

---

## E. Wave-close checklist for this order

### Show the user as DONE (each with its proof)
- [ ] F1: `--film` takes `film.json` (onoda.json fallback) and `--help` agrees — `aadc193`.
- [ ] Captions: 73-chunk re-chunk live; `FILM TELEMETRY: CHECK OK`, exit 0 — `5e90d7e` (supersedes the Wave-1 waiver; veto note below).
- [ ] Telemetry exit codes 0/1/3 with the verdict line authoritative — `aa10b13`; adapter re-proven both directions.
- [ ] Determinism: root cause fixed (`cabcc30`); CI-lavapipe pack PASS — run `43617370962`.
- [ ] Deliverable stage: `film/finish.sh` + proof MP4 (BT.709 all three tags, −14.00 LUFS) — `6ca97fc` + MP4-EVIDENCE. (With the D3 asterisk.)
- [ ] Frame fade, per-shot look, native continuity sig, dynamic character state in the driver — all present in source (G8/G9/G13/G14); list them as *stale claims corrected*, not new work.
- [ ] Batch-5 seats W41–W45, W47, W48 as they land, each with its gate/CI verdict at its batch close (files began landing during this matrix's writing — see the header postscript).

### Still OPEN at close, with owners
| Item | Owner | Row |
|---|---|---|
| Generic second-project render (data-driven `--film`) | post-W47 lead decision | G1/R1 |
| Auto-chunk transcription (headless-chicken class) | W47 scope or Batch 6 | G4/D6 |
| Telemetry shot/caption/cast tables | lead fixes pass or Batch 6 | G7/D8 |
| tools-ci red at HEAD (manifest absolute paths) | lead fixes pass | D2 |
| AAC 192k vs 256k alignment | lead (one line or one doc note) | D3 |
| `--help` scenes/film wording | lead (one line) | D4 |
| Cargo.lock / reproducible-build proof | lead; fact surfaced in W46 pack | D5/R0 |
| Errors step 2 (gpucore `expect()`s → exit 3/4) | lead fixes pass (re-keyed, GATE2-W40) | G17/R9 |
| §15 fold-in + anim/charanim/locomotion/cameramoves adoption | lead, A/B-gated | G9/G11/D7 |
| RADV perf + determinism RADV leg | **user's-PC session** (packs ready: W17/W34/W36) | G18 |
| Caption-isolated PSNR ≥ 40 dB accept test (R3 as literally written) | unqueued — only if the user wants the letter of R3 | R3 |

### The two user decisions embedded in this order
1. **v1.0.1 release — go / no-go.** W46 prepares the pack (version bump
   from HEAD, one canonical CI-built binary + checksum, replacing the
   two divergent 1.0.0 copies the analysis found). Nothing releases
   without the user's word; the published v1.0.0 binary predates even
   the professional CLI, so this decision also decides what "the
   binary" means for the user's PC copies.
2. **Caption re-chunk — confirm or veto.** The user's completion order
   reopened the lead's ACCEPT-WITH-WAIVER ruling and the re-chunk is
   now live in the data (gate CHECK OK). At close the user confirms it
   stands, or vetoes and the 66-chunk waiver state is restored from
   git history (WAVE2-RECORD §5 records both branches).

**Cross-reference, deliberately not conflated:** the **s28 golden pin
disposition** (and the 14 film first-pins) are *standing Wave-1/2 items*,
not decisions created by this gap order — the s28 packet recommends
against re-pinning, the pins remain two separate human acts
(GOLDEN-RESTAGE-W03), and the first visual-regression dispatch is
expected to fail on exactly those stale pins until they are struck.
They belong on the wave's general close list, not this order's.

*Format-decision note (not a user decision):* whether W42's strictness
report becomes a hard error on unknown JSON fields is a lead/format
call at adoption time, per the triage — recorded here so it is not
mistaken for a third user decision.
