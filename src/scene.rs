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

/// world.ts shade(hex, k): THREE.Color arithmetic — setHex
/// converts to the LINEAR working space, multiplyScalar applies
/// there, getHex quantises back to sRGB bytes (Math.round,
/// clamped). The previous byte-space multiply ran 25-40 LSB
/// darker than Chrome on derived shades (three wave seats
/// confirmed independently; corpus pins: shade('#3A2E22', 1.25)
/// = #413427, shade('#3A2A2A', 0.7) = #302222).
pub(crate) fn shade_rgb(s: &str, k: f32) -> (u8, u8, u8) {
    let (r, g, b) = hex_rgb(s);
    let f = |v: u8| -> u8 {
        let c = (v as f64) / 255.0;
        let lin = if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
        let m = lin * (k as f64);
        let sr = if m <= 0.0031308 { m * 12.92 } else { 1.055 * m.powf(1.0 / 2.4) - 0.055 };
        (255.0 * sr).clamp(0.0, 255.0).round() as u8
    };
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
pub(crate) fn rounded_rect_pts(x: f32, y: f32, w: f32, h: f32, r: f32, seg: usize) -> Vec<(f32, f32)> {
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
pub(crate) fn paint_sky(top_hex: &str, horizon_hex: &str, ground_hex: &str) -> (Vec<u8>, u32, u32) {
    // The source dome is a ShaderMaterial (world.ts sky()):
    //   y > 0: mix(hor, top, pow(clamp(y*1.6, 0, 1), 0.7));  y <= 0: gnd
    // with y = normalize(position).y. On the UV sphere the polar angle
    // is linear in v (v = 1 - theta/pi), so the direction's y at a
    // fragment is cos(pi*(1-v)) — NOT linear in v. Rows below the
    // horizon carry the flat ground colour.
    let (w, h) = (2usize, 512usize);
    let top = hex_lin(top_hex);
    let hor = hex_lin(horizon_hex);
    let gnd = hex_lin(ground_hex);
    let mut px = vec![0u8; w * h * 4];
    for j in 0..h {
        // wgpu samples memory row j at v = (j + 0.5) / h, and the
        // dome's uv has v = 1 at the top pole.
        let v = (j as f32 + 0.5) / h as f32;
        let ny = (std::f32::consts::PI * (1.0 - v)).cos();
        let col = if ny > 0.0 {
            let t = (ny * 1.6).clamp(0.0, 1.0).powf(0.7);
            [
                hor[0] + (top[0] - hor[0]) * t,
                hor[1] + (top[1] - hor[1]) * t,
                hor[2] + (top[2] - hor[2]) * t,
            ]
        } else {
            gnd
        };
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

// ===== registry painter: planks (wave pack, painters/planks.rs) =====
// ---------------------------------------------------------------------
// paint_planks — native port of the world.ts `T.planks` canvas painter
// (the film project's own code, NOT three.js library code):
//
//   ~/faceless-shorts/the-soldier-who-didnt-know/src/core/world.ts:58-65
//     planks: (col = '#A8693E', rep = [1, 1]) => canvasTex(
//       'planks' + col, 512, 512, (g, w, h) => { ... }, rep)
//   helpers it closes over:
//     rnd   — world.ts:10 (== lib.ts rnd; == crate::scene::rnd)
//     shade — world.ts:41 (THREE.Color multiplyScalar; see planks_shade)
//
// INTEGRATION (this file is a drop-in for src/scene.rs):
//   * Paste the items below into src/scene.rs next to paint_grass
//     (they use that module's Painter / Rgba / hex_rgb / rnd
//     unqualified, exactly like the sibling painters), OR include as a
//     submodule with `use super::{hex_rgb, rnd, Painter, Rgba};`.
//   * Register in src/scenefile.rs resolve_texture(), beside "grass":
//       "planks" => scene::paint_planks(str_field(spec, "base", ctx)?),
//   * Signature, size and determinism conventions mirror paint_grass
//     EXACTLY: `pub(crate) fn paint_*(base: &str) -> (Vec<u8>, u32,
//     u32)`, fixed 512x512 RGBA canvas, straight alpha, row 0 = top,
//     all randomness from the f64 sin-hash rnd() — no RNG state, so
//     two calls with the same base are byte-identical.
//   * `rep` (repeat) is NOT a painter input, in either engine: the
//     source applies it to the texture CLONE (canvasTex, world.ts:
//     24-33) while one canvas per colour is cached and shared. Repeat
//     and rotation live on the scene file's texture entry / material
//     UV transform. See painters/planks.md for the full mapping.
//
// Op-for-op transcription of the draw callback (world.ts:59-64).
// Canvas state carried across ops, as in the source: the joint rect
// reuses the seam's fillStyle (shade 0.45) because the source sets
// fillStyle once and never resets it between the two fillRects; the
// grain strokes' globalAlpha 0.35 is folded into the stroke colour's
// alpha byte (89), which is exactly how Painter::blend consumes it.
// ---------------------------------------------------------------------

/// three.js r170 `shade()` (world.ts:41):
///   const shade = (hex, k) => { const c = new THREE.Color(hex);
///     c.multiplyScalar(k); return '#' + c.getHexString(); };
/// With r170's default ColorManagement, `new THREE.Color(hex)` stores
/// the colour in the LINEAR-srgb working space, multiplyScalar scales
/// the linear components, and getHexString() converts back to sRGB
/// bytes. The conversions below are three's own constants
/// (ColorManagement.js: SRGBToLinear uses 0.0773993808 /
/// 0.9478672986 / 0.0521327014 with exponent 2.4; LinearToSRGB uses
/// 12.92 / 1.055 / exponent 0.41666), evaluated in f64 like all JS
/// number math, with the final byte ROUNDED, not truncated: the two
/// shade() outputs the film corpus recorded from the real pipeline
/// pin this — shade('#3A2A2A', 0.7) = #302222 and
/// shade('#3A2E22', 1.25) = #413427, whose green byte is 52 =
/// round(51.8); truncation would give 51 (#413327). See the tests.
///
/// This is deliberately NOT crate::scene::shade_rgb: that helper
/// multiplies the sRGB BYTES (it ports toon.ts's canvas shadeOf) and
/// runs 25-40 LSB darker on this painter's factors — e.g.
/// shade('#A8693E', 0.45) is (116, 71, 40) here vs (76, 47, 28) from
/// shade_rgb. The M0 audit (docs/audit/a1-engine.md, the
/// THREE.Color row) already flagged exactly this: "Reproduce that or
/// every derived shade shifts." paint_grass / film.rs still use
/// shade_rgb for world.ts painters — an inherited inconsistency in
/// the registry, documented in painters/planks.md; this module
/// follows the source.
fn planks_shade(hex: &str, k: f64) -> (u8, u8, u8) {
    fn s2l(c: f64) -> f64 {
        if c <= 0.04045 {
            c * 0.0773993808
        } else {
            (c * 0.9478672986 + 0.0521327014).powf(2.4)
        }
    }
    fn l2s(c: f64) -> f64 {
        if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(0.41666) - 0.055
        }
    }
    let (r, g, b) = hex_rgb(hex);
    let f = |v: u8| -> u8 { (l2s(s2l(v as f64 / 255.0) * k) * 255.0).round().clamp(0.0, 255.0) as u8 };
    (f(r), f(g), f(b))
}

fn planks_shade_rgba(hex: &str, k: f64) -> Rgba {
    let (r, g, b) = planks_shade(hex, k);
    (r, g, b, 255)
}

/// One grain curve (world.ts:61): the cubic bezier
///   moveTo(0, y)  bezierCurveTo(w*0.3, y+6,  w*0.6, y-6,  w, y+2)
/// flattened to a polyline for Painter::stroke_polyline. 128 segments
/// keep the polyline within ~0.01 px of the true curve everywhere
/// (the curve's total y excursion is 12 px over a 512 px run), far
/// under the stroke's own AA band. Canvas strokes this path with
/// butt caps; stroke_polyline's house model uses round caps — the
/// curves span the full canvas width, so the cap difference is
/// confined to the first/last pixel columns (see planks.md).
fn planks_grain_pts(w: f32, y: f32) -> Vec<(f32, f32)> {
    const SEG: usize = 128;
    let (x0, y0) = (0.0f64, y as f64);
    let (x1, y1) = (w as f64 * 0.3, y as f64 + 6.0);
    let (x2, y2) = (w as f64 * 0.6, y as f64 - 6.0);
    let (x3, y3) = (w as f64, y as f64 + 2.0);
    (0..=SEG)
        .map(|s| {
            let t = s as f64 / SEG as f64;
            let (u, v) = (1.0 - t, t);
            let px = u * u * u * x0 + 3.0 * u * u * v * x1 + 3.0 * u * v * v * x2 + v * v * v * x3;
            let py = u * u * u * y0 + 3.0 * u * u * v * y1 + 3.0 * u * v * v * y2 + v * v * v * y3;
            (px as f32, py as f32)
        })
        .collect()
}

/// T.planks(col): 512x512 canvas, 8 plank rows. Per row i
/// (world.ts:60-64), in source order:
///   1. row fill      shade(col, 0.86 + rnd(i*3)*0.24), full row
///   2. 4 grain strokes, shade(col, 0.7) @ alpha 0.35, lineWidth 2,
///      y = i*ph + 8 + rnd(i*7+k)*(ph-16), k = 0..4 — four SEPARATE
///      canvas strokes, so four separate stroke_polyline calls here
///      (crossings blend twice, as in the source)
///   3. seam band     shade(col, 0.45), fillRect(0, i*ph, w, 4)
///   4. butt joint    SAME fillStyle (0.45), fillRect(jx, i*ph, 4, ph)
///      with jx = rnd(i*11)*w
///   5. two knots     shade(col, 0.5), filled circles r = 3 at
///      (jx-14, i*ph + ph/2) and (jx+18, i*ph + ph/2)
///      (source: arc(x, y, 3, 0, 7) — 7 rad > 2*PI, a full circle)
pub(crate) fn paint_planks(base: &str) -> (Vec<u8>, u32, u32) {
    let (w, h) = (512usize, 512usize);
    let mut p = Painter::new(w, h);
    let n = 8usize;
    let ph = h as f32 / n as f32; // 64
    // globalAlpha 0.35 -> alpha byte: round(0.35 * 255) = 89.
    let (gr, gg, gb) = planks_shade(base, 0.7);
    let grain: Rgba = (gr, gg, gb, 89);
    let seam = planks_shade_rgba(base, 0.45);
    let knot = planks_shade_rgba(base, 0.5);
    for i in 0..n {
        let fi = i as f64;
        let top = i as f32 * ph;
        // 1. row fill
        let f = 0.86 + rnd(fi * 3.0) * 0.24;
        let fill = planks_shade_rgba(base, f);
        p.fill_rect(0.0, top, w as f32, ph, fill);
        // 2. grain strokes
        for k in 0..4usize {
            let y = top + 8.0 + (rnd(fi * 7.0 + k as f64) * (ph as f64 - 16.0)) as f32;
            let pts = planks_grain_pts(w as f32, y);
            p.stroke_polyline(&pts, 2.0, grain);
        }
        // 3. seam band (top 4 px of the row)
        p.fill_rect(0.0, top, w as f32, 4.0, seam);
        // 4. butt joint (same fillStyle as the seam in the source)
        let jx = (rnd(fi * 11.0) * w as f64) as f32;
        p.fill_rect(jx, top, 4.0, ph, seam);
        // 5. knots
        for cx in [jx - 14.0, jx + 18.0] {
            p.fill_ellipse(cx, top + ph / 2.0, 3.0, 3.0, 0.0, knot);
        }
    }
    (p.px, w as u32, h as u32)
}

#[cfg(test)]
mod planks_tests {
    use super::*;

    fn px_at(px: &[u8], w: usize, x: usize, y: usize) -> (u8, u8, u8, u8) {
        let k = (y * w + x) * 4;
        (px[k], px[k + 1], px[k + 2], px[k + 3])
    }

    #[test]
    fn output_size_and_opaque() {
        let (px, w, h) = paint_planks("#A8693E");
        assert_eq!((w, h), (512, 512));
        assert_eq!(px.len(), 512 * 512 * 4);
        // Every pixel is covered by an opaque row fill and every
        // later op blends OVER opaque, so alpha is 255 everywhere.
        assert!(px.chunks_exact(4).all(|c| c[3] == 255));
    }

    #[test]
    fn deterministic_and_base_sensitive() {
        let (a, _, _) = paint_planks("#B07A48");
        let (b, _, _) = paint_planks("#B07A48");
        assert_eq!(a, b, "same base must be byte-identical");
        let (c, _, _) = paint_planks("#9A6337");
        assert_ne!(a, c, "different base must differ");
    }

    /// shade() truth table. The first two rows are outputs the film
    /// corpus recorded from the REAL three.js pipeline (the s10
    /// bookshop data: sign edge #302222 = shade('#3A2A2A', 0.7);
    /// ceiling motif #413427 = shade('#3A2E22', 1.25)); the rest are
    /// this painter's own factors for the default base, computed
    /// from three r170's conversion constants in f64.
    #[test]
    fn shade_truth_table() {
        assert_eq!(planks_shade("#3A2A2A", 0.7), (0x30, 0x22, 0x22));
        assert_eq!(planks_shade("#3A2E22", 1.25), (0x41, 0x34, 0x27));
        assert_eq!(planks_shade("#A8693E", 0.45), (116, 71, 40));
        assert_eq!(planks_shade("#A8693E", 0.5), (122, 75, 43));
        assert_eq!(planks_shade("#A8693E", 0.7), (143, 89, 52));
        // k = 1 is the identity round trip.
        assert_eq!(planks_shade("#A8693E", 1.0), (0xA8, 0x69, 0x3E));
    }

    /// The seam band is painted AFTER the grain strokes, opaque, at
    /// shade(base, 0.45), across the full width — so every pixel of
    /// band row i*64+2 is exactly that colour (the butt joint shares
    /// the same fillStyle, and the knots sit at mid-row).
    #[test]
    fn seam_band_is_exact() {
        let (px, w, _) = paint_planks("#A8693E");
        let seam = planks_shade("#A8693E", 0.45);
        for i in 0..8usize {
            for x in [0usize, 17, 256, 511] {
                assert_eq!(px_at(&px, w as usize, x, i * 64 + 2), (seam.0, seam.1, seam.2, 255), "row {i} x {x}");
            }
        }
    }

    /// Row fills: pixel (x, i*64+63) — the plank's bottom row —
    /// is touched by nothing after the row fill when x is chosen
    /// 200 px away from the butt joint: grain curves stay above
    /// y = i*64 + ~60.2 (control-point max i*64+62 weighted by the
    /// bezier basis peaks at +2.7 over the endpoint max i*64+56,
    /// plus 1 px half-width and the AA band), knots sit at mid-row,
    /// seam at the top. Expected colours are shade('#A8693E', f_i)
    /// for f_i = 0.86 + rnd(3i)*0.24, computed from the algorithm.
    #[test]
    fn row_fills_are_exact() {
        let expected: [(u8, u8, u8); 8] = [
            (172, 108, 64),
            (169, 106, 62),
            (175, 110, 65),
            (174, 109, 65),
            (161, 100, 59),
            (172, 108, 64),
            (174, 109, 65),
            (159, 99, 58),
        ];
        let (px, w, _) = paint_planks("#A8693E");
        for i in 0..8usize {
            let jx = rnd(i as f64 * 11.0) * 512.0;
            let x = ((jx as usize + 200) % 512) as usize;
            assert_eq!(
                px_at(&px, w as usize, x, i * 64 + 63),
                (expected[i].0, expected[i].1, expected[i].2, 255),
                "row {i}"
            );
        }
    }

    /// Knot centres: the pixel containing a knot centre is at
    /// distance <= ~0.53 from it, so the r=3 disc covers the whole
    /// pixel (coverage 1) and the opaque knot colour replaces
    /// whatever the grain strokes left there. Row 0: jx =
    /// rnd(0)*512 = 419.488 -> knots at x = 405.488 and 437.488,
    /// y = 32. Row 3: jx = rnd(33)*512 = 184.326 -> x = 170.326
    /// and 202.326, y = 224.
    #[test]
    fn knot_centres_are_exact() {
        let (px, w, _) = paint_planks("#A8693E");
        let knot = planks_shade("#A8693E", 0.5);
        for (x, y) in [(405usize, 32usize), (437, 32), (170, 224), (202, 224)] {
            assert_eq!(px_at(&px, w as usize, x, y), (knot.0, knot.1, knot.2, 255), "knot at {x},{y}");
        }
    }

    /// Mean colour vs the value computed from the algorithm.
    /// Ignoring the grain strokes, the composition is exact: per
    /// 512x64 row, 30424 px of row fill shade(f_i), 2048 + 240 px
    /// of shade(0.45) (seam band + the joint's non-overlapping
    /// part), and ~56 px of shade(0.5) (two r=3 discs, area pi*9
    /// each, replacing row fill). The strokes are the only other
    /// term: they touch at most ~27% of pixels (4 curves x ~543 px
    /// x 4 px incl. AA fringe, per row) at alpha <= 89/255, moving
    /// a pixel at most toward shade(0.7). The tolerance is that
    /// bound, computed from the actual shade colours, + 1.5 LSB of
    /// slack for the disc-area and rounding approximations.
    /// For '#A8693E' the stroke-free expectation is
    /// (165.7, 103.6, 61.1).
    #[test]
    fn mean_colour_matches_algorithm() {
        let base = "#A8693E";
        let (px, w, h) = paint_planks(base);
        let npix = (w * h) as f64;
        let (mut sr, mut sg, mut sb) = (0f64, 0f64, 0f64);
        for c in px.chunks_exact(4) {
            sr += c[0] as f64;
            sg += c[1] as f64;
            sb += c[2] as f64;
        }
        let mean = (sr / npix, sg / npix, sb / npix);

        let seam = planks_shade(base, 0.45);
        let knot = planks_shade(base, 0.5);
        let grain = planks_shade(base, 0.7);
        let (mut er, mut eg, mut eb) = (0f64, 0f64, 0f64);
        let mut maxdist = (0f64, 0f64, 0f64);
        for i in 0..8usize {
            let f = 0.86 + rnd(i as f64 * 3.0) * 0.24;
            let fill = planks_shade(base, f);
            er += 30424.0 * fill.0 as f64 + 2288.0 * seam.0 as f64 + 56.0 * knot.0 as f64;
            eg += 30424.0 * fill.1 as f64 + 2288.0 * seam.1 as f64 + 56.0 * knot.1 as f64;
            eb += 30424.0 * fill.2 as f64 + 2288.0 * seam.2 as f64 + 56.0 * knot.2 as f64;
            for other in [fill, seam, knot] {
                maxdist.0 = maxdist.0.max((grain.0 as f64 - other.0 as f64).abs());
                maxdist.1 = maxdist.1.max((grain.1 as f64 - other.1 as f64).abs());
                maxdist.2 = maxdist.2.max((grain.2 as f64 - other.2 as f64).abs());
            }
        }
        let row_area = 512.0 * 64.0;
        let expect = (er / (8.0 * row_area), eg / (8.0 * row_area), eb / (8.0 * row_area));
        let tol = (
            0.27 * (89.0 / 255.0) * maxdist.0 + 1.5,
            0.27 * (89.0 / 255.0) * maxdist.1 + 1.5,
            0.27 * (89.0 / 255.0) * maxdist.2 + 1.5,
        );
        assert!((mean.0 - expect.0).abs() <= tol.0, "r: mean {} vs {} +- {}", mean.0, expect.0, tol.0);
        assert!((mean.1 - expect.1).abs() <= tol.1, "g: mean {} vs {} +- {}", mean.1, expect.1, tol.1);
        assert!((mean.2 - expect.2).abs() <= tol.2, "b: mean {} vs {} +- {}", mean.2, expect.2, tol.2);
    }
}

// ===== registry painter: wallpaper (wave pack, painters/wallpaper.rs) =====
// T.wallpaper — the bookshop wallpaper texture painter, ported
// op-for-op from the project's world.ts (world.ts:69-73) for the
// scene-format texture registry (docs/scene-format.md; the registry
// gap is B8 in scenes/sets/README.md).
//
// INTEGRATION: this file is the registry version of the painter
// film.rs already carries as a local duplicate (film.rs's
// paint_wallpaper, film.rs:526). Drop it into src/scene.rs beside
// paint_grass / paint_sky (deleting the `use` line below — those
// items are scene.rs's own) and register it in
// src/scenefile.rs::resolve_texture as:
//
//     "wallpaper" => scene::paint_wallpaper(
//         str_field(spec, "base", ctx)?,
//         str_field(spec, "motif", ctx)?,
//     ),
//
// The spec fields are the ones the film data already declares
// (scenes/sets/bookshop.scene.json, scenes/film/s10.scene.json):
// `base` (the TS `col`), `motif`, `canvas: [256, 256]`, plus a
// `repeat` the painter does not consume (repeat is a material-side
// clone property in the source too — canvasTex keys its cache by
// 'wp' + col + motif only and clones per repeat).
//
// Signature and conventions mirror the registry painters exactly:
// hex-string colour parameters in, (RGBA bytes, w, h) out, painted
// through the shared CPU Painter with its signed-distance coverage
// model. The source painter uses NO rnd() calls — it is fully
// geometric — so determinism here is exact, not hash-dependent.
//
// Two interpretation points (full discussion in
// painters/wallpaper.md): the two wing arcs share ONE canvas path
// in the source; they are painted as two discs because Chrome/Skia
// clamps an arc sweep to a full circle, which makes the inter-arc
// connector degenerate — the same reading film.rs's port took. And
// the fleur position list has four entries but an `i < 2` guard
// draws only the first two.


/// world.ts:35's `shade(hex, k)`: THREE.Color(hex).multiplyScalar(k)
/// read back with getHexString(). THREE.Color stores its components
/// in the LINEAR working space (the hex is converted on
/// construction), so the multiply happens in linear space and the
/// read-back re-encodes to sRGB (clamped, rounded) — NOT the canvas
/// shadeOf() byte multiply scene::shade_rgb implements. The two
/// visibly differ: shade('#3A2E22', 1.25) is #413427 this way and
/// #493A2B the shade_rgb way (scenes/film/ASSUMPTIONS.md, S10
/// divergences item 3 — the source value is #413427).
///
/// The wallpaper painter itself never calls shade() — interior()
/// pre-computes the ceiling motif with it (world.ts:313) and the
/// film data carries the result — but the registry consumer that
/// derives a variant from parameters needs it computed the source's
/// way, and the unit tests pin it against the evaluated constant.
pub(crate) fn shade_hex(hex: &str, k: f32) -> (u8, u8, u8) {
    let lin = hex_lin(hex);
    let f = |c: f32| -> u8 {
        (linear_to_srgb(c * k) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    (f(lin[0]), f(lin[1]), f(lin[2]))
}

/// T.wallpaper(col, motif): 256x256 — base fill, fleur motifs at
/// (64,64) and (192,192) at scale 0.9, then four alpha-0.5 dots
/// (world.ts:69-73). The source also sets strokeStyle/lineWidth = 5
/// before drawing; no stroke() is ever called, so there is nothing
/// to port for them.
pub(crate) fn paint_wallpaper(base: &str, motif: &str) -> (Vec<u8>, u32, u32) {
    let (w, h) = (256usize, 256usize);
    let mut p = Painter::new(w, h);
    p.fill_rect(0.0, 0.0, w as f32, h as f32, rgba_of(base));
    let m = rgba_of(motif);
    // fleur(x, y, s): translate(x, y) then scale(s, s), all fills in
    // the motif colour. One fleur is:
    //   petal — moveTo(0,-40), bezierCurveTo(18,-20, 18,0, 0,14),
    //           bezierCurveTo(-18,0, -18,-20, 0,-40), fill;
    //   wings — one path: arc(-22,4,12, 0,7) + arc(22,4,12, 0,7),
    //           fill (two discs — see the header note);
    //   stem  — fillRect(-3, 10, 6, 26).
    let fleur = |p: &mut Painter, cx: f32, cy: f32, s: f32| {
        let mut outline = flatten_cubic((0.0, -40.0), (18.0, -20.0), (18.0, 0.0), (0.0, 14.0), 24);
        let back = flatten_cubic((0.0, 14.0), (-18.0, 0.0), (-18.0, -20.0), (0.0, -40.0), 24);
        // Skip the return curve's first point (the joint, already
        // the forward curve's last) and its last (the start point,
        // which fill_polygon closes to on its own).
        outline.extend(back[1..back.len() - 1].iter().copied());
        let pts: Vec<(f32, f32)> = outline
            .iter()
            .map(|&(x, y)| (cx + x * s, cy + y * s))
            .collect();
        p.fill_polygon(&pts, m);
        p.fill_ellipse(cx - 22.0 * s, cy + 4.0 * s, 12.0 * s, 12.0 * s, 0.0, m);
        p.fill_ellipse(cx + 22.0 * s, cy + 4.0 * s, 12.0 * s, 12.0 * s, 0.0, m);
        p.fill_rect(cx - 3.0 * s, cy + 10.0 * s, 6.0 * s, 26.0 * s, m);
    };
    // [[64,64],[192,192],[192,64-256],[64,192]] with `i < 2 &&` —
    // only these two fleurs exist; the other two positions are dead
    // list entries (one of them, (192,-192), is off-canvas anyway).
    fleur(&mut p, 64.0, 64.0, 0.9);
    fleur(&mut p, 192.0, 192.0, 0.9);
    // globalAlpha 0.5 dots, r 6, at the four grid points in source
    // order: arc(i%2 ? 192 : 64, i<2 ? 192 : 64, 6, 0, 7). Alpha 128
    // for the 0.5 (0.5 * 255 = 127.5; the house port precedent —
    // film.rs's paint_wallpaper uses the same literal). Two of the
    // dots land on fleur centres (motif over motif: no visible
    // change); two land on bare base.
    for i in 0..4 {
        let (dx, dy) = match i {
            0 => (64.0, 192.0),
            1 => (192.0, 192.0),
            2 => (64.0, 64.0),
            _ => (192.0, 64.0),
        };
        p.fill_ellipse(dx, dy, 6.0, 6.0, 0.0, (m.0, m.1, m.2, 128));
    }
    (p.px, w as u32, h as u32)
}

#[cfg(test)]
mod wallpaper_tests {
    use super::*;

    fn at(px: &[u8], w: u32, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let k = ((y * w + x) * 4) as usize;
        (px[k], px[k + 1], px[k + 2], px[k + 3])
    }

    // The bookshop's wall parameters (world.ts interior() as staged
    // by shots.ts:227; declared on the wallpaper-walls/-back/-side
    // texture entries).
    const WALL: (u8, u8, u8, u8) = (0x5A, 0x46, 0x32, 255);
    const WALL_MOTIF: (u8, u8, u8, u8) = (0x6B, 0x56, 0x38, 255);
    // The ceiling variant: base #3A2E22, motif #413427 — the motif
    // is shade(ceiling, 1.25) precomputed by the source caller.
    const CEIL: (u8, u8, u8, u8) = (0x3A, 0x2E, 0x22, 255);
    const CEIL_MOTIF: (u8, u8, u8, u8) = (0x41, 0x34, 0x27, 255);

    #[test]
    fn size_and_fully_opaque() {
        let (px, w, h) = paint_wallpaper("#5A4632", "#6B5638");
        assert_eq!((w, h), (256, 256));
        assert_eq!(px.len(), 256 * 256 * 4);
        // Base fill is opaque and every later op is opaque or an
        // alpha blend over opaque, so no alpha but 255 survives.
        assert!(px.chunks_exact(4).all(|p| p[3] == 255));
    }

    #[test]
    fn deterministic() {
        let a = paint_wallpaper("#5A4632", "#6B5638");
        let b = paint_wallpaper("#5A4632", "#6B5638");
        assert_eq!(a, b);
    }

    #[test]
    fn bookshop_wall_colours() {
        let (px, w, _) = paint_wallpaper("#5A4632", "#6B5638");
        // Bare base, away from every fleur and dot.
        assert_eq!(at(&px, w, 10, 10), WALL);
        // Fleur petal interior (local (0.6, -26) of the (64,64)
        // fleur), both fleurs.
        assert_eq!(at(&px, w, 64, 40), WALL_MOTIF);
        assert_eq!(at(&px, w, 192, 167), WALL_MOTIF);
        // Left wing disc centre (local (-22, 4) at scale 0.9).
        assert_eq!(at(&px, w, 44, 67), WALL_MOTIF);
        // Stem interior (local (0.6, 22.8)).
        assert_eq!(at(&px, w, 64, 84), WALL_MOTIF);
        // Fleur centre: petal-covered, then dotted with its own
        // colour at alpha 0.5 — motif over motif is exact.
        assert_eq!(at(&px, w, 64, 64), WALL_MOTIF);
    }

    #[test]
    fn alpha_dot_over_base_composites() {
        let (px, w, _) = paint_wallpaper("#5A4632", "#6B5638");
        // The (192,64) dot lands on bare base: motif at alpha
        // 128/255 source-over the base, per the Painter blend.
        assert_eq!(at(&px, w, 192, 64), (99, 78, 53, 255));
        // The (64,192) grid point carries the same dot composite —
        // and nothing else: its fleur list entry is the fourth,
        // which the source's `i < 2` guard never draws.
        assert_eq!(at(&px, w, 64, 192), (99, 78, 53, 255));
        // Where that undrawn fleur's petal would sit (local
        // (0.6, -26) of (64,192)) the base is untouched.
        assert_eq!(at(&px, w, 64, 168), WALL);
    }

    #[test]
    fn ceiling_variant_and_linear_shade() {
        // The pinned constant, computed the source's way:
        // THREE.Color multiplyScalar in linear space (world.ts:35).
        // The sRGB byte multiply (scene::shade_rgb) would give
        // (0x49, 0x3A, 0x2B) here — that is film.rs's known S10
        // divergence, not the source value.
        assert_eq!(shade_hex("#3A2E22", 1.25), (0x41, 0x34, 0x27));
        let (px, w, _) = paint_wallpaper("#3A2E22", "#413427");
        assert_eq!(at(&px, w, 10, 10), CEIL);
        assert_eq!(at(&px, w, 64, 40), CEIL_MOTIF);
    }

    #[test]
    fn source_defaults() {
        // T.wallpaper's own defaults (world.ts:69).
        let (px, w, _) = paint_wallpaper("#9E2E3A", "#B8414C");
        assert_eq!(at(&px, w, 10, 10), (0x9E, 0x2E, 0x3A, 255));
        assert_eq!(at(&px, w, 64, 40), (0xB8, 0x41, 0x4C, 255));
    }
}

// ===== registry painter: felt (wave pack, painters/felt.rs) =====
// T.felt(col) — the bookshop's green baize (world.ts:93), a registry
// painter in the scene.rs conventions: same `paint_*` signature,
// same CPU `Painter`, same deterministic sin-hash `rnd`.
//
// Source recipe (world.ts:93, one line):
//   canvasTex('felt' + col, 256, 256, (g, w, h) => {
//     g.fillStyle = col; g.fillRect(0, 0, w, h);
//     for (let i = 0; i < 3000; i++) {
//       g.fillStyle = shade(col, 0.85 + rnd(i) * 0.3);
//       g.fillRect(rnd(i * 3) * w, rnd(i * 7) * h, 2, 2);
//     } })
//
// The one subtlety is `shade`: world.ts:35 defines it through
// THREE.Color — NOT the canvas shadeOf (toon.ts:94) that
// scene::shade_rgb mirrors. THREE.Color stores the hex converted
// to the LINEAR working space, multiplyScalar(k) scales the linear
// components, and getHexString() converts back to sRGB, quantising
// with Math.round(clamp(255*c, 0, 255)) per channel. This module
// reproduces that arithmetic exactly (three r170 transfer curves,
// f64 throughout, JS argument evaluation order: k and the speckle
// position are f64 until the Painter's f32 rasteriser, exactly as
// film.rs's painters cast rnd results to f32 at the fill call).
// scene::shade_lin does NOT equal it (it multiplies the sRGB bytes
// first, then converts) — cf. the W14 ceiling-motif finding
// (#413427 in three vs #493A2B through shade_lin).


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
    p.fill_rect(0.0, 0.0, w as f32, h as f32, rgba_of(base));
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
mod felt_tests {
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
        // Post root-fix (shade_rgb IS the THREE.Color linear
        // multiply now, c00897d) the two implementations agree —
        // this assertion pinned their difference against the old
        // byte multiply and is inverted deliberately: agreement
        // on the corpus witness is the pinned truth now.
        assert_eq!(
            shade_three("#1F5A45", 0.85),
            shade_rgb("#1F5A45", 0.85)
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

// ===== registry painter: parcel (wave pack, painters/parcel.rs) =====
// PR.parcel texture (props.ts:159) — the brown-paper parcel wrap,
// a registry painter in the scene.rs conventions: same `paint_*`
// signature shape, same CPU `Painter`, same deterministic
// sin-hash `rnd`.
//
// Source recipe (props.ts:159, the texture half of
// parcel(w, h, d, label = true) — the box dimensions never reach
// the canvas; only `label` does):
//   canvasTex('parcel' + label, 256, 256, (c) => {
//     c.fillStyle = '#C8935A'; c.fillRect(0, 0, 256, 256);
//     c.fillStyle = '#B07A48';
//     for (let i = 0; i < 30; i++)
//       c.fillRect(rnd(i) * 256, rnd(i * 3) * 256, 40, 2);
//     c.fillStyle = '#E9D7A5';
//     c.fillRect(120, 0, 16, 256); c.fillRect(0, 120, 256, 16);
//     if (label) {
//       c.fillStyle = '#FFFDF4'; c.fillRect(150, 150, 90, 70);
//       c.fillStyle = '#D8343F'; c.fillRect(214, 156, 20, 24);
//     } })
//
// No colour management is involved: every fill is a literal hex
// (canvas bytes), so this painter is a direct transcription. The
// bookshop stages both parcels with label = false.


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
mod parcel_tests {
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

// ===== registry painter: picture (wave pack, painters/picture.rs) =====
// paint_picture — registry texture generator "picture": the framed
// line-drawing that hangs in the bookshop interior.
//
// Source recipe, transcribed 1:1 from the Onoda project's
// core/world.ts:319 (`interior()`'s `pic` canvasTex, 256x200):
//   paper fill '#E9DCC0';
//   ink '#3A2A22', lineWidth 6, ONE path:
//     moveTo(40,140)
//     quadraticCurveTo(90,60, 140,120)
//     quadraticCurveTo(170,80, 220,140)
//   stroke();
//   sun: fill '#C9603F', arc(190,60,22, 0, 7), fill().
// (arc end angle 7 rad > TAU, so the sun is a full disc.)
//
// The gold FRAME is not part of this texture: in the source it is
// the box mesh's face materials (world.ts:320, '#C9A13B' on five
// faces, this texture on the front) — see PICTURE_FRAME below.
//
// Conventions mirrored from the existing registry painters in
// src/scene.rs (paint_grass / paint_cloud / paint_blob): signature
// returns (RGBA bytes, w, h); canvas coordinates, row 0 = top;
// pure function of its inputs (no state, no RNG — this recipe has
// no hash strokes at all); curves flattened with the house
// flatten_quad (the toonfull.rs precedent uses n = 20 per quad).
//
// Integration (lead): move to src/painters/picture.rs and add the
// scenefile.rs resolve_texture arm
//   "picture" => painters::picture::paint_picture(),
// or paste the body into src/scene.rs and drop the `use` line.
// The spec entry (scenes/sets/bookshop.scene.json, texture id
// "picture") carries no parameters — canvas is fixed 256x200.


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
mod picture_tests {
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

// ===== registry painter: sign (wave pack, painters/sign.rs) =====
// paint_sign — registry texture generator "sign": the bookshop's
// 'TANIGUCHI BOOKS' board (world.ts signBoard, core/world.ts:137-146).
//
// Source recipe:
//   canvas = (round(w*200), round(h*200)) px — 200 px per unit;
//     the bookshop board is w=2.2, h=0.4 -> 440x80.
//   fill bg over the whole canvas;
//   border: strokeStyle fg, lineWidth 6,
//     strokeRect(10, 10, cw-20, ch-20) — the stroke is CENTRED on
//     the rect path, so ink spans 7..13 px from each edge;
//   text: font `700 {fs}px Caladea, Georgia, serif`,
//     fs starts at ch*0.5 and shrinks by 2 while
//     measureText(text) > cw*0.86;
//     fillText(text, cw/2, ch/2 + 3), align centre, baseline middle.
//   The board's box side faces take shade(bg, 0.7) — see
//   sign_edge_rgba.
//
// TEXT STRATEGY (the engine's existing approach, stated plainly):
// this engine has NO Rust text rasteriser — every text-bearing
// texture enters as a PIL asset rendered from the project's own
// fonts (film.rs module docs; captions, the S07 calendar plates,
// and film/assets/sign.png for this very sign, all produced by
// film/gen_assets.py). paint_sign therefore paints the board and
// border natively and sign_with_text() composites a same-size
// transparent glyph strip (PIL, 700 Caladea, laid out per
// SIGN_FONT_* / sign_text_anchor) over it. Without a strip the
// painter returns the board+border exactly as the canvas looks
// before fillText.
//
// FONT SIZE AS DATA (gap B9): the shrink rule's outcome depends on
// font metrics. Measured with PIL against the project's own
// fonts/Caladea-Bold.ttf (2026-11-14): 'TANIGUCHI BOOKS' at the
// 40 px start size is 335.36 px wide, under the 378.4 px limit —
// NO shrink step fires; fs = 40 (see TANIGUCHI_BOOKS_PIL below).
// That is a PIL measurement: browser (Skia) Caladea metrics are
// unreproducible from source, so the browser's fs is not provable
// from here — but the 43.04 px headroom makes a shrink in the
// browser unlikely. gen_assets.py's own loop lands on 40 too.
//
// Integration (lead): move to src/painters/sign.rs and add the
// scenefile.rs resolve_texture arm, e.g.
//   "sign" => painters::sign::paint_sign(
//       str_field(spec, "bg", ctx)?, str_field(spec, "fg", ctx)?,
//       /* units from the mesh or canvas/200 */ ...),
// optionally followed by sign_with_text() when the spec names a
// glyph-strip file; or paste the body into src/scene.rs and drop
// the `use` line.


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
mod sign_tests {
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
