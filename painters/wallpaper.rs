//! T.wallpaper — the bookshop wallpaper texture painter, ported
//! op-for-op from the project's world.ts (world.ts:69-73) for the
//! scene-format texture registry (docs/scene-format.md; the registry
//! gap is B8 in scenes/sets/README.md).
//!
//! INTEGRATION: this file is the registry version of the painter
//! film.rs already carries as a local duplicate (film.rs's
//! paint_wallpaper, film.rs:526). Drop it into src/scene.rs beside
//! paint_grass / paint_sky (deleting the `use` line below — those
//! items are scene.rs's own) and register it in
//! src/scenefile.rs::resolve_texture as:
//!
//!     "wallpaper" => scene::paint_wallpaper(
//!         str_field(spec, "base", ctx)?,
//!         str_field(spec, "motif", ctx)?,
//!     ),
//!
//! The spec fields are the ones the film data already declares
//! (scenes/sets/bookshop.scene.json, scenes/film/s10.scene.json):
//! `base` (the TS `col`), `motif`, `canvas: [256, 256]`, plus a
//! `repeat` the painter does not consume (repeat is a material-side
//! clone property in the source too — canvasTex keys its cache by
//! 'wp' + col + motif only and clones per repeat).
//!
//! Signature and conventions mirror the registry painters exactly:
//! hex-string colour parameters in, (RGBA bytes, w, h) out, painted
//! through the shared CPU Painter with its signed-distance coverage
//! model. The source painter uses NO rnd() calls — it is fully
//! geometric — so determinism here is exact, not hash-dependent.
//!
//! Two interpretation points (full discussion in
//! painters/wallpaper.md): the two wing arcs share ONE canvas path
//! in the source; they are painted as two discs because Chrome/Skia
//! clamps an arc sweep to a full circle, which makes the inter-arc
//! connector degenerate — the same reading film.rs's port took. And
//! the fleur position list has four entries but an `i < 2` guard
//! draws only the first two.

use crate::scene::{flatten_cubic, hex_lin, linear_to_srgb, rgba_of, Painter};

