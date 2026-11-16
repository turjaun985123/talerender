//! GPUFIRST Phase 0 — PERF seat timing harness (`--gpupaint-time`).
//!
//! Measures, per character state, the two costs the GPU-first
//! decision turns on:
//!   (a) CPU paint: `toonfull::paint_toon_full` wall time
//!       (median of 9, 1 warmup, `std::time::Instant`).
//!   (b) GPU replay PROXY on whatever adapter wgpu selects
//!       (CI: lavapipe, a SOFTWARE renderer): upload of the painted
//!       2560x2816 canvas + a full-canvas textured-quad render,
//!       submit->complete wall time (median of 9), plus wgpu
//!       timestamp-query GPU-side ms when the adapter exposes
//!       `TIMESTAMP_QUERY`.
//!
//! HONESTY LAW (read before quoting any number from this file):
//! at HEAD bc12a5c there is NO GPU painter and NO display list in
//! this tree — the GPUX/TRACE seats' files had not landed when
//! this harness was written. Part (b) is therefore NOT an SDF /
//! display-list replay: it measures the texture path the film
//! renderer already uses for character cards (upload + sampled
//! quad), i.e. the floor of any GPU-first design that keeps
//! painted canvases, and the pipeline shape (submit/complete,
//! timestamp plumbing) a true replay will reuse. It does NOT
//! measure, and must never be quoted as, the cost of evaluating
//! the toon's primitives on the GPU. On lavapipe it also does
//! NOT predict RADV timings: lavapipe rasterises on the CPU.
//! The harness is adapter-agnostic and runs unchanged on the
//! user's PC, where the same lines will carry RADV numbers.
//!
//! Output grammar (grep prefix `GPUFIRST-TIME`): one line per
//! item plus `SUMMARY` lines; all times in milliseconds, medians
//! over n=9 timed repetitions after 1 warmup unless stated.

use crate::scene::{paint_toon, parse_json, rgba_of, Painter, ToonMood, ToonSpec, Value};
use crate::toonfull::{paint_toon_full, FullLook, FullState, Legs, Mood};
use std::time::Instant;

const REPS: usize = 9;
const CANVAS_W: u32 = 2560;
const CANVAS_H: u32 = 2816;

// ---------------------------------------------------------------------
// Small statistics helpers (sorted-copy; nearest-rank percentiles).
// ---------------------------------------------------------------------

fn sorted(v: &[f64]) -> Vec<f64> {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    s
}
fn median(v: &[f64]) -> f64 {
    let s = sorted(v);
    s[s.len() / 2]
}
fn p95(v: &[f64]) -> f64 {
    let s = sorted(v);
    let k = ((s.len() as f64 * 0.95).ceil() as usize).max(1).min(s.len());
    s[k - 1]
}
fn stats_line(label: &str, v: &[f64]) -> String {
    let s = sorted(v);
    format!(
        "GPUFIRST-TIME SUMMARY {label} n={} min={:.3} median={:.3} p95={:.3} max={:.3} mean={:.3}",
        s.len(),
        s[0],
        s[s.len() / 2],
        p95(v),
        s[s.len() - 1],
        s.iter().sum::<f64>() / s.len() as f64
    )
}

