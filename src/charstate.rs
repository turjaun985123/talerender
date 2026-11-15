// =====================================================================
// DELIVERY NOTE — TaleRender V2, Batch 2, Seat 7 (WP10)
//
// Files created:
//   src/charstate.rs  (this file; the ONLY file this seat writes)
//
// Contract points implemented (V2-CONTRACTS.md §1/§5 seat B2-7;
// V2-MASTER-PROMPT.md §14 "character rendering efficiency":
// state caching + partial updates + multi-character efficiency;
// ALL animation capabilities preserved — this module changes no
// paint behaviour, it only DESCRIBES state so the lead can skip
// work the current code redoes):
//   - `CharStateKey`: canonical, hashable (Hash + Eq) structured
//     summary of everything that determines a character's
//     painted pixels. f32 channels are stored as EXACT bit
//     patterns (`f32::to_bits`), mirroring film.rs `state_key`'s
//     exactness law — a quantised key is a determinism bug
//     (film.rs comment at `state_key`: 72.34 dB chunk-boundary
//     divergence from the old 2-dp rounding).
//   - `CharStateKey::approx_bytes()`: painted-canvas byte size
//     from the canvas dims in code (see CANVAS_W/H below).
//   - `diff(prev, next) -> ChangeClass { Identical,
//     Partial(Rect), FullRepaint }` (the task's StateDiff):
//     field-group classification justified per-group from the
//     paint code (see the table at `diff`).
//   - `MultiCharPlanner::plan`: per-char ChangeClass + per-char
//     `DirtyRects`, plus the cross-char union; pure functions,
//     output sorted by char_id (stable), deterministic.
//   - Pure logic only: no wgpu types, no I/O, no env, no new
//     dependencies, no `unwrap`/`expect` outside tests. The only
//     crate dependency is `crate::dirtyrect::{Rect, DirtyRects}`
//     (Batch 1, WP1.2) — Partial regions ARE dirtyrect rects in
//     painted-canvas coordinates, ready to feed a partial
//     `write_texture` exactly as dirtyrect.rs's note sketches.
//
// FIELD-BY-FIELD MAPPING to the real key (film.rs:2323,
//   fn state_key(look: usize, st: &FullState,
//                hold_l: Option<&str>, hold_r: Option<&str>)
//   -> String, format
//   "{look}|{mood:?}|{19 x f32-bits / tags / names}"):
//
//   CharStateKey field      Real key component          Paint input
//   ----------------------  --------------------------  ----------------
//   look_idx: u32           "{look}" prefix (usize)     FullLook via the
//                                                       film look registry
//   kid: bool               (folded into {look})        FullLook.kid —
//                                                       broken out here
//                                                       because the
//                                                       partial rects
//                                                       need it (below)
//   mood: MoodKey           "{:?}" of st.mood           FullState.mood
//   gaze_bits: [u32; 2]     q(st.look[0]), q(st.look[1])  gaze [f32; 2]
//   blink_bits: u32         q(st.blink)                 FullState.blink
//   talk_bits: u32          q(st.talk)                  FullState.talk
//   sweat: bool             *** ABSENT from real key ***  FullState.sweat
//   tears: bool             *** ABSENT from real key ***  FullState.tears
//   arm_l_bits: [u32; 2]    q(st.arm_l.0), q(st.arm_l.1)
//   arm_r_bits: [u32; 2]    q(st.arm_r.0), q(st.arm_r.1)
//   legs: LegsKey           "stand|walk|run|sit|jump"   FullState.legs
//   phase_bits: u32         q(st.phase)
//   lean_bits: u32          q(st.lean)
//   bob_bits: u32           q(st.bob)
//   squash_bits: u32        q(st.squash)
//   head_tilt_bits: u32     q(st.head_tilt)
//   head_turn_bits: u32     q(st.head_turn)
//   cam_pitch_bits: u32     q(st.cam_pitch)
//   sit_drop_bits: Option<u32>
//                           st.sit_drop.map(q).map(|v| v as i64)
//                             .unwrap_or(-1)
//                           (u32 zero-extends into i64, so the
//                           -1 None sentinel is unambiguous)
//   hold_l: Option<String>  hold_l.unwrap_or("-")       plate NAME;
//                           paint resolves it through
//                           assets.plates to a Plate
//   hold_r: Option<String>  hold_r.unwrap_or("-")       (same)
//
//   Coverage claim: FullState has exactly 19 fields
//   (toonfull.rs:140-161); every one is represented above.
//   FullLook has 14 fields (toonfull.rs:123-138: body, head,
//   shade, pattern, pat_col, hat, hat_col, band, neck,
//   neck_col, glasses, mustache, mustache_col, kid); all but
//   kid are represented by look_idx ALONE, which is sound
//   only because the film path's registry
//   (film.rs `look_registry`) is fixed for a run — an index
//   names exactly one FullLook. kid is additionally broken
//   out because `paint_toon_full` shifts the whole figure by
//   it (`by += 190 if kid`) and the Partial rects below are
//   computed from it.
//
//   *** FINDING — the real key is INCOMPLETE: `state_key`
//   omits `sweat` and `tears`, but the painter consumes both
//   (toonfull.rs:775 `if st.sweat || m == Worried` — sweat
//   drop; :788 `if st.tears || m == Sad` — tear ellipses).
//   Two states differing ONLY in sweat (with a non-Worried
//   mood) therefore collide in PaintCache today and the
//   second frame reuses the first frame's paint — the cache
//   stops being the pure optimisation film.rs's PaintCache
//   comment promises. This module's key INCLUDES both fields,
//   so `diff` classifies a sweat/tears-only change as
//   Partial(face) instead of missing it. If the lead migrates
//   the PaintCache key to CharStateKey, expect extra misses
//   exactly on such frames and POSSIBLE pixel changes there
//   (the byte-probe gate is the arbiter); do not migrate
//   silently inside another landing.
//
// POSITION-INDEPENDENCE — VERIFIED, with one caveat:
//   `paint_toon_full(look: &FullLook, st: &FullState)` takes
//   NO position. film.rs `resolve_chars` uses CharDraw's
//   pos / yaw / pitch ONLY after painting: the card model
//   matrix (t3(pos) * rotate_y(yaw) * rotate_x(pitch)), the
//   blob shadow, and the rifle parts; `tint` is a BasicDraw
//   field applied at composite; `rifle` adds separate 3D
//   parts, never card pixels. None of pos/yaw/pitch/tint/
//   rifle appears in `state_key` or in this key, so a
//   position-only change yields EQUAL keys and `diff` returns
//   Identical — the painted pixels genuinely do not change.
//   CAVEAT: `cam_pitch` IS a paint input (head radius
//   rp = r*(1+0.04*cam_pitch), eye line ey += cam_pitch*8)
//   and it is DERIVED from position + camera in
//   film.rs `finish_chars`/`cam_pitch_of`. A move that
//   changes cam_pitch's bits is therefore NOT position-only
//   for paint purposes: it lands in the head group below and
//   classifies Partial(head) — never Identical.
//
// Wiring the lead must do (this seat edits no existing file):
//   1. src/main.rs — add the module line after the anchor
//          mod charanim;
//      and before `mod composer;`:
//          mod charstate;
//   2. src/film.rs `resolve_chars` — the consult site for key
//      construction is beside the existing key call, anchored
//      by
//          let key = state_key(c.look, &st, c.hold_l, c.hold_r);
//      Build the CharStateKey from the same inputs
//      (c.look as u32, look.kid from `looks[c.look].0.kid`,
//      st.*, c.hold_l/c.hold_r as Option<String>) and keep it
//      per TexSrc::Dyn slot next to the existing prev-frame
//      state WP1.2's wiring keeps. The String key stays the
//      PaintCache key of record for this landing (see the
//      sweat/tears finding before changing that).
//   3. src/film.rs `Renderer::render` / `upload_dyn` (the
//      WP1.2 site): per Dyn slot, consult
//      `MultiCharPlanner::plan` (or `diff` directly) with the
//      slot's prev/cur keys — Identical: skip the upload
//      entirely once textures persist across frames;
//      Partial(r): partial-write the rect, cross-checked
//      against `DirtyRects::from_diff` during bring-up (the
//      diff is the ground truth; this classification is the
//      cheap predictor); FullRepaint: today's full upload.
//   4. OPTIONAL later landing: PaintCache::get_or_paint keyed
//      by CharStateKey instead of String (drops the per-frame
//      format! allocation); gated on the byte probes because
//      of the sweat/tears finding above.
//
// Tests included (in-module, #[cfg(test)]): key equality +
// hash sensitivity for EVERY field (incl. exact-bits cases:
// -0.0 vs 0.0 differ, sit_drop None vs Some(0.0) differ);
// approx_bytes against the in-code canvas dims; the full
// diff classification table (identical, position-caveat via
// cam_pitch, blink/talk/gaze/sweat/mood -> Partial(face) with
// the exact expected rect, squash/head group ->
// Partial(head), face+head -> head, phase-only Identical
// under Stand/Sit and Partial(legs) under Walk/Run, and
// FullRepaint for look/arm/legs/lean/bob/sit_drop/hold);
// face-rect shifts for kid and sitting figures; planner
// ordering by char_id, spawn (no prev) -> FullRepaint,
// per-char dirty contents, combined-union behaviour
// (identical rects coalesce, a full canvas absorbs a
// partial), and summary counts / planned bytes.
//
// Known limitations:
//   (a) ADVISORY UNTIL WIRED: no paint code changes, no call
//       sites — classification predicts; `DirtyRects::
//       from_diff` on the real pixels remains the ground
//       truth a wired caller should assert against in tests.
//   (b) The Partial rects are NOMINAL-POSE regions padded
//       generously (they over-cover, never under-cover, at
//       the shared context of the two keys — see `diff` for
//       why the context is shared). Extreme head_tilt / lean
//       values beyond the film's staged ranges could in
//       principle push a stroke past a rect edge; the rects
//       are sized from the painter's stage geometry with
//       40-200 px of margin, and (a)'s cross-check exists
//       precisely to catch any miss before partial uploads
//       are trusted.
//   (c) look_idx names a FullLook only within the film
//       registry path. scenefile.rs builds FullLooks ad hoc
//       and paints UNCACHED (scenefile.rs:687 calls
//       paint_toon_full directly); if that path ever caches,
//       it must intern looks or hash the FullLook fields —
//       an index into a mutable set is not a key.
//   (d) hold_l/hold_r store the plate NAME (as the real key
//       does). Within a run assets.plates maps a name to
//       exactly one Plate, so name equality is pixel
//       equality; two different plates sharing a name across
//       asset reloads would defeat it — same caveat the
//       String key already carries.
//   (e) approx_bytes() is the same for every state: the
//       painter always allocates the full 2560x2816 canvas
//       (Painter::new(cw, ch) unconditionally), so a state's
//       painted cost does not vary with its content. Held
//       plates are INPUTS shared via Assets, not part of the
//       painted entry's bytes (PaintBudget's entry_bytes
//       likewise counts only Painted.0).
// =====================================================================

