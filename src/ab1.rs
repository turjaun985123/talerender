//! §28 A/B reproduction scene: Onoda S01 (HOOK) at t = 1.50 s from the
//! delivered project ~/faceless-shorts/the-soldier-who-didnt-know
//! ("THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER", v4-era Tall Tale).
//!
//! Every staging value in this file was extracted from the project's
//! own sources (file:line citations live in
//! docs/ab/onoda-s01-t150/README.md). The scene is re-authored with
//! TaleRender's existing M1 pipelines: coloured geometry for the 3D
//! set, CPU-painted textures for the character card, grass, clouds,
//! sky gradient and blob shadow. Where the real engine has something
//! this renderer lacks (shadow maps, the composer chain, PMREM IBL,
//! canvas-painted planks textures, icosahedron canopies), the closest
//! honest primitive stands in and the substitution is logged in
//! docs/ab/onoda-s01-t150/gaps.md — nothing is silently approximated.
//!
//! Renderer of record for this scene: wgpu on lavapipe (software
//! Vulkan) — correctness evidence only, per contract §6.2.

use crate::math::{self, Mat4};
use std::borrow::Cow;

// ---------------------------------------------------------------------
// Colour helpers. The TS painters work in sRGB hex; three.js converts
// material colours to linear working space and the output pass encodes
// back. Vertex colours here are linearised so the sRGB frame target
// returns them to (approximately) the authored values; texture pixels
// are sRGB bytes and round-trip through the sRGB texture format as-is.
// ---------------------------------------------------------------------

fn hex_rgb(s: &str) -> (u8, u8, u8) {
    let n = u32::from_str_radix(s.trim_start_matches('#'), 16).expect("bad hex colour");
    (((n >> 16) & 255) as u8, ((n >> 8) & 255) as u8, (n & 255) as u8)
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// sRGB hex -> linear [r,g,b] for vertex colours.
fn hex_lin(s: &str) -> [f32; 3] {
    let (r, g, b) = hex_rgb(s);
    [
        srgb_to_linear(r as f32 / 255.0),
        srgb_to_linear(g as f32 / 255.0),
        srgb_to_linear(b as f32 / 255.0),
    ]
}

/// Canvas shadeOf(): multiply the sRGB bytes by k (round, clamp).
fn shade_rgb(s: &str, k: f32) -> (u8, u8, u8) {
    let (r, g, b) = hex_rgb(s);
    let f = |v: u8| -> u8 { (v as f32 * k).round().clamp(0.0, 255.0) as u8 };
    (f(r), f(g), f(b))
}

fn shade_lin(s: &str, k: f32) -> [f32; 3] {
    let (r, g, b) = shade_rgb(s, k);
    [
        srgb_to_linear(r as f32 / 255.0),
        srgb_to_linear(g as f32 / 255.0),
        srgb_to_linear(b as f32 / 255.0),
    ]
}

/// The engine's deterministic sin-hash (lib.ts/world.ts rnd(i)):
/// frac(sin(i*127.1 + 311.7) * 43758.5453), evaluated in f64 exactly
/// like JS Math.sin.
fn rnd(i: f64) -> f64 {
    let x = (i * 127.1 + 311.7).sin() * 43758.5453;
    x - x.floor()
}

// ---------------------------------------------------------------------
// Geometry builders (colour pipeline vertex format: pos, normal, col).
// All shapes are built centred on the origin in local space; placement
// happens through the model matrix, like the TS builders + three.js.
// ---------------------------------------------------------------------

struct Geo {
    verts: Vec<f32>,
    idx: Vec<u32>,
}

impl Geo {
    fn new() -> Self {
        Geo { verts: Vec::new(), idx: Vec::new() }
    }
    fn vert(&mut self, p: [f32; 3], n: [f32; 3], c: [f32; 3]) -> u32 {
        self.verts.extend_from_slice(&p);
        self.verts.extend_from_slice(&n);
        self.verts.extend_from_slice(&c);
        (self.verts.len() / 9 - 1) as u32
    }
    fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.idx.extend_from_slice(&[a, b, c]);
    }
    /// The pipeline culls back faces; generated shapes are convex-ish
    /// and centred on the origin, so a triangle whose geometric normal
    /// points away from its own centroid (outward build) — or toward
    /// it (inward build, e.g. the sky dome) — gets its winding flipped.
    fn fix_winding(&mut self, outward: bool) {
        for t in self.idx.chunks_exact_mut(3) {
            let p = |k: usize| -> [f32; 3] {
                let o = (t[k] as usize) * 9;
                [self.verts[o], self.verts[o + 1], self.verts[o + 2]]
            };
            let (a, b, c) = (p(0), p(1), p(2));
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let cen = [
                (a[0] + b[0] + c[0]) / 3.0,
                (a[1] + b[1] + c[1]) / 3.0,
                (a[2] + b[2] + c[2]) / 3.0,
            ];
            let d = n[0] * cen[0] + n[1] * cen[1] + n[2] * cen[2];
            if (outward && d < 0.0) || (!outward && d > 0.0) {
                t.swap(1, 2);
            }
        }
    }
    fn append(&mut self, other: Geo) {
        let base = (self.verts.len() / 9) as u32;
        self.verts.extend_from_slice(&other.verts);
        for i in other.idx {
            self.idx.push(i + base);
        }
    }
}

/// Axis-aligned box, w x h x d, centred. Same face/winding table as the
/// verified M1 cube, generalised to per-axis half extents.
fn box_geo(w: f32, h: f32, d: f32, col: [f32; 3]) -> Geo {
    let (x, y, z) = (w / 2.0, h / 2.0, d / 2.0);
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        ([0.0, 0.0, 1.0], [[-x, -y, z], [x, -y, z], [x, y, z], [-x, y, z]]),
        ([0.0, 0.0, -1.0], [[x, -y, -z], [-x, -y, -z], [-x, y, -z], [x, y, -z]]),
        ([1.0, 0.0, 0.0], [[x, -y, z], [x, -y, -z], [x, y, -z], [x, y, z]]),
        ([-1.0, 0.0, 0.0], [[-x, -y, -z], [-x, -y, z], [-x, y, z], [-x, y, -z]]),
        ([0.0, 1.0, 0.0], [[-x, y, z], [x, y, z], [x, y, -z], [-x, y, -z]]),
        ([0.0, -1.0, 0.0], [[-x, -y, -z], [x, -y, -z], [x, -y, z], [-x, -y, z]]),
    ];
    let mut g = Geo::new();
    for (nrm, corners) in faces {
        let base = (g.verts.len() / 9) as u32;
        for c in corners {
            g.vert(c, nrm, col);
        }
        g.idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    g
}

/// Cylinder along Y, centred (three.js CylinderGeometry layout):
/// radius_top, radius_bottom, height, radial segments, with caps.
fn cyl_geo(rt: f32, rb: f32, h: f32, seg: u32, col: [f32; 3]) -> Geo {
    let mut g = Geo::new();
    let hh = h / 2.0;
    let mut ring = |y: f32, r: f32, nrm_y: f32| -> Vec<u32> {
        let mut ids = Vec::new();
        for i in 0..seg {
            let a = (i as f32 / seg as f32) * std::f32::consts::TAU;
            let (s, c) = a.sin_cos();
            let n = if nrm_y == 0.0 { [c, 0.0, s] } else { [0.0, nrm_y, 0.0] };
            ids.push(g.vert([r * c, y, r * s], n, col));
        }
        ids
    };
    let bot = ring(-hh, rb, 0.0);
    let top = ring(hh, rt, 0.0);
    for i in 0..seg {
        let j = (i + 1) % seg;
        g.tri(bot[i as usize], top[j as usize], bot[j as usize]);
        g.tri(bot[i as usize], top[i as usize], top[j as usize]);
    }
    // Caps: fans around a centre vertex.
    let cb = g.vert([0.0, -hh, 0.0], [0.0, -1.0, 0.0], col);
    let bot_cap = ring(-hh, rb, -1.0);
    for i in 0..seg {
        let j = (i + 1) % seg;
        g.tri(cb, bot_cap[j as usize], bot_cap[i as usize]);
    }
    let ct = g.vert([0.0, hh, 0.0], [0.0, 1.0, 0.0], col);
    let top_cap = ring(hh, rt, 1.0);
    for i in 0..seg {
        let j = (i + 1) % seg;
        g.tri(ct, top_cap[i as usize], top_cap[j as usize]);
    }
    g.fix_winding(true);
    g
}

