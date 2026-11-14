//! Full drawToon port (production integration): the complete
//! character painter from the source engine's toon.ts — all 17
//! moods, walk/run/sit/jump legs, patterns (vest), neckwear (tie,
//! scarf), hats (cap, bowler), extras (glasses, mustache), blink /
//! talk / look / sweat / tears, headTilt / headTurn / camPitch /
//! squash, and held-prop plates — where scene::paint_toon (M4) is
//! the five-mood stand subset the §28 scene needs.
//!
//! The raster model is the M4 one (signed-distance coverage via
//! scene::Painter), and the transform convention matches
//! paint_toon: geometry is computed in stage space (512x1024, feet
//! at y = 1000) and mapped through the lean rotation into the
//! 1280x1408 canvas at res 2. Head-part geometry additionally
//! passes the squash (about the head centre) and headTilt (about
//! the neck pivot) stage-space transforms, in the canvas's own
//! nesting order (lean outermost, then tilt, then squash).
//!
//! Held props arrive as pre-rendered plates (RGBA, stage units at
//! res 2, rendered by film/gen_assets.py from the same prop
//! geometry — including the Inter text, which enters as data).
//! Every prop this film uses ignores its `angle` argument (paper
//! rotates a fixed -0.08, the leaflet -0.1, book/coconut none), so
//! a plate composites by translation to the hand point alone,
//! inverse-mapped through the lean rotation.

use crate::scene::{
    ellipse_sd, flatten_cubic, flatten_quad, hex_rgb, shade_rgb, Painter, Rgba,
};

pub(crate) const INK: &str = "#1F1C33";
const RES: f32 = 2.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mood {
    Neutral,
    Smug,
    Happy,
    Laugh,
    Shock,
    Worried,
    Angry,
    Sad,
    Sly,
    Proud,
    Tired,
    Dizzy,
    Determined,
    Embarrassed,
    Awe,
    Suspicious,
    Relieved,
}

