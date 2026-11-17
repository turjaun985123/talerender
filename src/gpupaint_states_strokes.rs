//! GPUFIRST Batch 2 — STROKE-BATT seat: the dedicated STROKES
//! parity battery.
//!
//! `stroke_spread()` returns (name, look, state) pairs whose
//! paints isolate the painter's stroke behaviour — the
//! union-distance capsule strokes with flattened-arc cap fans
//! (INVB §9: the painter's largest and parity-riskiest class),
//! exercised through the `stroke_polyline` / `stroke_quad` /
//! `stroke_ellipse` prims (src/gputrace.rs). CI parity run #1
//! covered strokes only in situ (M4 cards + the mixed Phase-0
//! spread); this battery stresses them directly:
//!
//! * `stroke-laugh-*` — the laugh-mouth outline quirk in situ
//!   (INVB §4.5: the outline `stroke_ellipse` lands on the
//!   tongue ellipse, never the mouth shape) across the missing
//!   head_turn {-1, 0, 1} x cam_pitch {-1, 0, 1} grid. The
//!   Phase-0 spread has mood-laugh at turn/pitch 0 only.
//! * `stroke-walk-*` / `stroke-run-*` — leg stroke_quad-flattened
//!   polylines sweeping the FULL gait cycle at 8 phases each
//!   (the Phase-0 spread samples Walk at quadrants only, Run at
//!   2 phases). Walk bob follows film's walk_bob law
//!   (amp 5: 5*(1-|sin phase|), as staged in s02).
//! * `stroke-sit-*` — the sit leg's 3-point polyline (hip->knee
//!   ->foot, lw 15): the sharpest fixed join in the leg set, at
//!   the default / film / high sit drops.
//! * `stroke-arm-*` — arm stroke polylines (flattened FK quads,
//!   lw 14) at extreme elbow joins: acute folds (|elbow| >= 140,
//!   interior angle <= 40 deg — the cap/join fan stress case)
//!   and obtuse/raised combinations, incl. one paint combining
//!   the film's two staged arm extremes (listen + celebrate).
//! * `stroke-mood-*` — brow + mouth stroke_quads across a
//!   14-mood sweep, incl. the per-side brow overrides (Smug/Sly
//!   right brow, Suspicious left brow, Puzzled split brows) and
//!   every mouth branch family (quad smiles/frowns, the Worried
//!   flattened-cubic polyline, the straight-line mouths).
//! * `stroke-ring-*` — stroke_ellipse rings: the head outline
//!   under squash {-0.2, 0, 0.2} x head_tilt {-5, 0, 6} (the
//!   film's tilt extremes, pose_listen / pose_think), glasses
//!   rims + bridge, the monocle rim + chain quad, and the two
//!   ring-stroked hats (cap: crown ring + open brim + button
//!   ring; bowler: brim ring + dome ring).
//!
//! Roll-up: every name carries the `stroke-` prefix, so
//! `state_group(name) == "stroke"` for the whole battery — the
//! coordinator reads these cases out of the parity summary by
//! that prefix (under the current `rollup_class` they fall in
//! the default bucket; the prefix is the strokes identifier).
//!
//! Constructors model on src/gpupaint_states.rs (its helpers
//! are private to that module, so the two look factories are
//! transcribed here). Every value is film-staged or reachable
//! through the scene-format loader's free f32 / string fields,
//! exactly as in the Phase-0 spread. Pure CPU data construction:
//! no wgpu types, no I/O, no env.

use crate::toonfull::{FullLook, FullState, Legs, Mood};

// ---------------------------------------------------------------------
// Look constructors (transcribed from gpupaint_states.rs).
// ---------------------------------------------------------------------

/// The scene-format default look (base for the face / ring
/// sweeps so stroke features are measured unobstructed).
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
    }
}

/// Registry idx 5 — shOnoda (film.rs:608): cap look, no shade
/// override. Base for the leg sweeps (s02's staged look).
fn look_onoda_sh() -> FullLook {
    FullLook {
        body: "#5C6644".to_string(),
        head: "#F7F7FA".to_string(),
        shade: None,
        pattern: "plain".to_string(),
        pat_col: None,
        hat: "cap".to_string(),
        hat_col: Some("#4C5638".to_string()),
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
    }
}