/// UV sphere, radius r, centred. Normals = positions normalised.
fn sphere_geo(r: f32, ws: u32, hs: u32, col: [f32; 3]) -> Geo {
    let mut g = Geo::new();
    let mut ids: Vec<Vec<u32>> = Vec::new();
    for iy in 0..=hs {
        let th = (iy as f32 / hs as f32) * std::f32::consts::PI;
        let (st, ct) = th.sin_cos();
        let mut row = Vec::new();
        for ix in 0..=ws {
            let ph = (ix as f32 / ws as f32) * std::f32::consts::TAU;
            let (sp, cp) = ph.sin_cos();
            let p = [r * st * cp, r * ct, r * st * sp];
            row.push(g.vert(p, [st * cp, ct, st * sp], col));
        }
        ids.push(row);
    }
    for iy in 0..hs {
        for ix in 0..ws {
            let (a, b) = (ids[iy as usize][ix as usize], ids[iy as usize][ix as usize + 1]);
            let (c, d) = (ids[iy as usize + 1][ix as usize + 1], ids[iy as usize + 1][ix as usize]);
            g.tri(a, b, d);
            g.tri(b, c, d);
        }
    }
    g.fix_winding(true);
    g
}

/// Cone along Y, centred: base radius r at -h/2, apex at +h/2
/// (three.js ConeGeometry = cylinder with radius_top 0).
fn cone_geo(r: f32, h: f32, seg: u32, col: [f32; 3]) -> Geo {
    let mut g = Geo::new();
    let hh = h / 2.0;
    let slope = (r / h).atan(); // normal tilt from vertical plane
    let mut base: Vec<u32> = Vec::new();
    for i in 0..seg {
        let a = (i as f32 / seg as f32) * std::f32::consts::TAU;
        let (s, c) = a.sin_cos();
        let n = [c * slope.cos(), slope.sin(), s * slope.cos()];
        base.push(g.vert([r * c, -hh, r * s], n, col));
    }
    let apex = g.vert([0.0, hh, 0.0], [0.0, 1.0, 0.0], col);
    for i in 0..seg {
        let j = (i + 1) % seg;
        g.tri(base[i as usize], apex, base[j as usize]);
    }
    let cb = g.vert([0.0, -hh, 0.0], [0.0, -1.0, 0.0], col);
    for i in 0..seg {
        let j = (i + 1) % seg;
        g.tri(cb, base[j as usize], base[i as usize]);
    }
    g.fix_winding(true);
    g
}

/// Flat disc in the XZ plane facing +Y, centred (colour version).
fn disc_geo(r: f32, seg: u32, col: [f32; 3]) -> Geo {
    let mut g = Geo::new();
    let c = g.vert([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], col);
    let mut rim: Vec<u32> = Vec::new();
    for i in 0..seg {
        let a = (i as f32 / seg as f32) * std::f32::consts::TAU;
        let (s, co) = a.sin_cos();
        rim.push(g.vert([r * co, 0.0, r * s], [0.0, 1.0, 0.0], col));
    }
    for i in 0..seg {
        let j = (i + 1) % seg;
        g.tri(c, rim[j as usize], rim[i as usize]);
    }
    g
}

// ---------------------------------------------------------------------
// Textured geometry (pos + uv).
// ---------------------------------------------------------------------

struct TexGeo {
    verts: Vec<f32>,
    idx: Vec<u32>,
}

impl TexGeo {
    fn new() -> Self {
        TexGeo { verts: Vec::new(), idx: Vec::new() }
    }
    fn vert(&mut self, p: [f32; 3], uv: [f32; 2]) -> u32 {
        self.verts.extend_from_slice(&p);
        self.verts.extend_from_slice(&uv);
        (self.verts.len() / 5 - 1) as u32
    }
    fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.idx.extend_from_slice(&[a, b, c]);
    }
}

/// XY quad, x in [-w/2, w/2], y in [y0, y0+h]; uv v = 0 at the TOP
/// edge (matches a CPU-painted image whose row 0 is the top).
fn quad_geo(w: f32, h: f32, y0: f32) -> TexGeo {
    let mut g = TexGeo::new();
    let hw = w / 2.0;
    g.vert([-hw, y0, 0.0], [0.0, 1.0]);
    g.vert([hw, y0, 0.0], [1.0, 1.0]);
    g.vert([hw, y0 + h, 0.0], [1.0, 0.0]);
    g.vert([-hw, y0 + h, 0.0], [0.0, 0.0]);
    g.tri(0, 1, 2);
    g.tri(0, 2, 3);
    g
}

/// Flat XZ quad (for the blob shadow), centred, size w x d.
fn flat_quad_geo(w: f32, d: f32) -> TexGeo {
    let mut g = TexGeo::new();
    let (hw, hd) = (w / 2.0, d / 2.0);
    g.vert([-hw, 0.0, -hd], [0.0, 0.0]);
    g.vert([hw, 0.0, -hd], [1.0, 0.0]);
    g.vert([hw, 0.0, hd], [1.0, 1.0]);
    g.vert([-hw, 0.0, hd], [0.0, 1.0]);
    g.tri(0, 2, 1);
    g.tri(0, 3, 2);
    g
}

/// Ground disc with grass-texture UVs: uv = ((x/r)+1)/2 * repeat
/// (three.js CircleGeometry uv layout x texture repeat).
fn ground_disc_geo(r: f32, seg: u32, repeat: f32) -> TexGeo {
    let mut g = TexGeo::new();
    let uv = |x: f32, z: f32| -> [f32; 2] { [(x / r + 1.0) / 2.0 * repeat, (z / r + 1.0) / 2.0 * repeat] };
    let c = g.vert([0.0, 0.0, 0.0], uv(0.0, 0.0));
    let mut rim: Vec<u32> = Vec::new();
    for i in 0..seg {
        let a = (i as f32 / seg as f32) * std::f32::consts::TAU;
        let (s, co) = a.sin_cos();
        let (x, z) = (r * co, r * s);
        rim.push(g.vert([x, 0.0, z], uv(x, z)));
    }
    for i in 0..seg {
        let j = (i + 1) % seg;
        g.tri(c, rim[j as usize], rim[i as usize]);
    }
    g
}

/// Sphere with latitude UVs (for the sky dome). `inward` flips the
/// winding so the inside surface is the front face.
fn sky_sphere_geo(r: f32, ws: u32, hs: u32) -> TexGeo {
    let mut g = TexGeo::new();
    let mut ids: Vec<Vec<u32>> = Vec::new();
    for iy in 0..=hs {
        let th = (iy as f32 / hs as f32) * std::f32::consts::PI;
        let (st, ct) = th.sin_cos();
        let mut row = Vec::new();
        for ix in 0..=ws {
            let ph = (ix as f32 / ws as f32) * std::f32::consts::TAU;
            let (sp, cp) = ph.sin_cos();
            // v: 1 at the top pole, 0 at the bottom (gradient lookup).
            row.push(g.vert([r * st * cp, r * ct, r * st * sp], [ix as f32 / ws as f32, 1.0 - iy as f32 / hs as f32]));
        }
        ids.push(row);
    }
    for iy in 0..hs {
        for ix in 0..ws {
            let (a, b) = (ids[iy as usize][ix as usize], ids[iy as usize][ix as usize + 1]);
            let (c, d) = (ids[iy as usize + 1][ix as usize + 1], ids[iy as usize + 1][ix as usize]);
            g.tri(a, d, b);
            g.tri(b, d, c);
        }
    }
    g
}

// ---------------------------------------------------------------------
// A small CPU rasteriser standing in for the Canvas2D painter. It is
// deliberately simple: shapes are evaluated per-pixel over their
// bounding box (ellipses, capsule strokes along flattened curves,
// even-odd polygon fills). This is the component the compatibility
// contract (§2) flags as identity-critical — Skia's stroke
// rasterisation is NOT reproduced here; see gaps.md.
// ---------------------------------------------------------------------

struct Painter {
    w: usize,
    h: usize,
    px: Vec<u8>, // RGBA, straight alpha, row 0 = top
}

type Rgba = (u8, u8, u8, u8);

fn rgba_of(s: &str) -> Rgba {
    let (r, g, b) = hex_rgb(s);
    (r, g, b, 255)
}

