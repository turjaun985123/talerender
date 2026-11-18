//! GPUFIRST Batch 3 — EARSHAIR seat: fox-ear + hair states for
//! the gpupaint parity spread (Spec §5b / §5c,
//! RUST-PARITY-SPECS).
//!
//! Exposes `earshair_spread()`; `gpupaint_states::state_spread()`
//! appends it with a single wiring line. These states exercise
//! the ear painter (Spec §5b) and the hair painter (Spec §5c)
//! that the CPU painter now implements in toonfull.rs. Both
//! channels trace through EXISTING prim kinds only (per
//! docs/integration/GPUFIRST-B2-CARRY.md §2: ears =
//! FillPolygon + StrokePolyline; hair = StrokeQuad x2 per
//! strand), so no new GPU primitive is involved — the GPU
//! executor takes the recorded geometry unchanged.
//!
//! Eleven entries, mirroring the specs' acceptance fixtures
//! exactly (the same cells the toonfull::earshair_tests CPU
//! acceptance runs):
//! * ears (6): the S9 fixture states on look {ears:'fox',
//!   body:'#DE7A35'} — default, earA +/-0.3, earA with a
//!   per-ear override on each side, and both overrides.
//!   IDs: ears-default, ears-eara-pos03, ears-eara-neg03,
//!   ears-earal-override, ears-earar-override,
//!   ears-both-overrides.
//! * hair (5): the S10 fixture cells on look {hair:true} —
//!   absent hairSegs, all-zero hairSegs (the EQ pair: both
//!   paint the 6 rest strands), mixed angles, the 3-strand
//!   cell, and hairCol '#101010'.
//!   IDs: hair-absent, hair-zeroes, hair-mixed,
//!   hair-three-strands, hair-col-101010.

use crate::toonfull::{FullLook, FullState};

/// Same values as `gpupaint_states::base_look` (duplicated here
/// because that helper is private to its module).
fn base_look(body: &str) -> FullLook {
    FullLook {
        body: body.to_string(),
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

pub(crate) fn earshair_spread() -> Vec<(String, FullLook, FullState)> {
    let mut v: Vec<(String, FullLook, FullState)> = Vec::new();

    // -- Ears: the S9 fixture cells (look {ears:'fox',
    // body:'#DE7A35'}, mood neutral = FullState::default).
    let ear_look = || {
        let mut l = base_look("#DE7A35");
        l.ears = true;
        l
    };
    v.push(("ears-default".to_string(), ear_look(), FullState::default()));
    {
        let mut s = FullState::default();
        s.ear_a = 0.3;
        v.push(("ears-eara-pos03".to_string(), ear_look(), s));
    }
    {
        let mut s = FullState::default();
        s.ear_a = -0.3;
        v.push(("ears-eara-neg03".to_string(), ear_look(), s));
    }
    {
        let mut s = FullState::default();
        s.ear_a = 0.3;
        s.ear_al = Some(-0.2);
        v.push(("ears-earal-override".to_string(), ear_look(), s));
    }
    {
        let mut s = FullState::default();
        s.ear_a = 0.3;
        s.ear_ar = Some(0.25);
        v.push(("ears-earar-override".to_string(), ear_look(), s));
    }
    {
        let mut s = FullState::default();
        s.ear_al = Some(-0.2);
        s.ear_ar = Some(0.25);
        v.push(("ears-both-overrides".to_string(), ear_look(), s));
    }

    // -- Hair: the S10 fixture cells (look {hair:true} on the
    // base body colour, mood neutral).
    let hair_look = || {
        let mut l = base_look("#E8734A");
        l.hair = true;
        l
    };
    v.push(("hair-absent".to_string(), hair_look(), FullState::default()));
    {
        let mut s = FullState::default();
        s.hair_segs = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        v.push(("hair-zeroes".to_string(), hair_look(), s));
    }
    {
        let mut s = FullState::default();
        s.hair_segs = vec![0.3, -0.3, 0.15, 0.0, 0.0, -0.15];
        v.push(("hair-mixed".to_string(), hair_look(), s));
    }
    {
        let mut s = FullState::default();
        s.hair_segs = vec![0.2, 0.0, -0.2];
        v.push(("hair-three-strands".to_string(), hair_look(), s));
    }
    {
        let mut l = hair_look();
        l.hair_col = Some("#101010".to_string());
        v.push(("hair-col-101010".to_string(), l, FullState::default()));
    }

    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toonfull::paint_toon_full;
    use std::collections::HashSet;

    fn fnv1a(px: &[u8]) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in px {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    #[test]
    fn earshair_spread_shape() {
        let v = earshair_spread();
        assert_eq!(v.len(), 11, "6 ears + 5 hair entries");
        assert_eq!(v.iter().filter(|(n, _, _)| n.starts_with("ears-")).count(), 6);
        assert_eq!(v.iter().filter(|(n, _, _)| n.starts_with("hair-")).count(), 5);
        let mut names: Vec<&str> = v.iter().map(|(n, _, _)| n.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), v.len(), "entry names must be unique");
    }

    /// The battery entries must actually exercise the painters
    /// at CPU level (the GPU replay compares against these CPU
    /// paints): S9 -> 6 distinct classes, S10 -> 4 distinct
    /// classes with the absent == zeroes EQ pair held.
    #[test]
    fn earshair_spread_paints_refine_to_spec_classes() {
        let v = earshair_spread();
        let hash = |name: &str| {
            let (_, l, s) = v.iter().find(|(n, _, _)| n == name).expect(name);
            fnv1a(&paint_toon_full(l, s).0)
        };
        let ear_hashes: HashSet<u64> = v
            .iter()
            .filter(|(n, _, _)| n.starts_with("ears-"))
            .map(|(_, l, s)| fnv1a(&paint_toon_full(l, s).0))
            .collect();
        assert_eq!(ear_hashes.len(), 6, "ears battery must paint 6 distinct classes");
        assert_eq!(hash("hair-absent"), hash("hair-zeroes"), "hair EQ pair must hold");
        let hair_classes: HashSet<u64> = ["hair-absent", "hair-mixed", "hair-three-strands", "hair-col-101010"]
            .iter()
            .map(|n| hash(n))
            .collect();
        assert_eq!(hair_classes.len(), 4, "hair battery must paint 4 distinct classes");
    }
}
