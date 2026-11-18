//! GPUFIRST Batch 3 — PORT-WING seat: wing-appendage states for
//! the gpupaint parity spread (RUST-PARITY-SPECS Spec 1).
//!
//! Exposes `wing_spread()`; `gpupaint_states::state_spread()`
//! appends it with a single wiring line. APPEND-ONLY: the
//! Batch 2 baseline spread (214 states: 77 base + 5 brow + 71
//! shape + 61 stroke, counted from the code) is untouched —
//! these 8 states are additions, taking the spread to 222
//! (inside the `spread_shape` test's 60..=230 bound).
//!
//! The wing paints through existing Prim kinds only (the
//! GPUFIRST-B2-CARRY §2 verdict: EXPRESSIBLE, no gap): panels
//! are `FillEllipse` (fill) + the flattened `StrokePolyline`
//! that `Painter::stroke_ellipse` records, the near-side
//! feather lines are `StrokeQuad`-flattened `StrokePolyline`s,
//! and the joint lines are `StrokePolyline`s — all geometry
//! host-side in device space, so the GPU executor takes the
//! wing unchanged. The trace test below proves the panel
//! counts prim-by-prim.
//!
//! The look is the reference bird as the bridge derives it
//! from the BIRD_RIG wing declaration (characters.ts: segments
//! 2, segLen 58, palette '#7FB6D9' -> wingCol '#7FB6D9',
//! wingLens [58, 46]) on the scene-format default body, so the
//! wing colour is distinguishable from every body shade.

use crate::toonfull::{FullLook, FullState};

/// The reference bird's look with a given declared wingLens.
fn pip_look(wing_lens: Vec<f32>) -> FullLook {
    FullLook {
        body: "#E8734A".to_string(),
        head: "#F7F7FA".to_string(),
        shade: None,
        pattern: "plain".to_string(),
        pat_col: None,
        hat: "none".to_string(),
        hat_col: None,
        band: None,
        neck: "none".to_string(),
        neck_col: None,
        glasses: false,
        mustache: false,
        mustache_col: None,
        kid: false,
        blush: false,
        freckles: false,
        pencil: false,
        monocle: false,
        hair_col: None,
        wing: true,
        wing_col: Some("#7FB6D9".to_string()),
        wing_lens,
        ears: false,
        hair: false,
    }
}

fn wing_state(wing_segs: Vec<f32>) -> FullState {
    let mut st = FullState::default();
    st.wing_segs = wing_segs;
    st
}

