//! GPUFIRST Batch 2 — BROW seat: brow-offset states for the
//! gpupaint parity spread (Spec 5a, RUST-PARITY-SPECS §5a).
//!
//! Exposes `brow_spread()`; `gpupaint_states::state_spread()`
//! appends it with a single wiring line. These states exercise
//! the post-table brow offsets (`FullState.brow_lift` /
//! `brow_tilt`) that the CPU painter now applies after the
//! per-side row/override resolution. Brows trace as existing
//! `stroke_quad` prims, so no new GPU primitive is involved —
//! the GPU executor takes the moved brow geometry unchanged.
//!
//! Five entries (keeps the total spread at 80, inside the
//! `spread_shape` test's 60..=80 bound on this branch):
//! both lift extremes, both tilt extremes, and a 0.5/0.5-step
//! state on Smug (whose right brow carries a per-side override,
//! so the offsets are proven to ride after the overrides).

use crate::toonfull::{FullLook, FullState, Mood};

/// Same values as `gpupaint_states::base_look` (duplicated here
/// because that helper is private to its module).
fn base_look() -> FullLook {
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
        wing: false,
        wing_col: None,
        wing_lens: Vec::new(),
        ears: false,
        hair: false,
    }
}

fn brow_state(mood: Mood, brow_lift: f32, brow_tilt: f32) -> FullState {
    let mut st = FullState { mood, ..FullState::default() };
    st.brow_lift = brow_lift;
    st.brow_tilt = brow_tilt;
    st
}

pub(crate) fn brow_spread() -> Vec<(String, FullLook, FullState)> {
    vec![
        (
            "brow-lift-neg8".to_string(),
            base_look(),
            brow_state(Mood::Neutral, -8.0, 0.0),
        ),
        (
            "brow-lift-14".to_string(),
            base_look(),
            brow_state(Mood::Neutral, 14.0, 0.0),
        ),
        (
            "brow-tilt-neg10".to_string(),
            base_look(),
            brow_state(Mood::Neutral, 0.0, -10.0),
        ),
        (
            "brow-tilt-10".to_string(),
            base_look(),
            brow_state(Mood::Neutral, 0.0, 10.0),
        ),
        (
            "brow-smug-half-steps".to_string(),
            base_look(),
            brow_state(Mood::Smug, 0.5, 0.5),
        ),
    ]
}
