//! paint_sign — registry texture generator "sign": the bookshop's
//! 'TANIGUCHI BOOKS' board (world.ts signBoard, core/world.ts:137-146).
//!
//! Source recipe:
//!   canvas = (round(w*200), round(h*200)) px — 200 px per unit;
//!     the bookshop board is w=2.2, h=0.4 -> 440x80.
//!   fill bg over the whole canvas;
//!   border: strokeStyle fg, lineWidth 6,
//!     strokeRect(10, 10, cw-20, ch-20) — the stroke is CENTRED on
//!     the rect path, so ink spans 7..13 px from each edge;
//!   text: font `700 {fs}px Caladea, Georgia, serif`,
//!     fs starts at ch*0.5 and shrinks by 2 while
//!     measureText(text) > cw*0.86;
//!     fillText(text, cw/2, ch/2 + 3), align centre, baseline middle.
//!   The board's box side faces take shade(bg, 0.7) — see
//!   sign_edge_rgba.
//!
//! TEXT STRATEGY (the engine's existing approach, stated plainly):
//! this engine has NO Rust text rasteriser — every text-bearing
//! texture enters as a PIL asset rendered from the project's own
//! fonts (film.rs module docs; captions, the S07 calendar plates,
//! and film/assets/sign.png for this very sign, all produced by
//! film/gen_assets.py). paint_sign therefore paints the board and
//! border natively and sign_with_text() composites a same-size
//! transparent glyph strip (PIL, 700 Caladea, laid out per
//! SIGN_FONT_* / sign_text_anchor) over it. Without a strip the
//! painter returns the board+border exactly as the canvas looks
//! before fillText.
//!
//! FONT SIZE AS DATA (gap B9): the shrink rule's outcome depends on
//! font metrics. Measured with PIL against the project's own
//! fonts/Caladea-Bold.ttf (2026-11-14): 'TANIGUCHI BOOKS' at the
//! 40 px start size is 335.36 px wide, under the 378.4 px limit —
//! NO shrink step fires; fs = 40 (see TANIGUCHI_BOOKS_PIL below).
//! That is a PIL measurement: browser (Skia) Caladea metrics are
//! unreproducible from source, so the browser's fs is not provable
//! from here — but the 43.04 px headroom makes a shrink in the
//! browser unlikely. gen_assets.py's own loop lands on 40 too.
//!
//! Integration (lead): move to src/painters/sign.rs and add the
//! scenefile.rs resolve_texture arm, e.g.
//!   "sign" => painters::sign::paint_sign(
//!       str_field(spec, "bg", ctx)?, str_field(spec, "fg", ctx)?,
//!       /* units from the mesh or canvas/200 */ ...),
//! optionally followed by sign_with_text() when the spec names a
//! glyph-strip file; or paste the body into src/scene.rs and drop
//! the `use` line.

use crate::scene::{hex_lin, linear_to_srgb, rgba_of, Painter, Rgba};

/// canvasTex scale in signBoard: canvas px per world unit.
pub(crate) const SIGN_PX_PER_UNIT: f32 = 200.0;
/// The shrink rule (world.ts:143-144): start at ch * 0.5, step -2
/// while the text is wider than cw * SIGN_FIT_FRACTION.
pub(crate) const SIGN_FONT_START_FRAC: f32 = 0.5;
pub(crate) const SIGN_FIT_FRACTION: f32 = 0.86;
pub(crate) const SIGN_FONT_STEP: f32 = 2.0;
/// fillText anchor offset: (cw/2, ch/2 + 3) (world.ts:145).
pub(crate) const SIGN_TEXT_DY: f32 = 3.0;

/// PIL-measured outcome of the shrink rule for the bookshop text:
/// (text, font px, measured width px at that size). PIL metrics,
/// the project's Caladea-Bold.ttf, 2026-11-14 — NOT browser
/// metrics (gap B9, see the module docs). No shrink fires:
/// 335.36 <= 440 * 0.86 = 378.4.
pub(crate) const TANIGUCHI_BOOKS_PIL: (&str, f32, f32) =
    ("TANIGUCHI BOOKS", 40.0, 335.36);

/// signBoard's canvas size rule: Math.round(w*200), Math.round(h*200).
pub(crate) fn sign_canvas_size(w_units: f32, h_units: f32) -> (u32, u32) {
    (
        (w_units * SIGN_PX_PER_UNIT).round() as u32,
        (h_units * SIGN_PX_PER_UNIT).round() as u32,
    )
}

/// The fillText anchor (centre / middle baseline): (cw/2, ch/2 + 3).
/// The PIL glyph strip must place the text here.
pub(crate) fn sign_text_anchor(cw: u32, ch: u32) -> (f32, f32) {
    (cw as f32 / 2.0, ch as f32 / 2.0 + SIGN_TEXT_DY)
}

/// shade(bg, 0.7) — the box side-face colour (world.ts:143: five
/// faces at shade(bg, 0.7)). Computed the SOURCE's way: world.ts:35
/// is THREE.Color — hex parsed to LINEAR, multiplied in linear,
/// converted back to sRGB. For the bookshop bg '#3A2A2A' this is
/// (48, 34, 34) = #302222. (scene.rs's shade_rgb multiplies in
/// sRGB space and would give (41, 29, 29) — the divergence W14
/// flagged; this helper follows the source.)
pub(crate) fn sign_edge_rgba(bg: &str) -> Rgba {
    let lin = hex_lin(bg);
    (
        (linear_to_srgb(lin[0] * 0.7) * 255.0).round() as u8,
        (linear_to_srgb(lin[1] * 0.7) * 255.0).round() as u8,
        (linear_to_srgb(lin[2] * 0.7) * 255.0).round() as u8,
        255,
    )
}

