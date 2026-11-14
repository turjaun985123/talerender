//! PR.parcel texture (props.ts:159) — the brown-paper parcel wrap,
//! a registry painter in the scene.rs conventions: same `paint_*`
//! signature shape, same CPU `Painter`, same deterministic
//! sin-hash `rnd`.
//!
//! Source recipe (props.ts:159, the texture half of
//! parcel(w, h, d, label = true) — the box dimensions never reach
//! the canvas; only `label` does):
//!   canvasTex('parcel' + label, 256, 256, (c) => {
//!     c.fillStyle = '#C8935A'; c.fillRect(0, 0, 256, 256);
//!     c.fillStyle = '#B07A48';
//!     for (let i = 0; i < 30; i++)
//!       c.fillRect(rnd(i) * 256, rnd(i * 3) * 256, 40, 2);
//!     c.fillStyle = '#E9D7A5';
//!     c.fillRect(120, 0, 16, 256); c.fillRect(0, 120, 256, 16);
//!     if (label) {
//!       c.fillStyle = '#FFFDF4'; c.fillRect(150, 150, 90, 70);
//!       c.fillStyle = '#D8343F'; c.fillRect(214, 156, 20, 24);
//!     } })
//!
//! No colour management is involved: every fill is a literal hex
//! (canvas bytes), so this painter is a direct transcription. The
//! bookshop stages both parcels with label = false.

use crate::scene::{hex_rgb, rnd, Painter};

/// PR.parcel texture: 256x256 — '#C8935A' paper, 30 fibre streaks
/// '#B07A48' (40x2, hash-placed), tape cross '#E9D7A5' (16 px bands
/// through (120..136) on both axes), and at label = true the
/// '#FFFDF4' label with its '#D8343F' stamp.
pub(crate) fn paint_parcel(label: bool) -> (Vec<u8>, u32, u32) {
    let (w, h) = (256usize, 256usize);
    let mut p = Painter::new(w, h);
    let c = hex_rgb("#C8935A");
    p.fill_rect(0.0, 0.0, w as f32, h as f32, (c.0, c.1, c.2, 255));
    let sp = hex_rgb("#B07A48");
    for i in 0..30 {
        let fi = i as f64;
        p.fill_rect(
            (rnd(fi) * 256.0) as f32,
            (rnd(fi * 3.0) * 256.0) as f32,
            40.0,
            2.0,
            (sp.0, sp.1, sp.2, 255),
        );
    }
    let tape = hex_rgb("#E9D7A5");
    let tape = (tape.0, tape.1, tape.2, 255);
    p.fill_rect(120.0, 0.0, 16.0, 256.0, tape);
    p.fill_rect(0.0, 120.0, 256.0, 16.0, tape);
    if label {
        let paper = hex_rgb("#FFFDF4");
        p.fill_rect(150.0, 150.0, 90.0, 70.0, (paper.0, paper.1, paper.2, 255));
        let stamp = hex_rgb("#D8343F");
        p.fill_rect(214.0, 156.0, 20.0, 24.0, (stamp.0, stamp.1, stamp.2, 255));
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
        let (px, w, h) = paint_parcel(false);
        assert_eq!((w, h), (256, 256));
        assert_eq!(px.len(), 256 * 256 * 4);
    }

    #[test]
    fn deterministic() {
        assert_eq!(paint_parcel(false).0, paint_parcel(false).0);
        assert_eq!(paint_parcel(true).0, paint_parcel(true).0);
        assert_ne!(paint_parcel(false).0, paint_parcel(true).0);
    }

    #[test]
    fn pinned_tape_pixel_count() {
        // Pinned statistic, computed from the algorithm: the tape
        // cross is two integer-aligned 16x256 bands, drawn last at
        // label = false, opaque — so every band pixel ends exactly
        // tape-coloured and no other pixel can (all remaining
        // colours and their blends stay below tape in at least one
        // channel... the streak colour and paper colour differ in
        // all three). 16*256 + 16*256 - 16*16 = 7936. Exact, not a
        // tolerance: integer edges give coverage exactly 0 or 1.
        let (px, _, _) = paint_parcel(false);
        assert_eq!(count_colour(&px, (0xE9, 0xD7, 0xA5)), 7936);
        // Column x = 128 lies inside the vertical band for all rows.
        for y in 0..256 {
            let k = (y * 256 + 128) * 4;
            assert_eq!(&px[k..k + 4], &[0xE9, 0xD7, 0xA5, 255]);
        }
    }

    #[test]
    fn label_false_draws_no_label_colours() {
        // Analytic: blends of base/streak/tape colours cannot
        // reach the label paper (255, 253, 244) — every source
        // colour has r <= 233 — nor the stamp (216, 52, 63): its
        // g = 52 is below every source colour's g (base 147,
        // streak 122, tape 215), and a blend's channel always lies
        // between its inputs' channels.
        let (px, _, _) = paint_parcel(false);
        assert_eq!(count_colour(&px, (0xFF, 0xFD, 0xF4)), 0);
        assert_eq!(count_colour(&px, (0xD8, 0x34, 0x3F)), 0);
    }

    #[test]
    fn label_true_counts() {
        // Integer-aligned opaque rects drawn after the tape: label
        // 90x70 minus the 20x24 stamp inside it, tape untouched
        // (the label starts at x = 150, past the band's 136 edge).
        let (px, _, _) = paint_parcel(true);
        assert_eq!(count_colour(&px, (0xE9, 0xD7, 0xA5)), 7936);
        assert_eq!(count_colour(&px, (0xFF, 0xFD, 0xF4)), 90 * 70 - 20 * 24);
        assert_eq!(count_colour(&px, (0xD8, 0x34, 0x3F)), 20 * 24);
    }
}
