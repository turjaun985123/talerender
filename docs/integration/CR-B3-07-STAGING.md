# CR-B3-07 — Previews staging convention (the NO-STAGING gap)

Contract: Chrome-replacement Batch 3, Seat 7. Deliverables (new
files only; nothing existing edited, nothing committed):

| File | Role |
|---|---|
| `~/workspace/chrome-replacement/batch3/B3-07/DESIGN.md` | The design: world staging, the poster rule, all rulings with costs |
| `~/workspace/chrome-replacement/batch3/B3-07/world_stage.py` | Prototype generator (stdlib): stage scenes + poster selection logic |
| `~/workspace/chrome-replacement/batch3/B3-07/validate_stage.py` | CPU load-validation mirroring scenefile.rs's documented laws |
| `~/workspace/chrome-replacement/batch3/B3-07/accounting.py` + `ACCOUNTING.json` / `ACCOUNTING.md` | All 318 items reclassified, named, with reasons |
| `~/workspace/chrome-replacement/batch3/B3-07/samples/` | 8 world scenes + 5 poster records; `validation-report.txt` |

**Standing rule honoured:** previews are a LOCAL functional
catalog only. Nothing here renders, installs, uploads, or commits
a preview.

## 1. The finding under the finding

There was no Chrome world staging to port. `castsheet.ts` stages
characters only; the 37 existing world/animated previews are
1080×1920 scene-shared frames reused across items (B2-05 hash
census: `set-meadow.jpg` ≡ `prop3d-house.jpg`, etc.). The
conventions below are new designs constrained by the era's facts
(1080×1920, no label bar on per-item files, world items shown in
a world, caption zone respected).

## 2. World staging (set / atmosphere / prop3d — 75)

One scene-format v2 stage scene per item: REF-STAGE (sky dome,
ground disc, hills, one tree, cloud billboards — film-corpus
vocabulary only), SET-CAM (`eye [0,2.6,10.5] → target [0,1.2,−1.5]`,
fovY 44) or the exact PROP-CAM rule (fovY 42; distance from the
bbox and the fov half-angle ×1.25; azimuth 35°, elevation 20°),
the **post-atmosphere** rig as data (R1), the preset's fog and
grade (grade.time pinned 0, R3). The generator's atmosphere
table is `env.ts` transcribed verbatim (all ten presets).

- **atmosphere (10): COMPLETE under the convention** — a preset
  is sky+fog+rig+grade; REF-STAGE is the constant. Samples:
  golden, fog.
- **set (14): convention complete, geometry is a per-set port.**
  Builders use painters with no native equivalent (facade /
  siding / brick / dirt / sand), InstancedMesh populations, and
  sprites. Prototype emits labelled SHELL-ONLY scenes (meadow,
  openSky — the no-floor variant) proving format/camera/rig;
  a shell must never be installed as a preview. Precedent that
  the port is possible: `scenes/sets/bookshop.scene.json` is
  the `interior` builder already transcribed for one project.
- **prop3d (51):** prop at origin on the REF ground, noon rig,
  PROP-CAM. Four builders transcribed faithfully in the
  prototype (chair, table, parcel, sandbags — the §6 `rnd` hash
  applied exactly for sandbag yaw). By audit: 42 mechanical
  ports (7 of them also carry a texture-painter gap), 6 blocked
  on TEXT, 3 on GEOMETRY (extrude ×2, points ×1).

## 3. Animated stills (shot / motion / clip — 243): the POSTER rule

Canonical staging at default index params; duration D resolved
in a fixed order (index `timing.seconds` → `duration`/`dur`
param default → the source function's own default duration,
transcribed → k-normalised D:=1 → static/state τ:=0 →
NO-DURATION, blocked by name); **poster time τ = D/2** under the
§15.8 time mapping; character items on the CHAR-STAGE (castsheet
staging verbatim), shot items on REF-STAGE with the move's
camera evaluated at τ, root-motion staged in place. State comes
from the §15 evaluator over transcribed tracks — this seat emits
poster *records* (params, D, τ, rule, host) for nod, walkNormal,
anticipation, pushIn, orbit, and invents no state values.
`previewSheet` is proposed as a six-frame strip at
[0, .2, .4, .6, .8, 1]×D, produced alongside but not gating
(ruling R-A4).

## 4. Updated accounting (all 318 named in ACCOUNTING.json)

| tier | n |
|---|---:|
| T1-CONVENTION (atmospheres) | 10 |
| T2-TRANSCRIBE | 145 |
| T2-PROBE (authored fixture needed) | 77 |
| T2-HARVEST (ActionClip builder evaluation) | 10 |
| T3-BLOCKED | 23 |
| T4-NO-FRAME (exempt-by-ruling recommended) | 53 |

**Generatable under the conventions: 242/318.** Of the 281
pending, 208 become generatable; 73 pending stay blocked (20 T3
+ 53 T4), so `--strict-previews` still fails until the T3 gaps
close *and* the T4 exemption is ruled. T3 by name: TEXT —
signpost, poster, platformScale, mailSack, train, chickCrate;
GEOMETRY — house, boat (extrude), stars (points); FORMAT —
shot-dutch (camera has no roll); APPENDAGE channels — tailChain,
tailPose, earFlick, earFlicks, hairSway, scarfWave,
appendageSpring, springFromAppendage, accessorySettle,
appendageLook; PAINTER — butterflyFollow, butterflyWander,
butterflyPerch.

## 5. Rulings needed (DESIGN.md §5, each with costs)

R-W1 set staging = default call (rec.) vs index snippet ·
R-W2 SET-CAM/PROP-CAM defaults (rec. as proposed) · R-W3 prop3d
on REF ground (rec.) vs alone vs dressed set · R-W4 scale figure:
off for prop3d/atmosphere, on for sets (rec.) · R-W5 port the
missing texture painters (rec.) vs substitute · R-W6 text via
the plate path generalised (rec.) vs native text painter vs
exempt · R-A1 τ = D/2 (rec.) vs authored τ vs peak τ · R-A2
root-motion in place (rec.) · R-A3 shot posters on REF-STAGE
(rec.) · R-A4 previewSheet produced, not gating (rec.) ·
R-A5 one canonical probe-fixture set (rec.) · R-A6 exempt the
53 T4 items per-item with reasons (rec.) · R-A7 camera-roll
format amendment (rec.) vs exempting dutch.

## 6. Verification (CPU only; renders UNVERIFIED-ENV)

- `world_stage.py --out samples` → 8 scenes + 5 poster records.
- `validate_stage.py samples/world` → **8/8 structurally valid,
  0 f32 violations** (parse-level mirror of the loader's laws;
  the binary's own load rides the PC/CI render leg).
- `accounting.py` → asserts 318, tiers sum to 318.
- No render attempted, none claimed.
