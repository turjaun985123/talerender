# AUDIT-W40 — Independent audit (red team) of the Onoda film data: s03, s07, s11

Seat W40, TaleRender 100-agent wave. Method: every sampled value was
recomputed first-hand from the project sources
(`~/faceless-shorts/the-soldier-who-didnt-know/src/`: `shots.ts`,
`core/world.ts`, `core/env.ts`, `core/toon.ts`, `core/motion.ts`,
`core/poses.ts`, `core/lib.ts`, `core/engine.ts`) plus
`film/onoda.json` and the project's `data/lines.json` / `data/words.json`.
The seats' `scenes/film/ASSUMPTIONS.md` sections were read only **after**
the computations, to compare derivations (see §5).

Tolerance: scene files store f32-rounded values. A file value within
f32 rounding of the f64 recomputation (≤ ~2×10⁻⁷ relative, or a value
the seat rounded to 4 decimal places that rounds back to the
recomputation) is a MATCH. Anything else is a MISMATCH.

## Summary

| Segment | Sampled | MATCH | MISMATCH |
|---|---|---|---|
| s03 | 30 | 29 | **1** — `post.grade.time` |
| s07 | 30 | 28 | **2** — `post.grade.time`; caption `cap_039` omitted |
| s11 | 30 | 30 | 0 |
| **Total** | **90** | **87** | **3** |

Plus one cross-cutting consumption defect (§4): in s07 and s11 the
grade saturation is written under the key `saturation`, which the
scene-file loader does not read — the values are correct, the key is
not the one consumed.

---

## 1. S03 — `scenes/film/s03.scene.json` (SH noon, film t 13.061–15.950, file `time` = lt 2.0)

