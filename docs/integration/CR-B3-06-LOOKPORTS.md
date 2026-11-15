# CR-B3-06 — G3/G4 painter ports, wave 1 (S-effort features)

Seat B3-06, Batch 3. Source assessment: B2-10
`look_completeness.md` (27 PORT designs). Base: talerender main
`63cc816`. Deliverables (patches + harness + raw results) live in
`~/workspace/chrome-replacement/batch3/B3-06/` — the repo was
never edited in place; the lead lands the patches.

## Scope landed: 15 features, all PORTED-PROVEN

Every S-effort, painter-local row of the assessment: the four
extras (blush condition extension, freckles, pencil, monocle),
seven hats (boater, beanie, visor, tophat, bun, flower,
mailcap), the bow hat (assessed S–M; included because it is
painter-local and its only extra field, `hairCol`, is a 1-line
loader read), collar, stripes, apron.

## Files / fields touched

| File | Change |
|---|---|
| `src/toonfull.rs` | `FullLook` +5 fields (below); new local helper `rounded_rect_pts_radii` (per-corner radii — scene's `rounded_rect_pts` takes one radius; TS uses `[16,16,4,4]` etc.); pattern arms `stripes`, `apron`; neckwear arm `collar`; face: monocle after glasses, blush condition extended, freckles after blush, pencil after mustache; bun at the head-part site **before** the head disc; hat arms boater/beanie/visor/tophat/bow/mailcap/flower |
| `src/scenefile.rs` | character-branch loader: `extra` → `blush`/`freckles`/`pencil`/`monocle`; `hairCol` → `hair_col` (the G4 loader half) |
| `src/film.rs` | `fl()` defaults the 5 new fields (false/None); `state_key` comment count 14 → 19 with the reasoning recorded |

New `FullLook` fields: `blush: bool`, `freckles: bool`,
`pencil: bool`, `monocle: bool`, `hair_col: Option<String>`.

**State-key audit (the sweat/tears defect class):** this wave
adds **no `FullState` field**, so `film.rs::state_key` and
`charstate.rs` need no functional change. The five new fields
are look fields: the film PaintCache keys a look by registry
index (`state_key(look: usize, …)`), every registry look is
built by `fl()` which now defaults all five, and the scenefile
path paints uncached (`scenefile.rs` calls `paint_toon_full`
directly). No two paints that differ only in a new field can
share a cache key. One stale text remains for the lead:
`charstate.rs`'s header still says "FullLook has 14 fields" —
it is a dated audit narrative, left untouched here.

## Proof (harness in the seat dir; geometry-level, per charter)

Two scratch crates (`harness/before`, `harness/after`) compile
the real `scene.rs` + `toonfull.rs` (before = pristine
`63cc816`, after = patched) with the local rustc, release mode.

**(a) Additivity — PASS.** A 136-combo matrix with no new
features (7 moods × 3 hats × 2 patterns × 3 necks, glasses +
mustache on, plus 10 varied-state paints: tilt/turn/squash/
camPitch, walk, sit+tears, jump+sweat, run) hashes
**byte-identical before/after on all 136** (FNV-1a over pixels
+ hand positions; `results_matrix_{before,after}.txt`, `diff`
empty). Pre-patch drop evidence: boater/stripes/collar in the
before harness give diff_px = 0 (`results_features_before_drop.txt`).

**(b) Fidelity — PASS, 29/29 probes.** Each feature painted at
the default state; probe pixels checked against TS-predicted
geometry/colours (stage constants hx 256, hy 178, rp 138,
ey 182, htop 44.14). Full log: `results_features_after.txt` +
`results_features_alpha_fixed.txt`.

| Feature | diff px vs base | Key measurements |
|---|---|---|
| blush | 6,800 | centre probes exact as blend-over-base (237,196,201); nominal ellipse area 6,635 (+2.5% = AA edge) |
| freckles | 1,032 | 3 disc-centre probes exact `#C98A64`; 792 exact-colour px (nominal 924; AA interior ≈800) |
| pencil | 9,736 | body centre `#F2C230` exact; eraser centre at the −0.5-rotated position `#E58A9A` exact; yellow 5,216 px |
| monocle | 6,752 | ring-top probe exact `#C9A13B`; 5,638 gold px (ring + chain) |
| collar | 11,936 | both triangle interiors pure white; white total 5,286 = nominal 8,832 − inner stroke band (perimeter ≈615 canvas px × 6 px ≈ 3,500 over both triangles), as TS's fill-then-stroke produces |
| stripes | 55,896 | stripe interior exact `shade(body,0.7)`; gap pixel exact body colour; 49,188 stripe px |
| apron | 90,718 | body probe exact `#F4EFE4`; pocket-stroke probe exact `shade(…,0.8)`; fill 76,096 px; pocket-inside-bean probe PASS at all 10 `by` values (−60…210, 5 px erosion ≥ 4 px half-stroke + AA); analytic min contour-to-boundary distance 3.79 stage units > 2.0 half-width |
| boater | 107,365 | band `#3C6FB6`, brim + crown-between-hatch probes exact |
| beanie | 116,444 | dome, brim `shade(c,0.8)`, pom `#F4EFE4` probes exact |
| visor | 50,923 | sheet exact as blend-over-base (95,197,138); band probe exact |
| tophat | 143,176 | crown, band `#B5372E`, brim probes exact |
| bun | 20,380 | centre probe exact `#6B4A3A` (drawn pre-disc, per TS) |
| flower | 12,212 | centre `#F6C945`, petal `#F29BB5` probes exact |
| bow | 56,447 | knot, loop, lock probes exact; with `hairCol=#224422` the lock probe is exactly that colour (same diff count — geometry unchanged) |
| mailcap | 75,051 | crown, badge `#E2B33C`, INK brim probes exact |

**Alpha-probe note (a finding about the base painter, not this
patch):** blush/visor centre pixels sit on base colour
(236,236,239), not flat head `#F7F7FA` — the native head-shade
pass is a soft disc where TS paints a hard even-odd crescent.
Pre-existing at `63cc816`, byte-identical in the matrix, out of
this seat's scope; flagged for the lead.

## Per-feature verdicts

| Feature (B2-10 ref) | Verdict |
|---|---|
| blush (§1.6) | **PORTED-PROVEN** |
| freckles (§1.5) | **PORTED-PROVEN** |
| pencil (§1.7) | **PORTED-PROVEN** (rotation direction settled by the eraser-centre probe) |
| monocle (§1.8) | **PORTED-PROVEN** |
| boater (§2.1.1) | **PORTED-PROVEN** |
| beanie (§2.1.2) | **PORTED-PROVEN** |
| visor (§2.1.3) | **PORTED-PROVEN** |
| tophat (§2.1.4) | **PORTED-PROVEN** |
| bun (§2.1.5) | **PORTED-PROVEN** |
| flower (§2.1.6) | **PORTED-PROVEN** |
| bow + hairCol (§2.1.7 + §1.4) | **PORTED-PROVEN** (hairCol's other consumer, hair strands §1.11, is M — not in this wave) |
| mailcap (§2.1.8) | **PORTED-PROVEN** |
| collar (§2.2.2) | **PORTED-PROVEN** |
| stripes (§2.3.2) | **PORTED-PROVEN** |
| apron (§2.3.4) | **PORTED-PROVEN** (unmasked pocket stroke justified by the erosion probe above) |
| check + patCol2 (§2.3.1/§1.1), bowtie + dots (§2.2.1/§1.2), overalls (§2.3.3), stamps (§2.3.5/§1.3) | DESIGN-ONLY — M/L effort, outside this seat's S scope; designs stand in B2-10 |
| ears/tail/hair/wing/scarf-streamer (§1.9–§1.13) | DESIGN-ONLY — M; each needs new `FullState` fields, so their patches must carry the `state_key` + charstate wiring in the same change (B2-10 fact 2) |

## Landing notes for the lead

- Patches: `toonfull.patch`, `scenefile.patch`, `film.patch`
  in the seat dir; verified to apply cleanly (`patch -p1
  --dry-run`) against pristine `63cc816` copies.
- `toonfull.rs` is compile- and run-proven locally via the
  harness. `scenefile.rs`/`film.rs` changes are field additions
  at the **only two** `FullLook` constructors in the repo
  (grep-verified); their full-crate compile rides the lead's CI
  landing, as do the golden/telemetry render gates (this VM has
  no Vulkan ICD).
- Skill-side follow-ups (not in these patches): shrink
  `castsheet_native.py`'s `NATIVE_LOOK_DROPPED` list by the 15
  features; B2-10 §5's painter census test remains recommended.