/// Time `f` once for warmup, then REPS times; return (median_ms, all_ms).
fn time_median<F: FnMut() -> R, R>(mut f: F) -> (f64, Vec<f64>) {
    let _ = f();
    let mut ms = Vec::with_capacity(REPS);
    for _ in 0..REPS {
        let t = Instant::now();
        let _ = f();
        ms.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    (median(&ms), ms)
}

fn coverage(px: &[u8]) -> u64 {
    // Non-transparent pixel count: the amount of canvas the state
    // actually paints over — the work proxy quoted next to each
    // timing (a full-canvas state and a sparse state are not the
    // same job, and the table must say which is which).
    px.iter().skip(3).step_by(4).filter(|&&a| a > 0).count() as u64
}

// ---------------------------------------------------------------------
// The STATES spread: registry looks x representative + extreme states.
// Poses replicate film.rs's pose_* helpers verbatim (they are private
// to film.rs); moods/legs/patterns/hats sweep the painter's branches.
// ---------------------------------------------------------------------

fn spread() -> Vec<(String, FullLook, FullState)> {
    // FullLook has no Clone derive (toonfull.rs): every look is
    // built fresh — look_registry() constructs a new registry per
    // call, so look(i) moves entry i out of a fresh registry.
    let n_looks = crate::film::look_registry().len();
    let look = |i: usize| crate::film::look_registry().into_iter().nth(i).expect("registry look").0;
    let mut out: Vec<(String, FullLook, FullState)> = Vec::new();
    let mut add = |name: String, lk: FullLook, st: FullState| out.push((name, lk, st));

    // Every registry look at the rest state.
    for i in 0..n_looks {
        add(format!("registry{i:02}-rest"), look(i), FullState::default());
    }
    // All 19 moods on the hero look.
    let moods = [
        Mood::Neutral, Mood::Smug, Mood::Happy, Mood::Laugh, Mood::Shock,
        Mood::Worried, Mood::Angry, Mood::Sad, Mood::Sly, Mood::Proud,
        Mood::Tired, Mood::Dizzy, Mood::Determined, Mood::Embarrassed,
        Mood::Awe, Mood::Suspicious, Mood::Relieved, Mood::Puzzled, Mood::Deadpan,
    ];
    for m in moods {
        let mut st = FullState::default();
        st.mood = m;
        add(format!("mood-{m:?}"), look(0), st);
    }
    // Legs: walk/run phase sweep, sit drops, jump.
    for (nm, ph) in [("walk-p0", 0.0f32), ("walk-p1", 1.57), ("walk-p2", 3.14), ("walk-p3", 4.71)] {
        let mut st = FullState::default();
        st.legs = Legs::Walk;
        st.phase = ph;
        add(nm.into(), look(0), st);
    }
    for (nm, ph) in [("run-p0", 0.0f32), ("run-p2", 3.14)] {
        let mut st = FullState::default();
        st.legs = Legs::Run;
        st.phase = ph;
        add(nm.into(), look(0), st);
    }
    for (nm, drop) in [("sit-default", None), ("sit-shallow", Some(80.0f32)), ("sit-deep", Some(320.0))] {
        let mut st = FullState::default();
        st.legs = Legs::Sit;
        st.sit_drop = drop;
        add(nm.into(), look(0), st);
    }
    {
        let mut st = FullState::default();
        st.legs = Legs::Jump;
        add("jump".into(), look(0), st);
    }
    // The film's named poses (film.rs pose_* values, copied).
    {
        let mut st = FullState::default();
        st.mood = Mood::Happy; st.arm_r = (68.0, -42.0); st.arm_l = (28.0, -24.0); st.look = [0.3, 0.0];
        add("pose-explain".into(), look(0), st);
        let mut st = FullState::default();
        st.mood = Mood::Laugh; st.arm_l = (150.0, 14.0); st.arm_r = (150.0, 14.0); st.bob = -8.0;
        add("pose-celebrate".into(), look(0), st);
        let mut st = FullState::default();
        st.mood = Mood::Sad; st.arm_l = (4.0, 4.0); st.arm_r = (4.0, 4.0); st.lean = 5.0; st.head_tilt = -4.0; st.bob = 4.0;
        add("pose-mourn".into(), look(0), st);
        let mut st = FullState::default();
        st.arm_r = (118.0, -105.0); st.arm_l = (10.0, 8.0); st.lean = 3.0; st.head_tilt = -5.0; st.head_turn = -0.2;
        add("pose-listen".into(), look(0), st);
        let mut st = FullState::default();
        st.arm_r = (48.0, -118.0); st.arm_l = (20.0, -30.0); st.head_tilt = 6.0; st.head_turn = 0.3; st.look = [0.4, -0.3];
        add("pose-think".into(), look(0), st);
    }
    // Channel extremes: blink/talk/sweat/tears/gaze/squash/tilt/turn/pitch.
    {
        let mut st = FullState::default(); st.blink = 1.0;
        add("x-blink-full".into(), look(0), st);
        let mut st = FullState::default(); st.talk = 1.0;
        add("x-talk-full".into(), look(0), st);
        let mut st = FullState::default(); st.sweat = true;
        add("x-sweat".into(), look(0), st);
        let mut st = FullState::default(); st.tears = true;
        add("x-tears".into(), look(0), st);
        let mut st = FullState::default(); st.look = [1.0, 1.0];
        add("x-gaze-corner".into(), look(0), st);
        let mut st = FullState::default(); st.squash = 0.3;
        add("x-squash-pos".into(), look(0), st);
        let mut st = FullState::default(); st.squash = -0.2;
        add("x-squash-neg".into(), look(0), st);
        let mut st = FullState::default(); st.head_tilt = 20.0; st.head_turn = 1.0;
        add("x-head-max".into(), look(0), st);
        let mut st = FullState::default(); st.cam_pitch = 1.0;
        add("x-campitch-pos".into(), look(0), st);
        let mut st = FullState::default(); st.cam_pitch = -1.0;
        add("x-campitch-neg".into(), look(0), st);
        // The B-1 re-anchor case: lying lean (envelope leaves canvas).
        let mut st = FullState::default(); st.lean = 80.0;
        add("x-lean-lying".into(), look(0), st);
        let mut st = FullState::default(); st.lean = -15.0;
        add("x-lean-back".into(), look(0), st);
    }
    // Look-branch sweep on the hero body: patterns, hats, neckwear,
    // extras, kid proportions (painter branches, not registry looks).
    {
        for pat in ["vest", "stripes", "apron"] {
            let mut lk = look(0);
            lk.pattern = pat.to_string();
            lk.pat_col = Some("#3B3346".to_string());
            add(format!("pattern-{pat}"), lk, FullState::default());
        }
        for hat in ["none", "cap", "bowler", "boater", "beanie", "visor", "tophat", "bun", "flower", "bow", "mailcap"] {
            let mut lk = look(0);
            lk.hat = hat.to_string();
            lk.hat_col = Some("#4C5638".to_string());
            add(format!("hat-{hat}"), lk, FullState::default());
        }
        for neck in ["tie", "scarf", "collar"] {
            let mut lk = look(0);
            lk.neck = neck.to_string();
            lk.neck_col = Some("#D8343F".to_string());
            add(format!("neck-{neck}"), lk, FullState::default());
        }
        let mut lk = look(0);
        lk.glasses = true; lk.mustache = true; lk.mustache_col = Some("#C9C9D2".to_string());
        lk.blush = true; lk.freckles = true; lk.pencil = true; lk.monocle = true;
        add("extras-all".into(), lk, FullState::default());
        let mut lk = look(0);
        lk.kid = true;
        add("kid-rest".into(), lk, FullState::default());
        let mut lk = look(0);
        lk.kid = true;
        let mut st = FullState::default();
        st.legs = Legs::Sit;
        add("kid-sit".into(), lk, st);
    }
    out
}

// ---------------------------------------------------------------------
// Battery cards (the M4 spec, docs/ab/stroke-battery/spec.json — the
// same embedded copy battery.rs paints; prims loop mirrors battery.rs).
// ---------------------------------------------------------------------

const SPEC: &str = include_str!("../docs/ab/stroke-battery/spec.json");

fn bnum(v: &Value) -> f32 { v.as_f64().expect("spec: expected number") as f32 }
fn bpt(v: &Value) -> (f32, f32) {
    let a = v.as_arr().expect("spec: expected point");
    (bnum(&a[0]), bnum(&a[1]))
}
fn bpts(v: &Value) -> Vec<(f32, f32)> { v.as_arr().expect("spec: expected points").iter().map(bpt).collect() }
fn bget<'a>(o: &'a Value, k: &str) -> &'a Value { o.get(k).unwrap_or_else(|| panic!("spec: missing {k}")) }

