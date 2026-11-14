//! S07 calendar-card overlay painter — a self-contained CPU
//! rasteriser for the "years flip 1945 → 1955 → 1965 → 1974" card
//! that `drawCalendar` draws in the Onoda project's shots.ts:295
//! (2D post-blit overlay, drawn inside the S07 shot draw, before
//! the film fade and the caption — main.ts's order, the same
//! overlay point film.rs uses).
//!
//! Data spec: scenes/film/s07-calendar.json (seat W09). film.rs
//! already composites this card from PIL plates
//! (film/gen_assets.py → film/assets/calendar-*.png, sampled by
//! overlay_calendar). This module is the vector path to the same
//! card: every shape is rasterised directly, with the house CPU
//! painter's conventions (src/scene.rs `Painter`, M4):
//!   * straight-alpha RGBA, src-over blending quantised to u8 at
//!     every blend (`blend_px` / `blend_cov_px` below copy
//!     Painter::blend / blend_cov's arithmetic exactly);
//!   * coverage from signed distance, cov = clamp(0.5 − sd),
//!     with the footprint integral estimated by a 4×4 binary
//!     supersample of the inside test inside the band |sd| < 1
//!     (cov_area, mirroring scene.rs cov_area0);
//!   * samples at pixel CENTRES (x + 0.5, y + 0.5).
//! The whole card (text included) lives in card-local space and
//! is carried onto the frame by the one canvas transform —
//! translate(352, 648) · rotate(−0.03) · scale(0.92, 0.92·sy) —
//! evaluated per pixel by inverse mapping, so the flip squash
//! applies to the text exactly as it does on canvas.
//!
//! TEXT: the codebase has NO Inter glyph rasteriser — text
//! enters the native film as data (PIL plates; telemetry.rs
//! notes an Inter measurer is future work). The three strings
//! here are fixed ('C A L E N D A R', the four year numerals,
//! 'LUBANG ISLAND'), so the glyphs are embedded below as
//! hand-authored vector shapes over a small fixed set
//! (A B C D E G I L N R S U, digits 1 4 5 6 7 9, space),
//! rasterised by the same 4×4 inside-test supersample. They are
//! geometric approximations of Inter ExtraBold/Black — NOT Inter
//! outlines; advances are hand-set (digits tabular at 0.60 em).
//! See painters/calendar.md for the fidelity limits. The PIL
//! plates remain the reference rendering.
//!
//! Wiring (integrator): drop this file into the crate (e.g.
//! src/calendar.rs + `mod calendar;`) and call `composite_over`
//! where film.rs currently calls overlay_calendar, or use
//! `render_overlay` for a standalone 1080×1920 RGBA buffer.
//! This file has no crate dependencies (std only) so it can be
//! type-checked and unit-tested standalone:
//!   rustc --edition 2021 --test painters/calendar.rs

#![allow(dead_code)]

use std::f64::consts::PI as PI64;

// ----------------------------------------------------------------
// Card data (scenes/film/s07-calendar.json; shots.ts drawCalendar)
// ----------------------------------------------------------------

pub const FRAME_W: u32 = 1080;
pub const FRAME_H: u32 = 1920;

/// calT0 on the film clock: S07 starts at 30.39 and drawCalendar
/// is called with t0 = shot start + 0.16 (film.rs calls
/// calendar_state(lt − 0.16)).
pub const T0_FILM: f64 = 30.55;
/// Card life in local seconds; visible while 0 <= lt <= LIFE.
pub const LIFE: f64 = 4.4;

pub const YEARS: [u32; 4] = [1945, 1955, 1965, 1974];
const HOLD: f64 = 0.62;
const FLIP: f64 = 0.42;
const CYC: f64 = HOLD + FLIP; // 1.04

// Card-local geometry (units; card centre = local origin).
const CARD_HX: f32 = 160.0; // card rect (-160,-200, 320,400)
const CARD_HY: f32 = 200.0;
const CARD_R: f32 = 14.0;
const SHADOW_CX: f32 = 8.0; // shadow rect offset by (+8,+10)
const SHADOW_CY: f32 = 10.0;
const HEADER_CY: f32 = -167.0; // header rect (-160,-200, 320,66)
const HEADER_HY: f32 = 33.0;
const RULE_CY: f32 = 160.5; // rule rect (-110,158, 220,5)
const RULE_HX: f32 = 110.0;
const RULE_HY: f32 = 2.5;

// Colours (sRGB bytes, as drawn on canvas).
const COL_SHADOW: [u8; 4] = [20, 16, 10, 89]; // rgba(20,16,10,0.35)
const COL_CARD: [u8; 4] = [244, 235, 214, 255]; // #F4EBD6
const COL_INK: [u8; 4] = [31, 28, 51, 255]; // #1F1C33
const COL_HEADER: [u8; 4] = [138, 48, 56, 255]; // #8A3038
const COL_SUB: [u8; 4] = [139, 131, 120, 255]; // #8B8378
const COL_DARK: [u8; 3] = [30, 25, 18]; // flip shade rgb(30,25,18)

