// gpupaint.wgsl — GPU executor for recorded CPU-Painter display lists
// (GPU-first Phase 0, seat GPUX). Embedded by src/gpupaint.rs via
// include_str!.
//
// NORMATIVE MATH SOURCE: docs/integration/GPUFIRST-INVA-RASTER-SPEC.md
// (INV-A). Every function below is a textual port of the scene.rs /
// toonfull.rs code the spec cites; where the spec flags a trap, the
// port follows the spec (round half-AWAY-from-zero implemented
// explicitly — WGSL round() is half-to-even; ellipse_mask uses the
// implicit quadratic, NOT ellipse_sd; cov_area0 supersamples ONLY
// |d0| < 1 strict).
//
// ONE canvas: a storage buffer of u32 per pixel, RGBA8 packed
// (R in the low byte). Every primitive is ONE compute dispatch over
// the primitive's CPU bounding box (workgroup 8x8); dispatches run
// in list order, so the blend order is the CPU's. Each invocation
// unpacks its pixel, blends in f32 exactly as Painter::blend /
// blend_cov do (including the per-primitive round-to-u8), and
// repacks. No atomics, no cross-pixel reads except the read-only
// plate data and mask buffers written by earlier dispatches —
// per-pixel independent, hence deterministic.
//
// MASKS ARE BBOX-SCOPED (INV-B scoping rule): a mask buffer holds
// ONLY its own geometry bbox; P.mk = (origin_x, origin_y, w, h) in
// canvas coords. Mask writers index locally; masked consumers
// translate global -> mask-local and treat out-of-bounds as false
// (the CPU mask is false outside its prim's bbox too).

struct Params {
    canvas_w: u32,
    canvas_h: u32,
    bx0: i32, // dispatch bbox origin: pixel coords of invocation (0,0)
    by0: i32,
    bx1: i32, // dispatch bbox max, inclusive (Painter loops are ..=)
    by1: i32,
    col: vec4<u32>, // source RGBA, 0..255 (blend entries)
    p0: vec4<f32>,  // per-prim slots, see gpupaint.rs prep code
    p1: vec4<f32>,
    p2: vec4<f32>,
    p3: vec4<f32>,
    u0: vec4<u32>,
    mk: vec4<i32>, // mask scope: origin (x, y), size (w, h)
}

@group(0) @binding(0) var<storage, read_write> canvas: array<u32>;
@group(0) @binding(1) var<uniform> P: Params;
@group(0) @binding(2) var<storage, read> pts: array<f32>;
@group(0) @binding(3) var<storage, read_write> mask: array<u32>;
@group(0) @binding(4) var<storage, read> plate: array<u32>;

const F32_MAX: f32 = 0x1.fffffep127;
const PI_F: f32 = 3.14159265358979323846;
const FRAC_PI_2_F: f32 = 1.57079632679489661923;
const OFFS: array<f32, 4> = array<f32, 4>(-0.375, -0.125, 0.125, 0.375);

// ------------------------------------------------------------------
// Blend: scene.rs Painter::blend / blend_cov, verbatim arithmetic
// (INV-A spec §2). Rust f32::round is half-AWAY-from-zero; WGSL
// round() is half-to-EVEN — never used here.
// ------------------------------------------------------------------

// Round half away from zero, sign-aware. Every quantization site
// in the painter feeds it non-negative values, where this reduces
// to floor(v + 0.5); the sign-aware form is kept so no future
// signed call site can silently change meaning.
fn round_away(x: f32) -> f32 {
    return sign(x) * floor(abs(x) + 0.5);
}

fn pack4(v: vec4<u32>) -> u32 {
    return v.x | (v.y << 8u) | (v.z << 16u) | (v.w << 24u);
}

fn unpack4(px: u32) -> vec4<u32> {
    return vec4<u32>(px & 0xffu, (px >> 8u) & 0xffu, (px >> 16u) & 0xffu, (px >> 24u) & 0xffu);
}