/// Registry idx 0 / 10 — jgOnoda (film.rs:603-607): cap +
/// explicit shade. Base for the film sit entry.
fn look_onoda_jg() -> FullLook {
    let mut l = look_onoda_sh();
    l.shade = Some("#878E6E".to_string());
    l
}

fn st(mood: Mood) -> FullState {
    FullState { mood, ..FullState::default() }
}

// ---------------------------------------------------------------------
// The strokes battery.
// ---------------------------------------------------------------------

pub(crate) fn stroke_spread() -> Vec<(String, FullLook, FullState)> {
    let mut v: Vec<(String, FullLook, FullState)> = Vec::new();
    let mut push = |name: &str, look: FullLook, state: FullState| {
        v.push((name.to_string(), look, state));
    };

    // -- Laugh-mouth quirk grid: mood Laugh across head_turn
    // {-1, 0, 1} x cam_pitch {-1, 0, 1}. head_turn moves the
    // face centre (fx = hx + turn*30) and narrows the eye span
    // (1 - 0.18*|turn|); cam_pitch scales the head radius
    // (1 + 0.04*pitch) and shifts ey (+8*pitch) — both move the
    // tongue ellipse the quirk stroke lands on, and the masked
    // tongue fill / teeth blend under it. All values are
    // scene-format free f32s (the film stages turn -0.45..+0.5
    // and pitch 0.28747; +-1 is the loader-reachable extreme).
    for (ttag, turn) in [("neg1", -1.0f32), ("zero", 0.0), ("pos1", 1.0)] {
        for (ptag, pitch) in [("neg1", -1.0f32), ("zero", 0.0), ("pos1", 1.0)] {
            let mut s = st(Mood::Laugh);
            s.head_turn = turn;
            s.cam_pitch = pitch;
            push(
                &format!("stroke-laugh-turn-{ttag}-pitch-{ptag}"),
                base_look(),
                s,
            );
        }
    }

    // -- Leg gait sweeps: Walk and Run at 8 phases sweeping the
    // full cycle (phase = k*PI/4, k = 0..7), on the shore Onoda
    // look with s02's staged arms (16,-6)/(10,6). The leg stroke
    // is a 20-segment flattened quad hip->knee->foot, lw 15; the
    // knee/foot geometry is sin/cos of phase (mirrored by PI for
    // the far leg), so 8 phases hit every join configuration:
    // both feet planted, both lifts peaking, and the crossovers.
    // Walk bob follows film's walk_bob law at amp 5; Run leaves
    // bob 0 (the film never stages Run; the Phase-0 run entries
    // do the same).
    for k in 0..8usize {
        let phase = k as f32 * std::f32::consts::FRAC_PI_4;
        let mut s = st(Mood::Determined);
        s.arm_r = (16.0, -6.0);
        s.arm_l = (10.0, 6.0);
        s.legs = Legs::Walk;
        s.phase = phase;
        s.bob = 5.0 * (1.0 - phase.sin().abs());
        push(&format!("stroke-walk-p{k}"), look_onoda_sh(), s);
    }
    for k in 0..8usize {
        let phase = k as f32 * std::f32::consts::FRAC_PI_4;
        let mut s = st(Mood::Determined);
        s.arm_r = (16.0, -6.0);
        s.arm_l = (10.0, 6.0);
        s.legs = Legs::Run;
        s.phase = phase;
        push(&format!("stroke-run-p{k}"), look_onoda_sh(), s);
    }

    // -- Sit legs: the 3-point sit polyline's fixed knee join
    // under three sit drops (the drop moves hip_y, hence the
    // whole polyline, through the painter's (426-drop)/0.98).
    {
        // Default drop (None -> 220.2), neutral standing arms.
        let mut s = st(Mood::Neutral);
        s.legs = Legs::Sit;
        push("stroke-sit-default", base_look(), s);
    }
    {
        // s06 film sit (film.rs:1839-1841): pose_mourn + the
        // film's exact sit_drop expression + s06 arm overrides,
        // on the hero look (transcribed from legs-sit-film).
        let mut s = st(Mood::Sad);
        s.arm_r = (16.0, -6.0);
        s.arm_l = (8.0, 5.0);
        s.lean = 5.0;
        s.head_tilt = -4.0;
        s.bob = 4.0;
        s.legs = Legs::Sit;
        s.sit_drop = Some((0.34 * 1024.0 / (1.75 / 0.96)) as f32);
        push("stroke-sit-film", look_onoda_jg(), s);
    }
    {
        // High seat (sit_drop 900, the canvas_anchor_tests
        // value): the polyline's foot clamps at y=1000.
        let mut s = st(Mood::Neutral);
        s.legs = Legs::Sit;
        s.sit_drop = Some(900.0);
        push("stroke-sit-high", base_look(), s);
    }

    // -- Arm FK extremes: the arm stroke is a 20-segment
    // flattened quad shoulder->elbow->hand, lw 14; the join
    // stress is the elbow, whose interior angle is
    // 180 - |elbow| (folded back on itself as |elbow| -> 180).
    // Acute set: |elbow| 140..150. Obtuse set: |elbow| <= 30
    // with raised shoulders. All angles are scene-format free
    // f32 pairs; the mixed entry combines the film's own staged
    // extremes (pose_listen arm_r (118,-105), film.rs:1479;
    // pose_celebrate (150,14), film.rs:1465) in one paint.
    {
        let mut s = st(Mood::Neutral);
        s.arm_r = (60.0, 150.0); // right elbow folded acute
        push("stroke-arm-acute-r", base_look(), s);
    }
    {
        let mut s = st(Mood::Neutral);
        s.arm_l = (60.0, -150.0); // left elbow folded acute (mirror fold)
        push("stroke-arm-acute-l", base_look(), s);
    }
    {
        let mut s = st(Mood::Neutral);
        s.arm_r = (140.0, -140.0);
        s.arm_l = (140.0, 140.0); // both elbows acute, arms raised
        push("stroke-arm-acute-both", base_look(), s);
    }
    {
        let mut s = st(Mood::Neutral);
        s.arm_r = (60.0, 30.0); // obtuse elbow (interior 150)
        push("stroke-arm-obtuse-r", base_look(), s);
    }
    {
        let mut s = st(Mood::Neutral);
        s.arm_l = (170.0, 25.0);
        s.arm_r = (170.0, -25.0); // both raised, obtuse elbows
        push("stroke-arm-obtuse-raised", base_look(), s);
    }
    {
        let mut s = st(Mood::Neutral);
        s.arm_r = (118.0, -105.0); // pose_listen (acute-leaning)
        s.arm_l = (150.0, 14.0); // pose_celebrate (obtuse)
        push("stroke-arm-extreme-mixed", base_look(), s);
    }

    // -- Brow/mouth stroke-quad sweep: 14 moods at the neutral
    // standing state on the base look. Brows are stroke_quads
    // (lw 12) whose (lift, tilt) come from Mood::brow() with
    // per-side overrides for Smug/Sly (right -> (16,-4)),
    // Suspicious (left -> (18,-6)) and Puzzled (left (16,-2) /
    // right (-2,8)) — all three override moods are included.
    // Mouths cover every stroke branch family: quad smile
    // (Happy/Proud-family, Neutral default), quad smirk
    // (Smug/Sly), quad frown (Angry/Sad), the Worried flattened
    // cubic polyline, the Tired straight-line polyline, the
    // Determined near-flat polyline, the Embarrassed chained
    // quads, the Suspicious/Puzzled small quads, and Shock's
    // open-mouth stroke_ellipse. (Laugh's mouth is the grid
    // above; Dizzy/Deadpan share Tired's straight-line branch,
    // Awe shares Shock's eye/mouth family, Relieved shares
    // Happy's smile/eye-arc family.)
    for (name, mood) in [
        ("neutral", Mood::Neutral),
        ("smug", Mood::Smug),
        ("happy", Mood::Happy),
        ("shock", Mood::Shock),
        ("worried", Mood::Worried),
        ("angry", Mood::Angry),
        ("sad", Mood::Sad),
        ("sly", Mood::Sly),
        ("proud", Mood::Proud),
        ("tired", Mood::Tired),
        ("determined", Mood::Determined),
        ("embarrassed", Mood::Embarrassed),
        ("suspicious", Mood::Suspicious),
        ("puzzled", Mood::Puzzled),
    ] {
        push(&format!("stroke-mood-{name}"), base_look(), st(mood));
    }

    // -- Stroke-ellipse rings: head outline under squash
    // {-0.2, 0, 0.2} x head_tilt {-5, 0, 6}. Squash rescales the
    // head ellipse about its centre ((1-0.3*sq) x (1+sq)) —
    // negative squash is the loader-reachable stretch inverse
    // (the film stages only -0.04..+0.5; the painter's factors
    // are linear in sq, so -0.2 exercises the same code path
    // mirrored). Tilts are the film's staged extremes
    // (pose_listen -5, pose_think +6).
    for (qtag, squash) in [("n020", -0.2f32), ("000", 0.0), ("p020", 0.2)] {
        for (ttag, tilt) in [("m5", -5.0f32), ("00", 0.0), ("p6", 6.0)] {
            let mut s = st(Mood::Neutral);
            s.squash = squash;
            s.head_tilt = tilt;
            push(&format!("stroke-ring-sq{qtag}-tilt{ttag}"), base_look(), s);
        }
    }
    {
        // Glasses: two r30 stroke_ellipse rims (lw 5) + the
        // bridge stroke_polyline, over the eye stack.
        let mut l = base_look();
        l.glasses = true;
        push("stroke-ring-glasses", l, st(Mood::Neutral));
    }
    {
        // Monocle: gold r30 stroke_ellipse rim (lw 5) at the
        // right eye + the chain stroke_quad to (fx+ex+10,
        // ey+140).
        let mut l = base_look();
        l.monocle = true;
        push("stroke-ring-monocle", l, st(Mood::Neutral));
    }
    {
        // Cap: crown ring stroke_polyline (closed, lw 9), brim
        // stroke_polyline (open — chord not stroked), button
        // stroke_ellipse (lw 9).
        let mut l = base_look();
        l.hat = "cap".to_string();
        l.hat_col = Some("#4C5638".to_string());
        push("stroke-ring-hat-cap", l, st(Mood::Neutral));
    }
    {
        // Bowler: brim stroke_ellipse (lw 9) + dome ring
        // stroke_polyline (lw 9) + band ring.
        let mut l = base_look();
        l.hat = "bowler".to_string();
        l.hat_col = Some("#2E2A3A".to_string());
        push("stroke-ring-hat-bowler", l, st(Mood::Neutral));
    }

    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpupaint_states::state_group;

    #[test]
    fn stroke_spread_shape() {
        let v = stroke_spread();
        // 9 laugh + 8 walk + 8 run + 3 sit + 6 arm + 14 mood
        // + 9 ring grid + glasses/monocle + cap/bowler = 61.
        assert_eq!(v.len(), 61, "stroke battery size changed");
        let mut names: Vec<&str> = v.iter().map(|(n, _, _)| n.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), v.len(), "entry names must be unique");
        // The whole battery carries the strokes roll-up prefix.
        for (n, _, _) in &v {
            assert!(n.starts_with("stroke-"), "{n} lacks the stroke- prefix");
            assert_eq!(state_group(n), "stroke", "{n} must group as stroke");
        }
        // Brief-mandated families.
        for prefix in [
            "stroke-laugh-turn-neg1-pitch-neg1",
            "stroke-laugh-turn-zero-pitch-zero",
            "stroke-laugh-turn-pos1-pitch-pos1",
            "stroke-walk-p0",
            "stroke-walk-p7",
            "stroke-run-p0",
            "stroke-run-p7",
            "stroke-sit-default",
            "stroke-sit-film",
            "stroke-sit-high",
            "stroke-arm-acute-r",
            "stroke-arm-acute-l",
            "stroke-arm-obtuse-r",
            "stroke-mood-smug",
            "stroke-mood-sly",
            "stroke-mood-suspicious",
            "stroke-mood-puzzled",
            "stroke-ring-sqn020-tiltm5",
            "stroke-ring-sq000-tilt00",
            "stroke-ring-sqp020-tiltp6",
            "stroke-ring-glasses",
            "stroke-ring-monocle",
            "stroke-ring-hat-cap",
            "stroke-ring-hat-bowler",
        ] {
            assert!(
                v.iter().any(|(n, _, _)| n == prefix),
                "missing family {prefix}"
            );
        }
        assert_eq!(
            v.iter().filter(|(n, _, _)| n.starts_with("stroke-laugh-")).count(),
            9,
            "laugh grid must be 3x3"
        );
        assert_eq!(
            v.iter().filter(|(n, _, _)| n.starts_with("stroke-mood-")).count(),
            14,
            "mood sweep must cover 14 moods"
        );
    }
}