fn paint_prims_card(spec: &Value) -> (Vec<u8>, u32, u32) {
    let canvas = spec.get("canvas").expect("spec.canvas");
    let res = bnum(canvas.get("res").expect("spec.canvas.res"));
    let (lw, lh) = (bnum(canvas.get("w").expect("spec.canvas.w")), bnum(canvas.get("h").expect("spec.canvas.h")));
    let (dw, dh) = ((lw * res) as usize, (lh * res) as usize);
    let mut p = Painter::new(dw, dh);
    let ink = rgba_of(spec.get("ink").and_then(|v| v.as_str()).expect("spec.ink"));
    let sc = |v: f32| v * res;
    let scp = |q: (f32, f32)| (q.0 * res, q.1 * res);
    for op in spec.get("prims").expect("spec.prims").as_arr().expect("spec.prims array") {
        let kind = op.get("op").and_then(|v| v.as_str()).expect("spec op kind");
        let col = op.get("col").and_then(|v| v.as_str()).map(rgba_of).unwrap_or(ink);
        match kind {
            "line" => { let q = bpts(op.get("p").expect("line.p")); p.stroke_polyline(&[scp(q[0]), scp(q[1])], sc(bnum(op.get("w").expect("line.w"))), col); }
            "polyline" => { let q: Vec<(f32, f32)> = bpts(op.get("p").expect("polyline.p")).iter().map(|&q| scp(q)).collect(); p.stroke_polyline(&q, sc(bnum(op.get("w").expect("w"))), col); }
            "quad" => { let q = bpts(op.get("p").expect("quad.p")); p.stroke_quad(scp(q[0]), scp(q[1]), scp(q[2]), sc(bnum(op.get("w").expect("quad.w"))), col); }
            "stroke_ellipse" => { let c = bpt(op.get("c").expect("c")); p.stroke_ellipse(sc(c.0), sc(c.1), sc(bnum(op.get("rx").expect("rx"))), sc(bnum(op.get("ry").expect("ry"))), 0.0, sc(bnum(op.get("w").expect("w"))), col); }
            "fill_ellipse" => { let c = bpt(op.get("c").expect("c")); p.fill_ellipse(sc(c.0), sc(c.1), sc(bnum(op.get("rx").expect("rx"))), sc(bnum(op.get("ry").expect("ry"))), 0.0, col); }
            "arc" => {
                let c = bpt(op.get("c").expect("arc.c"));
                let r = bnum(op.get("r").expect("arc.r"));
                let (a0, a1) = (bnum(op.get("a0").expect("a0")), bnum(op.get("a1").expect("a1")));
                let q: Vec<(f32, f32)> = (0..=32).map(|i| { let a = a0 + (i as f32 / 32.0) * (a1 - a0); scp((c.0 + r * a.cos(), c.1 + r * a.sin())) }).collect();
                p.stroke_polyline(&q, sc(bnum(op.get("w").expect("arc.w"))), col);
            }
            other => panic!("spec: unknown prim op {other}"),
        }
    }
    (p.px, dw as u32, dh as u32)
}