Ground truth recomputed first: `cut(i) = lines[i].start − 0.06`;
st = cut(2) = 13.121 − 0.06 = 13.061, end = cut(3) = 16.01 − 0.06 =
15.95, d = 2.889. `Wd('never',2)` = 14.78, `Wd('surrender',2)` = 15.02
(words.json). Taniguchi's armR track keys end at lt = 14.78 − 13.061 −
0.13 = 1.589; talk window lt < 15.02 − 13.061 + 0.35 = 2.309.

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 13.061 | cut(2) = 13.121 − 0.06 = 13.061 | MATCH |
| 2 | `shot.duration` | 2.889 | cut(3) − cut(2) = 15.95 − 13.061 = 2.889 | MATCH |
| 3 | `lights.key.color` | `#FFF0D6` | env.ts noon `keyCol` #FFF0D6 | MATCH |
| 4 | `lights.key.intensity` | 2.6 | noon `keyI` 2.6 | MATCH |
| 5 | `lights.key.position` | [−8, 11, 7] | SH `sunlight()` pos; noon carries no `keyPos`, so the set's pos survives | MATCH |
| 6 | `lights.key.shadow.far` | 80 | `sunlight()` overrides lightRig far 40 → 80 | MATCH |
| 7 | `lights.key.shadow.bias` | −0.0006 | `sunlight()` overrides bias −0.0004 → −0.0006 | MATCH |
| 8 | `lights.rim.intensity` | 0.4 | noon `rimI` 0.4 | MATCH |
| 9 | `lights.hemi.sky` | `#CFE6FF` | noon hemi sky #CFE6FF | MATCH |
| 10 | `lights.hemi.intensity` | 1.25 | noon hemi intensity 1.25 | MATCH |
| 11 | `lights.envIntensity` | 0.25 | lightRig default environment intensity 0.25 (no preset overrides it) | MATCH |
| 12 | `camera.keys[0].eye` | [0.2, 2.0, 8.4] | S03 `shoot()` from (shots.ts) | MATCH |
| 13 | `camera.keys[last].eye` | [0.05, 1.85, 7.2] | S03 `shoot()` to | MATCH |
| 14 | `camera.fovY` | 54 | S03 look fov 54 | MATCH |
| 15 | `camera.keys[1].t` | 0.2889 | camK hold 0.1 × d 2.889 = 0.2889 (motion.ts camK) | MATCH |
| 16 | `post.bokeh.focus` | 6.768082141876221 | camK(2.0, 2.889) = 0.8860474 → eye (0.06709, 1.86709, 7.33674); \|eye − (0, 1.22, 0.6)\| = 6.7680823391 (Δ = f32 rounding) | MATCH |
| 17 | `post.bokeh.aperture` | 0.0012 | G.shore aperture (shots.ts grade registry) | MATCH |
| 18 | `post.bloom.strength` | 0.16 | G.shore bloom 0.16 | MATCH |
| 19 | `post.grade.sat` | 0.84 | G.shore sat 0.84 (shore look is used verbatim, no gradeOf) | MATCH |
| 20 | `post.grade.time` | 15.060999870300293 | **0** — see Mismatch M1 | **MISMATCH** |
| 21 | `clear` | `#E4EEDC` | SH sky horizon #E4EEDC (sky:false keeps the set sky) | MATCH |
| 22 | `captions[cap_018].s/.e` | 13.15 / 13.61 | film/onoda.json chunk 18: s 13.15, e 13.61, "His orders" | MATCH |
| 23 | `captions[cap_020].e` | 16.01 | onoda.json chunk 20: e 16.00999… ("never surrender.") | MATCH |
| 24 | `characters[taniguchi].position` | [−1.05, 0, 1.0] | S03 staging (shots.ts) | MATCH |
| 25 | `characters[taniguchi].state.armR` | [96, −6] | poseTrack([[0,68],[1.589,96,0.3]], lt 2.0) = 96 (move long finished); elbow static −6 (POSES.explain override in S03) | MATCH |
| 26 | `characters[taniguchi].state.talk` | 0.38780462741851807 | talk(15.061, on) = 0.25 + 0.75·\|sin(13·15.061)\|·(0.6 + 0.4·sin(5.3·15.061)) = 0.3878046421 (Δ = f32 chain) | MATCH |
| 27 | `characters[taniguchi].channels.blink.directorSeed` | 9 | S03 set(): `blinkDirector(9)` | MATCH |
| 28 | `characters[onoda].state.blink` | 0 | max(blinkDirector(3)(15.061), blinkAt(15.061, [14.78])) = max(0, 0) = 0 (director interval walk: x−acc = 0.5 ≥ 0.12) | MATCH |
| 29 | `nodes[boat].transform` | translate [−4.6, 0, −7.5], rotate_y 1.15 | SH boat group (shots.ts) | MATCH |
| 30 | `nodes[hill 0].transform` | translate [−40, −3, 55], scale [34, 12, 20.4] | world.ts hills(z=55) list[0] = [−40, 55, r 34, h 12]; pos y = −h·0.25 = −3; scale (r, h, 0.6r) | MATCH |

### Mismatch M1 — `post.grade.time` (s03)

