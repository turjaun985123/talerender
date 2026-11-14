# Production integration — capability inventory

Target film: **"THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER"**
(the Onoda project, `~/faceless-shorts/the-soldier-who-didnt-know`).
60.357 s narration (`data/lines.json` duration), 12 narration
lines cut into **14 shot segments** (L4 and L7 take two shots
each), 30 fps → **1,811 frames** (n = 0..1810, t = n/30). Audio:
the project's existing `data/mix.wav` (60.46 s, −14.0 LUFS
master) is muxed unchanged. Chrome references: `review/frames/`
holds a 1080×1920 JPEG every second (t0.50 … t59.50) plus
`review/all.jpg` and `review/NOTES.md`.

Method: every staging fact below was read first-hand from the
project's `src/` (shots.ts 544 lines in full; core/{toon,world,
env,engine,motion,poses,props,cast,captions,timing,lib}.ts) and
its `data/` + `script.json`. Native state is what the engine at
HEAD `9a8cc79` actually contains — not what milestones planned.

Verdicts: **ready** = exists and proven · **work** = missing piece
named, buildable inside this seat · **blocked** = cannot be done
faithfully in this seat; named with reason (rule: never substitute
silently).

## Subsystem inventory

| Subsystem | State | Missing piece |
|---|---|---|
| JG jungle-camp set (sky, grass, hills, 19-tree ring, 3 palms, 900 tufts, camp: fire/stones/logs/lean-to/crates/barrel/bedroll) | **ready** — proven in ab1 at M5 (23.85 dB) | atmosphere variants change the rig (below) |
| SH shore set (sand, sea, foam, hills, trees, palms, boat, crates) | **work** | authoring only: sand disc, sea/foam planes, boat boxes — all existing PBR primitives |
| BK bookshop (interior) | **work** | wallpaper + planks painters (registry has grass/sky/cloud/blob only); shelf-unit builder with deterministic book rows; desk/parcel builders; sign text texture; bulb point lights. The source interior's own window view is a flat colour plane — no hidden depth |
| Light rig (key/rim/hemi/point + shadow) | **ready** in the ab1 renderer; **work** in the format | format v1 cannot express it (§12.3) → format v2 `lights` block |
| Atmospheres noon/fog/golden/dusk (env.ts) | **work** | per-shot rig/sky/grade parameter sets (values all read); **fog**: three.js `scene.fog` reaches every material — the native PBR/basic shaders have **no fog term** → fog support must be added to the shaders, or S06 is blocked (see below) |
| Character painter (`scene::paint_toon`) | **work — the largest item** | native painter is the M4 subset: 5 moods, stand legs, cap only. The film needs: moods {neutral, determined, suspicious, happy, laugh, sad, tired, worried, shock, awe, relieved}, walk + sit legs, squash, headTilt, blink, talk mouth, look offsets, bowler hat, vest pattern, tie, scarf, glasses, mustache, hold props (paper MAP, book open/closed, leaflet, coconut), sitOn. All geometry is specified in toon.ts (read in full); the M4 stroke model carries over |
| Character card placement (face(): yaw + perspective pitch, head plane, blob, envMatch tint) | **ready** | formulas in telemetry.rs + ab1 (pitch, tint uniform) |
| Rifle (held via syncAttachments; shore; ground drop) | **work** | ab1 has the 5-part rifle draws + hand-anchor formula for one pose; generalise to per-frame hand anchor + the S11 drop animation |
| Plane + 16 leaflets + falling leaflet (S05) | **work** | authoring: cone/cyl/boxes PBR + DoubleSide paper planes (basic pipeline is back-face culled → needs a double-sided variant or two draws) |
| Calendar overlay (S07, 2D post-blit) | **work** | CPU overlay painter (rounded-rect card + Inter text) composited post-grade like the caption; text enters via the project's own Inter fonts |
| Captions | **ready as data** | chunk timing from script.json `caps` + words.json (pure data); widths measured, not typeset natively (M6 pattern); strips rendered by PIL with the project's Inter-Black, composited post-grade exactly as ab1 |
| Camera moves (push-ins, cranes) | **work** | format v2 keyframed camera; evaluator ports `camK` smootherstep + `crane` lerp (motion.ts, read in full) |
| Motion formulas (poseTrack, walkPhase/walkBob, blinkDirector, blinkAt, settle, talk, seg/eo) | **work** | closed-form ports into the film evaluator; poseTrack already exists exactly (f64) in telemetry.rs |
| Film fades (in 0.25 s / out 0.4 s, main.ts) | **work** | trivial post-grade multiply, part of `draw(t)` in the source — must not be forgotten |
| Telemetry gate over the film | **ready** | M6 Audit machinery (0.25 s sampling, overflow/continuity/composition/captions) driven from the film evaluator instead of the ab1 staging |
| Finishing (x264 slow CRF 18, BT.709, faststart, .part) | **ready** | pipeline settings from A2 §4.4; ffmpeg 8.1.2 on this VM |

