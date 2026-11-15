// motion.js — the Tall Tale motion system (v3): pure easing / timing / follow-through helpers.
//
// PURITY RULE — read this before using anything here:
//   draw(t) in this engine is a PURE function of film time t. Capture draws arbitrary
//   times in arbitrary order (the QA sheet and the check pass jump around the film),
//   so every helper in this file MUST be a pure function of its arguments — usually
//   of t (or of a local time lt / a normalised progress u derived from t).
//   NO stateful integrators, NO closures that mutate across frames, NO "previous
//   value" anywhere. Follow-through and lag are ANALYTIC: a damped-oscillation
//   response after an event time t0 (see settle / springStep), never simulated.
//
// BEAT RULE:
//   An action's PEAK lands ON its spoken word, so the action STARTS early.
//   Use beatStart(wordTime) to start 3–5 frames before the word, give the move an
//   anticipation + an overshoot / snap curve, then HOLD the resulting pose for
//   6–10 frames (poseTrack is the hold-move-hold primitive). One primary action
//   per shot; ~60–70% of a shot is a held pose; the camera holds at both ends
//   (camK) and never drifts at a constant rate.
//
// Builds on lib.js (clamp / lerp / seg). Curves take a normalised progress
// u in [0, 1] unless their docs say otherwise, and clamp u themselves.
import { clamp, lerp, rnd } from './lib';
import type { Mood, ToonState } from './toon';

/**
 * smootherstep(u) — quintic smootherstep: zero slope at both ends, the softest
 * standard ease. The general-purpose "move and land without a bump" curve.
 * Usage: `const k = smootherstep(seg(lt, 0.2, 0.8)); arm = lerp(20, 90, k);`
 */
export function smootherstep(u: number) {
  u = clamp(u, 0, 1);
  return u * u * u * (u * (u * 6 - 15) + 10);
}

/**
 * camK(lt, d, hold = 0.1) — camera progress 0..1 for a shot of duration d at
 * local time lt: HOLDS at 0 for the first `hold` fraction of the shot, moves
 * with smootherstep through the middle, and HOLDS at 1 for the last `hold`
 * fraction. Cameras always enter and leave a shot at rest — never drift.
 * Usage: `const k = camK(lt, d); shoot(scene, cast, from, to, at, null, k, LOOK);`
 */
export function camK(lt: number, d: number, hold = 0.1) {
  const h = clamp(hold, 0, 0.45);
  const span = 1 - 2 * h;
  return smootherstep(((d > 0 ? lt / d : 1) - h) / (span || 1e-6));
}

/**
 * anticipate(u, dip = 0.18) — anticipation curve: for u in [0, 0.3] it eases
 * BACKWARD to -dip, then rises on a smootherstep leg to 1 at u = 1. The return
 * value is < 0 early BY DESIGN — lerp with it directly (do not clamp it), so
 * the lerped value visibly moves the wrong way before the real move: a crouch
 * before a jump, an arm winding back before a sweep, shoulders dipping before
 * a shrug.
 * Usage: `arm = lerp(30, 138, anticipate(seg(lt, 0.1, 0.7)));`
 */
export function anticipate(u: number, dip = 0.18) {
  u = clamp(u, 0, 1);
  if (u < 0.3) return -dip * smootherstep(u / 0.3);
  return lerp(-dip, 1, smootherstep((u - 0.3) / 0.7));
}

/**
 * overshoot(u, c = 1.35) — easeOutBack-style curve (proven on a real
 * episode as the house `over` curve): races past 1, peaks at roughly 1 + a few % (c controls the excess;
 * 1.35 is the house value, gentler than lib.js eob's 1.70158), then settles
 * back to exactly 1. For arrivals that should feel alive: a raised held prop, a
 * shrug, a pile popping in, a character scooped up.
 * Usage: `const raise = overshoot(seg(t, wHit - 0.52, wHit - 0.02));`
 */
