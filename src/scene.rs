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

#[derive(Clone)]
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

/// Mapped standard-material geometry (pos, normal, colour, uv —
/// 11 f32 per vertex): the diffuse is vertex colour x map, three.js's
/// `color * map` law. First consumer: the hills, whose material is
/// mat({ color: shade(col), map: grass(col, [6,3]) }) in world.ts.
#[derive(Clone)]
pub(crate) struct MapGeo {
    pub(crate) verts: Vec<f32>,
    pub(crate) idx: Vec<u32>,
}

/// UV sphere with three.js SphereGeometry's uv layout (u = ix/ws
/// around the equator, v = 1 - iy/hs so v = 1 at the top pole),
/// scaled by `uv_scale` (the texture repeat, baked in like the
/// ground disc's). Normals = positions normalised, as sphere_geo.
pub(crate) fn sphere_map_geo(r: f32, ws: u32, hs: u32, col: [f32; 3], uv_scale: [f32; 2]) -> MapGeo {
    let mut g = MapGeo { verts: Vec::new(), idx: Vec::new() };
    let mut vert = |p: [f32; 3], n: [f32; 3], uv: [f32; 2]| -> u32 {
        g.verts.extend_from_slice(&p);
        g.verts.extend_from_slice(&n);
        g.verts.extend_from_slice(&col);
        g.verts.extend_from_slice(&uv);
        (g.verts.len() / 11 - 1) as u32
    };
    let mut ids: Vec<Vec<u32>> = Vec::new();
    for iy in 0..=hs {
        let th = (iy as f32 / hs as f32) * std::f32::consts::PI;
        let (st, ct) = th.sin_cos();
        let mut row = Vec::new();
        for ix in 0..=ws {
            let ph = (ix as f32 / ws as f32) * std::f32::consts::TAU;
            let (sp, cp) = ph.sin_cos();
            let p = [r * st * cp, r * ct, r * st * sp];
            let uv = [
                (ix as f32 / ws as f32) * uv_scale[0],
                (1.0 - iy as f32 / hs as f32) * uv_scale[1],
            ];
            row.push(vert(p, [st * cp, ct, st * sp], uv));
        }
        ids.push(row);
    }
    // Same winding fix as Geo::fix_winding (outward build).
    let mut idx: Vec<u32> = Vec::new();
    for iy in 0..hs {
        for ix in 0..ws {
            let (a, b) = (ids[iy as usize][ix as usize], ids[iy as usize][ix as usize + 1]);
            let (c, d) = (ids[iy as usize + 1][ix as usize + 1], ids[iy as usize + 1][ix as usize]);
            idx.extend_from_slice(&[a, b, d, b, c, d]);
        }
    }
    for t in idx.chunks_exact_mut(3) {
        let p = |k: usize| -> [f32; 3] {
            let o = (t[k] as usize) * 11;
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
        if n[0] * cen[0] + n[1] * cen[1] + n[2] * cen[2] < 0.0 {
            t.swap(1, 2);
        }
    }
    g.idx = idx;
    g
}

// ---------------------------------------------------------------------
// Textured geometry (pos + uv).
// ---------------------------------------------------------------------

#[derive(Clone)]
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
// A small CPU rasteriser standing in for the Canvas2D painter.
// Shapes are evaluated per-pixel over their bounding box (ellipses,
// capsule strokes along flattened curves, even-odd polygon fills).
// This is the component the compatibility contract (§2) flags as
// identity-critical. Since M4 it rasterises with signed-distance
// coverage (cov = clamp(0.5 - sd)) instead of point sampling — the
// model the stroke battery (docs/ab/stroke-battery/) verifies
// against Skia; remaining deltas are logged in that package's
// gaps.md.
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
    /// Blend with an anti-aliasing coverage factor (0..1): coverage
    /// multiplies the source alpha — the per-pixel term Skia's
    /// analytic coverage contributes. Quantised to u8 like canvas
    /// storage. (M4: the painter was point-sampled/binary before;
    /// see docs/ab/stroke-battery/.)
    pub(crate) fn blend_cov(&mut self, x: i32, y: i32, src: Rgba, cov: f32) {
        if cov <= 0.0 {
            return;
        }
        if cov >= 1.0 {
            self.blend(x, y, src);
            return;
        }
        let a = (src.3 as f32 * cov).round().clamp(0.0, 255.0) as u8;
        if a > 0 {
            self.blend(x, y, (src.0, src.1, src.2, a));
        }
    }
    /// Coverage from a signed distance (negative inside):
    /// cov = clamp(0.5 - sd, 0, 1) — exact for straight edges, the
    /// standard approximation for curves.
    pub(crate) fn cov_of(sd: f32) -> f32 {
        (0.5 - sd).clamp(0.0, 1.0)
    }
    /// Filled ellipse (centre, radii, rotation rad), coverage-AA.
    pub(crate) fn fill_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, col: Rgba) {
        let ex = (rx * rot.cos()).abs() + (ry * rot.sin()).abs();
        let ey = (rx * rot.sin()).abs() + (ry * rot.cos()).abs();
        let (x0, y0, x1, y1) = self.bbox(cx - ex - 1.0, cy - ey - 1.0, cx + ex + 1.0, cy + ey + 1.0);
        let dist = |qx: f32, qy: f32| ellipse_sd(cx, cy, rx, ry, rot, qx, qy);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let cov = cov_area(&dist, x as f32 + 0.5, y as f32 + 0.5);
                if cov > 0.0 {
                    self.blend_cov(x, y, col, cov);
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
    /// Canvas2D lineCap/lineJoin 'round'), coverage-AA: per pixel the
    /// coverage is computed ONCE from the distance to the whole
    /// polyline (min over segments), so joins are the union shape —
    /// one canvas stroke is one shape, never a double blend.
    pub(crate) fn stroke_polyline(&mut self, pts: &[(f32, f32)], width: f32, col: Rgba) {
        if pts.len() < 2 {
            return;
        }
        let hw = width / 2.0;
        let (mut mnx, mut mny, mut mxx, mut mxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &(x, y) in pts {
            mnx = mnx.min(x);
            mny = mny.min(y);
            mxx = mxx.max(x);
            mxy = mxy.max(y);
        }
        let (x0, y0, x1, y1) = self.bbox(mnx - hw - 1.0, mny - hw - 1.0, mxx + hw + 1.0, mxy + hw + 1.0);
        let wpx = (x1 - x0 + 1).max(0) as usize;
        let hpx = (y1 - y0 + 1).max(0) as usize;
        // Pass 1: stamp each segment's distance into a dmin buffer
        // (per-segment bounding boxes keep this cheap).
        let mut dmin = vec![f32::MAX; wpx * hpx];
        for seg in pts.windows(2) {
            let (ax, ay) = seg[0];
            let (bx, by) = seg[1];
            let (sx0, sy0, sx1, sy1) = self.bbox(ax.min(bx) - hw - 2.0, ay.min(by) - hw - 2.0, ax.max(bx) + hw + 2.0, ay.max(by) + hw + 2.0);
            // Clamp to the union buffer (stamp padding hw+2 is
            // wider than the buffer's hw+1).
            let (sx0, sy0, sx1, sy1) = (sx0.max(x0), sy0.max(y0), sx1.min(x1), sy1.min(y1));
            for y in sy0..=sy1 {
                for x in sx0..=sx1 {
                    let d = seg_dist(x as f32 + 0.5, y as f32 + 0.5, ax, ay, bx, by);
                    let idx = ((y - y0) as usize) * wpx + (x - x0) as usize;
                    if d < dmin[idx] {
                        dmin[idx] = d;
                    }
                }
            }
        }
        // Pass 2: blend once per pixel from the union distance,
        // area-refined in the AA band (cov_area). Round caps at
        // open ends are Skia's flattened arcs, not exact discs
        // (see CapFan); closed loops have no caps.
        let nseg = pts.len() - 1;
        let closed = pts.len() > 2 && (pts[0].0 - pts[nseg].0).hypot(pts[0].1 - pts[nseg].1) < 0.05;
        let mk_fan = |vi: usize, ni: usize| -> Option<CapFan> {
            if closed {
                return None;
            }
            let (ax, ay) = pts[vi];
            let (bx, by) = pts[ni];
            let (dx, dy) = (ax - bx, ay - by); // outward: neighbour -> vertex
            let l = dx.hypot(dy);
            if l < 1e-6 {
                None
            } else {
                Some(cap_fan((ax, ay), (dx / l, dy / l), hw))
            }
        };
        let fan_start = mk_fan(0, 1);
        let fan_end = mk_fan(nseg, nseg - 1);
        let dist = |qx: f32, qy: f32| {
            let mut sd = f32::MAX;
            for (i, seg) in pts.windows(2).enumerate() {
                let (dd, t) = seg_dist_t(qx, qy, seg[0].0, seg[0].1, seg[1].0, seg[1].1);
                let s = if i == 0 && t == 0.0 {
                    fan_start.as_ref().map_or(dd - hw, |f| fan_sd(f, qx, qy))
                } else if i == nseg - 1 && t == 1.0 {
                    fan_end.as_ref().map_or(dd - hw, |f| fan_sd(f, qx, qy))
                } else {
                    dd - hw
                };
                sd = sd.min(s);
            }
            sd
        };
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d0 = dmin[((y - y0) as usize) * wpx + (x - x0) as usize] - hw;
                if d0 >= 1.0 {
                    continue; // beyond the AA band: coverage 0
                }
                // The fans shift the distance by up to the sagitta
                // (0.25 px) near terminal vertices; recompute the
                // centre distance with the fan-aware closure inside
                // a margin of the band. (Pass 1 stamps plain disc
                // distances; deeper inside, both models agree the
                // coverage is 1.)
                let d0 = if d0 > -1.5 {
                    dist(x as f32 + 0.5, y as f32 + 0.5)
                } else {
                    d0
                };
                let cov = cov_area0(&dist, x as f32 + 0.5, y as f32 + 0.5, d0);
                if cov > 0.0 {
                    self.blend_cov(x, y, col, cov);
                }
            }
        }
    }
    pub(crate) fn stroke_quad(&mut self, p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), width: f32, col: Rgba) {
        let pts = flatten_quad(p0, p1, p2, 24);
        self.stroke_polyline(&pts, width, col);
    }
    /// Even-odd polygon fill (flattened path supplied by the
    /// caller), coverage-AA from the signed distance to the outline
    /// (sign from the centre-sample inside test).
    pub(crate) fn fill_polygon(&mut self, pts: &[(f32, f32)], col: Rgba) {
        let (mut mnx, mut mny, mut mxx, mut mxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &(x, y) in pts {
            mnx = mnx.min(x);
            mny = mny.min(y);
            mxx = mxx.max(x);
            mxy = mxy.max(y);
        }
        let (x0, y0, x1, y1) = self.bbox(mnx - 1.0, mny - 1.0, mxx + 1.0, mxy + 1.0);
        let n = pts.len();
        let sd_at = |qx: f32, qy: f32| {
            let mut dmin = f32::MAX;
            for i in 0..n {
                let (ax, ay) = pts[i];
                let (bx, by) = pts[(i + 1) % n];
                dmin = dmin.min(seg_dist(qx, qy, ax, ay, bx, by));
            }
            if point_in_polygon(qx, qy, pts) {
                -dmin
            } else {
                dmin
            }
        };
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let sd0 = sd_at(px, py);
                if sd0 > 2.6 {
                    continue; // certainly outside, beyond the AA band
                }
                let cov = cov_area0(&sd_at, px, py, sd0);
                if cov > 0.0 {
                    self.blend_cov(x, y, col, cov);
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
    /// Boolean mask of an ellipse (for canvas clip() regions that are
    /// elliptical — the open-mouth clip in drawFace).
    pub(crate) fn ellipse_mask(&self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32) -> Vec<bool> {
        let mut m = vec![false; self.w * self.h];
        let ex = (rx * rot.cos()).abs() + (ry * rot.sin()).abs();
        let ey = (rx * rot.sin()).abs() + (ry * rot.cos()).abs();
        let (x0, y0, x1, y1) = self.bbox(cx - ex, cy - ey, cx + ex, cy + ey);
        let (s, c) = rot.sin_cos();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let (lx, ly) = (dx * c + dy * s, -dx * s + dy * c);
                if (lx / rx).powi(2) + (ly / ry).powi(2) <= 1.0 {
                    m[y as usize * self.w + x as usize] = true;
                }
            }
        }
        m
    }
    pub(crate) fn fill_ellipse_masked(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, col: Rgba, mask: &[bool]) {
        let ex = (rx * rot.cos()).abs() + (ry * rot.sin()).abs();
        let ey = (rx * rot.sin()).abs() + (ry * rot.cos()).abs();
        let (x0, y0, x1, y1) = self.bbox(cx - ex - 1.0, cy - ey - 1.0, cx + ex + 1.0, cy + ey + 1.0);
        let dist = |qx: f32, qy: f32| ellipse_sd(cx, cy, rx, ry, rot, qx, qy);
        for y in y0..=y1 {
            for x in x0..=x1 {
                if !mask[y as usize * self.w + x as usize] {
                    continue;
                }
                let cov = cov_area(&dist, x as f32 + 0.5, y as f32 + 0.5);
                if cov > 0.0 {
                    self.blend_cov(x, y, col, cov);
                }
            }
        }
    }
    /// Axis-aligned rect fill, coverage-AA via the box SDF.
    pub(crate) fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, col: Rgba) {
        let (x0, y0, x1, y1) = self.bbox(x - 1.0, y - 1.0, x + w + 1.0, y + h + 1.0);
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let (hw, hh) = (w / 2.0, h / 2.0);
        let dist = |qx: f32, qy: f32| {
            let (dx, dy) = ((qx - cx).abs() - hw, (qy - cy).abs() - hh);
            dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0)
        };
        for yy in y0..=y1 {
            for xx in x0..=x1 {
                let cov = cov_area(&dist, xx as f32 + 0.5, yy as f32 + 0.5);
                if cov > 0.0 {
                    self.blend_cov(xx, yy, col, cov);
                }
            }
        }
    }
}