impl Mood {
    pub(crate) fn from_str(s: &str) -> Mood {
        match s {
            "smug" => Mood::Smug,
            "happy" => Mood::Happy,
            "laugh" => Mood::Laugh,
            "shock" => Mood::Shock,
            "worried" => Mood::Worried,
            "angry" => Mood::Angry,
            "sad" => Mood::Sad,
            "sly" => Mood::Sly,
            "proud" => Mood::Proud,
            "tired" => Mood::Tired,
            "dizzy" => Mood::Dizzy,
            "determined" => Mood::Determined,
            "embarrassed" => Mood::Embarrassed,
            "awe" => Mood::Awe,
            "suspicious" => Mood::Suspicious,
            "relieved" => Mood::Relieved,
            _ => Mood::Neutral,
        }
    }
    /// drawFace brow table: [lift, tilt].
    fn brow(self) -> (f32, f32) {
        match self {
            Mood::Neutral => (4.0, 0.0),
            Mood::Smug => (-8.0, 10.0),
            Mood::Happy => (8.0, 0.0),
            Mood::Laugh => (10.0, 0.0),
            Mood::Shock => (24.0, 0.0),
            Mood::Worried => (10.0, -16.0),
            Mood::Angry => (-10.0, 22.0),
            Mood::Sad => (8.0, -18.0),
            Mood::Sly => (-6.0, 12.0),
            Mood::Proud => (6.0, 4.0),
            Mood::Tired => (-2.0, 0.0),
            Mood::Dizzy => (14.0, 0.0),
            Mood::Determined => (-4.0, 16.0),
            Mood::Embarrassed => (12.0, -10.0),
            Mood::Awe => (26.0, 0.0),
            Mood::Suspicious => (-2.0, 6.0),
            Mood::Relieved => (14.0, -4.0),
        }
    }
    fn heavy_lid(self) -> bool {
        matches!(self, Mood::Smug | Mood::Tired | Mood::Sly | Mood::Proud | Mood::Determined)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Legs {
    Stand,
    Walk,
    Run,
    Sit,
    Jump,
}

/// A held-prop plate: RGBA pixels at res 2 in stage units.
/// (x0, y0) is the stage-local coordinate of the plate's top-left
/// corner relative to the hand point the PropFn receives.
pub(crate) struct Plate {
    pub px: Vec<u8>,
    pub w: u32,
    pub h: u32,
    pub x0: f32,
    pub y0: f32,
}

pub(crate) struct FullLook {
    pub body: String,
    pub head: String,
    pub shade: Option<String>,
    pub pattern: String, // 'plain' | 'vest' (the film's set)
    pub pat_col: Option<String>,
    pub hat: String, // 'none' | 'cap' | 'bowler'
    pub hat_col: Option<String>,
    pub band: Option<String>,
    pub neck: String, // 'none' | 'tie' | 'scarf'
    pub neck_col: Option<String>,
    pub glasses: bool,
    pub mustache: bool,
    pub mustache_col: Option<String>,
    pub kid: bool,
}

pub(crate) struct FullState {
    pub mood: Mood,
    pub look: [f32; 2],
    pub blink: f32,
    pub talk: f32,
    pub sweat: bool, // drawFace: sweat drop also fires on 'worried'
    pub tears: bool, // drawFace: tears also fire on 'sad'
    pub arm_l: (f32, f32),
    pub arm_r: (f32, f32),
    pub legs: Legs,
    pub phase: f32,
    pub lean: f32,
    pub bob: f32,
    pub squash: f32,
    pub head_tilt: f32,
    pub head_turn: f32,
    pub cam_pitch: f32,
    pub sit_drop: Option<f32>,
    pub hold_l: Option<Plate>,
    pub hold_r: Option<Plate>,
}

impl Default for FullState {
    fn default() -> Self {
        FullState {
            mood: Mood::Neutral,
            look: [0.0, 0.0],
            blink: 0.0,
            talk: 0.0,
            sweat: false,
            tears: false,
            arm_l: (8.0, 6.0),
            arm_r: (8.0, 6.0),
            legs: Legs::Stand,
            phase: 0.0,
            lean: 0.0,
            bob: 0.0,
            squash: 0.0,
            head_tilt: 0.0,
            head_turn: 0.0,
            cam_pitch: 0.0,
            sit_drop: None,
            hold_l: None,
            hold_r: None,
        }
    }
}

fn rgba(s: &str) -> Rgba {
    let (r, g, b) = hex_rgb(s);
    (r, g, b, 255)
}

fn shade_rgba(s: &str, k: f32) -> Rgba {
    let (r, g, b) = shade_rgb(s, k);
    (r, g, b, 255)
}

/// Fill a polygon through a mask (the bean clip for patterns):
/// per-pixel centre test against both masks over the bbox.
fn fill_polygon_masked(p: &mut Painter, pts: &[(f32, f32)], mask: &[bool], col: Rgba) {
    let own = p.polygon_mask(pts);
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in pts {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let (bx0, by0, bx1, by1) = p.bbox(x0, y0, x1, y1);
    for y in by0..=by1 {
        for x in bx0..=bx1 {
            let k = y as usize * p.w + x as usize;
            if mask[k] && own[k] {
                p.blend(x, y, col);
            }
        }
    }
}

/// Composite a held-prop plate at a stage-space hand point,
/// inverse-mapping each canvas pixel through the lean rotation
/// (stage -> canvas is lean about (256, 1000), then +(384, 256),
/// then x res). Bilinear sampling of the plate.
fn composite_plate(p: &mut Painter, plate: &Plate, hand: (f32, f32), lean_rad: f32) {
    let (s, c) = lean_rad.sin_cos();
    // Canvas-space corners of the plate rect (forward map).
    let fwd = |lx: f32, ly: f32| -> (f32, f32) {
        let (dx, dy) = (lx - 256.0, ly - 1000.0);
        let (rx, ry) = (256.0 + dx * c - dy * s, 1000.0 + dx * s + dy * c);
        ((rx + 384.0) * RES, (ry + 256.0) * RES)
    };
    let corners = [
        fwd(hand.0 + plate.x0, hand.1 + plate.y0),
        fwd(hand.0 + plate.x0 + plate.w as f32 / RES, hand.1 + plate.y0),
        fwd(hand.0 + plate.x0, hand.1 + plate.y0 + plate.h as f32 / RES),
        fwd(hand.0 + plate.x0 + plate.w as f32 / RES, hand.1 + plate.y0 + plate.h as f32 / RES),
    ];
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in &corners {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let (bx0, by0, bx1, by1) = p.bbox(x0, y0, x1, y1);
    let (pw, ph) = (plate.w as f32, plate.h as f32);
    for y in by0..=by1 {
        for x in bx0..=bx1 {
            // canvas -> stage (inverse lean).
            let (ux, uy) = (x as f32 / RES - 384.0, y as f32 / RES - 256.0);
            let (dx, dy) = (ux - 256.0, uy - 1000.0);
            let sx = 256.0 + dx * c + dy * s;
            let sy = 1000.0 - dx * s + dy * c;
            let lx = (sx - hand.0 - plate.x0) * RES;
            let ly = (sy - hand.1 - plate.y0) * RES;
            if lx < -0.5 || ly < -0.5 || lx > pw - 0.5 || ly > ph - 0.5 {
                continue;
            }
            // Bilinear sample (premultiplied blend via blend_cov on
            // the sampled colour with the sampled alpha).
            let fx = lx.clamp(0.0, pw - 1.001);
            let fy = ly.clamp(0.0, ph - 1.001);
            let (ix, iy) = (fx as usize, fy as usize);
            let (tx, ty) = (fx - ix as f32, fy - iy as f32);
            let sample = |xx: usize, yy: usize| -> [f32; 4] {
                let k = (yy * plate.w as usize + xx) * 4;
                [
                    plate.px[k] as f32,
                    plate.px[k + 1] as f32,
                    plate.px[k + 2] as f32,
                    plate.px[k + 3] as f32,
                ]
            };
            let s00 = sample(ix, iy);
            let s10 = sample(ix + 1, iy);
            let s01 = sample(ix, iy + 1);
            let s11 = sample(ix + 1, iy + 1);
            let mut out = [0.0f32; 4];
            for ch in 0..4 {
                out[ch] = (s00[ch] * (1.0 - tx) + s10[ch] * tx) * (1.0 - ty)
                    + (s01[ch] * (1.0 - tx) + s11[ch] * tx) * ty;
            }
            let a = out[3] / 255.0;
            if a > 0.0 {
                p.blend_cov(
                    x,
                    y,
                    (out[0] as u8, out[1] as u8, out[2] as u8, 255),
                    a,
                );
            }
        }
    }
}

/// The full painter. Returns (pixels, w, h, hands) with hands in
/// stage coordinates, exactly as drawToon returns them.
pub(crate) fn paint_toon_full(
    look: &FullLook,
    st: &FullState,
) -> (Vec<u8>, u32, u32, (f32, f32), (f32, f32)) {
    let (cw, ch) = (1280usize * 2, 1408usize * 2);
    let mut p = Painter::new(cw, ch);
    let ink = rgba(INK);
    let lw = |w: f32| w * RES;
    let th = st.lean.to_radians();
    let (sth, cth) = th.sin_cos();
    // Stage -> canvas: lean about the feet (256, 1000), then the
    // canvas offset, then res (as paint_toon / Toon#set).
    let px = |x: f32, y: f32| -> (f32, f32) {
        let (dx, dy) = (x - 256.0, y - 1000.0);
        (
            (256.0 + dx * cth - dy * sth + 384.0) * RES,
            (1000.0 + dx * sth + dy * cth + 256.0) * RES,
        )
    };
    // Head-part point map: squash about (hx, hy), then headTilt
    // about (256, pivotY), then the lean map.
    let sit = st.legs == Legs::Sit;
    let jump = st.legs == Legs::Jump;
    let sit_s = st.sit_drop.unwrap_or(220.2);
    let by = (if sit { (426.0 - sit_s) / 0.98 } else { 0.0 })
        + if look.kid { 190.0 } else { 0.0 }
        + st.bob
        + if jump { -60.0 } else { 0.0 };
    let (bx, top, bot, bw) = (256.0f32, 330.0 + by, 600.0 + by * 0.98, 92.0f32);
    let hip_y = bot - 26.0;
    let (hx, hy, r) = (256.0f32, 178.0 + by * 0.99, 138.0f32);
    let pivot_y = 318.0 + by * 0.99;
    let tilt = st.head_tilt.to_radians();
    let (sti, cti) = tilt.sin_cos();
    let sq = st.squash;
    let hpt = |x: f32, y: f32| -> (f32, f32) {
        // squash about the head centre
        let (sx, sy) = (hx + (x - hx) * (1.0 - 0.3 * sq), hy + (y - hy) * (1.0 + sq));
        // tilt about the neck pivot
        let (dx, dy) = (sx - 256.0, sy - pivot_y);
        let (rx, ry) = (256.0 + dx * cti - dy * sti, pivot_y + dx * sti + dy * cti);
        px(rx, ry)
    };
    let head_rot = th + tilt;

    // ---- legs ----
    for (sx0, side) in [(bx - 42.0, -1.0f32), (bx + 42.0, 1.0f32)] {
        if sit {
            let (kx, ky) = (sx0 + side * 30.0, hip_y + 44.0);
            let (fx, fy) = (sx0 + side * 34.0, (hip_y + 220.2).min(1000.0));
            p.stroke_polyline(
                &[px(sx0, hip_y), px(kx, ky), px(fx, fy)],
                lw(15.0),
                ink,
            );
            let (ex, ey) = px(fx + side * 10.0, fy - 4.0);
            p.fill_ellipse(ex, ey, 20.0 * RES, 10.0 * RES, th, ink);
            continue;
        }
        let (kx, ky, fx, fy) = match st.legs {
            Legs::Walk | Legs::Run => {
                let amp = if st.legs == Legs::Run { 1.0 } else { 0.55 };
                let a = (st.phase + if side > 0.0 { std::f32::consts::PI } else { 0.0 }).sin() * amp;
                let lift = (-(st.phase + if side > 0.0 { std::f32::consts::PI } else { 0.0 }).cos())
                    .max(0.0)
                    * if st.legs == Legs::Run { 70.0 } else { 34.0 };
                (
                    sx0 + a * 22.0 + side * 6.0,
                    hip_y + 200.0 - lift * 0.6,
                    sx0 + a * 34.0 + side * 8.0,
                    1000.0 - lift,
                )
            }
            Legs::Jump => (sx0 + side * 40.0, hip_y + 170.0, sx0 + side * 18.0, 940.0),
            _ => (sx0 + side * 4.0, hip_y + 210.0, sx0 + side * 10.0, 1000.0),
        };
        let pts = flatten_quad(px(sx0, hip_y), px(kx, ky), px(fx, fy), 20);
        p.stroke_polyline(&pts, lw(15.0), ink);
        let (ex, ey) = px(fx + side * 10.0, fy - 4.0);
        p.fill_ellipse(ex, ey, 20.0 * RES, 10.0 * RES, th, ink);
    }

    // ---- body bean ----
    let mut bean: Vec<(f32, f32)> = Vec::new();
    let mut seg = |pts: Vec<(f32, f32)>| {
        if bean.is_empty() {
            bean.extend_from_slice(&pts);
        } else {
            bean.extend_from_slice(&pts[1..]);
        }
    };
    seg(flatten_cubic(px(bx - 58.0, top + 10.0), px(bx - 80.0, top + 40.0), px(bx - bw - 6.0, bot - 120.0), px(bx - bw, bot - 50.0), 28));
    seg(flatten_cubic(px(bx - bw, bot - 50.0), px(bx - bw + 4.0, bot + 6.0), px(bx + bw - 4.0, bot + 6.0), px(bx + bw, bot - 50.0), 28));
    seg(flatten_cubic(px(bx + bw, bot - 50.0), px(bx + bw + 6.0, bot - 120.0), px(bx + 80.0, top + 40.0), px(bx + 58.0, top + 10.0), 28));
    seg(flatten_cubic(px(bx + 58.0, top + 10.0), px(bx + 30.0, top - 8.0), px(bx - 30.0, top - 8.0), px(bx - 58.0, top + 10.0), 28));
    p.fill_polygon(&bean, rgba(&look.body));
    let mask = p.polygon_mask(&bean);
    // Pattern (clipped to the bean, as drawPattern runs inside the
    // body's clip): only 'vest' occurs in this film besides plain.
    if look.pattern == "vest" {
        let pc = look.pat_col.as_deref().unwrap_or("#3B3346");
        for s in [-1.0f32, 1.0] {
            let panel = [
                px(bx + s * 140.0, top - 10.0),
                px(bx + s * 8.0, top + 120.0),
                px(bx + s * 8.0, bot + 20.0),
                px(bx + s * 140.0, bot + 20.0),
            ];
            fill_polygon_masked(&mut p, &panel, &mask, rgba(pc));
        }
        for dy in [150.0f32, 195.0, 240.0] {
            let (cxp, cyp) = px(bx + 26.0, top + dy);
            p.fill_ellipse(cxp, cyp, 6.0 * RES, 6.0 * RES, th, rgba("#E9C46A"));
        }
    }
    // Shade + highlight, clipped.
    let (shx, shy) = px(bx + bw + 40.0, (top + bot) / 2.0 + 20.0);
    let shade_col = match &look.shade {
        Some(s) => rgba(s.as_str()),
        None => shade_rgba(&look.body, 0.8),
    };
    p.fill_ellipse_masked(shx, shy, 70.0 * RES, 260.0 * RES, th, (shade_col.0, shade_col.1, shade_col.2, (0.85 * 255.0) as u8), &mask);
    let (hix, hiy) = px(bx - 50.0, top + 90.0);
    p.fill_ellipse_masked(hix, hiy, 16.0 * RES, 60.0 * RES, 0.2 + th, (255, 255, 255, (0.25 * 255.0) as u8), &mask);
    {
        let mut ring = bean.clone();
        ring.push(bean[0]);
        p.stroke_polyline(&ring, lw(9.0), ink);
    }

    // ---- neckwear ----
    match look.neck.as_str() {
        "tie" => {
            let c = look.neck_col.as_deref().unwrap_or("#D8343F");
            let poly = [
                px(bx - 14.0, top + 10.0),
                px(bx + 14.0, top + 10.0),
                px(bx + 20.0, top + 140.0),
                px(bx, top + 170.0),
                px(bx - 20.0, top + 140.0),
            ];
            p.fill_polygon(&poly, rgba(c));
            let mut ring = poly.to_vec();
            ring.push(poly[0]);
            p.stroke_polyline(&ring, lw(6.0), ink);
        }
        "scarf" => {
            let c = look.neck_col.as_deref().unwrap_or("#D8343F");
            let tail = [
                px(bx + 20.0, top + 20.0),
                px(bx + 58.0, top + 130.0),
                px(bx + 24.0, top + 138.0),
                px(bx + 6.0, top + 26.0),
            ];
            p.fill_polygon(&tail, rgba(c));
            let mut ring = tail.to_vec();
            ring.push(tail[0]);
            p.stroke_polyline(&ring, lw(6.0), ink);
            // Wrap: two quads forming the band, then its shade line.
            let mut band: Vec<(f32, f32)> = Vec::new();
            band.extend_from_slice(&flatten_quad(px(bx - 70.0, top + 4.0), px(bx, top + 44.0), px(bx + 70.0, top + 4.0), 16));
            let lower = flatten_quad(px(bx + 70.0, top + 4.0), px(bx + 66.0, top + 34.0), px(bx, top + 76.0), 16);
            band.extend_from_slice(&lower[1..]);
            let lower2 = flatten_quad(px(bx, top + 76.0), px(bx - 66.0, top + 34.0), px(bx - 70.0, top + 4.0), 16);
            band.extend_from_slice(&lower2[1..]);
            p.fill_polygon(&band, rgba(c));
            let mut bring = band.clone();
            bring.push(band[0]);
            p.stroke_polyline(&bring, lw(6.0), ink);
            let shade = shade_rgba(c, 0.75);
            p.stroke_quad(
                px(bx - 50.0, top + 26.0),
                px(bx, top + 56.0),
                px(bx + 50.0, top + 26.0),
                lw(4.0),
                shade,
            );
        }
        _ => {}
    }

    // ---- arms (+ held plates, then the hand) ----
    let mut hands = ((0.0f32, 0.0f32), (0.0f32, 0.0f32));
    for (side, (a, e), hold) in [
        (-1.0f32, st.arm_l, &st.hold_l),
        (1.0f32, st.arm_r, &st.hold_r),
    ] {
        let (sx, sy) = (bx + side * 66.0, top + 46.0);
        let a1 = (a as f64).to_radians();
        let a2 = ((a + e) as f64).to_radians();
        let ex = sx + side * (a1.sin() as f32) * 118.0;
        let ey = sy + (a1.cos() as f32) * 118.0;
        let hx2 = ex + side * (a2.sin() as f32) * 112.0;
        let hy2 = ey + (a2.cos() as f32) * 112.0;
        let pts = flatten_quad(px(sx, sy), px(ex, ey), px(hx2, hy2), 20);
        p.stroke_polyline(&pts, lw(14.0), ink);
        if let Some(plate) = hold {
            composite_plate(&mut p, plate, (hx2, hy2), th);
        }
        let (hxp, hyp) = px(hx2, hy2);
        p.fill_ellipse(hxp, hyp, 15.0 * RES, 17.0 * RES, th, ink);
        if side < 0.0 {
            hands.0 = (hx2, hy2);
        } else {
            hands.1 = (hx2, hy2);
        }
    }

    // ---- head ----
    let turn = st.head_turn;
    let rp = r * (1.0 + 0.04 * st.cam_pitch);
    let fx = hx + turn * 30.0;
    // Head outline.
    let (chx, chy) = hpt(hx, hy);
    p.fill_ellipse(chx, chy, rp * (1.0 - 0.3 * sq) * RES, rp * 0.97 * (1.0 + sq) * RES, head_rot, rgba(&look.head));
    // Even-odd shade: inside the head ellipse, outside the offset
    // circle (coverage form, as paint_toon).
    {
        let (ccx, ccy) = hpt(hx + 52.0 + turn * 30.0, hy + 44.0);
        let shade = shade_rgb(&look.head, 0.9);
        let col = (shade.0, shade.1, shade.2, 255u8);
        let (rx_h, ry_h) = (rp * (1.0 - 0.3 * sq) * RES, rp * 0.97 * (1.0 + sq) * RES);
        let (rx_c, ry_c) = (rp * (1.0 - 0.3 * sq) * RES, rp * (1.0 + sq) * RES);
        let rad = rp * RES * 1.05;
        let (x0, y0, x1, y1) = p.bbox(chx - rad, chy - rad, chx + rad, chy + rad);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (pxx, pyy) = (x as f32 + 0.5, y as f32 + 0.5);
                let cov_head = Painter::cov_of(ellipse_sd(chx, chy, rx_h, ry_h, head_rot, pxx, pyy));
                if cov_head <= 0.0 {
                    continue;
                }
                let d = ellipse_sd(ccx, ccy, rx_c, ry_c, head_rot, pxx, pyy);
                let unshaded = (0.5 - d).clamp(0.0, 1.0);
                let cov = cov_head * unshaded;
                if cov > 0.0 {
                    p.blend_cov(x, y, col, cov);
                }
            }
        }
    }
    p.stroke_ellipse(chx, chy, rp * (1.0 - 0.3 * sq) * RES, rp * 0.97 * (1.0 + sq) * RES, head_rot, lw(10.0), ink);

    // ---- face ----
    let m = st.mood;
    let ex = 46.0 * (1.0 - 0.18 * turn.abs());
    let ey = hy + 4.0 + st.cam_pitch * 8.0;
    let (lift0, tilt0) = m.brow();
    for s in [-1.0f32, 1.0] {
        let mut lift = lift0;
        let mut tilt = tilt0 * s;
        if (m == Mood::Smug || m == Mood::Sly) && s > 0.0 {
            lift = 16.0;
            tilt = -4.0;
        }
        if m == Mood::Suspicious && s < 0.0 {
            lift = 18.0;
            tilt = -6.0;
        }
        let y = ey - 44.0 - lift;
        p.stroke_quad(
            hpt(fx + s * (ex - 26.0), y + tilt * 0.5),
            hpt(fx + s * ex, y - 7.0),
            hpt(fx + s * (ex + 26.0), y - tilt * 0.5),
            lw(12.0),
            ink,
        );
    }
    // Eyes.
    for s in [-1.0f32, 1.0] {
        let x = fx + s * ex;
        let [lx, ly] = st.look;
        if m == Mood::Happy || m == Mood::Laugh || m == Mood::Relieved {
            let pts: Vec<(f32, f32)> = (0..=24)
                .map(|i| {
                    let a = std::f32::consts::PI * 1.15 + (i as f32 / 24.0) * std::f32::consts::PI * 0.7;
                    hpt(x + 20.0 * a.cos(), ey + 10.0 + 20.0 * a.sin())
                })
                .collect();
            p.stroke_polyline(&pts, lw(10.0), ink);
            continue;
        }
        if m == Mood::Shock || m == Mood::Awe {
            let (exp, eyp) = hpt(x, ey);
            p.fill_ellipse(exp, eyp, 26.0 * RES, 30.0 * RES, head_rot, rgba("#FFFFFF"));
            p.stroke_ellipse(exp, eyp, 26.0 * RES, 30.0 * RES, head_rot, lw(6.0), ink);
            let (ppx, ppy) = hpt(x + lx * 8.0, ey + ly * 8.0);
            let pr = if m == Mood::Awe { 9.0 } else { 7.0 };
            p.fill_ellipse(ppx, ppy, pr * RES, pr * RES, head_rot, ink);
            if m == Mood::Awe {
                let (spx, spy) = hpt(x + lx * 8.0 - 3.0, ey + ly * 8.0 - 3.0);
                p.fill_ellipse(spx, spy, 3.0 * RES, 3.0 * RES, head_rot, rgba("#FFFFFF"));
            }
            continue;
        }
        if st.blink > 0.6 {
            p.stroke_quad(
                hpt(x - 14.0, ey + 4.0),
                hpt(x, ey + 10.0),
                hpt(x + 14.0, ey + 4.0),
                lw(8.0),
                ink,
            );
            continue;
        }
        let (ppx, ppy) = hpt(x + lx * 9.0, ey + ly * 8.0);
        let slit = if m == Mood::Suspicious && s < 0.0 { 0.55 } else { 1.0 };
        p.fill_ellipse(
            ppx,
            ppy,
            13.0 * RES,
            17.0 * (1.0 - st.blink * 0.6) * slit * RES,
            head_rot,
            ink,
        );
        if m.heavy_lid() {
            // Head-colour rect over the top half + the lid line.
            let rect_pts = [
                hpt(x - 24.0, ey - 28.0),
                hpt(x + 24.0, ey - 28.0),
                hpt(x + 24.0, ey - 2.0 + if m == Mood::Tired { 6.0 } else { 0.0 }),
                hpt(x - 24.0, ey - 2.0 + if m == Mood::Tired { 6.0 } else { 0.0 }),
            ];
            p.fill_polygon(&rect_pts, rgba(&look.head));
            p.stroke_polyline(
                &[
                    hpt(x - 20.0, ey - 2.0 + if m == Mood::Tired { 6.0 } else { 0.0 }),
                    hpt(x + 20.0, ey - 2.0 + if m == Mood::Tired { 6.0 } else { 0.0 }),
                ],
                lw(7.0),
                ink,
            );
        }
        if m == Mood::Sad || m == Mood::Worried {
            let (spx, spy) = hpt(x + lx * 9.0 - 3.0, ey + ly * 8.0 - 5.0);
            p.fill_ellipse(spx, spy, 3.5 * RES, 3.5 * RES, head_rot, rgba("#FFFFFF"));
        }
    }
    if look.glasses {
        for s in [-1.0f32, 1.0] {
            let (gx, gy) = hpt(fx + s * ex, ey);
            p.stroke_ellipse(gx, gy, 30.0 * RES, 30.0 * RES, head_rot, lw(5.0), ink);
        }
        p.stroke_polyline(&[hpt(fx - 16.0, ey - 4.0), hpt(fx + 16.0, ey - 4.0)], lw(5.0), ink);
    }
    if m == Mood::Laugh || m == Mood::Proud || m == Mood::Embarrassed {
        let blush = (240u8, 120u8, 130u8, (0.35 * 255.0) as u8);
        for s in [-1.0f32, 1.0] {
            let (bxp, byp) = hpt(fx + s * 78.0, ey + 40.0);
            p.fill_ellipse(bxp, byp, 22.0 * RES, 12.0 * RES, head_rot, blush);
        }
    }
    // Mouth.
    let my = ey + 62.0;
    let t = st.talk;
    let open_mouth = |p: &mut Painter, w: f32, h: f32, tongue: bool| {
        let (mxp, myp) = hpt(fx, my + h * 0.3);
        p.fill_ellipse(mxp, myp, w * RES, h.max(4.0) * RES, head_rot, rgba("#3A1A2A"));
        p.stroke_ellipse(mxp, myp, w * RES, h.max(4.0) * RES, head_rot, lw(8.0), ink);
        if tongue && h > 12.0 {
            let msk = p.ellipse_mask(mxp, myp, w * RES, h.max(4.0) * RES, head_rot);
            let (txp, typ) = hpt(fx, my + h * 1.1);
            p.fill_ellipse_masked(txp, typ, w * 0.7 * RES, h * 0.6 * RES, head_rot, rgba("#E2697A"), &msk);
        }
    };
    if m == Mood::Shock {
        open_mouth(&mut p, 16.0, 20.0 + t * 6.0, true);
    } else if m == Mood::Laugh {
        // As in drawFace — including its stroke quirk (see
        // paint_toon): the final stroke() lands on the tongue.
        let mut pts: Vec<(f32, f32)> = Vec::new();
        let mut chain = |qs: Vec<(f32, f32)>| {
            if pts.is_empty() {
                pts.extend_from_slice(&qs);
            } else {
                pts.extend_from_slice(&qs[1..]);
            }
        };
        chain(flatten_quad(hpt(fx - 40.0, my - 6.0), hpt(fx, my - 2.0), hpt(fx + 40.0, my - 6.0), 24));
        chain(flatten_quad(hpt(fx + 40.0, my - 6.0), hpt(fx + 34.0, my + 52.0), hpt(fx, my + 52.0), 24));
        chain(flatten_quad(hpt(fx, my + 52.0), hpt(fx - 34.0, my + 52.0), hpt(fx - 40.0, my - 6.0), 24));
        p.fill_polygon(&pts, rgba("#3A1A2A"));
        let msk = p.polygon_mask(&pts);
        let (txp, typ) = hpt(fx, my + 50.0);
        p.fill_ellipse_masked(txp, typ, 24.0 * RES, 18.0 * RES, head_rot, rgba("#E2697A"), &msk);
        let (ax, ay) = hpt(fx - 40.0, my - 10.0);
        let (bxp, byp) = hpt(fx + 40.0, my + 2.0);
        let (x0, y0, x1, y1) = p.bbox(ax.min(bxp), ay.min(byp), ax.max(bxp), ay.max(byp));
        for yy in y0..=y1 {
            for xx in x0..=x1 {
                if msk[yy as usize * p.w + xx as usize] {
                    p.blend(xx, yy, rgba("#FFFFFF"));
                }
            }
        }
        p.stroke_ellipse(txp, typ, 24.0 * RES, 18.0 * RES, head_rot, lw(8.0), ink);
    } else if t > 0.05 {
        open_mouth(&mut p, 14.0 + t * 10.0, 6.0 + t * 18.0, true);
    } else {
        match m {
            Mood::Happy | Mood::Proud => {
                p.stroke_quad(hpt(fx - 28.0, my), hpt(fx, my + 26.0), hpt(fx + 28.0, my), lw(8.0), ink);
            }
            Mood::Smug | Mood::Sly => {
                p.stroke_quad(hpt(fx - 22.0, my + 6.0), hpt(fx + 4.0, my + 16.0), hpt(fx + 24.0, my - 4.0), lw(8.0), ink);
            }
            Mood::Worried => {
                let pts = flatten_cubic(
                    hpt(fx - 26.0, my + 8.0),
                    hpt(fx - 12.0, my - 4.0),
                    hpt(fx + 8.0, my + 14.0),
                    hpt(fx + 26.0, my + 2.0),
                    24,
                );
                p.stroke_polyline(&pts, lw(8.0), ink);
            }
            Mood::Angry => {
                p.stroke_quad(hpt(fx - 30.0, my + 12.0), hpt(fx, my - 8.0), hpt(fx + 30.0, my + 12.0), lw(8.0), ink);
                let pts: Vec<(f32, f32)> = crate::scene::rounded_rect_pts(fx - 24.0, my - 2.0, 48.0, 14.0, 6.0, 6)
                    .iter()
                    .map(|&(x, y)| hpt(x, y))
                    .collect();
                p.fill_polygon(&pts, rgba("#FFFFFF"));
                let mut ring = pts.clone();
                ring.push(pts[0]);
                p.stroke_polyline(&ring, lw(5.0), ink);
            }
            Mood::Sad => {
                p.stroke_quad(hpt(fx - 24.0, my + 12.0), hpt(fx, my - 4.0), hpt(fx + 24.0, my + 12.0), lw(8.0), ink);
            }
            Mood::Tired | Mood::Dizzy => {
                p.stroke_polyline(&[hpt(fx - 18.0, my + 6.0), hpt(fx + 18.0, my + 6.0)], lw(8.0), ink);
            }
            Mood::Determined => {
                p.stroke_polyline(&[hpt(fx - 24.0, my + 5.0), hpt(fx + 24.0, my + 1.0)], lw(8.0), ink);
            }
            Mood::Embarrassed => {
                let mut pts = flatten_quad(hpt(fx - 17.0, my + 4.0), hpt(fx - 8.0, my - 2.0), hpt(fx, my + 4.0), 12);
                let q2 = flatten_quad(hpt(fx, my + 4.0), hpt(fx + 8.0, my + 10.0), hpt(fx + 17.0, my + 4.0), 12);
                pts.extend_from_slice(&q2[1..]);
                p.stroke_polyline(&pts, lw(8.0), ink);
            }
            Mood::Awe => {
                open_mouth(&mut p, 11.0, 13.0, false);
            }
            Mood::Suspicious => {
                p.stroke_quad(hpt(fx - 22.0, my), hpt(fx, my + 2.0), hpt(fx + 24.0, my + 9.0), lw(8.0), ink);
            }
            Mood::Relieved => {
                p.stroke_quad(hpt(fx - 20.0, my), hpt(fx, my + 18.0), hpt(fx + 20.0, my), lw(8.0), ink);
                let (ox, oy) = hpt(fx, my + 16.0);
                p.fill_ellipse(ox, oy, 5.0 * RES, 3.0 * RES, head_rot, rgba("#3A1A2A"));
            }
            _ => {
                p.stroke_quad(hpt(fx - 20.0, my + 2.0), hpt(fx, my + 14.0), hpt(fx + 20.0, my + 2.0), lw(8.0), ink);
            }
        }
    }
    if look.mustache {
        let mc = look.mustache_col.as_deref().unwrap_or("#3A2A22");
        let mut pts: Vec<(f32, f32)> = Vec::new();
        let mut chain = |qs: Vec<(f32, f32)>| {
            if pts.is_empty() {
                pts.extend_from_slice(&qs);
            } else {
                pts.extend_from_slice(&qs[1..]);
            }
        };
        chain(flatten_cubic(hpt(fx, my - 18.0), hpt(fx - 20.0, my - 32.0), hpt(fx - 60.0, my - 22.0), hpt(fx - 64.0, my - 2.0), 20));
        chain(flatten_cubic(hpt(fx - 64.0, my - 2.0), hpt(fx - 44.0, my - 8.0), hpt(fx - 20.0, my - 4.0), hpt(fx, my - 10.0), 20));
        chain(flatten_cubic(hpt(fx, my - 10.0), hpt(fx + 20.0, my - 4.0), hpt(fx + 44.0, my - 8.0), hpt(fx + 64.0, my - 2.0), 20));
        chain(flatten_cubic(hpt(fx + 64.0, my - 2.0), hpt(fx + 60.0, my - 22.0), hpt(fx + 20.0, my - 32.0), hpt(fx, my - 18.0), 20));
        p.fill_polygon(&pts, rgba(mc));
        let mut ring = pts.clone();
        ring.push(pts[0]);
        p.stroke_polyline(&ring, lw(4.0), ink);
    }
    if st.sweat || m == Mood::Worried {
        // Sweat drop at (fx + 104, hy - 70): two quads, fill +
        // stroke (alpha 1 in this film: S.sweat unset, worried
        // gives k = 1).
        let (sx, sy) = (fx + 104.0, hy - 70.0);
        let mut pts = flatten_quad(hpt(sx, sy - 26.0), hpt(sx + 18.0, sy), hpt(sx, sy + 12.0), 16);
        let q2 = flatten_quad(hpt(sx, sy + 12.0), hpt(sx - 18.0, sy), hpt(sx, sy - 26.0), 16);
        pts.extend_from_slice(&q2[1..]);
        p.fill_polygon(&pts, rgba("#8FD3F4"));
        let mut ring = pts.clone();
        ring.push(pts[0]);
        p.stroke_polyline(&ring, lw(4.0), ink);
    }
    if st.tears || m == Mood::Sad {
        for s in [-1.0f32, 1.0] {
            let (txp, typ) = hpt(fx + s * (ex + 6.0), ey + 40.0);
            p.fill_ellipse(txp, typ, 7.0 * RES, 14.0 * RES, head_rot, rgba("#8FD3F4"));
        }
    }

    // ---- hat (centre lags turn*18) ----
    let hatx = hx + turn * 18.0;
    let htop = hy - rp * 0.97;
    match look.hat.as_str() {
        "cap" => {
            let cap_hex = look.hat_col.as_deref().unwrap_or("#7A5236");
            let cap = rgba(cap_hex);
            {
                let (ccx, ccy) = hpt(hatx + 6.0, htop + 34.0);
                let (rx, ry, rot) = (rp * 0.98 * (1.0 - 0.3 * sq) * RES, 62.0 * (1.0 + sq) * RES, -0.05 + head_rot);
                let (s3, c3) = rot.sin_cos();
                let mut pts: Vec<(f32, f32)> = Vec::new();
                for i in 0..=48 {
                    let a = std::f32::consts::PI + (i as f32 / 48.0) * std::f32::consts::PI;
                    let (lx2, ly2) = (rx * a.cos(), ry * a.sin());
                    pts.push((ccx + lx2 * c3 - ly2 * s3, ccy + lx2 * s3 + ly2 * c3));
                }
                p.fill_polygon(&pts, cap);
                let mut ring = pts.clone();
                ring.push(pts[0]);
                p.stroke_polyline(&ring, lw(9.0), ink);
            }
            {
                let (bcx, bcy) = hpt(hatx - 24.0, htop + 40.0);
                let (rx, ry, rot) = (rp * 0.78 * (1.0 - 0.3 * sq) * RES, 22.0 * (1.0 + sq) * RES, 0.05 + head_rot);
                let (s3, c3) = rot.sin_cos();
                let mut pts: Vec<(f32, f32)> = Vec::new();
                for i in 0..=48 {
                    let a = (i as f32 / 48.0) * std::f32::consts::PI;
                    let (lx2, ly2) = (rx * a.cos(), ry * a.sin());
                    pts.push((bcx + lx2 * c3 - ly2 * s3, bcy + lx2 * s3 + ly2 * c3));
                }
                p.fill_polygon(&pts, shade_rgba(cap_hex, 0.75));
                p.stroke_polyline(&pts, lw(9.0), ink);
            }
            {
                let (bcx, bcy) = hpt(hatx + 6.0, htop - 26.0);
                p.fill_ellipse(bcx, bcy, 12.0 * RES, 12.0 * RES, 0.0, shade_rgba(cap_hex, 1.15));
                p.stroke_ellipse(bcx, bcy, 12.0 * RES, 12.0 * RES, 0.0, lw(9.0), ink);
            }
        }
        "bowler" => {
            let c_hex = look.hat_col.as_deref().unwrap_or("#7A5236");
            let c = rgba(c_hex);
            // Brim: full ellipse (r*1.02, 22) at top + 26.
            let (bcx, bcy) = hpt(hatx, htop + 26.0);
            p.fill_ellipse(bcx, bcy, rp * 1.02 * (1.0 - 0.3 * sq) * RES, 22.0 * (1.0 + sq) * RES, head_rot, c);
            p.stroke_ellipse(bcx, bcy, rp * 1.02 * (1.0 - 0.3 * sq) * RES, 22.0 * (1.0 + sq) * RES, head_rot, lw(9.0), ink);
            // Dome: upper half ellipse (r*0.66, 80) at top + 14, closed.
            let (dcx, dcy) = hpt(hatx, htop + 14.0);
            let (rx, ry) = (rp * 0.66 * (1.0 - 0.3 * sq) * RES, 80.0 * (1.0 + sq) * RES);
            let (s3, c3) = head_rot.sin_cos();
            let mut pts: Vec<(f32, f32)> = Vec::new();
            for i in 0..=48 {
                let a = std::f32::consts::PI + (i as f32 / 48.0) * std::f32::consts::PI;
                let (lx2, ly2) = (rx * a.cos(), ry * a.sin());
                pts.push((dcx + lx2 * c3 - ly2 * s3, dcy + lx2 * s3 + ly2 * c3));
            }
            p.fill_polygon(&pts, c);
            let mut ring = pts.clone();
            ring.push(pts[0]);
            p.stroke_polyline(&ring, lw(9.0), ink);
            // Band: rect (hx - w, top + 4 - 12, 2w, 24), w = r*0.64,
            // colour = band || shadeOf(c, 0.6), fill + stroke.
            let band_col = look.band.as_deref().map(|b| rgba(b)).unwrap_or_else(|| shade_rgba(c_hex, 0.6));
            let w = rp * 0.64;
            let rect = [
                hpt(hatx - w, htop + 4.0 - 12.0),
                hpt(hatx + w, htop + 4.0 - 12.0),
                hpt(hatx + w, htop + 4.0 + 12.0),
                hpt(hatx - w, htop + 4.0 + 12.0),
            ];
            p.fill_polygon(&rect, band_col);
            let mut ring = rect.to_vec();
            ring.push(rect[0]);
            p.stroke_polyline(&ring, lw(9.0), ink);
        }
        _ => {}
    }

    (p.px, cw as u32, ch as u32, hands.0, hands.1)
}