impl Painter {
    fn new(w: usize, h: usize) -> Self {
        Painter { w, h, px: vec![0u8; w * h * 4] }
    }
    fn blend(&mut self, x: i32, y: i32, src: Rgba) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let k = ((y as usize * self.w + x as usize) * 4) as usize;
        let sa = src.3 as f32 / 255.0;
        if sa <= 0.0 {
            return;
        }
        if sa >= 1.0 {
            self.px[k..k + 4].copy_from_slice(&[src.0, src.1, src.2, src.3]);
            return;
        }
        let da = self.px[k + 3] as f32 / 255.0;
        let oa = sa + da * (1.0 - sa);
        if oa <= 0.0 {
            return;
        }
        for c in 0..3 {
            let s = src[c] as f32 / 255.0;
            let d = self.px[k + c] as f32 / 255.0;
            self.px[k + c] = (((s * sa + d * da * (1.0 - sa)) / oa) * 255.0).round() as u8;
        }
        self.px[k + 3] = (oa * 255.0).round() as u8;
    }
    fn bbox(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> (i32, i32, i32, i32) {
        (
            (x0.floor() as i32).max(0),
            (y0.floor() as i32).max(0),
            (x1.ceil() as i32).min(self.w as i32 - 1),
            (y1.ceil() as i32).min(self.h as i32 - 1),
        )
    }
    /// Filled ellipse (centre, radii, rotation rad).
    fn fill_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, col: Rgba) {
        let ex = (rx * rot.cos()).abs() + (ry * rot.sin()).abs();
        let ey = (rx * rot.sin()).abs() + (ry * rot.cos()).abs();
        let (x0, y0, x1, y1) = self.bbox(cx - ex, cy - ey, cx + ex, cy + ey);
        let (s, c) = rot.sin_cos();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let (lx, ly) = (dx * c + dy * s, -dx * s + dy * c);
                if (lx / rx).powi(2) + (ly / ry).powi(2) <= 1.0 {
                    self.blend(x, y, col);
                }
            }
        }
    }
    /// Ellipse outline as a stroked flattened perimeter.
    fn stroke_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, width: f32, col: Rgba) {
        let n = 144;
        let (s, c) = rot.sin_cos();
        let pts: Vec<(f32, f32)> = (0..=n)
            .map(|i| {
                let a = (i as f32 / n as f32) * std::f32::consts::TAU;
                let (lx, ly) = (rx * a.cos(), ry * a.sin());
                (cx + lx * c - ly * s, cy + lx * s + ly * c)
            })
            .collect();
        self.stroke_polyline(&pts, width, col);
    }
    /// Capsule stroke along a polyline (round caps and joins, like
    /// Canvas2D lineCap/lineJoin 'round').
    fn stroke_polyline(&mut self, pts: &[(f32, f32)], width: f32, col: Rgba) {
        let hw = width / 2.0;
        for seg in pts.windows(2) {
            let (ax, ay) = seg[0];
            let (bx, by) = seg[1];
            let (x0, y0, x1, y1) = self.bbox(ax.min(bx) - hw, ay.min(by) - hw, ax.max(bx) + hw, ay.max(by) + hw);
            let (dx, dy) = (bx - ax, by - ay);
            let len2 = dx * dx + dy * dy;
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                    let t = if len2 > 0.0 {
                        (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let (qx, qy) = (ax + dx * t, ay + dy * t);
                    if (px - qx).hypot(py - qy) <= hw {
                        self.blend(x, y, col);
                    }
                }
            }
        }
    }
    fn stroke_quad(&mut self, p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), width: f32, col: Rgba) {
        let pts = flatten_quad(p0, p1, p2, 24);
        self.stroke_polyline(&pts, width, col);
    }
    /// Even-odd polygon fill (flattened path supplied by the caller).
    fn fill_polygon(&mut self, pts: &[(f32, f32)], col: Rgba) {
        let (mut mnx, mut mny, mut mxx, mut mxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &(x, y) in pts {
            mnx = mnx.min(x);
            mny = mny.min(y);
            mxx = mxx.max(x);
            mxy = mxy.max(y);
        }
        let (x0, y0, x1, y1) = self.bbox(mnx, mny, mxx, mxy);
        for y in y0..=y1 {
            for x in x0..=x1 {
                if point_in_polygon(x as f32 + 0.5, y as f32 + 0.5, pts) {
                    self.blend(x, y, col);
                }
            }
        }
    }
    /// Boolean mask of a polygon (for canvas clip()).
    fn polygon_mask(&self, pts: &[(f32, f32)]) -> Vec<bool> {
        let mut m = vec![false; self.w * self.h];
        let (mut mnx, mut mny, mut mxx, mut mxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &(x, y) in pts {
            mnx = mnx.min(x);
            mny = mny.min(y);
            mxx = mxx.max(x);
            mxy = mxy.max(y);
        }
        let (x0, y0, x1, y1) = self.bbox(mnx, mny, mxx, mxy);
        for y in y0..=y1 {
            for x in x0..=x1 {
                if point_in_polygon(x as f32 + 0.5, y as f32 + 0.5, pts) {
                    m[y as usize * self.w + x as usize] = true;
                }
            }
        }
        m
    }
    fn fill_ellipse_masked(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, col: Rgba, mask: &[bool]) {
        let ex = (rx * rot.cos()).abs() + (ry * rot.sin()).abs();
        let ey = (rx * rot.sin()).abs() + (ry * rot.cos()).abs();
        let (x0, y0, x1, y1) = self.bbox(cx - ex, cy - ey, cx + ex, cy + ey);
        let (s, c) = rot.sin_cos();
        for y in y0..=y1 {
            for x in x0..=x1 {
                if !mask[y as usize * self.w + x as usize] {
                    continue;
                }
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let (lx, ly) = (dx * c + dy * s, -dx * s + dy * c);
                if (lx / rx).powi(2) + (ly / ry).powi(2) <= 1.0 {
                    self.blend(x, y, col);
                }
            }
        }
    }
    fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, col: Rgba) {
        let (x0, y0, x1, y1) = self.bbox(x, y, x + w, y + h);
        for yy in y0..=y1 {
            for xx in x0..=x1 {
                self.blend(xx, yy, col);
            }
        }
    }
}

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

fn flatten_quad(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), n: usize) -> Vec<(f32, f32)> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let (a, b) = (1.0 - t, t);
            (
                a * a * p0.0 + 2.0 * a * b * p1.0 + b * b * p2.0,
                a * a * p0.1 + 2.0 * a * b * p1.1 + b * b * p2.1,
            )
        })
        .collect()
}

fn flatten_cubic(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), n: usize) -> Vec<(f32, f32)> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            (
                u.powi(3) * p0.0 + 3.0 * u.powi(2) * t * p1.0 + 3.0 * u * t.powi(2) * p2.0 + t.powi(3) * p3.0,
                u.powi(3) * p0.1 + 3.0 * u.powi(2) * t * p1.1 + 3.0 * u * t.powi(2) * p2.1 + t.powi(3) * p3.1,
            )
        })
        .collect()
}

// ---------------------------------------------------------------------
// The Onoda card: drawToon re-implemented for exactly the state S01
// produces at t = 1.50 (see README.md for the extraction). Stage space
// is 512x1024 with the figure's feet at stage y = 1000; the canvas is
// 1280x1408 with the stage at offset (384, 256), painted at res 2.
// ---------------------------------------------------------------------

const CAM_PITCH: f64 = 0.28747; // camPitchOf(jgOnoda, camPos) at t=1.50
const HEAD_TURN: f64 = -0.025; // poseTrack sweep value at lt=1.50
const LEAN_DEG: f64 = 1.0;