/// Approximate signed distance to an ellipse boundary (negative
/// inside): the implicit function g = sqrt((lx/rx)^2 + (ly/ry)^2),
/// normalised by its gradient magnitude — exact for circles, a close
/// estimate for the production eccentricities (head 0.97, pupils).
pub(crate) fn ellipse_sd(cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, x: f32, y: f32) -> f32 {
    let (s, c) = rot.sin_cos();
    let (dx, dy) = (x - cx, y - cy);
    let (lx, ly) = (dx * c + dy * s, -dx * s + dy * c);
    let g = ((lx / rx).powi(2) + (ly / ry).powi(2)).sqrt();
    if g == 0.0 {
        return -rx.min(ry);
    }
    let grad = (((lx / (rx * rx)).powi(2) + (ly / (ry * ry)).powi(2)).sqrt()) / g;
    (g - 1.0) / grad
}

/// Distance from a point to a segment.
pub(crate) fn seg_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    seg_dist_t(px, py, ax, ay, bx, by).0
}

/// Distance from a point to a segment, plus the clamped parameter
/// t of the closest point (0 at a, 1 at b).
pub(crate) fn seg_dist_t(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> (f32, f32) {
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    ((px - (ax + dx * t)).hypot(py - (ay + dy * t)), t)
}

/// A round line cap the way Skia actually draws it: the cap arc
/// is flattened into equal chords whose sagitta stays within the
/// rasteriser's flattening tolerance (0.25 device px), laid from
/// the arc's start at the +normal vertex. At R = 15 device px
/// that is 9 chords and the tip chord sits at R·cos(10°) ≈ 0.23 px
/// inside the true semicircle — the stroke battery's cap-end
/// class measures exactly this (reference axial edge 14.33 px at
/// R 15, where the exact semicircle gives 14.5). An exact-disc
/// cap is therefore measurably wrong against Skia, by ~0.3 px at
/// the tip.
pub(crate) struct CapFan {
    c: (f32, f32),    // cap centre (the terminal vertex)
    axis: (f32, f32), // unit outward direction
    r: f32,           // cap radius (= half the stroke width)
    n: usize,         // chord count over the semicircle
    cos_half: f32,    // cos(pi / (2n))
}

pub(crate) fn cap_fan(c: (f32, f32), axis: (f32, f32), r: f32) -> CapFan {
    let cos_max = (1.0f32 - 0.25 / r).clamp(-1.0, 1.0);
    let n = ((std::f32::consts::PI / (2.0 * cos_max.acos())).ceil() as usize).max(1);
    CapFan {
        c,
        axis,
        r,
        n,
        cos_half: (std::f32::consts::PI / (2.0 * n as f32)).cos(),
    }
}

/// Signed distance to the flattened cap boundary (radial model:
/// exact at chord vertices and chord midpoints, within ~0.05 px
/// elsewhere at production radii).
pub(crate) fn fan_sd(f: &CapFan, qx: f32, qy: f32) -> f32 {
    let (vx, vy) = (qx - f.c.0, qy - f.c.1);
    let r = vx.hypot(vy);
    if r < 1e-6 {
        return -f.r * f.cos_half;
    }
    let cosang = ((vx * f.axis.0 + vy * f.axis.1) / r).clamp(-1.0, 1.0);
    if cosang < 0.0 {
        return r - f.r; // behind the cap plane: the plain disc term
    }
    let phi = cosang.acos(); // 0..=pi/2 from the outward axis
    let dth = std::f32::consts::PI / f.n as f32;
    // Vertices at phi_k = pi/2 - k*dth, laid from the arc start.
    let k = (((std::f32::consts::FRAC_PI_2 - phi) / dth).floor() as i32).clamp(0, f.n as i32 - 1) as usize;
    let phi_mid = std::f32::consts::FRAC_PI_2 - (k as f32 + 0.5) * dth;
    let rb = f.r * f.cos_half / (phi - phi_mid).cos();
    r - rb
}

/// Area coverage of a pixel from a signed-distance closure.
/// Skia's coverage is the pixel FOOTPRINT integral of the shape;
/// the centre-distance ramp (cov_of) equals it only for straight
/// edges — wherever the boundary curves at the pixel scale the
/// centre sample misstates the covered fraction (the stroke
/// battery's cap-end class is the measured case: centre-distance
/// ramp 0.81 px vs Skia's 1.47 px at a 15-unit round cap).
/// Inside the band |sd| < 1 the integral is therefore estimated
/// directly: a 4x4 binary supersample of the inside test.
/// (Averaging cov_of over sub-points instead would convolve the
/// ramp with itself and widen even straight edges — measured
/// 1.41 px against Skia's 0.8 px on the battery's segment rows.)
pub(crate) fn cov_area<F: Fn(f32, f32) -> f32>(dist: &F, x: f32, y: f32) -> f32 {
    cov_area0(dist, x, y, dist(x, y))
}

/// cov_area with the centre distance already computed.
pub(crate) fn cov_area0<F: Fn(f32, f32) -> f32>(dist: &F, x: f32, y: f32, d0: f32) -> f32 {
    if d0.abs() < 1.0 {
        let mut inside = 0u32;
        for oy in [-0.375f32, -0.125, 0.125, 0.375] {
            for ox in [-0.375f32, -0.125, 0.125, 0.375] {
                if dist(x + ox, y + oy) < 0.0 {
                    inside += 1;
                }
            }
        }
        inside as f32 / 16.0
    } else {
        Painter::cov_of(d0)
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
// The character painter: drawToon re-implemented over the CPU
// Painter for the look/state subset the A/B work exercises. This
// began as §28's paint_onoda (the Onoda S01 card, hard-coded) and is
// generalised here for the M4 stroke battery: same geometry code,
// parameters instead of constants, and the drawFace mood branches the
// battery's face set needs (toon.ts drawFace / eye / openMouth).
// Stage space is 512x1024 with the figure's feet at stage y = 1000;
// the canvas is 1280x1408 with the stage at offset (384, 256),
// painted at res 2 — the production canvas (a3 §2).
// ---------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ToonMood {
    Neutral,
    Determined,
    Angry,
    Laugh,
    Shock,
}

pub(crate) struct ToonSpec {
    pub(crate) body: &'static str,
    pub(crate) head: &'static str,
    /// look.shade; None = shadeOf(body, 0.8), as in drawToonLayer.
    pub(crate) shade: Option<&'static str>,
    /// hatCol when the look wears the newsboy cap; None = hat 'none'.
    pub(crate) cap: Option<&'static str>,
    pub(crate) mood: ToonMood,
    pub(crate) arm_l: (f32, f32),
    pub(crate) arm_r: (f32, f32),
    pub(crate) lean_deg: f32,
    pub(crate) head_turn: f32,
    pub(crate) cam_pitch: f32,
}

/// Rounded-rect outline points (stage space), corners flattened with
/// `seg` segments each — the native stand-in for ctx.roundRect.
fn rounded_rect_pts(x: f32, y: f32, w: f32, h: f32, r: f32, seg: usize) -> Vec<(f32, f32)> {
    let mut pts: Vec<(f32, f32)> = Vec::new();
    let corners = [
        (x + w - r, y + r, -std::f32::consts::FRAC_PI_2),
        (x + w - r, y + h - r, 0.0),
        (x + r, y + h - r, std::f32::consts::FRAC_PI_2),
        (x + r, y + r, std::f32::consts::PI),
    ];
    for (cx, cy, a0) in corners {
        for i in 0..=seg {
            let a = a0 + (i as f32 / seg as f32) * std::f32::consts::FRAC_PI_2;
            pts.push((cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    pts
}

pub(crate) fn paint_toon(spec: &ToonSpec) -> (Vec<u8>, u32, u32) {
    let res = 2.0f32;
    let (cw, ch) = (1280usize * 2, 1408usize * 2);
    let mut p = Painter::new(cw, ch);
    // Stage -> canvas pixels: translate by (OX, OY), scale by res,
    // after the whole-figure lean rotation about the feet (256, 1000)
    // that drawToonLayer applies (rotate(+lean deg), canvas space).
    let th = spec.lean_deg.to_radians();
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
    p.fill_polygon(&bean, rgba_of(spec.body));
    let mask = p.polygon_mask(&bean);
    // Clipped shade (look.shade, alpha 0.85) + highlight.
    let (shx, shy) = px(bx + bw + 40.0, (top + bot) / 2.0 + 20.0);
    let (sr, sg, sb) = match spec.shade {
        Some(s) => hex_rgb(s),
        None => shade_rgb(spec.body, 0.8),
    };
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
    arm(&mut p, -1.0, spec.arm_l.0, spec.arm_l.1);
    arm(&mut p, 1.0, spec.arm_r.0, spec.arm_r.1);

    // ---- head ----
    let (hx, hy, r) = (256.0f32, 178.0f32, 138.0f32);
    let rp = r * (1.0 + 0.04 * spec.cam_pitch);
    let (hxp, hyp) = px(hx, hy);
    p.fill_ellipse(hxp, hyp, rp * res, rp * 0.97 * res, th, rgba_of(spec.head));
    // Even-odd shade: head area NOT covered by the offset circle.
    {
        let (ccx, ccy) = (hx + 52.0 + spec.head_turn * 30.0, hy + 44.0);
        let (ccxp, ccyp) = px(ccx, ccy);
        let shade = shade_rgb(spec.head, 0.9);
        let col = (shade.0, shade.1, shade.2, 255u8);
        let (x0, y0, x1, y1) = p.bbox(hxp - rp * res - 1.0, hyp - rp * res - 1.0, hxp + rp * res + 1.0, hyp + rp * res + 1.0);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (pxx, pyy) = (x as f32 + 0.5, y as f32 + 0.5);
                // Coverage form of the even-odd trick: inside the
                // head ellipse AND outside the offset circle.
                let cov_head = Painter::cov_of(ellipse_sd(hxp, hyp, rp * res, rp * 0.97 * res, th, pxx, pyy));
                if cov_head <= 0.0 {
                    continue;
                }
                let unshaded = ((pxx - ccxp).hypot(pyy - ccyp) - rp * res + 0.5).clamp(0.0, 1.0);
                let cov = cov_head * unshaded;
                if cov > 0.0 {
                    p.blend_cov(x, y, col, cov);
                }
            }
        }
    }
    p.stroke_ellipse(hxp, hyp, rp * res, rp * 0.97 * res, th, lw(10.0), ink);

    // ---- face (drawFace subset: the battery's five moods) ----
    let fx = hx + spec.head_turn * 30.0;
    let ex = 46.0 * (1.0 - 0.18 * spec.head_turn.abs());
    let ey = hy + 4.0 + spec.cam_pitch * 8.0;
    // Brow table [lift, tilt] per mood (toon.ts drawFace); tilt is
    // signed by side in the loop.
    let (lift, tilt_mag) = match spec.mood {
        ToonMood::Neutral => (4.0f32, 0.0f32),
        ToonMood::Determined => (-4.0, 16.0),
        ToonMood::Angry => (-10.0, 22.0),
        ToonMood::Laugh => (10.0, 0.0),
        ToonMood::Shock => (24.0, 0.0),
    };
    for s in [-1.0f32, 1.0] {
        let tilt = tilt_mag * s;
        let y = ey - 44.0 - lift;
        p.stroke_quad(
            px(fx + s * (ex - 26.0), y + tilt * 0.5),
            px(fx + s * ex, y - 7.0),
            px(fx + s * (ex + 26.0), y - tilt * 0.5),
            lw(12.0),
            ink,
        );
    }
    // Eyes (toon.ts eye()).
    for s in [-1.0f32, 1.0] {
        let x = fx + s * ex;
        match spec.mood {
            ToonMood::Laugh => {
                // Happy/laugh eye: arc(x, y+10, 20, 1.15pi, 1.85pi),
                // lineWidth 10 — flattened here, stroked as polyline.
                let pts: Vec<(f32, f32)> = (0..=24)
                    .map(|i| {
                        let a = std::f32::consts::PI * 1.15
                            + (i as f32 / 24.0) * std::f32::consts::PI * 0.7;
                        px(x + 20.0 * a.cos(), ey + 10.0 + 20.0 * a.sin())
                    })
                    .collect();
                p.stroke_polyline(&pts, lw(10.0), ink);
            }
            ToonMood::Shock => {
                let (exp, eyp) = px(x, ey);
                p.fill_ellipse(exp, eyp, 26.0 * res, 30.0 * res, th, rgba_of("#FFFFFF"));
                p.stroke_ellipse(exp, eyp, 26.0 * res, 30.0 * res, th, lw(6.0), ink);
                p.fill_ellipse(exp, eyp, 7.0 * res, 7.0 * res, th, ink);
            }
            _ => {
                let (exp, eyp) = px(x, ey);
                p.fill_ellipse(exp, eyp, 13.0 * res, 17.0 * res, th, ink);
                if spec.mood == ToonMood::Determined {
                    // Heavy lid: head-colour rect + lid line (width 7).
                    let (rx0, ry0) = px(x - 24.0, ey - 28.0);
                    p.fill_rect(rx0, ry0, 48.0 * res, 26.0 * res, rgba_of(spec.head));
                    p.stroke_polyline(&[px(x - 20.0, ey - 2.0), px(x + 20.0, ey - 2.0)], lw(7.0), ink);
                }
            }
        }
    }
    if spec.mood == ToonMood::Laugh {
        // Blush: rgba(240,120,130,0.35) ellipses 22x12 at fx+/-78.
        let blush = (240u8, 120u8, 130u8, (0.35 * 255.0) as u8);
        for s in [-1.0f32, 1.0] {
            let (bxp, byp) = px(fx + s * 78.0, ey + 40.0);
            p.fill_ellipse(bxp, byp, 22.0 * res, 12.0 * res, th, blush);
        }
    }
    // Mouth (my = ey + 62; lineWidth 8 unless a branch says otherwise).
    let my = ey + 62.0;
    match spec.mood {
        ToonMood::Determined => {
            p.stroke_polyline(&[px(fx - 24.0, my + 5.0), px(fx + 24.0, my + 1.0)], lw(8.0), ink);
        }
        ToonMood::Neutral => {
            p.stroke_quad(px(fx - 20.0, my + 2.0), px(fx, my + 14.0), px(fx + 20.0, my + 2.0), lw(8.0), ink);
        }
        ToonMood::Angry => {
            p.stroke_quad(px(fx - 30.0, my + 12.0), px(fx, my - 8.0), px(fx + 30.0, my + 12.0), lw(8.0), ink);
            // Teeth: roundRect(fx-24, my-2, 48, 14, 6), white fill +
            // ink stroke at lineWidth 5.
            let pts: Vec<(f32, f32)> = rounded_rect_pts(fx - 24.0, my - 2.0, 48.0, 14.0, 6.0, 6)
                .iter()
                .map(|&(x, y)| px(x, y))
                .collect();
            p.fill_polygon(&pts, rgba_of("#FFFFFF"));
            let mut ring = pts.clone();
            ring.push(pts[0]);
            p.stroke_polyline(&ring, lw(5.0), ink);
        }
        ToonMood::Laugh => {
            // Closed laugh path: three quads (toon.ts drawFace).
            let mut pts: Vec<(f32, f32)> = Vec::new();
            let mut chain = |qs: Vec<(f32, f32)>| {
                if pts.is_empty() {
                    pts.extend_from_slice(&qs);
                } else {
                    pts.extend_from_slice(&qs[1..]);
                }
            };
            chain(flatten_quad(px(fx - 40.0, my - 6.0), px(fx, my - 2.0), px(fx + 40.0, my - 6.0), 24));
            chain(flatten_quad(px(fx + 40.0, my - 6.0), px(fx + 34.0, my + 52.0), px(fx, my + 52.0), 24));
            chain(flatten_quad(px(fx, my + 52.0), px(fx - 34.0, my + 52.0), px(fx - 40.0, my - 6.0), 24));
            p.fill_polygon(&pts, rgba_of("#3A1A2A"));
            let m = p.polygon_mask(&pts);
            // Tongue, clipped to the mouth: ellipse(fx, my+50, 24, 18).
            let (txp, typ) = px(fx, my + 50.0);
            p.fill_ellipse_masked(txp, typ, 24.0 * res, 18.0 * res, th, rgba_of("#E2697A"), &m);
            // Teeth: fillRect(fx-40, my-10, 80, 12) white, same clip.
            // (Axis-aligned in device space: exact at lean 0, which is
            // the battery's face configuration.)
            let (ax, ay) = px(fx - 40.0, my - 10.0);
            let (bxp, byp) = px(fx + 40.0, my + 2.0);
            let (x0, y0, x1, y1) = p.bbox(ax.min(bxp), ay.min(byp), ax.max(bxp), ay.max(byp));
            for yy in y0..=y1 {
                for xx in x0..=x1 {
                    if m[yy as usize * p.w + xx as usize] {
                        p.blend(xx, yy, rgba_of("#FFFFFF"));
                    }
                }
            }
            // Canvas2D path-state quirk, reproduced faithfully:
            // the branch's final g.stroke() runs AFTER the tongue's
            // beginPath(), so it strokes the TONGUE ELLIPSE (lineWidth
            // 8), not the mouth outline — the production laugh mouth
            // has no outline; the tongue carries an ink ring instead.
            // (Found by the M4 stroke battery: docs/ab/stroke-battery.)
            p.stroke_ellipse(txp, typ, 24.0 * res, 18.0 * res, th, lw(8.0), ink);
        }
        ToonMood::Shock => {
            // openMouth(16, 20): ellipse(fx, my + h*0.3, w, h) filled
            // #3A1A2A and stroked; tongue clipped inside (h > 12).
            let (mxp, myp) = px(fx, my + 6.0);
            p.fill_ellipse(mxp, myp, 16.0 * res, 20.0 * res, th, rgba_of("#3A1A2A"));
            p.stroke_ellipse(mxp, myp, 16.0 * res, 20.0 * res, th, lw(8.0), ink);
            let m = p.ellipse_mask(mxp, myp, 16.0 * res, 20.0 * res, th);
            let (txp, typ) = px(fx, my + 22.0);
            p.fill_ellipse_masked(txp, typ, 11.2 * res, 12.0 * res, th, rgba_of("#E2697A"), &m);
        }
    }

    // ---- cap ('cap'), hat centre lags turn*18 ----
    if let Some(cap_hex) = spec.cap {
        let hatx = hx + spec.head_turn * 18.0;
        let htop = hy - rp * 0.97;
        let cap = rgba_of(cap_hex);
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
            // The real path is closePath()'d before fill+stroke:
            // the crown's chord is stroked too (the brim's is not —
            // no closePath there).
            let mut ring = pts.clone();
            ring.push(pts[0]);
            p.stroke_polyline(&ring, lw(9.0), ink);
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
            let sh = shade_rgb(cap_hex, 0.75);
            p.fill_polygon(&pts, (sh.0, sh.1, sh.2, 255));
            p.stroke_polyline(&pts, lw(9.0), ink);
        }
        // Button: shadeOf(cap, 1.15).
        {
            let (bcx, bcy) = px(hatx + 6.0, htop - 26.0);
            let sh = shade_rgb(cap_hex, 1.15);
            p.fill_ellipse(bcx, bcy, 12.0 * res, 12.0 * res, 0.0, (sh.0, sh.1, sh.2, 255));
            p.stroke_ellipse(bcx, bcy, 12.0 * res, 12.0 * res, 0.0, lw(9.0), ink);
        }
    }

    (p.px, cw as u32, ch as u32)
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

// ---------------------------------------------------------------------
// A small JSON parser: the subset the scene format needs (objects,
// arrays, strings with the standard escapes, numbers, true/false/
// null). Numbers are f64; the format loader casts per field, the
// same way the hand-authored code's literals land in f32/f64 slots.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(Vec<(String, Value)>),
}

impl Value {
    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub(crate) fn as_arr(&self) -> Option<&[Value]> {
        match self {
            Value::Arr(items) => Some(items),
            _ => None,
        }
    }
    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub(crate) fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }
}

pub(crate) fn parse_json(text: &str) -> Result<Value, String> {
    let mut p = JsonParser { bytes: text.as_bytes(), pos: 0 };
    p.skip_ws();
    let v = p.parse_value()?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(format!("json: trailing input at byte {}", p.pos));
    }
    Ok(v)
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\n' | b'\r') {
            self.pos += 1;
        }
    }
    fn peek(&self) -> Result<u8, String> {
        self.bytes.get(self.pos).copied().ok_or_else(|| "json: unexpected end of input".to_string())
    }
    fn parse_value(&mut self) -> Result<Value, String> {
        match self.peek()? {
            b'{' => self.parse_obj(),
            b'[' => self.parse_arr(),
            b'"' => Ok(Value::Str(self.parse_string()?)),
            b't' => self.parse_lit("true", Value::Bool(true)),
            b'f' => self.parse_lit("false", Value::Bool(false)),
            b'n' => self.parse_lit("null", Value::Null),
            _ => self.parse_number(),
        }
    }
    fn parse_lit(&mut self, lit: &str, val: Value) -> Result<Value, String> {
        if self.bytes[self.pos..].starts_with(lit.as_bytes()) {
            self.pos += lit.len();
            Ok(val)
        } else {
            Err(format!("json: bad literal at byte {}", self.pos))
        }
    }
    fn parse_obj(&mut self) -> Result<Value, String> {
        self.pos += 1; // '{'
        let mut pairs = Vec::new();
        self.skip_ws();
        if self.peek()? == b'}' {
            self.pos += 1;
            return Ok(Value::Obj(pairs));
        }
        loop {
            self.skip_ws();
            if self.peek()? != b'"' {
                return Err(format!("json: expected object key at byte {}", self.pos));
            }
            let key = self.parse_string()?;
            self.skip_ws();
            if self.peek()? != b':' {
                return Err(format!("json: expected ':' at byte {}", self.pos));
            }
            self.pos += 1;
            self.skip_ws();
            let val = self.parse_value()?;
            pairs.push((key, val));
            self.skip_ws();
            match self.peek()? {
                b',' => {
                    self.pos += 1;
                }
                b'}' => {
                    self.pos += 1;
                    return Ok(Value::Obj(pairs));
                }
                _ => return Err(format!("json: expected ',' or '}}' at byte {}", self.pos)),
            }
        }
    }
    fn parse_arr(&mut self) -> Result<Value, String> {
        self.pos += 1; // '['
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek()? == b']' {
            self.pos += 1;
            return Ok(Value::Arr(items));
        }
        loop {
            self.skip_ws();
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.peek()? {
                b',' => {
                    self.pos += 1;
                }
                b']' => {
                    self.pos += 1;
                    return Ok(Value::Arr(items));
                }
                _ => return Err(format!("json: expected ',' or ']' at byte {}", self.pos)),
            }
        }
    }
    fn parse_string(&mut self) -> Result<String, String> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            let b = self.peek()?;
            self.pos += 1;
            match b {
                b'"' => return Ok(out),
                b'\\' => {
                    let e = self.peek()?;
                    self.pos += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000C}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.parse_hex4()?;
                            if (0xD800..0xDC00).contains(&hi) {
                                // High surrogate: expect a low one.
                                if self.peek()? == b'\\' {
                                    self.pos += 1;
                                    if self.peek()? == b'u' {
                                        self.pos += 1;
                                        let lo = self.parse_hex4()?;
                                        let c = 0x10000 + ((hi - 0xD800) << 10) + (lo.wrapping_sub(0xDC00) & 0x3FF);
                                        out.push(char::from_u32(c).unwrap_or('\u{FFFD}'));
                                        continue;
                                    }
                                }
                                out.push('\u{FFFD}');
                            } else {
                                out.push(char::from_u32(hi).unwrap_or('\u{FFFD}'));
                            }
                        }
                        _ => return Err(format!("json: bad escape at byte {}", self.pos)),
                    }
                }
                _ => {
                    // Raw byte: copy the full UTF-8 sequence verbatim.
                    let start = self.pos - 1;
                    let mut end = self.pos;
                    if b >= 0x80 {
                        while end < self.bytes.len() && (self.bytes[end] & 0xC0) == 0x80 {
                            end += 1;
                        }
                    }
                    match std::str::from_utf8(&self.bytes[start..end]) {
                        Ok(s) => out.push_str(s),
                        Err(_) => return Err(format!("json: invalid utf-8 at byte {start}")),
                    }
                    self.pos = end;
                }
            }
        }
    }
    fn parse_hex4(&mut self) -> Result<u32, String> {
        if self.pos + 4 > self.bytes.len() {
            return Err("json: truncated \\u escape".to_string());
        }
        let s = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
            .map_err(|_| "json: bad \\u escape".to_string())?;
        let v = u32::from_str_radix(s, 16).map_err(|_| "json: bad \\u escape".to_string())?;
        self.pos += 4;
        Ok(v)
    }
    fn parse_number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        while self.pos < self.bytes.len()
            && matches!(self.bytes[self.pos], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E')
        {
            self.pos += 1;
        }
        if start == self.pos {
            return Err(format!("json: expected a value at byte {start}"));
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap_or("");
        s.parse::<f64>()
            .map(Value::Num)
            .map_err(|_| format!("json: bad number '{s}' at byte {start}"))
    }
}

#[cfg(test)]
mod json_tests {
    use super::*;

    #[test]
    fn parses_all_value_kinds() {
        let v = parse_json(r#"{"a": [1, -2.5e2, true, false, null, "x\nA"], "b": {}, "c": []}"#)
            .expect("parse");
        let a = v.get("a").and_then(Value::as_arr).expect("a is array");
        assert_eq!(a[0].as_f64(), Some(1.0));
        assert_eq!(a[1].as_f64(), Some(-250.0));
        assert_eq!(a[2], Value::Bool(true));
        assert_eq!(a[3], Value::Bool(false));
        assert_eq!(a[4], Value::Null);
        assert_eq!(a[5].as_str(), Some("x\nA"));
        assert_eq!(v.get("b"), Some(&Value::Obj(vec![])));
        assert_eq!(v.get("c"), Some(&Value::Arr(vec![])));
        assert_eq!(v.get("missing"), None);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_json("{").is_err());
        assert!(parse_json("[1,]").is_err());
        assert!(parse_json("{\"a\": }").is_err());
        assert!(parse_json("1 2").is_err());
        assert!(parse_json("").is_err());
    }
}
