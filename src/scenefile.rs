//! Scene-format loader and renderer (M2): declarative JSON scenes
//! (docs/scene-format.md) rendered through the same pipelines as the
//! hand-authored scenes.
//!
//! Two render paths, chosen by the file:
//! - **direct** (no `post` block): one pass into an sRGB target with
//!   the ab1 pipelines — vertex-colour geometry under the house light
//!   (shader_scene3d.wgsl: shader3d with the light as a uniform) and
//!   alpha-blended textured draws (shader_tex.wgsl).
//! - **composer** (`post.bokeh` present): the dof-chart chain —
//!   HalfFloat MSAA-4x scene target, packed-depth prepass, the
//!   transcribed r170 bokeh composite, Neutral + sRGB output stage
//!   (the dof.rs shaders, unchanged). Since M5 the chain also runs
//!   the optional `post.bloom` (UnrealBloomPass) between bokeh and
//!   output and the optional `post.grade` (the engine's GRADE shader)
//!   after output, both from src/composer.rs; absent fields leave
//!   the chain byte-identical to the M2/M3 form.
//!
//! Nodes draw in document order in both paths; group transforms
//! accumulate down the tree. The `code-ref` node type is the format's
//! declared escape hatch: it splices draws built by Rust code (the
//! §28 character card + rifle, whose painter is M4) into the draw
//! list at the node's position.

use crate::math::{self, Mat4};
use crate::scene::{
    self, box_geo, chart_texture_rgba, cone_geo, cross3, cyl_geo, disc_geo, flat_quad_geo,
    ground_disc_geo, hex_lin, norm3, quad_geo, shade_lin, sky_sphere_geo, sphere_geo, sub3,
    ColourDraw, Geo, TexDraw, TexGeo, Value,
};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

// ---------------------------------------------------------------------
// Loaded scene
// ---------------------------------------------------------------------

struct Camera {
    fov_y_deg: f32,
    near: f32,
    far: f32,
    eye: [f32; 3],
    target: [f32; 3],
    up: [f32; 3],
}

struct Bokeh {
    focus: f32,
    aperture: f32,
    maxblur: f32,
}

/// UnrealBloomPass settings (M5). Field defaults mirror the engine's
/// per-frame application (engine.ts): threshold ?? 0.85, radius
/// ?? 0.6; strength defaults to 0, so an empty block is inert.
struct BloomCfg {
    threshold: f32,
    strength: f32,
    radius: f32,
}

/// The engine's GRADE settings (M5). Defaults mirror engine.ts's
/// per-frame fallbacks: vignette 0.18, grain 0.008, warm 0.02,
/// sat 1.12, uTime = t ?? 0.
struct GradeCfg {
    vignette: f32,
    grain: f32,
    warm: f32,
    sat: f32,
    exposure: f32,
    time: f32,
}

enum Draw {
    Colour(ColourDraw),
    Tex(TexDraw),
}

/// Format v2 light rig (spec §14): the production rig the source
/// engine builds with lightRig()/sunlight() — parsed and validated
/// here; the film renderer (src/film.rs) consumes it in full. The
/// still paths consume what their shaders can express and print
/// the rig in the scene header line.
pub(crate) struct KeyCfg {
    pub color: String,
    pub intensity: f32,
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub shadow: bool,
    pub shadow_box: f32,
    pub far: f32,
    pub bias: f32,
    pub normal_bias: f32,
}
pub(crate) struct RimCfg {
    pub color: String,
    pub intensity: f32,
    pub position: [f32; 3],
}
pub(crate) struct HemiCfg {
    pub sky: String,
    pub ground: String,
    pub intensity: f32,
}
pub(crate) struct PointCfg {
    pub color: String,
    pub intensity: f32,
    pub position: [f32; 3],
    pub distance: f32,
    pub decay: f32,
}
pub(crate) struct LightsCfg {
    pub key: Option<KeyCfg>,
    pub rim: Option<RimCfg>,
    pub hemisphere: Option<HemiCfg>,
    pub points: Vec<PointCfg>,
    pub env_intensity: f32,
}

struct Loaded {
    name: String,
    version: u32,
    time: f32,
    camera: Camera,
    clear: Option<[f32; 3]>, // linearised sRGB
    light_dir: [f32; 3],
    ambient: f32,
    lights: Option<LightsCfg>,
    textures: Vec<(Vec<u8>, u32, u32)>,
    draws: Vec<Draw>,
    bokeh: Option<Bokeh>,
    bloom: Option<BloomCfg>,
    grade: Option<GradeCfg>,
    fog: Option<FogCfg>,
}

/// Format v2 `fog` block (spec §14): three.js scene.fog —
/// { color (sRGB hex, stored linearised), near, far } in metres.
struct FogCfg {
    color: [f32; 3],
    near: f32,
    far: f32,
}

// ---------------------------------------------------------------------
// Small typed accessors over the JSON tree, with node-path context.
// ---------------------------------------------------------------------

fn err(ctx: &str, msg: &str) -> String {
    format!("{ctx}: {msg}")
}

fn req<'a>(v: &'a Value, key: &str, ctx: &str) -> Result<&'a Value, String> {
    v.get(key).ok_or_else(|| err(ctx, &format!("missing required field '{key}'")))
}

fn num(v: &Value, ctx: &str) -> Result<f64, String> {
    v.as_f64().ok_or_else(|| err(ctx, "expected a number"))
}

fn num_field(v: &Value, key: &str, ctx: &str) -> Result<f64, String> {
    num(req(v, key, ctx)?, &format!("{ctx}.{key}"))
}

fn f32_field(v: &Value, key: &str, ctx: &str) -> Result<f32, String> {
    Ok(num_field(v, key, ctx)? as f32)
}

/// An optional f32 field with a documented default (the post.bloom /
/// post.grade blocks carry engine-mirroring defaults per field).
fn f32_field_or(v: &Value, key: &str, ctx: &str, default: f32) -> Result<f32, String> {
    match v.get(key) {
        Some(x) => Ok(num(x, &format!("{ctx}.{key}"))? as f32),
        None => Ok(default),
    }
}

fn u32_field(v: &Value, key: &str, ctx: &str) -> Result<u32, String> {
    let n = num_field(v, key, ctx)?;
    if n < 0.0 || n.fract() != 0.0 {
        return Err(err(ctx, &format!("'{key}' must be a non-negative integer")));
    }
    Ok(n as u32)
}

fn str_field<'a>(v: &'a Value, key: &str, ctx: &str) -> Result<&'a str, String> {
    req(v, key, ctx)?
        .as_str()
        .ok_or_else(|| err(ctx, &format!("'{key}' must be a string")))
}

fn vec3(v: &Value, ctx: &str) -> Result<[f32; 3], String> {
    let a = v.as_arr().ok_or_else(|| err(ctx, "expected [x, y, z]"))?;
    if a.len() != 3 {
        return Err(err(ctx, "expected exactly 3 components"));
    }
    Ok([num(&a[0], ctx)? as f32, num(&a[1], ctx)? as f32, num(&a[2], ctx)? as f32])
}

fn vec3_field(v: &Value, key: &str, ctx: &str) -> Result<[f32; 3], String> {
    vec3(req(v, key, ctx)?, &format!("{ctx}.{key}"))
}

// ---------------------------------------------------------------------
// Format pieces
// ---------------------------------------------------------------------

fn compose_transform(spec: Option<&Value>, ctx: &str) -> Result<Mat4, String> {
    let mut m = math::identity();
    let Some(spec) = spec else { return Ok(m) };
    let ops = spec.as_arr().ok_or_else(|| err(ctx, "'transform' must be an array of ops"))?;
    for (i, op) in ops.iter().enumerate() {
        let octx = format!("{ctx}.transform[{i}]");
        let Value::Obj(pairs) = op else {
            return Err(err(&octx, "transform op must be a single-key object"));
        };
        if pairs.len() != 1 {
            return Err(err(&octx, "transform op must have exactly one key"));
        }
        let (key, val) = &pairs[0];
        // Ops compose left to right: m = m * op (the hand-authored
        // code's dominant association; see docs/scene-format.md).
        let opm = match key.as_str() {
            "translate" => {
                let t = vec3(val, &octx)?;
                math::translate(t[0], t[1], t[2])
            }
            "rotate_x" => math::rotate_x(num(val, &octx)? as f32),
            "rotate_y" => math::rotate_y(num(val, &octx)? as f32),
            "rotate_z" => math::rotate_z(num(val, &octx)? as f32),
            "euler_xyz" => {
                let e = vec3(val, &octx)?;
                math::euler_xyz(e[0], e[1], e[2])
            }
            "scale" => {
                let s = vec3(val, &octx)?;
                math::scale(s[0], s[1], s[2])
            }
            other => return Err(err(&octx, &format!("unknown transform op '{other}'"))),
        };
        m = math::mul(&m, &opm);
    }
    Ok(m)
}

/// Material colour: a hex string, {hex, shade} (the canvas shadeOf
/// law: multiply sRGB bytes by k), or {hex, shadeHash} where
/// k = base + rnd(index) * span in f64, cast to f32 — the exact
/// expression the hand-authored scene code evaluates.
fn material_colour(spec: &Value, ctx: &str) -> Result<[f32; 3], String> {
    let colour = req(spec, "color", ctx)?;
    match colour {
        Value::Str(hex) => Ok(hex_lin(hex)),
        Value::Obj(_) => {
            let hex = str_field(colour, "hex", ctx)?;
            if let Some(k) = colour.get("shade") {
                Ok(shade_lin(hex, num(k, ctx)? as f32))
            } else if let Some(h) = colour.get("shadeHash") {
                let index = num_field(h, "index", ctx)?;
                let base = num_field(h, "base", ctx)?;
                let span = num_field(h, "span", ctx)?;
                let k = (base + scene::rnd(index) * span) as f32;
                Ok(shade_lin(hex, k))
            } else {
                Ok(hex_lin(hex))
            }
        }
        _ => Err(err(ctx, "'color' must be a hex string or an object")),
    }
}

