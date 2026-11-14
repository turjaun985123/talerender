# AUDIT-W42 — Second independent audit (breadth): s01, s02, s04, s05a, s05b, s06, s08, s08b, s09, s10, s12

Seat W42, TaleRender 100-agent wave. Companion to AUDIT-W40 (which
covered s03/s07/s11 at 30 values each, 87/90). This audit covers the
remaining eleven segments at 15 values each — 165 values — every one
recomputed first-hand from the project sources
(`~/faceless-shorts/the-soldier-who-didnt-know/src/`: `shots.ts`,
`core/env.ts`, `core/world.ts`, `core/motion.ts`, `core/lib.ts`,
`core/timing.ts`, `core/toon.ts`, `core/poses.ts`) plus the project's
`data/lines.json` / `data/words.json` and this repo's
`film/onoda.json` (caption chunks). The seats'
`scenes/film/ASSUMPTIONS.md` sections were read only **after** the
computations, to compare derivations (see §6).

Audit basis: the working tree as read on 2026-11-14, while Batch 5
was in flight. One Batch-5 seat's fix (grade.time sweep) had already
landed in the working tree — uncommitted — for s02–s07 before these
files were read; where a value differs between HEAD (`c7db74d`) and
the working tree, both are reported (this matters for the census in
§2 and nowhere else: `git diff HEAD` shows the grade.time line is the
*only* changed line in each of s02–s07).

Tolerance (same as W40): a file value within f32 rounding of the f64
recomputation (≤ ~2×10⁻⁷ relative), or a value the seat rounded to a
stated precision that rounds back to the recomputation, is a MATCH.
Anything else is a MISMATCH.

## Summary

| Segment | Sampled | MATCH | MISMATCH |
|---|---|---|---|
| s01 | 15 | 15 | 0 |
| s02 | 15 | 15 | 0 |
| s04 | 15 | 15 | 0 |
| s05a | 15 | 15 | 0 |
| s05b | 15 | 15 | 0 |
| s06 | 15 | 12 | **3** — `shot.at`, `shot.duration`, fire intensity (one root cause) |
| s08 | 15 | 15 | 0 |
| s08b | 15 | 15 | 0 |
| s09 | 15 | 15 | 0 |
| s10 | 15 | 15 | 0 |
| s12 | 15 | 15 | 0 |
| **Total** | **165** | **162** | **3** |

Plus the census finding (§2): the `post.grade.time` defect W40 found
in 2 of his 3 files was present at HEAD in **7 of the film's 14
files** — five of them in this audit's segments. And one
documentation observation on caption head-bleeds (§5) that is *not*
counted as a mismatch, for reasons stated there.

Ground truth used throughout: `cut(i) = lines[i].start − 0.06`
(cut(0) = 0); word times from `data/words.json` via the project's
`Wd()` prefix rule — `Wd('thought',4)` = 24.68, `Wd('trick',4)` =
25.6, `Wd('decades',6)` = 30.64, `Wd('coconuts',6)` = 32.4,
`Wd('found',7)` = 38.48, `Wd('refused',7)` = 40.46,
`Wd('commander',8)` = 44.04, `Wd('only',8)` = 42.52,
`Wd('bookseller',9)` = 48.16, `Wd('flew',9)` = 49.22,
`Wd('order',10)` = 51.78, `Wd('surrendered',10)` = 53.02,
`Wd('loyal',11)` = 58.12.

---

## 1. Mismatches — all in s06, one root cause

### M1 — `shot.at` (s06)

- File: `26.13`. Correct: **26.129**.
- Source: `cut(5) = L(5) − 0.06 = 26.189 − 0.06 = 26.129`
  (shots.ts S06 `st = cut(5)`; `data/lines.json` line 5 start
  26.189).