// Canvas transform.
const ORIGIN: (f32, f32) = (352.0, 648.0);
const ROT: f32 = -0.03;
const SCALE: f32 = 0.92;
// Plate extent (the PIL plates' local frame): nothing is drawn
// outside x [-180,180], y [-220,220]; used as the cull bound.
const PLATE_HX: f32 = 180.0;
const PLATE_HY: f32 = 220.0;

// ----------------------------------------------------------------
// Flip-cycle state — the data spec's cycle law (== shots.ts).
// ----------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalState {
    /// Index into YEARS, 0..=3.
    pub year_idx: usize,
    /// The year numeral on the card at this instant.
    pub year: u32,
    /// Vertical flip scale, 0.1..=1 (max(0.1, |cos(u·π)|)).
    pub sy: f32,
    /// Flip shading amount 0..=1 (1 − sy while flipping).
    pub dark: f32,
    /// Global alpha: clamp(lt/0.18) · clamp((4.4 − lt)/0.35).
    pub alpha: f32,
}

fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

/// The card's state at film time `t` (seconds), or None while
/// the card is not on screen (lt outside [0, 4.4]).
pub fn state_at(t: f64) -> Option<CalState> {
    let lt = t - T0_FILM;
    if !(0.0..=LIFE).contains(&lt) {
        return None;
    }
    let idx = (lt / CYC).floor() as usize;
    let (mut year_idx, mut sy, mut dark) = (idx.min(3), 1.0f64, 0.0f64);
    if idx < 3 {
        let ph = lt - idx as f64 * CYC;
        if ph >= HOLD {
            let u = (ph - HOLD) / FLIP;
            sy = (u * PI64).cos().abs().max(0.1);
            dark = 1.0 - sy;
            if u >= 0.5 {
                year_idx = idx + 1;
            }
        }
    }
    let alpha = clamp01(lt / 0.18) * clamp01((LIFE - lt) / 0.35);
    Some(CalState {
        year_idx,
        year: YEARS[year_idx],
        sy: sy as f32,
        dark: dark as f32,
        alpha: alpha as f32,
    })
}

// ----------------------------------------------------------------
// Pixel machinery — mirrors src/scene.rs's Painter exactly:
// straight-alpha src-over, u8-quantised per blend; coverage from
// signed distance with a 4×4 supersampled inside test in the
// band |sd| < 1 (scene.rs cov_area0's estimator).
// ----------------------------------------------------------------

fn blend_px(dst: &mut [u8; 4], src: [u8; 4]) {
    let sa = src[3] as f32 / 255.0;
    if sa <= 0.0 {
        return;
    }
    if sa >= 1.0 {
        *dst = src;
        return;
    }
    let da = dst[3] as f32 / 255.0;
    let oa = sa + da * (1.0 - sa);
    if oa <= 0.0 {
        return;
    }
    for c in 0..3 {
        let s = src[c] as f32 / 255.0;
        let d = dst[c] as f32 / 255.0;
        dst[c] = (((s * sa + d * da * (1.0 - sa)) / oa) * 255.0).round() as u8;
    }
    dst[3] = (oa * 255.0).round() as u8;
}

fn blend_cov_px(dst: &mut [u8; 4], src: [u8; 4], cov: f32) {
    if cov <= 0.0 {
        return;
    }
    if cov >= 1.0 {
        blend_px(dst, src);
        return;
    }
    let a = (src[3] as f32 * cov).round().clamp(0.0, 255.0) as u8;
    if a > 0 {
        blend_px(dst, [src[0], src[1], src[2], a]);
    }
}

fn cov_of(sd: f32) -> f32 {
    (0.5 - sd).clamp(0.0, 1.0)
}

/// Coverage of one frame pixel for the shape whose signed
/// distance (card-local units, negative inside) is `dist`.
/// `subs` are the pixel's 16 sub-sample points in card-local
/// space (the 4×4 grid at ±0.125/±0.375 frame px, inverse-mapped
/// — the same offsets scene.rs cov_area uses).
fn cov_area<F: Fn(f32, f32) -> f32>(dist: &F, c: (f32, f32), subs: &[(f32, f32); 16]) -> f32 {
    let d0 = dist(c.0, c.1);
    if d0.abs() < 1.0 {
        let mut inside = 0u32;
        for &(sx, sy) in subs {
            if dist(sx, sy) < 0.0 {
                inside += 1;
            }
        }
        inside as f32 / 16.0
    } else {
        cov_of(d0)
    }
}

fn fill_sd<F: Fn(f32, f32) -> f32>(
    px: &mut [u8; 4],
    col: [u8; 4],
    c: (f32, f32),
    subs: &[(f32, f32); 16],
    dist: &F,
) {
    let cov = cov_area(dist, c, subs);
    blend_cov_px(px, col, cov);
}