- File: `15.060999870300293` (film t at the file's evaluation instant).
- Correct: **0**.
- Source: `core/engine.ts:55` — `grade.uniforms.uTime.value =
  look.t ?? 0`. No shot in `shots.ts` ever passes a `t` in its look
  object (verified across all 14 shot definitions), so the GRADE
  shader's `uTime` is 0 for every frame of the film. Ruling R3
  (docs/scene-format.md §14.5) independently pins GRADE grain time 0.
- This is live, not inert: the scene-file loader reads the field
  (`src/scenefile.rs:976`, default 0.0) and passes it into the
  composer's GradeParams (`src/scenefile.rs:2051`). A render driven
  by this file seeds the grain hash with uTime = 15.061 — the source
  GRADE hash is `fract(sin(dot(p, …) + uTime·7.13)·43758.5453)`
  (engine.ts:22) — so the grain pattern on every S03 frame would
  differ from the Chrome reference. (The hardcoded `src/film.rs`
  path pins `time: 0.0` at film.rs:3233 and is unaffected; the
  scene-file path is the one these files exist for.)

## 2. S07 — `scenes/film/s07.scene.json` (JG golden, film t 30.39–34.209, file `time` = lt 0)

Ground truth recomputed first: st = `Wd('decades',6)` − 0.25 =
30.64 − 0.25 = 30.39; end = cut(7) = 34.209; d = 3.819;
calT0 = st + 0.16 = 30.55; `Wd('coconuts',6)` = 32.4.

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 30.39 | Wd('decades',6) − 0.25 = 30.64 − 0.25 = 30.39 | MATCH |
| 2 | `shot.duration` | 3.819 | cut(7) − st = 34.209 − 30.39 = 3.819 | MATCH |
| 3 | `lights.key.color` | `#FFB35C` | env.ts golden keyCol | MATCH |
| 4 | `lights.key.intensity` | 2.3 | golden keyI 2.3 | MATCH |
| 5 | `lights.key.position` | [−8, 4.5, 5] | golden `keyPos` [−8, 4.5, 5] (preset moves the key) | MATCH |
| 6 | `lights.hemi.ground` | `#B07A48` | golden hemi ground | MATCH |
| 7 | `lights.hemi.intensity` | 1.0 | golden hemi ×1.0 | MATCH |
| 8 | `lights.points[fire].channel.level` | 0 | S07 stages `fireLevel(0, t)` (fire out) | MATCH |
| 9 | `lights.points[fire].channel.formula` | `(4.6 + 0.7*sin(17t) + 0.5*sin(7.3t)) * level` | world.ts fireLevel law; base 4.6 is the fireLevel basis (R1 — never the constructor's 5.0) | MATCH |
| 10 | `fog.color` | `#FFD98A` | golden fog colour | MATCH |
| 11 | `fog.far` | 140 | golden fog 30/140 | MATCH |
| 12 | `camera.keys[0].eye` | [1.4, 1.6, 3.4] | S07 crane from | MATCH |
| 13 | `camera.keys[last].target` | [1.8, 0.8, −2.4] | S07 crane lookTo | MATCH |
| 14 | `camera.fovY` | 46 | S07 look fov 46 | MATCH |
| 15 | `post.focusChannel.atStart` | 5.2345 | \|(1.4,1.6,3.4) − (1.4,1.0,−1.8)\| = √27.4 = 5.2345009 | MATCH |
| 16 | `post.focusChannel.atEnd` | 10.7963 | \|(3.6,5.4,7.2) − (1.8,0.8,−2.4)\| = √116.56 = 10.7962957 | MATCH |
| 17 | `post.grade.saturation` (value) | 0.85 | gradeOf(G.jungle, golden).sat = 1.15, then S07 clamps sat to min(1.15, 0.85) = 0.85 — value correct; key name see §4 | MATCH |
| 18 | `post.grade.exposure` (value) | 1.02 | golden grade exposure 1.02 — value correct; key see §4 | MATCH |
| 19 | `post.grade.time` | 30.39 | **0** — see Mismatch M2 | **MISMATCH** |
| 20 | `clear` | `#FFD98A` | golden sky horizon (S07 does not pass sky:false; sky re-dressed) | MATCH |
| 21 | `captions[cap_040].s/.e` | 31.55 / 32.99 | onoda.json chunk 40: s 31.55, e 32.99, "surviving on coconuts", width 989.95, fit 0.96974 | MATCH |
| 22 | `captions` completeness — chunk 39 | **absent** | chunk 39 "for decades," spans 30.29–31.55 (onoda.json) and is the active caption for the first 1.16 s of the shot — see Mismatch M3 | **MISMATCH** |
| 23 | `characters[onoda].position` | [−0.5, 0, −1.76] | walk from; wk = seg(0, 0.2, 1.85) = 0 | MATCH |
| 24 | `characters[onoda].state.mood` | `tired` | lt 0 < 1.2 branch (shots.ts S07) | MATCH |
| 25 | `characters[onoda].state.armR` | [12, −10] | reach poseTrack at lt 0 = first key 12; elbow static −10 | MATCH |
| 26 | `characters[onoda].state.camPitch` | 0.1108 | camPitchOf (toon.ts:720): atan2(1.6 − 1.75·0.6, hypot(1.9, 5.16)) / 0.9 = 0.11076914 | MATCH |
| 27 | `characters[onoda].channels.armR.keys[1]` | [1.91, 136, 0.3] | key time = Wd('coconuts',6) − st − 0.1 = 32.4 − 30.39 − 0.1 = 1.91 | MATCH |
| 28 | `characters[onoda].channels.holdL.atFilm` | 32.35 | coconut appears at Wd('coconuts',6) − 0.05 = 32.35 | MATCH |
| 29 | `overlays[calendar].t0Film` | 30.55 | calT0 = st + 0.16 = 30.55 (s07-calendar.json agrees: life 4.4, alpha law, card 320×400 at translate(352,648) rot −0.03 scale 0.92) | MATCH |
| 30 | `nodes[palm role=walk-target].transform` | translate [4.6, 0, −4.6], scale 1.05, rotate_z 0.14 | shots.ts palm(1.05, 0.14) placed at (4.6, 0, −4.6) | MATCH |

### Mismatch M2 — `post.grade.time` (s07)

- File: `30.39` (film t at shot start). Correct: **0**.
- Same source citation as M1 (engine.ts:55, `look.t ?? 0`; no shot
  passes `t`; R3 pins 0). Same live-consumption path
  (scenefile.rs:976 → :2051): every S07 frame's grain would be
  seeded with uTime = 30.39 instead of 0.

### Mismatch M3 — caption chunk `cap_039` omitted (s07)

- File: `captions` contains only cap_040 and cap_041.
- Source: film/onoda.json chunk 39 = line 6, "for decades,"
  s 30.29, e 31.55. Caption display is a film-time overlay: the
  chunk active at time t is drawn. From the S07 cut (t = 30.39)
  until 31.55 the active chunk is cap_039 — it is on screen for the
  first 1.16 s of the segment, and the file does not list it.
- The wave's own convention confirms the omission is an error, not
  a choice: s11 lists cap_056, which bleeds only 0.042 s into its
  head, and s03 lists cap_020, which overruns its tail. W09's own
  ASSUMPTIONS section for S07 also names cap_039 among the shot's
  captions ("Captions: cap_039 'for decades,' (30.29–31.55),
  cap_040 …, cap_041 …") — the file disagrees with its own
  derivation note.

## 3. S11 — `scenes/film/s11.scene.json` (JG dusk, film t 50.648–55.520, file `time` = lt 0)

Ground truth recomputed first: st = cut(10) = 50.708 − 0.06 = 50.648;
end = cut(11) = 55.52; d = 4.872. `Wd('order',10)` = 51.78,
`Wd('surrendered',10)` = 53.02, dropT = 53.02 + 0.55 = 53.57.

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 50.648 | cut(10) = 50.708 − 0.06 | MATCH |
| 2 | `shot.duration` | 4.872 | cut(11) − cut(10) = 55.52 − 50.648 | MATCH |
| 3 | `lights.key.color` | `#FFB27A` | env.ts dusk keyCol | MATCH |
| 4 | `lights.key.intensity` | 2.0 | dusk keyI 2.0 | MATCH |
| 5 | `lights.key.position` | [−8, 5, −14] | dusk `keyPos` [−8, 5, −14] | MATCH |
| 6 | `lights.rim.intensity` | 0.5 | dusk rimI 0.5 | MATCH |
| 7 | `lights.hemi.sky` | `#B9A2D8` | dusk hemi sky | MATCH |
| 8 | `lights.hemi.intensity` | 0.9 | dusk hemi ×0.9 | MATCH |
| 9 | `lights.points[fire].channel.level` | 1 | S11 stages `fireLevel(1, t)`; static intensity 4.6 = formula basis (R1) | MATCH |
| 10 | `fog.color` | `#F59C6B` | dusk fog colour (28/120 also correct) | MATCH |
| 11 | `camera.keys[0].eye` | [0.45, 1.95, 5.0] | S11 `shoot()` from | MATCH |
| 12 | `camera.keys[last].eye` | [0.3, 1.7, 3.6] | S11 `shoot()` to | MATCH |
| 13 | `camera.fovY` | 48 | S11 look fov 48 | MATCH |
| 14 | `post.focusChannel.atStart` | 6.152235366108809 | \|(0.45,1.95,5.0) − (0.45,1.15,−1.1)\| = √37.85 = 6.152235366108809 (exact) | MATCH |
| 15 | `post.focusChannel.atEnd` | 4.734448225506326 | \|(0.3,1.7,3.6) − (0.45,1.15,−1.1)\| = √22.415 = 4.734448225506326 (exact) | MATCH |
| 16 | `post.grade.saturation` (value) | 1.08 | gradeOf(G.jungle, dusk).sat = 1.08 — value correct; key name see §4 | MATCH |
| 17 | `post.grade.exposure` (value) | 0.97 | dusk grade exposure 0.97 — value correct; key see §4 | MATCH |
| 18 | `post.grade.time` | 0 | engine uTime = 0 (engine.ts:55; R3) — this seat got it right | MATCH |
| 19 | `captions[cap_059].s/.e/.fit` | 52.07 / 53.57 / 0.92305 | onoda.json chunk 59: "and Onoda surrendered", width 1040.03, fit 0.92305 | MATCH |
| 20 | `captions[cap_060].e` | 55.55 | onoda.json chunk 60: e 55.55 ("after 29 years.") | MATCH |
| 21 | `characters[onoda].position` | [−2.65, 0, −0.45] | walk from; wk = seg(0, 0.1, 0.882) = 0 | MATCH |
| 22 | `characters[onoda].state.mood` | `determined` | t = 50.648 < Wd('order',10) branch | MATCH |
| 23 | `characters[onoda].channels.mood.keys[1].t` | 1.132 | lt of 'awe' step = 51.78 − 50.648 = 1.132 | MATCH |
| 24 | `characters[onoda].channels.visible.window[1]` | 2.922 | dropT − st = 53.57 − 50.648 = 2.922 (instance swap) | MATCH |
| 25 | `characters[onoda2].state.armR` | [34, 0] | att(0) = 34, attE(0) = 0 (poseTrack first keys) | MATCH |
| 26 | `characters[onoda2].channels.armR.keys[1].t` | 3.472 | dropT − st + 0.55 = 2.922 + 0.55 = 3.472 | MATCH |
| 27 | `characters[taniguchi].channels.armR.keys[1]` | [1.002, 112, 0.32] | key time = Wd('order',10) − st − 0.13 = 51.78 − 50.648 − 0.13 = 1.002 | MATCH |
| 28 | `characters[taniguchi].channels.lean.keys[1].t` | 4.072 | bow key = dropT − st + 1.15 = 2.922 + 1.15 = 4.072 | MATCH |
| 29 | `nodes[rifle (on ground)].channels.translate` key @ lt 2.947 | y = 0.2625636574074076 | drop law: u = (2.947−2.922)/0.3 = 1/12; eo(u) = 1−(1−u)³ = 0.22974537; y = lerp(0.32, 0.07, eo) = 0.2625636574074076 (exact); final key y = 0.07 at lt 3.222 also exact | MATCH |
| 30 | `shot.cues[thud].t/.gain` | 53.84 / 0.5 | dropT + 0.27 = 53.84, gain 0.5 (shots.ts S11 cue) | MATCH |

S11 is clean: 30/30.

## 4. Cross-cutting consumption defect — grade key names in s07 and s11

Not counted in the value tables above (the values themselves are
correct), but it changes what the native renderer actually does
today, so it is reported as a defect for the lead:

- The scene-file loader reads grade saturation **only** from the key
  `sat` (`src/scenefile.rs:975`: `f32_field_or(g, "sat", …, 1.12)`)
  and has no exposure field in GradeCfg at all (scenefile.rs:970–976).
- s03 writes `sat` — consumable. s07 and s11 write `saturation`
  (and `exposure`). Loaded as-is, s07 renders with sat = 1.12
  (default) instead of 0.85, and s11 with 1.12 instead of 1.08;
  their exposure values are not read by this loader path.
- Both seats flagged this themselves in ASSUMPTIONS.md (the S07
  section notes "the grade parser reads `sat` and has no exposure
  field"), so it is a known format divergence awaiting a lead
  decision — but as data shipped for the loader, s07/s11 saturation
  is effectively wrong until either the files or the loader change.

## 5. Comparison with the seats' ASSUMPTIONS derivations (read after computing)

- **S03 (W05)** and **S07 (W09)** both document their grade.time
  derivation explicitly — S03: "grade … time = film t 15.061";
  S07: "`grade.time` = 30.39 (film t at shot start; the grain hash
  is time-seeded…)". My recomputation disagrees with both
  derivations: the source never seeds the hash with film time
  (uTime = `look.t ?? 0` = 0; R3 pins 0). The seats recorded a
  plausible-sounding convention that the source does not have.
  **S11 (W16)** wrote 0 and agrees with the source.
- **S07 captions**: W09's ASSUMPTIONS text lists cap_039 among the
  segment's captions; the JSON file omits it (M3). Derivation and
  data disagree inside the same seat's deliverable.
- Everything else sampled — timing anchors, rigs, camera keys,
  focus distances, poseTrack key times, blink seeds, caption chunk
  times, set-node transforms — the seats' stated derivations agree
  with my independent recomputation, including the non-obvious ones
  (SH sunlight shadow overrides, golden/dusk keyPos moves, the
  fireLevel 4.6 basis under R1, S07's sat clamp to 0.85, the rifle
  drop's cubic-out keys to full f64 precision).

## 6. Additional values verified beyond the 30-per-segment sample

- s07 calendar tock cues: 31.17 / 32.21 / 33.25 at gain 0.32 =
  calT0 + 0.62, then +1.04 cycle steps — MATCH.
- s07 palm frond 0: rotate_y 0.4 = angle a₀; colour #4C8A3A = leaf2
  (i%2 parity, post-C1); tip 0 colour #3F7D33 = leaf, position
  (sin a·1.85, 4.07, cos a·1.85), euler (cos a·0.5, a, −sin a·0.5) —
  all MATCH shots.ts palm().
- s03 hills 1–3 transforms ([10,−4,45]/[46,16,27.6],
  [60,−2.75,59]/[36,11,21.6], [−90,−2.5,65]/[40,10,24]) and the
  boat bow/seat boxes — MATCH.
- s11 'inhale' cue: t 51.43 = Wd('order',10) − 0.35, gain 0.28,
  dur 0.7 — MATCH. Rifle ground node base euler (0, −0.5, π/2−0.1)
  = resting rotation with the drop's u=0 z-offset — MATCH.
- s03 companion blink states: blinkDirector(7)(15.061) =
  blinkDirector(11)(15.061) = 0, file states 0.0 — MATCH.
- s11 lists cap_056 (line 9, 49.63–50.69): its s/e/text match
  onoda.json exactly. Its presence in two segments' lists is the
  already-known W27 caption-ownership item, not a value error.


---

## Resolution — seat W41 (2026-11-14)

Fixes applied to the film data after this audit. Scope: scene files
+ `scenes/film/ASSUMPTIONS.md` + this addendum. `src/` untouched,
nothing committed. Validator re-run after all edits:
**7/7 PASS, exit 0**, with counts identical to seat W31's recorded
run (check 3: 883 exact f32 / 506 source literals / 0 computed
failures; check 5: 66 listings / 66 unique / 0 duplicates /
0 uncovered) — so `tools/film/VALIDATION.md` was not modified.

### M1 + M2 — `post.grade.time`: FIXED, and the sweep found five more

M1 (s03) and M2 (s07) named two files. The W41 sweep of all 14
segment files found the same defect — `post.grade.time` carrying a
film/evaluation time instead of 0 — in **seven** files. All seven
are now exactly `0`:

| File | Was | Now |
|---|---|---|
| s02 | 9.484 | 0 |
| s03 | 15.060999870300293 (M1) | 0 |
| s04 | 18.67 | 0 |
| s05a | 22.974 | 0 |
| s05b | 25.31 | 0 |
| s06 | 29.729999542236328 | 0 |
| s07 | 30.39 (M2) | 0 |

Already 0, verified and untouched: s01, s08, s08b, s09, s10, s11,
s12. A post-edit re-scan of every `scenes/film/*.scene.json`
confirms no nonzero `post.grade.time` remains.

On WAVE-RECORD D20's "inert" framing: that holds only for the
hardcoded `src/film.rs` path, which pins grade time 0.0 itself.
As §M1 of this audit established, the scene-file loader reads the
field (`src/scenefile.rs:976` → GradeParams at `:2051`), so on
the loader path these values were live — the data fix, not the
renderer pin, is what makes all seven segments grain-identical
to the Chrome reference. No other pre-R3 grade leftovers were
found in s05b/s07 (D20 names `grade.time` only; the rest of both
grade blocks matches their looks).

### M3 — s07 caption `cap_039`: NOT listed; resolved by record (read carefully)

W41 did **not** add `cap_039` to `s07.scene.json`'s `captions`
list. Doing so would have been wrong against the wave's own
established model, for three reasons:

1. **The manifest start-time rule already adjudicates this
   chunk.** `scenes/film/film.json` assigns chunk 39 to s06
   (s06's caption range is 32–39; s07's is exactly 40–41; the
   ranges tile 0–65 without overlap). Seat W27's rule, recorded
   in `scenes/film/ASSUMPTIONS.md` ("CAPTION OWNERSHIP +
   VALIDATOR v2 — seat W27"), is: a chunk belongs to the
   segment containing its START; *the neighbour it bleeds into
   keeps a bleed note, not a listing* — the S04/S09 convention.
   W27 deliberately REMOVED chunk 39 from s07 under this rule,
   and W31 applied the identical removal to chunks 46 (s08b)
   and 56 (s11).
2. **The bleed is already recorded in s07 itself.** The file's
   top-level `captionsNote` (seat W27) reads: "chunk 39 ('for
   decades,', 30.29-31.55) starts in S06 and is owned by s06
   under the manifest start-time rule (scenes/film/film.json);
   it stays on screen for this shot's first 1.16 s and is not
   listed here (seat W27). Captions are film-global, drawn by
   time." W41 verified the note verbatim against
   `film/onoda.json` (chunk 39: s 30.29, e 31.55; s07 starts
   30.39, so the on-screen bleed is 31.55 − 30.39 = 1.16 s).
   No renderer consuming the film data loses the chunk.
3. **The validator has no bleed-entry concept.** Check 5 of
   `tools/film/validate_film.py` counts every entry in every
   file's `captions[]` toward coverage and duplicates; a
   second listing of chunk 39 would fail it ("chunk 39 listed
   2 times (in s06, s07)") and drop the validator to 6/7.
   There is no marker field anywhere in the data model that
   would exempt a bleed listing.

The audit's supporting evidence is also superseded on one
point: s11's listing of `cap_056`, cited in §2/§6 as the
convention, was the double-listing itself — W31 removed it as
an error, and s11 now carries only the note. (s03's `cap_020`
is an owner listing whose chunk overruns the segment tail,
not a bleed-in listing; it sets no contrary precedent.)

**Recommendation to the lead:** if an in-list bleed entry is
wanted despite the above, it is a format change, not a data
fix — it needs a bleed marker on the entry, a matching
exemption in validator check 5, and an amendment to the
manifest start-time rule. Until that decision is taken, M3's
underlying fact (cap_039 is on screen for s07's first 1.16 s)
stands recorded in the file by the established mechanism, and
W40's §5 observation that W09's S07 prose names cap_039 among
the shot's captions is answered by W27's ownership section,
which supersedes that prose.

### §4 — grade key names (`saturation`/`exposure` vs loader's `sat`): NOT TOUCHED

The audit itself scopes this as "a known format divergence
awaiting a lead decision" (files vs loader). It is not M1–M3
and no W41 edit touches it: s07 still writes `saturation`
0.85 / `exposure` 1.02, s11 `saturation` 1.08 / `exposure`
0.97, under keys the loader does not read. Recorded here so
the resolution of M1–M3 is not mistaken for a resolution of
§4.

### Prose sweep — ASSUMPTIONS.md rifle placement (pre-R2 text)

The S04 and S02 sections described the rifle as recorded in a
top-level `attachments` field. As built (seat W19, ruling R2)
the rifle lives ON the Onoda character node in both files —
`attachments: [{ kind: "rifle", hand: "R", offset:
[0, 0.03, 0.07] }]` — and neither file has a top-level
`attachments` key (verified first-hand by W41; validator check
4 likewise reports 0 top-level violations, 10 on-node
attachments). Dated **[W41 correction, 2026-11-14 …]** notes
were added at each passage; the original text is retained.
A third note was added at the S01 section's parenthetical
("(Placement differs from S04's top-level `attachments`
field; same content.)"), which made the same stale claim
about S04 by cross-reference: since W19, S01's placement no
longer differs from S04's — all three files carry the
identical on-node entry.
