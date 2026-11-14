# ANIM-ADOPT-W32 — staging → clips converter (`stage_clips.py`)

**Seat:** TaleRender Wave 2, W32 (Batch 4 — animation adoption tooling).
**Date:** 2026-11-14. **Status:** tool DONE and proven on fixtures +
the real film data (§5). New files only; nothing committed, no
existing file edited.

**Deliverables**

- `tools/format/stage_clips.py` — stdlib-only converter, house
  style (numbered checks, `OVERALL:` line, exit 0/1/2).
- `tools/format/fixtures/stage_clips/` — 2 positive fragments with
  frozen expected outputs, 3 negative fragments (one defect each).
- This note.

```bash
python3 tools/format/stage_clips.py scenes/film/s03.scene.json -o /tmp/s03.clips.json
python3 tools/format/check_clips.py /tmp/s03.clips.json   # PASS 5/5
```

The output is a standalone clips document — the `clips` block plus
minimal node/camera stubs carrying the `clip` instances, so
`check_clips.py` can resolve track targets. The lead's adoption
step merges the `clips` block into the scene file and drops the
stubs (the real file already has the nodes). Deterministic:
same input → byte-identical output (verified, §5).

## 1. What the input actually is (surveyed, not assumed)

The brief said "camera keys and/or per-character pose/mood steps,
in the shapes the repo actually uses". The repo's staging data is
the scene files' **`channels` audit blocks** (per node, plus a
file-root block in 5 files) and the **§14.4 `camera` block**. A
full scene file is therefore a valid input; a fragment is the same
shapes in a smaller object (`camera`, `nodes`, `duration`,
`segmentStart`). Census of the 14 segment files: **172 channel
records** in 10 shapes, of which 5 carry key data:

| shape (as found in the data) | where | conversion |
|---|---|---|
| `camera.keys` `{t, eye, target, fovY?, ease?}` | all 14 files | eye/target/fovY tracks, 1:1 (§2) |
| poseTrack `{kind, keys: [[t,v],[t,v,m]…]}` (+ nested `{poseTrack:{keys}}` form s05b, `elbowKeys` s09, `elbow` sub-record s11, scalar `elbow`) | headTurn/headTilt/lean/armR/armL | §5.2 hold+arrival (§2) |
| step tables `{kind:"step", keys\|steps: [[t,v]…]}` | mood, legs, lean | step tracks, enum-checked |
| stepAt `{kind:"stepAt", clock, at, from, to}` | s08 mood | two-entry step table |
| segLerp `{kind:"segLerp", segment, from, to}` | position ×5 | two keys, arrival `linear` (§3) |
| position keys `[[t, [x,y,z]]…]` + seg() formula | s06 ×3 | keys, arrivals `linear` (§3) |
| object keys `[{t, value, ease?}]` | node translate ×19, euler ×18 | translate 1:1; euler splits to rotate.x/y/z |

