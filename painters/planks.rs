// ---------------------------------------------------------------------
// paint_planks — native port of the world.ts `T.planks` canvas painter
// (the film project's own code, NOT three.js library code):
//
//   ~/faceless-shorts/the-soldier-who-didnt-know/src/core/world.ts:58-65
//     planks: (col = '#A8693E', rep = [1, 1]) => canvasTex(
//       'planks' + col, 512, 512, (g, w, h) => { ... }, rep)
//   helpers it closes over:
//     rnd   — world.ts:10 (== lib.ts rnd; == crate::scene::rnd)
//     shade — world.ts:41 (THREE.Color multiplyScalar; see planks_shade)
//
// INTEGRATION (this file is a drop-in for src/scene.rs):
//   * Paste the items below into src/scene.rs next to paint_grass
//     (they use that module's Painter / Rgba / hex_rgb / rnd
//     unqualified, exactly like the sibling painters), OR include as a
//     submodule with `use super::{hex_rgb, rnd, Painter, Rgba};`.
//   * Register in src/scenefile.rs resolve_texture(), beside "grass":
//       "planks" => scene::paint_planks(str_field(spec, "base", ctx)?),
//   * Signature, size and determinism conventions mirror paint_grass
//     EXACTLY: `pub(crate) fn paint_*(base: &str) -> (Vec<u8>, u32,
//     u32)`, fixed 512x512 RGBA canvas, straight alpha, row 0 = top,
//     all randomness from the f64 sin-hash rnd() — no RNG state, so
//     two calls with the same base are byte-identical.
//   * `rep` (repeat) is NOT a painter input, in either engine: the
//     source applies it to the texture CLONE (canvasTex, world.ts:
//     24-33) while one canvas per colour is cached and shared. Repeat
//     and rotation live on the scene file's texture entry / material
//     UV transform. See painters/planks.md for the full mapping.
//
// Op-for-op transcription of the draw callback (world.ts:59-64).
// Canvas state carried across ops, as in the source: the joint rect
// reuses the seam's fillStyle (shade 0.45) because the source sets
// fillStyle once and never resets it between the two fillRects; the
// grain strokes' globalAlpha 0.35 is folded into the stroke colour's
// alpha byte (89), which is exactly how Painter::blend consumes it.
// ---------------------------------------------------------------------

/// three.js r170 `shade()` (world.ts:41):
///   const shade = (hex, k) => { const c = new THREE.Color(hex);
///     c.multiplyScalar(k); return '#' + c.getHexString(); };
/// With r170's default ColorManagement, `new THREE.Color(hex)` stores
/// the colour in the LINEAR-srgb working space, multiplyScalar scales
/// the linear components, and getHexString() converts back to sRGB
/// bytes. The conversions below are three's own constants
/// (ColorManagement.js: SRGBToLinear uses 0.0773993808 /
/// 0.9478672986 / 0.0521327014 with exponent 2.4; LinearToSRGB uses
/// 12.92 / 1.055 / exponent 0.41666), evaluated in f64 like all JS
/// number math, with the final byte ROUNDED, not truncated: the two
/// shade() outputs the film corpus recorded from the real pipeline
/// pin this — shade('#3A2A2A', 0.7) = #302222 and
/// shade('#3A2E22', 1.25) = #413427, whose green byte is 52 =
/// round(51.8); truncation would give 51 (#413327). See the tests.
///
/// This is deliberately NOT crate::scene::shade_rgb: that helper
/// multiplies the sRGB BYTES (it ports toon.ts's canvas shadeOf) and
/// runs 25-40 LSB darker on this painter's factors — e.g.
/// shade('#A8693E', 0.45) is (116, 71, 40) here vs (76, 47, 28) from
/// shade_rgb. The M0 audit (docs/audit/a1-engine.md, the
/// THREE.Color row) already flagged exactly this: "Reproduce that or
/// every derived shade shifts." paint_grass / film.rs still use
/// shade_rgb for world.ts painters — an inherited inconsistency in
/// the registry, documented in painters/planks.md; this module
/// follows the source.
fn planks_shade(hex: &str, k: f64) -> (u8, u8, u8) {
    fn s2l(c: f64) -> f64 {
        if c <= 0.04045 {
            c * 0.0773993808
        } else {
            (c * 0.9478672986 + 0.0521327014).powf(2.4)
        }
    }
    fn l2s(c: f64) -> f64 {
        if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(0.41666) - 0.055
        }
    }
    let (r, g, b) = hex_rgb(hex);
    let f = |v: u8| -> u8 { (l2s(s2l(v as f64 / 255.0) * k) * 255.0).round().clamp(0.0, 255.0) as u8 };
    (f(r), f(g), f(b))
}

