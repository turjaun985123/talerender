//! Shared scene building blocks (ADR-001's `scene` module, landed
//! with M2): the closed geometry vocabulary, the CPU painter the
//! environment textures are painted with, the texture painters
//! themselves, the draw records the renderers consume, and the
//! engine's deterministic sin-hash.
//!
//! Everything here was extracted verbatim from ab1.rs (the §28 scene)
//! when the scene-format loader became the second consumer; the
//! hand-authored scenes' rendering is unchanged by the move, which
//! the M2 A/B package verifies pixel-for-pixel. The one deliberate
//! generalisation: paint_grass / paint_sky take their colours as
//! parameters (the §28 scene passes its extracted values).
//!
//! This module also carries the DoF chart's procedural texture
//! (moved from dof.rs, which shares it with the format loader) and a
//! small hand-rolled JSON parser — the crate keeps its dependency
//! surface at wgpu/pollster/image (cf. math.rs), and the scene
//! format needs only the JSON subset parsed here.

use crate::math::Mat4;

// ---------------------------------------------------------------------
// Colour helpers. The TS painters work in sRGB hex; three.js converts
// material colours to linear working space and the output pass encodes
// back. Vertex colours here are linearised so the sRGB frame target
// returns them to (approximately) the authored values; texture pixels
// are sRGB bytes and round-trip through the sRGB texture format as-is.
// ---------------------------------------------------------------------

pub(crate) fn hex_rgb(s: &str) -> (u8, u8, u8) {
    let n = u32::from_str_radix(s.trim_start_matches('#'), 16).expect("bad hex colour");
    (((n >> 16) & 255) as u8, ((n >> 8) & 255) as u8, (n & 255) as u8)
}

pub(crate) fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub(crate) fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// sRGB hex -> linear [r,g,b] for vertex colours.
pub(crate) fn hex_lin(s: &str) -> [f32; 3] {
    let (r, g, b) = hex_rgb(s);
    [
        srgb_to_linear(r as f32 / 255.0),
        srgb_to_linear(g as f32 / 255.0),
        srgb_to_linear(b as f32 / 255.0),
    ]
}

/// Canvas shadeOf(): multiply the sRGB bytes by k (round, clamp).
pub(crate) fn shade_rgb(s: &str, k: f32) -> (u8, u8, u8) {
    let (r, g, b) = hex_rgb(s);
    let f = |v: u8| -> u8 { (v as f32 * k).round().clamp(0.0, 255.0) as u8 };
    (f(r), f(g), f(b))
}

