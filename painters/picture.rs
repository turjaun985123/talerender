//! paint_picture — registry texture generator "picture": the framed
//! line-drawing that hangs in the bookshop interior.
//!
//! Source recipe, transcribed 1:1 from the Onoda project's
//! core/world.ts:319 (`interior()`'s `pic` canvasTex, 256x200):
//!   paper fill '#E9DCC0';
//!   ink '#3A2A22', lineWidth 6, ONE path:
//!     moveTo(40,140)
//!     quadraticCurveTo(90,60, 140,120)
//!     quadraticCurveTo(170,80, 220,140)
//!   stroke();
//!   sun: fill '#C9603F', arc(190,60,22, 0, 7), fill().
//! (arc end angle 7 rad > TAU, so the sun is a full disc.)
//!
//! The gold FRAME is not part of this texture: in the source it is
//! the box mesh's face materials (world.ts:320, '#C9A13B' on five
//! faces, this texture on the front) — see PICTURE_FRAME below.
//!
//! Conventions mirrored from the existing registry painters in
//! src/scene.rs (paint_grass / paint_cloud / paint_blob): signature
//! returns (RGBA bytes, w, h); canvas coordinates, row 0 = top;
//! pure function of its inputs (no state, no RNG — this recipe has
//! no hash strokes at all); curves flattened with the house
//! flatten_quad (the toonfull.rs precedent uses n = 20 per quad).
//!
//! Integration (lead): move to src/painters/picture.rs and add the
//! scenefile.rs resolve_texture arm
//!   "picture" => painters::picture::paint_picture(),
//! or paste the body into src/scene.rs and drop the `use` line.
//! The spec entry (scenes/sets/bookshop.scene.json, texture id
//! "picture") carries no parameters — canvas is fixed 256x200.

use crate::scene::{flatten_quad, rgba_of, Painter};

/// Canvas size fixed by the source canvasTex('pic', 256, 200, ...).
pub(crate) const PICTURE_W: u32 = 256;
pub(crate) const PICTURE_H: u32 = 200;

/// The recipe's three colours (world.ts:319).
pub(crate) const PICTURE_PAPER: &str = "#E9DCC0";
pub(crate) const PICTURE_INK: &str = "#3A2A22";
pub(crate) const PICTURE_SUN: &str = "#C9603F";

/// Frame colour of the MESH (world.ts:320) — geometry material,
/// not texture; recorded here so the data has one home.
pub(crate) const PICTURE_FRAME: &str = "#C9A13B";

pub(crate) fn paint_picture() -> (Vec<u8>, u32, u32) {
    let (w, h) = (PICTURE_W as usize, PICTURE_H as usize);
    let mut p = Painter::new(w, h);
    p.fill_rect(0.0, 0.0, w as f32, h as f32, rgba_of(PICTURE_PAPER));
    // The two hill strokes are a single canvas path (the second
    // quadratic starts where the first ends, at (140,120)), so they
    // are stroked as ONE polyline — one canvas stroke is one shape,
    // and the join is the union, never a double blend.
    let mut pts = flatten_quad((40.0, 140.0), (90.0, 60.0), (140.0, 120.0), 20);
    let tail = flatten_quad((140.0, 120.0), (170.0, 80.0), (220.0, 140.0), 20);
    pts.extend_from_slice(&tail[1..]);
    p.stroke_polyline(&pts, 6.0, rgba_of(PICTURE_INK));
    // Sun disc (drawn after the stroke, as in the source).
    p.fill_ellipse(190.0, 60.0, 22.0, 22.0, 0.0, rgba_of(PICTURE_SUN));
    (p.px, PICTURE_W, PICTURE_H)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(px: &[u8], x: u32, y: u32) -> (u8, u8, u8, u8) {
        let k = ((y * PICTURE_W + x) * 4) as usize;
        (px[k], px[k + 1], px[k + 2], px[k + 3])
    }

    #[test]
    fn size_and_opacity() {
        let (px, w, h) = paint_picture();
        assert_eq!((w, h), (256, 200));
        assert_eq!(px.len(), 256 * 200 * 4);
        // Interior sample points are fully opaque (the paper fill
        // covers the whole canvas; AA lives only at shape edges).
        for (x, y) in [(4, 4), (128, 20), (30, 180), (250, 100)] {
            assert_eq!(at(&px, x, y).3, 255, "alpha at ({x},{y})");
        }
    }

    #[test]
    fn deterministic() {
        let (a, _, _) = paint_picture();
        let (b, _, _) = paint_picture();
        assert_eq!(a, b);
    }

    #[test]
    fn pinned_pixels() {
        let (px, _, _) = paint_picture();
        // Paper, far from ink and sun.
        assert_eq!(at(&px, 4, 4), (233, 220, 192, 255));
        // Sun centre: (201, 96, 63) = #C9603F.
        assert_eq!(at(&px, 190, 60), (201, 96, 63, 255));
        // The first quadratic passes exactly through (90, 95) at
        // t = 0.5 (0.25*(40,140) + 0.5*(90,60) + 0.25*(140,120));
        // the pixel centre is < 1 px from the curve, deep inside
        // the 6 px stroke, so it carries the full ink colour
        // (58, 42, 34) = #3A2A22.
        assert_eq!(at(&px, 90, 95), (58, 42, 34, 255));
    }

    #[test]
    fn sun_disc_area_band() {
        // Pinned statistic (band, not exact count: the disc's AA
        // rim is coverage-blended, so only the fully-covered
        // interior carries the exact sun colour — expected near
        // pi * 21.5^2 ~ 1450 px; tighten after the first CI run).
        let (px, _, _) = paint_picture();
        let n = (0..256 * 200)
            .filter(|&i| {
                let k = i * 4;
                (px[k], px[k + 1], px[k + 2]) == (201, 96, 63)
            })
            .count();
        assert!((1250..1600).contains(&n), "sun interior px = {n}");
    }
}