export function overshoot(u: number, c = 1.35) {
  u = clamp(u, 0, 1);
  return 1 + (c + 1) * Math.pow(u - 1, 3) + c * Math.pow(u - 1, 2);
}

/**
 * snap(u) — easeOutQuart strike: extremely fast start, long decelerating tail.
 * For strikes and hits whose PEAK must land on a word: a swing coming down, a
 * stamp slamming, a peck strike. Pair with a short anticipate before it.
 * Usage: `armAng = lerp(170, 60, snap(seg(t, wDown - 0.15, wDown + 0.02)));`
 */
export function snap(u: number) {
  return 1 - Math.pow(1 - clamp(u, 0, 1), 4);
}

/**
 * softLand(u) — very gentle ease-out for camera stops and quiet settles:
 * 1 - (1-u)^5, a fast-ish start that feathers into the landing with zero
 * slope at u = 1 and no overshoot at all. Softer and calmer than snap; use it
 * where overshoot would be wrong (a lean-in to inspect something, a head
 * settling onto a pillow, the last bit of a crane move done by hand).
 * Usage: `lean = 7 * softLand(seg(lt, 0, 0.55));`
 */
export function softLand(u: number) {
  return 1 - Math.pow(1 - clamp(u, 0, 1), 5);
}

/**
 * settle(t, t0, freq = 6, decay = 5) — damped oscillation AFTER an event at
 * time t0 (the follow-through channel): 0 for t <= t0, otherwise
 * sin((t - t0) * freq * 2π) * exp(-(t - t0) * decay). Starts at 0, swings
 * positive first, and decays to nothing. Apply it, scaled small, to a
 * SECONDARY value after a peak: bob, lean, a hat's squash, a held prop's
 * angle — never to the primary motion itself.
 * This generalises a `pulse` helper proven on a real episode
 * (sin((t - tc) * 17) * exp(-(t - tc) * 5.5)): that exact feel is
 * freq ≈ 17 / (2π) ≈ 2.7, decay 5.5. The defaults (6 Hz, decay 5) are a
 * brisker, shorter wobble — pass freq ≈ 2.7 for that episode feel.
 * Usage: `armAng = landedAngle + settle(t, wDown + 0.02) * 4;`
 */
export function settle(t: number, t0: number, freq = 6, decay = 5) {
  const dt = t - t0;
  if (dt <= 0) return 0;
  return Math.sin(dt * freq * 2 * Math.PI) * Math.exp(-dt * decay);
}

/**
 * springStep(t, t0, { freq = 2.2, decay = 3.5 } = {}) — analytic underdamped
 * step response: exactly 0 before t0 and at t0, then rises PAST 1 (overshoot)
 * and rings down to exactly 1:
 *   1 - exp(-decay*dt) * (cos(w*dt) + (decay/w) * sin(w*dt)),  w = 2π * freq.
 * Use it for a value that should ARRIVE with spring lag rather than on a
 * fixed-duration ease: a prop appearing / popping to full size, a hat
 * bouncing via squash, a held prop's angle lagging the arm that carries it
 * (drive the arm with its ease, drive the prop angle with springStep off the
 * same t0). Requires decay < w to overshoot; the defaults ring visibly.
 * Usage: `pile.scale.setScalar(0.55 + 0.45 * springStep(t, wPop - 0.05));`
 */
export function springStep(t: number, t0: number, { freq = 2.2, decay = 3.5 }: { freq?: number; decay?: number } = {}) {
  const dt = t - t0;
  if (dt <= 0) return 0;
  const w = 2 * Math.PI * freq;
  return 1 - Math.exp(-decay * dt) * (Math.cos(w * dt) + (decay / w) * Math.sin(w * dt));
}