fn blend_ch(s: u32, d: u32, sa: f32, da: f32, oa: f32) -> u32 {
    let sf = f32(s) / 255.0;
    let df = f32(d) / 255.0;
    // scene.rs: (((s*sa + d*da*(1.0-sa)) / oa) * 255.0).round() as u8
    return u32(round_away(((sf * sa + df * da * (1.0 - sa)) / oa) * 255.0));
}

fn blend_at(idx: u32, src: vec4<u32>) {
    let sa = f32(src.w) / 255.0;
    if (sa <= 0.0) {
        return;
    }
    if (sa >= 1.0) {
        canvas[idx] = pack4(src);
        return;
    }
    let dst = unpack4(canvas[idx]);
    let da = f32(dst.w) / 255.0;
    let oa = sa + da * (1.0 - sa);
    if (oa <= 0.0) {
        return;
    }
    var res4: vec4<u32>;
    res4.x = blend_ch(src.x, dst.x, sa, da, oa);
    res4.y = blend_ch(src.y, dst.y, sa, da, oa);
    res4.z = blend_ch(src.z, dst.z, sa, da, oa);
    res4.w = u32(round_away(oa * 255.0));
    canvas[idx] = pack4(res4);
}

fn blend_cov_at(idx: u32, src: vec4<u32>, cov: f32) {
    if (cov <= 0.0) {
        return;
    }
    if (cov >= 1.0) {
        blend_at(idx, src);
        return;
    }
    // scene.rs blend_cov: a = (src.a as f32 * cov).round().clamp(0,255) as u8
    let a = u32(clamp(round_away(f32(src.w) * cov), 0.0, 255.0));
    if (a > 0u) {
        blend_at(idx, vec4<u32>(src.x, src.y, src.z, a));
    }
}

fn cov_of(sd: f32) -> f32 {
    return clamp(0.5 - sd, 0.0, 1.0);
}

// Rust f32::hypot is NOT sqrt(x*x + y*y) by contract; the CPU code
// calls .hypot() (seg_dist_t, fan_sd, fill_rect's box SDF, head
// shade form 1). This is the closest WGSL formulation — flagged
// in GPUFIRST-GPUX.md (INV-A spec §2.3: most pervasive risk).
fn hyp2(x: f32, y: f32) -> f32 {
    return sqrt(x * x + y * y);
}

// Bbox-scoped mask read: global pixel -> mask-local; outside the
// mask's own bbox the CPU mask is false.
fn mask_at(x: i32, y: i32) -> bool {
    let lx = x - P.mk.x;
    let ly = y - P.mk.y;
    if (lx < 0 || ly < 0 || lx >= P.mk.z || ly >= P.mk.w) {
        return false;
    }
    return mask[u32(ly) * u32(P.mk.z) + u32(lx)] != 0u;
}

// ------------------------------------------------------------------
// Shared geometry (textual ports; (sin, cos) of any rotation arrive
// as host-computed uniforms, never from in-shader transcendentals).
// ------------------------------------------------------------------

