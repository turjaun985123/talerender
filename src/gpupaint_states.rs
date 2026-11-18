//! GPUFIRST Phase 0 — STATES seat: the real-film-state parity spread.
//!
//! `state_spread()` returns the states the PaintCache actually
//! serves — film-staged (look, state) pairs and painter-reachable
//! combinations — NOT battery primitives and NOT random fuzz.
//! The GPU painter (TRACE seat's `gpupaint` harness) is measured
//! against `toonfull::paint_toon_full` (CPU) on exactly this list;
//! the CPU output is the reference. See
//! docs/integration/GPUFIRST-STATES.md for the spread definition,
//! provenance per entry, metric definitions and run commands.
//!
//! Wiring (as landed with the TRACE skeleton): 
//!   Cargo.toml:  [features] gpupaint = []
//!   main.rs:     #[cfg(feature = "gpupaint")] mod gpupaint_states;
//!   gpupaint.rs: `gpupaint::state_spread()` delegates to
//!                `crate::gpupaint_states::state_spread()` here,
//! so the chartered entry point `gpupaint::state_spread()` IS
//! this spread. Everything here is pure CPU data construction:
//! no wgpu types, no I/O, no env. `FullLook` / `FullState` are
//! `pub(crate)` in toonfull.rs and are constructed literally,
//! field by field.
//!
//! LAWS honoured by construction:
//! * Every value is film-staged (cited to film.rs / the scene
//!   files) or reachable through the scene-format loader
//!   (scenefile.rs character node: free `pattern` / `hat` /
//!   `neck` strings, `extra` list, `kid`, and f32 state fields).
//! * sweat / tears entries are SYNTHETIC-BUT-REACHABLE: the film
//!   path never sets `FullState.sweat` / `tears` (no staging site
//!   assigns them; the painter fires sweat on `sweat || worried`
//!   and tears on `tears || sad`, toonfull.rs), which is exactly
//!   why they need explicit coverage — the state_key precedent
//!   (film.rs `state_key` + its regression tests) already proved
//!   sweat-only / tears-only differences are real paints that a
//!   key once collapsed. They are separate entries here for the
//!   same reason.
//! * Names are stable and prefixed by class so per-class
//!   roll-ups are a prefix match (see `state_group` /
//!   `rollup_class`).

use crate::scene::paint_parcel;
use crate::toonfull::{FullLook, FullState, Legs, Mood, Plate};

// ---------------------------------------------------------------------
// Look constructors. `FullLook` has no Clone derive in toonfull.rs,
// so every helper builds a fresh value.
// ---------------------------------------------------------------------

/// The scene-format default look (scenefile.rs character node:
/// body `#E8734A`, head `#F7F7FA`, pattern plain, hat/neck none,
/// no extras, not kid). Base for the mood / face / fx sweeps so
/// face features are measured unobstructed by hats or extras.
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