fn planks_shade_rgba(hex: &str, k: f64) -> Rgba {
    let (r, g, b) = planks_shade(hex, k);
    (r, g, b, 255)
}

/// One grain curve (world.ts:61): the cubic bezier
///   moveTo(0, y)  bezierCurveTo(w*0.3, y+6,  w*0.6, y-6,  w, y+2)
/// flattened to a polyline for Painter::stroke_polyline. 128 segments
/// keep the polyline within ~0.01 px of the true curve everywhere
/// (the curve's total y excursion is 12 px over a 512 px run), far
/// under the stroke's own AA band. Canvas strokes this path with
/// butt caps; stroke_polyline's house model uses round caps — the
/// curves span the full canvas width, so the cap difference is
/// confined to the first/last pixel columns (see planks.md).
fn planks_grain_pts(w: f32, y: f32) -> Vec<(f32, f32)> {
    const SEG: usize = 128;
    let (x0, y0) = (0.0f64, y as f64);
    let (x1, y1) = (w as f64 * 0.3, y as f64 + 6.0);
    let (x2, y2) = (w as f64 * 0.6, y as f64 - 6.0);
    let (x3, y3) = (w as f64, y as f64 + 2.0);
    (0..=SEG)
        .map(|s| {
            let t = s as f64 / SEG as f64;
            let (u, v) = (1.0 - t, t);
            let px = u * u * u * x0 + 3.0 * u * u * v * x1 + 3.0 * u * v * v * x2 + v * v * v * x3;
            let py = u * u * u * y0 + 3.0 * u * u * v * y1 + 3.0 * u * v * v * y2 + v * v * v * y3;
            (px as f32, py as f32)
        })
        .collect()
}

/// T.planks(col): 512x512 canvas, 8 plank rows. Per row i
/// (world.ts:60-64), in source order:
///   1. row fill      shade(col, 0.86 + rnd(i*3)*0.24), full row
///   2. 4 grain strokes, shade(col, 0.7) @ alpha 0.35, lineWidth 2,
///      y = i*ph + 8 + rnd(i*7+k)*(ph-16), k = 0..4 — four SEPARATE
///      canvas strokes, so four separate stroke_polyline calls here
///      (crossings blend twice, as in the source)
///   3. seam band     shade(col, 0.45), fillRect(0, i*ph, w, 4)
///   4. butt joint    SAME fillStyle (0.45), fillRect(jx, i*ph, 4, ph)
///      with jx = rnd(i*11)*w
///   5. two knots     shade(col, 0.5), filled circles r = 3 at
///      (jx-14, i*ph + ph/2) and (jx+18, i*ph + ph/2)
///      (source: arc(x, y, 3, 0, 7) — 7 rad > 2*PI, a full circle)
pub(crate) fn paint_planks(base: &str) -> (Vec<u8>, u32, u32) {
    let (w, h) = (512usize, 512usize);
    let mut p = Painter::new(w, h);
    let n = 8usize;
    let ph = h as f32 / n as f32; // 64
    // globalAlpha 0.35 -> alpha byte: round(0.35 * 255) = 89.
    let (gr, gg, gb) = planks_shade(base, 0.7);
    let grain: Rgba = (gr, gg, gb, 89);
    let seam = planks_shade_rgba(base, 0.45);
    let knot = planks_shade_rgba(base, 0.5);
    for i in 0..n {
        let fi = i as f64;
        let top = i as f32 * ph;
        // 1. row fill
        let f = 0.86 + rnd(fi * 3.0) * 0.24;
        let fill = planks_shade_rgba(base, f);
        p.fill_rect(0.0, top, w as f32, ph, fill);
        // 2. grain strokes
        for k in 0..4usize {
            let y = top + 8.0 + (rnd(fi * 7.0 + k as f64) * (ph as f64 - 16.0)) as f32;
            let pts = planks_grain_pts(w as f32, y);
            p.stroke_polyline(&pts, 2.0, grain);
        }
        // 3. seam band (top 4 px of the row)
        p.fill_rect(0.0, top, w as f32, 4.0, seam);
        // 4. butt joint (same fillStyle as the seam in the source)
        let jx = (rnd(fi * 11.0) * w as f64) as f32;
        p.fill_rect(jx, top, 4.0, ph, seam);
        // 5. knots
        for cx in [jx - 14.0, jx + 18.0] {
            p.fill_ellipse(cx, top + ph / 2.0, 3.0, 3.0, 0.0, knot);
        }
    }
    (p.px, w as u32, h as u32)
}