//! Character-state keys, change classification and a
//! multi-character update planner (WP10).
//!
//! The renderer's per-frame character cost has two halves:
//! painting (CPU, cached by film.rs's `PaintCache` under an
//! exact String key) and uploading the painted canvas
//! (2560x2816 RGBA, ~29 MB per character per frame, via
//! `Renderer::upload_dyn`). WP1.2's `dirtyrect` module covers
//! the upload half once the caller knows WHAT changed; this
//! module is the cheap predictor that answers that question
//! from state alone, without touching a pixel: given the
//! previous and current `CharStateKey` of each character,
//! decide whether its painted canvas is byte-identical
//! (skip everything), changed only inside a known face/head/
//! legs region (repaint or re-upload just that region), or
//! changed arbitrarily (full repaint).
//!
//! Characters remain what the project charter says they are:
//! 2D painted cards inside dimensional 3D environments. Every
//! field here describes the 2D paint; world placement
//! (position, yaw, pitch, tint) is deliberately NOT part of
//! the key because it is applied at composite time, after
//! painting — see the delivery note's position-independence
//! verification.
//!
//! Determinism: keys compare f32 channels by exact bit
//! pattern, rects are pure functions of the keys, and the
//! planner's output order is sorted by char_id — nothing here
//! depends on hash iteration, insertion order, time or env.

use crate::dirtyrect::{DirtyRects, Rect};

/// Painted-canvas width in texels: `paint_toon_full` paints
/// at `(1280 * 2, 1408 * 2)` (toonfull.rs, RES = 2 over the
/// 1280x1408 card). Matches paintbudget::CANVAS_BYTES's dims.
pub(crate) const CANVAS_W: u32 = 2560;

/// Painted-canvas height in texels; see `CANVAS_W`.
pub(crate) const CANVAS_H: u32 = 2816;

/// Bytes per texel of the painted canvas (RGBA8).
pub(crate) const BYTES_PER_PIXEL: u64 = 4;