Clocks: channel records spell `clock: "lt"` (or omit it — lt) or
`"film t"`/`"t"`; film-clock records are **rebased** by the
segment start (`shot.at`, or the fragment's `segmentStart`) and a
film-clock record with no segment start in the input fails by
name. poseTrack / object-form / array-form tables are shot-local
by construction in every record the film carries; a film-clock
spelling of one fails by name rather than converting silently.

## 2. The ease-direction remap (the brief's headline)

Three placements exist in this codebase; the conversion touches
two of them:

1. **§14.4 camera keys and W15 clip keys agree**: ease sits on
   the key a segment *arrives* at (scenefile.rs:788 reads it from
   the end key). Camera conversion is therefore a verbatim copy —
   no remap, and saying so is the documentation.
2. **poseTrack is the real remap.** Its `moveSecs` is a property
   of the arrival key describing its *incoming* segment, but the
   segment itself (hold, then move) does not exist in the clip
   grammar until it is spelled as keys. Per W15 §5.2 each
   `[t, v, m]` becomes **two** clip keys — a hold key
   `{t−m, <previous value>}` and the arrival
   `{t, v, ease: "smoother"}` — which evaluates identically to
   `pose_track()` at every t. `[t, v]` keys take the film.rs
   `key()` default m = 0.35; m = 0 becomes `ease: "step"` on the
   arrival. A move window that would start before the previous
   key (pose_track never holds there) fails by name — it does not
   occur in the film data.
3. **anim.rs is the third placement** (easing-to-next, W11 §4.1):
   the loader that feeds `anim` flips format key[i+1].ease →
   anim key[i].ease. That flip belongs to the lead's loader code,
   not to data conversion; the clip files this tool emits stay in
   the format's arriving-key spelling.

Step channels (`mood`/`legs`) carry `ease: "step"` on **every**
key including the first — `check_clips` check 3 requires it.
Steps before the shot window fold into the t=0 key (their value
is the one in force at the cut); steps after the duration drop
(never in force in-shot) — both are reported in the conversion
notes, never silent.

Every emitted number is rounded to the nearest f32 (§11 law —
`check_clips` check 5 rejects computed f64 decimals); the report
states the largest delta applied (≤ 3.9e-07 across the film).

## 3. Two findings the lead should know

**Camera blocks are spelled in two clocks.** The block has no
`clock` field, and the files disagree: s01/s03/s06/s07/s08b/s11/
s12 keys span `0..duration` (shot-local); s02/s04/s05a/s05b/s08/
s09/s10 keys span `shot.at..shot.at+duration` (film time) — each
file's own `time` field corroborates its spelling. The first
census run converted the film-clocked files *wrong* (keys at
t≈40 in a 6 s clip, papered over by duration extension) — the
validator passed them, which is why this is flagged here and not
just fixed: **a green validator run does not prove a conversion
right; the key spans have to be read.** The tool now detects the
spelling from the key span (matching neither window = FAIL),
rebases film-clocked keys, snaps ±1e-5 boundary jitter (f32
transcription vs the f64 `shot.at`), and reports every rebase.

**seg() is linear; one audit record says otherwise.** Position
conversions use arrival ease `linear` because the code of record
does: film.rs `seg()` is the clamped linear factor (film.rs:112)
used verbatim by the s06/s07/s08/s11 drivers, and s11's own law
says "lib.ts seg: clamp-linear, no easing". s07's position law
parenthesises "(smootherstep, motion.ts)" — if the lead rules
the JS original authoritative over the Rust transcription, the
fix is one word per position track; the tool follows film.rs and
says so in its docstring rather than resolving it silently.

## 4. What is deliberately NOT converted (W15 §5.3)

Every non-converted record is accounted **by name** in check 4
of the report, under its §5.3 class; a record matching no class
fails check 4 (no silent drops). The classes, with film counts:

- **blink directors / beats** (28) — seeded procedural system.
- **gait derivations** (23) — walkPhase / walkBob / distKeys /
  position drivers without keys (s02's camK march): functions of
  travelled distance, not independent keys. (Position *endpoints*
  with key data do convert — §1.)
- **fire / flame formulas** (11) — the R1 flicker laws, incl.
  `fireLight.intensity`; baking them would break the R1
  cross-check.
- **visibility windows** (20) — structural instance swaps.
- **talk() waveforms** (2), **camPitchOf** (4) — closed forms /
  camera-derived.
- **pose bundles & swaps** (3) — pose data and bundle switches
  (s09's before/after bundle, s08b's `pose` record, s06's `sit`).
- **prop-hold swaps** (3) — holdL/holdR are attachment state,
  not channels.
- **derived channels** (1) — s11 headTilt = −0.8 × the bow
  track; the bow (lean) converts, the derivation stays code.
- **formula records on ordinary channels** (8) — closed forms
  that happen to sit on animatable channels: the propeller's
  `euler.z` spin (35t), settle() head-turn shakes, s02's legs
  threshold formulas.
- **law-only records** (4) — window formulas over word anchors
  whose numbers live outside the record (s10 headTurn/legs,
  s07/s11 legs).

**Blocked, not code-only (2):** s12's `lean` and `headTilt`
poseTracks carry an `apply` transform (`lean = bowA * 0.6`,
`headTilt = −bowA`) — their keys are the *pre-transform* bow
track, so verbatim conversion would write wrong channel values.
The tool fails the s12 run naming both records. The lead's call:
apply the two affine factors in the staging data, or extend the
tool to apply them deliberately. s12 is the only file that does
not convert (exit 1); its other tracks (mood step, camera) are
unaffected and convert in the same run's (unwritten) report.

Also not tracked, by design: a pair channel's **static elbow**
(s03 armR `elbow` −6 in prose / state) — the value already lives
in the node's `state`; the report notes each occurrence. When
elbow *key data* exists (`elbowKeys`, elbow sub-record) it
converts to the `.1` track (s09, s11).

## 5. Verification (all first-hand)

**Fixtures** (`tools/format/fixtures/stage_clips/`):

| fixture | result |
|---|---|
| `fragment-camera-character` | PASS 4/4 — camera 3 tracks/9 keys; Hero armR.0+armR.1 (hold keys at 1.89/2.96), headTurn 5 keys, mood steps, segLerp position; Guide stepAt rebased 13.5−10.0=3.5; blink accounted. Output == frozen expected file byte-for-byte, and `check_clips` PASS 5/5 |
| `fragment-nodes` | PASS 4/4 — plane translate verbatim (ease preserved), leaflet translate interpolation→linear arrivals, euler → rotate.x/y/z; `visible` accounted code-only. Output == frozen expected, validator PASS 5/5 |
| `bad-unknown-channel` | exit 1, check 2 FAIL naming `nodes[Hero].channels.zoom`; no output written |
| `bad-key-shape` | exit 1, check 2 FAIL naming `keys[1]` ("array of 4"); no output |
| `bad-apply` | exit 1, check 2 FAIL naming the record and its `apply` text; no output |
| missing file / invalid JSON / non-staging object | exit 2, one-line `ERROR:` on stderr, no report |

**Real data** — the brief's required run (one camera move + one
character track): **s03** converts its 4-key crane move to
eye/target/fovY tracks (12 keys, verbatim) and Taniguchi's armR
poseTrack to `armR.0` with the §5.2 hold key at 1.289 (= 1.589 −
0.3); validator PASS 5/5. Full census over all 14 files:

| file | result | tracks | keys | code-only |
|---|---|---|---|---|
| s01 | PASS | 4 | 11 | 2 |
| s02 | PASS | 3 | 6 | 15 |
| s03 | PASS | 4 | 15 | 5 |
| s04 | PASS | 4 | 11 | 8 |
| s05a | PASS | 68 | 434 | 19 |
| s05b | PASS | 8 | 49 | 5 |
| s06 | PASS | 6 | 13 | 6 |
| s07 | PASS | 7 | 17 | 6 |
| s08 | PASS | 6 | 12 | 11 |
| s08b | PASS | 3 | 6 | 6 |
| s09 | PASS | 5 | 16 | 5 |
| s10 | PASS | 6 | 14 | 6 |
| s11 | PASS | 15 | 80 | 11 |
| s12 | **FAIL (apply ×2)** | (4) | (8) | 2 |

All 13 written outputs pass `check_clips.py` — **PASS 5/5,
13/13**. (The counts include the 14 camera blocks → 42 camera
tracks; s12's parenthesised counts are staged-but-unwritten.)

## 6. The honest fraction

Of the film's **staging data** (the universe this tool can
measure): 172 channel records + 14 camera blocks.

- **Converted: 63 records (37%) + 14/14 camera blocks** →
  139 tracks / 684 keys across 13 files.
- **Code-only by §5.3 design: 107 records (62%)** — procedural
  systems (blink/gait/fire), derivations, formulas, structural
  swaps. These were never key data; W15 §5.3 already ruled they
  stay code.
- **Blocked: 2 records (1%)** — s12's `apply` pair (§4).

Of the records that *carry key data* (65), **63 convert
mechanically (97%)**. The boundary of the claim: film.rs also
contains staging that exists only as code with no data record
(the per-shot `shoot()` framings behind the transcribed camera
keys, pose bundles applied imperatively). No fraction is quoted
for that — it is not countable from data, and the clips adoption
does not need it to be: the data side is now mechanical, and the
code side is W12/lead territory per the wave's adoption map.

## 7. Adoption notes for the lead

- Clip names are `<file-stem>-<owner-slug>` (`s03-camera`,
  `s05a-leaflet-3`, …) — deterministic, collision-suffixed.
- **Duration extensions** (reported per clip, never silent):
  genuine past-the-cut moves — s05b armL arrival at 1.590 in a
  1.569 shot, s05b dropped-leaflet keys to 2.44, s10 position
  segLerp ending 5.555 in a 5.283 shot (its own record says the
  walk is unfinished at the cut) — extend that clip's duration
  to its last key; boundary-jitter extensions (~1e-6) are
  labelled as such in the report.
- Suggested tools-ci wiring: run the converter over
  `scenes/film/*.scene.json` into a temp dir and
  `check_clips.py` over the results, tolerating exactly one
  expected failure (s12, until the `apply` decision lands).