#[cfg(test)]
mod planks_tests {
    use super::*;

    fn px_at(px: &[u8], w: usize, x: usize, y: usize) -> (u8, u8, u8, u8) {
        let k = (y * w + x) * 4;
        (px[k], px[k + 1], px[k + 2], px[k + 3])
    }

    #[test]
    fn output_size_and_opaque() {
        let (px, w, h) = paint_planks("#A8693E");
        assert_eq!((w, h), (512, 512));
        assert_eq!(px.len(), 512 * 512 * 4);
        // Every pixel is covered by an opaque row fill and every
        // later op blends OVER opaque, so alpha is 255 everywhere.
        assert!(px.chunks_exact(4).all(|c| c[3] == 255));
    }

    #[test]
    fn deterministic_and_base_sensitive() {
        let (a, _, _) = paint_planks("#B07A48");
        let (b, _, _) = paint_planks("#B07A48");
        assert_eq!(a, b, "same base must be byte-identical");
        let (c, _, _) = paint_planks("#9A6337");
        assert_ne!(a, c, "different base must differ");
    }

    /// shade() truth table. The first two rows are outputs the film
    /// corpus recorded from the REAL three.js pipeline (the s10
    /// bookshop data: sign edge #302222 = shade('#3A2A2A', 0.7);
    /// ceiling motif #413427 = shade('#3A2E22', 1.25)); the rest are
    /// this painter's own factors for the default base, computed
    /// from three r170's conversion constants in f64.
    #[test]
    fn shade_truth_table() {
        assert_eq!(planks_shade("#3A2A2A", 0.7), (0x30, 0x22, 0x22));
        assert_eq!(planks_shade("#3A2E22", 1.25), (0x41, 0x34, 0x27));
        assert_eq!(planks_shade("#A8693E", 0.45), (116, 71, 40));
        assert_eq!(planks_shade("#A8693E", 0.5), (122, 75, 43));
        assert_eq!(planks_shade("#A8693E", 0.7), (143, 89, 52));
        // k = 1 is the identity round trip.
        assert_eq!(planks_shade("#A8693E", 1.0), (0xA8, 0x69, 0x3E));
    }

    /// The seam band is painted AFTER the grain strokes, opaque, at
    /// shade(base, 0.45), across the full width — so every pixel of
    /// band row i*64+2 is exactly that colour (the butt joint shares
    /// the same fillStyle, and the knots sit at mid-row).
    #[test]
    fn seam_band_is_exact() {
        let (px, w, _) = paint_planks("#A8693E");
        let seam = planks_shade("#A8693E", 0.45);
        for i in 0..8usize {
            for x in [0usize, 17, 256, 511] {
                assert_eq!(px_at(&px, w as usize, x, i * 64 + 2), (seam.0, seam.1, seam.2, 255), "row {i} x {x}");
            }
        }
    }

    /// Row fills: pixel (x, i*64+63) — the plank's bottom row —
    /// is touched by nothing after the row fill when x is chosen
    /// 200 px away from the butt joint: grain curves stay above
    /// y = i*64 + ~60.2 (control-point max i*64+62 weighted by the
    /// bezier basis peaks at +2.7 over the endpoint max i*64+56,
    /// plus 1 px half-width and the AA band), knots sit at mid-row,
    /// seam at the top. Expected colours are shade('#A8693E', f_i)
    /// for f_i = 0.86 + rnd(3i)*0.24, computed from the algorithm.
    #[test]
    fn row_fills_are_exact() {
        let expected: [(u8, u8, u8); 8] = [
            (172, 108, 64),
            (169, 106, 62),
            (175, 110, 65),
            (174, 109, 65),
            (161, 100, 59),
            (172, 108, 64),
            (174, 109, 65),
            (159, 99, 58),
        ];
        let (px, w, _) = paint_planks("#A8693E");
        for i in 0..8usize {
            let jx = rnd(i as f64 * 11.0) * 512.0;
            let x = ((jx as usize + 200) % 512) as usize;
            assert_eq!(
                px_at(&px, w as usize, x, i * 64 + 63),
                (expected[i].0, expected[i].1, expected[i].2, 255),
                "row {i}"
            );
        }
    }