// ----------------------------------------------------------------
// Signed distances. sd_round_box takes per-corner radii in
// canvas order [top-left, top-right, bottom-right, bottom-left].
// ----------------------------------------------------------------

fn sd_round_box(
    px: f32,
    py: f32,
    cx: f32,
    cy: f32,
    hx: f32,
    hy: f32,
    r: [f32; 4],
) -> f32 {
    let (dx, dy) = (px - cx, py - cy);
    let rad = if dx < 0.0 {
        if dy < 0.0 { r[0] } else { r[3] }
    } else if dy < 0.0 {
        r[1]
    } else {
        r[2]
    };
    let (qx, qy) = (dx.abs() - (hx - rad), dy.abs() - (hy - rad));
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - rad
}

fn sd_card(px: f32, py: f32) -> f32 {
    sd_round_box(px, py, 0.0, 0.0, CARD_HX, CARD_HY, [CARD_R; 4])
}

// ----------------------------------------------------------------
// Embedded glyphs. Authored on a 1000-unit em: x 0..advance,
// y −360 (cap top) .. +360 (baseline), y DOWN (card space is
// y-down; the text anchor is the em centre, matching canvas
// textBaseline 'middle' for these cap/digit-only strings).
// A glyph is a union of positive shapes minus negative shapes,
// tested per sub-sample point. Stroke weight ≈ 180 units
// (0.18 em — Inter Black stems are ≈ 0.19 em).
// ----------------------------------------------------------------