fn battery_toon_spec(lookv: &Value, state: &Value, mood: &str) -> ToonSpec {
    let pair = |o: &Value, k: &str| { let a = bget(o, k).as_arr().expect("spec: expected pair"); (bnum(&a[0]), bnum(&a[1])) };
    let leak = |v: &Value| -> &'static str { Box::leak(v.as_str().expect("spec: expected string").to_string().into_boxed_str()) };
    let hat = bget(lookv, "hat").as_str().expect("spec: hat");
    ToonSpec {
        body: leak(bget(lookv, "body")),
        head: leak(bget(lookv, "head")),
        shade: lookv.get("shade").map(leak),
        cap: if hat == "cap" { Some(leak(bget(lookv, "hatCol"))) } else { None },
        mood: match mood {
            "neutral" => ToonMood::Neutral,
            "determined" => ToonMood::Determined,
            "angry" => ToonMood::Angry,
            "laugh" => ToonMood::Laugh,
            "shock" => ToonMood::Shock,
            other => panic!("spec: unknown mood {other}"),
        },
        arm_l: pair(state, "armL"),
        arm_r: pair(state, "armR"),
        lean_deg: bnum(bget(state, "lean")),
        head_turn: bnum(bget(state, "headTurn")),
        cam_pitch: bnum(bget(state, "camPitch")),
    }
}

// ---------------------------------------------------------------------
// Per-prim-class synthetic costs: one Painter at production canvas
// size, each primitive class looped 200x (warmup 20) — micro-costs
// that, multiplied by a future display list's composition, split a
// state's paint cost by class. Labelled SYNTHETIC: looped overpaint
// on one canvas, not a traced paint.
// ---------------------------------------------------------------------