/**
 * arc(p0, p1, lift, u) — quadratic Bézier between two 2D points [x, y] at
 * progress u. The control point is the midpoint raised by `lift` in -y:
 * in screen / canvas space up is -y, so a POSITIVE lift arcs the path upward
 * (toward smaller y); a negative lift sags it downward. `lift` is in the
 * points' own units. Hands and props travel on arcs, never straight lines.
 * Usage: `const [hx, hy] = arc([40, 300], [300, 180], 60, smootherstep(u));`
 */
export function arc(p0: readonly [number, number], p1: readonly [number, number], lift: number, u: number): [number, number] {
  u = clamp(u, 0, 1);
  const cx = (p0[0] + p1[0]) / 2, cy = (p0[1] + p1[1]) / 2 - lift;
  const a = (1 - u) * (1 - u), b = 2 * (1 - u) * u, c = u * u;
  return [a * p0[0] + b * cx + c * p1[0], a * p0[1] + b * cy + c * p1[1]];
}

/**
 * arcV(a, b, lift, u, up = 'y') — arc() for plain {x, y, z} objects (wrap the
 * result in V3 for three.js): quadratic Bézier whose control point is the
 * midpoint offset by `lift` along the `up` axis. Unlike arc(), world space is
 * Y-up, so the default up = 'y' with a positive lift RAISES the path (+y).
 * Pass up = '-y' to sag, or 'x' / 'z' (optionally signed) for a sideways bow.
 * Returns a NEW plain object; a and b are not mutated.
 * Usage: `obj.position.copy(V3(...Object.values(arcV(p0, p1, 0.4, k))));`
 *   — or explicitly: `const p = arcV(p0, p1, 0.4, k); obj.position.set(p.x, p.y, p.z);`
 */
export interface Vec3Like { x: number; y: number; z: number }
export function arcV(a: Vec3Like, b: Vec3Like, lift: number, u: number, up = 'y'): Vec3Like {
  u = clamp(u, 0, 1);
  const neg = typeof up === 'string' && up.startsWith('-');
  const axis = (neg ? up.slice(1) : up) || 'y';
  const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2, z: (a.z + b.z) / 2 };
  if (axis === 'x' || axis === 'y' || axis === 'z') mid[axis] += (neg ? -lift : lift);
  const q = (1 - u) * (1 - u), r = 2 * (1 - u) * u, s = u * u;
  return {
    x: q * a.x + r * mid.x + s * b.x,
    y: q * a.y + r * mid.y + s * b.y,
    z: q * a.z + r * mid.z + s * b.z,
  };
}

/**
 * poseTrack(keys, t) — hold-move-hold, the "pose, hold, move on the beat"
 * primitive. keys = [[time, value, moveSecs?, curve?], ...] sorted by time.
 * Semantics: the move INTO a key happens over that key's moveSecs, ENDING at
 * that key's time — so the value HOLDS at key i's value until
 * (key i+1's time - key i+1's moveSecs), then moves to key i+1's value over
 * moveSecs using key i+1's curve (default smootherstep). moveSecs defaults to
 * 0.35. Before the first key → the first value; after the last key → the last
 * value (the final pose holds forever, which is what an outro wants).
 * Usage (shrug starts moving at 0.25, arrives at 0.6, then holds):
 *   `const shrug = poseTrack([[0, 0], [0.6, 1, 0.35]], lt);`
 */
export type PoseKey = readonly [number, number, number?, ((u: number) => number)?];
export function poseTrack(keys: ReadonlyArray<PoseKey>, t: number) {
  if (!keys || keys.length === 0) return 0;
  if (t <= keys[0][0] || keys.length === 1) return keys[0][1];
  for (let i = 0; i < keys.length - 1; i++) {
    const next = keys[i + 1];
    const moveSecs = next[2] ?? 0.35;
    const curve = typeof next[3] === 'function' ? next[3] : smootherstep;
    const start = next[0] - moveSecs;
    if (t < start) return keys[i][1];
    if (t < next[0]) {
      const u = moveSecs > 0 ? (t - start) / moveSecs : 1;
      return lerp(keys[i][1], next[1], curve(clamp(u, 0, 1)));
    }
  }
  return keys[keys.length - 1][1];
}