pub(crate) fn shade_lin(s: &str, k: f32) -> [f32; 3] {
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
pub(crate) fn rnd(i: f64) -> f64 {
    let x = (i * 127.1 + 311.7).sin() * 43758.5453;
    x - x.floor()
}

// ---------------------------------------------------------------------
// Geometry builders (colour pipeline vertex format: pos, normal, col).
// All shapes are built centred on the origin in local space; placement
// happens through the model matrix, like the TS builders + three.js.
// ---------------------------------------------------------------------

pub(crate) struct Geo {
    pub(crate) verts: Vec<f32>,
    pub(crate) idx: Vec<u32>,
}

impl Geo {
    pub(crate) fn new() -> Self {
        Geo { verts: Vec::new(), idx: Vec::new() }
    }
    pub(crate) fn vert(&mut self, p: [f32; 3], n: [f32; 3], c: [f32; 3]) -> u32 {
        self.verts.extend_from_slice(&p);
        self.verts.extend_from_slice(&n);
        self.verts.extend_from_slice(&c);
        (self.verts.len() / 9 - 1) as u32
    }
    pub(crate) fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.idx.extend_from_slice(&[a, b, c]);
    }
    /// The pipeline culls back faces; generated shapes are convex-ish
    /// and centred on the origin, so a triangle whose geometric normal
    /// points away from its own centroid (outward build) — or toward
    /// it (inward build, e.g. the sky dome) — gets its winding flipped.
    pub(crate) fn fix_winding(&mut self, outward: bool) {
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
    pub(crate) fn append(&mut self, other: Geo) {
        let base = (self.verts.len() / 9) as u32;
        self.verts.extend_from_slice(&other.verts);
        for i in other.idx {
            self.idx.push(i + base);
        }
    }
}

/// Axis-aligned box, w x h x d, centred. Same face/winding table as the
/// verified M1 cube, generalised to per-axis half extents.
pub(crate) fn box_geo(w: f32, h: f32, d: f32, col: [f32; 3]) -> Geo {
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
pub(crate) fn cyl_ring(g: &mut Geo, y: f32, r: f32, seg: u32, cap: bool, col: [f32; 3]) -> Vec<u32> {
    let mut ids = Vec::new();
    for i in 0..seg {
        let a = (i as f32 / seg as f32) * std::f32::consts::TAU;
        let (s, c) = a.sin_cos();
        let n = if cap {
            [0.0, if y > 0.0 { 1.0 } else { -1.0 }, 0.0]
        } else {
            [c, 0.0, s]
        };
        ids.push(g.vert([r * c, y, r * s], n, col));
    }
    ids
}

pub(crate) fn cyl_geo(rt: f32, rb: f32, h: f32, seg: u32, col: [f32; 3]) -> Geo {
    let mut g = Geo::new();
    let hh = h / 2.0;
    let bot = cyl_ring(&mut g, -hh, rb, seg, false, col);
    let top = cyl_ring(&mut g, hh, rt, seg, false, col);
    let bot_cap = cyl_ring(&mut g, -hh, rb, seg, true, col);
    let top_cap = cyl_ring(&mut g, hh, rt, seg, true, col);
    let cb = g.vert([0.0, -hh, 0.0], [0.0, -1.0, 0.0], col);
    let ct = g.vert([0.0, hh, 0.0], [0.0, 1.0, 0.0], col);
    for i in 0..seg {
        let j = (i + 1) % seg;
        let (i, j) = (i as usize, j as usize);
        g.tri(bot[i], top[j], bot[j]);
        g.tri(bot[i], top[i], top[j]);
        g.tri(cb, bot_cap[j], bot_cap[i]);
        g.tri(ct, top_cap[i], top_cap[j]);
    }
    g.fix_winding(true);
    g
}

/// UV sphere, radius r, centred. Normals = positions normalised.
pub(crate) fn sphere_geo(r: f32, ws: u32, hs: u32, col: [f32; 3]) -> Geo {
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
pub(crate) fn cone_geo(r: f32, h: f32, seg: u32, col: [f32; 3]) -> Geo {
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
pub(crate) fn disc_geo(r: f32, seg: u32, col: [f32; 3]) -> Geo {
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

pub(crate) struct TexGeo {
    pub(crate) verts: Vec<f32>,
    pub(crate) idx: Vec<u32>,
}

impl TexGeo {
    pub(crate) fn new() -> Self {
        TexGeo { verts: Vec::new(), idx: Vec::new() }
    }
    pub(crate) fn vert(&mut self, p: [f32; 3], uv: [f32; 2]) -> u32 {
        self.verts.extend_from_slice(&p);
        self.verts.extend_from_slice(&uv);
        (self.verts.len() / 5 - 1) as u32
    }
    pub(crate) fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.idx.extend_from_slice(&[a, b, c]);
    }
}

/// XY quad, x in [-w/2, w/2], y in [y0, y0+h]; uv v = 0 at the TOP
/// edge (matches a CPU-painted image whose row 0 is the top).
pub(crate) fn quad_geo(w: f32, h: f32, y0: f32) -> TexGeo {
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
pub(crate) fn flat_quad_geo(w: f32, d: f32) -> TexGeo {
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
pub(crate) fn ground_disc_geo(r: f32, seg: u32, repeat: f32) -> TexGeo {
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
pub(crate) fn sky_sphere_geo(r: f32, ws: u32, hs: u32) -> TexGeo {
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

pub(crate) struct Painter {
    pub(crate) w: usize,
    pub(crate) h: usize,
    pub(crate) px: Vec<u8>, // RGBA, straight alpha, row 0 = top
}

pub(crate) type Rgba = (u8, u8, u8, u8);

pub(crate) fn rgba_of(s: &str) -> Rgba {
    let (r, g, b) = hex_rgb(s);
    (r, g, b, 255)
}

impl Painter {
    pub(crate) fn new(w: usize, h: usize) -> Self {
        Painter { w, h, px: vec![0u8; w * h * 4] }
    }
    pub(crate) fn blend(&mut self, x: i32, y: i32, src: Rgba) {
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
        let src_rgb = [src.0, src.1, src.2];
        for c in 0..3 {
            let s = src_rgb[c] as f32 / 255.0;
            let d = self.px[k + c] as f32 / 255.0;
            self.px[k + c] = (((s * sa + d * da * (1.0 - sa)) / oa) * 255.0).round() as u8;
        }
        self.px[k + 3] = (oa * 255.0).round() as u8;
    }
    pub(crate) fn bbox(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> (i32, i32, i32, i32) {
        (
            (x0.floor() as i32).max(0),
            (y0.floor() as i32).max(0),
            (x1.ceil() as i32).min(self.w as i32 - 1),
            (y1.ceil() as i32).min(self.h as i32 - 1),
        )
    }
    /// Filled ellipse (centre, radii, rotation rad).
    pub(crate) fn fill_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, col: Rgba) {
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
    pub(crate) fn stroke_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, width: f32, col: Rgba) {
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
    pub(crate) fn stroke_polyline(&mut self, pts: &[(f32, f32)], width: f32, col: Rgba) {
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
    pub(crate) fn stroke_quad(&mut self, p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), width: f32, col: Rgba) {
        let pts = flatten_quad(p0, p1, p2, 24);
        self.stroke_polyline(&pts, width, col);
    }
    /// Even-odd polygon fill (flattened path supplied by the caller).
    pub(crate) fn fill_polygon(&mut self, pts: &[(f32, f32)], col: Rgba) {
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
    pub(crate) fn polygon_mask(&self, pts: &[(f32, f32)]) -> Vec<bool> {
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
    pub(crate) fn fill_ellipse_masked(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, col: Rgba, mask: &[bool]) {
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
    pub(crate) fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, col: Rgba) {
        let (x0, y0, x1, y1) = self.bbox(x, y, x + w, y + h);
        for yy in y0..=y1 {
            for xx in x0..=x1 {
                self.blend(xx, yy, col);
            }
        }
    }
}

pub(crate) fn point_in_polygon(x: f32, y: f32, pts: &[(f32, f32)]) -> bool {
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

pub(crate) fn flatten_quad(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), n: usize) -> Vec<(f32, f32)> {
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

pub(crate) fn flatten_cubic(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), n: usize) -> Vec<(f32, f32)> {
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
// Painted textures for the set (world.ts T.grass / sky / cloudTex /
// blob shadow), re-implemented from their recipes.
// ---------------------------------------------------------------------

/// T.grass('#5B9A48', [48, 48]): base fill + 5000 hash-placed strokes.
pub(crate) fn paint_grass(base: &str) -> (Vec<u8>, u32, u32) {
    let (w, h) = (512usize, 512usize);
    let mut p = Painter::new(w, h);
    p.fill_rect(0.0, 0.0, w as f32, h as f32, rgba_of(base));
    for i in 0..5000 {
        let fi = i as f64;
        let x = (rnd(fi * 1.3) * w as f64) as f32;
        let y = (rnd(fi * 2.9) * h as f64) as f32;
        let sh = shade_rgb(base, (0.7 + rnd(fi * 7.7) * 0.6) as f32);
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
pub(crate) fn paint_sky(top_hex: &str, horizon_hex: &str) -> (Vec<u8>, u32, u32) {
    let (w, h) = (2usize, 512usize);
    let top = hex_lin(top_hex);
    let hor = hex_lin(horizon_hex);
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
pub(crate) fn paint_cloud() -> (Vec<u8>, u32, u32) {
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
pub(crate) fn paint_blob() -> (Vec<u8>, u32, u32) {
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

pub(crate) struct ColourDraw {
    pub(crate) geo: Geo,
    pub(crate) model: Mat4,
}

pub(crate) struct TexDraw {
    pub(crate) geo: TexGeo,
    pub(crate) model: Mat4,
    pub(crate) tex: usize, // index into the texture table
    pub(crate) repeat: bool,
}
pub(crate) fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn norm3(a: [f32; 3]) -> [f32; 3] {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    [a[0] / l, a[1] / l, a[2] / l]
}

/// One jungle tree at `pos`: trunk + (round: 5 blobs | cypress: 1
/// scaled sphere), in world.ts tree() proportions. Icosahedron
// ---------------------------------------------------------------------
// The DoF chart texture (moved from dof.rs, which shares it with the
// scene-format loader): one procedural pattern, evaluated in f64 so
// texel bytes agree exactly with the reference project's pattern(),
// including at cell boundaries.
// ---------------------------------------------------------------------

/// The chart pattern, as a function of (u, w): u rightward, w DOWNWARD
/// from the panel's top edge.
pub(crate) fn chart_pattern(u: f64, w: f64) -> (u8, u8, u8) {
    if u < 0.015 || u > 0.985 || w < 0.015 || w > 0.985 {
        return (255, 255, 255);
    }
    if u < 0.5 {
        let p = ((u * 10.0).floor() as i32 + (w * 10.0).floor() as i32) % 2;
        return if p == 0 { (238, 238, 238) } else { (20, 20, 20) };
    }
    if u < 0.58 {
        return (128, 128, 128);
    }
    if u < 0.79 {
        (20, 20, 20)
    } else {
        (238, 238, 238)
    }
}

/// The chart texture as RGBA bytes, `size` x `size`.
pub(crate) fn chart_texture_rgba(size: u32) -> Vec<u8> {
    let s = size as usize;
    let mut data = vec![0u8; s * s * 4];
    for y in 0..s {
        for x in 0..s {
            let (r, g, b) = chart_pattern((x as f64 + 0.5) / s as f64, (y as f64 + 0.5) / s as f64);
            let k = (y * s + x) * 4;
            data[k] = r;
            data[k + 1] = g;
            data[k + 2] = b;
            data[k + 3] = 255;
        }
    }
    data
}
