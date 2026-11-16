//! --scene stroke-battery: the M4 stroke battery cards, painted with
//! the CPU Painter (src/scene.rs) from the SAME spec.json the Chrome
//! reference harness consumes (docs/ab/stroke-battery/spec.json,
//! embedded here at compile time so the two sides can never drift
//! apart). Pure CPU — no wgpu, no GPU claim of any kind; the cards
//! are the painter's raw output, which is exactly the surface
//! compatibility contract §2 binds.
//!
//! Cards written to the --out directory:
//!   prims.png          the primitive battery (segments, arcs, joins)
//!   toon.png           the full character (spec's toon look/state)
//!   face-<mood>.png    one full figure per face-set mood (the head
//!                      region is the measured surface)

use crate::scene::{paint_toon, parse_json, rgba_of, Painter, ToonMood, ToonSpec, Value};

const SPEC: &str = include_str!("../docs/ab/stroke-battery/spec.json");

fn num(v: &Value) -> f32 {
    v.as_f64().expect("spec: expected number") as f32
}

fn pt(v: &Value) -> (f32, f32) {
    let a = v.as_arr().expect("spec: expected point");
    (num(&a[0]), num(&a[1]))
}

fn pts(v: &Value) -> Vec<(f32, f32)> {
    v.as_arr().expect("spec: expected points").iter().map(pt).collect()
}

fn save(px: &[u8], w: u32, h: u32, path: &str) -> crate::errors::TaleResult<()> {
    let img = image::RgbaImage::from_raw(w, h, px.to_vec()).expect("pixel buffer size mismatch");
    img.save(path).map_err(|e| crate::errors::TaleError::image_io(path, e))?;
    println!("wrote {path} ({w}x{h})");
    Ok(())
}

fn mood_of(s: &str) -> ToonMood {
    match s {
        "neutral" => ToonMood::Neutral,
        "determined" => ToonMood::Determined,
        "angry" => ToonMood::Angry,
        "laugh" => ToonMood::Laugh,
        "shock" => ToonMood::Shock,
        other => panic!("spec: unknown mood {other}"),
    }
}

fn get<'a>(o: &'a Value, k: &str) -> &'a Value {
    o.get(k).unwrap_or_else(|| panic!("spec: missing {k}"))
}

fn toon_spec(look: &Value, state: &Value, mood: &str) -> ToonSpec {
    let pair = |o: &Value, k: &str| {
        let a = get(o, k).as_arr().expect("spec: expected pair");
        (num(&a[0]), num(&a[1]))
    };
    // &'static str for ToonSpec: the spec strings are short and few;
    // leak them deliberately (process-lifetime, one-shot CLI).
    let leak = |v: &Value| -> &'static str {
        Box::leak(v.as_str().expect("spec: expected string").to_string().into_boxed_str())
    };
    let hat = get(look, "hat").as_str().expect("spec: hat");
    ToonSpec {
        body: leak(get(look, "body")),
        head: leak(get(look, "head")),
        shade: look.get("shade").map(leak),
        cap: if hat == "cap" { Some(leak(get(look, "hatCol"))) } else { None },
        mood: mood_of(mood),
        arm_l: pair(state, "armL"),
        arm_r: pair(state, "armR"),
        lean_deg: num(get(state, "lean")),
        head_turn: num(get(state, "headTurn")),
        cam_pitch: num(get(state, "camPitch")),
    }
}