- Root cause, from the seat's own derivation (ASSUMPTIONS.md S06,
  read after computing): "`shot.at = cut(5) = 26.13` … Matches
  INVENTORY.md S06 row (26.13–30.39)." The seat took the value from
  the inventory's 2-dp display row instead of recomputing the cut.
  S06 is the only segment where this bites: every other cut in the
  film is exact at 2 dp (5.907 is 3 dp but was recomputed by its
  seats), while 26.129 → 26.13 is a real 0.001 s shift — one third
  of a frame at 30 fps, and a §11-class rounding of a computed
  value, not an f32 spelling (f32(26.129) = 26.129000778198242,
  which is what s05b's `end` correctly carries).

### M2 — `shot.duration` (s06)

- File: `4.260000228881836` (f32 of 4.26). Correct: **4.261**.
- Source: end − at = 30.39 − 26.129 = 4.261 (end = `Wd('decades',6)
  − 0.25` = 30.39, the S07 start; the file's `end` is correct).
  The duration was derived from the rounded `at` (30.39 − 26.13),
  so it inherits M1's 0.001 s error. The file is internally
  consistent (at + duration = end) but both values sit 0.001 s off
  the source cut.

### M3 — `lights.points[0].intensity` (s06, knock-on of M1)

- File: `0.6314269304275513`. Correct: **0.6333623112323646**.
- Source: S06 stages `fireLevel(clamp(1 − seg(lt, sitAt−0.6,
  sitAt+0.15)), t)` with sitAt = 3.55 (shots.ts S06). At the file's
  evaluation instant lt = 3.6: level = 1 − seg(3.6, 2.95, 3.7) =
  1 − 0.866667 = 0.133333; film t = cut(5) + 3.6 = 29.729;
  intensity = (4.6 + 0.7·sin(17·29.729) + 0.5·sin(7.3·29.729)) ×
  0.133333 = 0.6333623112323646.
- The file's value is the same formula evaluated at t = 29.73 —
  i.e. at the file's own rounded `at` (26.13 + 3.6) — reproducing
  to 1.2×10⁻⁸ (formula(29.73) × level = 0.6314269181663684, f32
  chain). The seat's ASSUMPTIONS fire section states the evaluation
  as "(at 26.13 + lt 3.60)". So M3 is not an independent error: it
  is M1 propagating into the one evaluated state in the file that
  depends on absolute film time. Fix M1 and this value follows.

Everything else sampled in s06 — the fog override (#C9CFC2, 4.5/20
replacing the fog preset's #E6E9EA, 3/38, exactly as shots.ts S06
stages it), the fog-preset rig, sat 0.52, the evaluated camera eye,
Onoda's seat-walk position and sitting squash (−0.02 =
−0.04·sin(π/6)), compA's walked-away position (−4.0, 0, −10.5) —
recomputes exactly.

---

## 2. `post.grade.time` census — all 14 files

W40 found film-t values in s03 (15.061) and s07 (30.39), correct
value 0 in both cases (`core/engine.ts:55`: `uTime = look.t ?? 0`;
no shot passes a `t`; ruling R3 pins 0). This census reads the field
in all 14 files, at HEAD (`c7db74d`, via `git show`) and in the
current working tree.

| File | At HEAD `c7db74d` | Working tree (audit read) | Verdict |
|---|---|---|---|
| s01 | 0.0 | 0.0 | clean |
| s02 | **9.484** | 0 | was wrong — film t at the file's instant |
| s03 | **15.060999870300293** (W40 M1) | 0 | was wrong |
| s04 | **18.67** | 0 | was wrong |
| s05a | **22.974** | 0 | was wrong |
| s05b | **25.31** | 0 | was wrong |
| s06 | **29.729999542236328** | 0 | was wrong — f32 of 29.73 = M1's rounded at + lt 3.6 |
| s07 | **30.39** (W40 M2) | 0 | was wrong |
| s08 | 0.0 | 0.0 | clean |
| s08b | 0.0 | 0.0 | clean |
| s09 | 0 | 0 | clean |
| s10 | 0 | 0 | clean |
| s11 | 0 (W40: correct) | 0 | clean |
| s12 | 0 | 0 | clean |

**Census result: 7 of 14 files carried the defect at HEAD** — W40's
two were the visible part of a majority convention, not an
isolated slip. Every wrong value is exactly the film time at its
file's evaluation instant (s06's is built from its rounded `at`,
compounding M1). The seven clean files are the ones whose seats
evaluated at lt 0 under an lt clock or wrote the static value. The
working-tree zeroes are the Batch-5 fix seat's sweep (one changed
line per file in s02–s07, uncommitted at audit time); I verified
each zeroed value reads 0 and that no other line in those files
changed. The per-segment grade.time rows in §3 are scored against
the working tree (MATCH) with the HEAD value disclosed.

