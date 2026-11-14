# Calendar overlay painter — `painters/calendar.rs`

Seat W25 deliverable notes. The module rasterises the S07
calendar card (the Onoda project's `drawCalendar`, shots.ts:295;
data spec `scenes/film/s07-calendar.json`) as a standalone
1080×1920 RGBA overlay, or composites it over a frame buffer —
the vector alternative to film.rs's PIL-plate path
(`overlay_calendar` + `film/assets/calendar-*.png`).

**Text approach: embedded minimal glyphs.** The codebase has no
Inter glyph rasteriser (text enters the film as PIL plate data;
an Inter measurer is noted as future work in telemetry.rs), so
the three fixed strings are drawn from hand-authored vector
glyphs embedded in the module — rects, discs and polygons on a
1000-unit em, boolean union-minus per glyph, rasterised by the
same 4×4 inside-test supersample the house painter uses.
Glyph set: `A B C D E G I L N R S U`, digits `1 4 5 6 7 9`,
space — everything 'C A L E N D A R', the four years and
'LUBANG ISLAND' need, nothing more.

Shapes, blending and coverage follow src/scene.rs's `Painter`
(M4) exactly: straight-alpha src-over quantised to u8 per blend,
signed-distance coverage `cov = clamp(0.5 − sd)` with the 4×4
footprint estimator inside |sd| < 1, samples at pixel centres.

## t → year mapping (film clock; lt = t − 30.55)

| film t | lt | year | sy | dark | note |
|---|---|---|---|---|---|
| 30.55 | 0.00 | 1945 | 1 | 0 | card appears, alpha 0 |
| 30.64 | 0.09 | 1945 | 1 | 0 | alpha 0.5 (fade-in) |
| 30.73–31.17 | 0.18–0.62 | 1945 | 1 | 0 | alpha 1, first hold |
| 31.17–31.38 | 0.62–0.83 | 1945 | 1 → 0.1 | 0 → 0.9 | flip 1, first half |
| 31.38 | 0.83 | **1955** | 0.1 | 0.9 | u = 0.5: year switch, max squash/shade |
| 31.38–31.59 | 0.83–1.04 | 1955 | 0.1 → 1 | 0.9 → 0 | flip 1, second half |
| 31.59–32.21 | 1.04–1.66 | 1955 | 1 | 0 | second hold |
| 32.21–32.63 | 1.66–2.08 | 1955 → 1965 | dips to 0.1 at 32.42 | to 0.9 | flip 2 (switch at film 32.42) |
| 32.63–33.25 | 2.08–2.70 | 1965 | 1 | 0 | third hold |
| 33.25–33.67 | 2.70–3.12 | 1965 → 1974 | dips to 0.1 at 33.46 | to 0.9 | flip 3 (switch at film 33.46) |
| 33.67–34.209 | 3.12–3.659 | 1974 | 1 | 0 | steady; **the shot cuts here**, alpha still 1 |
| (34.60–34.95) | (4.05–4.40) | 1974 | 1 | 0 | fade-out — never reached inside S07 |

Unit tests sample the required instants: t = 31.0 → 1945,
31.5 → 1955, 32.5 → 1965, 33.5 → 1974.

## Geometry checks

- Card on frame at sy = 1, before rotation: x 204.8…499.2,
  y 464…832 (320×400 at scale 0.92 about (352, 648)); the
  −0.03 rad rotation moves corners by ≲ 12 px. The module culls
  to the forward-mapped plate bbox (frame ≈ x 174…530,
  y 430…866 at sy = 1).
- Everything drawn fits the plate frame (±180 × ±220 local):
  card + centred stroke reach |x| 163.5, |y| 203.5; the drop
  shadow reaches (168, 210) — ≥ 10 units of margin everywhere.
- Text fit (embedded advances, card units vs the 320 width):
  header 'C A L E N D A R' @30 → 215.4 (67%); year @118 →
  283.2 (88.5%); 'LUBANG ISLAND' @25 → 199.5 (62%). No string
  can touch the stroke.