/**
 * beatStart(wordTime, framesEarly = 4, fps = 30) — the time an action must
 * START so its peak lands on the spoken word at wordTime: start `framesEarly`
 * frames (house rule: 3–5) before the word. Feed the result into seg() as the
 * start of the move and the word time (or just after it) as the peak.
 * Usage: `const raise = overshoot(seg(t, beatStart(wHit, 5), wHit));`
 */
export function beatStart(wordTime: number, framesEarly = 4, fps = 30) {
  return wordTime - framesEarly / (fps || 30);
}

/**
 * walkPhase(distance, stride = 0.62) — walk-cycle phase in radians for a
 * walker who has travelled `distance` metres: phase = distance / stride * π
 * (one π = one step; a full 2π cycle = 2 steps = 2 * stride metres), matching
 * drawToon's sin(phase) leg swing. ALWAYS derive phase from DISTANCE
 * TRAVELLED (advance the walker's position by speed * shot progress, then
 * convert that distance), never from t — phase-from-t makes feet slide
 * whenever speed and stride disagree. Stride guidance: stride ≈ 0.35 × the
 * character's height in metres (≈ 0.62 for a 1.75 m adult, ≈ 0.49 for a
 * 1.4 m kid) — pass it per character.
 * Usage: `const dist = speed * lt; walker.at(x0 + dist, 0, z); const phase = walkPhase(dist, 0.35 * height);`
 */
export function walkPhase(distance: number, stride = 0.62) {
  return (distance / (stride || 0.62)) * Math.PI;
}

/**
 * walkBob(phase, amp = 6) — vertical bob in px for drawToon's `bob` state,
 * paired with walkPhase: bob = amp * (1 - |sin(phase)|). drawToon's bob ADDS
 * downward px to the body, and a walking body is LOWEST as the feet pass each
 * other (sin(phase) ≈ 0) and HIGHEST when the legs are split (|sin| = 1) —
 * so bob peaks (body lowest) at passing and is 0 at full split. It bobs twice
 * per 2π cycle, once per step, as a real walk does.
 * Usage: `toon.set({ legs: 'walk', phase, bob: walkBob(phase) });`
 */
export function walkBob(phase: number, amp = 6) {
  return amp * (1 - Math.abs(Math.sin(phase)));
}

// Base blink intervals (seconds), proven on a real episode: deliberately
// irregular — never metronomic, and (via the seed) never in sync across cast.
const BLINK_IV = [2.7, 3.9, 3.1, 4.4, 2.9, 3.6];

/**
 * blinkDirector(seed = 0) — returns a pure blink(t) function (1 while the
 * eyes are shut, else 0; blink width 0.12 s) for ONE character. The interval
 * pattern [2.7, 3.9, 3.1, 4.4, 2.9, 3.6] s is rotated by the seed (starting
 * index = floor(seed * 7) mod 6, as proven on a real episode) and the clock is
 * offset by seed * 1.37 s, so two characters with different seeds never blink
 * in sync. Give every character their own seed and keep it for the whole film.
 * The returned function is still pure: it reads only t and the frozen seed.
 * Usage: `const blinkA = blinkDirector(0); const blinkB = blinkDirector(1);
 *         a.set({ blink: blinkA(t) }); b.set({ blink: blinkB(t) });`
 */
export function blinkDirector(seed = 0): (t: number) => number {
  const s = Number.isFinite(seed) ? seed : 0;
  const offset = s * 1.37;
  const startIdx = ((Math.floor(s * 7) % BLINK_IV.length) + BLINK_IV.length) % BLINK_IV.length;
  return function blink(t: number) {
    const x = t + offset;
    if (!(x >= 0)) return 0;
    let acc = 0, i = startIdx;
    // Walk the repeating interval pattern up to x (bounded: ~1 iteration per
    // ~3.4 s of film time). All state is local — nothing survives the call.
    while (x > acc + BLINK_IV[i % BLINK_IV.length]) {
      acc += BLINK_IV[i % BLINK_IV.length];
      i++;
    }
    return (x - acc) < 0.12 ? 1 : 0;
  };
}