fn prim_class_costs() {
    let (w, h) = (CANVAS_W as usize, CANVAS_H as usize);
    let ink = (31u8, 28u8, 51u8, 255u8);
    let bean: Vec<(f32, f32)> = (0..=112)
        .map(|i| {
            let a = i as f32 / 112.0 * std::f32::consts::TAU;
            (1280.0 + 360.0 * a.cos(), 1408.0 + 620.0 * a.sin())
        })
        .collect();
    let _ = &bean;
    macro_rules! bench_prim {
        ($name:expr, $body:expr) => {{
            let mut p = Painter::new(w, h);
            let f: &dyn Fn(&mut Painter) = &$body;
            for _ in 0..20 { f(&mut p); }
            let t = Instant::now();
            for _ in 0..200 { f(&mut p); }
            let per = t.elapsed().as_secs_f64() * 1e6 / 200.0; // microseconds
            println!("GPUFIRST-TIME PRIM class={} us_per_prim={:.2} reps=200 warmup=20 synthetic=1", $name, per);
        }};
    }
    let bean2: Vec<(f32, f32)> = (0..=112).map(|i| { let a = i as f32 / 112.0 * std::f32::consts::TAU; (1280.0 + 360.0 * a.cos(), 1408.0 + 620.0 * a.sin()) }).collect();
    let bean3 = bean2.clone();
    let bean4 = bean2.clone();
    let bean5 = bean2.clone();
    bench_prim!("stroke_polyline-long113", |p: &mut Painter| { let mut r = bean2.clone(); r.push(bean2[0]); p.stroke_polyline(&r, 18.0, ink); });
    bench_prim!("stroke_polyline-limb3", |p: &mut Painter| p.stroke_polyline(&[(900.0, 1500.0), (860.0, 2000.0), (920.0, 2500.0)], 30.0, ink));
    bench_prim!("fill_polygon-bean113", |p: &mut Painter| p.fill_polygon(&bean3, ink));
    bench_prim!("polygon_mask-bean113", |p: &mut Painter| { let m = p.polygon_mask(&bean4); std::hint::black_box(m.len()); });
    bench_prim!("fill_ellipse_masked-head", |p: &mut Painter| { let m = p.polygon_mask(&bean5); p.fill_ellipse_masked(1400.0, 1400.0, 140.0, 520.0, 0.0, ink, &m); });
    bench_prim!("fill_ellipse-head", |p: &mut Painter| p.fill_ellipse(1280.0, 700.0, 276.0, 276.0, 0.0, ink));
    bench_prim!("fill_ellipse-small", |p: &mut Painter| p.fill_ellipse(1200.0, 650.0, 24.0, 36.0, 0.0, ink));
    bench_prim!("stroke_ellipse-head", |p: &mut Painter| p.stroke_ellipse(1280.0, 700.0, 276.0, 276.0, 0.0, 14.0, ink));
    bench_prim!("fill_rect-fullcanvas", |p: &mut Painter| p.fill_rect(0.0, 0.0, 2560.0, 2816.0, ink));
}

// ---------------------------------------------------------------------
// GPU replay proxy (see the honesty law in the module header).
// ---------------------------------------------------------------------