- Vertical layout: header text band −176.8…−155.2 inside the
  header band −200…−134; year band −34.5…+50.5; sub band
  109…127; rule at 158…163 — 31 units clear of the sub text.
- Digit width vs Inter: Inter's tabular figures advance
  0.6020 em; the embedded digits use 0.60 em, so a year is
  283.2 local units wide against Inter's ≈ 284.2 — 0.4%.

## Fidelity limits

1. **The glyphs are not Inter.** They are geometric
   approximations (stem ≈ 0.18 em) of Inter ExtraBold (the two
   800 strings) and Inter Black (the 900 year). C and G are true
   circles where Inter is slightly squared; the S is two
   circular bowls with slit counters; the 4 is the open form;
   the 5's bowl is a full ring; the 9/6 tails are straight
   polygons, not curves. Advances are hand-set (digits
   near-exact, see above; the header's letter-spaced caps will
   sit slightly tighter than Inter's). The PIL plates remain
   the reference rendering; this path trades exact letterforms
   for a self-contained, resolution-independent card.
2. **Text AA** is a 4×4 box supersample of the inside test —
   marginally softer than Skia's analytic coverage. Shape
   layers use the house SDF coverage unchanged.
3. **Sampling at pixel centres** (the painter's convention);
   film.rs's plate path samples at pixel indices — up to
   0.5 px registration difference against the plate composite.
4. **Flip-frame AA:** SDF coverage is measured in card-local
   units, so during a flip the frame-space AA ramp narrows by
   sy vertically (near-hard edges at sy = 0.1), where the plate
   path's bilinear filtering softens instead. Confined to the
   three 0.42 s flip windows.
5. **Global alpha** is applied as a final scale of the
   composited alpha — exactly film.rs's plate treatment. It
   differs from canvas's per-draw globalAlpha only where
   translucent layers overlap (shadow under the card edge),
   by ≤ 1 LSB after u8 quantisation.
6. **Rule colour discrepancy in the shipped plates:**
   drawCalendar's `fillRect` inherits fillStyle `#8B8378`, and
   the data spec records `#8B8378` — but gen_assets.py paints
   the plate's rule with INK `#1F1C33`. This module follows the
   source/data spec (`#8B8378`); the plate path differs on that
   one 220×5 rect.
7. **Knife-edges:** the state law mirrors the source's f64
   arithmetic, including the u = 0.5 year switch and the
   lt = 4.4 visibility edge, where floating-point dust decides
   — the same behaviour class as the JS original and film.rs.
   (film.rs's `calendar_state` uses `local > 0.62` where the
   source uses `ph >= 0.62`; at ph = 0.62 exactly, u = 0 and
   year/sy/dark are identical either way. This module follows
   the source.)

## Verification

- 7 unit tests in the module (alpha law at the boundaries,
  years at 31.0/31.5/32.5/33.5, sy floor at the flip midpoint,
  exact card/header/rule/ink pixel colours, mid-flip shading
  ≈ (202, 194, 176), text-fit, off-screen) — **all passing**
  under a standalone build:
  `rustc --edition 2021 --test painters/calendar.rs`
  (the file is std-only, so this checks it exactly; the crate
  itself was not built locally, per the CI-only rule, and
  nothing in `src/` was touched).
- Visual check of rendered overlays at lt 0.3 / 0.95 / 2.3:
  'CALENDAR', '1945', '1965' and 'LUBANG ISLAND' all legible;
  the lt 0.95 frame shows the 1955 card squashed (sy ≈ 0.78)
  and flip-shaded, as staged.

## Wiring for the integrator

`state_at(t)` returns the same fields film.rs's `CalendarState`
carries (`year` there is the 0…3 index — here `year_idx`, with
the numeral in `year`). To use this path in the driver: add the
file as a module and call `composite_over(&mut px, w, h, t)` at
the point film.rs calls `overlay_calendar` (inside the shot
draw, before fade and caption), or take `render_overlay(t)` as
a standalone buffer.
