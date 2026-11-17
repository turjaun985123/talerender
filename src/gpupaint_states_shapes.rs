//! GPUFIRST Batch 2 — SHAPE-BATT seat: dedicated per-class parity
//! batteries for the shape prims of the GPU executor
//! (src/gpupaint.rs + gpupaint.wgsl, feature `gpupaint`).
//!
//! `shape_spread()` returns (name, look, state) pairs whose paints
//! ISOLATE one shape class each, so the coordinator's parity
//! summary can report per-class numbers instead of the mixed
//! in-situ numbers of CI parity run #1 (77/82 byte-identical over
//! the 75-state Phase-0 spread, where every paint mixes classes).
//! The classes, per INVB §9 (per-class risk) and §6
//! (canvas_anchor + composite_plate):
//!
//! * `shape-fill-*` — FILLS (`fill_polygon`): the bean body fill
//!   under lean / bob extremes, the mustache's chained-cubic
//!   wings, the Angry teeth rounded-rect, and the neckwear set
//!   (tie blade / scarf tail+band / collar triangles).
//! * `shape-ell-*` — ELLIPSES + ARCS (`fill_ellipse` /
//!   `stroke_ellipse`): high-eccentricity tear drops (7x14 stage,
//!   the INVB-flagged approximate-SDF stress case), pupils at the
//!   four gaze corners {-1,1}^2, hand/foot discs at FK extremes,
//!   and the flower hat's six petal ellipses (+stroke rings).
//! * `shape-shd-*` — HEAD SHADE (the "gradient" class): this
//!   painter has NO gradient primitive — the shading surface is
//!   the `head_shade` prim (an open-coded even-odd coverage
//!   product on the CPU, INVB §3.5/§9) plus the masked bean
//!   shade/highlight ellipses (`shape-msk-*`). The battery grids
//!   head_shade over squash {-0.2, 0, 0.2} x head_turn {-1, 0, 1}
//!   x cam_pitch {-1, 0, 1} (27 states).
//! * `shape-msk-*` — MASKED PATTERNS: stripes / apron / vest via
//!   `fill_polygon_masked` + `fill_ellipse_masked` over the bean
//!   `polygon_mask`. INVB flags the BINARY mask edge (centre-
//!   sampled, no AA) as the parity surface; each pattern is run
//!   at lean -20 / +20 so the mask edge sweeps across the bean.
//! * `shape-mou-*` — MASKED MOUTHS: the `open_mouth` closure
//!   variants (Shock; the talk sweep {0.06, 0.3, 1.0} crossing
//!   the tongue gate h > 12; Awe, whose tongue is suppressed by
//!   the tongue:false flag alone) and the Laugh tongue
//!   (`fill_ellipse_masked` over the laugh-mouth polygon mask,
//!   plus the masked teeth `masked_blend`).
//! * `shape-plt-*` — PLATES + ANCHOR: `composite_plate` held
//!   parcels (both hands, both parcel variants, and both-hands
//!   mixes) at anchor-engaging states. The anchor is host-side
//!   geometry (INVB §6.1): these cases prove the GPU path's
//!   anchor+plate composition matches the CPU's where the anchor
//!   actually engages — engagement is asserted in the tests
//!   below, mirroring gpupaint_states.rs's
//!   `anchor_entries_actually_engage`.
//!
//! Roll-up mapping (for the coordinator): every name starts
//! `shape-`, so `state_group(name) == "shape"` selects the whole
//! battery; the SECOND segment selects the report class —
//! fill -> fills, ell -> ellipses-arcs, shd + msk + mou ->
//! shade+masks, plt -> plates+anchor. (Under the current
//! `rollup_class` these fall in the default "transforms" bucket;
//! the two-segment prefix is the identifier, as with the sibling
//! STROKE-BATT battery.)
//!
//! Constructors model on src/gpupaint_states.rs (its helpers are
//! private to that module, so the look factories are transcribed
//! here). Every value is film-staged or reachable through the
//! scene-format loader's free f32 / string fields, exactly as in
//! the Phase-0 spread. Pure CPU data construction: no wgpu
//! types, no I/O, no env.

use crate::scene::paint_parcel;
use crate::toonfull::{FullLook, FullState, Legs, Mood, Plate};

