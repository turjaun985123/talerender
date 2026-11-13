# Gaps — stroke battery, native painter vs Chrome/Skia

Every known difference between the native cards and the reference
cards, each mapped to the audit finding or contract clause it belongs
to. (Filled in alongside the measurement; the structural entries
below hold regardless of the scores.)

1. **Flattening.** The native painter strokes quadratics flattened
   at a fixed 24 segments and ellipses at 144; Skia flattens
   adaptively to its own tolerance. Residual curve deviation is
   sub-0.1 device px at production radii but is not zero — it shows
   up in the profile RMSE floor, not in width/ramp. → a3 §1.1
   (curve flattening differs per rasterizer); contract §2.1 binds
   perceived weight/softness, which the battery measures directly.
2. **Clip masks are binary.** Canvas `clip()` in Skia carries an
   anti-aliased mask; the native clip masks (`polygon_mask`,
   `ellipse_mask`) are point-sampled. The affected edges are the
   laugh/shock tongue boundary and the body-pattern/shade clip —
   mostly covered by a stroke drawn over the same boundary (mouth
   outline, bean outline), so the exposure is the tongue edge only.
   Consistent with that bound, face-laugh is the worst face crop
   (band RMSE 0.081 vs the 0.12 tolerance). → a3 §5.4.
6. **Round joins are exact discs; Skia flattens join arcs too.**
   The CapFan chord model is applied at open ends (caps), where
   the battery has a dedicated axial-profile class. Interior
   round joins keep the exact-disc distance. The residual is
   bounded by the join crop classes: band RMSE 0.054–0.067
   against the 0.12 tolerance — the flattening sag (≤ 0.25 device
   px at the arc) is inside the crops' noise, but a dedicated
   join-arc profile class would be needed to resolve it, and the
   contract's bar is met without one. → contract §2; a3 §5.1.
7. **The cap fan is a radial model.** `fan_sd` prices the
   flattened cap boundary as R·cos(θ/2)/cos(φ − φ_mid) along the
   ray from the cap centre: exact at chord vertices and chord
   midpoints, within ~0.05 device px elsewhere at production
   radii (R 9–15 units → 18–30 device px). The cap-end class
   matches to 0.000 on all three measures, so the approximation
   is below this battery's resolution; it is recorded here
   because it is an approximation, not the chord polygon itself.
   → README, "Three findings", #1.
3. **Mood/look subset.** `paint_toon` ports the drawFace branches
   the battery exercises (5 moods, pupil/lid/arc/shock eyes, 5 mouth
   shapes, cap or no hat, plain pattern). The remaining 14 moods,
   hats, patterns, neckwear, props and the kid/elder look geometry
   are not yet ported — they are painter *coverage* of the catalogue,
   not rasterisation differences, and the battery's verdict does not
   extend to them. → a3 §4 (full state/look inventory).
4. **Text is out of scope here.** §2.4 (caption fit under Inter 900)
   is a separate clause with its own corpus test; no battery card
   draws text, matching the fact that the toon canvas's identity
   surface is its strokes. In-world text props ride the same font
   stack question and are likewise excluded.
5. **Premultiplied-alpha path (§2.3/§2.5).** The battery compares
   the painters' straight-alpha canvas output. The downstream path —
   CanvasTexture upload, mipmaps, the dual alphaTest thresholds
   (0.04 colour / 0.5 shadow) — is exercised end-to-end only by the
   ab1 frame regression, not isolated here.