fn paint_onoda() -> (Vec<u8>, u32, u32) {
    let res = 2.0f32;
    let (cw, ch) = (1280usize * 2, 1408usize * 2);
    let mut p = Painter::new(cw, ch);
    // Stage -> canvas pixels: translate by (OX, OY), scale by res,
    // after the whole-figure lean rotation about the feet (256, 1000)
    // that drawToonLayer applies (rotate(+lean deg), canvas space).
    let th = (LEAN_DEG as f32).to_radians();
    let (sth, cth) = th.sin_cos();
    let lean = |x: f32, y: f32| -> (f32, f32) {
        let (dx, dy) = (x - 256.0, y - 1000.0);
        (256.0 + dx * cth - dy * sth, 1000.0 + dx * sth + dy * cth)
    };
    let px = |x: f32, y: f32| -> (f32, f32) {
        let (lx, ly) = lean(x, y);
        ((lx + 384.0) * res, (ly + 256.0) * res)
    };
    let ink = rgba_of("#1F1C33");
    let lw = |w: f32| w * res;

    // ---- legs (stand), behind the body ----
    let (bx, top, bot, bw) = (256.0f32, 330.0f32, 600.0f32, 92.0f32);
    let hip_y = bot - 26.0;
    for (sx, side) in [(bx - 42.0, -1.0f32), (bx + 42.0, 1.0f32)] {
        let knee = (sx + side * 4.0, hip_y + 210.0);
        let foot = (sx + side * 10.0, 1000.0);
        let pts = flatten_quad(px(sx, hip_y), px(knee.0, knee.1), px(foot.0, foot.1), 20);
        // NOTE: px() applied per point bends the quad slightly under
        // the lean rotation — exactly what the canvas transform does.
        p.stroke_polyline(&pts, lw(15.0), ink);
        let (fxp, fyp) = px(foot.0 + side * 10.0, foot.1 - 4.0);
        p.fill_ellipse(fxp, fyp, 20.0 * res, 10.0 * res, th, ink);
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
    p.fill_polygon(&bean, rgba_of("#5C6644"));
    let mask = p.polygon_mask(&bean);
    // Clipped shade (look.shade '#878E6E', alpha 0.85) + highlight.
    let (shx, shy) = px(bx + bw + 40.0, (top + bot) / 2.0 + 20.0);
    let (sr, sg, sb) = hex_rgb("#878E6E");
    p.fill_ellipse_masked(shx, shy, 70.0 * res, 260.0 * res, th, (sr, sg, sb, (0.85 * 255.0) as u8), &mask);
    let (hix, hiy) = px(bx - 50.0, top + 90.0);
    p.fill_ellipse_masked(hix, hiy, 16.0 * res, 60.0 * res, 0.2 + th, (255, 255, 255, (0.25 * 255.0) as u8), &mask);
    p.stroke_polyline(&bean, lw(9.0), ink);

    // ---- arms ----
    // arm FK identical to drawToon: shoulder (bx +/- 66, top + 46),
    // upper 118, fore 112, angles in degrees from hanging-down.
    let arm = |p: &mut Painter, side: f32, a: f32, e: f32| -> (f32, f32) {
        let (sx, sy) = (bx + side * 66.0, top + 46.0);
        let a1 = (a as f64).to_radians();
        let a2 = ((a + e) as f64).to_radians();
        let ex = sx + side * (a1.sin() as f32) * 118.0;
        let ey = sy + (a1.cos() as f32) * 118.0;
        let hx = ex + side * (a2.sin() as f32) * 112.0;
        let hy = ey + (a2.cos() as f32) * 112.0;
        let pts = flatten_quad(px(sx, sy), px(ex, ey), px(hx, hy), 20);
        p.stroke_polyline(&pts, lw(14.0), ink);
        let (hxp, hyp) = px(hx, hy);
        p.fill_ellipse(hxp, hyp, 15.0 * res, 17.0 * res, th, ink);
        (hx, hy)
    };
    arm(&mut p, -1.0, 10.0, 6.0); // armL [10, 6]
    arm(&mut p, 1.0, 16.0, -6.0); // armR [16, -6]

    // ---- head ----
    let (hx, hy, r) = (256.0f32, 178.0f32, 138.0f32);
    let rp = r * (1.0 + 0.04 * CAM_PITCH as f32);
    let (hxp, hyp) = px(hx, hy);
    p.fill_ellipse(hxp, hyp, rp * res, rp * 0.97 * res, th, rgba_of("#F7F7FA"));
    // Even-odd shade: head area NOT covered by the offset circle.
    {
        let (ccx, ccy) = (hx + 52.0 + HEAD_TURN as f32 * 30.0, hy + 44.0);
        let (ccxp, ccyp) = px(ccx, ccy);
        let shade = shade_rgb("#F7F7FA", 0.9);
        let col = (shade.0, shade.1, shade.2, 255u8);
        let (x0, y0, x1, y1) = p.bbox(hxp - rp * res, hyp - rp * res, hxp + rp * res, hyp + rp * res);
        let (s2, c2) = th.sin_cos();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = (x as f32 + 0.5 - hxp, y as f32 + 0.5 - hyp);
                let (lx, ly) = (dx * c2 + dy * s2, -dx * s2 + dy * c2);
                if (lx / (rp * res)).powi(2) + (ly / (rp * 0.97 * res)).powi(2) > 1.0 {
                    continue;
                }
                let (dcx, dcy) = (x as f32 + 0.5 - ccxp, y as f32 + 0.5 - ccyp);
                if (dcx * dcx + dcy * dcy) < (rp * res).powi(2) {
                    continue; // inside the offset circle: unshaded
                }
                p.blend(x, y, col);
            }
        }
    }
    p.stroke_ellipse(hxp, hyp, rp * res, rp * 0.97 * res, th, lw(10.0), ink);

    // ---- face ('determined') ----
    let fx = hx + HEAD_TURN as f32 * 30.0;
    let ex = 46.0 * (1.0 - 0.18 * (HEAD_TURN as f32).abs());
    let ey = hy + 4.0 + CAM_PITCH as f32 * 8.0;
    // Brows: [lift -4, tilt 16]; per side s: tilt signed by s.
    for s in [-1.0f32, 1.0] {
        let (lift, tilt) = (-4.0f32, 16.0f32 * s);
        let y = ey - 44.0 - lift;
        p.stroke_quad(
            px(fx + s * (ex - 26.0), y + tilt * 0.5),
            px(fx + s * ex, y - 7.0),
            px(fx + s * (ex + 26.0), y - tilt * 0.5),
            lw(12.0),
            ink,
        );
    }
    // Eyes: pupil ellipse + heavy lid (head-colour rect + lid line).
    for s in [-1.0f32, 1.0] {
        let x = fx + s * ex;
        let (exp, eyp) = px(x, ey);
        p.fill_ellipse(exp, eyp, 13.0 * res, 17.0 * res, th, ink);
        let (rx0, ry0) = px(x - 24.0, ey - 28.0);
        p.fill_rect(rx0, ry0, 48.0 * res, 26.0 * res, rgba_of("#F7F7FA"));
        p.stroke_polyline(&[px(x - 20.0, ey - 2.0), px(x + 20.0, ey - 2.0)], lw(7.0), ink);
    }
    // Mouth: straight line, my = ey + 62.
    let my = ey + 62.0;
    p.stroke_polyline(&[px(fx - 24.0, my + 5.0), px(fx + 24.0, my + 1.0)], lw(8.0), ink);

    // ---- cap ('cap', hatCol '#4C5638'), hat centre lags turn*18 ----
    let hatx = hx + HEAD_TURN as f32 * 18.0;
    let htop = hy - rp * 0.97;
    let cap = rgba_of("#4C5638");
    // Crown: upper half of the rotated ellipse, closed by its chord.
    {
        let (ccx, ccy) = px(hatx + 6.0, htop + 34.0);
        let (rx, ry, rot) = (rp * 0.98 * res, 62.0 * res, -0.05 + th);
        let (s3, c3) = rot.sin_cos();
        let mut pts: Vec<(f32, f32)> = Vec::new();
        for i in 0..=48 {
            let a = std::f32::consts::PI + (i as f32 / 48.0) * std::f32::consts::PI;
            let (lx, ly) = (rx * a.cos(), ry * a.sin());
            pts.push((ccx + lx * c3 - ly * s3, ccy + lx * s3 + ly * c3));
        }
        p.fill_polygon(&pts, cap);
        p.stroke_polyline(&pts, lw(9.0), ink);
    }
    // Brim band: lower half ellipse, shadeOf(cap, 0.75).
    {
        let (bcx, bcy) = px(hatx - 24.0, htop + 40.0);
        let (rx, ry, rot) = (rp * 0.78 * res, 22.0 * res, 0.05 + th);
        let (s3, c3) = rot.sin_cos();
        let mut pts: Vec<(f32, f32)> = Vec::new();
        for i in 0..=48 {
            let a = (i as f32 / 48.0) * std::f32::consts::PI;
            let (lx, ly) = (rx * a.cos(), ry * a.sin());
            pts.push((bcx + lx * c3 - ly * s3, bcy + lx * s3 + ly * c3));
        }
        let sh = shade_rgb("#4C5638", 0.75);
        p.fill_polygon(&pts, (sh.0, sh.1, sh.2, 255));
        p.stroke_polyline(&pts, lw(9.0), ink);
    }
    // Button: shadeOf(cap, 1.15).
    {
        let (bcx, bcy) = px(hatx + 6.0, htop - 26.0);
        let sh = shade_rgb("#4C5638", 1.15);
        p.fill_ellipse(bcx, bcy, 12.0 * res, 12.0 * res, 0.0, (sh.0, sh.1, sh.2, 255));
        p.stroke_ellipse(bcx, bcy, 12.0 * res, 12.0 * res, 0.0, lw(9.0), ink);
    }

    (p.px, cw as u32, ch as u32)
}

// ---------------------------------------------------------------------
// Painted textures for the set (world.ts T.grass / sky / cloudTex /
// blob shadow), re-implemented from their recipes.
// ---------------------------------------------------------------------

/// T.grass('#5B9A48', [48, 48]): base fill + 5000 hash-placed strokes.
fn paint_grass() -> (Vec<u8>, u32, u32) {
    let (w, h) = (512usize, 512usize);
    let mut p = Painter::new(w, h);
    p.fill_rect(0.0, 0.0, w as f32, h as f32, rgba_of("#5B9A48"));
    for i in 0..5000 {
        let fi = i as f64;
        let x = (rnd(fi * 1.3) * w as f64) as f32;
        let y = (rnd(fi * 2.9) * h as f64) as f32;
        let sh = shade_rgb("#5B9A48", (0.7 + rnd(fi * 7.7) * 0.6) as f32);
        let x2 = x + ((rnd(fi) - 0.5) * 6.0) as f32;
        let y2 = y - 6.0 - (rnd(fi * 9.0) * 8.0) as f32;
        p.stroke_polyline(&[(x, y), (x2, y2)], 2.0, (sh.0, sh.1, sh.2, 255));
    }
    (p.px, w as u32, h as u32)
}