fn colour_geo(spec: &Value, colour: [f32; 3], ctx: &str) -> Result<Geo, String> {
    let kind = str_field(spec, "kind", ctx)?;
    Ok(match kind {
        "box" => {
            let s = vec3_field(spec, "size", ctx)?;
            box_geo(s[0], s[1], s[2], colour)
        }
        "cylinder" => cyl_geo(
            f32_field(spec, "radiusTop", ctx)?,
            f32_field(spec, "radiusBottom", ctx)?,
            f32_field(spec, "height", ctx)?,
            u32_field(spec, "segments", ctx)?,
            colour,
        ),
        "sphere" => sphere_geo(
            f32_field(spec, "radius", ctx)?,
            u32_field(spec, "widthSegments", ctx)?,
            u32_field(spec, "heightSegments", ctx)?,
            colour,
        ),
        "cone" => cone_geo(
            f32_field(spec, "radius", ctx)?,
            f32_field(spec, "height", ctx)?,
            u32_field(spec, "segments", ctx)?,
            colour,
        ),
        "disc" => disc_geo(
            f32_field(spec, "radius", ctx)?,
            u32_field(spec, "segments", ctx)?,
            colour,
        ),
        other => return Err(err(ctx, &format!("unknown colour geometry kind '{other}'"))),
    })
}

/// Billboard corners from the camera basis, exactly as the
/// hand-authored code builds its cloud sprites and caption quad:
/// right = norm(cross(fwd, world-up)), up = cross(right, fwd).
fn billboard_geo(spec: &Value, cam: &Camera, ctx: &str) -> Result<TexGeo, String> {
    let centre = vec3_field(spec, "center", ctx)?;
    let (hw, hh) = (f32_field(spec, "width", ctx)? / 2.0, f32_field(spec, "height", ctx)? / 2.0);
    let fwd = norm3(sub3(cam.target, cam.eye));
    let right = norm3(cross3(fwd, [0.0, 1.0, 0.0]));
    let upv = cross3(right, fwd);
    let corner = |sx: f32, sy: f32| -> [f32; 3] {
        [
            centre[0] + right[0] * hw * sx + upv[0] * hh * sy,
            centre[1] + right[1] * hw * sx + upv[1] * hh * sy,
            centre[2] + right[2] * hw * sx + upv[2] * hh * sy,
        ]
    };
    let mut g = TexGeo::new();
    g.vert(corner(-1.0, -1.0), [0.0, 1.0]);
    g.vert(corner(1.0, -1.0), [1.0, 1.0]);
    g.vert(corner(1.0, 1.0), [1.0, 0.0]);
    g.vert(corner(-1.0, 1.0), [0.0, 0.0]);
    g.tri(0, 1, 2);
    g.tri(0, 2, 3);
    Ok(g)
}

fn tex_geo(spec: &Value, cam: &Camera, ctx: &str) -> Result<TexGeo, String> {
    let kind = str_field(spec, "kind", ctx)?;
    Ok(match kind {
        "quad" => {
            let y0 = match spec.get("y0") {
                Some(v) => num(v, ctx)? as f32,
                None => 0.0,
            };
            quad_geo(f32_field(spec, "width", ctx)?, f32_field(spec, "height", ctx)?, y0)
        }
        "flat-quad" => flat_quad_geo(
            f32_field(spec, "width", ctx)?,
            f32_field(spec, "depth", ctx)?,
        ),
        "ground-disc" => ground_disc_geo(
            f32_field(spec, "radius", ctx)?,
            u32_field(spec, "segments", ctx)?,
            match spec.get("uvRepeat") {
                Some(v) => num(v, ctx)? as f32,
                None => 1.0,
            },
        ),
        "sky-sphere" => sky_sphere_geo(
            f32_field(spec, "radius", ctx)?,
            u32_field(spec, "widthSegments", ctx)?,
            u32_field(spec, "heightSegments", ctx)?,
        ),
        "billboard" => billboard_geo(spec, cam, ctx)?,
        other => return Err(err(ctx, &format!("unknown textured geometry kind '{other}'"))),
    })
}

/// The annulus scatter: instance i takes angle rnd(i*angleHash) *
/// angleScale and radius radiusBase + rnd(i*radiusHash) * radiusSpan
/// (f64 hash law, contracts §4.1), is offset by `center`, and is
/// merged into a single geometry — the §28 grass-tuft placement,
/// generalised only over its constants.
fn scatter_geo(base: Geo, spec: &Value, ctx: &str) -> Result<Geo, String> {
    let count = u32_field(spec, "count", ctx)? as usize;
    let p = req(spec, "placement", ctx)?;
    let kind = str_field(p, "kind", ctx)?;
    if kind != "annulus" {
        return Err(err(ctx, &format!("unknown scatter placement kind '{kind}'")));
    }
    let angle_hash = num_field(p, "angleHash", ctx)?;
    let radius_hash = num_field(p, "radiusHash", ctx)?;
    let radius_base = num_field(p, "radiusBase", ctx)?;
    let radius_span = num_field(p, "radiusSpan", ctx)?;
    let angle_scale = match p.get("angleScale") {
        Some(v) => num(v, ctx)?,
        None => std::f64::consts::TAU,
    };
    let center = vec3_field(p, "center", ctx)?;
    let mut merged = Geo::new();
    for i in 0..count {
        let fi = i as f64;
        let a = scene::rnd(fi * angle_hash) * angle_scale;
        let r = radius_base + scene::rnd(fi * radius_hash) * radius_span;
        let off = [
            (a.cos() * r) as f32 + center[0],
            center[1],
            (a.sin() * r) as f32 + center[2],
        ];
        let mut inst = Geo { verts: base.verts.clone(), idx: base.idx.clone() };
        for v in inst.verts.chunks_exact_mut(9) {
            v[0] += off[0];
            v[1] += off[1];
            v[2] += off[2];
        }
        merged.append(inst);
    }
    Ok(merged)
}

fn resolve_texture(spec: &Value, scene_dir: &Path, ctx: &str) -> Result<(Vec<u8>, u32, u32), String> {
    if let Some(path) = spec.get("file") {
        let path = path.as_str().ok_or_else(|| err(ctx, "'file' must be a string"))?;
        let full = scene_dir.join(path);
        let img = image::open(&full)
            .map_err(|e| err(ctx, &format!("cannot load texture file {}: {e}", full.display())))?
            .to_rgba8();
        let (w, h) = img.dimensions();
        return Ok((img.into_raw(), w, h));
    }
    let gen = str_field(spec, "generator", ctx)?;
    Ok(match gen {
        "grass" => scene::paint_grass(str_field(spec, "base", ctx)?),
        // Registry painters (wave pack, pasted into scene.rs with
        // their pinned tests): the bookshop/prop texture family.
        "planks" => scene::paint_planks(str_field(spec, "base", ctx)?),
        "wallpaper" => scene::paint_wallpaper(
            str_field(spec, "base", ctx)?,
            str_field(spec, "motif", ctx)?,
        ),
        "felt" => scene::paint_felt(str_field(spec, "base", ctx)?),
        "parcel" => scene::paint_parcel(truthy(spec.get("label"))),
        "picture" => scene::paint_picture(),
        "sign" => {
            let c = spec.get("canvas").and_then(Value::as_arr)
                .filter(|a| a.len() == 2)
                .ok_or_else(|| err(ctx, "'canvas' must be [w, h]"))?;
            scene::paint_sign_px(
                str_field(spec, "bg", ctx)?,
                str_field(spec, "fg", ctx)?,
                num(&c[0], ctx)? as u32,
                num(&c[1], ctx)? as u32,
            )
        }
        // Flat colour panel (fog-chart + test scenes): 64x64 sRGB
        // bytes of the entry's `color`, per tools/fog-chart/README.
        "solid" => {
            let (r, g, b) = scene::hex_rgb(str_field(spec, "color", ctx)?);
            let mut px = vec![0u8; 64 * 64 * 4];
            for i in 0..64 * 64 {
                px[i * 4] = r;
                px[i * 4 + 1] = g;
                px[i * 4 + 2] = b;
                px[i * 4 + 3] = 255;
            }
            (px, 64, 64)
        }
        "sky-gradient" => scene::paint_sky(
            str_field(spec, "top", ctx)?,
            str_field(spec, "horizon", ctx)?,
            spec.get("ground").and_then(|v| v.as_str()).unwrap_or("#BFE4F7"),
        ),
        "cloud" => scene::paint_cloud(),
        "blob-shadow" => scene::paint_blob(),
        "onoda-card" => crate::ab1::paint_onoda(),
        "dof-chart-pattern" => {
            let size = match spec.get("size") {
                Some(v) => num(v, ctx)? as u32,
                None => 512,
            };
            let data = chart_texture_rgba(size);
            (data, size, size)
        }
        other => return Err(err(ctx, &format!("unknown texture generator '{other}'"))),
    })
}

fn truthy(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::Bool(true))) || v.and_then(Value::as_f64).unwrap_or(0.0) != 0.0
}

fn opt_str(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(|s| s.to_string())
}

