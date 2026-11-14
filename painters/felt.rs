//! T.felt(col) — the bookshop's green baize (world.ts:93), a registry
//! painter in the scene.rs conventions: same `paint_*` signature,
//! same CPU `Painter`, same deterministic sin-hash `rnd`.
//!
//! Source recipe (world.ts:93, one line):
//!   canvasTex('felt' + col, 256, 256, (g, w, h) => {
//!     g.fillStyle = col; g.fillRect(0, 0, w, h);
//!     for (let i = 0; i < 3000; i++) {
//!       g.fillStyle = shade(col, 0.85 + rnd(i) * 0.3);
//!       g.fillRect(rnd(i * 3) * w, rnd(i * 7) * h, 2, 2);
//!     } })
//!
//! The one subtlety is `shade`: world.ts:35 defines it through
//! THREE.Color — NOT the canvas shadeOf (toon.ts:94) that
//! scene::shade_rgb mirrors. THREE.Color stores the hex converted
//! to the LINEAR working space, multiplyScalar(k) scales the linear
//! components, and getHexString() converts back to sRGB, quantising
//! with Math.round(clamp(255*c, 0, 255)) per channel. This module
//! reproduces that arithmetic exactly (three r170 transfer curves,
//! f64 throughout, JS argument evaluation order: k and the speckle
//! position are f64 until the Painter's f32 rasteriser, exactly as
//! film.rs's painters cast rnd results to f32 at the fill call).
//! scene::shade_lin does NOT equal it (it multiplies the sRGB bytes
//! first, then converts) — cf. the W14 ceiling-motif finding
//! (#413427 in three vs #493A2B through shade_lin).

use crate::scene::{hex_rgb, rnd, Painter};

/// three r170 SRGBToLinear: c < 0.04045 ? 0.0773993808*c
/// : pow(0.9478672986*c + 0.0521327014, 2.4). (Strict `<`, as in
/// three; scene::srgb_to_linear uses `<=` — equal only at the exact
/// boundary input, which no byte/255 value hits: 0.04045*255 =
/// 10.31475 is not an integer.)
fn srgb_to_linear_f64(c: f64) -> f64 {
    if c < 0.04045 {
        0.0773993808 * c
    } else {
        (0.9478672986 * c + 0.0521327014).powf(2.4)
    }
}

/// three r170 LinearToSRGB: c < 0.0031308 ? 12.92*c
/// : 1.055 * pow(c, 0.41666) - 0.055. Note the exponent literal
/// 0.41666 — not 1/2.4 (scene::linear_to_srgb uses 1.0/2.4 in f32;
/// the difference is below one byte step for the felt greens but
/// is reproduced verbatim here because the shade round-trip is the
/// whole point of this painter).
fn linear_to_srgb_f64(c: f64) -> f64 {
    if c < 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(0.41666) - 0.055
    }
}

/// world.ts:35 shade(hex, k) -> '#rrggbb', as bytes: hex -> linear
/// (THREE.Color setHex with colour management on), multiply by k
/// in linear, back to sRGB (getHex), quantise
/// Math.round(clamp(255*c, 0, 255)). JS Math.round on non-negative
/// values rounds half up; f64::round rounds half away from zero —
/// identical on the clamped [0, 255] domain.
fn shade_three(hex: &str, k: f64) -> (u8, u8, u8) {
    let (r, g, b) = hex_rgb(hex);
    let q = |v: u8| -> u8 {
        let lin = srgb_to_linear_f64(v as f64 / 255.0) * k;
        (255.0 * linear_to_srgb_f64(lin)).clamp(0.0, 255.0).round() as u8
    };
    (q(r), q(g), q(b))
}

/// T.felt(base): 256x256 — base fill + 3000 hash speckle rects of
/// 2x2 px, each shaded shade(base, 0.85 + rnd(i)*0.3). Used by the
/// bookshop with base '#1F5A45' (counter, the T.felt default) and
/// '#2E5A44' (desk).
pub(crate) fn paint_felt(base: &str) -> (Vec<u8>, u32, u32) {
    let (w, h) = (256usize, 256usize);
    let mut p = Painter::new(w, h);
    p.fill_rect(0.0, 0.0, w as f32, h as f32, crate::scene::rgba_of(base));
    for i in 0..3000 {
        let fi = i as f64;
        let sh = shade_three(base, 0.85 + rnd(fi) * 0.3);
        p.fill_rect(
            (rnd(fi * 3.0) * w as f64) as f32,
            (rnd(fi * 7.0) * h as f64) as f32,
            2.0,
            2.0,
            (sh.0, sh.1, sh.2, 255),
        );
    }
    (p.px, w as u32, h as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_colour(px: &[u8], rgb: (u8, u8, u8)) -> usize {
        px.chunks_exact(4)
            .filter(|p| p[0] == rgb.0 && p[1] == rgb.1 && p[2] == rgb.2)
            .count()
    }

    #[test]
    fn size_and_dims() {
        let (px, w, h) = paint_felt("#1F5A45");
        assert_eq!((w, h), (256, 256));
        assert_eq!(px.len(), 256 * 256 * 4);
    }

    #[test]
    fn deterministic() {
        assert_eq!(paint_felt("#1F5A45").0, paint_felt("#1F5A45").0);
        assert_eq!(paint_felt("#2E5A44").0, paint_felt("#2E5A44").0);
    }

    #[test]
    fn fully_opaque() {
        // Analytic: the base fill covers every pixel at coverage 1
        // and every speckle is opaque, so source-over never leaves
        // alpha < 255 anywhere.
        let (px, _, _) = paint_felt("#1F5A45");
        assert!(px.chunks_exact(4).all(|p| p[3] == 255));
    }

    #[test]
    fn shade_is_the_three_linear_multiply() {
        // k = 1 must round-trip a colour byte-exactly through the
        // linear excursion (conversion + reconversion + round).
        assert_eq!(shade_three("#1F5A45", 1.0), (0x1F, 0x5A, 0x45));
        // The linear multiply is NOT the sRGB byte multiply: for
        // k = 0.85 the canvas shadeOf gives (26, 77, 59) while the
        // three.js shade gives the brighter (28, 83, 64) — the
        // linear curve lifts mid-tones. Pinned from the r170
        // arithmetic above.
        assert_eq!(shade_three("#1F5A45", 0.85), (28, 83, 64));
        assert_ne!(
            shade_three("#1F5A45", 0.85),
            crate::scene::shade_rgb("#1F5A45", 0.85)
        );
    }

    /// Measured base-colour survivors for '#1F5A45' (see test).
    const PINNED_BASE_COUNT: i64 = 50047;

    #[test]
    fn pinned_base_pixel_count() {
        // Pinned statistic, computed from the algorithm: of the
        // 65536 pixels, this many keep the untouched base colour
        // (3000 speckles of 2x2 at hash positions, AA edges
        // included — any pixel a speckle touches at any coverage
        // leaves the base triple). Measured 50047 on the reference
        // build. The ±6 tolerance is also measured, not guessed:
        // re-running the whole painter with every sin() argument
        // nudged ±1 ulp (the V8-fdlibm vs Rust-libm gap) moves the
        // count by at most 4 and changes ~0.05% of bytes (speckle
        // edge blends only).
        let (px, _, _) = paint_felt("#1F5A45");
        let n = count_colour(&px, (0x1F, 0x5A, 0x45));
        assert!((n as i64 - PINNED_BASE_COUNT).abs() <= 6, "base count {n}");
    }
}