/// Sky gradient strip (2 x 512): the world.ts sky() shader —
/// y > 0: mix(horizon, top, pow(clamp(y*1.6, 0, 1), 0.7)); below the
/// horizon the ground colour defaults to the horizon colour. The mix
/// is done in linear space like three.js colour management.
fn paint_sky() -> (Vec<u8>, u32, u32) {
    let (w, h) = (2usize, 512usize);
    let top = hex_lin("#6E9CC0");
    let hor = hex_lin("#D8E8DA");
    let mut px = vec![0u8; w * h * 4];
    for j in 0..h {
        // wgpu samples memory row j at v = (j + 0.5) / h, and the
        // dome's uv has v = 1 at the top pole, so row j carries the
        // direction ny = 2v - 1 for ITS v.
        let v = (j as f32 + 0.5) / h as f32;
        let ny = 2.0 * v - 1.0;
        let t = if ny > 0.0 { (ny * 1.6).clamp(0.0, 1.0).powf(0.7) } else { 0.0 };
        let col = [
            hor[0] + (top[0] - hor[0]) * t,
            hor[1] + (top[1] - hor[1]) * t,
            hor[2] + (top[2] - hor[2]) * t,
        ];
        for i in 0..w {
            let k = (j * w + i) * 4;
            px[k] = (linear_to_srgb(col[0]) * 255.0).round() as u8;
            px[k + 1] = (linear_to_srgb(col[1]) * 255.0).round() as u8;
            px[k + 2] = (linear_to_srgb(col[2]) * 255.0).round() as u8;
            px[k + 3] = 255;
        }
    }
    (px, w as u32, h as u32)
}

/// cloudTex(): 7 puffs, a blue-grey shadow pass offset (+4, +14) and a
/// white main pass offset (-2, -2) at r*0.92, tinted '#EFF4EA'. The
/// canvas blurs (3 px shadow, 10 px bottom strip) are approximated by
/// radial alpha falloff and a bottom alpha fade — see gaps.md.
fn paint_cloud() -> (Vec<u8>, u32, u32) {
    let (w, h) = (512usize, 256usize);
    let mut p = Painter::new(w, h);
    let puffs: [(f32, f32, f32); 7] = [
        (110.0, 175.0, 58.0),
        (185.0, 140.0, 80.0),
        (285.0, 118.0, 96.0),
        (380.0, 150.0, 70.0),
        (250.0, 178.0, 70.0),
        (440.0, 180.0, 44.0),
        (70.0, 190.0, 36.0),
    ];
    let tint = hex_rgb("#EFF4EA");
    let soft_disc = |p: &mut Painter, cx: f32, cy: f32, r: f32, col: (u8, u8, u8)| {
        let (x0, y0, x1, y1) = p.bbox(cx - r, cy - r, cx + r, cy + r);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt() / r;
                if d < 1.0 {
                    let a = if d < 0.72 { 1.0 } else { 1.0 - (d - 0.72) / 0.28 };
                    p.blend(x, y, (col.0, col.1, col.2, (a * 255.0) as u8));
                }
            }
        }
    };
    for &(cx, cy, r) in &puffs {
        soft_disc(&mut p, cx + 4.0, cy + 14.0, r, hex_rgb("#B9CCE6"));
    }
    for &(cx, cy, r) in &puffs {
        soft_disc(&mut p, cx - 2.0, cy - 2.0, r * 0.92, (255, 255, 255));
    }
    // Bottom soft strip + tint multiply.
    for y in 0..h {
        for x in 0..w {
            let k = (y * w + x) * 4;
            if y >= 214 {
                let f = 1.0 - 0.85 * ((y as f32 - 214.0) / 42.0).powf(1.5);
                p.px[k + 3] = (p.px[k + 3] as f32 * f) as u8;
            }
            for c in 0..3 {
                let tv = [tint.0, tint.1, tint.2][c] as f32 / 255.0;
                p.px[k + c] = (p.px[k + c] as f32 * tv).round() as u8;
            }
        }
    }
    (p.px, w as u32, h as u32)
}

/// Blob shadow texture (toon.ts blobTex): radial gradient
/// rgba(30,20,40, 0.75 -> 0.25 @ 0.6 -> 0), with the material's 0.55
/// opacity baked in.
fn paint_blob() -> (Vec<u8>, u32, u32) {
    let (w, h) = (128usize, 128usize);
    let mut px = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f32 + 0.5 - 64.0).powi(2) + (y as f32 + 0.5 - 64.0).powi(2)).sqrt() / 64.0;
            let a = if d >= 1.0 {
                0.0
            } else if d <= 0.6 {
                0.75 + (0.25 - 0.75) * (d / 0.6)
            } else {
                0.25 * (1.0 - (d - 0.6) / 0.4)
            };
            let k = (y * w + x) * 4;
            px[k] = 30;
            px[k + 1] = 20;
            px[k + 2] = 40;
            px[k + 3] = (a * 0.55 * 255.0).round() as u8;
        }
    }
    (px, w as u32, h as u32)
}

// ---------------------------------------------------------------------
// Scene assembly: the JG jungle camp at t = 1.50, re-authored from
// shots.ts (see README.md for the line-by-line extraction).
// ---------------------------------------------------------------------

struct ColourDraw {
    geo: Geo,
    model: Mat4,
}

struct TexDraw {
    geo: TexGeo,
    model: Mat4,
    tex: usize, // index into the texture table
    repeat: bool,
}

const TEX_SKY: usize = 0;
const TEX_GRASS: usize = 1;
const TEX_CLOUD: usize = 2;
const TEX_BLOB: usize = 3;
const TEX_CARD: usize = 4;
const TEX_CAPTION: usize = 5;

// Camera at t = 1.50 (camK smootherstep k = 0.05228 through the S01
// push-in; fov 44 from G.jungle spread in the shoot() call).
const EYE: [f32; 3] = [0.83432, 2.64354, 5.28499];
const TGT: [f32; 3] = [0.35, 1.11477, -0.55];

fn t3(x: f32, y: f32, z: f32) -> Mat4 {
    math::translate(x, y, z)
}

struct Scene {
    colour: Vec<ColourDraw>,
    sky: TexDraw,
    grass: TexDraw,
    blob: TexDraw,
    clouds: Vec<TexDraw>,
    card: TexDraw,
    caption: TexDraw,
}