    /// Knot centres: the pixel containing a knot centre is at
    /// distance <= ~0.53 from it, so the r=3 disc covers the whole
    /// pixel (coverage 1) and the opaque knot colour replaces
    /// whatever the grain strokes left there. Row 0: jx =
    /// rnd(0)*512 = 419.488 -> knots at x = 405.488 and 437.488,
    /// y = 32. Row 3: jx = rnd(33)*512 = 184.326 -> x = 170.326
    /// and 202.326, y = 224.
    #[test]
    fn knot_centres_are_exact() {
        let (px, w, _) = paint_planks("#A8693E");
        let knot = planks_shade("#A8693E", 0.5);
        for (x, y) in [(405usize, 32usize), (437, 32), (170, 224), (202, 224)] {
            assert_eq!(px_at(&px, w as usize, x, y), (knot.0, knot.1, knot.2, 255), "knot at {x},{y}");
        }
    }

    /// Mean colour vs the value computed from the algorithm.
    /// Ignoring the grain strokes, the composition is exact: per
    /// 512x64 row, 30424 px of row fill shade(f_i), 2048 + 240 px
    /// of shade(0.45) (seam band + the joint's non-overlapping
    /// part), and ~56 px of shade(0.5) (two r=3 discs, area pi*9
    /// each, replacing row fill). The strokes are the only other
    /// term: they touch at most ~27% of pixels (4 curves x ~543 px
    /// x 4 px incl. AA fringe, per row) at alpha <= 89/255, moving
    /// a pixel at most toward shade(0.7). The tolerance is that
    /// bound, computed from the actual shade colours, + 1.5 LSB of
    /// slack for the disc-area and rounding approximations.
    /// For '#A8693E' the stroke-free expectation is
    /// (165.7, 103.6, 61.1).
    #[test]
    fn mean_colour_matches_algorithm() {
        let base = "#A8693E";
        let (px, w, h) = paint_planks(base);
        let npix = (w * h) as f64;
        let (mut sr, mut sg, mut sb) = (0f64, 0f64, 0f64);
        for c in px.chunks_exact(4) {
            sr += c[0] as f64;
            sg += c[1] as f64;
            sb += c[2] as f64;
        }
        let mean = (sr / npix, sg / npix, sb / npix);

        let seam = planks_shade(base, 0.45);
        let knot = planks_shade(base, 0.5);
        let grain = planks_shade(base, 0.7);
        let (mut er, mut eg, mut eb) = (0f64, 0f64, 0f64);
        let mut maxdist = (0f64, 0f64, 0f64);
        for i in 0..8usize {
            let f = 0.86 + rnd(i as f64 * 3.0) * 0.24;
            let fill = planks_shade(base, f);
            er += 30424.0 * fill.0 as f64 + 2288.0 * seam.0 as f64 + 56.0 * knot.0 as f64;
            eg += 30424.0 * fill.1 as f64 + 2288.0 * seam.1 as f64 + 56.0 * knot.1 as f64;
            eb += 30424.0 * fill.2 as f64 + 2288.0 * seam.2 as f64 + 56.0 * knot.2 as f64;
            for other in [fill, seam, knot] {
                maxdist.0 = maxdist.0.max((grain.0 as f64 - other.0 as f64).abs());
                maxdist.1 = maxdist.1.max((grain.1 as f64 - other.1 as f64).abs());
                maxdist.2 = maxdist.2.max((grain.2 as f64 - other.2 as f64).abs());
            }
        }
        let row_area = 512.0 * 64.0;
        let expect = (er / (8.0 * row_area), eg / (8.0 * row_area), eb / (8.0 * row_area));
        let tol = (
            0.27 * (89.0 / 255.0) * maxdist.0 + 1.5,
            0.27 * (89.0 / 255.0) * maxdist.1 + 1.5,
            0.27 * (89.0 / 255.0) * maxdist.2 + 1.5,
        );
        assert!((mean.0 - expect.0).abs() <= tol.0, "r: mean {} vs {} +- {}", mean.0, expect.0, tol.0);
        assert!((mean.1 - expect.1).abs() <= tol.1, "g: mean {} vs {} +- {}", mean.1, expect.1, tol.1);
        assert!((mean.2 - expect.2).abs() <= tol.2, "b: mean {} vs {} +- {}", mean.2, expect.2, tol.2);
    }
}