/// film.rs `fl()` (film.rs:572) transcribed: the registry's look
/// factory — head always `#F7F7FA`, shade/pattern/neck/extras at
/// their defaults unless the registry entry mutates them.
fn fl(body: &str, hat: &str, hat_col: Option<&str>) -> FullLook {
    FullLook {
        body: body.to_string(),
        head: "#F7F7FA".to_string(),
        shade: None,
        pattern: "plain".to_string(),
        pat_col: None,
        hat: hat.to_string(),
        hat_col: hat_col.map(|s| s.to_string()),
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

/// Registry idx 0 / 10 — jgOnoda, the hero (film.rs:603-607):
/// cap + explicit shade. Staged Determined in s01.
fn look_onoda_jg() -> FullLook {
    let mut l = fl("#5C6644", "cap", Some("#4C5638"));
    l.shade = Some("#878E6E".to_string());
    l
}
/// Registry idx 5 — shOnoda (film.rs:608): same cap look, NO
/// shade override (the only FullLook difference from jgOnoda).
fn look_onoda_sh() -> FullLook {
    fl("#5C6644", "cap", Some("#4C5638"))
}
/// Registry idx 1 / 6 — jgCompA / shCompA (film.rs:609).
fn look_comp_a() -> FullLook {
    fl("#68744E", "cap", Some("#525C3C"))
}
/// Registry idx 2 / 7 — CompB (film.rs:610-615): cap + scarf.
fn look_comp_b() -> FullLook {
    let mut l = fl("#59613F", "cap", Some("#59613F"));
    l.neck = "scarf".to_string();
    l.neck_col = Some("#8A7A52".to_string());
    l
}
/// Registry idx 3 — Suzuki (film.rs:616-620): cap + glasses.
fn look_suzuki() -> FullLook {
    let mut l = fl("#C96A42", "cap", Some("#E8B83A"));
    l.glasses = true;
    l
}
/// Registry idx 4 / 8 / 9 — Tani (film.rs:621-631): bowler +
/// vest + tie + glasses + mustache.
fn look_tani() -> FullLook {
    let mut l = fl("#4A4560", "bowler", Some("#2E2A3A"));
    l.pattern = "vest".to_string();
    l.pat_col = Some("#33304A".to_string());
    l.neck = "tie".to_string();
    l.neck_col = Some("#7A2E3A".to_string());
    l.glasses = true;
    l.mustache = true;
    l.mustache_col = Some("#C9C9D2".to_string());
    l
}
/// ex07's Finn (toonfull.rs canvas_anchor_tests::finn_look,
/// toonfull.rs:1690): plain, hatless — the B-1 anchor case look.
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

/// A held parcel plate from scene.rs `paint_parcel` (scene.rs:
/// 2550): 256x256 px = 128x128 stage units at res 2, centred on
/// the hand point (x0 = y0 = -64), the centred-hold framing the
/// film's coconut plate approximates (film/gen_assets.py:
/// 200x200 units at origin (-100, -108)). `label = true` adds
/// the label paper + stamp rects (the text-bearing variant —
/// glyph text proper enters production plates via gen_assets.py;
/// the label rects are what this painter draws).
fn parcel_plate(label: bool) -> Plate {
    let (px, w, h) = paint_parcel(label);
    Plate { px, w, h, x0: -64.0, y0: -64.0 }
}

fn st(mood: Mood) -> FullState {
    FullState { mood, ..FullState::default() }
}

// ---------------------------------------------------------------------
// The spread.
// ---------------------------------------------------------------------

/// The parity spread: (name, look, state). ~76 entries; see
/// GPUFIRST-STATES.md for the per-entry provenance table.
pub(crate) fn state_spread() -> Vec<(String, FullLook, FullState)> {
    let mut v: Vec<(String, FullLook, FullState)> = Vec::new();
    let mut push = |name: &str, look: FullLook, state: FullState| {
        v.push((name.to_string(), look, state));
    };

    // -- Class: mood features — all 19 moods at the neutral
    // standing state (FullState::default with only mood set) on
    // the base look. Mood names are Mood::from_str's spellings.
    for (name, mood) in [
        ("neutral", Mood::Neutral),
        ("smug", Mood::Smug),
        ("happy", Mood::Happy),
        ("laugh", Mood::Laugh),
        ("shock", Mood::Shock),
        ("worried", Mood::Worried),
        ("angry", Mood::Angry),
        ("sad", Mood::Sad),
        ("sly", Mood::Sly),
        ("proud", Mood::Proud),
        ("tired", Mood::Tired),
        ("dizzy", Mood::Dizzy),
        ("determined", Mood::Determined),
        ("embarrassed", Mood::Embarrassed),
        ("awe", Mood::Awe),
        ("suspicious", Mood::Suspicious),
        ("relieved", Mood::Relieved),
        ("puzzled", Mood::Puzzled),
        ("deadpan", Mood::Deadpan),
    ] {
        push(&format!("mood-{name}"), base_look(), st(mood));
    }

    // -- Class: film looks — the registry's six distinct
    // FullLooks (film.rs look_registry, film.rs:598), each with
    // a state its owner is actually staged in.
    {
        // s01 hero (film.rs s01): Determined, rifle arms, lean 1,
        // headTurn track start -0.45.
        let mut s = st(Mood::Determined);
        s.arm_r = (16.0, -6.0);
        s.arm_l = (10.0, 6.0);
        s.lean = 1.0;
        s.head_turn = -0.45;
        push("film-onoda-jg-s01", look_onoda_jg(), s);
    }
    {
        // s02 shore walk (film.rs s02): Determined, walk legs.
        let mut s = st(Mood::Determined);
        s.arm_r = (16.0, -6.0);
        s.arm_l = (10.0, 6.0);
        s.legs = Legs::Walk;
        s.phase = 1.5707964; // PI/2
        s.bob = 0.0; // walk_bob(PI/2, 5) = 5*(1-|sin|) = 0
        push("film-onoda-sh-s02-walk", look_onoda_sh(), s);
    }
    {
        // s04 celebrate (film.rs pose_celebrate, film.rs:1465):
        // Laugh, both arms up, bob -8.
        let mut s = st(Mood::Laugh);
        s.arm_l = (150.0, 14.0);
        s.arm_r = (150.0, 14.0);
        s.bob = -8.0;
        push("film-comp-a-celebrate", look_comp_a(), s);
    }
    {
        // s04: CompB celebrates at Happy (film.rs s04 overrides
        // pose_celebrate's Laugh to Happy for CompB).
        let mut s = st(Mood::Happy);
        s.arm_l = (150.0, 14.0);
        s.arm_r = (150.0, 14.0);
        s.bob = -8.0;
        push("film-comp-b-celebrate", look_comp_b(), s);
    }
    {
        // s08b (film.rs s08b): pose_listen (film.rs:1479) +
        // Worried — sweat fires via the mood in this entry.
        let mut s = st(Mood::Worried);
        s.arm_r = (118.0, -105.0);
        s.arm_l = (10.0, 8.0);
        s.lean = 3.0;
        s.head_tilt = -5.0;
        s.head_turn = -0.2;
        push("film-suzuki-listen-worried", look_suzuki(), s);
    }
    {
        // s03 (film.rs s03): pose_explain (film.rs:1459) then
        // mood overridden to Determined, arm_r tracked to 68.
        let mut s = st(Mood::Determined);
        s.arm_r = (68.0, -42.0);
        s.arm_l = (28.0, -24.0);
        s.look = [0.3, 0.0];
        push("film-tani-explain", look_tani(), s);
    }

    // -- Class: limbs — legs on the shore Onoda look, s02's
    // staged arms. Walk phases sample the painter's sin/cos
    // cycle at quadrants; bob follows film's walk_bob law
    // (amp*(1-|sin phase|), amp 5 as staged in s02's gait_eval).
    for (tag, phase, bob) in [
        ("p0", 0.0f32, 5.0f32),
        ("p1", 1.5707964, 0.0),
        ("p2", 3.1415927, 5.0),
        ("p3", 4.712389, 0.0),
    ] {
        let mut s = st(Mood::Determined);
        s.arm_r = (16.0, -6.0);
        s.arm_l = (10.0, 6.0);
        s.legs = Legs::Walk;
        s.phase = phase;
        s.bob = bob;
        push(&format!("legs-walk-{tag}"), look_onoda_sh(), s);
    }
    for (tag, phase) in [("p0", 0.0f32), ("p1", 3.1415927f32)] {
        // Run is painter/scene-format reachable (scenefile legs
        // "run"); the film itself never stages it — recorded
        // here so the branch is not silently unmeasured.
        let mut s = st(Mood::Determined);
        s.arm_r = (16.0, -6.0);
        s.arm_l = (10.0, 6.0);
        s.legs = Legs::Run;
        s.phase = phase;
        push(&format!("legs-run-{tag}"), look_onoda_sh(), s);
    }
    {
        // s06 sit (film.rs:1839-1841): pose_mourn (film.rs:1471)
        // + Sit + the film's exact sit_drop expression + the
        // s06 arm overrides.
        let mut s = st(Mood::Sad);
        s.arm_r = (16.0, -6.0);
        s.arm_l = (8.0, 5.0);
        s.lean = 5.0;
        s.head_tilt = -4.0;
        s.bob = 4.0;
        s.legs = Legs::Sit;
        s.sit_drop = Some((0.34 * 1024.0 / (1.75 / 0.96)) as f32);
        push("legs-sit-film", look_onoda_jg(), s);
    }
    {
        // Jump: scene-format reachable (legs "jump"); the film
        // never stages it. Default arms, painter's -60 bob lift
        // is internal to the branch.
        let mut s = st(Mood::Happy);
        s.legs = Legs::Jump;
        push("legs-jump", look_onoda_sh(), s);
    }

    // -- Class: mood features (face dynamics) — talk sweep on
    // the film's talking configuration (Tani in s03/s11 talks
    // while Determined; values are the task's sweep points,
    // inside talk()'s staged range 0.25..~1.4 clamped by the
    // painter's own use).
    for (tag, talk) in [("025", 0.25f32), ("050", 0.5), ("075", 0.75), ("100", 1.0)] {
        let mut s = st(Mood::Determined);
        s.talk = talk;
        push(&format!("face-talk-{tag}"), base_look(), s);
    }
    // Blink: 1.0 is the film's blink value (blink_amount /
    // blink_at are 0/1); 0.5 exercises the painter's partial
    // branch (blink <= 0.6: pupil height *(1-blink*0.6)) vs
    // 1.0's closed-eye stroke branch (blink > 0.6). 0.5 is
    // reachable via the scene-format blink f32 field.
    for (tag, blink) in [("050", 0.5f32), ("100", 1.0)] {
        let mut s = st(Mood::Neutral);
        s.blink = blink;
        push(&format!("face-blink-{tag}"), base_look(), s);
    }
    // Look offsets (task-specified extremes; the painter scales
    // them by 9/8 px with no clamp, scenefile pair field).
    for (tag, look) in [("r", [1.0f32, 0.0]), ("lu", [-1.0, 0.5])] {
        let mut s = st(Mood::Neutral);
        s.look = look;
        push(&format!("face-look-{tag}"), base_look(), s);
    }

    // -- Class: mood features (fx) — sweat/tears. Synthetic-but-
    // reachable (see header law): the film path never assigns
    // these fields. Separate entries per the state_key
    // precedent; the mood-fired pair proves the OR-branches.
    {
        let mut s = st(Mood::Neutral);
        s.sweat = Some(1.0);
        push("fx-sweat-only", base_look(), s);
    }
    {
        let mut s = st(Mood::Neutral);
        s.tears = true;
        push("fx-tears-only", base_look(), s);
    }
    {
        let mut s = st(Mood::Neutral);
        s.sweat = Some(1.0);
        s.tears = true;
        push("fx-sweat-tears", base_look(), s);
    }
    {
        // Spec 5e gradation: alpha = min(1, k) must reach the GPU
        // prims at fractional strengths (see sweat_tests in
        // toonfull.rs for the 16-class CPU partition).
        let mut s = st(Mood::Neutral);
        s.sweat = Some(0.5);
        push("fx-sweat-050", base_look(), s);
    }
    {
        let mut s = st(Mood::Neutral);
        s.sweat = Some(0.99);
        push("fx-sweat-099", base_look(), s);
    }
    push("fx-worried-sweat-via-mood", base_look(), st(Mood::Worried));
    push("fx-sad-tears-via-mood", base_look(), st(Mood::Sad));

    // -- Class: plates — held parcels (scene.rs paint_parcel).
    {
        // pose_explain arms (film.rs:1459) raise the right hand.
        let mut s = st(Mood::Happy);
        s.arm_r = (68.0, -42.0);
        s.arm_l = (28.0, -24.0);
        s.look = [0.3, 0.0];
        s.hold_r = Some(parcel_plate(true));
        push("plate-holdr-parcel-label", base_look(), s);
    }
    {
        let mut s = st(Mood::Happy);
        s.arm_r = (68.0, -42.0);
        s.arm_l = (28.0, -24.0);
        s.look = [0.3, 0.0];
        s.hold_r = Some(parcel_plate(false));
        push("plate-holdr-parcel-plain", base_look(), s);
    }
    {
        // s05b pre-drop leaflet hold (film.rs s05b): Suspicious,
        // arm_l tracked pose (58, -24), look [0.1, 0.3] — here
        // holding the parcel in the left hand.
        let mut s = st(Mood::Suspicious);
        s.arm_l = (58.0, -24.0);
        s.arm_r = (16.0, -6.0);
        s.look = [0.1, 0.3];
        s.hold_l = Some(parcel_plate(false));
        push("plate-holdl-parcel", look_onoda_jg(), s);
    }

    // -- Class: anchor — canvas_anchor engagement (B-1 states,
    // transcribed from toonfull.rs canvas_anchor_tests).
    {
        // The lying lean (toonfull.rs:1726): Dizzy, lean 82.
        let mut s = st(Mood::Dizzy);
        s.arm_l = (150.0, 20.0);
        s.arm_r = (138.0, -30.0);
        s.head_tilt = 4.0;
        s.lean = 82.0;
        push("anchor-lying-lean82", look_finn(), s);
    }
    {
        // The high seat (toonfull.rs:1751): Sit, sit_drop 900.
        let mut s = st(Mood::Neutral);
        s.legs = Legs::Sit;
        s.sit_drop = Some(900.0);
        push("anchor-high-sit", look_finn(), s);
    }

    // -- Class: patterns — isolated on the base look (vest is
    // also covered in situ by film-tani-explain).
    for (tag, pattern, pat_col) in [
        ("stripes", "stripes", None),
        ("apron", "apron", None),
        ("vest", "vest", Some("#33304A")),
    ] {
        let mut l = base_look();
        l.pattern = pattern.to_string();
        l.pat_col = pat_col.map(|s| s.to_string());
        push(&format!("pattern-{tag}"), l, st(Mood::Neutral));
    }

    // -- Class: hats — cap in situ in every film entry; these
    // isolate each hat branch on the base look. hat_col None
    // exercises the painter's per-hat default colours.
    for (tag, hat) in [
        ("cap", "cap"),
        ("tophat", "tophat"),
        ("bun", "bun"),
        ("bow", "bow"),
        ("beanie", "beanie"),
        ("mailcap", "mailcap"),
    ] {
        let mut l = base_look();
        l.hat = hat.to_string();
        if hat == "cap" {
            l.hat_col = Some("#4C5638".to_string()); // hero cap green
        }
        push(&format!("hat-{tag}"), l, st(Mood::Neutral));
    }

    // -- Class: neck — tie/scarf also in situ (Tani / CompB);
    // collar exists only as a painter branch. neck_col None
    // exercises the painter defaults (#D8343F; collar ignores
    // neck_col by design, toonfull.rs collar branch).
    for (tag, neck) in [("tie", "tie"), ("scarf", "scarf"), ("collar", "collar")] {
        let mut l = base_look();
        l.neck = neck.to_string();
        push(&format!("neck-{tag}"), l, st(Mood::Neutral));
    }

    // -- Class: extras — one flag each on the base look
    // (glasses/mustache also in situ on Suzuki/Tani; blush /
    // freckles / pencil / monocle are the CR-B3-06 ports,
    // reachable via the scene-format `extra` list).
    {
        let mut l = base_look();
        l.glasses = true;
        push("extra-glasses", l, st(Mood::Neutral));
    }
    {
        let mut l = base_look();
        l.mustache = true;
        push("extra-mustache", l, st(Mood::Neutral));
    }
    {
        let mut l = base_look();
        l.blush = true;
        push("extra-blush", l, st(Mood::Neutral));
    }
    {
        let mut l = base_look();
        l.freckles = true;
        push("extra-freckles", l, st(Mood::Neutral));
    }
    {
        let mut l = base_look();
        l.pencil = true;
        push("extra-pencil", l, st(Mood::Neutral));
    }
    {
        let mut l = base_look();
        l.monocle = true;
        push("extra-monocle", l, st(Mood::Neutral));
    }

    // -- Class: transforms / proportions.
    {
        let mut l = base_look();
        l.kid = true; // painter by += 190 (toonfull.rs)
        push("xf-kid", l, st(Mood::Happy));
    }
    {
        // Task-specified squash extreme; scene-format reachable
        // (squash is a free f32 state field). Film-staged squash
        // is only the -0.04 s06 settle — recorded in the doc.
        let mut s = st(Mood::Happy);
        s.squash = 0.5;
        push("xf-squash-050", base_look(), s);
    }
    {
        // pose_listen's tilt extreme (film.rs:1479).
        let mut s = st(Mood::Neutral);
        s.head_tilt = -5.0;
        push("xf-headtilt-neg5", base_look(), s);
    }
    {
        // pose_think's tilt extreme (film.rs:1486).
        let mut s = st(Mood::Neutral);
        s.head_tilt = 6.0;
        s.head_turn = 0.3;
        s.look = [0.4, -0.3];
        push("xf-headtilt-pos6-think", base_look(), s);
    }
    {
        // s04 headTurn track start (film.rs s04): +0.5, the
        // film's positive extreme (negative extreme -0.45 is
        // film-onoda-jg-s01).
        let mut s = st(Mood::Suspicious);
        s.head_turn = 0.5;
        push("xf-headturn-pos05", look_onoda_jg(), s);
    }
    {
        // cam_pitch != 0: the S01 perspective value staged in
        // scenes/character-v2.scene.json (camPitch 0.28747,
        // film t=1.50; film.rs finish_chars / cam_pitch_of is
        // the producing formula).
        let mut s = st(Mood::Determined);
        s.arm_r = (16.0, -6.0);
        s.arm_l = (10.0, 6.0);
        s.lean = 1.0;
        s.cam_pitch = 0.28747;
        push("xf-campitch-s01", look_onoda_jg(), s);
    }

    v.extend(crate::gpupaint_states_brow::brow_spread());
    v.extend(crate::gpupaint_states_shapes::shape_spread());
    v.extend(crate::gpupaint_states_strokes::stroke_spread());
    v.extend(crate::gpupaint_states_earshair::earshair_spread());
    v
}

/// Fine group of a spread entry: the name up to the first '-'
/// (mood / film / legs / face / fx / plate / anchor / pattern /
/// hat / neck / extra / xf).
pub(crate) fn state_group(name: &str) -> &str {
    name.split('-').next().unwrap_or(name)
}

/// Per-class roll-up used by the parity report (the classes the
/// Phase-0 brief names, plus film-looks and transforms):
/// mood-features (mood-* + face-* + fx-*), limbs (legs-*),
/// plates (plate-*), anchor (anchor-*), patterns-and-looks
/// (pattern-* + hat-* + neck-* + extra-*), film-looks (film-*),
/// transforms (xf-*).
pub(crate) fn rollup_class(name: &str) -> &'static str {
    // Batch 2 class batteries: the shape battery encodes its class
    // in the second name segment; read those out by prefix before
    // the group match (state_group would return "shape" for all).
    if name.starts_with("shape-fill-") {
        return "fills";
    }
    if name.starts_with("shape-ell-") {
        return "ellipses-arcs";
    }
    if name.starts_with("shape-shd-") || name.starts_with("shape-msk-") || name.starts_with("shape-mou-") {
        return "shade+masks";
    }
    if name.starts_with("shape-plt-") {
        return "plates+anchor";
    }
    match state_group(name) {
        "stroke" => "strokes",
        "brow" => "brow-offsets",
        "ears" | "hair" => "ears+hair",
        "mood" | "face" | "fx" => "mood-features",
        "legs" => "limbs",
        "plate" => "plates",
        "anchor" => "anchor",
        "pattern" | "hat" | "neck" | "extra" => "patterns-and-looks",
        "film" => "film-looks",
        _ => "transforms",
    }
}

// ---------------------------------------------------------------------
// Parity metrics (shared by the GPU harness and the CPU-vs-CPU
// dry run). Reference = CPU paint_toon_full output.
// ---------------------------------------------------------------------

/// One state's parity result. PSNR fields are f64::INFINITY
/// when their MSE is 0.
#[derive(Clone, Debug)]
pub(crate) struct Parity {
    pub identical: bool,
    /// Pixels (of w*h) differing in ANY channel.
    pub diff_px: u64,
    /// Max |delta| per channel [R, G, B, A] (the headline
    /// "max per-channel |Δ|" of the coordinator ruling).
    pub max_delta_rgba: [u8; 4],
    /// Max per-channel |delta| over RGBA (max of the above).
    pub max_delta: u8,
    /// PSNR over RGBA: 10*log10(255^2 / MSE), MSE over all
    /// 4*w*h channel samples (the house film formula —
    /// docs/ab/film/README.md — extended from RGB to RGBA).
    /// Labelled "RGBA PSNR" wherever reported.
    pub psnr_db: f64,
    /// Alpha-only PSNR: 10*log10(255^2 / MSE_alpha), MSE over
    /// the alpha channel only (BATT's headline metric — the
    /// M4 battery's 47.9 dB was alpha PSNR). Labelled
    /// "alpha PSNR" wherever reported. Both PSNRs are always
    /// reported together (coordinator ruling).
    pub psnr_alpha_db: f64,
    /// Localization split of the DIFFERING pixels (see
    /// compare_paints for the definitions).
    pub edge: u64,
    pub fringe: u64,
    pub interior: u64,
}

/// Compare two same-size RGBA canvases.
///
/// Localization — CANONICAL definitions (coordinator ruling,
/// aligning STATES with BATT, docs/integration/GPUFIRST-BATT.md,
/// and the independent AUDIT checker): the boundary set is the
/// fractional-alpha pixels of the CPU REFERENCE only,
///   B = { p : 0 < A_ref(p) < 255 }.
/// A differing pixel at Chebyshev distance d from B is
/// * `edge`     — d <= 1,
/// * `fringe`   — d == 2,
/// * `interior` — d > 2 (including "no boundary exists").
/// The boundary is NOT dilated before measuring: a differing
/// pixel 2 px from a fractional pixel in an axial direction is
/// FRINGE, never EDGE. The three classes partition the
/// differing pixels exactly (edge + fringe + interior ==
/// diff_px). Note the specified consequence: ink fully inside
/// an opaque region (e.g. mouth interior) has no alpha
/// boundary nearby and classifies INTERIOR.
pub(crate) fn compare_paints(reference: &[u8], test: &[u8], w: u32, h: u32) -> Parity {
    assert_eq!(reference.len(), test.len(), "canvas size mismatch");
    assert_eq!(reference.len(), (w as usize) * (h as usize) * 4, "canvas dims mismatch");
    let (w, h) = (w as i64, h as i64);
    let is_boundary = |x: i64, y: i64| -> bool {
        if x < 0 || y < 0 || x >= w || y >= h {
            false
        } else {
            let a = reference[((y * w + x) as usize) * 4 + 3];
            a > 0 && a < 255
        }
    };
    // Chebyshev distance from (x, y) to B, capped at 3: scan the
    // radius-1 ring, then the radius-2 ring.
    let chebyshev_to_boundary = |x: i64, y: i64| -> i64 {
        for r in 1..=2i64 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) == r && is_boundary(x + dx, y + dy) {
                        return r;
                    }
                }
            }
        }
        3 // > 2 (also the no-boundary-anywhere case)
    };
    let mut diff_px = 0u64;
    let mut max_delta_rgba = [0u8; 4];
    let mut se = 0f64; // sum of squared per-channel deltas (RGBA)
    let mut se_alpha = 0f64; // alpha channel only
    let (mut edge, mut fringe, mut interior) = (0u64, 0u64, 0u64);
    for y in 0..h {
        for x in 0..w {
            let k = ((y * w + x) as usize) * 4;
            let mut differs = false;
            for c in 0..4 {
                let d = (reference[k + c] as i32 - test[k + c] as i32).abs();
                if d > 0 {
                    differs = true;
                    se += (d * d) as f64;
                    if c == 3 {
                        se_alpha += (d * d) as f64;
                    }
                    if d as u8 > max_delta_rgba[c] {
                        max_delta_rgba[c] = d as u8;
                    }
                }
            }
            if !differs {
                continue;
            }
            diff_px += 1;
            let d = if is_boundary(x, y) { 0 } else { chebyshev_to_boundary(x, y) };
            if d <= 1 {
                edge += 1;
            } else if d == 2 {
                fringe += 1;
            } else {
                interior += 1;
            }
        }
    }
    debug_assert_eq!(edge + fringe + interior, diff_px);
    let identical = reference == test;
    let psnr = |se: f64, samples: f64| -> f64 {
        if se == 0.0 {
            f64::INFINITY
        } else {
            10.0 * (255.0f64 * 255.0 / (se / samples)).log10()
        }
    };
    let psnr_db = psnr(se, reference.len() as f64);
    let psnr_alpha_db = psnr(se_alpha, (w * h) as f64);
    let max_delta = *max_delta_rgba.iter().max().unwrap_or(&0);
    Parity { identical, diff_px, max_delta_rgba, max_delta, psnr_db, psnr_alpha_db, edge, fringe, interior }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toonfull::{canvas_anchor, paint_toon_full};

    #[test]
    fn spread_shape() {
        let v = state_spread();
        assert!((60..=245).contains(&v.len()), "spread has {} entries", v.len()); // Batch 2: bound raised by the coordinator at integration (batteries: +2 sweat, +5 brow, +71 shape, +61 stroke over the 75 base). Batch 3 EARSHAIR: +11 (6 ears + 5 hair, gpupaint_states_earshair) over the 221 Batch-2 total -> 232; bound raised to 245 for this seat's additions only.
        let mut names: Vec<&str> = v.iter().map(|(n, _, _)| n.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), v.len(), "entry names must be unique");
        // Every brief-mandated family is present.
        for prefix in [
            "mood-", "film-", "legs-walk-", "legs-run-", "legs-sit-", "legs-jump", "face-talk-",
            "face-blink-", "face-look-", "fx-sweat-only", "fx-tears-only", "fx-sweat-tears",
            "plate-holdr-parcel-label", "plate-holdr-parcel-plain", "plate-holdl-parcel",
            "anchor-", "pattern-stripes", "pattern-apron", "pattern-vest", "hat-cap",
            "hat-tophat", "hat-bun", "hat-bow", "hat-beanie", "hat-mailcap", "neck-tie",
            "neck-scarf", "neck-collar", "extra-glasses", "extra-mustache", "extra-blush",
            "extra-freckles", "extra-pencil", "extra-monocle", "xf-kid", "xf-squash-050",
            "xf-campitch-s01",
        ] {
            assert!(
                v.iter().any(|(n, _, _)| n.starts_with(prefix) || n == prefix),
                "missing family {prefix}"
            );
        }
        // All 19 moods.
        assert_eq!(v.iter().filter(|(n, _, _)| n.starts_with("mood-")).count(), 19);
    }

    #[test]
    fn anchor_entries_actually_engage() {
        let v = state_spread();
        for name in ["anchor-lying-lean82", "anchor-high-sit"] {
            let (_, look, state) = v.iter().find(|(n, _, _)| n == name).expect(name);
            let a = canvas_anchor(look, state);
            println!("ANCHOR {name}: ({:.2}, {:.2})", a.0, a.1);
            assert!(a != (0.0, 0.0), "{name}: anchor must engage, got {a:?}");
        }
        // And the neutral standing state must NOT engage (the
        // anchor is zero for every already-fitting state).
        let (_, look, state) = v.iter().find(|(n, _, _)| n == "mood-neutral").expect("mood");
        assert_eq!(canvas_anchor(look, state), (0.0, 0.0));
    }

    #[test]
    fn sweat_and_tears_states_paint_differently() {
        // The state_key precedent, at paint level: each fx entry
        // must differ from the neutral base paint, and from each
        // other — otherwise the separate entries are dead weight.
        let v = state_spread();
        let get = |name: &str| v.iter().find(|(n, _, _)| n == name).unwrap();
        let (_, bl, bs) = get("mood-neutral");
        let base = paint_toon_full(bl, bs).0;
        for name in ["fx-sweat-only", "fx-tears-only", "fx-sweat-tears"] {
            let (_, l, s) = get(name);
            assert_ne!(paint_toon_full(l, s).0, base, "{name} must differ from neutral");
        }
        let (_, l1, s1) = get("fx-sweat-only");
        let (_, l2, s2) = get("fx-tears-only");
        assert_ne!(paint_toon_full(l1, s1).0, paint_toon_full(l2, s2).0);
    }

    #[test]
    fn compare_localization_matches_the_ruling() {
        // Hand-checkable fixture for the canonical definitions:
        // 9x1 canvas, reference alpha fractional (128) only at
        // x=4, opaque elsewhere; test differs at x=2..6 in R.
        // Distances to B={x=4}: x=4 -> 0 EDGE, x=3/5 -> 1 EDGE,
        // x=2/6 -> 2 FRINGE. No INTERIOR.
        let (w, h) = (9u32, 1u32);
        let mut reference = vec![0u8; (w * h * 4) as usize];
        let mut test = reference.clone();
        for x in 0..w as usize {
            reference[x * 4 + 3] = if x == 4 { 128 } else { 255 };
            test[x * 4 + 3] = reference[x * 4 + 3];
        }
        for x in 2..=6usize {
            test[x * 4] = 10; // R delta 10 at x=2..6
        }
        let p = compare_paints(&reference, &test, w, h);
        assert!(!p.identical);
        assert_eq!(p.diff_px, 5);
        assert_eq!((p.edge, p.fringe, p.interior), (3, 2, 0));
        assert_eq!(p.max_delta_rgba, [10, 0, 0, 0]);
        // Alpha is identical in the pair: alpha PSNR inf, RGBA finite.
        assert!(p.psnr_alpha_db.is_infinite());
        assert!(p.psnr_db.is_finite());
        // A differing pixel far from any boundary is INTERIOR,
        // and with NO fractional pixel anywhere every differing
        // pixel is INTERIOR (distance to a non-existent boundary
        // is > 2, per the AUDIT checker's sentinel case).
        let mut test2 = reference.clone();
        for x in 0..w as usize {
            test2[x * 4 + 3] = 255;
        }
        let reference2 = test2.clone();
        test2[0] = 7; // differ at x=0 only, alphas all opaque
        let p2 = compare_paints(&reference2, &test2, w, h);
        assert_eq!((p2.diff_px, p2.edge, p2.fringe, p2.interior), (1, 0, 0, 1));
    }

    /// SWEAT seat (Spec 5e): record ONLY the fx sweat states to
    /// $SWEAT_OUT (default: target-local sweat-evidence) — PNG +
    /// display-list JSON per state, same shape as
    /// gpupaint::record_states — for GPU replay parity of the
    /// gradation states. Ignored by default (full-canvas paints).
    /// Run with:
    ///   SWEAT_OUT=<dir> cargo test --features gpupaint sweat_record_states -- --ignored --nocapture
    #[test]
    #[ignore]
    fn sweat_record_states() {
        let out = std::env::var("SWEAT_OUT").unwrap_or_else(|_| "sweat-evidence".to_string());
        std::fs::create_dir_all(&out).expect("create SWEAT_OUT");
        let v = state_spread();
        crate::gputrace::set_armed(true);
        for name in ["fx-sweat-only", "fx-sweat-050", "fx-sweat-099", "fx-sweat-tears", "fx-worried-sweat-via-mood"] {
            let (_, look, state) = v.iter().find(|(n, _, _)| n == name).expect(name);
            crate::gputrace::begin(2560, 2816);
            let (px, w, h, _, _) = paint_toon_full(look, state);
            let list = crate::gputrace::take();
            // The traced sweat-drop prims must carry the painter's
            // alpha (fill #8FD3F4 and the INK stroke, both at the
            // gradation alpha) — print them for the parity report.
            for (i, prim) in list.prims.iter().enumerate() {
                println!("{name} prim {i}: {prim:?}");
            }
            let img = image::RgbaImage::from_raw(w, h, px).expect("pixels");
            img.save(format!("{out}/{name}.png")).expect("save png");
            std::fs::write(format!("{out}/{name}.json"), list.to_json()).expect("write json");
            println!("recorded {name}: {} prims", list.prims.len());
        }
        crate::gputrace::set_armed(false);
    }

    /// CPU-vs-CPU dry run: paint the whole spread twice through
    /// paint_toon_full and require byte identity on every entry
    /// (validates the spread constructs, paints deterministically
    /// and that compare_paints reports identity correctly).
    /// Ignored by default — it paints 2x the spread at full
    /// 2560x2816; run it with:
    ///   cargo test --release --features gpupaint dry_run -- --ignored --nocapture
    #[test]
    #[ignore]
    fn dry_run_cpu_vs_cpu() {
        let v = state_spread();
        println!(
            "name,identical,diff_px,max_dR,max_dG,max_dB,max_dA,psnr_rgba_db,psnr_alpha_db,edge,fringe,interior"
        );
        for (name, look, state) in &v {
            let a = paint_toon_full(look, state);
            let b = paint_toon_full(look, state);
            let p = compare_paints(&a.0, &b.0, a.1, a.2);
            println!(
                "{name},{},{},{},{},{},{},{:.3},{:.3},{},{},{}",
                p.identical,
                p.diff_px,
                p.max_delta_rgba[0],
                p.max_delta_rgba[1],
                p.max_delta_rgba[2],
                p.max_delta_rgba[3],
                p.psnr_db,
                p.psnr_alpha_db,
                p.edge,
                p.fringe,
                p.interior
            );
            assert!(p.identical, "{name}: CPU-vs-CPU must be byte-identical");
            assert_eq!(p.diff_px, 0);
            assert_eq!(p.max_delta, 0);
        }
    }
}