fn build_scene() -> Scene {
    let mut colour: Vec<ColourDraw> = Vec::new();
    let mut add = |draws: &mut Vec<ColourDraw>, geo: Geo, model: Mat4| {
        draws.push(ColourDraw { geo, model });
    };

    // ---- dirt patch (world.ts: CircleGeometry r 4.8, flat '#86714C')
    add(&mut colour, disc_geo(4.8, 28, hex_lin("#86714C")), t3(0.0, 0.012, -0.9));

    // ---- hills (z -65..-85) with their small trees ----
    let hills: [(f32, f32, f32, f32); 4] = [
        (-40.0, -75.0, 34.0, 12.0),
        (10.0, -85.0, 46.0, 16.0),
        (60.0, -71.0, 36.0, 11.0),
        (-90.0, -65.0, 40.0, 10.0),
    ];
    for (i, &(hx, hz, r, h)) in hills.iter().enumerate() {
        let col = shade_lin("#5E8F52", (0.92 + rnd(i as f64) * 0.16) as f32);
        let model = math::mul(&t3(hx, -h * 0.25, hz), &math::scale(r, h, r * 0.6));
        add(&mut colour, sphere_geo(1.0, 24, 16, col), model);
        // 6 small trees per hill (castShadow false in the original).
        for k in 0..6 {
            let a = rnd(i as f64 * 9.0 + k as f64) * 2.0 - 1.0;
            let s = (0.5 + rnd(k as f64) * 0.4) as f32 * 2.2;
            let pos = [
                hx + (a as f32) * r * 0.6,
                (h as f64 * (0.75 - a.abs() * 0.45) - 1.0) as f32,
                hz + r * 0.25,
            ];
            add_tree(&mut colour, pos, 0.0, s, "#3E7A34", true);
        }
    }

    // ---- tree ring (19 trees, rotation.y = i*1.7) ----
    let ring: [(f32, f32, f32, bool); 19] = [
        (-6.5, -6.0, 1.5, true),
        (-9.0, -2.0, 1.3, false),
        (-7.5, -11.0, 1.7, true),
        (-12.0, -7.0, 1.5, true),
        (-13.0, -14.0, 1.8, false),
        (6.8, -6.5, 1.6, true),
        (9.5, -2.5, 1.35, false),
        (8.0, -12.0, 1.75, true),
        (12.5, -8.0, 1.5, true),
        (13.5, -15.0, 1.9, false),
        (-4.5, -13.0, 1.6, true),
        (4.6, -14.0, 1.55, true),
        (0.0, -17.0, 1.9, true),
        (-16.0, -4.0, 1.6, true),
        (16.5, -5.0, 1.7, true),
        (-5.5, 4.5, 1.3, true),
        (6.2, 5.0, 1.25, false),
        (-11.0, 3.0, 1.5, true),
        (11.5, 3.5, 1.45, true),
    ];
    for (i, &(x, z, s, round)) in ring.iter().enumerate() {
        let col = if i % 3 != 0 { "#3E7A34" } else { "#356B2C" };
        add_tree(&mut colour, [x, 0.0, z], i as f32 * 1.7, s, col, round);
    }

    // ---- palms ----
    add_palm(&mut colour, [4.6, 0.0, -4.6], 1.05, 0.14, 0.0);
    add_palm(&mut colour, [-5.2, 0.0, -5.4], 0.9, -0.2, 1.2);
    add_palm(&mut colour, [10.5, 0.0, -11.0], 1.15, 0.1, 0.0);

    // ---- grass tufts: 900 instanced cones in the original; here the
    // same hash-placed cones merged into one geometry ----
    {
        let mut merged = Geo::new();
        let cone = cone_geo(0.05, 0.34, 4, hex_lin("#4C8A3E"));
        for i in 0..900 {
            let fi = i as f64;
            let a = rnd(fi * 1.31) * 6.283185307;
            let r = 2.6 + rnd(fi * 2.7) * 20.0;
            let (x, z) = ((a.cos() * r) as f32, (a.sin() * r) as f32 - 3.0);
            let mut inst = Geo { verts: cone.verts.clone(), idx: cone.idx.clone() };
            for v in inst.verts.chunks_exact_mut(9) {
                v[0] += x;
                v[1] += 0.15;
                v[2] += z;
            }
            merged.append(inst);
        }
        add(&mut colour, merged, math::identity());
    }

    // ---- camp (fire pit at FX=-1.35, FZ=-2.45) ----
    let (fx, fz) = (-1.35f32, -2.45f32);
    let stone = hex_lin("#8B8B84");
    for i in 0..8 {
        let a = (i as f32 / 8.0) * std::f32::consts::TAU;
        let r = (0.09 + rnd(i as f64) * 0.04) as f32;
        add(
            &mut colour,
            sphere_geo(r, 8, 6, stone),
            t3(fx + a.cos() * 0.44, 0.05, fz + a.sin() * 0.44),
        );
    }
    let log_col = hex_lin("#7A5A38");
    for (i, &(dx, dz)) in [(0.25f32, 0.3f32), (-0.2, -0.28), (0.05, 0.02)].iter().enumerate() {
        let model = math::mul(
            &t3(fx + dx, 0.09, fz + dz),
            &math::euler_xyz(0.0, i as f32 * 0.7, std::f32::consts::FRAC_PI_2),
        );
        add(&mut colour, cyl_geo(0.05, 0.05, 0.72, 7, log_col), model);
    }
    // Flames at t = 1.50: fireLevel scaleY f = 1.13146 / 0.86854.
    // Emissive materials in the original; flat bright colours here
    // (no emissive/bloom/tonemap in this renderer — gaps.md).
    let flame_model = math::mul(&t3(fx, 0.34, fz), &math::scale(1.0, 1.13146, 1.0));
    add(&mut colour, cone_geo(0.17, 0.52, 8, hex_lin("#FF8A30")), flame_model);
    let flame2_model = math::mul(&t3(fx + 0.03, 0.30, fz + 0.04), &math::scale(1.0, 0.86854, 1.0));
    add(&mut colour, cone_geo(0.09, 0.34, 8, hex_lin("#FFD23C")), flame2_model);
    // Seat log.
    let seat_model = math::mul(
        &t3(-0.5, 0.17, -1.8),
        &math::euler_xyz(0.0, 0.12, std::f32::consts::FRAC_PI_2),
    );
    add(&mut colour, cyl_geo(0.17, 0.17, 1.15, 9, log_col), seat_model);
    // Lean-to: two poles + plank roof (flat colour — planks texture
    // not reproduced, gaps.md).
    for x in [-3.9f32, -2.7] {
        add(&mut colour, cyl_geo(0.035, 0.035, 1.5, 6, log_col), t3(x, 0.75, -2.7));
    }
    let roof_model = math::mul(&t3(-3.3, 1.52, -2.95), &math::rotate_x(0.42));
    add(&mut colour, box_geo(1.9, 0.05, 1.5, hex_lin("#8A6B42")), roof_model);
    // Crates + barrel + bedroll (flat colours; planks maps dropped).
    add(&mut colour, box_geo(0.55, 0.55, 0.55, hex_lin("#B07A48")), t3(-2.35, 0.275, -3.15));
    let crate2_model = math::mul(&t3(-2.3, 0.55 + 0.21, -3.12), &math::rotate_y(0.4));
    add(&mut colour, box_geo(0.42, 0.42, 0.42, hex_lin("#B07A48")), crate2_model);
    add(&mut colour, cyl_geo(0.33, 0.33, 0.8, 16, hex_lin("#6E4525")), t3(-3.05, 0.4, -2.2));
    let band_col = hex_lin("#3A3A3F");
    for y in [0.12f32, 0.68] {
        add(&mut colour, cyl_geo(0.345, 0.345, 0.05, 16, band_col), t3(-3.05, y, -2.2));
    }
    let bed_model = math::mul(&t3(-3.3, 0.05, -2.1), &math::rotate_y(0.2));
    add(&mut colour, box_geo(1.5, 0.09, 0.55, hex_lin("#77805E")), bed_model);

    // ---- Onoda card + rifle ----
    // Toon H = 1.75 / 0.96; plane 1.25H x 1.375H; geometry centre sits
    // 552 stage units above the feet (toon.ts plane derivation).
    let h_t = 1.75f32 / 0.96;
    let su = h_t / 1024.0;
    let plane_w = h_t * 1280.0 / 1024.0;
    let plane_h = h_t * 1408.0 / 1024.0;
    let centre_y = 552.0 * su;
    // face(): yaw atan2(cam.x - p.x, cam.z - p.z) and perspective
    // pitch clamp(pitchAng * 0.45) about the pivot (order YXZ).
    let card_model = math::mul(
        &t3(0.35, 0.0, -0.55),
        &math::mul(&math::rotate_y(0.08282), &math::rotate_x(0.11642)),
    );
    let card = TexDraw {
        geo: quad_geo(plane_w, plane_h, centre_y - plane_h / 2.0),
        model: card_model,
        tex: TEX_CARD,
        repeat: false,
    };
    // Rifle (attach3D, hand R): anchor from the painter's hand
    // position through syncAttachments' formula, offset [0, .03, .07].
    {
        let (hx, hy) = (373.98f32, 599.73f32); // stage-space hand R
        let w = [(384.0 + hx - 640.0) * su, (1256.0 - (256.0 + hy)) * su, 0.0];
        let rifle_model = math::mul(
            &card_model,
            &t3(w[0] + 0.0, w[1] + 0.03, w[2] + 0.07),
        );
        let wood = hex_lin("#6E4F30");
        let steel = hex_lin("#3A3D42");
        let parts: [(Geo, Mat4); 5] = [
            (box_geo(0.075, 0.52, 0.13, wood), t3(0.0, -0.17, 0.0)),
            (box_geo(0.06, 0.1, 0.09, steel), t3(0.0, 0.13, 0.0)),
            (cyl_geo(0.016, 0.016, 0.55, 8, steel), t3(0.0, 0.42, 0.0)),
            (box_geo(0.02, 0.05, 0.02, steel), t3(0.0, 0.68, 0.0)),
            (box_geo(0.05, 0.05, 0.16, wood), t3(0.0, -0.02, 0.05)),
        ];
        for (geo, local) in parts {
            add(&mut colour, geo, math::mul(&rifle_model, &local));
        }
    }

    // ---- sky dome + grass ground + blob + clouds + caption ----
    let sky = TexDraw { geo: sky_sphere_geo(380.0, 32, 16), model: math::identity(), tex: TEX_SKY, repeat: false };
    let grass = TexDraw { geo: ground_disc_geo(80.0, 48, 48.0), model: math::identity(), tex: TEX_GRASS, repeat: true };
    let blob = TexDraw {
        geo: flat_quad_geo((h_t / 2.0) * 0.9, (h_t / 2.0) * 0.34),
        model: t3(0.35, 0.02, -0.55),
        tex: TEX_BLOB,
        repeat: false,
    };
    // Clouds: three sprites, billboarding toward the camera.
    let fwd = norm3(sub3(TGT, EYE));
    let right = norm3(cross3(fwd, [0.0, 1.0, 0.0]));
    let upv = cross3(right, fwd);
    let mut clouds = Vec::new();
    for (cx, cy, cz, s) in [
        (-28.0f32, 23.0f32, -85.0f32, 13.0f32),
        (26.0, 29.0, -95.0, 16.0),
        (58.0, 19.0, -75.0, 10.0),
    ] {
        let (hw, hh) = (s, s / 2.0); // sprite scale (2s, s)
        let corner = |sx: f32, sy: f32| -> [f32; 3] {
            [
                cx + right[0] * hw * sx + upv[0] * hh * sy,
                cy + right[1] * hw * sx + upv[1] * hh * sy,
                cz + right[2] * hw * sx + upv[2] * hh * sy,
            ]
        };
        let mut g = TexGeo::new();
        g.vert(corner(-1.0, -1.0), [0.0, 1.0]);
        g.vert(corner(1.0, -1.0), [1.0, 1.0]);
        g.vert(corner(1.0, 1.0), [1.0, 0.0]);
        g.vert(corner(-1.0, 1.0), [0.0, 0.0]);
        g.tri(0, 1, 2);
        g.tri(0, 2, 3);
        clouds.push(TexDraw { geo: g, model: math::identity(), tex: TEX_CLOUD, repeat: false });
    }
    // Caption strip: "fighting" at (540, 1340) in a 1080x1920 frame,
    // as a camera-facing quad 2 m ahead (see README.md).
    let caption = build_caption_draw();

    Scene { colour, sky, grass, blob, clouds, card, caption }
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm3(a: [f32; 3]) -> [f32; 3] {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    [a[0] / l, a[1] / l, a[2] / l]
}

/// One jungle tree at `pos`: trunk + (round: 5 blobs | cypress: 1
/// scaled sphere), in world.ts tree() proportions. Icosahedron
/// canopies are approximated by spheres (gaps.md).
fn add_tree(draws: &mut Vec<ColourDraw>, pos: [f32; 3], rot_y: f32, s: f32, col: &str, round: bool) {
    let base = math::mul(&t3(pos[0], pos[1], pos[2]), &math::rotate_y(rot_y));
    let trunk = cyl_geo(0.12 * s, 0.18 * s, 1.6 * s, 7, hex_lin("#7A5234"));
    draws.push(ColourDraw {
        geo: trunk,
        model: math::mul(&base, &t3(0.0, 0.8 * s, 0.0)),
    });
    if round {
        let blobs: [(f32, f32, f32, f32); 5] = [
            (0.0, 2.3, 0.0, 1.0),
            (0.6, 2.0, 0.2, 0.75),
            (-0.6, 2.0, -0.1, 0.8),
            (0.1, 2.9, -0.2, 0.75),
            (0.2, 2.2, 0.6, 0.7),
        ];
        for (i, &(ox, oy, oz, f)) in blobs.iter().enumerate() {
            let c = shade_lin(col, (0.9 + rnd(i as f64) * 0.2) as f32);
            draws.push(ColourDraw {
                geo: sphere_geo(f * s, 12, 9, c),
                model: math::mul(&base, &t3(ox * s, oy * s, oz * s)),
            });
        }
    } else {
        let model = math::mul(
            &math::mul(&base, &t3(0.0, 3.2 * s, 0.0)),
            &math::scale(0.7 * s, 2.6 * s, 0.7 * s),
        );
        draws.push(ColourDraw { geo: sphere_geo(1.0, 16, 12, hex_lin(col)), model });
    }
}

/// One palm (shots.ts palm()): leaning trunk, 7 fronds with tips,
/// 3 coconuts. Frond spheres approximate the original's flattened
/// sphere fronds with the same scale/placement.
fn add_palm(draws: &mut Vec<ColourDraw>, pos: [f32; 3], s: f32, lean: f32, rot_y: f32) {
    let base = math::mul(
        &t3(pos[0], pos[1], pos[2]),
        &math::mul(&math::rotate_y(rot_y), &math::mul(&math::rotate_z(lean), &math::scale(s, s, s))),
    );
    draws.push(ColourDraw {
        geo: cyl_geo(0.085, 0.13, 4.4, 8, hex_lin("#7A6248")),
        model: math::mul(&base, &t3(0.0, 2.2, 0.0)),
    });
    let top_y = 4.35f32;
    let leaf = hex_lin("#3F7D33");
    let leaf2 = hex_lin("#4C8A3A");
    for i in 0..7 {
        let a = (i as f32 / 7.0) * std::f32::consts::TAU + 0.4;
        let (sa, ca) = a.sin_cos();
        let col = if i % 2 == 0 { leaf } else { leaf2 };
        let frond_model = math::mul(
            &math::mul(&base, &t3(sa * 1.0, top_y + 0.1 - (i % 3) as f32 * 0.07, ca * 1.0)),
            &math::mul(&math::rotate_y(a), &math::scale(0.17, 0.05, 1.2)),
        );
        draws.push(ColourDraw { geo: sphere_geo(1.0, 10, 6, col), model: frond_model });
        let tip_model = math::mul(
            &math::mul(&base, &t3(sa * 1.85, top_y - 0.28 - (i % 3) as f32 * 0.06, ca * 1.85)),
            &math::mul(&math::euler_xyz(ca * 0.5, a, -sa * 0.5), &math::scale(0.13, 0.04, 0.8)),
        );
        draws.push(ColourDraw { geo: sphere_geo(1.0, 10, 6, col), model: tip_model });
    }
    let nut = hex_lin("#6B4A2E");
    for &(nx, nz) in [(0.14f32, 0.0f32), (-0.12, 0.1), (0.0, -0.14)].iter() {
        draws.push(ColourDraw {
            geo: sphere_geo(0.13, 10, 8, nut),
            model: math::mul(&base, &t3(nx, top_y - 0.2, nz)),
        });
    }
}

/// The caption as a textured quad floating 2 m in front of the
/// camera, centred on the caption's NDC position (0, -0.3958) and
/// spanning the full frame width, 300/1920 of the frame height.
fn build_caption_draw() -> TexDraw {
    let d = 2.0f32;
    let tan_h = (22.0f32).to_radians().tan(); // fov 44 / 2
    let aspect = 1080.0f32 / 1920.0;
    let cy = -0.395833f32 * tan_h * d;
    let hw = aspect * tan_h * d;
    let hh = (300.0f32 / 1920.0) * tan_h * d;
    let fwd = norm3(sub3(TGT, EYE));
    let right = norm3(cross3(fwd, [0.0, 1.0, 0.0]));
    let upv = cross3(right, fwd);
    let corner = |sx: f32, sy: f32| -> [f32; 3] {
        [
            EYE[0] + right[0] * hw * sx + upv[0] * (cy + hh * sy) + fwd[0] * d,
            EYE[1] + right[1] * hw * sx + upv[1] * (cy + hh * sy) + fwd[1] * d,
            EYE[2] + right[2] * hw * sx + upv[2] * (cy + hh * sy) + fwd[2] * d,
        ]
    };
    let mut g = TexGeo::new();
    g.vert(corner(-1.0, -1.0), [0.0, 1.0]);
    g.vert(corner(1.0, -1.0), [1.0, 1.0]);
    g.vert(corner(1.0, 1.0), [1.0, 0.0]);
    g.vert(corner(-1.0, 1.0), [0.0, 0.0]);
    g.tri(0, 1, 2);
    g.tri(0, 2, 3);
    TexDraw { geo: g, model: math::identity(), tex: TEX_CAPTION, repeat: false }
}

// ---------------------------------------------------------------------
// Rendering. Same device/pipeline structure as the M1 scene in
// main.rs, with two honest differences, both forced by what this
// scene is: the textured pipeline alpha-blends (the character is a
// cut-out with transparent margins, the clouds are sprites — M1's
// card was opaque), and textures carry no mipmaps (three.js builds
// them; logged in gaps.md).
// ---------------------------------------------------------------------

struct Uniforms {
    mvp: Mat4,
    model: Mat4,
}

fn mat_bytes(m: &Mat4) -> [u8; 64] {
    let mut out = [0u8; 64];
    let mut k = 0;
    for col in 0..4 {
        for row in 0..4 {
            out[k..k + 4].copy_from_slice(&m[col][row].to_ne_bytes());
            k += 4;
        }
    }
    out
}

fn uniform_bytes(u: &Uniforms) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(&mat_bytes(&u.mvp));
    out.extend_from_slice(&mat_bytes(&u.model));
    out
}

