//! GPUFIRST Batch 3 — FLIPPITCH seat: flip (§5d) + bodyPitch
//! (Spec 3) states for the gpupaint parity spread, APPENDED to
//! the Batch-2 spread (221 states) by a single wiring line in
//! `gpupaint_states::state_spread()` — the first 221 entries are
//! untouched, in their original order.
//!
//! These states exercise the Batch-3 painter ports:
//! * `flip-*` — the whole-figure stage mirror (x -> 512 - x,
//!   after the canvas anchor, before lean). Flip is a host-side
//!   point-map reflection, so every recorded prim arrives
//!   already mirrored and the GPU executor needs nothing new —
//!   these states prove exactly that, including the asymmetric
//!   Suspicious face (the slit eye must mirror WITH the figure,
//!   not swap fields) and the S12 fixture axes
//!   ({neutral, happy} x bob {0, 14} x lean {0, 9}).
//! * `pitch-*` — the bodyPitch hip-pivot seam at the crawl
//!   working range and the Spec-3 acceptance configuration
//!   (headTilt 10 at pitch 60, counters 1 / 0.5 / 0), plus a
//!   negative pitch and a kid pitch.
//! * `flip-plate-*` / `pitch-plate-*` / `flippitch-plate-*` —
//!   B3-CARRY-1 proving cases: a held parcel plate under flip,
//!   under pitch, and under both, so the CPU-vs-GPU parity run
//!   measures the extended CompositePlate inverse map (flip +
//!   pitch in the WGSL `main_plate`) against the CPU's.
//!
//! Roll-up: `gpupaint_states::rollup_class` maps the
//! flip-/pitch-/flippitch- prefixes to "flip-pitch".
//!
//! Constructors model on gpupaint_states.rs (its helpers are
//! private, so the look factories are transcribed here). Every
//! value is reachable through the scene-format loader's state
//! keys ("flip" / "bodyPitch" / "bodyPitchCounter", plumbed in
//! Batch 2). Pure CPU data construction: no wgpu types, no I/O.

use crate::scene::paint_parcel;
use crate::toonfull::{FullLook, FullState, Legs, Mood, Plate};

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

/// A held parcel plate (same construction as
/// gpupaint_states::parcel_plate): 128x128 stage units centred
/// on the hand point.
fn parcel_plate(label: bool) -> Plate {
    let (px, w, h) = paint_parcel(label);
    Plate { px, w, h, x0: -64.0, y0: -64.0 }
}

fn st(mood: Mood) -> FullState {
    FullState { mood, ..FullState::default() }
}

/// pose_explain arms (film.rs:1459) — the held-plate pose the
/// shape battery's plate family also uses.
fn explain(s: &mut FullState) {
    s.arm_r = (68.0, -42.0);
    s.arm_l = (28.0, -24.0);
}