/// Exact byte size of one painted character canvas:
/// 2560 x 2816 x 4 = 28,835,840. The same constant
/// paintbudget.rs keeps as `CANVAS_BYTES`.
pub(crate) const CANVAS_BYTES: u64 =
    CANVAS_W as u64 * CANVAS_H as u64 * BYTES_PER_PIXEL;

/// Mirror of `toonfull::Mood` (17 variants, same declaration
/// order, so the discriminant matches the source enum's).
/// Mirrored rather than imported so this module stays a
/// standalone pure-logic unit (the charanim.rs precedent);
/// the lead's wiring maps `Mood -> MoodKey` 1:1 by variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MoodKey {
    Neutral,
    Smug,
    Happy,
    Laugh,
    Shock,
    Worried,
    Angry,
    Sad,
    Sly,
    Proud,
    Tired,
    Dizzy,
    Determined,
    Embarrassed,
    Awe,
    Suspicious,
    Relieved,
}

/// Mirror of `toonfull::Legs` (same variants, same order);
/// see `MoodKey` for why it is mirrored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LegsKey {
    Stand,
    Walk,
    Run,
    Sit,
    Jump,
}

/// Canonical summary of everything that determines one
/// character's painted pixels. Two states with equal keys
/// paint byte-identical canvases; two states with different
/// keys MAY still paint identically in one documented case
/// (phase under non-gait legs — `diff` knows about it), never
/// the reverse: key equality is the sound direction, exactly
/// as with film.rs's String key.
///
/// All f32 channels are stored as `f32::to_bits()` patterns,
/// so Eq/Hash are exact-bit comparisons (0.0 != -0.0, NaN
/// equals only a NaN with the same payload) — the same
/// semantics the String key gets from formatting the bits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CharStateKey {
    /// Index into the film look registry (the real key's
    /// `"{look}"` prefix). Names the character's whole
    /// FullLook for the run; see the delivery note.
    pub look_idx: u32,
    /// FullLook.kid, broken out of the look because the
    /// painter shifts the figure by it and the Partial rects
    /// are computed from it.
    pub kid: bool,
    /// Expression (real key: `{:?}` of FullState.mood).
    pub mood: MoodKey,
    /// Gaze direction, FullState.look, exact bits.
    pub gaze_bits: [u32; 2],
    /// Eyelid closure 0..1, FullState.blink, exact bits.
    pub blink_bits: u32,
    /// Mouth openness (lip-sync), FullState.talk, exact bits.
    pub talk_bits: u32,
    /// Sweat drop flag. ABSENT from the real String key —
    /// see the delivery note's finding; the painter reads it.
    pub sweat: bool,
    /// Tear flag. ABSENT from the real String key; the
    /// painter reads it.
    pub tears: bool,
    /// Left arm (shoulder, elbow) degrees, exact bits.
    pub arm_l_bits: [u32; 2],
    /// Right arm (shoulder, elbow) degrees, exact bits.
    pub arm_r_bits: [u32; 2],
    /// Leg mode (real key: "stand|walk|run|sit|jump").
    pub legs: LegsKey,
    /// Gait phase, radians, exact bits. Consumed ONLY by the
    /// Walk/Run leg branch of the painter.
    pub phase_bits: u32,
    /// Whole-figure lean, degrees, exact bits. The painter
    /// rotates the ENTIRE canvas mapping by it.
    pub lean_bits: u32,
    /// Vertical bob, stage px, exact bits. Shifts the whole
    /// figure (body, head, hips all derive from it).
    pub bob_bits: u32,
    /// Head squash, unitless, exact bits. Head/hat only.
    pub squash_bits: u32,
    /// Head tilt, degrees, exact bits. Head parts only.
    pub head_tilt_bits: u32,
    /// Head turn, -1..1-ish, exact bits. Head parts only.
    pub head_turn_bits: u32,
    /// Camera-pitch factor, exact bits. Head size + eye line
    /// only — but DERIVED from world position (see note).
    pub cam_pitch_bits: u32,
    /// Seat drop for sitting, stage px, exact bits; None =
    /// the painter's default (220.2). The real key encodes
    /// this as the bits widened to i64, or -1 for None.
    pub sit_drop_bits: Option<u32>,
    /// Held-prop plate NAME, left hand (the real key's
    /// `hold_l.unwrap_or("-")`).
    pub hold_l: Option<String>,
    /// Held-prop plate NAME, right hand.
    pub hold_r: Option<String>,
}

impl CharStateKey {
    /// Build a key from paint-state values, converting every
    /// f32 channel to its exact bit pattern. This is the
    /// constructor the lead's wiring calls from
    /// `resolve_chars` (values are FullState's fields plus
    /// the look index, kid flag and hold names).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        look_idx: u32,
        kid: bool,
        mood: MoodKey,
        gaze: [f32; 2],
        blink: f32,
        talk: f32,
        sweat: bool,
        tears: bool,
        arm_l: (f32, f32),
        arm_r: (f32, f32),
        legs: LegsKey,
        phase: f32,
        lean: f32,
        bob: f32,
        squash: f32,
        head_tilt: f32,
        head_turn: f32,
        cam_pitch: f32,
        sit_drop: Option<f32>,
        hold_l: Option<String>,
        hold_r: Option<String>,
    ) -> CharStateKey {
        CharStateKey {
            look_idx,
            kid,
            mood,
            gaze_bits: [gaze[0].to_bits(), gaze[1].to_bits()],
            blink_bits: blink.to_bits(),
            talk_bits: talk.to_bits(),
            sweat,
            tears,
            arm_l_bits: [arm_l.0.to_bits(), arm_l.1.to_bits()],
            arm_r_bits: [arm_r.0.to_bits(), arm_r.1.to_bits()],
            legs,
            phase_bits: phase.to_bits(),
            lean_bits: lean.to_bits(),
            bob_bits: bob.to_bits(),
            squash_bits: squash.to_bits(),
            head_tilt_bits: head_tilt.to_bits(),
            head_turn_bits: head_turn.to_bits(),
            cam_pitch_bits: cam_pitch.to_bits(),
            sit_drop_bits: sit_drop.map(f32::to_bits),
            hold_l,
            hold_r,
        }
    }

    /// Estimated bytes of the painted canvas for this state.
    /// The painter always allocates the full canvas
    /// (2560x2816 RGBA), so this is `CANVAS_BYTES` for every
    /// state — content does not change the allocation. It is
    /// an ESTIMATE in the telemetry sense only in that the
    /// Rc/tuple overhead beside the pixel buffer is excluded,
    /// exactly as paintbudget::entry_bytes excludes it.
    pub(crate) fn approx_bytes(&self) -> u64 {
        CANVAS_BYTES
    }

    /// Decoded gaze (FullState.look).
    pub(crate) fn gaze(&self) -> [f32; 2] {
        [f32::from_bits(self.gaze_bits[0]), f32::from_bits(self.gaze_bits[1])]
    }

    /// Decoded blink.
    pub(crate) fn blink(&self) -> f32 {
        f32::from_bits(self.blink_bits)
    }

    /// Decoded talk.
    pub(crate) fn talk(&self) -> f32 {
        f32::from_bits(self.talk_bits)
    }

    /// Decoded gait phase.
    pub(crate) fn phase(&self) -> f32 {
        f32::from_bits(self.phase_bits)
    }

    /// Decoded lean, degrees.
    pub(crate) fn lean(&self) -> f32 {
        f32::from_bits(self.lean_bits)
    }

    /// Decoded bob, stage px.
    pub(crate) fn bob(&self) -> f32 {
        f32::from_bits(self.bob_bits)
    }

    /// Decoded sit drop, stage px.
    pub(crate) fn sit_drop(&self) -> Option<f32> {
        self.sit_drop_bits.map(f32::from_bits)
    }
}