fn pack_f32(data: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for v in data {
        out.extend_from_slice(&v.to_ne_bytes());
    }
    out
}

fn pack_u32(data: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for v in data {
        out.extend_from_slice(&v.to_ne_bytes());
    }
    out
}

pub async fn run_ab1(width: u32, height: u32, out: &str, caption_path: &str) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });
    let adapter = crate::request_adapter(&instance).await;
    let info = adapter.get_info();
    println!(
        "adapter: {} | backend: {:?} | device type: {:?} | driver: {} {}",
        info.name, info.backend, info.device_type, info.driver, info.driver_info
    );
    println!("scene: ab1 — Onoda S01 @ t=1.50 (jungle camp JG), fov 44");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("talerender device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .expect("request_device failed");

    // ---- camera: S01's extracted frame ----
    let aspect = width as f32 / height as f32;
    let proj = math::perspective(44f32.to_radians(), aspect, 0.05, 800.0);
    let view = math::look_at(EYE, TGT, [0.0, 1.0, 0.0]);
    let vp = math::mul(&proj, &view);

    // ---- pipelines ----
    let shader3d = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("geometry shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader3d.wgsl"))),
    });
    let bgl3d = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("uniform layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let layout3d = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("colour pipeline layout"),
        bind_group_layouts: &[&bgl3d],
        push_constant_ranges: &[],
    });
    let colour_attrs = [
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 24, shader_location: 2 },
    ];
    let pipeline3d = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("colour pipeline"),
        layout: Some(&layout3d),
        vertex: wgpu::VertexState {
            module: &shader3d,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 36,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &colour_attrs,
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader3d,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let shader_tex = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("textured shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader_tex.wgsl"))),
    });
    let bgl_tex = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("textured layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let layout_tex = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("textured pipeline layout"),
        bind_group_layouts: &[&bgl_tex],
        push_constant_ranges: &[],
    });
    let tex_attrs = [
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 12, shader_location: 1 },
    ];
    let pipeline_tex = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("textured pipeline"),
        layout: Some(&layout_tex),
        vertex: wgpu::VertexState {
            module: &shader_tex,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 20,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &tex_attrs,
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_tex,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // ---- textures ----
    let caption_img = image::open(caption_path)
        .unwrap_or_else(|e| panic!("cannot load caption strip {caption_path}: {e}"))
        .to_rgba8();
    let (cap_w, cap_h) = caption_img.dimensions();
    let textures_px: [(Vec<u8>, u32, u32); 6] = [
        paint_sky(),
        paint_grass(),
        paint_cloud(),
        paint_blob(),
        paint_onoda(),
        (caption_img.into_raw(), cap_w, cap_h),
    ];
    let mut tex_views: Vec<wgpu::TextureView> = Vec::new();
    for (px, w, h) in &textures_px {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene texture"),
            size: wgpu::Extent3d { width: *w, height: *h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            px,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(*h),
            },
            wgpu::Extent3d { width: *w, height: *h, depth_or_array_layers: 1 },
        );
        tex_views.push(tex.create_view(&wgpu::TextureViewDescriptor::default()));
    }
    let sampler_clamp = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("clamp sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let sampler_repeat = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("repeat sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    // ---- geometry buffers + bind groups ----
    let scene = build_scene();
    let make_uniform = |model: &Mat4| -> wgpu::Buffer {
        let u = Uniforms { mvp: math::mul(&vp, model), model: *model };
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buf, 0, &uniform_bytes(&u));
        buf
    };
    let make_vb = |verts: &[f32], idx: &[u32]| -> (wgpu::Buffer, wgpu::Buffer, u32) {
        let vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex buffer"),
            size: (verts.len() * 4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vb, 0, &pack_f32(verts));
        let ib = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index buffer"),
            size: (idx.len() * 4) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&ib, 0, &pack_u32(idx));
        (vb, ib, idx.len() as u32)
    };
    struct PreparedColour {
        vb: wgpu::Buffer,
        ib: wgpu::Buffer,
        n: u32,
        bg: wgpu::BindGroup,
        _ub: wgpu::Buffer,
    }
    struct PreparedTex {
        vb: wgpu::Buffer,
        ib: wgpu::Buffer,
        n: u32,
        bg: wgpu::BindGroup,
        _ub: wgpu::Buffer,
    }
    let mut colour_prepared: Vec<PreparedColour> = Vec::new();
    for d in &scene.colour {
        let (vb, ib, n) = make_vb(&d.geo.verts, &d.geo.idx);
        let ub = make_uniform(&d.model);
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("colour bind group"),
            layout: &bgl3d,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() }],
        });
        colour_prepared.push(PreparedColour { vb, ib, n, bg, _ub: ub });
    }
    let prepare_tex = |d: &TexDraw| -> PreparedTex {
        let (vb, ib, n) = make_vb(&d.geo.verts, &d.geo.idx);
        let ub = make_uniform(&d.model);
        let sampler = if d.repeat { &sampler_repeat } else { &sampler_clamp };
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("textured bind group"),
            layout: &bgl_tex,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex_views[d.tex]) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
            ],
        });
        PreparedTex { vb, ib, n, bg, _ub: ub }
    };
    let sky_p = prepare_tex(&scene.sky);
    let grass_p = prepare_tex(&scene.grass);
    let blob_p = prepare_tex(&scene.blob);
    let clouds_p: Vec<PreparedTex> = scene.clouds.iter().map(&prepare_tex).collect();
    let card_p = prepare_tex(&scene.card);
    let caption_p = prepare_tex(&scene.caption);

    // ---- frame target + readback ----
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frame target"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view_tex = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth target"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    let padded = math::padded_row_bytes(width);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback buffer"),
        size: (padded * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("frame encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ab1 scene pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view_tex,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Horizon colour '#D8E8DA', in linear values (the
                    // sRGB target encodes on write).
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: srgb_to_linear(216.0 / 255.0) as f64,
                        g: srgb_to_linear(232.0 / 255.0) as f64,
                        b: srgb_to_linear(218.0 / 255.0) as f64,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        // Sky + grass (textured, opaque content) first.
        pass.set_pipeline(&pipeline_tex);
        for tp in [&sky_p, &grass_p] {
            pass.set_bind_group(0, &tp.bg, &[]);
            pass.set_vertex_buffer(0, tp.vb.slice(..));
            pass.set_index_buffer(tp.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..tp.n, 0, 0..1);
        }
        // All coloured geometry.
        pass.set_pipeline(&pipeline3d);
        for cp in &colour_prepared {
            pass.set_bind_group(0, &cp.bg, &[]);
            pass.set_vertex_buffer(0, cp.vb.slice(..));
            pass.set_index_buffer(cp.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..cp.n, 0, 0..1);
        }
        // Transparent textured content, back to front.
        pass.set_pipeline(&pipeline_tex);
        for tp in std::iter::once(&blob_p).chain(clouds_p.iter()).chain([&card_p, &caption_p]) {
            pass.set_bind_group(0, &tp.bg, &[]);
            pass.set_vertex_buffer(0, tp.vb.slice(..));
            pass.set_index_buffer(tp.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..tp.n, 0, 0..1);
        }
    }
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().expect("map channel closed").expect("buffer map failed");
    let data = slice.get_mapped_range();
    let row = (width * 4) as usize;
    let stride = padded as usize;
    let mut pixels = Vec::with_capacity(row * height as usize);
    for y in 0..height as usize {
        pixels.extend_from_slice(&data[y * stride..y * stride + row]);
    }
    drop(data);
    readback.unmap();

    let img = image::RgbaImage::from_raw(width, height, pixels).expect("pixel buffer size mismatch");
    img.save(out).expect("failed to save PNG");
    println!("wrote {out} ({width}x{height})");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri_normal_dot_centroid(verts: &[f32], idx: &[u32]) -> f32 {
        let p = |k: usize| -> [f32; 3] {
            let o = (idx[k] as usize) * 9;
            [verts[o], verts[o + 1], verts[o + 2]]
        };
        let (a, b, c) = (p(0), p(1), p(2));
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let cen = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
        n[0] * cen[0] + n[1] * cen[1] + n[2] * cen[2]
    }

    #[test]
    fn generated_shapes_face_outward() {
        // The colour pipeline culls back faces: a wrongly wound shape
        // would vanish. Scan for the first non-degenerate triangle
        // (sphere poles produce degenerate ones) and check its normal
        // points away from the centroid.
        let check = |g: &Geo, name: &str| {
            let mut found = false;
            for t in g.idx.chunks_exact(3) {
                let d = tri_normal_dot_centroid(&g.verts, t);
                if d.abs() > 1e-4 {
                    assert!(d > 0.0, "{name}: first solid triangle faces inward");
                    found = true;
                    break;
                }
            }
            assert!(found, "{name}: no non-degenerate triangle found");
        };
        check(&sphere_geo(1.0, 8, 6, [0.0; 3]), "sphere");
        check(&cyl_geo(0.3, 0.4, 1.0, 8, [0.0; 3]), "cylinder");
        check(&cone_geo(0.4, 1.0, 8, [0.0; 3]), "cone");
        check(&box_geo(1.0, 2.0, 0.5, [0.0; 3]), "box");
    }

    #[test]
    fn sky_sphere_faces_inward() {
        // The dome is viewed from inside; its triangles must face the
        // centre or the back-face cull removes the whole sky.
        let g = sky_sphere_geo(10.0, 8, 6);
        let mut found = false;
        for t in g.idx.chunks_exact(3) {
            let p = |k: usize| -> [f32; 3] {
                let o = (t[k] as usize) * 5;
                [g.verts[o], g.verts[o + 1], g.verts[o + 2]]
            };
            let (a, b, c) = (p(0), p(1), p(2));
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let cen = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
            let d = n[0] * cen[0] + n[1] * cen[1] + n[2] * cen[2];
            if d.abs() > 1e-3 {
                assert!(d < 0.0, "sky triangle faces outward");
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn rnd_matches_the_js_sin_hash() {
        // rnd(i) = frac(sin(i*127.1 + 311.7) * 43758.5453)
        assert!((rnd(0.0) - 0.81931).abs() < 0.001, "rnd(0) = {}", rnd(0.0));
        for i in 0..100 {
            let v = rnd(i as f64);
            assert!((0.0..1.0).contains(&v), "rnd({i}) = {v}");
        }
    }

    #[test]
    fn onoda_card_paints_a_figure() {
        let (px, w, h) = paint_onoda();
        assert_eq!((w, h), (2560, 2816));
        // The head centre (stage 256, 178 -> canvas px) must be the
        // head colour, and a corner must stay transparent.
        let cx = ((256.0 + 384.0) * 2.0) as usize;
        let cy = ((178.0 + 256.0) * 2.0) as usize;
        let k = (cy * w as usize + cx) * 4;
        assert_eq!(&px[k..k + 3], &[247, 247, 250], "head centre should be #F7F7FA");
        assert_eq!(px[3], 0, "canvas corner should be transparent");
    }
}