/// world.ts:35's `shade(hex, k)`: THREE.Color(hex).multiplyScalar(k)
/// read back with getHexString(). THREE.Color stores its components
/// in the LINEAR working space (the hex is converted on
/// construction), so the multiply happens in linear space and the
/// read-back re-encodes to sRGB (clamped, rounded) — NOT the canvas
/// shadeOf() byte multiply scene::shade_rgb implements. The two
/// visibly differ: shade('#3A2E22', 1.25) is #413427 this way and
/// #493A2B the shade_rgb way (scenes/film/ASSUMPTIONS.md, S10
/// divergences item 3 — the source value is #413427).
///
/// The wallpaper painter itself never calls shade() — interior()
/// pre-computes the ceiling motif with it (world.ts:313) and the
/// film data carries the result — but the registry consumer that
/// derives a variant from parameters needs it computed the source's
/// way, and the unit tests pin it against the evaluated constant.
pub(crate) fn shade_hex(hex: &str, k: f32) -> (u8, u8, u8) {
    let lin = hex_lin(hex);
    let f = |c: f32| -> u8 {
        (linear_to_srgb(c * k) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    (f(lin[0]), f(lin[1]), f(lin[2]))
}

/// T.wallpaper(col, motif): 256x256 — base fill, fleur motifs at
/// (64,64) and (192,192) at scale 0.9, then four alpha-0.5 dots
/// (world.ts:69-73). The source also sets strokeStyle/lineWidth = 5
/// before drawing; no stroke() is ever called, so there is nothing
/// to port for them.
pub(crate) fn paint_wallpaper(base: &str, motif: &str) -> (Vec<u8>, u32, u32) {
    let (w, h) = (256usize, 256usize);
    let mut p = Painter::new(w, h);
    p.fill_rect(0.0, 0.0, w as f32, h as f32, rgba_of(base));
    let m = rgba_of(motif);
    // fleur(x, y, s): translate(x, y) then scale(s, s), all fills in
    // the motif colour. One fleur is:
    //   petal — moveTo(0,-40), bezierCurveTo(18,-20, 18,0, 0,14),
    //           bezierCurveTo(-18,0, -18,-20, 0,-40), fill;
    //   wings — one path: arc(-22,4,12, 0,7) + arc(22,4,12, 0,7),
    //           fill (two discs — see the header note);
    //   stem  — fillRect(-3, 10, 6, 26).
    let fleur = |p: &mut Painter, cx: f32, cy: f32, s: f32| {
        let mut outline = flatten_cubic((0.0, -40.0), (18.0, -20.0), (18.0, 0.0), (0.0, 14.0), 24);
        let back = flatten_cubic((0.0, 14.0), (-18.0, 0.0), (-18.0, -20.0), (0.0, -40.0), 24);
        // Skip the return curve's first point (the joint, already
        // the forward curve's last) and its last (the start point,
        // which fill_polygon closes to on its own).
        outline.extend(back[1..back.len() - 1].iter().copied());
        let pts: Vec<(f32, f32)> = outline
            .iter()
            .map(|&(x, y)| (cx + x * s, cy + y * s))
            .collect();
        p.fill_polygon(&pts, m);
        p.fill_ellipse(cx - 22.0 * s, cy + 4.0 * s, 12.0 * s, 12.0 * s, 0.0, m);
        p.fill_ellipse(cx + 22.0 * s, cy + 4.0 * s, 12.0 * s, 12.0 * s, 0.0, m);
        p.fill_rect(cx - 3.0 * s, cy + 10.0 * s, 6.0 * s, 26.0 * s, m);
    };
    // [[64,64],[192,192],[192,64-256],[64,192]] with `i < 2 &&` —
    // only these two fleurs exist; the other two positions are dead
    // list entries (one of them, (192,-192), is off-canvas anyway).
    fleur(&mut p, 64.0, 64.0, 0.9);
    fleur(&mut p, 192.0, 192.0, 0.9);
    // globalAlpha 0.5 dots, r 6, at the four grid points in source
    // order: arc(i%2 ? 192 : 64, i<2 ? 192 : 64, 6, 0, 7). Alpha 128
    // for the 0.5 (0.5 * 255 = 127.5; the house port precedent —
    // film.rs's paint_wallpaper uses the same literal). Two of the
    // dots land on fleur centres (motif over motif: no visible
    // change); two land on bare base.
    for i in 0..4 {
        let (dx, dy) = match i {
            0 => (64.0, 192.0),
            1 => (192.0, 192.0),
            2 => (64.0, 64.0),
            _ => (192.0, 64.0),
        };
        p.fill_ellipse(dx, dy, 6.0, 6.0, 0.0, (m.0, m.1, m.2, 128));
    }
    (p.px, w as u32, h as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(px: &[u8], w: u32, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let k = ((y * w + x) * 4) as usize;
        (px[k], px[k + 1], px[k + 2], px[k + 3])
    }

    // The bookshop's wall parameters (world.ts interior() as staged
    // by shots.ts:227; declared on the wallpaper-walls/-back/-side
    // texture entries).
    const WALL: (u8, u8, u8, u8) = (0x5A, 0x46, 0x32, 255);
    const WALL_MOTIF: (u8, u8, u8, u8) = (0x6B, 0x56, 0x38, 255);
    // The ceiling variant: base #3A2E22, motif #413427 — the motif
    // is shade(ceiling, 1.25) precomputed by the source caller.
    const CEIL: (u8, u8, u8, u8) = (0x3A, 0x2E, 0x22, 255);
    const CEIL_MOTIF: (u8, u8, u8, u8) = (0x41, 0x34, 0x27, 255);

    #[test]
    fn size_and_fully_opaque() {
        let (px, w, h) = paint_wallpaper("#5A4632", "#6B5638");
        assert_eq!((w, h), (256, 256));
        assert_eq!(px.len(), 256 * 256 * 4);
        // Base fill is opaque and every later op is opaque or an
        // alpha blend over opaque, so no alpha but 255 survives.
        assert!(px.chunks_exact(4).all(|p| p[3] == 255));
    }

    #[test]
    fn deterministic() {
        let a = paint_wallpaper("#5A4632", "#6B5638");
        let b = paint_wallpaper("#5A4632", "#6B5638");
        assert_eq!(a, b);
    }

    #[test]
    fn bookshop_wall_colours() {
        let (px, w, _) = paint_wallpaper("#5A4632", "#6B5638");
        // Bare base, away from every fleur and dot.
        assert_eq!(at(&px, w, 10, 10), WALL);
        // Fleur petal interior (local (0.6, -26) of the (64,64)
        // fleur), both fleurs.
        assert_eq!(at(&px, w, 64, 40), WALL_MOTIF);
        assert_eq!(at(&px, w, 192, 167), WALL_MOTIF);
        // Left wing disc centre (local (-22, 4) at scale 0.9).
        assert_eq!(at(&px, w, 44, 67), WALL_MOTIF);
        // Stem interior (local (0.6, 22.8)).
        assert_eq!(at(&px, w, 64, 84), WALL_MOTIF);
        // Fleur centre: petal-covered, then dotted with its own
        // colour at alpha 0.5 — motif over motif is exact.
        assert_eq!(at(&px, w, 64, 64), WALL_MOTIF);
    }

    #[test]
    fn alpha_dot_over_base_composites() {
        let (px, w, _) = paint_wallpaper("#5A4632", "#6B5638");
        // The (192,64) dot lands on bare base: motif at alpha
        // 128/255 source-over the base, per the Painter blend.
        assert_eq!(at(&px, w, 192, 64), (99, 78, 53, 255));
        // The (64,192) grid point carries the same dot composite —
        // and nothing else: its fleur list entry is the fourth,
        // which the source's `i < 2` guard never draws.
        assert_eq!(at(&px, w, 64, 192), (99, 78, 53, 255));
        // Where that undrawn fleur's petal would sit (local
        // (0.6, -26) of (64,192)) the base is untouched.
        assert_eq!(at(&px, w, 64, 168), WALL);
    }

    #[test]
    fn ceiling_variant_and_linear_shade() {
        // The pinned constant, computed the source's way:
        // THREE.Color multiplyScalar in linear space (world.ts:35).
        // The sRGB byte multiply (scene::shade_rgb) would give
        // (0x49, 0x3A, 0x2B) here — that is film.rs's known S10
        // divergence, not the source value.
        assert_eq!(shade_hex("#3A2E22", 1.25), (0x41, 0x34, 0x27));
        let (px, w, _) = paint_wallpaper("#3A2E22", "#413427");
        assert_eq!(at(&px, w, 10, 10), CEIL);
        assert_eq!(at(&px, w, 64, 40), CEIL_MOTIF);
    }

    #[test]
    fn source_defaults() {
        // T.wallpaper's own defaults (world.ts:69).
        let (px, w, _) = paint_wallpaper("#9E2E3A", "#B8414C");
        assert_eq!(at(&px, w, 10, 10), (0x9E, 0x2E, 0x3A, 255));
        assert_eq!(at(&px, w, 64, 40), (0xB8, 0x41, 0x4C, 255));
    }
}