fn pair_field(v: &Value, key: &str, ctx: &str, default: (f32, f32)) -> Result<(f32, f32), String> {
    match v.get(key) {
        None => Ok(default),
        Some(a) => {
            let a = a.as_arr().ok_or_else(|| err(ctx, &format!("'{key}' must be [a, e]")))?;
            if a.len() != 2 {
                return Err(err(ctx, &format!("'{key}' must be [a, e]")));
            }
            Ok((num(&a[0], ctx)? as f32, num(&a[1], ctx)? as f32))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_nodes(
    nodes: &[Value],
    parent: &Mat4,
    cam: &Camera,
    tex_index: &HashMap<String, usize>,
    textures: &mut Vec<(Vec<u8>, u32, u32)>,
    draws: &mut Vec<Draw>,
    ctx: &str,
) -> Result<(), String> {
    for (i, node) in nodes.iter().enumerate() {
        let nctx = format!("{ctx}[{i}]");
        // §15.1 reservation guard: a prefab instance or a clip
        // field on a node names its extension instead of falling
        // through to the misleading missing-'type' error (or, for
        // clip, being silently ignored).
        if node.get("prefab").is_some() {
            return Err(err(&nctx, "prefab extension not supported"));
        }
        if node.get("clip").is_some() || node.get("clipTime").is_some() {
            return Err(err(&nctx, "clips extension not supported"));
        }
        let ty = str_field(node, "type", &nctx)?;
        match ty {
            "group" => {
                let local = compose_transform(node.get("transform"), &nctx)?;
                let m = math::mul(parent, &local);
                let children = req(node, "children", &nctx)?
                    .as_arr()
                    .ok_or_else(|| err(&nctx, "'children' must be an array"))?;
                walk_nodes(children, &m, cam, tex_index, textures, draws, &format!("{nctx}.children"))?;
            }
            "mesh" => {
                let local = compose_transform(node.get("transform"), &nctx)?;
                let model = math::mul(parent, &local);
                let geo_spec = req(node, "geometry", &nctx)?;
                if let Some(texref) = node.get("texture") {
                    let (id, wrap) = match texref {
                        Value::Str(id) => (id.as_str(), "clamp"),
                        Value::Obj(_) => (
                            str_field(texref, "id", &nctx)?,
                            texref.get("wrap").and_then(Value::as_str).unwrap_or("clamp"),
                        ),
                        _ => return Err(err(&nctx, "'texture' must be a string or an object")),
                    };
                    let tex = tex_index
                        .get(id)
                        .ok_or_else(|| err(&nctx, &format!("unknown texture id '{id}'")))?;
                    draws.push(Draw::Tex(TexDraw {
                        geo: tex_geo(geo_spec, cam, &nctx)?,
                        model,
                        tex: *tex,
                        repeat: wrap == "repeat",
                    }));
                } else {
                    let mat = req(node, "material", &nctx)?;
                    // "roughness" is carried for M5's PBR shader; the
                    // v1 fixed-light shader does not consume it. Type
                    // checked here so a malformed file fails loudly.
                    if let Some(r) = mat.get("roughness") {
                        num(r, &nctx)?;
                    }
                    let colour = material_colour(mat, &nctx)?;
                    draws.push(Draw::Colour(ColourDraw {
                        geo: colour_geo(geo_spec, colour, &nctx)?,
                        model,
                    }));
                }
            }
            "scatter" => {
                let local = compose_transform(node.get("transform"), &nctx)?;
                let model = math::mul(parent, &local);
                let mat = req(node, "material", &nctx)?;
                let colour = material_colour(mat, &nctx)?;
                let base = colour_geo(req(node, "geometry", &nctx)?, colour, &nctx)?;
                draws.push(Draw::Colour(ColourDraw {
                    geo: scatter_geo(base, node, &nctx)?,
                    model,
                }));
            }
            "code-ref" => {
                let id = str_field(node, "id", &nctx)?;
                match id {
                    "ab1-onoda-rifle" => {
                        let (_, rifle) = crate::ab1::onoda_card_and_rifle();
                        for d in rifle {
                            draws.push(Draw::Colour(ColourDraw {
                                geo: d.geo,
                                model: math::mul(parent, &d.model),
                            }));
                        }
                    }
                    "ab1-onoda-card" => {
                        let (card, _) = crate::ab1::onoda_card_and_rifle();
                        let tex = tex_index.get("onoda-card").ok_or_else(|| {
                            err(&nctx, "code-ref 'ab1-onoda-card' needs a texture with id 'onoda-card'")
                        })?;
                        draws.push(Draw::Tex(TexDraw {
                            geo: card.geo,
                            model: math::mul(parent, &card.model),
                            tex: *tex,
                            repeat: false,
                        }));
                    }
                    other => return Err(err(&nctx, &format!("unknown code-ref id '{other}'"))),
                }
            }
            "character" => {
                // Format v2 (spec §14.3): a real character node —
                // the full painter (src/toonfull.rs) replaces the
                // v1 code-ref escape hatch. The node is evaluated
                // at the file's `time` by construction: the state
                // in the file IS the state at that time.
                use crate::toonfull::{FullLook, FullState, Legs, Mood, Plate};
                let pos = vec3_field(node, "position", &nctx)?;
                let height = f32_field_or(node, "height", &nctx, 1.75)?;
                let h_t = height / 0.96;
                let perspective = truthy(node.get("perspective"));
                let look_spec = node.get("look").cloned().unwrap_or(Value::Null);
                let extra: Vec<String> = look_spec
                    .get("extra")
                    .and_then(Value::as_arr)
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                let look = FullLook {
                    body: opt_str(&look_spec, "body").unwrap_or_else(|| "#E8734A".to_string()),
                    head: opt_str(&look_spec, "head").unwrap_or_else(|| "#F7F7FA".to_string()),
                    shade: opt_str(&look_spec, "shade"),
                    pattern: opt_str(&look_spec, "pattern").unwrap_or_else(|| "plain".to_string()),
                    pat_col: opt_str(&look_spec, "patCol"),
                    hat: opt_str(&look_spec, "hat").unwrap_or_else(|| "none".to_string()),
                    hat_col: opt_str(&look_spec, "hatCol"),
                    band: opt_str(&look_spec, "band"),
                    neck: opt_str(&look_spec, "neck").unwrap_or_else(|| "none".to_string()),
                    neck_col: opt_str(&look_spec, "neckCol"),
                    glasses: extra.iter().any(|e| e == "glasses"),
                    mustache: extra.iter().any(|e| e == "mustache"),
                    mustache_col: opt_str(&look_spec, "mustacheCol"),
                    kid: truthy(look_spec.get("kid")),
                };
                let st_spec = node.get("state").cloned().unwrap_or(Value::Null);
                let plate_of = |key: &str| -> Result<Option<Plate>, String> {
                    match st_spec.get(key) {
                        None => Ok(None),
                        Some(h) => {
                            let id = str_field(h, "plate", &nctx)?;
                            let ti = tex_index.get(id).ok_or_else(|| {
                                err(&nctx, &format!("hold plate texture '{id}' not declared"))
                            })?;
                            let (tpx, tw, th) = &textures[*ti];
                            Ok(Some(Plate {
                                px: tpx.clone(),
                                w: *tw,
                                h: *th,
                                x0: f32_field_or(h, "x0", &nctx, 0.0)?,
                                y0: f32_field_or(h, "y0", &nctx, 0.0)?,
                            }))
                        }
                    }
                };
                let state = FullState {
                    mood: Mood::from_str(opt_str(&st_spec, "mood").as_deref().unwrap_or("neutral")),
                    look: { let (a, b) = pair_field(&st_spec, "look", &nctx, (0.0, 0.0))?; [a, b] },
                    blink: f32_field_or(&st_spec, "blink", &nctx, 0.0)?,
                    talk: f32_field_or(&st_spec, "talk", &nctx, 0.0)?,
                    sweat: truthy(st_spec.get("sweat")),
                    tears: truthy(st_spec.get("tears")),
                    arm_l: pair_field(&st_spec, "armL", &nctx, (8.0, 6.0))?,
                    arm_r: pair_field(&st_spec, "armR", &nctx, (8.0, 6.0))?,
                    legs: match opt_str(&st_spec, "legs").as_deref() {
                        Some("walk") => Legs::Walk,
                        Some("run") => Legs::Run,
                        Some("sit") => Legs::Sit,
                        Some("jump") => Legs::Jump,
                        _ => Legs::Stand,
                    },
                    phase: f32_field_or(&st_spec, "phase", &nctx, 0.0)?,
                    lean: f32_field_or(&st_spec, "lean", &nctx, 0.0)?,
                    bob: f32_field_or(&st_spec, "bob", &nctx, 0.0)?,
                    squash: f32_field_or(&st_spec, "squash", &nctx, 0.0)?,
                    head_tilt: f32_field_or(&st_spec, "headTilt", &nctx, 0.0)?,
                    head_turn: f32_field_or(&st_spec, "headTurn", &nctx, 0.0)?,
                    cam_pitch: f32_field_or(&st_spec, "camPitch", &nctx, 0.0)?,
                    sit_drop: match st_spec.get("sitOn") {
                        Some(v) => Some((num(v, &nctx)? as f32) * 1024.0 / h_t),
                        None => None,
                    },
                    hold_l: plate_of("holdL")?,
                    hold_r: plate_of("holdR")?,
                };
                let (pxbuf, pw, ph, hand_l, hand_r) = crate::toonfull::paint_toon_full(&look, &state);
                textures.push((pxbuf, pw, ph));
                let card_tex = textures.len() - 1;
                textures.push(scene::paint_blob());
                let blob_tex = textures.len() - 1;
                // face(): yaw toward the (evaluated) camera, plus
                // the perspective pitch when the node opts in.
                let yaw = (cam.eye[0] - pos[0]).atan2(cam.eye[2] - pos[2]);
                let pitch = if perspective {
                    let dy = cam.eye[1] - (pos[1] + h_t * 0.6);
                    let dh = ((cam.eye[0] - pos[0]).powi(2) + (cam.eye[2] - pos[2]).powi(2)).sqrt();
                    if dh < 1e-4 {
                        0.0
                    } else {
                        (dy.atan2(dh) * 0.45).clamp(-0.38, 0.38)
                    }
                } else {
                    0.0
                };
                let model = math::mul(
                    parent,
                    &math::mul(
                        &math::translate(pos[0], pos[1], pos[2]),
                        &math::mul(&math::rotate_y(yaw), &math::rotate_x(pitch)),
                    ),
                );
                let su = h_t / 1024.0;
                let plane_w = h_t * 1280.0 / 1024.0;
                let plane_h = h_t * 1408.0 / 1024.0;
                draws.push(Draw::Tex(TexDraw {
                    geo: flat_quad_geo((h_t / 2.0) * 0.9, (h_t / 2.0) * 0.34),
                    model: math::mul(parent, &math::translate(pos[0], pos[1] + 0.01, pos[2])),
                    tex: blob_tex,
                    repeat: false,
                }));
                draws.push(Draw::Tex(TexDraw {
                    geo: quad_geo(plane_w, plane_h, 552.0 * su - plane_h / 2.0),
                    model,
                    tex: card_tex,
                    repeat: false,
                }));
                // Attachments (spec §14.5 R2): declared ON the
                // character node; the rifle is anchored at the
                // painted hand via the syncAttachments card-space
                // offset, exactly as the film evaluator places it.
                if let Some(atts) = node.get("attachments").and_then(Value::as_arr) {
                    for att in atts {
                        let kind = str_field(att, "kind", &nctx)?;
                        if kind != "rifle" {
                            return Err(err(&nctx, &format!("unknown attachment kind '{kind}'")));
                        }
                        let (hx, hy) = if opt_str(att, "hand").as_deref() == Some("L") { hand_l } else { hand_r };
                        let off = att.get("offset").and_then(Value::as_arr)
                            .map(|a| -> Result<[f32; 3], String> {
                                Ok([num(a.first().unwrap_or(&Value::Null), &nctx)? as f32,
                                    num(a.get(1).unwrap_or(&Value::Null), &nctx)? as f32,
                                    num(a.get(2).unwrap_or(&Value::Null), &nctx)? as f32])
                            })
                            .transpose()?
                            .unwrap_or([0.0, 0.03, 0.07]);
                        let local = [(384.0 + hx - 640.0) * su, (1256.0 - (256.0 + hy)) * su, 0.0];
                        let rifle_model = math::mul(
                            &model,
                            &math::translate(local[0] + off[0], local[1] + off[1], local[2] + off[2]),
                        );
                        let wood = crate::scene::hex_lin("#6E4F30");
                        let steel = crate::scene::hex_lin("#3A3D42");
                        let parts: [(Geo, [f32; 3], [f32; 3]); 5] = [
                            (box_geo(0.075, 0.52, 0.13, wood), [0.0, -0.17, 0.0], wood),
                            (box_geo(0.06, 0.1, 0.09, steel), [0.0, 0.13, 0.0], steel),
                            (crate::scene::cyl_geo(0.016, 0.016, 0.55, 8, steel), [0.0, 0.42, 0.0], steel),
                            (box_geo(0.02, 0.05, 0.02, steel), [0.0, 0.68, 0.0], steel),
                            (box_geo(0.05, 0.05, 0.16, wood), [0.0, -0.02, 0.05], wood),
                        ];
                        for (geo, pl, _c) in parts {
                            draws.push(Draw::Colour(ColourDraw {
                                geo,
                                model: math::mul(&rifle_model, &math::translate(pl[0], pl[1], pl[2])),
                            }));
                        }
                    }
                }
            }
            other => return Err(err(&nctx, &format!("unknown node type '{other}'"))),
        }
    }
    Ok(())
}

/// motion.ts smootherstep (the camK easing): u^3 (u (6u - 15) + 10),
/// clamped to [0, 1]. Computed in f64, as the source does.
fn smootherstep(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u * u * (u * (u * 6.0 - 15.0) + 10.0)
}

/// Evaluate format v2 camera keys at `time`: returns (eye, target,
/// fovY). Keys are { t, eye, target, fovY?, ease? }; before the
/// first / after the last key the end key holds.
fn eval_camera_keys(
    keys: &[Value],
    time: f64,
    base: &Camera,
) -> Result<([f32; 3], [f32; 3], f32), String> {
    let key_at = |k: &Value| -> Result<(f64, [f32; 3], [f32; 3], f32, bool), String> {
        let t = num_field(k, "t", "scene.camera.keys")?;
        let eye = vec3_field(k, "eye", "scene.camera.keys")?;
        let target = vec3_field(k, "target", "scene.camera.keys")?;
        let fov = match k.get("fovY") {
            Some(v) => num(v, "scene.camera.keys.fovY")? as f32,
            None => base.fov_y_deg,
        };
        let linear = k.get("ease").and_then(Value::as_str) == Some("linear");
        Ok((t, eye, target, fov, linear))
    };
    let first = key_at(&keys[0])?;
    if time <= first.0 || keys.len() == 1 {
        return Ok((first.1, first.2, first.3));
    }
    for w in keys.windows(2) {
        let a = key_at(&w[0])?;
        let b = key_at(&w[1])?;
        if time <= b.0 {
            let span = b.0 - a.0;
            let raw = if span > 0.0 { (time - a.0) / span } else { 1.0 };
            let u = if b.4 { raw.clamp(0.0, 1.0) } else { smootherstep(raw) } as f32;
            let mix3 = |x: [f32; 3], y: [f32; 3]| {
                [
                    x[0] + (y[0] - x[0]) * u,
                    x[1] + (y[1] - x[1]) * u,
                    x[2] + (y[2] - x[2]) * u,
                ]
            };
            return Ok((mix3(a.1, b.1), mix3(a.2, b.2), a.3 + (b.3 - a.3) * u));
        }
    }
    let last = key_at(&keys[keys.len() - 1])?;
    Ok((last.1, last.2, last.3))
}

fn vec3_field_or(v: &Value, key: &str, ctx: &str, default: [f32; 3]) -> Result<[f32; 3], String> {
    match v.get(key) {
        Some(x) => vec3(x, ctx),
        None => Ok(default),
    }
}

fn load(path: &str) -> Result<Loaded, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read scene file {path}: {e}"))?;
    let root = scene::parse_json(&text).map_err(|e| format!("{path}: {e}"))?;
    if let Some(f) = root.get("format") {
        if f.as_str() != Some("talerender-scene") {
            return Err(format!("{path}: 'format' must be \"talerender-scene\""));
        }
    }
    let version: u32 = match root.get("version").and_then(Value::as_f64) {
        Some(1.0) => 1,
        Some(2.0) => 2,
        other => {
            return Err(format!(
                "{path}: unsupported scene format version {other:?} (this loader reads versions 1 and 2)"
            ))
        }
    };
    // §15.1 reservation guard (scene-format §15, adoption step
    // 1): the v2 extensions' reserved keys fail LOUD with a
    // named error — never silently ignored under §1's rule.
    // Until the implementations land (§15.10), check_prefabs.py
    // / check_clips.py are the only consumers of these keys.
    if root.get("prefabs").is_some() {
        return Err(format!("{path}: prefab extension not supported"));
    }
    if root.get("clips").is_some() {
        return Err(format!("{path}: clips extension not supported"));
    }
    // §15.1: the instance fields "clip"/"clipTime" are reserved
    // on the camera/lights/post/fog blocks as well as on nodes
    // (nodes are guarded in walk_nodes).
    for block in ["camera", "lights", "post", "fog"] {
        if let Some(b) = root.get(block) {
            if b.get("clip").is_some() || b.get("clipTime").is_some() {
                return Err(format!("{path}: clips extension not supported"));
            }
        }
    }
    // Format v2: the shot's evaluation time. v1 files are stills;
    // a v2 file's camera keys and character state are read at this
    // t (the state in the file is the state at t by construction).
    let time: f32 = match root.get("time") {
        Some(v) => num(v, "scene.time")? as f32,
        None => 0.0,
    };
    let name = root
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("(unnamed)")
        .to_string();

    let cam_spec = req(&root, "camera", "scene")?;
    let mut camera = Camera {
        fov_y_deg: f32_field(cam_spec, "fovY", "scene.camera")?,
        near: match cam_spec.get("near") {
            Some(v) => num(v, "scene.camera.near")? as f32,
            None => 0.05,
        },
        far: match cam_spec.get("far") {
            Some(v) => num(v, "scene.camera.far")? as f32,
            None => 800.0,
        },
        eye: vec3_field(cam_spec, "eye", "scene.camera")?,
        target: vec3_field(cam_spec, "target", "scene.camera")?,
        up: match cam_spec.get("up") {
            Some(v) => vec3(v, "scene.camera.up")?,
            None => [0.0, 1.0, 0.0],
        },
    };
    // Format v2 camera keys: keyframed eye/target/fovY evaluated
    // at `time` (smootherstep between keys, the engine's camK
    // easing; a key with "ease": "linear" eases its incoming
    // segment linearly). Static eye/target remain the fallback
    // and the value outside the key range clamps to the end keys.
    if let Some(keys) = cam_spec.get("keys") {
        if version < 2 {
            return Err(err("scene.camera.keys", "camera keys are a format v2 field"));
        }
        let keys = keys.as_arr().ok_or_else(|| err("scene.camera.keys", "must be an array"))?;
        if keys.is_empty() {
            return Err(err("scene.camera.keys", "must not be empty"));
        }
        let eval = eval_camera_keys(keys, time as f64, &camera)?;
        camera.eye = eval.0;
        camera.target = eval.1;
        camera.fov_y_deg = eval.2;
    }

    let clear = match root.get("clear") {
        Some(Value::Str(hex)) => Some(hex_lin(hex)),
        Some(_) => return Err("scene.clear: must be a hex colour string".to_string()),
        None => None,
    };

    let (light_dir, ambient) = match root.get("lighting") {
        Some(l) => {
            let d = req(l, "directional", "scene.lighting")?;
            (
                vec3_field(d, "direction", "scene.lighting.directional")?,
                match d.get("ambient") {
                    Some(v) => num(v, "scene.lighting.directional.ambient")? as f32,
                    None => 0.35,
                },
            )
        }
        None => ([0.5, 0.8, 0.6], 0.35),
    };

    // Format v2 light rig (spec §14.2). v1 files have no `lights`
    // block; a v1 file carrying one is rejected rather than
    // silently reinterpreted.
    let lights: Option<LightsCfg> = match root.get("lights") {
        None => None,
        Some(l) => {
            if version < 2 {
                return Err(err("scene.lights", "the light rig is a format v2 field"));
            }
            let key = match l.get("key") {
                None => None,
                Some(k) => Some(KeyCfg {
                    color: opt_str(k, "color").unwrap_or_else(|| "#FFE2C2".to_string()),
                    intensity: f32_field_or(k, "intensity", "scene.lights.key", 3.2)?,
                    position: vec3_field_or(k, "position", "scene.lights.key", [2.5, 4.0, 3.0])?,
                    target: vec3_field_or(k, "target", "scene.lights.key", [0.0, 1.0, 0.0])?,
                    shadow: truthy(k.get("shadow")),
                    shadow_box: f32_field_or(k, "shadowBox", "scene.lights.key", 3.0)?,
                    far: f32_field_or(k, "far", "scene.lights.key", 20.0)?,
                    bias: f32_field_or(k, "bias", "scene.lights.key", -0.0004)?,
                    normal_bias: f32_field_or(k, "normalBias", "scene.lights.key", 0.02)?,
                }),
            };
            let rim = match l.get("rim") {
                None => None,
                Some(r) => Some(RimCfg {
                    color: opt_str(r, "color").unwrap_or_else(|| "#BFD8FF".to_string()),
                    intensity: f32_field_or(r, "intensity", "scene.lights.rim", 2.2)?,
                    position: vec3_field_or(r, "position", "scene.lights.rim", [-3.0, 3.0, -3.0])?,
                }),
            };
            let hemisphere = match l.get("hemisphere") {
                None => None,
                Some(h) => Some(HemiCfg {
                    sky: opt_str(h, "sky").unwrap_or_else(|| "#FFF4E6".to_string()),
                    ground: opt_str(h, "ground").unwrap_or_else(|| "#8C6E55".to_string()),
                    intensity: f32_field_or(h, "intensity", "scene.lights.hemisphere", 0.9)?,
                }),
            };
            let mut points = Vec::new();
            if let Some(ps) = l.get("points") {
                let ps = ps.as_arr().ok_or_else(|| err("scene.lights.points", "must be an array"))?;
                for (i, p) in ps.iter().enumerate() {
                    let pctx = format!("scene.lights.points[{i}]");
                    points.push(PointCfg {
                        color: opt_str(p, "color").unwrap_or_else(|| "#FFFFFF".to_string()),
                        intensity: f32_field_or(p, "intensity", &pctx, 1.0)?,
                        position: vec3_field(p, "position", &pctx)?,
                        distance: f32_field_or(p, "distance", &pctx, 0.0)?,
                        decay: f32_field_or(p, "decay", &pctx, 2.0)?,
                    });
                }
            }
            Some(LightsCfg {
                key,
                rim,
                hemisphere,
                points,
                env_intensity: f32_field_or(l, "envIntensity", "scene.lights", 0.25)?,
            })
        }
    };

    let scene_dir = Path::new(path).parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    let mut textures = Vec::new();
    let mut tex_index: HashMap<String, usize> = HashMap::new();
    if let Some(list) = root.get("textures") {
        let list = list.as_arr().ok_or_else(|| err("scene.textures", "must be an array"))?;
        for (i, spec) in list.iter().enumerate() {
            let tctx = format!("scene.textures[{i}]");
            let id = str_field(spec, "id", &tctx)?;
            if tex_index.contains_key(id) {
                return Err(err(&tctx, &format!("duplicate texture id '{id}'")));
            }
            tex_index.insert(id.to_string(), textures.len());
            textures.push(resolve_texture(spec, &scene_dir, &tctx)?);
        }
    }

    let mut draws = Vec::new();
    let nodes = req(&root, "nodes", "scene")?
        .as_arr()
        .ok_or_else(|| err("scene.nodes", "must be an array"))?;
    walk_nodes(nodes, &math::identity(), &camera, &tex_index, &mut textures, &mut draws, "scene.nodes")?;

    let bokeh = match root.get("post") {
        Some(post) => match post.get("bokeh") {
            Some(b) => Some(Bokeh {
                focus: f32_field(b, "focus", "scene.post.bokeh")?,
                aperture: f32_field(b, "aperture", "scene.post.bokeh")?,
                maxblur: f32_field(b, "maxblur", "scene.post.bokeh")?,
            }),
            None => None,
        },
        None => None,
    };
    let bloom = match root.get("post").and_then(|p| p.get("bloom")) {
        Some(b) => Some(BloomCfg {
            threshold: f32_field_or(b, "threshold", "scene.post.bloom", 0.85)?,
            strength: f32_field_or(b, "strength", "scene.post.bloom", 0.0)?,
            radius: f32_field_or(b, "radius", "scene.post.bloom", 0.6)?,
        }),
        None => None,
    };
    let grade = match root.get("post").and_then(|p| p.get("grade")) {
        Some(g) => Some(GradeCfg {
            vignette: f32_field_or(g, "vignette", "scene.post.grade", 0.18)?,
            grain: f32_field_or(g, "grain", "scene.post.grade", 0.008)?,
            warm: f32_field_or(g, "warm", "scene.post.grade", 0.02)?,
            sat: f32_field_or(g, "sat", "scene.post.grade", 1.12)?,
            exposure: f32_field_or(g, "exposure", "scene.post.grade", 1.0)?,
            time: f32_field_or(g, "time", "scene.post.grade", 0.0)?,
        }),
        None => None,
    };
    // Format v1 enters the composer path through post.bokeh (the
    // chain's anchor); bloom/grade ride that path, so a file naming
    // them without bokeh is rejected rather than silently dropped.
    if bokeh.is_none() && (bloom.is_some() || grade.is_some()) {
        return Err(err("scene.post", "post.bloom / post.grade require post.bokeh (format v1 enters the composer path via bokeh)"));
    }

    let fog = match root.get("fog") {
        Some(fg) => Some(FogCfg {
            color: scene::hex_lin(str_field(fg, "color", "scene.fog")?),
            near: f32_field(fg, "near", "scene.fog")?,
            far: f32_field(fg, "far", "scene.fog")?,
        }),
        None => None,
    };
    Ok(Loaded { name, version, time, camera, clear, light_dir, ambient, lights, textures, draws, bokeh, bloom, grade, fog })
}

// ---------------------------------------------------------------------
// Rendering helpers (byte packing follows the crate's house pattern:
// native-endian f32/u32 streams, column-major matrices).
// ---------------------------------------------------------------------

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

/// Textured-draw uniforms: {mvp, model} (shader_tex / shader_depth).
fn uniform_bytes_tex(mvp: &Mat4, model: &Mat4) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(&mat_bytes(mvp));
    out.extend_from_slice(&mat_bytes(model));
    out
}

/// Colour-draw uniforms: {mvp, model, light} (shader_scene3d).
fn uniform_bytes_colour(mvp: &Mat4, model: &Mat4, light: [f32; 4]) -> Vec<u8> {
    let mut out = Vec::with_capacity(144);
    out.extend_from_slice(&mat_bytes(mvp));
    out.extend_from_slice(&mat_bytes(model));
    out.extend_from_slice(&pack_f32(&light));
    out
}

fn make_buffer(device: &wgpu::Device, queue: &wgpu::Queue, label: &str, bytes: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: bytes.len() as u64,
        usage,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buf, 0, bytes);
    buf
}