// ---------------------------------------------------------------------
// Partial-update regions (painted-canvas coordinates).
//
// Geometry derivation (toonfull.rs; stage space is 512x1024
// with feet at y=1000, mapped to canvas by
// ((x + 384) * 2, (y + 256) * 2) before the lean rotation):
// head centre stage (256, 178), radius 138; eyes at stage
// y ~= 182, mouth at y ~= 244, sweat drop at (fx + 104,
// hy - 70), tears at eye y + 40, hat top reaches stage
// y ~= -22 (bowler dome), hips at stage y ~= 574, feet at
// y = 1000. The rects below are those spans converted to
// canvas texels and padded; `*_rect_for` then shifts them by
// the figure offset the painter itself applies (kid / sit /
// bob / jump via `by`, lean via the rotation about the feet),
// which is well-defined for a Partial pair because any
// difference in those very fields classifies FullRepaint —
// a Partial pair always shares them (see `diff`).
// ---------------------------------------------------------------------

/// Face region: brows, eyes (incl. Shock/Awe white eyes and
/// glasses), mouth (incl. open/laugh mouths and mustache),
/// blush, sweat drop and tears — every pixel the face field
/// group can touch at the nominal pose (adult, standing,
/// no lean). Canvas x 960..1640, y 620..1200.
pub(crate) const FACE_RECT: Rect = Rect::new(960, 620, 680, 580);

/// Head region: the whole head ellipse, its shade, and the
/// hat (cap or bowler, dome top included), plus everything
/// in FACE_RECT — every pixel the head field group can
/// touch. Contains FACE_RECT. Canvas x 900..1700,
/// y 400..1200.
pub(crate) const HEAD_RECT: Rect = Rect::new(900, 400, 800, 800);

/// Legs region: both leg strokes, knees, feet ellipses and
/// the gait lift span — every pixel a phase change under
/// Walk/Run can touch. Canvas x 1000..1560, y 1640..2580.
pub(crate) const LEGS_RECT: Rect = Rect::new(1000, 1640, 560, 940);

/// The painter's figure offset `by` (stage px) for a key:
/// sit term + kid term + bob + jump term, in the painter's
/// own formula (toonfull.rs: `by = (if sit {(426 - sit_s) /
/// 0.98} else {0}) + (if kid {190} else {0}) + bob +
/// (if jump {-60} else {0})`, sit_s defaulting to 220.2).
fn by_stage(key: &CharStateKey) -> f32 {
    let sit = if key.legs == LegsKey::Sit {
        (426.0 - key.sit_drop().unwrap_or(220.2)) / 0.98
    } else {
        0.0
    };
    let kid = if key.kid { 190.0 } else { 0.0 };
    let jump = if key.legs == LegsKey::Jump { -60.0 } else { 0.0 };
    sit + kid + key.bob() + jump
}

/// Shift a nominal rect by (dx, dy) canvas px, clamped so
/// the rect stays fully inside the canvas (rects are all
/// smaller than the canvas, so clamping is total).
fn shift_rect(base: Rect, dx: i64, dy: i64) -> Rect {
    let max_x = i64::from(CANVAS_W - base.w);
    let max_y = i64::from(CANVAS_H - base.h);
    Rect {
        x: (i64::from(base.x) + dx).clamp(0, max_x) as u32,
        y: (i64::from(base.y) + dy).clamp(0, max_y) as u32,
        w: base.w,
        h: base.h,
    }
}

/// Canvas offset of the head for a key: vertically the
/// painter's `by * 0.99` head factor at RES 2; horizontally
/// the lean rotation about the feet pivot (stage (256,
/// 1000)) — the head centre sits ~(822 - by*0.99) stage px
/// above the pivot, so its canvas x moves by that distance
/// times sin(lean). Both use the SHARED context of a
/// Partial pair (lean/bob/sit/kid are equal — see `diff`).
fn head_offset(key: &CharStateKey) -> (i64, i64) {
    let by = by_stage(key);
    let dy = (by * 0.99 * 2.0).round() as i64;
    let lean_rad = key.lean().to_radians();
    let dx = ((822.0 - by * 0.99) * lean_rad.sin() * 2.0).round() as i64;
    (dx, dy)
}

/// The face region for the figure a key describes:
/// FACE_RECT shifted by the key's head offset.
pub(crate) fn face_rect_for(key: &CharStateKey) -> Rect {
    let (dx, dy) = head_offset(key);
    shift_rect(FACE_RECT, dx, dy)
}

/// The head region for the figure a key describes:
/// HEAD_RECT shifted by the key's head offset.
pub(crate) fn head_rect_for(key: &CharStateKey) -> Rect {
    let (dx, dy) = head_offset(key);
    shift_rect(HEAD_RECT, dx, dy)
}

/// The legs region for the figure a key describes:
/// LEGS_RECT shifted vertically by the hip factor
/// (`by * 0.98` at RES 2 — the painter's body-bottom
/// factor) and horizontally by the lean offset.
pub(crate) fn legs_rect_for(key: &CharStateKey) -> Rect {
    let by = by_stage(key);
    let dy = (by * 0.98 * 2.0).round() as i64;
    let lean_rad = key.lean().to_radians();
    let dx = ((822.0 - by * 0.99) * lean_rad.sin() * 2.0).round() as i64;
    shift_rect(LEGS_RECT, dx, dy)
}