/**
 * blinkAt(t, beats) — deliberate blink ON a thought beat: 1 if t is within
 * 0.12 s after any time in the `beats` array (e.g. the moment a character
 * decides, realises, or lands a sly look), else 0. Combine with a character's
 * ambient blink by taking the max of the two.
 * Usage: `blink: Math.max(blinkA(t), blinkAt(t, [wRealise, wDecide]))`
 */
export function blinkAt(t: number, beats?: ReadonlyArray<number> | null) {
  if (!beats) return 0;
  for (let i = 0; i < beats.length; i++) {
    const dt = t - beats[i];
    if (dt >= 0 && dt < 0.12) return 1;
  }
  return 0;
}

// ---------------------------------------------------------------------------
// Camera rigs — named camera moves that return a whole frame at once.
//
// A rig returns a RigFrame { pos, look, fov? }: the camera position, the
// point it aims at, and (only when the move changes it) the field of view.
// Shots feed the frame straight to their shoot() / camera code instead of
// hand-rolling separate from/to lerps for position and look. Rigs are
// THREE-free — plain Vec3Like in, plain Vec3Like out — and pure like
// everything else in this file: same k in, same frame out, inputs never
// mutated, every returned object newly allocated.
//
// Always drive a rig with an eased k from camK (k = camK(lt, d)), never
// with raw shot progress, so rigs hold at both ends like every other
// camera move in the system: the camera enters and leaves the shot at
// rest, moves through the middle, and never drifts at a constant rate.
// Usage: `const k = camK(lt, d);
//         const f = crane(from, to, at, at2, k);
//         shoot(scene, cast, f.pos, null, f.look, null, 1, LOOK, f.fov);`
// ---------------------------------------------------------------------------

/**
 * RigFrame — one camera frame produced by a rig helper: `pos` is the
 * camera position, `look` is the point the camera aims at, and `fov`
 * (degrees) is present only when the move changes the field of view
 * (dollyZoom); rigs that leave fov alone omit it so the caller keeps
 * whatever fov the shot already uses.
 */
export interface RigFrame { pos: Vec3Like; look: Vec3Like; fov?: number }

function lerpV(a: Vec3Like, b: Vec3Like, k: number): Vec3Like {
  return { x: lerp(a.x, b.x, k), y: lerp(a.y, b.y, k), z: lerp(a.z, b.z, k) };
}

/**
 * dollyZoom(from, to, look, k, fov0, fov1) — the vertigo pair: the camera
 * position lerps from → to while the field of view lerps fov0 → fov1,
 * with `look` fixed (returned as a copy). Dolly in while zooming out (or
 * the reverse) and a subject framed on `look` stays the SAME SIZE on
 * screen while the world behind it breathes — the background stretches
 * or compresses around a locked subject. Choose from/to and fov0/fov1
 * as a matched pair: the further the camera moves, the wider the fov
 * swing needed to hold the subject's size. Both lerps are linear on k;
 * pass a camK-eased k so the move holds at both ends.
 * Usage: `const f = dollyZoom(camA, camB, heroPos, camK(lt, d), 36, 55);`
 */
export function dollyZoom(from: Vec3Like, to: Vec3Like, look: Vec3Like, k: number, fov0: number, fov1: number): RigFrame {
  const kk = clamp(k, 0, 1);
  return {
    pos: lerpV(from, to, kk),
    look: { x: look.x, y: look.y, z: look.z },
    fov: lerp(fov0, fov1, kk),
  };
}