fn upload_texture(device: &wgpu::Device, queue: &wgpu::Queue, px: &[u8], w: u32, h: u32) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene-file texture"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
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
            rows_per_image: Some(h),
        },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

async fn make_device(
    label: &'static str,
) -> crate::errors::TaleResult<(wgpu::Device, wgpu::Queue)> {
    let instance = crate::gpucore::new_instance();
    let adapter = crate::gpucore::request_adapter(&instance).await?;
    let info = adapter.get_info();
    println!(
        "adapter: {} | backend: {:?} | device type: {:?} | driver: {} {}",
        info.name, info.backend, info.device_type, info.driver, info.driver_info
    );
    // Standard limits, not downlevel: the real character canvas
    // is 2560x2816 (see ab1.rs) — request_device_default's law.
    crate::gpucore::request_device_default(&adapter, label).await
}

fn view_proj(cam: &Camera, width: u32, height: u32) -> Mat4 {
    let aspect = width as f32 / height as f32;
    let proj = math::perspective(cam.fov_y_deg.to_radians(), aspect, cam.near, cam.far);
    let view = math::look_at(cam.eye, cam.target, cam.up);
    math::mul(&proj, &view)
}

const COLOUR_ATTRS: [wgpu::VertexAttribute; 3] = [
    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 24, shader_location: 2 },
];
const TEX_ATTRS: [wgpu::VertexAttribute; 2] = [
    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 12, shader_location: 1 },
];