pub(crate) fn run_battery(out_dir: &str) -> crate::errors::TaleResult<()> {
    let spec = parse_json(SPEC).expect("embedded battery spec must parse");
    let canvas = spec.get("canvas").expect("spec.canvas");
    let res = num(canvas.get("res").expect("spec.canvas.res"));
    let (lw, lh) = (num(canvas.get("w").expect("spec.canvas.w")), num(canvas.get("h").expect("spec.canvas.h")));
    let (dw, dh) = ((lw * res) as usize, (lh * res) as usize);
    std::fs::create_dir_all(out_dir).map_err(|e| crate::errors::TaleError::image_io(out_dir, e))?;
    println!("renderer: TaleRender CPU painter (src/scene.rs Painter) — no GPU involved; cards are raw painter output at {}x{} (logical {}x{} @ res {})", dw, dh, lw, lh, res);

    // ---- prims card ----
    #[cfg(feature = "gpupaint")]
    crate::gputrace::begin_card(dw as u32, dh as u32);
    let mut p = Painter::new(dw, dh);
    let ink = rgba_of(spec.get("ink").and_then(|v| v.as_str()).expect("spec.ink"));
    let sc = |v: f32| v * res;
    let scp = |q: (f32, f32)| (q.0 * res, q.1 * res);
    for op in spec.get("prims").expect("spec.prims").as_arr().expect("spec.prims array") {
        let kind = op.get("op").and_then(|v| v.as_str()).expect("spec op kind");
        let col = op.get("col").and_then(|v| v.as_str()).map(rgba_of).unwrap_or(ink);
        match kind {
            "line" => {
                let q = pts(op.get("p").expect("line.p"));
                p.stroke_polyline(&[scp(q[0]), scp(q[1])], sc(num(op.get("w").expect("line.w"))), col);
            }
            "polyline" => {
                let q: Vec<(f32, f32)> = pts(op.get("p").expect("polyline.p")).iter().map(|&q| scp(q)).collect();
                p.stroke_polyline(&q, sc(num(op.get("w").expect("polyline.w"))), col);
            }
            "quad" => {
                let q = pts(op.get("p").expect("quad.p"));
                p.stroke_quad(scp(q[0]), scp(q[1]), scp(q[2]), sc(num(op.get("w").expect("quad.w"))), col);
            }
            "stroke_ellipse" => {
                let c = pt(op.get("c").expect("stroke_ellipse.c"));
                p.stroke_ellipse(sc(c.0), sc(c.1), sc(num(op.get("rx").expect("rx"))), sc(num(op.get("ry").expect("ry"))), 0.0, sc(num(op.get("w").expect("w"))), col);
            }
            "fill_ellipse" => {
                let c = pt(op.get("c").expect("fill_ellipse.c"));
                p.fill_ellipse(sc(c.0), sc(c.1), sc(num(op.get("rx").expect("rx"))), sc(num(op.get("ry").expect("ry"))), 0.0, col);
            }
            "arc" => {
                let c = pt(op.get("c").expect("arc.c"));
                let r = num(op.get("r").expect("arc.r"));
                let (a0, a1) = (num(op.get("a0").expect("arc.a0")), num(op.get("a1").expect("arc.a1")));
                let q: Vec<(f32, f32)> = (0..=32)
                    .map(|i| {
                        let a = a0 + (i as f32 / 32.0) * (a1 - a0);
                        scp((c.0 + r * a.cos(), c.1 + r * a.sin()))
                    })
                    .collect();
                p.stroke_polyline(&q, sc(num(op.get("w").expect("arc.w"))), col);
            }
            other => panic!("spec: unknown prim op {other}"),
        }
    }
    #[cfg(feature = "gpupaint")]
    crate::gpupaint::finish_card(out_dir, "prims");
    save(&p.px, dw as u32, dh as u32, &format!("{out_dir}/prims.png"))?;

    // ---- toon card ----
    let toon = spec.get("toon").expect("spec.toon");
    let ts = toon_spec(
        toon.get("look").expect("toon.look"),
        toon.get("state").expect("toon.state"),
        toon.get("state").unwrap().get("mood").and_then(|v| v.as_str()).expect("toon.state.mood"),
    );
    // paint_toon's canvas is fixed at 2560x2816 (1280x1408 @ res 2).
    #[cfg(feature = "gpupaint")]
    crate::gputrace::begin_card(2560, 2816);
    let (px, w, h) = paint_toon(&ts);
    #[cfg(feature = "gpupaint")]
    crate::gpupaint::finish_card(out_dir, "toon");
    save(&px, w, h, &format!("{out_dir}/toon.png"))?;

    // ---- face cards ----
    let faces = spec.get("faces").expect("spec.faces");
    for m in faces.get("moods").expect("faces.moods").as_arr().expect("faces.moods array") {
        let m = m.as_str().expect("mood name");
        let ts = toon_spec(faces.get("look").expect("faces.look"), faces.get("state").expect("faces.state"), m);
        #[cfg(feature = "gpupaint")]
        crate::gputrace::begin_card(2560, 2816);
        let (px, w, h) = paint_toon(&ts);
        #[cfg(feature = "gpupaint")]
        crate::gpupaint::finish_card(out_dir, &format!("face-{m}"));
        save(&px, w, h, &format!("{out_dir}/face-{m}.png"))?;
    }
    Ok(())
}