#[derive(Clone, Copy)]
enum Shape {
    Rect(f32, f32, f32, f32), // x, y, w, h (y down)
    Disc(f32, f32, f32),      // cx, cy, r
    Poly(&'static [(f32, f32)]),
}

impl Shape {
    fn contains(&self, x: f32, y: f32) -> bool {
        match *self {
            Shape::Rect(rx, ry, w, h) => x >= rx && x < rx + w && y >= ry && y < ry + h,
            Shape::Disc(cx, cy, r) => {
                let (dx, dy) = (x - cx, y - cy);
                dx * dx + dy * dy <= r * r
            }
            Shape::Poly(pts) => point_in_polygon(x, y, pts),
        }
    }
}

/// Even-odd point-in-polygon (copied from src/scene.rs).
fn point_in_polygon(x: f32, y: f32, pts: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let n = pts.len();
    for i in 0..n {
        let (xi, yi) = pts[i];
        let (xj, yj) = pts[(i + 1) % n];
        if (yi > y) != (yj > y) {
            let xint = xi + (y - yi) / (yj - yi) * (xj - xi);
            if x < xint {
                inside = !inside;
            }
        }
    }
    inside
}

#[derive(Clone, Copy)]
struct Glyph {
    adv: f32,
    pos: &'static [Shape],
    neg: &'static [Shape],
}

impl Glyph {
    fn contains(&self, x: f32, y: f32) -> bool {
        self.pos.iter().any(|s| s.contains(x, y)) && !self.neg.iter().any(|s| s.contains(x, y))
    }
}

static A_OUT: &[(f32, f32)] = &[
    (60.0, 360.0), (295.0, -360.0), (385.0, -360.0), (620.0, 360.0),
    (455.0, 360.0), (398.0, 80.0), (282.0, 80.0), (225.0, 360.0),
];
static A_HOLE: &[(f32, f32)] = &[(340.0, -175.0), (375.0, -55.0), (305.0, -55.0)];
static B_TOP: &[(f32, f32)] = &[
    (60.0, -360.0), (430.0, -360.0), (565.0, -255.0), (565.0, -110.0), (455.0, -15.0), (60.0, -15.0),
];
static B_BOT: &[(f32, f32)] = &[
    (60.0, -15.0), (470.0, -15.0), (590.0, 95.0), (590.0, 250.0), (445.0, 360.0), (60.0, 360.0),
];
static B_CT: &[(f32, f32)] = &[(240.0, -200.0), (390.0, -200.0), (395.0, -95.0), (240.0, -95.0)];
static B_CB: &[(f32, f32)] = &[(240.0, 60.0), (405.0, 60.0), (405.0, 200.0), (240.0, 200.0)];
static D_BOWL: &[(f32, f32)] = &[
    (60.0, -360.0), (400.0, -360.0), (605.0, -215.0), (605.0, 215.0), (400.0, 360.0), (60.0, 360.0),
];
static D_CNT: &[(f32, f32)] = &[
    (240.0, -195.0), (385.0, -195.0), (430.0, -105.0), (430.0, 105.0), (385.0, 195.0), (240.0, 195.0),
];
static N_DIAG: &[(f32, f32)] = &[
    (240.0, -360.0), (420.0, -360.0), (500.0, 180.0), (500.0, 360.0), (320.0, 360.0), (240.0, -180.0),
];
static R_BOWL: &[(f32, f32)] = &[
    (60.0, -360.0), (430.0, -360.0), (570.0, -245.0), (570.0, -100.0), (455.0, 0.0), (60.0, 0.0),
];
static R_LEG: &[(f32, f32)] = &[(330.0, 0.0), (470.0, 0.0), (610.0, 360.0), (420.0, 360.0)];
static R_CNT: &[(f32, f32)] = &[(240.0, -195.0), (385.0, -195.0), (390.0, -100.0), (240.0, -100.0)];
static U_OUT: &[(f32, f32)] = &[
    (60.0, -360.0), (660.0, -360.0), (660.0, 180.0), (590.0, 300.0),
    (480.0, 360.0), (240.0, 360.0), (130.0, 300.0), (60.0, 180.0),
];
static U_CNT: &[(f32, f32)] = &[
    (240.0, -360.0), (480.0, -360.0), (480.0, 150.0), (420.0, 215.0), (300.0, 215.0), (240.0, 150.0),
];
static ONE: &[(f32, f32)] = &[
    (170.0, 360.0), (170.0, -140.0), (85.0, -205.0), (85.0, -285.0),
    (335.0, -360.0), (415.0, -360.0), (415.0, 360.0),
];
static FOUR_DIAG: &[(f32, f32)] = &[
    (245.0, -360.0), (415.0, -360.0), (160.0, 36.0), (55.0, 36.0),
];
static SIX_TAIL: &[(f32, f32)] = &[
    (115.0, -90.0), (330.0, -360.0), (480.0, -360.0), (255.0, -75.0), (190.0, 10.0),
];
static SEVEN: &[(f32, f32)] = &[(390.0, -195.0), (540.0, -195.0), (300.0, 360.0), (155.0, 360.0)];
static NINE_TAIL: &[(f32, f32)] = &[
    (510.0, 40.0), (350.0, 360.0), (200.0, 360.0), (425.0, 20.0), (460.0, -60.0),
];

const NO_SHAPES: &[Shape] = &[];

static A_POS: &[Shape] = &[Shape::Poly(A_OUT)];
static A_NEG: &[Shape] = &[Shape::Poly(A_HOLE)];
static B_POS: &[Shape] = &[
    Shape::Rect(60.0, -360.0, 180.0, 720.0),
    Shape::Poly(B_TOP),
    Shape::Poly(B_BOT),
];
static B_NEG: &[Shape] = &[Shape::Poly(B_CT), Shape::Poly(B_CB)];
static C_POS: &[Shape] = &[Shape::Disc(350.0, 0.0, 360.0)];
static C_NEG: &[Shape] = &[Shape::Disc(350.0, 0.0, 180.0), Shape::Rect(480.0, -105.0, 240.0, 210.0)];
static D_POS: &[Shape] = &[Shape::Rect(60.0, -360.0, 180.0, 720.0), Shape::Poly(D_BOWL)];
static D_NEG: &[Shape] = &[Shape::Poly(D_CNT)];
static E_POS: &[Shape] = &[
    Shape::Rect(60.0, -360.0, 180.0, 720.0),
    Shape::Rect(60.0, -360.0, 520.0, 165.0),
    Shape::Rect(60.0, -82.0, 460.0, 165.0),
    Shape::Rect(60.0, 195.0, 520.0, 165.0),
];
static G_POS: &[Shape] = &[
    Shape::Disc(370.0, 0.0, 370.0),
    Shape::Rect(400.0, 30.0, 340.0, 150.0),
    Shape::Rect(555.0, 30.0, 185.0, 330.0),
];
static G_NEG: &[Shape] = &[Shape::Disc(370.0, 0.0, 190.0), Shape::Rect(540.0, -75.0, 200.0, 150.0)];
static I_POS: &[Shape] = &[Shape::Rect(50.0, -360.0, 180.0, 720.0)];
static L_POS: &[Shape] = &[
    Shape::Rect(60.0, -360.0, 180.0, 720.0),
    Shape::Rect(60.0, 195.0, 500.0, 165.0),
];
static N_POS: &[Shape] = &[
    Shape::Rect(60.0, -360.0, 180.0, 720.0),
    Shape::Rect(500.0, -360.0, 180.0, 720.0),
    Shape::Poly(N_DIAG),
];
static R_POS: &[Shape] = &[
    Shape::Rect(60.0, -360.0, 180.0, 720.0),
    Shape::Poly(R_BOWL),
    Shape::Poly(R_LEG),
];
static R_NEG: &[Shape] = &[Shape::Poly(R_CNT)];
static S_POS: &[Shape] = &[Shape::Disc(320.0, -165.0, 195.0), Shape::Disc(320.0, 165.0, 195.0)];
static S_NEG: &[Shape] = &[
    Shape::Disc(320.0, -165.0, 70.0),
    Shape::Disc(320.0, 165.0, 70.0),
    Shape::Rect(320.0, -60.0, 320.0, 120.0),
    Shape::Rect(0.0, -60.0, 320.0, 120.0),
];
static U_POS: &[Shape] = &[Shape::Poly(U_OUT)];
static U_NEG: &[Shape] = &[Shape::Poly(U_CNT)];
static ONE_POS: &[Shape] = &[Shape::Poly(ONE), Shape::Rect(105.0, 285.0, 375.0, 75.0)];
static FOUR_POS: &[Shape] = &[
    Shape::Rect(280.0, -360.0, 170.0, 720.0),
    Shape::Poly(FOUR_DIAG),
    Shape::Rect(55.0, 36.0, 395.0, 170.0),
];
static FIVE_POS: &[Shape] = &[
    Shape::Rect(70.0, -360.0, 475.0, 165.0),
    Shape::Rect(70.0, -360.0, 175.0, 320.0),
    Shape::Disc(325.0, 140.0, 220.0),
];
static FIVE_NEG: &[Shape] = &[Shape::Disc(325.0, 140.0, 90.0)];
static SIX_POS: &[Shape] = &[Shape::Disc(300.0, 115.0, 245.0), Shape::Poly(SIX_TAIL)];
static SIX_NEG: &[Shape] = &[Shape::Disc(300.0, 115.0, 105.0)];
static SEVEN_POS: &[Shape] = &[Shape::Rect(60.0, -360.0, 480.0, 165.0), Shape::Poly(SEVEN)];
static NINE_POS: &[Shape] = &[Shape::Disc(300.0, -100.0, 260.0), Shape::Poly(NINE_TAIL)];
static NINE_NEG: &[Shape] = &[Shape::Disc(300.0, -100.0, 105.0)];

fn glyph(ch: char) -> Glyph {
    match ch {
        'A' => Glyph { adv: 680.0, pos: A_POS, neg: A_NEG },
        'B' => Glyph { adv: 660.0, pos: B_POS, neg: B_NEG },
        'C' => Glyph { adv: 720.0, pos: C_POS, neg: C_NEG },
        'D' => Glyph { adv: 720.0, pos: D_POS, neg: D_NEG },
        'E' => Glyph { adv: 600.0, pos: E_POS, neg: NO_SHAPES },
        'G' => Glyph { adv: 740.0, pos: G_POS, neg: G_NEG },
        'I' => Glyph { adv: 280.0, pos: I_POS, neg: NO_SHAPES },
        'L' => Glyph { adv: 560.0, pos: L_POS, neg: NO_SHAPES },
        'N' => Glyph { adv: 740.0, pos: N_POS, neg: NO_SHAPES },
        'R' => Glyph { adv: 660.0, pos: R_POS, neg: R_NEG },
        'S' => Glyph { adv: 640.0, pos: S_POS, neg: S_NEG },
        'U' => Glyph { adv: 720.0, pos: U_POS, neg: U_NEG },
        '1' => Glyph { adv: 600.0, pos: ONE_POS, neg: NO_SHAPES },
        '4' => Glyph { adv: 600.0, pos: FOUR_POS, neg: NO_SHAPES },
        '5' => Glyph { adv: 600.0, pos: FIVE_POS, neg: FIVE_NEG },
        '6' => Glyph { adv: 600.0, pos: SIX_POS, neg: SIX_NEG },
        '7' => Glyph { adv: 600.0, pos: SEVEN_POS, neg: NO_SHAPES },
        '9' => Glyph { adv: 600.0, pos: NINE_POS, neg: NINE_NEG },
        ' ' => Glyph { adv: 260.0, pos: NO_SHAPES, neg: NO_SHAPES },
        _ => Glyph { adv: 300.0, pos: NO_SHAPES, neg: NO_SHAPES },
    }
}

/// Advance width of a string at font size `size` (card units).
pub fn text_width(s: &str, size: f32) -> f32 {
    s.chars().map(|c| glyph(c).adv).sum::<f32>() * size / 1000.0
}

/// Coverage of one frame pixel by the centred string `s` drawn
/// at (cx, cy) with font size `size` — a straight 4×4 inside
/// test over the pixel's sub-sample points (no centre-distance
/// shortcut: the glyph inside test is the coverage estimator,
/// the same one cov_area uses inside the AA band).
fn text_cov(s: &str, size: f32, cx: f32, cy: f32, subs: &[(f32, f32); 16]) -> f32 {
    let scale = 1000.0 / size;
    let x0 = cx - text_width(s, size) / 2.0;
    // Cheap reject per sub-point is inside the loop; the band
    // check (cap height 720/1000 em, centred) skips most points.
    let mut inside = 0u32;
    for &(lx, ly) in subs {
        let gy = (ly - cy) * scale;
        if !(-360.0..360.0).contains(&gy) || lx < x0 {
            continue;
        }
        let mut pen = x0;
        for ch in s.chars() {
            let g = glyph(ch);
            let adv_px = g.adv / scale;
            if lx < pen + adv_px {
                if g.contains((lx - pen) * scale, gy) {
                    inside += 1;
                }
                break;
            }
            pen += adv_px;
        }
    }
    inside as f32 / 16.0
}

// ----------------------------------------------------------------
// The card, painted in card-local space. Layer order == canvas
// draw order in drawCalendar: shadow, card fill, card stroke,
// header band, header text, year, sub text, rule, flip shading.
// The canvas globalAlpha is applied the way film.rs applies it
// to the plates: as a final scale of the composited alpha.
// ----------------------------------------------------------------

fn paint_local(c: (f32, f32), subs: &[(f32, f32); 16], st: &CalState) -> [u8; 4] {
    let mut px = [0u8; 4];
    // 1. Drop shadow: card rect offset (+8, +10), rgba(20,16,10,.35)
    fill_sd(&mut px, COL_SHADOW, c, subs, &|x, y| {
        sd_round_box(x, y, SHADOW_CX, SHADOW_CY, CARD_HX, CARD_HY, [CARD_R; 4])
    });
    // 2. Card body #F4EBD6
    fill_sd(&mut px, COL_CARD, c, subs, &|x, y| sd_card(x, y));
    // 3. Card stroke #1F1C33, lineWidth 7 centred on the path
    fill_sd(&mut px, COL_INK, c, subs, &|x, y| sd_card(x, y).abs() - 3.5);
    // 4. Header band #8A3038, radii [14,14,0,0]
    fill_sd(&mut px, COL_HEADER, c, subs, &|x, y| {
        sd_round_box(x, y, 0.0, HEADER_CY, CARD_HX, HEADER_HY, [CARD_R, CARD_R, 0.0, 0.0])
    });
    // 5. Header text 'C A L E N D A R', 800 30px, #F4EBD6 @ (0,-166)
    let cov = text_cov("C A L E N D A R", 30.0, 0.0, -166.0, subs);
    blend_cov_px(&mut px, COL_CARD, cov);
    // 6. Year, 900 118px, #1F1C33 @ (0, 8)
    let year_str = st.year.to_string();
    let cov = text_cov(&year_str, 118.0, 0.0, 8.0, subs);
    blend_cov_px(&mut px, COL_INK, cov);
    // 7. Sub text 'LUBANG ISLAND', 800 25px, #8B8378 @ (0, 118)
    let cov = text_cov("LUBANG ISLAND", 25.0, 0.0, 118.0, subs);
    blend_cov_px(&mut px, COL_SUB, cov);
    // 8. Rule, fillRect(-110, 158, 220, 5), #8B8378
    fill_sd(&mut px, COL_SUB, c, subs, &|x, y| {
        sd_round_box(x, y, 0.0, RULE_CY, RULE_HX, RULE_HY, [0.0; 4])
    });
    // 9. Flip shading rgba(30,25,18, dark·0.22) over the card
    if st.dark > 0.0 {
        let cov = cov_area(&|x, y| sd_card(x, y), c, subs) * st.dark * 0.22;
        blend_cov_px(&mut px, [COL_DARK[0], COL_DARK[1], COL_DARK[2], 255], cov);
    }
    // Canvas globalAlpha, applied as film.rs applies it to the
    // plates: scale the composited (straight) alpha.
    px[3] = (px[3] as f32 * st.alpha).round() as u8;
    px
}

// ----------------------------------------------------------------
// Frame mapping. Forward: frame = T + R(−0.03) · (0.92·lx,
// 0.92·sy·ly). Inverse per pixel (pixel centres), exactly the
// mapping film.rs overlay_calendar applies to the plates (but
// at pixel centres, per the painter's convention).
// ----------------------------------------------------------------

fn to_local(fx: f32, fy: f32, sy: f32) -> (f32, f32) {
    let (dx, dy) = (fx - ORIGIN.0, fy - ORIGIN.1);
    let (s, c) = (0.03f32).sin_cos(); // inverse rotation R(+0.03)
    let (rx, ry) = (dx * c - dy * s, dx * s + dy * c);
    (rx / SCALE, ry / (SCALE * sy))
}

fn fwd(lx: f32, ly: f32, sy: f32) -> (f32, f32) {
    let (s, c) = ROT.sin_cos();
    let (x, y) = (lx * SCALE, ly * SCALE * sy);
    (ORIGIN.0 + x * c - y * s, ORIGIN.1 + x * s + y * c)
}

const SUB_OFF: [f32; 4] = [-0.375, -0.125, 0.125, 0.375];

/// Visit every frame pixel the card can touch, in the plate
/// bbox (plus a small AA margin), with the painted overlay pixel.
fn for_each_card_pixel<F: FnMut(u32, u32, [u8; 4])>(w: u32, h: u32, st: &CalState, mut f: F) {
    let corners = [
        fwd(-PLATE_HX, -PLATE_HY, st.sy),
        fwd(PLATE_HX, -PLATE_HY, st.sy),
        fwd(-PLATE_HX, PLATE_HY, st.sy),
        fwd(PLATE_HX, PLATE_HY, st.sy),
    ];
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in &corners {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let (ix0, iy0) = ((x0.floor() as i32 - 1).max(0), (y0.floor() as i32 - 1).max(0));
    let (ix1, iy1) = ((x1.ceil() as i32 + 1).min(w as i32 - 1), (y1.ceil() as i32 + 1).min(h as i32 - 1));
    for y in iy0..=iy1 {
        for x in ix0..=ix1 {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let c = to_local(fx, fy, st.sy);
            if c.0.abs() > PLATE_HX + 2.0 || c.1.abs() > PLATE_HY + 2.0 {
                continue;
            }
            let mut subs = [(0.0f32, 0.0f32); 16];
            let mut i = 0;
            for &oy in &SUB_OFF {
                for &ox in &SUB_OFF {
                    subs[i] = to_local(fx + ox, fy + oy, st.sy);
                    i += 1;
                }
            }
            let px = paint_local(c, &subs, st);
            if px[3] > 0 {
                f(x as u32, y as u32, px);
            }
        }
    }
}

/// The standalone overlay: a full 1080×1920 RGBA buffer
/// (straight alpha, transparent elsewhere) for film time `t`,
/// or None while the card is off screen.
pub fn render_overlay(t: f64) -> Option<Vec<u8>> {
    let st = state_at(t)?;
    let mut buf = vec![0u8; (FRAME_W * FRAME_H * 4) as usize];
    for_each_card_pixel(FRAME_W, FRAME_H, &st, |x, y, px| {
        let k = ((y * FRAME_W + x) * 4) as usize;
        buf[k..k + 4].copy_from_slice(&px);
    });
    Some(buf)
}

/// Composite the card over an existing frame buffer (any size;
/// RGB lerp by source alpha in sRGB space — film.rs blend_over's
/// arithmetic; the destination alpha byte is left unchanged).
/// Returns false while the card is off screen.
pub fn composite_over(px: &mut [u8], w: u32, h: u32, t: f64) -> bool {
    let st = match state_at(t) {
        Some(st) => st,
        None => return false,
    };
    for_each_card_pixel(w, h, &st, |x, y, src| {
        let a = (src[3] as f32 / 255.0).clamp(0.0, 1.0);
        if a <= 0.0 {
            return;
        }
        let k = ((y * w + x) * 4) as usize;
        for i in 0..3 {
            px[k + i] = (src[i] as f32 * a + px[k + i] as f32 * (1.0 - a)).round() as u8;
        }
    });
    true
}

// ----------------------------------------------------------------
// Tests
// ----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    /// Frame pixel whose centre is nearest the forward-mapped
    /// card-local point (at flip scale `sy`).
    fn frame_px_of_local(lx: f32, ly: f32, sy: f32) -> (u32, u32) {
        let (fx, fy) = fwd(lx, ly, sy);
        (fx.floor() as u32, fy.floor() as u32)
    }

    fn sample(buf: &[u8], x: u32, y: u32) -> [u8; 4] {
        let k = ((y * FRAME_W + x) * 4) as usize;
        [buf[k], buf[k + 1], buf[k + 2], buf[k + 3]]
    }

    #[test]
    fn alpha_law_boundaries() {
        // Fade-in: alpha = clamp(lt / 0.18)
        assert_eq!(state_at(T0_FILM).unwrap().alpha, 0.0);
        assert!(approx(state_at(T0_FILM + 0.09).unwrap().alpha, 0.5, 1e-6));
        assert!(approx(state_at(T0_FILM + 0.18).unwrap().alpha, 1.0, 1e-6));
        // Full alpha through the shot (shot ends at lt 3.659).
        assert!(approx(state_at(T0_FILM + 2.0).unwrap().alpha, 1.0, 1e-6));
        assert!(approx(state_at(T0_FILM + 3.659).unwrap().alpha, 1.0, 1e-6));
        // Fade-out: alpha = clamp((4.4 − lt) / 0.35)
        assert!(approx(state_at(T0_FILM + 4.225).unwrap().alpha, 0.5, 1e-6));
        // The fade reaches 0 at lt = 4.4: (4.4 − 4.399)/0.35.
        // (Exactly at the end point, f64 dust in (30.55 + 4.4)
        // − 30.55 decides between alpha ≈ 0 and off-screen —
        // the same knife-edge the canvas source and film.rs
        // have; the card is invisible either way.)
        assert!(approx(state_at(T0_FILM + 4.399).unwrap().alpha, 0.001 / 0.35, 1e-4));
        // Off screen outside the life.
        assert!(state_at(T0_FILM - 0.01).is_none());
        assert!(state_at(T0_FILM + 4.41).is_none());
    }

    #[test]
    fn years_at_task_times() {
        // t = 31.0 → lt 0.45: first hold, 1945, no flip.
        let st = state_at(31.0).unwrap();
        assert_eq!(st.year, 1945);
        assert_eq!(st.sy, 1.0);
        assert_eq!(st.dark, 0.0);
        // t = 31.5 → lt 0.95: flip 1 at u = 0.786 → second year.
        let st = state_at(31.5).unwrap();
        assert_eq!(st.year, 1955);
        assert!(st.sy < 1.0 && st.dark > 0.0);
        // t = 32.5 → lt 1.95: flip 2 at u = 0.690 → third year.
        assert_eq!(state_at(32.5).unwrap().year, 1965);
        // t = 33.5 → lt 2.95: flip 3 at u = 0.595 → final year.
        assert_eq!(state_at(33.5).unwrap().year, 1974);
        // Steady 1974 from lt 3.12 (film 33.67) to the cut.
        let st = state_at(34.0).unwrap();
        assert_eq!(st.year, 1974);
        assert_eq!(st.sy, 1.0);
        assert_eq!(st.dark, 0.0);
    }

    #[test]
    fn flip_sy_floor() {
        // u = 0.5 at lt 0.83: |cos(u·π)| = 0 → sy floors at 0.1.
        let st = state_at(T0_FILM + 0.83).unwrap();
        assert!(approx(st.sy, 0.1, 1e-6), "sy = {}", st.sy);
        assert!(approx(st.dark, 0.9, 1e-6));
        // The year switch is u >= 0.5 (source: u < 0.5 ? years[idx]
        // : years[idx+1]); lt 0.83 lands within f64 dust of the
        // knife-edge, so assert the switch from either side.
        assert_eq!(state_at(T0_FILM + 0.82).unwrap().year, 1945);
        assert_eq!(state_at(T0_FILM + 0.84).unwrap().year, 1955);
    }

    #[test]
    fn card_pixel_colours() {
        // lt 0.3: alpha 1, sy 1, year 1945.
        let buf = render_overlay(T0_FILM + 0.3).unwrap();
        assert_eq!(buf.len(), (FRAME_W * FRAME_H * 4) as usize);
        // Bare card interior (below the year, above the sub text).
        let (x, y) = frame_px_of_local(0.0, 65.0, 1.0);
        assert_eq!(sample(&buf, x, y), COL_CARD, "card interior");
        // Header band, left of the header text.
        let (x, y) = frame_px_of_local(-120.0, -167.0, 1.0);
        assert_eq!(sample(&buf, x, y), COL_HEADER, "header band");
        // The rule.
        let (x, y) = frame_px_of_local(0.0, 160.0, 1.0);
        assert_eq!(sample(&buf, x, y), COL_SUB, "rule");
        // Year glyph ink: inside the '9' ring of '1945'.
        let (x, y) = frame_px_of_local(-35.0, 20.0, 1.0);
        assert_eq!(sample(&buf, x, y), COL_INK, "year ink");
        // Header glyph ink: inside the 'C' stroke.
        let (x, y) = frame_px_of_local(-103.0, -166.0, 1.0);
        assert_eq!(sample(&buf, x, y), COL_CARD, "header ink");
        // Far outside the card: transparent.
        assert_eq!(sample(&buf, 5, 5), [0, 0, 0, 0]);
    }

    #[test]
    fn dark_shading_midflip() {
        // lt 0.83: u = 0.5 → dark 0.9; shade = (30,25,18) at
        // 0.9·0.22 = 0.198 over the card colour → ≈ (202,194,176).
        let t = T0_FILM + 0.83;
        let st = state_at(t).unwrap();
        let buf = render_overlay(t).unwrap();
        let (x, y) = frame_px_of_local(0.0, 65.0, st.sy);
        let px = sample(&buf, x, y);
        assert_eq!(px[3], 255, "shaded card stays opaque");
        for (got, want) in px[..3].iter().zip([202u8, 194, 176]) {
            assert!((*got as i32 - want as i32).abs() <= 3, "channel {got} vs {want}");
        }
    }

    #[test]
    fn text_widths_fit_the_card() {
        // All three strings must fit the 320-wide card.
        assert!(text_width("C A L E N D A R", 30.0) < 320.0);
        assert!(text_width("1945", 118.0) < 320.0);
        assert!(text_width("LUBANG ISLAND", 25.0) < 320.0);
        // Digits are tabular: every year is the same width.
        for y in ["1945", "1955", "1965", "1974"] {
            assert!(approx(text_width(y, 118.0), 283.2, 0.01), "{y}");
        }
    }

    #[test]
    fn off_screen_renders_nothing() {
        assert!(render_overlay(T0_FILM - 1.0).is_none());
        assert!(render_overlay(T0_FILM + 5.0).is_none());
        let mut frame = vec![0u8; (FRAME_W * FRAME_H * 4) as usize];
        assert!(!composite_over(&mut frame, FRAME_W, FRAME_H, T0_FILM - 1.0));
        assert!(composite_over(&mut frame, FRAME_W, FRAME_H, T0_FILM + 0.3));
    }
}