/// How a character's painted canvas changed between two
/// states. `Partial` carries the affected region in
/// painted-canvas coordinates (a `dirtyrect::Rect`): every
/// texel that can differ lies inside it (over-cover only,
/// the dirtyrect law).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChangeClass {
    /// Byte-identical pixels: skip repaint AND re-upload.
    Identical,
    /// Changes confined to the carried region: repaint /
    /// re-upload that region only.
    Partial(Rect),
    /// Changes anywhere: full repaint / full upload.
    FullRepaint,
}

/// Classify the change from `prev` to `next` (the task's
/// StateDiff). Rules, each justified from toonfull.rs:
///
/// - Equal keys -> Identical. Position-only changes land
///   here too BY CONSTRUCTION: pos/yaw/pitch/tint/rifle are
///   not paint inputs and not key fields (delivery note).
/// - FACE group only (mood, gaze, blink, talk, sweat,
///   tears) -> Partial(face): blink is read only in the eye
///   block (closed-eye stroke / pupil height), talk only
///   in the mouth block, gaze only shifts pupils, and mood
///   drives brows / eye shape / mouth shape / blush plus
///   the auto-fired sweat (Worried) and tears (Sad) — all
///   inside FACE_RECT. sweat/tears flags likewise paint
///   only their drop/ellipse inside FACE_RECT.
/// - HEAD group (head_tilt, head_turn, squash, cam_pitch),
///   alone or with the face group -> Partial(head): these
///   are consumed only by the head transform (`hpt`), the
///   head ellipse/shade, the face (via hpt) and the hat —
///   all inside HEAD_RECT, which contains FACE_RECT, so a
///   mixed face+head change is still one Partial(head).
/// - phase only -> Identical when legs are not Walk/Run
///   (the painter reads phase ONLY in the Walk/Run leg
///   branch; under Stand/Sit/Jump the channel is dead and
///   the current String key wastes a repaint on it), else
///   Partial(legs).
/// - Anything else -> FullRepaint: look_idx / kid (whole
///   look), arm_l / arm_r (arm strokes span the body and
///   carry the held plates to arbitrary hand points),
///   hold_l / hold_r (plate pixels composite at the hand,
///   size and content set by the plate), legs mode (leg
///   geometry + sit/jump figure offset), lean (rotates the
///   ENTIRE canvas mapping about the feet), bob (shifts
///   body, head and hips together), sit_drop (figure offset
///   + seated leg reach).
///
/// Invariant the rects rely on: a Partial verdict implies
/// lean, bob, legs, sit_drop and kid are EQUAL in prev and
/// next (any difference is FullRepaint above), so the
/// figure offset used by `*_rect_for(next)` is the pair's
/// shared offset.
pub(crate) fn diff(prev: &CharStateKey, next: &CharStateKey) -> ChangeClass {
    if prev == next {
        return ChangeClass::Identical;
    }
    let full = prev.look_idx != next.look_idx
        || prev.kid != next.kid
        || prev.arm_l_bits != next.arm_l_bits
        || prev.arm_r_bits != next.arm_r_bits
        || prev.legs != next.legs
        || prev.lean_bits != next.lean_bits
        || prev.bob_bits != next.bob_bits
        || prev.sit_drop_bits != next.sit_drop_bits
        || prev.hold_l != next.hold_l
        || prev.hold_r != next.hold_r;
    if full {
        return ChangeClass::FullRepaint;
    }
    let head = prev.head_tilt_bits != next.head_tilt_bits
        || prev.head_turn_bits != next.head_turn_bits
        || prev.squash_bits != next.squash_bits
        || prev.cam_pitch_bits != next.cam_pitch_bits;
    if head {
        return ChangeClass::Partial(head_rect_for(next));
    }
    let face = prev.mood != next.mood
        || prev.gaze_bits != next.gaze_bits
        || prev.blink_bits != next.blink_bits
        || prev.talk_bits != next.talk_bits
        || prev.sweat != next.sweat
        || prev.tears != next.tears;
    if face {
        return ChangeClass::Partial(face_rect_for(next));
    }
    if prev.phase_bits != next.phase_bits {
        return match next.legs {
            LegsKey::Walk | LegsKey::Run => ChangeClass::Partial(legs_rect_for(next)),
            _ => ChangeClass::Identical,
        };
    }
    // Unreachable by construction (every field is in one of
    // the groups above, and equal keys returned earlier);
    // Identical is the safe answer if a future field is
    // added to the key without updating this function, the
    // wired caller's from_diff cross-check would catch it.
    ChangeClass::Identical
}

/// One character's planned update: its classification and
/// the dirty set that classification implies on its own
/// canvas (empty for Identical, the Partial rect, or the
/// full canvas for FullRepaint).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CharPlan {
    pub char_id: u32,
    pub class: ChangeClass,
    pub dirty: DirtyRects,
}

/// The planner's output for one frame's character set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MultiPlan {
    /// Per-character plans, sorted by char_id ascending
    /// (stable: duplicate ids keep input order).
    pub plans: Vec<CharPlan>,
    /// Union of every character's dirty set (absorbed, as
    /// `DirtyRects::absorb` defines it). Today each character
    /// owns a separate canvas/texture, so the PER-CHAR sets
    /// are the actionable output; the union is the figure a
    /// future shared atlas (WP10's atlas integration) or a
    /// frame-level "any partial upload worth doing" decision
    /// consumes.
    pub combined: DirtyRects,
}

impl MultiPlan {
    /// Characters whose paint is unchanged.
    pub(crate) fn identical_count(&self) -> usize {
        self.plans.iter().filter(|p| p.class == ChangeClass::Identical).count()
    }

    /// Characters with a region-confined change.
    pub(crate) fn partial_count(&self) -> usize {
        self.plans
            .iter()
            .filter(|p| matches!(p.class, ChangeClass::Partial(_)))
            .count()
    }

    /// Characters needing a full repaint (incl. spawns).
    pub(crate) fn full_count(&self) -> usize {
        self.plans.iter().filter(|p| p.class == ChangeClass::FullRepaint).count()
    }

    /// Total bytes the planned uploads move across all
    /// characters: the sum of each character's own
    /// `planned_upload_bytes` (partial sum, or the full
    /// canvas when that character's set falls back). NOT
    /// derived from `combined` — the characters live on
    /// separate canvases, so their bytes add, not union.
    pub(crate) fn total_planned_upload_bytes(&self) -> u64 {
        self.plans
            .iter()
            .map(|p| p.dirty.planned_upload_bytes(CANVAS_W, CANVAS_H, BYTES_PER_PIXEL as u32))
            .sum()
    }
}