## Shot-by-shot table

Sets: JG jungle camp · SH shore · BK bookshop. Grades: G.jungle /
G.shore / G.shop + atmosphere.

| Shot | t (s) | Set / grade | Characters & action | Props | Camera | Overlay | Verdict |
|---|---|---|---|---|---|---|---|
| S01 | 0.00–5.91 | JG noon | Onoda stand, head sweep poseTrack, determined | rifle held, fire lit | push-in 44° | caption | **ready** (proven scene; blink channel is the only painter gap) |
| S02 | 5.91–13.06 | SH noon | Onoda + compA + compB march (walk legs, phase-locked) | rifle held | crane down 46° | caption | work: SH set, walk legs |
| S03 | 13.06–15.95 | SH noon | Taniguchi explains + talks; 3 soldiers at attention | — | push-in 54° | caption | work: talk mouth, bowler/vest/tie/glasses/mustache |
| S04 | 15.95–21.38 | JG noon | compA/B celebrate (laugh/happy, bounce bob); Onoda FG suspicious sweep | rifle, fire | push-in 50° | caption | work: celebrate arms exist as data; laugh mood ready |
| S05a | 21.38–24.56 | JG noon | — (no people) | plane flyover, 16 falling leaflets, fire out | crane 46° | caption | work: plane/leaflets authoring |
| S05b | 24.56–26.13 | JG noon | Onoda reads leaflet, head-shake settle, drops it | leaflet hold prop + falling leaflet mesh, fire | push-in 42° | caption | work: leaflet prop painter |
| S06 | 26.13–30.39 | JG **fog** | compA/B walk away (sad/tired); Onoda sits (sit legs, sitOn 0.34, mourn), fire dies | rifle, dying fire | push-in 46° | caption | work: fog in shaders, sit legs |
| S07 | 30.39–34.21 | JG **golden** | Onoda walks to palm, reaches, holds coconut | coconut prop, rifle | crane up 46° | caption + **calendar card** | work: calendar overlay painter |
| S08 | 34.21–40.36 | JG noon | Suzuki walks in with MAP (shock at "found"); Onoda steps out | paper MAP prop, rifle, fire | push-in 62° | caption | work: paper prop painter |
| S08b | 40.36–41.73 | JG noon | Onoda head-shake on "refused"; Suzuki listen pose, worried | MAP, rifle, fire | push-in 54° | caption | work: none beyond S08 |
| S09 | 41.73–45.37 | JG noon | Onoda chest-tap poseTrack; Suzuki listen→think | MAP, rifle, fire | push-in 54° | caption | work: none beyond S08 |
| S10 | 45.37–50.65 | BK noon (G.shop) | Taniguchi in bookshop: awe→determined, closes book, short walk | book LEDGER open/closed, shelves, desk, parcels | push-in 46° | caption | work: BK set, book prop |
| S11 | 50.65–55.52 | JG **dusk** | Taniguchi gives order (arm track, talk, bow); Onoda walks in, instance-swap at dropT, lays rifle down, attention | rifle held → rifle on ground (animated drop), fire | push-in 48° | caption | work: rifle drop animation |
| S12 | 55.52–60.36 | JG **dusk** | Onoda2 small bow on "loyal"; Taniguchi still | rifle on ground, fire | crane up/away 46° | caption | work: none beyond S11 |

## Blocked list

**Nothing is blocked outright.** Two items carry declared
residual risk rather than a block:

1. **S06 fog fidelity.** three.js fog is per-fragment linear
   fog on every material including the character cards. The
   native shaders have no fog term today. Plan: add the fog
   term (colour/near/far uniforms) to the PBR + basic shaders
   with the shot's exact values (0xC9CFC2, 4.5, 20). If the
   shader work fails verification, S06 ships with the fog
   *grade* (sat 0.52, atmosphere rig) and the missing depth-fade
   is declared in the film docs — not silently.
2. **Caption width measurement.** Widths are measured with PIL
   against the project's Inter-Black.otf — the same font file
   the Chrome canvas uses, but PIL's metrics are not Skia's.
   Only the `fit` scale of over-wide chunks can differ by a
   small factor; chunk text and timing are exact. Declared in
   the film docs (the M6 "measurement enters as data" limit,
   carried).

## Build order (this seat)

1. Format v2 (lights, character node, camera keys, time) —
   loader + spec + v1 regression.
2. Painter completion to the film's channel set (above).
3. Film evaluator + driver (chunked, resumable, deterministic).
4. Caption/calendar asset generation (PIL, project fonts).
5. A/B per shot → full render → telemetry → MP4 → docs.