// scene.rs seg_dist_t -> (distance, t)
fn seg_dt(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> vec2<f32> {
    let dx = bx - ax;
    let dy = by - ay;
    let len2 = dx * dx + dy * dy;
    var t = 0.0;
    if (len2 > 0.0) {
        t = clamp(((px - ax) * dx + (py - ay) * dy) / len2, 0.0, 1.0);
    }
    return vec2<f32>(hyp2(px - (ax + dx * t), py - (ay + dy * t)), t);
}

fn pt(i: u32) -> vec2<f32> {
    return vec2<f32>(pts[i * 2u], pts[i * 2u + 1u]);
}

// scene.rs point_in_polygon (even-odd), verbatim op order.
fn pip(qx: f32, qy: f32, n: u32) -> bool {
    var inside = false;
    for (var i = 0u; i < n; i++) {
        let a = pt(i);
        let b = pt((i + 1u) % n);
        if ((a.y > qy) != (b.y > qy)) {
            let xint = a.x + (qy - a.y) / (b.y - a.y) * (b.x - a.x);
            if (qx < xint) {
                inside = !inside;
            }
        }
    }
    return inside;
}

// scene.rs ellipse_sd, parameterised by ellipse; the rotation's
// (sin, cos) come from P.p1.xy, computed on the HOST in Rust
// (f32::sin_cos) — INV-A spec §1.5 sanctions the hoist, and
// forbids trusting a GPU sin/cos pair to equal Rust libm's.
fn ellipse_sd_p(cx: f32, cy: f32, rx: f32, ry: f32, qx: f32, qy: f32) -> f32 {
    let dx = qx - cx;
    let dy = qy - cy;
    let s = P.p1.x;
    let c = P.p1.y;
    let lx = dx * c + dy * s;
    let ly = -dx * s + dy * c;
    let gx = lx / rx;
    let gy = ly / ry;
    let g = sqrt(gx * gx + gy * gy);
    if (g == 0.0) {
        return -min(rx, ry);
    }
    let ax = lx / (rx * rx);
    let ay = ly / (ry * ry);
    let grad = sqrt(ax * ax + ay * ay) / g;
    return (g - 1.0) / grad;
}

// The cov_area0 mechanism (INV-A spec §1.3): supersample ONLY
// |d0| < 1 (strict), 4x4 binary inside tests at +-0.125/+-0.375;
// outside the band cov_of(d0), which is then saturated 0/1. Each
// entry point instantiates it over its own distance function.

// ------------------------------------------------------------------
// fill_rect — scene.rs Painter::fill_rect box SDF.
// P.p0 = (cx, cy, hw, hh) of the rect.
// ------------------------------------------------------------------

fn rect_sd(qx: f32, qy: f32) -> f32 {
    let dx = abs(qx - P.p0.x) - P.p0.z;
    let dy = abs(qy - P.p0.y) - P.p0.w;
    return hyp2(max(dx, 0.0), max(dy, 0.0)) + min(max(dx, dy), 0.0);
}

@compute @workgroup_size(8, 8)
fn main_rect(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = i32(gid.x) + P.bx0;
    let y = i32(gid.y) + P.by0;
    if (x > P.bx1 || y > P.by1) {
        return;
    }
    let qx = f32(x) + 0.5;
    let qy = f32(y) + 0.5;
    let d0 = rect_sd(qx, qy);
    var cov = cov_of(d0);
    if (abs(d0) < 1.0) {
        var inside = 0u;
        for (var j = 0; j < 4; j++) {
            for (var i = 0; i < 4; i++) {
                if (rect_sd(qx + OFFS[i], qy + OFFS[j]) < 0.0) {
                    inside += 1u;
                }
            }
        }
        cov = f32(inside) / 16.0;
    }
    if (cov > 0.0) {
        blend_cov_at(u32(y) * P.canvas_w + u32(x), P.col, cov);
    }
}

// ------------------------------------------------------------------
// fill_ellipse / fill_ellipse_masked — scene.rs ellipse_sd.
// P.p0 = (cx, cy, rx, ry); P.p1 = (sin(rot), cos(rot), 0, 0).
// P.u0.w bit 0: honour the (bbox-scoped) mask (fill_ellipse_masked).
// ------------------------------------------------------------------

fn ellipse_sd(qx: f32, qy: f32) -> f32 {
    return ellipse_sd_p(P.p0.x, P.p0.y, P.p0.z, P.p0.w, qx, qy);
}

@compute @workgroup_size(8, 8)
fn main_ellipse(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = i32(gid.x) + P.bx0;
    let y = i32(gid.y) + P.by0;
    if (x > P.bx1 || y > P.by1) {
        return;
    }
    let idx = u32(y) * P.canvas_w + u32(x);
    if ((P.u0.w & 1u) != 0u && !mask_at(x, y)) {
        return;
    }
    let qx = f32(x) + 0.5;
    let qy = f32(y) + 0.5;
    let d0 = ellipse_sd(qx, qy);
    var cov = cov_of(d0);
    if (abs(d0) < 1.0) {
        var inside = 0u;
        for (var j = 0; j < 4; j++) {
            for (var i = 0; i < 4; i++) {
                if (ellipse_sd(qx + OFFS[i], qy + OFFS[j]) < 0.0) {
                    inside += 1u;
                }
            }
        }
        cov = f32(inside) / 16.0;
    }
    if (cov > 0.0) {
        blend_cov_at(idx, P.col, cov);
    }
}

// ------------------------------------------------------------------
// stroke_polyline — scene.rs Painter::stroke_polyline, two-pass
// semantics in one dispatch (INV-A spec §6.1 item 2 sanctions the
// per-pixel fusion: pass 1's dmin is used only at the pixel's own
// index as a skip/branch oracle; the fan-aware closure is
// recomputed whenever it matters). Pass 1's per-segment stamp
// boxes (hw+2, clamped to the union bbox) cannot change the min
// at any pixel that survives the d0 >= 1.0 rejection — see
// GPUFIRST-GPUX.md.
// P.p0 = (hw, 0, 0, 0)
// P.p1 = (fan_start.c, fan_start.axis)
// P.p2 = (fan_start.r, fan_start.cos_half, fan_end.c)
// P.p3 = (fan_end.axis, fan_end.r, fan_end.cos_half)
// P.u0 = (n_pts, fan_start.n, fan_end.n, flags: bit0 has fan_start,
//         bit1 has fan_end). Fan n / cos_half are computed on the
//         HOST in Rust (cap_fan's formula, spec §1.8); fan_sd
//         itself is per-sample and runs here — its acos/cos are
//         the flagged highest-risk spot (spec §8).
// ------------------------------------------------------------------

fn fan_sd(which: u32, qx: f32, qy: f32) -> f32 {
    var c: vec2<f32>;
    var axis: vec2<f32>;
    var r: f32;
    var cos_half: f32;
    var n: u32;
    if (which == 0u) {
        c = P.p1.xy;
        axis = P.p1.zw;
        r = P.p2.x;
        cos_half = P.p2.y;
        n = P.u0.y;
    } else {
        c = P.p2.zw;
        axis = P.p3.xy;
        r = P.p3.z;
        cos_half = P.p3.w;
        n = P.u0.z;
    }
    let vx = qx - c.x;
    let vy = qy - c.y;
    let rl = hyp2(vx, vy);
    if (rl < 1e-6) {
        return -r * cos_half;
    }
    let cosang = clamp((vx * axis.x + vy * axis.y) / rl, -1.0, 1.0);
    if (cosang < 0.0) {
        return rl - r;
    }
    // acos refinement (GPUFIRST Batch 2, numerics seat): WGSL acos
    // is implementation-precision — on lavapipe it lands up to
    // ~851 ulp from Rust libm's acos for the same input bits
    // (measured: cosang 0x3f72efc3 -> phi 0x3ea441c4 vs libm
    // 0x3ea44517), and that error moves rb by ~55 ulp, enough to
    // flip a supersample inside test whose CPU sd is +4.8e-6
    // (toon prim 24 end cap, pixel (1360,1009): CPU cov 6/16,
    // unrefined GPU 7/16). One Newton step on cos(phi) = cosang,
    // using the driver's cos/sin (measured bit-exact vs libm at
    // the operating point), brings phi to within a few ulp of
    // libm's. Guards: near the axis (sin ~ 0) rb is insensitive
    // to phi (tan(phi - phi_mid) ~ 0), and an implausible
    // correction keeps the raw acos value — never worse.
    var phi = acos(cosang);
    let s0 = sin(phi);
    if (s0 > 0.02) {
        let corr = (cos(phi) - cosang) / s0;
        if (abs(corr) < 0.002) {
            phi = phi + corr;
        }
    }
    let dth = PI_F / f32(n);
    var k = i32(floor((FRAC_PI_2_F - phi) / dth));
    k = clamp(k, 0, i32(n) - 1);
    let phi_mid = FRAC_PI_2_F - (f32(k) + 0.5) * dth;
    let rb = r * cos_half / cos(phi - phi_mid);
    return rl - rb;
}

fn stroke_sd(qx: f32, qy: f32) -> f32 {
    let hw = P.p0.x;
    let nseg = P.u0.x - 1u;
    var sd = F32_MAX;
    for (var i = 0u; i < nseg; i++) {
        let a = pt(i);
        let b = pt(i + 1u);
        let dt = seg_dt(qx, qy, a.x, a.y, b.x, b.y);
        var s = dt.x - hw;
        if (i == 0u && dt.y == 0.0) {
            if ((P.u0.w & 1u) != 0u) {
                s = fan_sd(0u, qx, qy);
            }
        } else if (i == nseg - 1u && dt.y == 1.0) {
            if ((P.u0.w & 2u) != 0u) {
                s = fan_sd(1u, qx, qy);
            }
        }
        sd = min(sd, s);
    }
    return sd;
}

@compute @workgroup_size(8, 8)
fn main_stroke(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = i32(gid.x) + P.bx0;
    let y = i32(gid.y) + P.by0;
    if (x > P.bx1 || y > P.by1) {
        return;
    }
    let qx = f32(x) + 0.5;
    let qy = f32(y) + 0.5;
    let nseg = P.u0.x - 1u;
    var dmin = F32_MAX;
    for (var i = 0u; i < nseg; i++) {
        let a = pt(i);
        let b = pt(i + 1u);
        dmin = min(dmin, seg_dt(qx, qy, a.x, a.y, b.x, b.y).x);
    }
    var d0 = dmin - P.p0.x;
    if (d0 >= 1.0) {
        return;
    }
    // Fan-aware recompute iff d0 > -1.5 (spec §3.3, exact rule).
    if (d0 > -1.5) {
        d0 = stroke_sd(qx, qy);
    }
    var cov = cov_of(d0);
    if (abs(d0) < 1.0) {
        var inside = 0u;
        for (var j = 0; j < 4; j++) {
            for (var i = 0; i < 4; i++) {
                if (stroke_sd(qx + OFFS[i], qy + OFFS[j]) < 0.0) {
                    inside += 1u;
                }
            }
        }
        cov = f32(inside) / 16.0;
    }
    if (cov > 0.0) {
        blend_cov_at(u32(y) * P.canvas_w + u32(x), P.col, cov);
    }
}

// ------------------------------------------------------------------
// fill_polygon — scene.rs Painter::fill_polygon (even-odd, signed
// distance to the outline, sign from the centre inside test).
// P.u0.x = n_pts.
// ------------------------------------------------------------------

fn poly_sd(qx: f32, qy: f32, n: u32) -> f32 {
    var dmin = F32_MAX;
    for (var i = 0u; i < n; i++) {
        let a = pt(i);
        let b = pt((i + 1u) % n);
        dmin = min(dmin, seg_dt(qx, qy, a.x, a.y, b.x, b.y).x);
    }
    if (pip(qx, qy, n)) {
        return -dmin;
    }
    return dmin;
}

@compute @workgroup_size(8, 8)
fn main_fillpoly(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = i32(gid.x) + P.bx0;
    let y = i32(gid.y) + P.by0;
    if (x > P.bx1 || y > P.by1) {
        return;
    }
    let n = P.u0.x;
    let qx = f32(x) + 0.5;
    let qy = f32(y) + 0.5;
    let sd0 = poly_sd(qx, qy, n);
    if (sd0 > 2.6) {
        return;
    }
    var cov = cov_of(sd0);
    if (abs(sd0) < 1.0) {
        var inside = 0u;
        for (var j = 0; j < 4; j++) {
            for (var i = 0; i < 4; i++) {
                if (poly_sd(qx + OFFS[i], qy + OFFS[j], n) < 0.0) {
                    inside += 1u;
                }
            }
        }
        cov = f32(inside) / 16.0;
    }
    if (cov > 0.0) {
        blend_cov_at(u32(y) * P.canvas_w + u32(x), P.col, cov);
    }
}

// ------------------------------------------------------------------
// Mask writers — scene.rs Painter::polygon_mask / ellipse_mask:
// centre-sampled boolean tests, NO bbox padding, written into the
// bbox-scoped mask buffer at LOCAL indices (dispatch origin is the
// mask bbox origin; the buffer starts zeroed = CPU's vec![false]).
// ellipse_mask uses the implicit quadratic (spec §4.2 / D3),
// NOT ellipse_sd.
// ------------------------------------------------------------------

@compute @workgroup_size(8, 8)
fn main_mask_poly(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = i32(gid.x) + P.bx0;
    let y = i32(gid.y) + P.by0;
    if (x > P.bx1 || y > P.by1) {
        return;
    }
    if (pip(f32(x) + 0.5, f32(y) + 0.5, P.u0.x)) {
        mask[gid.y * u32(P.mk.z) + gid.x] = 1u;
    }
}

@compute @workgroup_size(8, 8)
fn main_mask_ellipse(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = i32(gid.x) + P.bx0;
    let y = i32(gid.y) + P.by0;
    if (x > P.bx1 || y > P.by1) {
        return;
    }
    let dx = f32(x) + 0.5 - P.p0.x;
    let dy = f32(y) + 0.5 - P.p0.y;
    let s = P.p1.x;
    let c = P.p1.y;
    let lx = dx * c + dy * s;
    let ly = -dx * s + dy * c;
    let ex = lx / P.p0.z;
    let ey = ly / P.p0.w;
    if (ex * ex + ey * ey <= 1.0) {
        mask[gid.y * u32(P.mk.z) + gid.x] = 1u;
    }
}

// ------------------------------------------------------------------
// fill_polygon_masked — toonfull.rs fill_polygon_masked: FULL
// blend (not blend_cov) where mask && own-centre-test, over the
// unpadded polygon bbox (spec §4.4). The own test is the same
// centre pip the CPU's internal polygon_mask computes.
// ------------------------------------------------------------------

@compute @workgroup_size(8, 8)
fn main_fillpoly_masked(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = i32(gid.x) + P.bx0;
    let y = i32(gid.y) + P.by0;
    if (x > P.bx1 || y > P.by1) {
        return;
    }
    if (!mask_at(x, y)) {
        return;
    }
    if (!pip(f32(x) + 0.5, f32(y) + 0.5, P.u0.x)) {
        return;
    }
    blend_at(u32(y) * P.canvas_w + u32(x), P.col);
}

// ------------------------------------------------------------------
// masked_blend — the laugh-teeth pattern (toonfull.rs, spec §4.4
// call-site variant): per pixel in the recorded integer rect, if
// the (bbox-scoped) mask is set, blend at full coverage. No
// coverage, no own-shape test.
// ------------------------------------------------------------------

@compute @workgroup_size(8, 8)
fn main_masked_blend(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = i32(gid.x) + P.bx0;
    let y = i32(gid.y) + P.by0;
    if (x > P.bx1 || y > P.by1) {
        return;
    }
    if (!mask_at(x, y)) {
        return;
    }
    blend_at(u32(y) * P.canvas_w + u32(x), P.col);
}

// ------------------------------------------------------------------
// head_shade — the painters' open-coded even-odd head shade
// (toonfull.rs:995-1018 and its paint_toon twin scene.rs:1131-1138;
// spec §5.1 / D7). The ONLY direct cov_of ramp use: centre samples
// only, NO cov_area supersample.
//   cov = cov_of(ellipse_sd(head)) * unshaded; blend_cov.
// P.p0 = (head_c, head_r); P.p2 = (circ_c, circ_r);
// P.p1 = (sin(rot), cos(rot), 0, 0) host-computed.
// P.u0.x = form: 0 -> unshaded = clamp(0.5 - ellipse_sd(circ))
//                1 -> unshaded = clamp(hypot(q - circ_c) - circ_r.x + 0.5)
// The cov_head <= 0 early-out is the CPU's own `continue`.
// ------------------------------------------------------------------

@compute @workgroup_size(8, 8)
fn main_headshade(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = i32(gid.x) + P.bx0;
    let y = i32(gid.y) + P.by0;
    if (x > P.bx1 || y > P.by1) {
        return;
    }
    let qx = f32(x) + 0.5;
    let qy = f32(y) + 0.5;
    let cov_head = cov_of(ellipse_sd_p(P.p0.x, P.p0.y, P.p0.z, P.p0.w, qx, qy));
    if (cov_head <= 0.0) {
        return;
    }
    var unshaded: f32;
    if (P.u0.x == 0u) {
        let d = ellipse_sd_p(P.p2.x, P.p2.y, P.p2.z, P.p2.w, qx, qy);
        unshaded = clamp(0.5 - d, 0.0, 1.0);
    } else {
        unshaded = clamp(hyp2(qx - P.p2.x, qy - P.p2.y) - P.p2.z + 0.5, 0.0, 1.0);
    }
    let cov = cov_head * unshaded;
    if (cov > 0.0) {
        blend_cov_at(u32(y) * P.canvas_w + u32(x), P.col, cov);
    }
}

// ------------------------------------------------------------------
// composite_plate — toonfull.rs composite_plate (spec §5.2): per
// canvas pixel, inverse-map through the lean rotation (stage
// pivot (256, 1000), anchor, +(384, 256), x RES=2), bilinear-
// sample the plate, truncate the sampled RGB (as u8), and
// blend_cov at the sampled alpha with source alpha forced 255.
// P.p0 = (hand.x, hand.y, plate.x0, plate.y0)
// P.p1 = (sin(lean), cos(lean), anchor.x, anchor.y) — host Rust.
// P.p2 = (plate.w as f32, plate.h as f32, RES, 0)
// P.p3 = (sin(pitch), cos(pitch), hip.x, hip.y) — B3-CARRY-1.
// P.u0 = (plate.w, plate.h, flip flag, pitched flag) — B3-CARRY-1:
// the inverse map undoes anchor, then the flip mirror
// (x -> 512 - x), then lean, then the bodyPitch rotation about
// the hip pivot. Both added steps branch on their flags so a
// default (unflipped, unpitched) list computes exactly the
// pre-Batch-3 values.
// ------------------------------------------------------------------

fn plate_at(ix: u32, iy: u32) -> vec4<f32> {
    let v = unpack4(plate[iy * P.u0.x + ix]);
    return vec4<f32>(f32(v.x), f32(v.y), f32(v.z), f32(v.w));
}

@compute @workgroup_size(8, 8)
fn main_plate(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = i32(gid.x) + P.bx0;
    let y = i32(gid.y) + P.by0;
    if (x > P.bx1 || y > P.by1) {
        return;
    }
    let res = P.p2.z;
    let pw = P.p2.x;
    let ph = P.p2.y;
    let s = P.p1.x;
    let c = P.p1.y;
    var ux = f32(x) / res - 384.0 - P.p1.z;
    let uy = f32(y) / res - 256.0 - P.p1.w;
    if (P.u0.z != 0u) {
        ux = 512.0 - ux;
    }
    let dx = ux - 256.0;
    let dy = uy - 1000.0;
    var sx = 256.0 + dx * c + dy * s;
    var sy = 1000.0 - dx * s + dy * c;
    if (P.u0.w != 0u) {
        let pdx = sx - P.p3.z;
        let pdy = sy - P.p3.w;
        sx = P.p3.z + pdx * P.p3.y + pdy * P.p3.x;
        sy = P.p3.w - pdx * P.p3.x + pdy * P.p3.y;
    }
    let lx = (sx - P.p0.x - P.p0.z) * res;
    let ly = (sy - P.p0.y - P.p0.w) * res;
    if (lx < -0.5 || ly < -0.5 || lx > pw - 0.5 || ly > ph - 0.5) {
        return;
    }
    let fx = clamp(lx, 0.0, pw - 1.001);
    let fy = clamp(ly, 0.0, ph - 1.001);
    let ix = u32(fx);
    let iy = u32(fy);
    let tx = fx - f32(ix);
    let ty = fy - f32(iy);
    let s00 = plate_at(ix, iy);
    let s10 = plate_at(ix + 1u, iy);
    let s01 = plate_at(ix, iy + 1u);
    let s11 = plate_at(ix + 1u, iy + 1u);
    let smp = (s00 * (1.0 - tx) + s10 * tx) * (1.0 - ty)
        + (s01 * (1.0 - tx) + s11 * tx) * ty;
    let a = smp.w / 255.0;
    if (a > 0.0) {
        blend_cov_at(
            u32(y) * P.canvas_w + u32(x),
            vec4<u32>(u32(smp.x), u32(smp.y), u32(smp.z), 255u),
            a,
        );
    }
}