// ---------------------------------------------------------------------
// Look constructors (transcribed from gpupaint_states.rs).
// ---------------------------------------------------------------------

/// The scene-format default look (scenefile.rs character node).
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

/// ex07's Finn (toonfull.rs canvas_anchor_tests::finn_look):
/// plain, hatless — the B-1 anchor case look, used for the
/// plates+anchor family so its anchor behaviour is the known one.
fn look_finn() -> FullLook {
    FullLook {
        body: "#DE7A35".to_string(),
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

/// A held parcel plate from scene.rs `paint_parcel` (256x256 px
/// = 128x128 stage units at res 2, centred on the hand point) —
/// transcribed from gpupaint_states.rs::parcel_plate. `label`
/// adds the label paper + stamp rects.
fn parcel_plate(label: bool) -> Plate {
    let (px, w, h) = paint_parcel(label);
    Plate { px, w, h, x0: -64.0, y0: -64.0 }
}

fn st(mood: Mood) -> FullState {
    FullState { mood, ..FullState::default() }
}

// ---------------------------------------------------------------------
// The battery.
// ---------------------------------------------------------------------

pub(crate) fn shape_spread() -> Vec<(String, FullLook, FullState)> {
    let mut v: Vec<(String, FullLook, FullState)> = Vec::new();
    let mut push = |name: &str, look: FullLook, state: FullState| {
        v.push((name.to_string(), look, state));
    };

    // -- FILLS (shape-fill-): fill_polygon isolation. The bean
    // fill is a 109-pt flattened-cubic polygon (INVB §3.2); lean
    // rotates it about the feet and bob translates it, so the
    // polygon edge lands on fractional pixels across the sweep.
    for (tag, lean) in [("neg30", -30.0f32), ("pos30", 30.0)] {
        let mut s = st(Mood::Neutral);
        s.lean = lean;
        push(&format!("shape-fill-bean-lean-{tag}"), base_look(), s);
    }
    for (tag, bob) in [("neg40", -40.0f32), ("pos40", 40.0)] {
        let mut s = st(Mood::Neutral);
        s.bob = bob;
        push(&format!("shape-fill-bean-bob-{tag}"), base_look(), s);
    }
    {
        // Mustache: 4 chained cubics, symmetric wings to fx+-64
        // (INVB §4.6) — the chained-cubic fill stress case.
        let mut l = base_look();
        l.mustache = true;
        push("shape-fill-mustache", l, st(Mood::Neutral));
    }
    {
        // Mustache under head_turn + tilt extremes: the wings
        // shear asymmetrically through the hpt stack.
        let mut l = base_look();
        l.mustache = true;
        let mut s = st(Mood::Neutral);
        s.head_turn = 1.0;
        s.head_tilt = 6.0;
        push("shape-fill-mustache-turn", l, s);
    }
    // Angry teeth: white rounded-rect fill_polygon (48x14, r6)
    // inside the frown (INVB §4.5).
    push("shape-fill-teeth-angry", base_look(), st(Mood::Angry));
    // Neckwear set (INVB §3.3): each branch's fill_polygon(s) on
    // the base look, neck_col None = painter defaults.
    for (tag, neck) in [("tie", "tie"), ("scarf", "scarf"), ("collar", "collar")] {
        let mut l = base_look();
        l.neck = neck.to_string();
        push(&format!("shape-fill-neck-{tag}"), l, st(Mood::Neutral));
    }

    // -- ELLIPSES + ARCS (shape-ell-): fill_ellipse isolation.
    // Tear drops are r 7x14 stage ellipses (INVB §4.6) — the
    // highest-eccentricity analytic shape in the painter, where
    // the approximate ellipse SDF is weakest (INVB §9).
    for (tag, mood, tears) in [
        ("neutral", Mood::Neutral, true),
        ("sad", Mood::Sad, true),     // tears also fire via mood
        ("worried", Mood::Worried, true), // + sweat drop via mood
    ] {
        let mut s = st(mood);
        s.tears = tears;
        push(&format!("shape-ell-tears-{tag}"), base_look(), s);
    }
    // Pupils at the four gaze corners {-1,1}^2 (the painter
    // offsets pupils by look*(9,8) device-scaled, no clamp).
    for (tag, look) in [
        ("nn", [-1.0f32, -1.0f32]),
        ("pp", [1.0, 1.0]),
        ("np", [-1.0, 1.0]),
        ("pn", [1.0, -1.0]),
    ] {
        let mut s = st(Mood::Neutral);
        s.look = look;
        push(&format!("shape-ell-gaze-{tag}"), base_look(), s);
    }
    {
        // Hands at FK extremes: left arm folded up (170 deg),
        // right arm swept down-back (-160 deg fore) — hand discs
        // land far from the torso, on empty canvas.
        let mut s = st(Mood::Neutral);
        s.arm_l = (170.0, 20.0);
        s.arm_r = (-30.0, -160.0);
        push("shape-ell-fk-hands", base_look(), s);
    }
    {
        // Feet at the Run cycle's opposite phase (amp 1.0, lift
        // <= 70 stage): feet ellipses at maximal separation.
        let mut s = st(Mood::Determined);
        s.arm_r = (16.0, -6.0);
        s.arm_l = (10.0, 6.0);
        s.legs = Legs::Run;
        s.phase = std::f32::consts::PI;
        push("shape-ell-fk-feet-run", base_look(), s);
    }
    {
        // Sit feet: the sit branch's foot ellipses at a mid seat
        // (film s06 uses sit_drop ~191; 200 is the same family).
        let mut s = st(Mood::Sad);
        s.arm_r = (16.0, -6.0);
        s.arm_l = (8.0, 5.0);
        s.lean = 5.0;
        s.head_tilt = -4.0;
        s.bob = 4.0;
        s.legs = Legs::Sit;
        s.sit_drop = Some(200.0);
        push("shape-ell-fk-feet-sit", base_look(), s);
    }
    {
        // Flower hat: 6 petal fill_ellipses + stroke_ellipse
        // rings + centre disc (INVB §5).
        let mut l = base_look();
        l.hat = "flower".to_string();
        push("shape-ell-flower-petals", l, st(Mood::Neutral));
    }
    {
        // Petals under head_turn -1 + squash 0.2: petal centres
        // ride the hpt stack to the head's edge, radii squash.
        let mut l = base_look();
        l.hat = "flower".to_string();
        let mut s = st(Mood::Neutral);
        s.head_turn = -1.0;
        s.squash = 0.2;
        push("shape-ell-flower-turn-squash", l, s);
    }

    // -- HEAD SHADE (shape-shd-): the "gradient" class. This
    // painter has NO gradient prim; the shading surface is the
    // head_shade prim (even-odd coverage product, INVB §3.5).
    // Full grid: squash {-0.2, 0, 0.2} x head_turn {-1, 0, 1} x
    // cam_pitch {-1, 0, 1} — squash rescales both ellipses of
    // the product, turn translates the offset circle, cam_pitch
    // rescales rp (1 + 0.04*pitch).
    for (sq_tag, squash) in [("neg", -0.2f32), ("zero", 0.0), ("pos", 0.2)] {
        for (tn_tag, turn) in [("neg", -1.0f32), ("zero", 0.0), ("pos", 1.0)] {
            for (cp_tag, pitch) in [("neg", -1.0f32), ("zero", 0.0), ("pos", 1.0)] {
                let mut s = st(Mood::Neutral);
                s.squash = squash;
                s.head_turn = turn;
                s.cam_pitch = pitch;
                push(
                    &format!("shape-shd-sq{sq_tag}-tn{tn_tag}-cp{cp_tag}"),
                    base_look(),
                    s,
                );
            }
        }
    }

    // -- MASKED PATTERNS (shape-msk-): fill_polygon_masked /
    // fill_ellipse_masked over the bean polygon_mask, at lean
    // -20 / +20 so the binary mask edge sweeps the bean (INVB
    // §3.2, §9: the mask edge is the parity surface).
    for (tag, pattern, pat_col) in [
        ("stripes", "stripes", None),
        ("apron", "apron", None),
        ("vest", "vest", Some("#33304A")),
    ] {
        for (lean_tag, lean) in [("neg20", -20.0f32), ("pos20", 20.0)] {
            let mut l = base_look();
            l.pattern = pattern.to_string();
            l.pat_col = pat_col.map(|s| s.to_string());
            let mut s = st(Mood::Neutral);
            s.lean = lean;
            push(&format!("shape-msk-{tag}-lean-{lean_tag}"), l, s);
        }
    }

    // -- MASKED MOUTHS (shape-mou-): the open_mouth closure
    // (INVB §4.5) + the Laugh tongue.
    // Shock: (w16, h 20+6*talk, tongue) at talk 0.
    push("shape-mou-shock", base_look(), st(Mood::Shock));
    // Talk sweep on Determined: h = 6+18*talk crosses the
    // tongue gate (h > 12) between 0.3 and 1.0; 0.06 sits just
    // above the talk > 0.05 dispatch threshold.
    for (tag, talk) in [("006", 0.06f32), ("030", 0.3), ("100", 1.0)] {
        let mut s = st(Mood::Determined);
        s.talk = talk;
        push(&format!("shape-mou-talk-{tag}"), base_look(), s);
    }
    // Awe: (w11, h13, tongue:false) — h=13 passes the h>12 test,
    // so the flag alone suppresses the tongue.
    push("shape-mou-awe", base_look(), st(Mood::Awe));
    // Laugh: tongue fill_ellipse_masked over the mouth polygon
    // mask + masked_blend teeth strip.
    push("shape-mou-laugh-tongue", base_look(), st(Mood::Laugh));

    // -- PLATES + ANCHOR (shape-plt-): composite_plate at
    // anchor-engaging states (INVB §6). Held pose = pose_explain
    // arms (film.rs:1459) unless the state family overrides them.
    let explain = |s: &mut FullState| {
        s.arm_r = (68.0, -42.0);
        s.arm_l = (28.0, -24.0);
    };
    for (hand_tag, lean_tag, lean, label, left) in [
        ("holdr-label", "pos40", 40.0f32, true, false),
        ("holdr-label", "neg40", -40.0, true, false),
        ("holdl-plain", "pos40", 40.0, false, true),
        ("holdl-plain", "neg40", -40.0, false, true),
    ] {
        let mut s = st(Mood::Happy);
        explain(&mut s);
        s.lean = lean;
        if left {
            s.hold_l = Some(parcel_plate(label));
        } else {
            s.hold_r = Some(parcel_plate(label));
        }
        push(
            &format!("shape-plt-{hand_tag}-lean-{lean_tag}"),
            look_finn(),
            s,
        );
    }
    for (tag, lean, r_label) in [("pos55", 55.0f32, true), ("neg55", -55.0, false)] {
        // Both hands: one parcel variant per hand, swapped
        // between the two lean signs so both (hand, variant)
        // pairings appear at both signs.
        let mut s = st(Mood::Happy);
        explain(&mut s);
        s.lean = lean;
        s.hold_r = Some(parcel_plate(r_label));
        s.hold_l = Some(parcel_plate(!r_label));
        push(&format!("shape-plt-both-lean-{tag}"), look_finn(), s);
    }
    {
        // The lying-lean anchor family (anchor-lying-lean82 in
        // the Phase-0 spread) extended with a held plate.
        let mut s = st(Mood::Dizzy);
        s.arm_l = (150.0, 20.0);
        s.arm_r = (138.0, -30.0);
        s.head_tilt = 4.0;
        s.lean = 82.0;
        s.hold_r = Some(parcel_plate(false));
        push("shape-plt-lying82-holdr", look_finn(), s);
    }
    {
        // Mirror lying lean, plate in the other hand.
        let mut s = st(Mood::Dizzy);
        s.arm_l = (138.0, -30.0);
        s.arm_r = (150.0, 20.0);
        s.head_tilt = -4.0;
        s.lean = -82.0;
        s.hold_l = Some(parcel_plate(false));
        push("shape-plt-lying-neg82-holdl", look_finn(), s);
    }
    for (tag, left, label) in [("holdr-label", false, true), ("holdl-plain", true, false)] {
        // High seat + plate: the anchor-high-sit family
        // (sit_drop 900) with a held parcel.
        let mut s = st(Mood::Neutral);
        s.legs = Legs::Sit;
        s.sit_drop = Some(900.0);
        if left {
            s.hold_l = Some(parcel_plate(label));
        } else {
            s.hold_r = Some(parcel_plate(label));
        }
        push(&format!("shape-plt-sit900-{tag}"), look_finn(), s);
    }

    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toonfull::canvas_anchor;

    #[test]
    fn shape_spread_shape() {
        let v = shape_spread();
        // 10 fills + 12 ellipses + 27 shade grid + 6 patterns
        // + 6 mouths + 10 plates = 71.
        assert_eq!(v.len(), 71, "shape battery size changed");
        let mut names: Vec<&str> = v.iter().map(|(n, _, _)| n.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), v.len(), "entry names must be unique");
        for (prefix, count) in [
            ("shape-fill-", 10usize),
            ("shape-ell-", 12),
            ("shape-shd-", 27),
            ("shape-msk-", 6),
            ("shape-mou-", 6),
            ("shape-plt-", 10),
        ] {
            assert_eq!(
                v.iter().filter(|(n, _, _)| n.starts_with(prefix)).count(),
                count,
                "family {prefix}"
            );
        }
    }

    // -----------------------------------------------------------------
    // Local-run recorders (ignored by default): mirror
    // gpupaint.rs::record_states exactly (gputrace window +
    // paint_toon_full + PNG/JSON write) but restricted to one
    // shape family, so this battery can be recorded without
    // repainting the whole Phase-0 spread. Output dir from the
    // SHAPE_OUT env var. Run e.g.:
    //   SHAPE_OUT=<dir> cargo test --release --features gpupaint \
    //     record_shape_fill -- --ignored --nocapture
    // -----------------------------------------------------------------
    fn record_family(prefix: &str) {
        let out = std::env::var("SHAPE_OUT").expect("SHAPE_OUT not set");
        std::fs::create_dir_all(&out).expect("create SHAPE_OUT");
        crate::gputrace::set_armed(true);
        let mut n = 0;
        for (name, look, state) in shape_spread().iter().filter(|(n, _, _)| n.starts_with(prefix)) {
            crate::gputrace::begin(2560, 2816);
            let (px, w, h, _, _) = crate::toonfull::paint_toon_full(look, state);
            let list = crate::gputrace::take();
            let img = image::RgbaImage::from_raw(w, h, px).expect("pixel buffer size mismatch");
            img.save(format!("{out}/{name}.png")).expect("save png");
            std::fs::write(format!("{out}/{name}.json"), list.to_json()).expect("write json");
            println!("recorded {name} ({} prims)", list.prims.len());
            n += 1;
        }
        crate::gputrace::set_armed(false);
        assert!(n > 0, "no states matched {prefix}");
    }

    #[test]
    #[ignore]
    fn record_shape_fill() {
        record_family("shape-fill-");
    }
    #[test]
    #[ignore]
    fn record_shape_ell() {
        record_family("shape-ell-");
    }
    #[test]
    #[ignore]
    fn record_shape_shd() {
        record_family("shape-shd-");
    }
    #[test]
    #[ignore]
    fn record_shape_msk() {
        record_family("shape-msk-");
    }
    #[test]
    #[ignore]
    fn record_shape_mou() {
        record_family("shape-mou-");
    }
    #[test]
    #[ignore]
    fn record_shape_plt() {
        record_family("shape-plt-");
    }

    #[test]
    fn plate_states_anchor_actually_engages() {
        // The anchor is host-side geometry: a plates battery at
        // non-engaging states would prove nothing about the
        // anchor+plate composition. Every shape-plt-* state must
        // engage it (mirrors anchor_entries_actually_engage).
        let v = shape_spread();
        for (name, look, state) in v.iter().filter(|(n, _, _)| n.starts_with("shape-plt-")) {
            let a = canvas_anchor(look, state);
            println!("ANCHOR {name}: ({:.2}, {:.2})", a.0, a.1);
            assert!(a != (0.0, 0.0), "{name}: anchor must engage, got {a:?}");
        }
        // And the battery's neutral standing states must NOT
        // engage (the anchor is zero for fitting states).
        let (_, look, state) =
            v.iter().find(|(n, _, _)| n == "shape-fill-bean-lean-neg30").expect("fill");
        let _ = (look, state); // lean -30 may engage; checked below by name instead
        let (_, look, state) =
            v.iter().find(|(n, _, _)| n == "shape-shd-sqzero-tnzero-cpzero").expect("shd");
        assert_eq!(canvas_anchor(look, state), (0.0, 0.0));
    }
}