Related key census (from the same read): the grade saturation key
is `sat` in only s03 and s06; the other **12 of 14** files write
`saturation` (most also write `exposure`). W40 §4 reported this
consumption defect for s07/s11 — it in fact spans every file except
s03/s06. Values are correct everywhere I sampled them (§3); the
loader reads only `sat` (W40's citation: `src/scenefile.rs:975`),
so the lead decision W40 flagged (fix files or loader) covers
twelve files, not two.

---

## 3. Per-segment tables (15 sampled values each)

### S01 — `s01.scene.json` (JG noon, film t 0.00–5.907, file `time` = lt 0)

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 0.0 | cut(0) = 0 | MATCH |
| 2 | `shot.duration` | 5.9070000648498535 | cut(1) = 5.967 − 0.06 = 5.907 (f32 spelling) | MATCH |
| 3 | `lights.key.intensity` | 2.6 | noon `keyI` 2.6 — applyAtmosphere overrides the JG sunlight's i = 2.4 (env.ts noon; shots.ts noonJG) | MATCH |
| 4 | `lights.key.position` | [7, 12, 5] | JG `sunlight()` pos [7, 12, 5]; noon carries no `keyPos`, so the set's pos survives | MATCH |
| 5 | `lights.hemisphere.ground` | `#C48A5A` | noon hemiGround — overrides the JG sunlight's ground `#46603A` | MATCH |
| 6 | `camera.keys[1].eye` | [0.55, 1.62, 3.2] | S01 `shoot()` to | MATCH |
| 7 | `camera.fovY` | 44 | S01 look fov 44 | MATCH |
| 8 | `post.grade.time` | 0.0 | 0 (engine.ts:55; R3) — HEAD also 0 | MATCH |
| 9 | `post.grade.saturation` | 0.8 | G.jungle sat 0.8 (value; key name see §2) | MATCH |
| 10 | `post.bokeh.focus` | 6.176479816436768 | \|(0.85,2.7,5.4) − (0.35,1.12,−0.55)\| = 6.1764795798 (shoot() focus = eye–target distance at lt 0) | MATCH |
| 11 | `characters[onoda].state.headTurn` | −0.45 | poseTrack([[0,−0.45],[2.6,0.4,2.2],[5.2,−0.2,2.2]], 0) = first key | MATCH |
| 12 | `characters[onoda].state.camPitch` | 0.29956525564193726 | camPitchOf (toon.ts:720): atan2(2.7 − 1.75·0.6, hypot(0.5, 5.95)) / 0.9 = 0.2995652572 | MATCH |
| 13 | `characters[onoda].attachments[0].offset` | [0, 0.03, 0.07] | `jgOnoda.attach(rifleHeld, 'R', { offset: [0, 0.03, 0.07] })` (on the character node, per R2) | MATCH |
| 14 | `captions[cap_012].s/.e` | 5.29 / 5.937 | onoda.json chunk 12: s 5.29, e 5.937 | MATCH |
| 15 | `textures[grass-hills].repeat` | [6, 3] | world.ts `hills()`: `T.grass(col, [6, 3])` | MATCH |

### S02 — `s02.scene.json` (SH noon, film t 5.907–13.061, file `time` = film t 9.484, lt 3.577)

At lt 3.577: k = camK(3.577, 7.154, 0.03) = 0.5 exactly (the file's
instant is the move's midpoint), travelled = 3.4,
phase = walkPhase(3.4, 0.62).

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 5.907 | cut(1) = 5.967 − 0.06 | MATCH |
| 2 | `shot.duration` | 7.154 | cut(2) − cut(1) = 13.061 − 5.907 | MATCH |
| 3 | `lights.key.position` | [−8, 11, 7] | SH `sunlight()` pos; noon has no `keyPos` | MATCH |
| 4 | `lights.key.shadow.far` (file: `lights.key.far`) | 80 | world.ts `sunlight()`: `key.shadow.camera.far = 80` (overrides lightRig 40) | MATCH |
| 5 | `camera.keys[0].eye` | [2.0, 3.1, 2.4] | S02 crane from | MATCH |
| 6 | `camera.fovY` | 46 | S02 look fov 46 | MATCH |
| 7 | `post.grade.time` | 0 | 0 — HEAD carried **9.484** (see census §2) | MATCH (fixed in worktree) |
| 8 | `post.bokeh.aperture` | 0.0007 | S02 look override `aperture: 0.0007` (G.shore base is 0.0012) | MATCH |
| 9 | `post.bokeh.focus` | 6.5050458908081055 | crane at k = 0.5: eye (1.45, 2.475, 1.5), look (−0.05, 1.2, −4.7), distance 6.5050461182 | MATCH |
| 10 | `characters[onoda].position` | [0, 0, −7.599999904632568] | [0, 0, −11 + 3.4] = −7.6 (f32 spelling); source formula, no clamp at k = 0.5 | MATCH |
| 11 | `characters[onoda].state.phase` | 17.22808837890625 | walkPhase(3.4, 0.62) = 3.4/0.62·π = 17.2280887455 | MATCH |
| 12 | `characters[onoda].state.bob` | 0.0064174639992415905 | walkBob(phase, 5) = 5·(1 − \|sin(17.2280887)\|) = 0.0064174641 | MATCH |
| 13 | `characters[onoda].attachments[0].offset` | [0, 0.03, 0.07] | `shOnoda.attach(rifleShore, 'R', { offset: [0, 0.03, 0.07] })` | MATCH |
| 14 | `captions[cap_017].s/.e` | 10.39 / 13.15 | onoda.json chunk 17 ("in 1944."): s 10.39, e 13.15 | MATCH |
| 15 | captions completeness — chunk 12 | not listed | chunk 12 ("ended.", 5.29–5.937) is on screen for the first 0.030 s; not listed under the manifest start-time rule and the omission is documented in the seat's ASSUMPTIONS with the correct duration — see §5 | MATCH (see §5) |

### S04 — `s04.scene.json` (JG noon, film t 15.95–21.384, file `time` = film t 18.67, lt 2.72)

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 15.95 | cut(3) = 16.01 − 0.06 | MATCH |
| 2 | `shot.duration` | 5.434 | cut(4) − cut(3) = 21.384 − 15.95 | MATCH |
| 3 | `lights.key.intensity` | 2.6 | noon keyI over JG sunlight 2.4 | MATCH |
| 4 | `camera.keys[1].eye` | [0.85, 1.65, 6.6] | S04 `shoot()` to | MATCH |
| 5 | `camera.fovY` | 50 | S04 look fov 50 | MATCH |
| 6 | `post.grade.time` | 0 | 0 — HEAD carried **18.67** (census §2) | MATCH (fixed in worktree) |
| 7 | `post.bokeh.focus` | 7.725 | \|eye(camK(2.72, 5.434) = 0.501294) − (0.8, 1.1, −0.6)\| = 7.7250264738 (file rounds to 3 dp) | MATCH |
| 8 | `characters[compA].state.bob` | −3.923394203186035 | bounce = −4.5·\|sin(4.6·18.67)\| = −3.9233941542 | MATCH |
| 9 | `characters[compB].state.bob` | −3.4757516384124756 | −4.5·\|sin(4.6·18.67 + 1.2)\| = −3.4757516020 (compB's +1.2 phase) | MATCH |
| 10 | `characters[compA].state.armL` | [150, 14] | POSES.celebrate armL [150, 14] (poses.ts) | MATCH |
| 11 | `characters[onoda].state.headTurn` | −0.3499925434589386 | poseTrack([[0,0.5],[2.2,−0.35,1.9],[4.6,0.3,1.9]], 2.72) = −0.3499925379 (move into the 4.6 key just begun) | MATCH |
| 12 | `characters[onoda].position` | [1.8, 0, 1.55] | S04 staging `jgOnoda.at(1.8, 0, 1.55)` | MATCH |
| 13 | `characters[onoda].attachments[0].offset` | [0, 0.03, 0.07] | rifleHeld attach offset | MATCH |
| 14 | `captions[cap_024].s/.e` | 19.93 / 20.47 | onoda.json chunk 24: s 19.93, e 20.47 | MATCH |
| 15 | captions completeness — chunk 20 | not listed | chunk 20 ("never surrender.", 14.75–16.01) on screen for the first 0.060 s; documented in ASSUMPTIONS with the correct duration — see §5 | MATCH (see §5) |

### S05a — `s05a.scene.json` (JG noon, film t 21.384–24.56, file `time` = film t 22.974, lt 1.59)

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 21.384 | cut(4) = 21.444 − 0.06 | MATCH |
| 2 | `shot.duration` | 3.176 | end (S05b start = Wd('thought',4) − 0.12 = 24.56) − at | MATCH |
| 3 | `shot.cues[0].at/.gain` | 21.884 / 0.5 | `cue(st + 0.5, 'whoosh', { gain: 0.5, dur: 1.6 })` | MATCH |
| 4 | `camera.fovY` | 46 | S05a look fov 46 | MATCH |
| 5 | `camera.keys[0].target` | [−4.5, 6.4, −11] | crane lookFrom | MATCH |
| 6 | `post.grade.time` | 0 | 0 — HEAD carried **22.974** (census §2) | MATCH (fixed in worktree) |
| 7 | `post.bokeh.focus` | 16.296 | crane at k = camK(1.59, 3.176) = 0.501476: \|eye − look\| = 16.2959410874 (file rounds to 3 dp) | MATCH |
| 8 | `channels[fireLight.intensity].level` | 0 | S05a stages `fireLevel(0, t)` — fire out; no points block in the file | MATCH |
| 9 | `nodes[plane].transform` translate | [−0.984395, 9.6, −13] | px = lerp(−13, 11, seg(1.59, 0.05, 3.126)) = −0.9843953186 (constant-speed flyover, no easing) | MATCH |
| 10 | `nodes[propeller].transform` euler z | 804.09 | prop.rotation.z = 35·t = 35·22.974 (absolute film t) | MATCH |
| 11 | `nodes[leaflet 0].transform` translate | [−6.568394, 6.354367, −10.660129] | closed form at lt 1.59: ft = seg(1.59, 0.4, 3.5) = 0.383871; x = sx + sin(4ft)·0.9 + 1.2ft, y = 9.4 − ft^1.12·8.9, z = −13 + (rnd(0)−0.5)·3 + 3.6ft → [−6.5683944603, 6.3543674431, −10.6601293257] (file at 6 dp) | MATCH |
| 12 | `captions[cap_027].s/.e` | 22.61 / 23.59 | onoda.json chunk 27: s 22.61, e 23.59 | MATCH |
| 13 | captions completeness — chunk 25 | not listed | chunk 25 ("the message.", 20.47–21.61) on screen for the first 0.226 s; documented in ASSUMPTIONS ("tails in until 21.61 but starts in S04") — see §5 | MATCH (see §5) |
| 14 | `lights.key.intensity` | 2.6 | noon keyI (JG rig) | MATCH |
| 15 | `nodes[leaflet 0].geometry.size` | [0.34, 0.45, 0.005] | PlaneGeometry(0.34, 0.45) (file renders it as a 5 mm box — declared substitution) | MATCH |

### S05b — `s05b.scene.json` (JG noon, film t 24.56–26.129, file `time` = film t 25.31, lt 0.75)

dropT = Wd('trick',4) + 0.55 = 26.15 — after the shot's end
(26.129), so the leaflet is held for the whole shot (the seat's
ASSUMPTIONS §"The drop that never plays" agrees; the armL move
window [1.14, 1.59] never starts inside the shot either).

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 24.56 | Wd('thought',4) − 0.12 = 24.68 − 0.12 | MATCH |
| 2 | `shot.duration` | 1.569 | cut(5) − at = 26.129 − 24.56 | MATCH |
| 3 | `lights.points[0].intensity` | 4.968071937561035 | fireLevel formula at film t 25.31: 4.6 + 0.7·sin(17·25.31) + 0.5·sin(7.3·25.31) = 4.9680718611 | MATCH |
| 4 | `camera.fovY` | 42 | S05b look fov 42 | MATCH |
| 5 | `camera.keys[1].eye` | [0.7, 1.6, 1.8] | S05b `shoot()` to | MATCH |
| 6 | `post.grade.time` | 0 | 0 — HEAD carried **25.31** (census §2) | MATCH (fixed in worktree) |
| 7 | `post.bokeh.aperture` | 0.0012 | G.jungle aperture | MATCH |
| 8 | `characters[onoda].position` | [0.35, 0, −0.55] | S05b staging | MATCH |
| 9 | `characters[onoda].state.armL` | [58, −24] | poseTrack([[0,58],[1.59,16,0.45]], 0.75) = 58 (move window starts 1.14); elbow −24 constant | MATCH |
| 10 | `characters[onoda].state.headTurn` | 0.04 | t = 25.31 < wTrick 25.6 → shake = 0.04 branch | MATCH |
| 11 | `characters[onoda].attachments[0].offset` | [0, 0.03, 0.07] | rifleHeld attach offset | MATCH |
| 12 | `captions[cap_031].s/.e` | 25.57 / 26.19 | onoda.json chunk 31 ("trick."): s 25.57, e 26.19 | MATCH |
| 13 | captions completeness — chunk 29 | not listed | chunk 29 ("He thought it", 24.49–24.97) on screen for the first 0.410 s; documented in ASSUMPTIONS — see §5 | MATCH (see §5) |
| 14 | `characters[onoda].channels.armL.poseTrack.keys[1]` | [1.590000033378601, 16, 0.45] | [dropT − st, 16, 0.45] = [1.59, 16, 0.45] (f32 spelling) | MATCH |
| 15 | `nodes[flame].transform` translate | [−1.35, 0.34, −2.45] (f32 spellings) | flame cone at (FX, 0.34, FZ) = (−1.35, 0.34, −2.45) | MATCH |

### S06 — `s06.scene.json` (JG fog, film t 26.129–30.39, file `time` = lt 3.6)

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 26.13 | cut(5) = **26.129** — M1 | **MISMATCH** |
| 2 | `shot.duration` | 4.260000228881836 | 30.39 − 26.129 = **4.261** — M2 | **MISMATCH** |
| 3 | `fog.color` | `#C9CFC2` | S06 replaces the preset fog in the draw: `new THREE.Fog(0xC9CFC2, 4.5, 20)` (the fog preset's own colour #E6E9EA survives only as the character tint colour) | MATCH |
| 4 | `fog.far` | 20 | same override (preset far is 38) | MATCH |
| 5 | `lights.key.color` | `#EAF0F2` | env.ts fog preset keyCol (applyAtmosphere with the JG rig) | MATCH |
| 6 | `post.grade.sat` | 0.52 | S06 look: `{ ...gradeOf(G.jungle, fog), sat: 0.52 }` — and this file uses the loader-consumed `sat` key (§2) | MATCH |
| 7 | `post.grade.time` | 0 | 0 — HEAD carried **29.729999542236328** (census §2; built from M1's rounded at) | MATCH (fixed in worktree) |
| 8 | `camera.keys[0].eye` | [1.7, 1.9, 4.4] | S06 `shoot()` from | MATCH |
| 9 | `camera.eye` (evaluated) | [1.151602029800415, 1.6806408166885376, 2.9043691158294678] | mix(from, to, camK(3.6, 4.261) = 0.9970567) = [1.1516188282, 1.6806475313, 2.9044149861] (Δ ≤ 4.6×10⁻⁵, inside the tolerance band; the seat's chain used its own d) | MATCH |
| 10 | `lights.points[0].intensity` | 0.6314269304275513 | fireLevel(0.133333) at film t 29.729 = **0.6333623112323646** — M3 (file evaluates at its rounded at: t 29.73) | **MISMATCH** |
| 11 | `characters[onoda].position` | [−0.5, 0, −1.6200000047683716] | z = lerp(−1.48, −1.76, seg(3.6, 3.4, 3.8) = 0.5) = −1.62 (f32 spelling) | MATCH |
| 12 | `characters[onoda].state.squash` | −0.019999999552965164 | sitting at lt 3.6 ≥ sitAt: −0.04·sin(seg(3.6, 3.55, 3.85)·π) = −0.04·sin(π/6) = −0.02 | MATCH |
| 13 | `characters[compA].position` | [−4.0, 0, −10.5] | wk = seg(3.6, 0.1, 3.55) = 1 → at(−1.7 − 2.3, 0, −5.0 − 5.5) | MATCH |
| 14 | `captions[cap_036].s/.e` | 28.49 / 29.351 | onoda.json chunk 36: s 28.49, e 29.351 | MATCH |
| 15 | `captionsNote` chunk-31 bleed claim | 0.06 s | chunk 31 (25.57–26.19) ends 0.061 s past cut(5) = 26.129 — the note's 0.06 is exact against the file's own at (26.13) and correct to 2 dp against the source cut | MATCH |

### S08 — `s08.scene.json` (JG noon, film t 34.209–40.36, file `time` = film t 37.209, lt 3.0)

wkS = seg(3.0, 0.1, 3.3) = 0.90625 (Suzuki's walk),
wkO = seg(3.0, 0.35, 1.95) = 1 (Onoda arrived).

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 34.208999999999996 | cut(7) = 34.269 − 0.06 = 34.209 | MATCH |
| 2 | `shot.duration` | 6.151000000000003 | end (S08b start = Wd('refused',7) − 0.1 = 40.36) − at | MATCH |
| 3 | `lights.points[0].intensity` | 4.474712371826172 | fireLevel(1) formula at film t 37.209 = 4.4747121351 | MATCH |
| 4 | `camera.fovY` | 62 | S08 look fov 62 | MATCH |
| 5 | `camera.keys[1].eye` | [0.8, 1.8, 5.0] | S08 `shoot()` to | MATCH |
| 6 | `post.grade.time` | 0.0 | 0 — HEAD also 0 | MATCH |
| 7 | `post.bokeh.focus` | 6.904236793518066 | \|eye(camK(3.0, 6.151) = 0.471250) − (0.35, 1.1, −1.2)\| = 6.9042366444 | MATCH |
| 8 | `characters[suzuki].position` | [1.7421875, 0, −1.024999976158142] | [lerp(3.6, 1.55, 0.90625), 0, lerp(−3.2, −0.8, 0.90625)] = [1.7421875, 0, −1.025] | MATCH |
| 9 | `characters[suzuki].state.bob` | 4.28253698348999 | walkBob(walkPhase(0.90625·3.28, 0.6) = 15.5640, 5) = 4.2825368900 | MATCH |
| 10 | `characters[suzuki].state.mood` | `neutral` | t = 37.209 < wFound 38.48 → 'neutral' branch | MATCH |
| 11 | `characters[onoda].position` | [−1.100000023841858, 0, −1.5] | wkO = 1 → [−1.1, 0, −1.5] (f32 spelling) | MATCH |
| 12 | `characters[onoda].attachments[0].offset` | [0, 0.03, 0.07] | rifleHeld attach offset | MATCH |
| 13 | `captions[cap_044].s/.e` | 37.13 / 38.45 | onoda.json chunk 44: s 37.13, e 38.45 | MATCH |
| 14 | captions completeness — chunk 41 | not listed | chunk 41 ("and bananas.", 32.99–34.27) on screen for the first 0.061 s; documented in ASSUMPTIONS ("tails 0.06 s into the shot head… the S04 chunk-20 rule") — see §5 | MATCH (see §5) |
| 15 | `textures[planks-crate].base/.repeat` | `#B07A48` / [0.5, 0.5] | world.ts `crate()`: `T.planks('#B07A48', [0.5, 0.5])` | MATCH |

### S08b — `s08b.scene.json` (JG noon, film t 40.36–41.732, file `time` = lt 0.686)

camK(0.686, 1.372) = 0.5 exactly — the evaluated camera is the move
midpoint. Film t at the instant = 41.046 > wRef 40.46, so Onoda's
head-shake settle is running.

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 40.36000061035156 | Wd('refused',7) − 0.1 = 40.36 (f32 spelling) | MATCH |
| 2 | `shot.duration` | 1.371999979019165 | cut(8) − at = 41.732 − 40.36 = 1.372 (f32 spelling) | MATCH |
| 3 | `camera.eye` (evaluated) | [0.2750000059604645, 1.7400000095367432, 4.824999809265137] | mix((0.3,1.78,5.1), (0.25,1.7,4.55), 0.5) = (0.275, 1.74, 4.825) (f32 spellings) | MATCH |
| 4 | `camera.fovY` | 54 | S08b look fov 54 | MATCH |
| 5 | `post.grade.time` | 0.0 | 0 — HEAD also 0 | MATCH |
| 6 | `post.bokeh.focus` | 5.999403953552246 | \|(0.275, 1.74, 4.825) − (0.25, 1.2, −1.15)\| = 5.9994041371 | MATCH |
| 7 | `characters[onoda].state.headTurn` | 0.05645225569605827 | settle(41.046, 40.46, 5.5, 3.4)·0.42 = 0.0564522569 | MATCH |
| 8 | `characters[onoda].position` | [−0.95, 0, −1.5] | S08b staging | MATCH |
| 9 | `characters[suzuki].state.armR/.armL` | [118, −105] / [10, 8] | POSES.listen armR [118, −105], armL [10, 8] (poses.ts) | MATCH |
| 10 | `characters[suzuki].state.mood` | `worried` | S08b overrides listen's mood to 'worried' | MATCH |
| 11 | `characters[onoda].attachments[0].offset` | [0, 0.03, 0.07] | rifleHeld attach offset | MATCH |
| 12 | `captions[cap_047].s/.e` | 40.83 / 41.762 | onoda.json chunk 47 ("to leave."): s 40.83, e 41.762 | MATCH |
| 13 | `captionsNote` chunk-46 bleed claim | 0.47 s | chunk 46 ends 40.83; 40.83 − 40.36 = 0.47 | MATCH |
| 14 | `lights.key.position` | [7, 12, 5] | JG sunlight pos (noon: no keyPos) | MATCH |
| 15 | `lights.points[0].intensity` (static) | 4.6 | the fireLevel basis (R1 — never the constructor's 5.0); this file stores the basis and carries the formula in its channel, where s05b/s08 store the evaluated value — both forms recompute to the same law | MATCH |

### S09 — `s09.scene.json` (JG noon, film t 41.732–45.365, file `time` = film t 43.92, lt 2.188)

lt 2.188 is exactly the chest-tap arrival instant:
Wd('commander',8) − st − 0.12 = 44.04 − 41.732 − 0.12 = 2.188.

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 41.732 | cut(8) = 41.792 − 0.06 | MATCH |
| 2 | `shot.duration` | 3.633 | cut(9) − cut(8) = 45.365 − 41.732 | MATCH |
| 3 | `lights.points[0].channel.atTime` | 4.076015949249268 | fire formula at film t 43.92 = 4.076015727691372 (file is its f32 spelling; agrees with W31's independent recomputation) | MATCH |
| 4 | `camera.fovY` | 54 | S09 look fov 54 | MATCH |
| 5 | `post.grade.time` | 0 | 0 — HEAD also 0 | MATCH |
| 6 | `post.focusChannel.atTime` | 5.6708228225257145 | \|eye(camK(2.188, 3.633) = 0.729428) − (0.25, 1.2, −1.15)\| = 5.6708228225257145 (exact) | MATCH |
| 7 | `post.focusChannel.atStart` | 6.075154319027625 | \|(0.3, 1.75, 4.9) − (0.25, 1.2, −1.15)\| = 6.075154319027625 (exact) | MATCH |
| 8 | `characters[onoda].channels.armR.keys[1]` | [2.187999963760376, 56, 0.3] | [wComm − st − 0.12, 56, 0.3] = [2.188, 56, 0.3] (f32 spelling) | MATCH |
| 9 | `characters[onoda].state.armR` | [56, −116] | chest poseTracks at the arrival instant lt 2.188: armR 56, elbow track −116 | MATCH |
| 10 | `characters[onoda].position` | [−0.95, 0, −1.5] | S09 staging | MATCH |
| 11 | `characters[suzuki].state.armR/.armL` | [48, −118] / [20, −30] | POSES.think (t = 43.92 > wOnly + 0.3 = 42.82, so think, not listen) | MATCH |
| 12 | `characters[onoda].attachments[0].offset` | [0, 0.03, 0.07] | rifleHeld attach offset | MATCH |
| 13 | `captions[cap_050].s/.e` | 43.13 / 44.01 | onoda.json chunk 50: s 43.13, e 44.01 | MATCH |
| 14 | captions completeness — chunk 47 | not listed | chunk 47 ("to leave.", 40.83–41.762) on screen for the first 0.030 s; documented in ASSUMPTIONS ("the S04 convention") — see §5 | MATCH (see §5) |
| 15 | `lights.key.intensity` | 2.6 | noon keyI | MATCH |

### S10 — `s10.scene.json` (BK noon, film t 45.365–50.648, file `time` = film t 48.6, lt 3.235)

The bookshop is not re-dressed: `applyAtmosphere(BK.scene, 'noon')`
is called with no rig (shots.ts), so the interior's own rig survives
intact — every light below is world.ts `interior()`'s, not the noon
preset's. At t = 48.6: wk = seg(48.6, wFlew+0.5, wFlew+1.7) = 0
(Taniguchi has not started walking), and wBooks 48.16 ≤ t < wFlew
49.22 (the 'awe' window).

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 45.365 | cut(9) = 45.425 − 0.06 | MATCH |
| 2 | `shot.duration` | 5.2829999923706055 | cut(10) − cut(9) = 5.283 (f32 spelling) | MATCH |
| 3 | `lights.key.color/.intensity` | `#FFE4B8` / 2.4 | interior() sun: `DirectionalLight('#FFE4B8', 2.4)` | MATCH |
| 4 | `lights.hemisphere.intensity` + `lights.envIntensity` | 1.1 + 0.35 | interior() HemisphereLight ×1.1; `scene.environmentIntensity = 0.35` | MATCH |
| 5 | `shot.cues[0].t` | 49.63999938964844 | `cue(wFlew + 0.42, 'plop')` = 49.64 (f32 spelling) | MATCH |
| 6 | `camera.fovY` | 46 | S10 look fov 46 | MATCH |
| 7 | `post.grade.time` | 0 | 0 — HEAD also 0 | MATCH |
| 8 | `post.grade.saturation` | 0.78 | G.shop sat 0.78, used verbatim (no gradeOf for the shop) | MATCH |
| 9 | `post.bokeh.focus` | 6.319650650024414 | \|eye(camK(3.235, 5.283) = 0.749782) − (−1.5, 1.45, −5.3)\| = 6.3196505596 | MATCH |
| 10 | `characters[taniguchi].position` | [−1.7000000476837158, 0, −5.5] | wk = 0 → at(−1.7, 0, −5.5) (f32 spelling) | MATCH |
| 11 | `characters[taniguchi].state.mood` | `awe` | wBooks ≤ t < wFlew branch | MATCH |
| 12 | `characters[taniguchi].state.armR` | [42, −28] | poseTrack([[0,42],[wFlew−st,17,0.5]], 3.235) = 42 (move window starts 3.355); elbow −28 | MATCH |
| 13 | `characters[taniguchi].state.headTurn` | 0.3 | wBooks ≤ t < wFlew → headTurn 0.3 | MATCH |
| 14 | `captions[cap_054].s/.e` | 47.77 / 48.93 | onoda.json chunk 54: s 47.77, e 48.93 | MATCH |
| 15 | captions completeness — chunk 51 | not listed | chunk 51 ("commander ordered it.", 44.01–45.395) on screen for the first 0.030 s; documented in ASSUMPTIONS — see §5 | MATCH (see §5) |

### S12 — `s12.scene.json` (JG dusk, film t 55.52–60.357, file `time` = lt 3.48)

bowA = poseTrack([[0,0],[2.5,3.5,0.45]], 3.48) = 3.5 (move long
finished); film t at the instant = 59.0 ≥ Wd('loyal',11) = 58.12.

| # | JSON path | File value | Recomputed from source | Verdict |
|---|---|---|---|---|
| 1 | `shot.at` | 55.52 | cut(11) = 55.58 − 0.06 | MATCH |
| 2 | `shot.duration` | 4.837 | film duration 60.357 − cut(11) | MATCH |
| 3 | `fog.color/.far` | `#F59C6B` / 120 | dusk preset fog (#F59C6B, 28/120) | MATCH |
| 4 | `lights.key.position` + `lights.hemisphere.sky` | [−8, 5, −14] + `#B9A2D8` | dusk `keyPos` [−8, 5, −14] (preset moves the JG key); dusk hemiSky | MATCH |
| 5 | `post.grade.time` | 0 | 0 — HEAD also 0 | MATCH |
| 6 | `post.grade.saturation` | 1.08 | gradeOf(G.jungle, dusk).sat = dusk grade sat 1.08 (value; key name see §2) | MATCH |
| 7 | `camera.keys[1].eye` | [1.6, 4.6, 9.8] | S12 crane to | MATCH |
| 8 | `post.focusChannel.atTime` | 11.389776820801682 | crane at k = camK(3.48, 4.837) = 0.9204535282: \|eye(k) − target(k)\| = 11.389776820801677 — the file's own note states k = 0.9204535282241786, confirmed | MATCH |
| 9 | `characters[onoda2].state.lean/.headTilt` | 2.1 / −3.5 | lean = bowA·0.6 = 2.1; headTilt = −bowA = −3.5 | MATCH |
| 10 | `characters[onoda2].state.mood` | `neutral` | film t 59.0 ≥ wLoyal → 'neutral' (the 'relieved' step ends at lt 2.6 = wLoyal − st) | MATCH |
| 11 | `characters[onoda2].position` | [−0.15, 0, −0.95] | S12 staging (the rifle-less instance) | MATCH |
| 12 | `nodes[rifle (on ground)].transform` | translate [0.62, 0.07, −0.1], euler [0, −0.5, π/2] | rifleDown rest transform; S12 pins y = 0.07 and rotation.z = π/2 in the draw | MATCH |
| 13 | `characters[taniguchi].position/.state.headTilt` | [2.3, 0, −2.6] / −3 | S12 staging `jgTani.at(2.3, 0, −2.6)`, headTilt −3 | MATCH |
| 14 | `captions[cap_065].s/.e` | 58.35 / 60.357 | onoda.json chunk 65: s 58.35, e 60.357 (film end) | MATCH |
| 15 | captions completeness — chunk 60 | not listed | chunk 60 ("after 29 years.", 53.57–55.55) on screen for the first 0.030 s; documented in ASSUMPTIONS ("the overlap is staging truth… belongs to the S11 file") — see §5 | MATCH (see §5) |

---

## 4. Additional values verified beyond the 15-per-segment sample

All MATCH:

- s01 `nodes[palm p1].transform`: translate (4.6, 0, −4.6),
  rotate_z 0.14, scale 1.05 — `palm(1.05, 0.14)` at (4.6, 0, −4.6)
  (file values are f32 spellings). Palms p2/p3 likewise.
- s01 hill 0 transform: translate (−40, −3, −75), scale
  (34, 12, 20.4) — world.ts `hills()` default list[0] under
  JG's z = −75: pos y = −h·0.25, scale (r, h, 0.6r).
- s02 `characters[compA].position` [−1.4, 0, −8.5]: march with
  dz = −0.9 at travelled 3.4.
- s02 `nodes[boat].transform`: translate (−4.6, 0, −7.5),
  rotate_y 1.15 — SH boat group.
- s04 `textures[planks-roof]`: base `#8A6B42`, repeat [1, 1] —
  lean-to roof `T.planks('#8A6B42', [1, 1])`.
- s05a `textures[grass].base` `#5B9A48` — JG ground
  `T.grass('#5B9A48', [48, 48])`.
- s06 `lights.hemisphere.intensity` 0.9 — fog preset hemiI.
- s08b `textures[grass].base` `#5B9A48` — JG ground.
- s09 `textures[planks-barrel]`: base `#9A6337`, repeat [1, 1] —
  `barrel()`'s `T.planks('#9A6337')` default repeat.
- s10 `clear` `#1A1418` — interior() `scene.background`.
- s10 `textures[planks-floor]`: base `#7A5230`, repeat
  [3.2, 3.6] — `[w/2.5, (depth+2)/2.5]` at w = 8, depth = 7.
- s10 `nodes[shelf unit 1].transform` translate
  (−1.8, 0, −6.55) (f32 spellings) — `shelfUnit(3.4, 2.5, 1)`
  placement.
- s12 `shot.cues[0]`: t 55.92, gain 0.16 — `cue(st + 0.4,
  'chirp', { gain: 0.16, dur: 1.8 })`.
- s04 `characters[onoda].state.blink` 0.0:
  max(blinkDirector(3)(18.67), blinkAt(18.67, [20.7])) = 0
  (director interval walk: x − acc = 2.18 ≥ 0.12).
- s02 `characters[onoda].state.blink` 0.0:
  blinkDirector(3)(9.484) = 0 (x − acc = 2.694 ≥ 0.12).

## 5. Caption head-bleeds — why they are not counted as mismatches

Eight of my segments open mid-chunk: the caption chunk active at
the cut started in the previous segment, is on screen for the
first 0.030–0.410 s, and is not in the file's caption list
(s02/12: 0.030 s; s04/20: 0.060 s; s05a/25: 0.226 s; s05b/29:
0.410 s; s08/41: 0.061 s; s09/47: 0.030 s; s10/51: 0.030 s;
s12/60: 0.030 s).

W40 counted the same shape in s07 as a mismatch (M3). The cases
differ in the decisive respect: W09's ASSUMPTIONS section *listed*
cap_039 among S07's captions while its file omitted it — the
derivation contradicted the data. Here, every seat's ASSUMPTIONS
section states the same rule and applies it consistently:
chunks are listed by the segment they start in (the manifest
start-time rule that W27/W31 later formalised and the validator
enforces), the bleeding chunk is named, and its bleed duration is
stated correctly in every case I checked (quoted in §6). The
files follow their stated derivations exactly, and the renderer
draws captions film-globally by time, so no frame loses the
chunk. I therefore score the completeness rows MATCH.

One documentation inconsistency is worth the lead's attention,
however: only s06, s08b (and s11, per W31) carry the disclosure
inside the file as a `captionsNote`. In the other eight files the
disclosure exists only in ASSUMPTIONS.md. A consumer reading the
scene files alone cannot tell an intentional ownership exclusion
from an omission — the exact ambiguity that made W40's s07 case
look like a choice. Adding the note to the eight files is a
one-line-per-file change; the data itself needs none.

(For the record: W40's M3 itself is still open in the working
tree — s07's captions still list only cap_040/cap_041; the
Batch-5 fix had zeroed s07's grade.time but had not yet added
cap_039 when this audit read the file.)

## 6. Comparison with the seats' ASSUMPTIONS derivations (read after computing)

- **S06 (timing)**: the derivation states "`shot.at = cut(5) =
  26.13` … Matches INVENTORY.md S06 row (26.13–30.39)" — the seat
  sourced the cut from the inventory's rounded display row rather
  than from `lines.json`. Every other seat's timing section
  recomputes its cut from line starts (e.g. S02: "cut(1) = L(1) −
  0.06 = 5.907"). This single sourcing slip is the root of M1–M3;
  the seat's downstream arithmetic (duration from its own at,
  fire at its own at + lt) is internally exact.
- **Captions**: as detailed in §5, all eight head-bleed seats
  documented the bleed and the ownership rule with correct
  durations (S02 "still on screen for the first 0.03 s… it
  belongs to line 0 and is not listed"; S05a "chunk 25 tails in
  until 21.61 but starts in S04"; S08 "the S04 chunk-20 rule";
  S12 "the overlap is staging truth"). Derivations and files
  agree in all eleven segments.
- **S05b's drop finding**: the seat flagged on its own that
  dropT (26.15) falls 0.021 s past the shot's end, so the
  leaflet-drop animation never plays inside S05b. My recomputation
  confirms the arithmetic; it is a source truth (the cut at
  cut(5) = 26.129 precedes the drop), not a data error.
- **Fire intensity forms differ by file, lawfully**: s05b/s08
  store the evaluated intensity at the file's instant, s09 stores
  it in a channel `atTime`, s08b/s12 store the static 4.6 basis
  with the formula in the channel, s06 stores the evaluated value
  (M3 aside). All five forms recompute to the same fireLevel law;
  the format tolerates all of them, but a consumer must handle
  both — worth one line in the format doc.
- Everything else sampled — preset-over-set light resolution
  (noon overriding JG/SH sunlight intensities but never positions;
  the interior exempt because its noon call passes no rig),
  crane/camK focus distances at the files' instants (including
  s12's self-stated k, confirmed to 7 decimal places), poseTrack
  arrival instants (s09's lt is *exactly* its chest-tap key time),
  walk phase/bob chains, POSES bundle fields, shelf/plank/grass
  texture parameters — the seats' stated derivations agree with
  my independent recomputation.