pub(crate) fn wing_spread() -> Vec<(String, FullLook, FullState)> {
    let mut v: Vec<(String, FullLook, FullState)> = Vec::new();
    // The reference bird at rest: scalar fallback (no wingSegs,
    // wingA 0) — the folded two-panel wing of Spec 1
    // acceptance 4.
    v.push(("wing-pip-folded-rest".to_string(), pip_look(vec![58.0, 46.0]), FullState::default()));
    // The evaluator's spread rest chain [1.5, 1.68] on the
    // declared 2 segments.
    v.push(("wing-pip-spread".to_string(), pip_look(vec![58.0, 46.0]), wing_state(vec![1.5, 1.68])));
    // Declared counts 1, 3, 5 (count 2 is the two entries
    // above): the spread chain at each count, with the
    // bridge-pattern lens for that count.
    v.push(("wing-segs1".to_string(), pip_look(vec![58.0]), wing_state(vec![1.5])));
    v.push(("wing-segs3".to_string(), pip_look(vec![58.0, 46.0, 34.0]), wing_state(vec![1.5, 1.68, 1.86])));
    v.push((
        "wing-segs5".to_string(),
        pip_look(vec![58.0, 46.0, 34.0, 24.0, 24.0]),
        wing_state(vec![1.5, 1.68, 1.86, 2.04, 2.22]),
    ));
    // The scalar wingA fallback, rotated (wingA-era staging).
    {
        let mut st = FullState::default();
        st.wing_a = 0.8;
        v.push(("wing-a-fallback-08".to_string(), pip_look(vec![58.0, 46.0]), st));
    }
    // Pip is a kid in the TS proof look (kid body drop + folded
    // fallback wing).
    {
        let mut look = pip_look(vec![58.0, 46.0]);
        look.kid = true;
        v.push(("wing-kid-folded".to_string(), look, FullState::default()));
    }
    // Spread wing under lean: the panel rotations compose with
    // the lean in the host-side point map.
    {
        let mut st = wing_state(vec![1.5, 1.68]);
        st.lean = 20.0;
        v.push(("wing-lean20-spread".to_string(), pip_look(vec![58.0, 46.0]), st));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gputrace::Prim;
    use crate::toonfull::paint_toon_full;

    #[test]
    fn wing_spread_shape() {
        let v = wing_spread();
        assert_eq!(v.len(), 8, "wing battery size changed");
        let mut names: Vec<&str> = v.iter().map(|(n, _, _)| n.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), v.len(), "entry names must be unique");
        for (n, _, _) in &v {
            assert!(n.starts_with("wing-"), "{n} lacks the wing- prefix");
        }
    }

    fn record(look: &FullLook, st: &FullState) -> Vec<Prim> {
        crate::gputrace::set_armed(true);
        crate::gputrace::begin(2560, 2816);
        let _ = paint_toon_full(look, st);
        let list = crate::gputrace::take();
        crate::gputrace::set_armed(false);
        list.prims
    }

    /// Spec 1 acceptance 2 at prim level: for declared counts
    /// 1, 3, 5 the traced paint carries exactly n near panel
    /// fills in the wing colour and n far panel fills in the
    /// 0.72 shade, over and above the identical wingless paint
    /// (same look with the wing flag off, same state). The
    /// StrokePolyline delta is 4n − 1: 2n panel outline strokes
    /// (the flattened stroke_ellipse), n feather quads, n − 1
    /// joint lines — every one an existing Prim kind.
    #[test]
    fn wing_trace_panel_counts() {
        let v = wing_spread();
        let near_col = (127u8, 182u8, 217u8, 255u8); // '#7FB6D9'
        let (fr, fg, fb) = crate::scene::shade_rgb("#7FB6D9", 0.72);
        let far_col = (fr, fg, fb, 255u8);
        for (name, n) in [("wing-segs1", 1usize), ("wing-segs3", 3), ("wing-segs5", 5)] {
            let (_, look, st) = v.iter().find(|(nm, _, _)| nm == name).expect(name);
            let prims = record(look, st);
            let mut wingless = pip_look(look.wing_lens.clone());
            wingless.wing = false;
            let base = record(&wingless, st);
            let count_fill = |ps: &[Prim], col| {
                ps.iter()
                    .filter(|p| matches!(p, Prim::FillEllipse { col: c, .. } if *c == col))
                    .count()
            };
            let count_polylines = |ps: &[Prim]| {
                ps.iter().filter(|p| matches!(p, Prim::StrokePolyline { .. })).count()
            };
            assert_eq!(count_fill(&prims, near_col), n, "{name}: near panels");
            assert_eq!(count_fill(&prims, far_col), n, "{name}: far panels");
            assert_eq!(count_fill(&base, near_col), 0, "{name}: wingless baseline has no near panels");
            assert_eq!(count_fill(&base, far_col), 0, "{name}: wingless baseline has no far panels");
            let fill_delta = prims.iter().filter(|p| matches!(p, Prim::FillEllipse { .. })).count()
                - base.iter().filter(|p| matches!(p, Prim::FillEllipse { .. })).count();
            assert_eq!(fill_delta, 2 * n, "{name}: FillEllipse delta must be 2n (one panel per segment per side)");
            assert_eq!(
                count_polylines(&prims) - count_polylines(&base),
                4 * n - 1,
                "{name}: StrokePolyline delta must be 4n-1 (2n outlines + n feathers + n-1 joints)"
            );
        }
    }
}