/**
 * orbit(centre, radius, height, a0, a1, k, lookAt?) — the camera travels
 * the circle of `radius` around `centre`: the angle lerps a0 → a1
 * (radians) on k, and pos = { x: centre.x + sin(a) * radius, y: height,
 * z: centre.z + cos(a) * radius }, where `height` is the ABSOLUTE camera
 * height (world y), not an offset above centre. look = lookAt ?? centre
 * (returned as a copy either way), so by default the camera keeps the
 * centre of the circle framed while it swings around it. A quarter turn
 * is π/2 of angle; keep the swing modest inside one shot — a full
 * revolution reads as a different shot, not a move.
 * Usage: `const f = orbit(heroPos, 6, 2.2, -0.6, 0.6, camK(lt, d));`
 */
export function orbit(centre: Vec3Like, radius: number, height: number, a0: number, a1: number, k: number, lookAt?: Vec3Like): RigFrame {
  const kk = clamp(k, 0, 1);
  const a = lerp(a0, a1, kk);
  const lookSrc = lookAt ?? centre;
  return {
    pos: { x: centre.x + Math.sin(a) * radius, y: height, z: centre.z + Math.cos(a) * radius },
    look: { x: lookSrc.x, y: lookSrc.y, z: lookSrc.z },
  };
}

/**
 * crane(from, to, lookFrom, lookTo, k) — the general move: position and
 * look both lerp linearly on k. This is exactly the from/to + at/at2
 * pattern shots already hand-roll, named: a rise that tilts down as it
 * climbs, a push-in that reframes from one character to another, any
 * move where where the camera IS and where it LOOKS travel together.
 * The helper itself does not ease — callers pass a camK-eased k so the
 * move holds at both ends.
 * Usage: `const f = crane(lowWide, highTight, atStart, atEnd, camK(lt, d));`
 */
export function crane(from: Vec3Like, to: Vec3Like, lookFrom: Vec3Like, lookTo: Vec3Like, k: number): RigFrame {
  const kk = clamp(k, 0, 1);
  return { pos: lerpV(from, to, kk), look: lerpV(lookFrom, lookTo, kk) };
}

/**
 * trackAhead(subjFrom, subjTo, offset, lead, k) — tracking shot with
 * look-ahead: the subject position subj(k) lerps subjFrom → subjTo on k,
 * the camera rides at pos = subj(k) + offset (offset in world axes,
 * added component-wise — a fixed side / front / high perch relative to
 * the subject's path), and the camera aims at look = subj(clamp(k +
 * lead, 0, 1)), where the subject is ABOUT to be, so the subject sits
 * slightly behind the frame's centre of attention and the shot feels
 * led rather than dragged. lead ~0.08–0.15 is typical; lead 0 aims
 * exactly at the subject. k is clamped to [0, 1] and the look-ahead
 * target is clamped too, so the aim settles on subjTo at the end.
 * Usage: `const f = trackAhead(walkA, walkB, { x: 3, y: 0.6, z: 4 }, 0.12, camK(lt, d));`
 */
export function trackAhead(subjFrom: Vec3Like, subjTo: Vec3Like, offset: Vec3Like, lead: number, k: number): RigFrame {
  const kk = clamp(k, 0, 1);
  const subj = lerpV(subjFrom, subjTo, kk);
  return {
    pos: { x: subj.x + offset.x, y: subj.y + offset.y, z: subj.z + offset.z },
    look: lerpV(subjFrom, subjTo, clamp(kk + lead, 0, 1)),
  };
}