fn uniform_bgl(device: &wgpu::Device, fragment: bool) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("uniform layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: if fragment {
                wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT
            } else {
                wgpu::ShaderStages::VERTEX
            },
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

fn tex_bgl(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    })
}

pub async fn run_scene_file(width: u32, height: u32, out: &str, path: &str, knobs: crate::Knobs) -> crate::errors::TaleResult<()> {
    let loaded = load(path).map_err(|e| crate::errors::TaleError::data_load(path, e))?;
    if loaded.bokeh.is_some() {
        run_composer(width, height, out, &loaded, knobs).await
    } else {
        run_direct(width, height, out, &loaded, knobs).await
    }
}

// ---------------------------------------------------------------------
// Direct path (the ab1 pass structure)
// ---------------------------------------------------------------------

async fn run_direct(width: u32, height: u32, out: &str, scene: &Loaded, knobs: crate::Knobs) -> crate::errors::TaleResult<()> {
    let tag = if knobs.is_draft() {
        format!(" {}", knobs.tag())
    } else {
        String::new()
    };
    let (device, queue) = make_device("talerender scene-file device").await?;
    println!(
        "scene: file — {} (format v{}, direct path, {} draws, {} textures)",
        scene.version,
        scene.name,
        scene.draws.len(),
        scene.textures.len()
    );
    let vp = view_proj(&scene.camera, width, height);
    let light = [scene.light_dir[0], scene.light_dir[1], scene.light_dir[2], scene.ambient];

    // ---- pipelines ----
    let shader_colour = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scene colour shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader_scene3d.wgsl"))),
    });
    let bgl_colour = uniform_bgl(&device, true);
    let pipeline_colour = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scene colour pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene colour layout"),
            bind_group_layouts: &[&bgl_colour],
            push_constant_ranges: &[],
        })),
        vertex: wgpu::VertexState {
            module: &shader_colour,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 36,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &COLOUR_ATTRS,
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_colour,
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
        label: Some("scene textured shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader_tex.wgsl"))),
    });
    let bgl_tex = tex_bgl(&device);
    let pipeline_tex = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scene textured pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene textured layout"),
            bind_group_layouts: &[&bgl_tex],
            push_constant_ranges: &[],
        })),
        vertex: wgpu::VertexState {
            module: &shader_tex,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 20,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &TEX_ATTRS,
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

    // ---- textures + samplers (ab1's pair) ----
    let tex_views: Vec<wgpu::TextureView> = scene
        .textures
        .iter()
        .map(|(px, w, h)| upload_texture(&device, &queue, px, *w, *h))
        .collect();
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

    // ---- prepare draws ----
    struct Prepared {
        colour: bool,
        vb: wgpu::Buffer,
        ib: wgpu::Buffer,
        n: u32,
        bg: wgpu::BindGroup,
        _ub: wgpu::Buffer,
    }
    let mut prepared: Vec<Prepared> = Vec::new();
    for draw in &scene.draws {
        match draw {
            Draw::Colour(d) => {
                let vb = make_buffer(&device, &queue, "vb", &pack_f32(&d.geo.verts), wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST);
                let ib = make_buffer(&device, &queue, "ib", &pack_u32(&d.geo.idx), wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST);
                let ub = make_buffer(
                    &device,
                    &queue,
                    "uniforms",
                    &uniform_bytes_colour(&math::mul(&vp, &d.model), &d.model, light),
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                );
                let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("colour bind group"),
                    layout: &bgl_colour,
                    entries: &[wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() }],
                });
                prepared.push(Prepared { colour: true, vb, ib, n: d.geo.idx.len() as u32, bg, _ub: ub });
            }
            Draw::Tex(d) => {
                let vb = make_buffer(&device, &queue, "vb", &pack_f32(&d.geo.verts), wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST);
                let ib = make_buffer(&device, &queue, "ib", &pack_u32(&d.geo.idx), wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST);
                let ub = make_buffer(
                    &device,
                    &queue,
                    "uniforms",
                    &uniform_bytes_tex(&math::mul(&vp, &d.model), &d.model),
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                );
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
                prepared.push(Prepared { colour: false, vb, ib, n: d.geo.idx.len() as u32, bg, _ub: ub });
            }
        }
    }

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
    let clear = scene.clear.unwrap_or([0.0, 0.0, 0.0]);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("frame encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene-file pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view_tex,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: clear[0] as f64,
                        g: clear[1] as f64,
                        b: clear[2] as f64,
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
        // Document order, switching pipelines per draw kind — the
        // hand-authored scenes interleave the same way.
        for p in &prepared {
            pass.set_pipeline(if p.colour { &pipeline_colour } else { &pipeline_tex });
            pass.set_bind_group(0, &p.bg, &[]);
            pass.set_vertex_buffer(0, p.vb.slice(..));
            pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..p.n, 0, 0..1);
        }
    }
    let rb = crate::gpucore::Readback::record(&device, &mut encoder, &texture, width, height, "readback buffer");
    queue.submit(Some(encoder.finish()));
    let pixels = rb.collect(&device)?;

    let img = image::RgbaImage::from_raw(width, height, pixels).expect("pixel buffer size mismatch");
    img.save(out).map_err(|e| crate::errors::TaleError::image_io(out, e))?;
    println!("wrote {out} ({width}x{height}){tag}");
    Ok(())
}