const REPLAY_WGSL: &str = "
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
struct Vout { @builtin(position) clip: vec4f, @location(0) uv: vec2f };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> Vout {
    var o: Vout;
    let x = f32(i32(vi & 1u)) * 4.0 - 1.0;
    let y = f32(i32((vi >> 1u) & 1u)) * 4.0 - 1.0;
    o.clip = vec4f(x, y, 0.0, 1.0);
    o.uv = vec2f((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
    return o;
}
@fragment fn fs_main(in: Vout) -> @location(0) vec4f {
    return textureSample(tex, samp, in.uv);
}
";

async fn gpu_replay(
    items: &[(String, FullLook, FullState)],
) -> crate::errors::TaleResult<()> {
    let instance = crate::gpucore::new_instance();
    let adapter = crate::gpucore::request_adapter(&instance).await?;
    let info = adapter.get_info();
    println!(
        "{}",
        crate::adapter::report_line(&crate::adapter::describe(&info))
    );
    let want_ts = adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);
    let feats = if want_ts { wgpu::Features::TIMESTAMP_QUERY } else { wgpu::Features::empty() };
    println!("GPUFIRST-TIME GPU timestamp_query_available={}", want_ts);
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("gpupaint-time"),
            required_features: feats,
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        })
        .await
        .map_err(|e| crate::errors::TaleError::device_request("request_device (gpupaint-time)", e))?;

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gpupaint-replay"),
        source: wgpu::ShaderSource::Wgsl(REPLAY_WGSL.into()),
    });
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("gpupaint-replay-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        cache: None,
        label: Some("gpupaint-replay-pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpupaint-replay-layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        })),
        vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("gpupaint-replay-sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gpupaint-replay-target"),
        size: wgpu::Extent3d { width: CANVAS_W, height: CANVAS_H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let src_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gpupaint-replay-src"),
        size: wgpu::Extent3d { width: CANVAS_W, height: CANVAS_H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let src_view = src_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("gpupaint-replay-bg"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&src_view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
        ],
    });
    let query_set = if want_ts {
        Some(device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("gpupaint-replay-ts"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        }))
    } else { None };
    // wgpu validation: MAP usage cannot combine with QUERY_RESOLVE
    // at all — resolve into query_buf, then copy to a mappable
    // staging buffer (the standard two-buffer timestamp readback).
    let query_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpupaint-replay-ts-buf"),
        size: 16,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let query_stage = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpupaint-replay-ts-stage"),
        size: 16,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let period_ns = queue.get_timestamp_period() as f64;
    let wait = || {
        device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
    };

    // F_gpu (INTEG's named requirement): the per-paint GPU FIXED
    // cost — dispatch + full-canvas raster with NO prim work.
    // Proxy form for this harness: (i) an empty submit (no pass)
    // prices dispatch alone; (ii) a replay whose source texture
    // is all-transparent (the "empty display list" analogue)
    // prices dispatch + full-canvas quad raster + sampling.
    // F_gpu := (ii); the marginal cost of real prim content is
    // then (per-state replay) - F_gpu. Lavapipe-labelled: on
    // lavapipe the raster is CPU work, so this BOUNDS the shape,
    // it does not predict the RADV fixed cost.
    {
        let zeros = vec![0u8; (CANVAS_W * CANVAS_H * 4) as usize];
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &src_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &zeros,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(CANVAS_W * 4), rows_per_image: Some(CANVAS_H) },
            wgpu::Extent3d { width: CANVAS_W, height: CANVAS_H, depth_or_array_layers: 1 },
        );
        wait();
        let mut empty_submit = Vec::with_capacity(REPS);
        for _ in 0..REPS {
            let t = Instant::now();
            let enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("gpupaint-fgpu-empty") });
            queue.submit(Some(enc.finish()));
            wait();
            empty_submit.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        let mut empty_replay = Vec::with_capacity(REPS);
        for _ in 0..REPS {
            let t = Instant::now();
            let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("gpupaint-fgpu-replay") });
            {
                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("gpupaint-fgpu-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
            queue.submit(Some(enc.finish()));
            wait();
            empty_replay.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        println!(
            "GPUFIRST-TIME F-GPU empty_submit_median_ms={:.3} empty_replay_median_ms={:.3} empty_replay_p95_ms={:.3} n={} definition=dispatch+full-canvas-raster-no-prims proxy=texture-quad",
            median(&empty_submit), median(&empty_replay), p95(&empty_replay), REPS
        );
    }

    let mut wall_all: Vec<f64> = Vec::new();
    let mut ts_all: Vec<f64> = Vec::new();
    let mut up_all: Vec<f64> = Vec::new();
    for (name, lk, st) in items {
        // One untimed paint supplies the pixels this state's replay
        // uploads (the CPU cost of the same paint is the CPU table).
        let (px, _, _, _, _) = paint_toon_full(lk, st);
        let upload = |queue: &wgpu::Queue| {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &src_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                &px,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(CANVAS_W * 4), rows_per_image: Some(CANVAS_H) },
                wgpu::Extent3d { width: CANVAS_W, height: CANVAS_H, depth_or_array_layers: 1 },
            );
        };
        // Upload-inclusive cost: write_texture + drain.
        let mut ups = Vec::with_capacity(3);
        for _ in 0..3 {
            let t = Instant::now();
            upload(&queue);
            wait();
            ups.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        let up_med = median(&ups);
        up_all.push(up_med);
        // Resident replay: submit -> complete, texture already up.
        let mut run_once = |with_ts: bool| -> Option<f64> {
            let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("gpupaint-replay-enc") });
            let tw = if with_ts {
                query_set.as_ref().map(|qs| wgpu::RenderPassTimestampWrites {
                    query_set: qs,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                })
            } else { None };
            {
                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("gpupaint-replay-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: tw,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
            if with_ts {
                if let Some(qs) = &query_set {
                    enc.resolve_query_set(qs, 0..2, &query_buf, 0);
                }
                enc.copy_buffer_to_buffer(&query_buf, 0, &query_stage, 0, 16);
            }
            queue.submit(Some(enc.finish()));
            wait();
            if with_ts {
                let slice = query_stage.slice(..);
                let (tx, rx) = std::sync::mpsc::channel();
                slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
                wait();
                if rx.recv().map(|r| r.is_ok()).unwrap_or(false) {
                    if let Ok(data) = slice.get_mapped_range() {
                        let t0 = u64::from_le_bytes(data[0..8].try_into().unwrap());
                        let t1 = u64::from_le_bytes(data[8..16].try_into().unwrap());
                        drop(data);
                        query_stage.unmap();
                        return Some(t1.wrapping_sub(t0) as f64 * period_ns / 1.0e6);
                    }
                }
                query_stage.unmap();
                return None;
            }
            None
        };
        let _ = run_once(want_ts); // warmup
        let mut walls = Vec::with_capacity(REPS);
        let mut tss = Vec::new();
        for _ in 0..REPS {
            let t = Instant::now();
            let ts = run_once(want_ts);
            walls.push(t.elapsed().as_secs_f64() * 1000.0);
            if let Some(v) = ts { tss.push(v); }
        }
        let wmed = median(&walls);
        wall_all.push(wmed);
        let ts_str = if tss.is_empty() {
            "na".to_string()
        } else {
            let m = median(&tss);
            ts_all.push(m);
            format!("{m:.3}")
        };
        println!(
            "GPUFIRST-TIME GPU-REPLAY-PROXY state={} wall_median_ms={:.3} wall_p95_ms={:.3} gpu_timestamp_median_ms={} upload_median_ms={:.3} n={} proxy=texture-quad-not-displaylist",
            name, wmed, p95(&walls), ts_str, up_med, REPS
        );
    }
    println!("{}", stats_line("gpu-replay-wall-ms", &wall_all));
    println!("{}", stats_line("gpu-upload-ms", &up_all));
    if !ts_all.is_empty() {
        println!("{}", stats_line("gpu-timestamp-ms", &ts_all));
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Entry point.
// ---------------------------------------------------------------------

pub(crate) fn run(with_gpu: bool) -> i32 {
    run_mode(with_gpu, false)
}

pub(crate) fn run_mode(with_gpu: bool, gpu_only: bool) -> i32 {
    println!("GPUFIRST-TIME BEGIN head=bc12a5c reps={} warmup=1 canvas={}x{} gpu_only={}", REPS, CANVAS_W, CANVAS_H, gpu_only);
    println!("GPUFIRST-TIME NOTE gpu column is a texture-replay PROXY (no display list exists at this head); lavapipe numbers are software rendering and do not predict RADV");
    if gpu_only {
        let items = spread();
        match pollster::block_on(gpu_replay(&items)) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("GPUFIRST-TIME GPU-UNAVAILABLE {e}");
                println!("GPUFIRST-TIME GPU-REPLAY-PROXY unavailable=1");
            }
        }
        println!("GPUFIRST-TIME END");
        return 0;
    }

    // ---- CPU: states spread ----
    let items = spread();
    let mut cpu_all: Vec<f64> = Vec::new();
    let mut rows: Vec<(String, f64, u64)> = Vec::new();
    for (name, lk, st) in &items {
        let (med, _) = time_median(|| paint_toon_full(lk, st));
        let (px, _, _, _, _) = paint_toon_full(lk, st);
        let cov = coverage(&px);
        cpu_all.push(med);
        rows.push((name.clone(), med, cov));
        println!("GPUFIRST-TIME CPU state={} median_ms={:.3} coverage_px={} n={}", name, med, cov, REPS);
    }
    // Display-list composition per state (feature gpupaint only,
    // via the TRACE seat's gputrace recorder): one traced paint
    // per state, prims counted by class. This is the composition
    // a per-class cost split multiplies against.
    #[cfg(feature = "gpupaint")]
    {
        use crate::gputrace::Prim;
        let class_of = |p: &Prim| -> &'static str {
            match p {
                Prim::FillPolygon { .. } => "fill_polygon",
                Prim::FillPolygonMasked { .. } => "fill_polygon_masked",
                Prim::FillEllipse { .. } => "fill_ellipse",
                Prim::FillEllipseMasked { .. } => "fill_ellipse_masked",
                Prim::StrokeEllipse { .. } => "stroke_ellipse",
                Prim::StrokePolyline { .. } => "stroke_polyline",
                Prim::StrokeQuad { .. } => "stroke_quad",
                Prim::FillRect { .. } => "fill_rect",
                Prim::PolygonMask { .. } => "polygon_mask",
                Prim::EllipseMask { .. } => "ellipse_mask",
                Prim::CompositePlate { .. } => "composite_plate",
                Prim::HeadShade { .. } => "head_shade",
                Prim::MaskedBlend { .. } => "masked_blend",
            }
        };
        crate::gputrace::set_armed(true);
        let mut totals: std::collections::BTreeMap<&'static str, u64> = std::collections::BTreeMap::new();
        let mut total_prims: u64 = 0;
        for (name, lk, st) in &items {
            crate::gputrace::begin(CANVAS_W, CANVAS_H);
            let _ = paint_toon_full(lk, st);
            let list = crate::gputrace::take();
            let mut counts: std::collections::BTreeMap<&'static str, u64> = std::collections::BTreeMap::new();
            for p in &list.prims {
                *counts.entry(class_of(p)).or_default() += 1;
                *totals.entry(class_of(p)).or_default() += 1;
            }
            total_prims += list.prims.len() as u64;
            let parts: Vec<String> = counts.iter().map(|(k, v)| format!("{k}={v}")).collect();
            println!("GPUFIRST-TIME COMPOSITION state={} prims={} {}", name, list.prims.len(), parts.join(" "));
        }
        crate::gputrace::set_armed(false);
        let parts: Vec<String> = totals.iter().map(|(k, v)| format!("{k}={v}")).collect();
        println!("GPUFIRST-TIME COMPOSITION-TOTAL states={} prims={} {}", items.len(), total_prims, parts.join(" "));
    }
    println!("{}", stats_line("cpu-state-ms", &cpu_all));
    let mut by_cost = rows.clone();
    by_cost.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for (name, med, cov) in by_cost.iter().take(10) {
        println!("GPUFIRST-TIME CPU-HEAVIEST state={} median_ms={:.3} coverage_px={}", name, med, cov);
    }

    // ---- CPU: battery cards ----
    let spec = parse_json(SPEC).expect("embedded battery spec must parse");
    let (med, _) = time_median(|| paint_prims_card(&spec));
    println!("GPUFIRST-TIME CPU battery=prims median_ms={:.3} n={}", med, REPS);
    let toon = spec.get("toon").expect("spec.toon");
    let ts = battery_toon_spec(
        toon.get("look").expect("toon.look"),
        toon.get("state").expect("toon.state"),
        toon.get("state").unwrap().get("mood").and_then(|v| v.as_str()).expect("toon.state.mood"),
    );
    let (med, _) = time_median(|| paint_toon(&ts));
    println!("GPUFIRST-TIME CPU battery=toon median_ms={:.3} n={}", med, REPS);
    let faces = spec.get("faces").expect("spec.faces");
    for m in faces.get("moods").expect("faces.moods").as_arr().expect("faces.moods array") {
        let m = m.as_str().expect("mood name");
        let ts = battery_toon_spec(faces.get("look").expect("faces.look"), faces.get("state").expect("faces.state"), m);
        let (med, _) = time_median(|| paint_toon(&ts));
        println!("GPUFIRST-TIME CPU battery=face-{} median_ms={:.3} n={}", m, med, REPS);
    }

    // ---- CPU: per-prim-class synthetics ----
    prim_class_costs();

    // ---- GPU replay proxy ----
    if with_gpu {
        match pollster::block_on(gpu_replay(&items)) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("GPUFIRST-TIME GPU-UNAVAILABLE {e}");
                println!("GPUFIRST-TIME GPU-REPLAY-PROXY unavailable=1 (CPU table above stands alone)");
            }
        }
    } else {
        println!("GPUFIRST-TIME GPU-REPLAY-PROXY skipped=1 (--gpupaint-no-gpu)");
    }
    println!("GPUFIRST-TIME END");
    0
}