// ---------------------------------------------------------------------------
// Acting cues (v5, D4 §2–§3) — mood transitions, reactions, lip-sync-lite.
//
// Faces are DISCRETE drawings: drawFace cannot interpolate a brow path into a
// spiral eye, so an in-shot mood change is the cut-out animator's real
// technique — swap the mood during a forced blink, with a small squash /
// head-dip accent masking the swap frame (moodCue). reactCue builds the
// genre's core move, the word-anchored double-take, on the same masking idea.
// talkTrack drives the existing `talk` state field from word timings for
// ON-CAMERA lines only (narration is voice-over; faces pantomime it — driving
// talk under VO is a rubric fault). All helpers are pure in t, like everything
// in this file: same inputs in, same bundle out, nothing mutated.
// ---------------------------------------------------------------------------

/** one moodCue key: [time, mood, maskSecs?] — maskSecs defaults to 0.16 s */
export type MoodKey = readonly [number, Mood, number?];
export interface MoodCue { mood: Mood; blink: number; squash: number; headTilt: number }

/**
 * moodCue(keys, t) — a masked mood track: a per-character mood arc as ONE
 * auditable key list. `mood` = the last key with time <= t (the first key's
 * mood before it). Around each later key, a mask window of key[2] ?? 0.16 s
 * centred on the key time: inside it `blink` is a triangle peaking at exactly
 * 1 AT the key time (eyes shut on the swap instant), `squash` dips to -0.02
 * and `headTilt` dips +2° at the key time; all three are 0 outside every
 * window; overlapping windows take the max. Keys may be given unsorted.
 * Usage: `const mc = moodCue([[0, 'smug'], [Wd('down'), 'shock'], [Wd('gone'), 'sad']], lt);
 *         p.set({ ...POSES.explain, mood: mc.mood, blink: Math.max(blinkA(lt), mc.blink),
 *                 squash: mc.squash, headTilt: mc.headTilt });`
 */
export function moodCue(keys: ReadonlyArray<MoodKey>, t: number): MoodCue {
  if (!keys || keys.length === 0) return { mood: 'neutral', blink: 0, squash: 0, headTilt: 0 };
  const sorted = [...keys].sort((a, b) => a[0] - b[0]);
  let mood = sorted[0][1];
  for (const k of sorted) if (t >= k[0]) mood = k[1];
  let blink = 0, squash = 0, headTilt = 0;
  for (let i = 1; i < sorted.length; i++) {
    const mask = sorted[i][2] ?? 0.16, half = mask / 2, dt = t - sorted[i][0];
    if (half > 0 && Math.abs(dt) <= half) {
      const tri = 1 - Math.abs(dt) / half;
      blink = Math.max(blink, tri);
      squash = Math.min(squash, -0.02 * tri);
      headTilt = Math.max(headTilt, 2 * tri);
    }
  }
  return { mood, blink, squash, headTilt };
}

export interface ReactOpts { mood?: Mood; dir?: 1 | -1; takes?: 1 | 2; hold?: number }

/**
 * reactCue(t, tHit, opts) — the double-take, anchored ON a spoken word: call
 * it with tHit = the word time itself (the anticipation is inside the helper)
 * and spread the result LAST into set(). Timeline (dt = t - tHit, dir = the
 * side the subject is on, mood defaults to 'shock'):
 *   dt < -0.10        {} — no effect
 *   -0.10 <= dt < 0   gaze drifts AWAY (look/headTurn negative) so the snap back reads
 *   0 <= dt < 0.06    { blink: 1 } — the masked flip; NO mood yet, it lands as eyes open
 *   dt >= 0.06        the reaction hold: mood, gaze snapped toward the subject,
 *                     squash ringing via settle(t, tHit, 2.7, 5.5) (the episode-proven wobble)
 *   takes: 2          adds the classic second take: at dt in [0.34, 0.46] the gaze flicks
 *                     away again (blink 1 at dt ≈ 0.40), then returns at 0.46 with a
 *                     half-size squash settle
 *   after `hold` s    {} again (default hold = ∞ — the shot owns the exit)
 * Usage: `p.set({ ...base, ...reactCue(t, Wd('down'), { dir: -1 }) })`.
 */