/// Board + border, canvas-pixel dimensions (the form the scene
/// spec's "canvas": [440, 80] entry speaks in).
pub(crate) fn paint_sign_px(bg: &str, fg: &str, cw: u32, ch: u32) -> (Vec<u8>, u32, u32) {
    let mut p = Painter::new(cw as usize, ch as usize);
    p.fill_rect(0.0, 0.0, cw as f32, ch as f32, rgba_of(bg));
    // strokeRect(10, 10, cw-20, ch-20) at lineWidth 6: one closed
    // polyline through the rect corners, stroked as one shape.
    let (x1, y1) = (cw as f32 - 10.0, ch as f32 - 10.0);
    let rect = [
        (10.0, 10.0),
        (x1, 10.0),
        (x1, y1),
        (10.0, y1),
        (10.0, 10.0),
    ];
    p.stroke_polyline(&rect, 6.0, rgba_of(fg));
    (p.px, cw, ch)
}

/// Board + border, world-unit dimensions (signBoard's own shape:
/// canvas derived by the 200 px/unit rule).
pub(crate) fn paint_sign(bg: &str, fg: &str, w_units: f32, h_units: f32) -> (Vec<u8>, u32, u32) {
    let (cw, ch) = sign_canvas_size(w_units, h_units);
    paint_sign_px(bg, fg, cw, ch)
}

/// Composite a glyph strip over a painted board. The strip is a
/// same-size RGBA image, transparent except for the glyphs (the
/// PIL step renders ONLY the text, at sign_text_anchor, in fg).
/// Blending reuses Painter::blend (source-over, straight alpha) —
/// the same semantics the single-image PIL asset gets for free.
pub(crate) fn sign_with_text(board: (Vec<u8>, u32, u32), strip: &[u8]) -> (Vec<u8>, u32, u32) {
    let (px, w, h) = board;
    assert_eq!(
        strip.len(),
        px.len(),
        "sign glyph strip must be the same size as the board"
    );
    let mut p = Painter { w: w as usize, h: h as usize, px };
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let k = ((y as usize * w as usize + x as usize) * 4) as usize;
            if strip[k + 3] > 0 {
                p.blend(x, y, (strip[k], strip[k + 1], strip[k + 2], strip[k + 3]));
            }
        }
    }
    (p.px, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BG: &str = "#3A2A2A";
    const FG: &str = "#F2E6CF";

    fn at(px: &[u8], w: u32, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let k = ((y * w + x) * 4) as usize;
        (px[k], px[k + 1], px[k + 2], px[k + 3])
    }

    #[test]
    fn canvas_size_rule() {
        // The bookshop board: 2.2 x 0.4 units -> 440 x 80 px.
        assert_eq!(sign_canvas_size(2.2, 0.4), (440, 80));
        // signBoard defaults: 2 x 0.5 -> 400 x 100.
        assert_eq!(sign_canvas_size(2.0, 0.5), (400, 100));
    }

    #[test]
    fn edge_shade_is_the_linear_source_value() {
        // shade('#3A2A2A', 0.7) in THREE.Color (linear) = #302222.
        assert_eq!(sign_edge_rgba(BG), (48, 34, 34, 255));
    }

    #[test]
    fn text_anchor_matches_fill_text() {
        assert_eq!(sign_text_anchor(440, 80), (220.0, 43.0));
    }

    #[test]
    fn board_pixels() {
        let (px, w, h) = paint_sign(BG, FG, 2.2, 0.4);
        assert_eq!((w, h), (440, 80));
        // Board centre: plain bg (58, 42, 42) — no text without a strip.
        assert_eq!(at(&px, w, 220, 40), (58, 42, 42, 255));
        // Left border at mid-height: pixel centre (10.5, 40.5) is
        // 0.5 px from the stroke path -> full fg (242, 230, 207).
        assert_eq!(at(&px, w, 10, 40), (242, 230, 207, 255));
        // Just outside the border's ink (spans 7..13): still bg.
        assert_eq!(at(&px, w, 20, 40), (58, 42, 42, 255));
    }

    #[test]
    fn deterministic() {
        let (a, _, _) = paint_sign(BG, FG, 2.2, 0.4);
        let (b, _, _) = paint_sign(BG, FG, 2.2, 0.4);
        assert_eq!(a, b);
    }

    #[test]
    fn pil_measurement_means_no_shrink() {
        // The recorded PIL measurement: at the 40 px start size the
        // text already fits the 86% limit, so the rule keeps 40.
        let (text, fs, width) = TANIGUCHI_BOOKS_PIL;
        assert_eq!(text, "TANIGUCHI BOOKS");
        assert_eq!(fs, 80.0 * SIGN_FONT_START_FRAC);
        assert!(width <= 440.0 * SIGN_FIT_FRACTION);
    }

    #[test]
    fn strip_composites_over_board() {
        let board = paint_sign(BG, FG, 2.2, 0.4);
        let mut strip = vec![0u8; board.0.len()];
        // One opaque fg "glyph" block at the text anchor.
        for y in 38..48usize {
            for x in 215..225usize {
                let k = (y * 440 + x) * 4;
                strip[k..k + 4].copy_from_slice(&[242, 230, 207, 255]);
            }
        }
        let (px, w, _) = sign_with_text(board, &strip);
        assert_eq!(at(&px, w, 220, 43), (242, 230, 207, 255));
        // Untouched corner stays bg.
        assert_eq!(at(&px, w, 20, 40), (58, 42, 42, 255));
    }
}