// ---------------------------------------------------------------------
// Composer path (the dof-chart pass chain): HalfFloat MSAA-4x scene
// -> packed-depth prepass -> transcribed r170 bokeh -> Neutral+sRGB
// output. Setup mirrors dof.rs; the differences are only that draws
// come from the file (per-draw uniforms and textures) and the depth
// prepass reads position-only vertex buffers through
// shader_depth_pos.wgsl (fragment arithmetic identical to
// shader_depth.wgsl).
// ---------------------------------------------------------------------

fn draw_positions(draw: &Draw) -> Vec<f32> {
    let (verts, stride): (&[f32], usize) = match draw {
        Draw::Colour(d) => (&d.geo.verts, 9),
        Draw::Tex(d) => (&d.geo.verts, 5),
    };
    verts.chunks_exact(stride).flat_map(|v| [v[0], v[1], v[2]]).collect()
}

fn draw_indices(draw: &Draw) -> &[u32] {
    match draw {
        Draw::Colour(d) => &d.geo.idx,
        Draw::Tex(d) => &d.geo.idx,
    }
}

fn draw_model(draw: &Draw) -> &Mat4 {
    match draw {
        Draw::Colour(d) => &d.model,
        Draw::Tex(d) => &d.model,
    }
}

async fn run_composer(width: u32, height: u32, out: &str, scene: &Loaded, knobs: crate::Knobs) -> crate::errors::TaleResult<()> {
    let tag = if knobs.is_draft() {
        format!(" {}", knobs.tag())
    } else {
        String::new()
    };
    let samples = knobs.samples();
    let bokeh = scene.bokeh.as_ref().expect("composer path requires post.bokeh");
    let (device, queue) = make_device("talerender scene-file composer device").await?;
    println!(
        "scene: file — {} (format v{}, composer path: HalfFloat MSAA-4x -> packed-depth prepass -> r170 bokeh -> Neutral+sRGB; focus {} m, aperture {}, maxblur {})",
        scene.name, scene.version, bokeh.focus, bokeh.aperture, bokeh.maxblur
    );
    let cam = &scene.camera;
    let vp = view_proj(cam, width, height);
    let aspect = width as f32 / height as f32;
    let light = [scene.light_dir[0], scene.light_dir[1], scene.light_dir[2], scene.ambient];

    // Packed-depth storage: same rule as dof.rs — the byte target is
    // lossless for the byte packing on every driver; the env var
    // restores BokehPass's HalfFloat target configuration.
    let depth_format = if std::env::var("TALERENDER_DOF_DEPTH_FORMAT").is_ok_and(|v| v == "halffloat") {
        wgpu::TextureFormat::Rgba16Float
    } else {
        wgpu::TextureFormat::Rgba8Unorm
    };

    // ---- samplers (dof.rs's set, plus a repeat variant) ----
    let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("composer linear sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });
    let repeat_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("composer repeat sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });
    let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("composer nearest sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });

    // ---- targets ----
    let scene_msaa = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Rgba16Float, samples, wgpu::TextureUsages::RENDER_ATTACHMENT, "composer-scene-msaa");
    let scene_resolved = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Rgba16Float, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, "composer-scene-resolved");
    let scene_depth = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Depth32Float, samples, wgpu::TextureUsages::RENDER_ATTACHMENT, "composer-scene-depth");
    let packed_depth = crate::gpucore::make_target(&device, width, height, depth_format, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC, "composer-packed-depth");
    let prepass_depth = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Depth32Float, 1, wgpu::TextureUsages::RENDER_ATTACHMENT, "composer-prepass-depth");
    let bokeh_out = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Rgba16Float, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, "composer-bokeh-out");
    let final_tex = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Rgba8Unorm, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, "composer-final");
    let scene_view = scene_resolved.create_view(&wgpu::TextureViewDescriptor::default());
    let packed_view = packed_depth.create_view(&wgpu::TextureViewDescriptor::default());
    // Fog (format v2 `fog` block): the resolved HDR is fogged into
    // this target after the depth prepass and bokeh reads IT
    // instead of the raw scene target. Built only when the file
    // declares fog — absent block, the path is bit-identical.
    let fog_tex = scene.fog.as_ref().map(|_| {
        crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Rgba16Float, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, "composer-fogged")
    });
    let fog_view = fog_tex.as_ref().map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()));
    let bokeh_view = bokeh_out.create_view(&wgpu::TextureViewDescriptor::default());

    // ---- scene pipelines (HalfFloat target, MSAA 4x, no culling —
    // dof.rs's scene-pass configuration) ----
    let bgl_tex = tex_bgl(&device);
    let shader_tex = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("composer textured shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader_tex.wgsl"))),
    });
    let scene_tex_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("composer scene tex pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("composer scene tex layout"),
            bind_group_layouts: &[&bgl_tex],
            push_constant_ranges: &[],
        })),
        vertex: wgpu::VertexState {
            module: &shader_tex,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 20,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &TEX_ATTRS,
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_tex,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
        multiview: None,
    });

    let bgl_colour = uniform_bgl(&device, true);
    let shader_colour = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("composer colour shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader_scene3d.wgsl"))),
    });
    let scene_colour_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("composer scene colour pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("composer scene colour layout"),
            bind_group_layouts: &[&bgl_colour],
            push_constant_ranges: &[],
        })),
        vertex: wgpu::VertexState {
            module: &shader_colour,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 36,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &COLOUR_ATTRS,
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_colour,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
        multiview: None,
    });

    // ---- depth prepass pipeline (position-only) ----
    let bgl_depth = uniform_bgl(&device, false);
    let shader_depth = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("composer depth shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader_depth_pos.wgsl"))),
    });
    let depth_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("composer depth pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("composer depth layout"),
            bind_group_layouts: &[&bgl_depth],
            push_constant_ranges: &[],
        })),
        vertex: wgpu::VertexState {
            module: &shader_depth,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                }],
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_depth,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: depth_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // ---- fog pass (shader_fog.wgsl): three fog_fragment over
    // the packed depth, linear, pre-bokeh ----
    let fog_assets = scene.fog.as_ref().map(|fg| {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("composer fog shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader_fog.wgsl"))),
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("composer fog layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let ub = make_buffer(&device, &queue, "fog uniforms", &pack_f32(&[
            fg.color[0], fg.color[1], fg.color[2], fg.near,
            fg.far, cam.near, cam.far, 0.0,
        ]), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composer fog bind group"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&scene_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&linear_sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&packed_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&nearest_sampler) },
            ],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("composer fog pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("composer fog pipeline layout"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState { module: &module, entry_point: "vs_main", buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        (pipeline, bg, ub)
    });

    // ---- bokeh + output pipelines (dof.rs's, unchanged shaders) ----
    let bokeh_uniform_data: [f32; 8] = [bokeh.focus, aspect, bokeh.aperture, bokeh.maxblur, cam.near, cam.far, 0.0, 0.0];
    let bokeh_uniform_buf = make_buffer(&device, &queue, "bokeh uniforms", &pack_f32(&bokeh_uniform_data), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
    let bokeh_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("composer bokeh shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader_bokeh.wgsl"))),
    });
    let bokeh_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("composer bokeh layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ],
    });
    let bokeh_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("composer bokeh bind group"),
        layout: &bokeh_bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: bokeh_uniform_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(fog_view.as_ref().unwrap_or(&scene_view)) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&linear_sampler) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&packed_view) },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&nearest_sampler) },
        ],
    });
    let bokeh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("composer bokeh pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("composer bokeh pipeline layout"),
            bind_group_layouts: &[&bokeh_bgl],
            push_constant_ranges: &[],
        })),
        vertex: wgpu::VertexState { module: &bokeh_module, entry_point: "vs_main", buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState {
            module: &bokeh_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let output_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("composer output shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader_output_exp.wgsl"))),
    });
    let output_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("composer output layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
        ],
    });
    // post.grade.exposure reaches the OutputPass uniform (the
    // composer's exposure slot, as in film.rs); absent grade -> 1.0,
    // where the exp shader reduces to the old hardcoded output
    // bit-for-bit.
    let exposure = scene.grade.as_ref().map(|g| g.exposure).unwrap_or(1.0);
    let output_ub = make_buffer(&device, &queue, "output uniforms", &pack_f32(&[exposure, 0.0, 0.0, 0.0]), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
    let output_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("composer output bind group"),
        layout: &output_bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&bokeh_view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&linear_sampler) },
            wgpu::BindGroupEntry { binding: 2, resource: output_ub.as_entire_binding() },
        ],
    });
    let output_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("composer output pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("composer output pipeline layout"),
            bind_group_layouts: &[&output_bgl],
            push_constant_ranges: &[],
        })),
        vertex: wgpu::VertexState { module: &output_module, entry_point: "vs_main", buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState {
            module: &output_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    // ---- textures + prepared draws ----
    let tex_views: Vec<wgpu::TextureView> = scene
        .textures
        .iter()
        .map(|(px, w, h)| upload_texture(&device, &queue, px, *w, *h))
        .collect();
    struct Prepared {
        colour: bool,
        vb: wgpu::Buffer,
        ib: wgpu::Buffer,
        n: u32,
        scene_bg: wgpu::BindGroup,
        depth_vb: wgpu::Buffer,
        depth_bg: wgpu::BindGroup,
        _ubs: Vec<wgpu::Buffer>,
    }
    let mut prepared: Vec<Prepared> = Vec::new();
    for draw in &scene.draws {
        let model = draw_model(draw);
        let mvp = math::mul(&vp, model);
        let idx = draw_indices(draw);
        let ib = make_buffer(&device, &queue, "ib", &pack_u32(idx), wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST);
        let depth_vb = make_buffer(&device, &queue, "depth vb", &pack_f32(&draw_positions(draw)), wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST);
        let depth_ub = make_buffer(&device, &queue, "depth uniforms", &uniform_bytes_tex(&mvp, model), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
        let depth_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("depth bind group"),
            layout: &bgl_depth,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: depth_ub.as_entire_binding() }],
        });
        let (colour, vb, scene_bg, ub) = match draw {
            Draw::Colour(d) => {
                let vb = make_buffer(&device, &queue, "vb", &pack_f32(&d.geo.verts), wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST);
                let ub = make_buffer(&device, &queue, "uniforms", &uniform_bytes_colour(&mvp, &d.model, light), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
                let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("scene colour bind group"),
                    layout: &bgl_colour,
                    entries: &[wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() }],
                });
                (true, vb, bg, ub)
            }
            Draw::Tex(d) => {
                let vb = make_buffer(&device, &queue, "vb", &pack_f32(&d.geo.verts), wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST);
                let ub = make_buffer(&device, &queue, "uniforms", &uniform_bytes_tex(&mvp, &d.model), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
                let sampler = if d.repeat { &repeat_sampler } else { &linear_sampler };
                let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("scene tex bind group"),
                    layout: &bgl_tex,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex_views[d.tex]) },
                        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
                    ],
                });
                (false, vb, bg, ub)
            }
        };
        prepared.push(Prepared {
            colour,
            vb,
            ib,
            n: idx.len() as u32,
            scene_bg,
            depth_vb,
            depth_bg,
            _ubs: vec![ub, depth_ub],
        });
    }

    // ---- M5 post stages (created only when the file asks) ----
    let post_chain = if scene.bloom.is_some() || scene.grade.is_some() {
        Some(crate::composer::PostChain::new(&device))
    } else {
        None
    };
    let grade_tex = scene.grade.as_ref().map(|_| {
        crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Rgba8Unorm, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, "composer-grade-src")
    });

    // ---- passes (dof.rs's order) ----
    let clear = scene.clear.unwrap_or([0.0, 0.0, 0.0]);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("composer encoder"),
    });
    {
        let msaa_view = scene_msaa.create_view(&wgpu::TextureViewDescriptor::default());
        let sdepth_view = scene_depth.create_view(&wgpu::TextureViewDescriptor::default());
        // Phase 2 (W54 s4 shape): view/resolve chosen together;
        // field-for-field the pre-knob attachment at samples > 1.
        let scene_attachment = wgpu::RenderPassColorAttachment {
            view: if samples > 1 { &msaa_view } else { &scene_view },
            resolve_target: if samples > 1 { Some(&scene_view) } else { None },
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: clear[0] as f64,
                    g: clear[1] as f64,
                    b: clear[2] as f64,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composer scene pass"),
            color_attachments: &[Some(scene_attachment)],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &sdepth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        for p in &prepared {
            pass.set_pipeline(if p.colour { &scene_colour_pipeline } else { &scene_tex_pipeline });
            pass.set_bind_group(0, &p.scene_bg, &[]);
            pass.set_vertex_buffer(0, p.vb.slice(..));
            pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..p.n, 0, 0..1);
        }
    }
    {
        let packed_v = packed_depth.create_view(&wgpu::TextureViewDescriptor::default());
        let pdepth_view = prepass_depth.create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composer depth prepass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &packed_v,
                resolve_target: None,
                // BokehPass clears its depth target to WHITE (dof.rs).
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &pdepth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&depth_pipeline);
        for p in &prepared {
            pass.set_bind_group(0, &p.depth_bg, &[]);
            pass.set_vertex_buffer(0, p.depth_vb.slice(..));
            pass.set_index_buffer(p.ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..p.n, 0, 0..1);
        }
    }
    if let (Some((fog_pipeline, fog_bg, _fog_ub)), Some(fv)) = (&fog_assets, &fog_view) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composer fog pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: fv,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(fog_pipeline);
        pass.set_bind_group(0, fog_bg, &[]);
        pass.draw(0..3, 0..1);
    }
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composer bokeh pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &bokeh_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&bokeh_pipeline);
        pass.set_bind_group(0, &bokeh_bg, &[]);
        pass.draw(0..3, 0..1);
    }
    // M5: UnrealBloomPass between bokeh and output (additively over
    // the bokeh result, which is the pass's read buffer).
    if let (Some(chain), Some(b)) = (&post_chain, &scene.bloom) {
        chain.bloom(
            &device,
            &queue,
            &mut encoder,
            &bokeh_view,
            &bokeh_view,
            width,
            height,
            &crate::composer::BloomParams { threshold: b.threshold, strength: b.strength, radius: b.radius },
        );
    }
    {
        // With GRADE present the output stage lands in the grade
        // input; otherwise it is the final image, as before.
        let out_view = match &grade_tex {
            Some(t) => t.create_view(&wgpu::TextureViewDescriptor::default()),
            None => final_tex.create_view(&wgpu::TextureViewDescriptor::default()),
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composer output pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &out_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&output_pipeline);
        pass.set_bind_group(0, &output_bg, &[]);
        pass.draw(0..3, 0..1);
    }
    // M5: GRADE after output (sRGB space), into the final target.
    if let (Some(chain), Some(g), Some(t)) = (&post_chain, &scene.grade, &grade_tex) {
        let src_view = t.create_view(&wgpu::TextureViewDescriptor::default());
        let dst_view = final_tex.create_view(&wgpu::TextureViewDescriptor::default());
        chain.grade(
            &device,
            &queue,
            &mut encoder,
            &src_view,
            &dst_view,
            &crate::composer::GradeParams { vignette: g.vignette, grain: g.grain, warm: g.warm, sat: g.sat, time: g.time },
        );
    }

    let rb = crate::gpucore::Readback::record(&device, &mut encoder, &final_tex, width, height, "composer readback");
    queue.submit(Some(encoder.finish()));
    let rgba = rb.collect(&device)?;
    image::save_buffer(out, &rgba, width, height, image::ColorType::Rgba8)
        .map_err(|e| crate::errors::TaleError::image_io(out, e))?;
    println!("wrote {out} ({width}x{height}){tag}");
    Ok(())
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_camera() -> Camera {
        Camera {
            fov_y_deg: 30.0,
            near: 0.05,
            far: 800.0,
            eye: [0.0, 0.0, 0.0],
            target: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
        }
    }

    #[test]
    fn transform_ops_compose_left_to_right() {
        let v = scene::parse_json(
            r##"[{"translate": [1.0, 0.0, 0.0]}, {"rotate_y": 1.5707963705062866}]"##,
        )
        .unwrap();
        let m = compose_transform(Some(&v), "t").unwrap();
        // m = T * Ry: (1,0,0) -> Ry -> (0,0,-1) -> T -> (1,0,-1).
        let p = math::mul_vec4(&m, [1.0, 0.0, 0.0, 1.0]);
        assert!((p[0] - 1.0).abs() < 1e-6 && p[1].abs() < 1e-6 && (p[2] + 1.0).abs() < 1e-6, "{p:?}");
        // Reversed op order must give the different, expected result.
        let v2 = scene::parse_json(
            r##"[{"rotate_y": 1.5707963705062866}, {"translate": [1.0, 0.0, 0.0]}]"##,
        )
        .unwrap();
        let m2 = compose_transform(Some(&v2), "t").unwrap();
        let p2 = math::mul_vec4(&m2, [1.0, 0.0, 0.0, 1.0]);
        assert!(p2[0].abs() < 1e-6 && (p2[2] + 2.0).abs() < 1e-6, "{p2:?}");
    }

    #[test]
    fn shade_hash_colour_matches_the_hand_written_expression() {
        let spec = scene::parse_json(
            r##"{"color": {"hex": "#5E8F52", "shadeHash": {"index": 3, "base": 0.92, "span": 0.16}}}"##,
        )
        .unwrap();
        let got = material_colour(&spec, "t").unwrap();
        let expect = shade_lin("#5E8F52", (0.92 + scene::rnd(3.0) * 0.16) as f32);
        assert_eq!(got, expect);
        // Plain shade and plain hex paths.
        let spec2 = scene::parse_json(r##"{"color": {"hex": "#5E8F52", "shade": 0.75}}"##).unwrap();
        assert_eq!(material_colour(&spec2, "t").unwrap(), shade_lin("#5E8F52", 0.75));
        let spec3 = scene::parse_json(r##"{"color": "#5E8F52"}"##).unwrap();
        assert_eq!(material_colour(&spec3, "t").unwrap(), hex_lin("#5E8F52"));
    }

    #[test]
    fn scatter_reproduces_the_sin_hash_placement() {
        let base = cone_geo(0.05, 0.34, 4, [0.0; 3]);
        let spec = scene::parse_json(
            r##"{"count": 3, "placement": {"kind": "annulus", "angleHash": 1.31,
               "radiusHash": 2.7, "radiusBase": 2.6, "radiusSpan": 20.0,
               "angleScale": 6.283185307, "center": [0.0, 0.15, -3.0]}}"##,
        )
        .unwrap();
        let merged = scatter_geo(base.clone(), &spec, "t").unwrap();
        let per = base.verts.len();
        assert_eq!(merged.verts.len(), per * 3);
        // Instance 0's offset, from the hash law written out here.
        let a = scene::rnd(0.0) * 6.283185307;
        let r = 2.6 + scene::rnd(0.0) * 20.0;
        let off = [(a.cos() * r) as f32, 0.15f32, (a.sin() * r) as f32 - 3.0];
        for c in 0..3 {
            let got = merged.verts[c] - base.verts[c];
            assert!((got - off[c]).abs() < 1e-6, "component {c}: {got} vs {}", off[c]);
        }
        // Instance 1 sits at a different offset (hash actually varies).
        let got1 = merged.verts[per] - base.verts[0];
        assert!((got1 - off[0]).abs() > 1e-3, "instance 1 should not reuse instance 0's offset");
    }

    #[test]
    fn dof_panel_node_matches_the_hand_authored_panel_formula() {
        // dof.rs panel_geometry for the focal panel (0, 0, 10 m),
        // recomputed here from its constants.
        let aspect = 1080.0f32 / 1920.0f32;
        let tan_h = (30.0f32 * 0.5).to_radians().tan();
        let d = 10.0f32;
        let hh = d * tan_h;
        let hw = hh * aspect;
        let (w, h) = (0.56f32 * hw, 0.36f32 * hh);
        let json = format!(
            r##"[{{"type": "mesh", "geometry": {{"kind": "quad", "width": {w}, "height": {h}, "y0": {}}},
               "texture": "chart", "transform": [{{"translate": [0.0, 0.0, {}]}}]}}]"##,
            -h / 2.0,
            -d
        );
        let nodes = scene::parse_json(&json).unwrap();
        let nodes = nodes.as_arr().unwrap();
        let mut tex_index = HashMap::new();
        tex_index.insert("chart".to_string(), 0usize);
        let mut draws = Vec::new();
        walk_nodes(nodes, &math::identity(), &test_camera(), &tex_index, &mut Vec::new(), &mut draws, "t").unwrap();
        assert_eq!(draws.len(), 1);
        let Draw::Tex(td) = &draws[0] else { panic!("expected a textured draw") };
        // Local geometry: BL corner at (-w/2, -h/2, 0), uv (0, 1).
        assert!((td.geo.verts[0] - (-w / 2.0)).abs() < 1e-6);
        assert!((td.geo.verts[1] - (-h / 2.0)).abs() < 1e-6);
        assert_eq!(td.geo.verts[3], 0.0);
        assert_eq!(td.geo.verts[4], 1.0);
        // Model: pure translation to (0, 0, -10) — so the world BL
        // corner equals dof.rs's (cx - w/2, cy - h/2, -d).
        let world = math::mul_vec4(&td.model, [td.geo.verts[0], td.geo.verts[1], 0.0, 1.0]);
        assert!((world[0] - (-w / 2.0)).abs() < 1e-6);
        assert!((world[1] - (-h / 2.0)).abs() < 1e-6);
        assert!((world[2] - (-d)).abs() < 1e-6);
    }

    #[test]
    fn load_rejects_other_versions_and_loads_a_minimal_scene() {
        let dir = std::env::temp_dir();
        let bad = dir.join("talerender-test-v3.scene.json");
        std::fs::write(&bad, r##"{"version": 3, "camera": {}, "nodes": []}"##).unwrap();
        let e = load(bad.to_str().unwrap()).err().expect("version 3 must be rejected");
        assert!(e.contains("version"), "{e}");

        let good = dir.join("talerender-test-v1.scene.json");
        std::fs::write(
            &good,
            r##"{"format": "talerender-scene", "version": 1, "name": "minimal",
                "camera": {"fovY": 30.0, "eye": [0, 0, 0], "target": [0, 0, -1]},
                "nodes": [{"type": "mesh",
                           "geometry": {"kind": "box", "size": [1, 1, 1]},
                           "material": {"color": "#FF0000"},
                           "transform": [{"translate": [0, 0, -5]}]}]}"##,
        )
        .unwrap();
        let loaded = load(good.to_str().unwrap()).unwrap();
        assert_eq!(loaded.name, "minimal");
        assert_eq!(loaded.draws.len(), 1);
        assert!(loaded.bokeh.is_none());
        // House-light defaults.
        assert_eq!(loaded.light_dir, [0.5, 0.8, 0.6]);
        assert_eq!(loaded.ambient, 0.35);
    }

    #[test]
    fn v2_keys_lights_and_character_node() {
        let dir = std::env::temp_dir();
        let f = dir.join("talerender-test-v2full.scene.json");
        std::fs::write(
            &f,
            r##"{"format": "talerender-scene", "version": 2, "name": "v2proof",
                "time": 1.0,
                "camera": {"fovY": 40.0, "eye": [0, 2, 6], "target": [0, 1, 0],
                           "keys": [{"t": 0.0, "eye": [0, 2, 8], "target": [0, 1, 0], "fovY": 44.0},
                                    {"t": 2.0, "eye": [0, 2, 4], "target": [0, 1, 0], "fovY": 36.0}]},
                "lights": {"key": {"color": "#FFF0D6", "intensity": 2.6, "position": [7, 12, 5], "target": [0, 0, -2], "shadow": true, "shadowBox": 17.0},
                           "points": [{"color": "#FF9A3C", "intensity": 5.0, "position": [-1.35, 0.6, -2.45], "distance": 7.0, "decay": 1.6}]},
                "nodes": [{"type": "character", "position": [0.35, 0, -0.55],
                           "look": {"body": "#5C6644", "hat": "cap", "hatCol": "#4C5638"},
                           "state": {"mood": "determined", "armL": [10, 6], "armR": [16, -6], "lean": 1.0}}]}"##,
        )
        .unwrap();
        let loaded = load(f.to_str().unwrap()).unwrap();
        assert_eq!(loaded.version, 2);
        assert_eq!(loaded.time, 1.0);
        // t = 1.0 is the smootherstep midpoint of the key segment:
        // eye z = 8 + (4 - 8) * 0.5 = 6, fovY = 44 + (36 - 44) * 0.5 = 40.
        assert!((loaded.camera.eye[2] - 6.0).abs() < 1e-6, "{:?}", loaded.camera.eye);
        assert!((loaded.camera.fov_y_deg - 40.0).abs() < 1e-6);
        let lights = loaded.lights.expect("v2 lights parsed");
        assert_eq!(lights.key.as_ref().unwrap().intensity, 2.6);
        assert_eq!(lights.points.len(), 1);
        // The character node emits blob + card textured draws and
        // appends their two textures (card, blob).
        assert_eq!(loaded.draws.len(), 2);
        assert_eq!(loaded.textures.len(), 2);
    }

    #[test]
    fn v1_file_with_v2_fields_is_rejected() {
        let dir = std::env::temp_dir();
        let f = dir.join("talerender-test-v1keys.scene.json");
        std::fs::write(
            &f,
            r##"{"format": "talerender-scene", "version": 1, "name": "bad",
                "camera": {"fovY": 30.0, "eye": [0, 0, 0], "target": [0, 0, -1],
                           "keys": [{"t": 0.0, "eye": [0, 0, 0], "target": [0, 0, -1]}]},
                "nodes": []}"##,
        )
        .unwrap();
        let e = load(f.to_str().unwrap()).err().expect("v1 + keys must be rejected");
        assert!(e.contains("v2"), "{e}");
    }
}