export function reactCue(t: number, tHit: number, opts: ReactOpts = {}): Partial<ToonState> {
  const dt = t - tHit, dir = opts.dir ?? 1, mood = opts.mood ?? 'shock', hold = opts.hold ?? Infinity;
  if (dt < -0.10) return {};
  if (dt < 0) return { look: [-0.6 * dir, 0], headTurn: -0.25 * dir };
  if (dt < 0.06) return { blink: 1 };
  if (dt > hold) return {};
  const out: Partial<ToonState> = { mood, look: [0.7 * dir, 0], headTurn: 0.15 * dir, squash: 0.05 * settle(t, tHit, 2.7, 5.5) };
  if ((opts.takes ?? 1) === 2) {
    if (dt >= 0.46) out.squash = 0.025 * settle(t, tHit + 0.46, 2.7, 5.5);
    else if (dt >= 0.34) {
      out.look = [-0.5 * dir, 0]; out.headTurn = -0.15 * dir;
      if (dt >= 0.395 && dt < 0.425) out.blink = 1;
    }
  }
  return out;
}

/**
 * crowdReact(t, tHit, seed, opts) — crowd reaction ripple: reactCue delayed
 * by rnd(seed) * 0.18 s with the squash scaled 0.6, so a crowd ripples instead
 * of snapping in unison. Give each crowd member a different seed; seed-equal
 * calls stay available when unison IS the gag.
 * Usage: `extras.forEach((p, i) => p.set({ ...base, ...crowdReact(t, wHit, i + 1) }))`.
 */
export function crowdReact(t: number, tHit: number, seed: number, opts: ReactOpts = {}): Partial<ToonState> {
  const out = reactCue(t, tHit + rnd(seed) * 0.18, opts);
  if (typeof out.squash === 'number') out.squash *= 0.6;
  return out;
}

/** one timed word for talkTrack: timing.ts TimingWord ({w, line, s, e}) is a superset */
export interface TalkWord { s: number; e: number; line?: number }

/**
 * talkTrack(words, t, opts) — lip-sync-lite: a `talk` value (0..~0.85) derived
 * from word TIMINGS alone (words.json carries no energy data). 0 outside every
 * word and in the gaps between words. Inside a word [s, e]: syllable pulses at
 * one per `rate` s (default 0.14), shaped by a sin(πu)^0.5 envelope so the
 * mouth closes at word boundaries; output quantised to 0.05 steps so Toon#set
 * repaint keys stay few while reading identically. `opts.line` filters to one
 * line's words — the spoken line. Apply ONLY to the character delivering an
 * on-camera line, for that line's span, and pair with a mouth that talk can
 * open (neutral/happy/smug/worried — with laugh/awe the mouth is fixed-shape
 * and talk is inert).
 * Usage: `p.set({ mood: 'happy', talk: talkTrack(T.words, t, { line: 7 }) })`.
 */
export function talkTrack(words: ReadonlyArray<TalkWord>, t: number, opts: { line?: number; gain?: number; rate?: number } = {}): number {
  if (!words) return 0;
  const gain = opts.gain ?? 0.85, rate = opts.rate ?? 0.14;
  for (const w of words) {
    if (opts.line != null && w.line !== opts.line) continue;
    if (t < w.s || t > w.e) continue;
    const dur = w.e - w.s;
    if (!(dur > 0)) return 0;
    const u = (t - w.s) / dur;
    const beats = Math.max(1, Math.round(dur / (rate || 0.14)));
    const v = gain * Math.abs(Math.sin(Math.PI * u * beats)) * Math.pow(Math.sin(Math.PI * u), 0.5);
    return Math.round(v * 20) / 20;
  }
  return 0;
}

/** speakingAt(words, t) — true while t falls inside any word's [s, e] span. */
export function speakingAt(words: ReadonlyArray<TalkWord>, t: number): boolean {
  if (!words) return false;
  return words.some((w) => t >= w.s && t <= w.e);
}