/// Multi-character update planner: pure, deterministic,
/// allocation-light (one Vec + one DirtyRects per char).
pub(crate) struct MultiCharPlanner;

impl MultiCharPlanner {
    /// Plan one frame's updates. Each entry is
    /// `(char_id, prev_key, next_key)`; a `None` prev key is
    /// a character with no previous paint (first frame,
    /// spawn) and plans as FullRepaint over the full canvas.
    ///
    /// Output plans are sorted by char_id ascending with a
    /// stable sort, so the result is a function of the SET
    /// of updates, not the caller's ordering (duplicate ids,
    /// which the caller should not produce, keep their
    /// relative input order).
    pub(crate) fn plan(updates: &[(u32, Option<&CharStateKey>, &CharStateKey)]) -> MultiPlan {
        let mut plans: Vec<CharPlan> = Vec::with_capacity(updates.len());
        let mut combined = DirtyRects::new();
        for &(char_id, prev, next) in updates {
            let class = match prev {
                None => ChangeClass::FullRepaint,
                Some(p) => diff(p, next),
            };
            let mut dirty = DirtyRects::new();
            match class {
                ChangeClass::Identical => {}
                ChangeClass::Partial(r) => dirty.add(r),
                ChangeClass::FullRepaint => dirty.add_full(CANVAS_W, CANVAS_H),
            }
            combined.absorb(&dirty);
            plans.push(CharPlan { char_id, class, dirty });
        }
        plans.sort_by_key(|p| p.char_id);
        MultiPlan { plans, combined }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn hash_of(k: &CharStateKey) -> u64 {
        let mut h = DefaultHasher::new();
        k.hash(&mut h);
        h.finish()
    }

    /// The painter's rest state (film.rs FullState::default):
    /// adult, neutral, standing, arms at (8, 6), no holds.
    fn base() -> CharStateKey {
        CharStateKey::new(
            0,
            false,
            MoodKey::Neutral,
            [0.0, 0.0],
            0.0,
            0.0,
            false,
            false,
            (8.0, 6.0),
            (8.0, 6.0),
            LegsKey::Stand,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            None,
            None,
            None,
        )
    }

    #[test]
    fn canvas_dims_match_the_painter() {
        // toonfull.rs paints (1280*2) x (1408*2) RGBA;
        // paintbudget.rs CANVAS_BYTES agrees.
        assert_eq!(CANVAS_W, 2560);
        assert_eq!(CANVAS_H, 2816);
        assert_eq!(CANVAS_BYTES, 2560 * 2816 * 4);
        assert_eq!(CANVAS_BYTES, 28_835_840);
        assert_eq!(base().approx_bytes(), CANVAS_BYTES);
        // Every state costs the same canvas (full allocation).
        let mut k = base();
        k.mood = MoodKey::Laugh;
        k.legs = LegsKey::Run;
        assert_eq!(k.approx_bytes(), CANVAS_BYTES);
    }

    #[test]
    fn equal_keys_are_equal_and_hash_equal() {
        let a = base();
        let b = base();
        assert_eq!(a, b);
        assert_eq!(hash_of(&a), hash_of(&b));
        // Decoded accessors round-trip the constructor input.
        assert_eq!(a.gaze(), [0.0, 0.0]);
        assert_eq!(a.blink(), 0.0);
        assert_eq!(a.talk(), 0.0);
        assert_eq!(a.phase(), 0.0);
        assert_eq!(a.lean(), 0.0);
        assert_eq!(a.bob(), 0.0);
        assert_eq!(a.sit_drop(), None);
    }

    #[test]
    fn every_field_is_key_sensitive() {
        let a = base();
        let mut variants: Vec<CharStateKey> = Vec::new();
        let mut v = base();
        v.look_idx = 1;
        variants.push(v);
        let mut v = base();
        v.kid = true;
        variants.push(v);
        let mut v = base();
        v.mood = MoodKey::Happy;
        variants.push(v);
        let mut v = base();
        v.gaze_bits = [0.5f32.to_bits(), 0];
        variants.push(v);
        let mut v = base();
        v.gaze_bits = [0, (-0.25f32).to_bits()];
        variants.push(v);
        let mut v = base();
        v.blink_bits = 1.0f32.to_bits();
        variants.push(v);
        let mut v = base();
        v.talk_bits = 0.5f32.to_bits();
        variants.push(v);
        let mut v = base();
        v.sweat = true;
        variants.push(v);
        let mut v = base();
        v.tears = true;
        variants.push(v);
        let mut v = base();
        v.arm_l_bits = [9.0f32.to_bits(), 6.0f32.to_bits()];
        variants.push(v);
        let mut v = base();
        v.arm_r_bits = [8.0f32.to_bits(), 7.0f32.to_bits()];
        variants.push(v);
        let mut v = base();
        v.legs = LegsKey::Walk;
        variants.push(v);
        let mut v = base();
        v.phase_bits = 1.0f32.to_bits();
        variants.push(v);
        let mut v = base();
        v.lean_bits = 2.0f32.to_bits();
        variants.push(v);
        let mut v = base();
        v.bob_bits = 3.0f32.to_bits();
        variants.push(v);
        let mut v = base();
        v.squash_bits = 0.1f32.to_bits();
        variants.push(v);
        let mut v = base();
        v.head_tilt_bits = 5.0f32.to_bits();
        variants.push(v);
        let mut v = base();
        v.head_turn_bits = 0.4f32.to_bits();
        variants.push(v);
        let mut v = base();
        v.cam_pitch_bits = 0.2f32.to_bits();
        variants.push(v);
        let mut v = base();
        v.sit_drop_bits = Some(220.2f32.to_bits());
        variants.push(v);
        let mut v = base();
        v.hold_l = Some("paper".to_string());
        variants.push(v);
        let mut v = base();
        v.hold_r = Some("book".to_string());
        variants.push(v);
        assert_eq!(variants.len(), 22);
        for v in &variants {
            assert_ne!(&a, v, "field change must change the key: {v:?}");
            assert_ne!(hash_of(&a), hash_of(v), "field change must change the hash: {v:?}");
        }
    }

    #[test]
    fn exact_bits_semantics_match_the_string_key() {
        // -0.0 and 0.0 have different bit patterns, so the
        // keys differ (the String key formats the same bits).
        let mut neg = base();
        neg.blink_bits = (-0.0f32).to_bits();
        assert_ne!(base(), neg);
        assert_ne!(base().blink_bits, neg.blink_bits);
        // But the CLASSIFICATION is still only Partial(face):
        // exactness costs a regional repaint, not a full one.
        assert_eq!(diff(&base(), &neg), ChangeClass::Partial(FACE_RECT));
        // sit_drop None vs Some(0.0): different states (the
        // painter's default only applies when legs == Sit,
        // and the key must not conflate them).
        let mut some_zero = base();
        some_zero.sit_drop_bits = Some(0.0f32.to_bits());
        assert_ne!(base(), some_zero);
    }

    #[test]
    fn diff_identical_states() {
        assert_eq!(diff(&base(), &base()), ChangeClass::Identical);
    }

    #[test]
    fn diff_face_group_is_partial_face() {
        // Blink-only: THE partial case — a blink touches the
        // eye block and nothing else.
        let mut blink = base();
        blink.blink_bits = 1.0f32.to_bits();
        assert_eq!(diff(&base(), &blink), ChangeClass::Partial(FACE_RECT));
        // Talk-only (lip-sync mouth).
        let mut talk = base();
        talk.talk_bits = 0.8f32.to_bits();
        assert_eq!(diff(&base(), &talk), ChangeClass::Partial(FACE_RECT));
        // Gaze-only (pupils move a few texels).
        let mut gaze = base();
        gaze.gaze_bits = [1.0f32.to_bits(), 0];
        assert_eq!(diff(&base(), &gaze), ChangeClass::Partial(FACE_RECT));
        // Mood-only (brows/eyes/mouth/blush, all facial).
        let mut mood = base();
        mood.mood = MoodKey::Angry;
        assert_eq!(diff(&base(), &mood), ChangeClass::Partial(FACE_RECT));
        // Sweat-only — the change the real String key cannot
        // even see (delivery-note finding).
        let mut sweat = base();
        sweat.sweat = true;
        assert_eq!(diff(&base(), &sweat), ChangeClass::Partial(FACE_RECT));
        // Tears-only, same story.
        let mut tears = base();
        tears.tears = true;
        assert_eq!(diff(&base(), &tears), ChangeClass::Partial(FACE_RECT));
        // The whole face group together is still one Partial.
        let mut all = base();
        all.mood = MoodKey::Laugh;
        all.blink_bits = 0.5f32.to_bits();
        all.talk_bits = 1.0f32.to_bits();
        all.sweat = true;
        assert_eq!(diff(&base(), &all), ChangeClass::Partial(FACE_RECT));
        // At the nominal pose the rect helper is the const.
        assert_eq!(face_rect_for(&base()), FACE_RECT);
    }

    #[test]
    fn diff_head_group_is_partial_head() {
        let mut squash = base();
        squash.squash_bits = 0.2f32.to_bits();
        assert_eq!(diff(&base(), &squash), ChangeClass::Partial(HEAD_RECT));
        let mut tilt = base();
        tilt.head_tilt_bits = 6.0f32.to_bits();
        assert_eq!(diff(&base(), &tilt), ChangeClass::Partial(HEAD_RECT));
        let mut turn = base();
        turn.head_turn_bits = (-0.5f32).to_bits();
        assert_eq!(diff(&base(), &turn), ChangeClass::Partial(HEAD_RECT));
        // cam_pitch: the position-caveat case — a camera /
        // position move that changes cam_pitch is NOT
        // Identical, it is a head-region repaint.
        let mut pitch = base();
        pitch.cam_pitch_bits = 0.3f32.to_bits();
        assert_eq!(diff(&base(), &pitch), ChangeClass::Partial(HEAD_RECT));
        // Face + head together: HEAD_RECT contains FACE_RECT,
        // so the union is the head rect alone.
        assert!(HEAD_RECT.x <= FACE_RECT.x && HEAD_RECT.y <= FACE_RECT.y);
        assert!(HEAD_RECT.right() >= FACE_RECT.right());
        assert!(HEAD_RECT.bottom() >= FACE_RECT.bottom());
        let mut both = base();
        both.blink_bits = 1.0f32.to_bits();
        both.squash_bits = 0.2f32.to_bits();
        assert_eq!(diff(&base(), &both), ChangeClass::Partial(HEAD_RECT));
        assert_eq!(head_rect_for(&base()), HEAD_RECT);
    }

    #[test]
    fn diff_phase_depends_on_gait() {
        // Phase under Stand is a dead channel: identical
        // pixels despite different keys.
        let mut p = base();
        p.phase_bits = 2.0f32.to_bits();
        assert_ne!(base(), p);
        assert_eq!(diff(&base(), &p), ChangeClass::Identical);
        // Same under Sit.
        let mut sit_a = base();
        sit_a.legs = LegsKey::Sit;
        let mut sit_b = sit_a.clone();
        sit_b.phase_bits = 2.0f32.to_bits();
        assert_eq!(diff(&sit_a, &sit_b), ChangeClass::Identical);
        // Under Walk / Run the phase drives the legs.
        let mut walk_a = base();
        walk_a.legs = LegsKey::Walk;
        let mut walk_b = walk_a.clone();
        walk_b.phase_bits = 2.0f32.to_bits();
        assert_eq!(diff(&walk_a, &walk_b), ChangeClass::Partial(LEGS_RECT));
        let mut run_a = base();
        run_a.legs = LegsKey::Run;
        let mut run_b = run_a.clone();
        run_b.phase_bits = 1.0f32.to_bits();
        assert_eq!(diff(&run_a, &run_b), ChangeClass::Partial(LEGS_RECT));
        assert_eq!(legs_rect_for(&base()), LEGS_RECT);
    }

    #[test]
    fn diff_full_repaint_group() {
        let mut cases: Vec<CharStateKey> = Vec::new();
        let mut v = base();
        v.look_idx = 3;
        cases.push(v);
        let mut v = base();
        v.kid = true;
        cases.push(v);
        let mut v = base();
        v.arm_l_bits = [20.0f32.to_bits(), 6.0f32.to_bits()];
        cases.push(v);
        let mut v = base();
        v.arm_r_bits = [8.0f32.to_bits(), 30.0f32.to_bits()];
        cases.push(v);
        let mut v = base();
        v.legs = LegsKey::Sit;
        cases.push(v);
        let mut v = base();
        v.lean_bits = 4.0f32.to_bits();
        cases.push(v);
        let mut v = base();
        v.bob_bits = 4.0f32.to_bits();
        cases.push(v);
        let mut v = base();
        v.sit_drop_bits = Some(180.0f32.to_bits());
        cases.push(v);
        let mut v = base();
        v.hold_l = Some("leaflet".to_string());
        cases.push(v);
        let mut v = base();
        v.hold_r = Some("paper".to_string());
        cases.push(v);
        // A full-group change dominates a face change.
        let mut v = base();
        v.blink_bits = 1.0f32.to_bits();
        v.arm_l_bits = [20.0f32.to_bits(), 6.0f32.to_bits()];
        cases.push(v);
        for c in &cases {
            assert_eq!(diff(&base(), c), ChangeClass::FullRepaint, "case: {c:?}");
        }
    }

    #[test]
    fn face_rect_shifts_with_the_figure() {
        // Kid: by += 190 stage px; head dy = 190*0.99*2 = 376.
        let mut kid = base();
        kid.kid = true;
        assert_eq!(face_rect_for(&kid), Rect::new(960, 620 + 376, 680, 580));
        let mut kid_blink = kid.clone();
        kid_blink.blink_bits = 1.0f32.to_bits();
        assert_eq!(
            diff(&kid, &kid_blink),
            ChangeClass::Partial(Rect::new(960, 996, 680, 580))
        );
        // Sitting at the default drop: by = (426-220.2)/0.98
        // = 210 stage px; head dy = 210*0.99*2 = 416 (rounded).
        let mut sit = base();
        sit.legs = LegsKey::Sit;
        sit.sit_drop_bits = Some(220.2f32.to_bits());
        assert_eq!(face_rect_for(&sit), Rect::new(960, 620 + 416, 680, 580));
        // Lean rotates the head about the feet: at 5 degrees
        // (the key stores lean in DEGREES, as FullState does;
        // the painter converts) the head centre moves ~143
        // canvas px in +x.
        let mut leaned = base();
        leaned.lean_bits = 5.0f32.to_bits();
        assert_eq!(face_rect_for(&leaned), Rect::new(960 + 143, 620, 680, 580));
        // Rects always stay inside the canvas, even pushed.
        let mut sunk = base();
        sunk.kid = true;
        sunk.legs = LegsKey::Sit;
        sunk.sit_drop_bits = Some(0.0f32.to_bits());
        for r in [face_rect_for(&sunk), head_rect_for(&sunk), legs_rect_for(&sunk)] {
            assert!(r.right() <= CANVAS_W && r.bottom() <= CANVAS_H, "rect escapes canvas: {r:?}");
        }
    }

    #[test]
    fn planner_orders_by_char_id() {
        let a = base();
        let mut blink = base();
        blink.blink_bits = 1.0f32.to_bits();
        let plan = MultiCharPlanner::plan(&[
            (7, Some(&a), &blink),
            (2, Some(&a), &a),
            (5, None, &a),
        ]);
        let ids: Vec<u32> = plan.plans.iter().map(|p| p.char_id).collect();
        assert_eq!(ids, vec![2, 5, 7]);
        // Deterministic: a differently-ordered input with the
        // same set yields the same plan vector.
        let plan2 = MultiCharPlanner::plan(&[
            (5, None, &a),
            (7, Some(&a), &blink),
            (2, Some(&a), &a),
        ]);
        assert_eq!(plan.plans, plan2.plans);
        // The combined set's MERGED form is the deterministic
        // figure (the unmerged storage order follows input
        // order by design — absorb is accumulation, merged()
        // is the canonical view).
        assert_eq!(plan.combined.merged(), plan2.combined.merged());
    }

    #[test]
    fn planner_classifies_and_builds_dirty_sets() {
        let a = base();
        let mut blink = base();
        blink.blink_bits = 1.0f32.to_bits();
        let mut arm = base();
        arm.arm_r_bits = [40.0f32.to_bits(), 6.0f32.to_bits()];
        let plan = MultiCharPlanner::plan(&[
            (1, Some(&a), &a),       // identical
            (2, Some(&a), &blink),   // partial face
            (3, Some(&a), &arm),     // full
            (4, None, &a),           // spawn -> full
        ]);
        assert_eq!(plan.plans[0].class, ChangeClass::Identical);
        assert!(plan.plans[0].dirty.is_empty());
        assert_eq!(plan.plans[1].class, ChangeClass::Partial(FACE_RECT));
        assert_eq!(plan.plans[1].dirty.merged(), vec![FACE_RECT]);
        assert_eq!(plan.plans[2].class, ChangeClass::FullRepaint);
        assert_eq!(plan.plans[2].dirty.merged(), vec![Rect::new(0, 0, CANVAS_W, CANVAS_H)]);
        assert_eq!(plan.plans[3].class, ChangeClass::FullRepaint);
        assert_eq!(plan.identical_count(), 1);
        assert_eq!(plan.partial_count(), 1);
        assert_eq!(plan.full_count(), 2);
        // Planned bytes: 0 + face partial + 2 full canvases.
        let face_bytes = FACE_RECT.area() * BYTES_PER_PIXEL;
        assert_eq!(plan.total_planned_upload_bytes(), face_bytes + 2 * CANVAS_BYTES);
    }

    #[test]
    fn planner_combined_union_behaviour() {
        let a = base();
        let mut blink = base();
        blink.blink_bits = 1.0f32.to_bits();
        // Two characters with the SAME partial rect: the
        // union coalesces to that one rect.
        let plan = MultiCharPlanner::plan(&[(1, Some(&a), &blink), (2, Some(&a), &blink)]);
        assert_eq!(plan.combined.merged(), vec![FACE_RECT]);
        assert!(plan.combined.coverage(CANVAS_W, CANVAS_H) < 0.10);
        // A full-canvas char absorbs a partial char's rect.
        let mut arm = base();
        arm.arm_l_bits = [30.0f32.to_bits(), 6.0f32.to_bits()];
        let plan = MultiCharPlanner::plan(&[(1, Some(&a), &blink), (2, Some(&a), &arm)]);
        assert_eq!(plan.combined.merged(), vec![Rect::new(0, 0, CANVAS_W, CANVAS_H)]);
        assert!(plan.combined.should_fall_back(CANVAS_W, CANVAS_H));
        // All-identical: the union is empty, nothing uploads.
        let plan = MultiCharPlanner::plan(&[(1, Some(&a), &a), (2, Some(&a), &a)]);
        assert!(plan.combined.is_empty());
        assert_eq!(plan.total_planned_upload_bytes(), 0);
        // Empty input is a valid, empty plan.
        let plan = MultiCharPlanner::plan(&[]);
        assert!(plan.plans.is_empty());
        assert!(plan.combined.is_empty());
    }
}