pub(crate) fn flippitch_spread() -> Vec<(String, FullLook, FullState)> {
    let mut v: Vec<(String, FullLook, FullState)> = Vec::new();
    let mut push = |name: &str, look: FullLook, state: FullState| {
        v.push((name.to_string(), look, state));
    };

    // -- Flip: the S12 axes on the base look.
    {
        let mut s = st(Mood::Neutral);
        s.flip = true;
        push("flip-neutral", base_look(), s);
    }
    {
        let mut s = st(Mood::Happy);
        s.flip = true;
        s.bob = 14.0;
        push("flip-happy-bob14", base_look(), s);
    }
    {
        let mut s = st(Mood::Neutral);
        s.flip = true;
        s.lean = 9.0;
        push("flip-neutral-lean9", base_look(), s);
    }
    {
        // Asymmetric face: the Suspicious slit eye (figure's
        // left) must mirror with the figure.
        let mut s = st(Mood::Suspicious);
        s.flip = true;
        push("flip-suspicious-slit", base_look(), s);
    }
    {
        // Anchor-engaging lean, flipped: the anchor is solved in
        // mirrored space (its ax negates the unflipped case).
        let mut s = st(Mood::Dizzy);
        s.arm_l = (150.0, 20.0);
        s.arm_r = (138.0, -30.0);
        s.head_tilt = 4.0;
        s.lean = 82.0;
        s.flip = true;
        push("flip-anchor-lying82", base_look(), s);
    }

    // -- bodyPitch: Spec-3 acceptance configuration + crawl range.
    for (tag, counter) in [("c100", 1.0f32), ("c050", 0.5), ("c000", 0.0)] {
        let mut s = st(Mood::Neutral);
        s.head_tilt = 10.0;
        s.body_pitch = 60.0;
        s.body_pitch_counter = counter;
        push(&format!("pitch-60-{tag}"), base_look(), s);
    }
    {
        let mut s = st(Mood::Neutral);
        s.body_pitch = -30.0;
        push("pitch-neg30", base_look(), s);
    }
    {
        // Crawl working range (moves.ts crawlClip posture
        // 55–70°): pitched torso over WALK legs — the legs must
        // stay un-pitched while the upper assembly rotates.
        let mut s = st(Mood::Determined);
        s.legs = Legs::Walk;
        s.phase = 1.5707964;
        s.body_pitch = 62.0;
        push("pitch-crawl62-walk", base_look(), s);
    }
    {
        let mut l = base_look();
        l.kid = true;
        let mut s = st(Mood::Happy);
        s.body_pitch = 60.0;
        push("pitch-kid-60", l, s);
    }

    // -- Flip + pitch composed (anchor ∘ flip ∘ lean ∘ pitch).
    {
        let mut s = st(Mood::Neutral);
        s.flip = true;
        s.body_pitch = 60.0;
        s.lean = 9.0;
        push("flippitch-60-lean9", base_look(), s);
    }

    // -- B3-CARRY-1 proving cases: held plates under flip /
    // pitch / both. IDs: flip-plate-*, pitch-plate-*,
    // flippitch-plate-*.
    {
        let mut s = st(Mood::Happy);
        explain(&mut s);
        s.look = [0.3, 0.0];
        s.flip = true;
        s.hold_r = Some(parcel_plate(true));
        push("flip-plate-holdr-label", base_look(), s);
    }
    {
        let mut s = st(Mood::Happy);
        explain(&mut s);
        s.look = [0.3, 0.0];
        s.flip = true;
        s.lean = 20.0;
        s.hold_l = Some(parcel_plate(false));
        push("flip-plate-holdl-lean20", base_look(), s);
    }
    {
        let mut s = st(Mood::Happy);
        explain(&mut s);
        s.look = [0.3, 0.0];
        s.body_pitch = 60.0;
        s.hold_r = Some(parcel_plate(true));
        push("pitch-plate-holdr-p60", base_look(), s);
    }
    {
        let mut s = st(Mood::Happy);
        explain(&mut s);
        s.look = [0.3, 0.0];
        s.flip = true;
        s.body_pitch = 60.0;
        s.hold_r = Some(parcel_plate(false));
        push("flippitch-plate-holdr-p60", base_look(), s);
    }

    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flippitch_spread_shape() {
        let v = flippitch_spread();
        assert_eq!(v.len(), 16, "flippitch battery size changed");
        let mut names: Vec<&str> = v.iter().map(|(n, _, _)| n.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), v.len(), "entry names must be unique");
        assert_eq!(v.iter().filter(|(n, _, _)| n.starts_with("flip-")).count(), 7);
        assert_eq!(v.iter().filter(|(n, _, _)| n.starts_with("pitch-")).count(), 7);
        assert_eq!(v.iter().filter(|(n, _, _)| n.starts_with("flippitch-")).count(), 2);
        // The B3-CARRY-1 proving cases are present by ID.
        for id in ["flip-plate-holdr-label", "pitch-plate-holdr-p60", "flippitch-plate-holdr-p60"] {
            assert!(v.iter().any(|(n, _, _)| n == id), "missing B3-CARRY-1 case {id}");
        }
        // Every entry actually carries its channel (no dead states).
        for (name, _, s) in &v {
            if name.starts_with("flip-") || name.starts_with("flippitch-") {
                assert!(s.flip, "{name}: flip not set");
            }
            if name.starts_with("pitch-") || name.starts_with("flippitch-") {
                assert!(s.body_pitch != 0.0, "{name}: body_pitch not set");
            }
        }
    }
}
