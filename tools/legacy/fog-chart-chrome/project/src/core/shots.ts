// core/shots.ts — the shot grammar: cut points, the shoot() camera/render
// orchestrator, and (v5) the named-shot catalog. Factored verbatim out of the
// template example (ex-shots.ts) so every project shares one grammar instead
// of re-deriving it from the example. Purity: same rule as core/motion —
// everything here is a pure function of its arguments / film time;
// createShoot closes over the engine + camera it is given, nothing else.
//
// The one call pattern for every catalog helper (helpers are pure, THREE-free
// Vec3Like in/out, and return a RigFrame):
//   const f = punchIn(a, b, at, t, Wd('kid', 8));
//   shoot(scene, cast, V3(f.pos), V3(f.pos), V3(f.look), null, 1,
//         { ...LOOK, ...(f.fov ? { fov: f.fov } : {}) });
//
// Unanchored helpers take a caller-eased k (camK) — rigs never ease, and
// neither do these, except where the helper's whole point is its internal
// timing (smashIn, whipPan, the anchored helpers), stated per entry.
// Word-anchored helpers take the absolute film time t (draw's third argument)
// and a word time from Wd(): their move ARRIVES exactly on the word.
import type * as THREE from 'three';
import { L } from './timing';
import { clamp } from './lib';
import { crane, orbit, snap, smootherstep, type RigFrame, type Vec3Like } from './motion';
import type { Engine, RenderLook } from './engine';
import type { Toon } from './toon';

export interface Shot { id: string; at: number; note: string; draw: (lt: number, d: number, t: number, blit: () => void) => void }
export interface Cue { t: number; type: string; gain?: number; dur?: number }
/** roll (radians, opt-in) — applied to the camera by shoot() after lookAt;
 *  absent/0 = the v4 behaviour exactly. Sign: roll > 0 rotates the camera
 *  counter-clockwise around its view axis, so the horizon appears to tip
 *  clockwise — its right side drops. (Pinned by the v5 B7 probe sheet.) */
export type ShotLook = RenderLook & { fov?: number; roll?: number };

/** cut(i) — the cut point for line i: the line's start, 0.06 s early so the
 *  new shot is settled as the line begins; line 0 cuts at 0. */
export const cut = (i: number): number => (i === 0 ? 0 : L(i) - 0.06);

export type ShootFn = (
  scene: THREE.Scene, people: Toon[], from: THREE.Vector3, to: THREE.Vector3,
  at: THREE.Vector3, at2: THREE.Vector3 | null, k: number, look: ShotLook,
  focus?: number,
) => void;

/**
 * createShoot(E, cam, { baseFov = 36 }) — builds the shoot() every shot calls:
 * sets the camera fov (look.fov, else baseFov — 36 is the template's house
 * default, NOT engine.camera()'s 30), moves the camera from→to by k, aims it
 * at at→at2 by k, faces every person to the camera, and renders with the
 * shot's look; focus defaults to the camera→target distance.
 * The body is the template's original shoot() arithmetic, unchanged and in
 * the same order; the only addition is the opt-in roll branch, not taken
 * when look.roll is absent/0.
 */
export function createShoot(E: Engine, cam: THREE.PerspectiveCamera, opts: { baseFov?: number } = {}): ShootFn {
  const baseFov = opts.baseFov ?? 36;
  const mix = (a: THREE.Vector3, b: THREE.Vector3, k: number) => a.clone().lerp(b, k);
  return (scene, people, from, to, at, at2, k, look, focus) => {
    const fov = look.fov || baseFov;
    if (cam.fov !== fov) { cam.fov = fov; cam.updateProjectionMatrix(); }
    cam.position.copy(mix(from, to, k));
    const tgt = at2 ? mix(at, at2, k) : at;
    cam.lookAt(tgt);
    if (look.roll) cam.rotateZ(look.roll);
    people.forEach((p) => p.face(cam));
    E.render(scene, cam, { ...look, focus: focus ?? cam.position.distanceTo(tgt) });
  };
}

/** anchored(t, wordAt, dur, curve) — progress that is 0 until wordAt - dur,
 *  rises on `curve`, and ARRIVES (exactly 1) at wordAt, holding 1 after.
 *  The beat rule (STYLE §6) as a camera primitive: the move's peak lands ON
 *  the word by construction. */
export function anchored(t: number, wordAt: number, dur = 0.3, curve: (u: number) => number = snap): number {
  return curve(clamp((t - (wordAt - dur)) / (dur || 1e-6), 0, 1));
}

/** headY(height, kid = false) — world height of the head centre above the
 *  feet line: (1000-178) stage units × H/1024 for adults; kids' heads sit
 *  lower in the stage (drawToon by = 190 → centre (1000-366) units). */
export function headY(height: number, kid = false): number {
  return (kid ? 634 : 822) * (height / 0.96) / 1024;
}

// ── the named-shot catalog ───────────────────────────────────────────────

/** smashIn(from, to, look, lt, landSecs = 0.5) — hook-smash-in: a crash push
 *  that lands by landSecs and then HOLDS for the rest of the shot (the hold
 *  is what separates it from a zoom). Beat: shot 1 of the film and the
 *  punchline return — at most twice per film. FOV 36–44, 7–9 m → 3.5–4.5 m. */
export function smashIn(from: Vec3Like, to: Vec3Like, look: Vec3Like, lt: number, landSecs = 0.5): RigFrame {
  return crane(from, to, look, look, snap(clamp(lt / (landSecs || 1e-6), 0, 1)));
}

/** slowPush(from, to, look, k) — the slow push under its intent name.
 *  Contract: caller passes k = camK(lt, d, 0.15); end/start distance ratio
 *  0.70–0.85; shot duration ≥ 2 s. Beat: explanation stretches, intimacy
 *  build, the serious tone's default move. FOV 36 (44 if the subject also
 *  travels laterally), 5.5 → 4 m typical. */
export function slowPush(from: Vec3Like, to: Vec3Like, look: Vec3Like, k: number): RigFrame {
  return crane(from, to, look, look, k);
}

/** whipPan(centre, radius, height, a0, a1, lt, dur = 0.25) — orbit geometry
 *  with the angle driven internally by snap: the swing completes inside dur,
 *  then holds. Swing 0.5–1.2 rad. Beat: an intra-line change of place inside
 *  one continuous space ("meanwhile…"), comic teleport. FOV 44–55, radius
 *  4–7 m. Honest limits: the engine has no motion blur — speed (≤ 0.3 s)
 *  plus a `whoosh` cue at lt = 0 sells it; both ends must be pre-staged in
 *  the same set, within the ±60° facing margin at both ends. */
export function whipPan(centre: Vec3Like, radius: number, height: number, a0: number, a1: number, lt: number, dur = 0.25): RigFrame {
  const a = a0 + (a1 - a0) * snap(clamp(lt / (dur || 1e-6), 0, 1));
  return { pos: { x: centre.x + Math.sin(a) * radius, y: height, z: centre.z + Math.cos(a) * radius }, look: { x: centre.x, y: centre.y, z: centre.z } };
}

/** reveal(from, to, lookFrom, lookTo, t, wordAt, dur = 0.55) — dolly reveal:
 *  the camera travels (usually sideways, clearing an occluder or the frame
 *  edge) and the subject is FULLY revealed exactly on its word. Ends 3–5 m
 *  with ≥ 1.2 m of lateral travel (less never clears an occluder).
 *  Usage: reveal(a, b, la, lb, t, Wd('banned', 17)). FOV 36–50. */
export function reveal(from: Vec3Like, to: Vec3Like, lookFrom: Vec3Like, lookTo: Vec3Like, t: number, wordAt: number, dur = 0.55): RigFrame {
  return crane(from, to, lookFrom, lookTo, anchored(t, wordAt, dur, smootherstep));
}

/** punchIn(from, to, at, t, wordAt, dur = 0.28) — the classic punch-in,
 *  arriving exactly on the word: reveal-word emphasis and the deadpan
 *  micro-beat after the biggest reveal. End = 0.55–0.70 × start distance.
 *  FOV 36 fixed (a fov change inside a punch reads as a zoom fault). */
export function punchIn(from: Vec3Like, to: Vec3Like, at: Vec3Like, t: number, wordAt: number, dur = 0.28): RigFrame {
  return crane(from, to, at, at, anchored(t, wordAt, dur, snap));
}

/** overShoulder(fg, target, opts) — camera 1.0–1.6 m behind the foreground
 *  figure, aimed over their shoulder at the target. Beat: dialogue exchange,
 *  "he showed him…", a confrontation. FOV 44–55; fg→target 2.5–5 m.
 *  LIMITATION: a Toon is a camera-facing plane — there is no back of the
 *  head to shoot over. The working substitute is composition: the fg figure
 *  large and cropped at the frame's lower corner, focus thrown to the
 *  target (shoot's explicit focus + aperture). Staging contract: EXCLUDE
 *  the fg Toon from shoot's people list (or zero its pivot per STYLE §12)
 *  and pose it with headTurn toward the target — otherwise face() turns it
 *  to the lens and the shot reads as a two-shot, not an over-shoulder. */
export function overShoulder(fg: Vec3Like, target: Vec3Like, opts: { side?: number; back?: number; across?: number; camY?: number; lookY?: number } = {}): RigFrame {
  const side = opts.side ?? 1, back = opts.back ?? 1.15, across = opts.across ?? 0.55, camY = opts.camY ?? 1.35, lookY = opts.lookY ?? 1.3;
  let dx = target.x - fg.x, dz = target.z - fg.z;
  const len = Math.hypot(dx, dz) || 1; dx /= len; dz /= len;
  const rx = -dz * side, rz = dx * side;
  return {
    pos: { x: fg.x - dx * back + rx * across, y: fg.y + camY, z: fg.z - dz * back + rz * across },
    look: { x: target.x, y: target.y + lookY, z: target.z },
  };
}

/** dutch(deg = 5) — spread into the shot's look: { ...LOOK, ...dutch(6) }.
 *  Composes with slowPush or a near-static shoot. Requires createShoot's
 *  roll branch. Beat: unease, a lie, the villain's beat. Range 3–7° — in
 *  this flat style more reads as a rendering error. Never on the hook or
 *  the punchline shot. Sign: see ShotLook.roll. */
export function dutch(deg = 5): { roll: number } {
  return { roll: deg * Math.PI / 180 };
}

/** orbitShot(centre, k, opts) — wonder, a new place revealed, scale around
 *  a subject. FOV 36–44, radius 4.5–7 m, |swing| ≤ 1.0 rad; the camera
 *  azimuth relative to each faced subject must stay within ±60° at BOTH
 *  ends (STYLE §4) — the centre subject satisfies this automatically. */
export function orbitShot(centre: Vec3Like, k: number, opts: { radius?: number; height?: number; swing?: number; from?: number } = {}): RigFrame {
  const swing = opts.swing ?? 0.7, from = opts.from ?? -swing / 2;
  return orbit(centre, opts.radius ?? 5.5, opts.height ?? 2.2, from, from + swing, k);
}

/** craneDown(from, to, lookFrom, lookTo, k) — establishing hook that lands
 *  on the subject: start height 4–7 m → end 1.5–2.2 m. FOV 40–48. k = camK. */
export function craneDown(from: Vec3Like, to: Vec3Like, lookFrom: Vec3Like, lookTo: Vec3Like, k: number): RigFrame {
  return crane(from, to, lookFrom, lookTo, k);
}

/** craneUp(from, to, lookFrom, lookTo, k) — the reverse: endings, scale
 *  reveals, the loop-return finale. FOV 36–44. k = camK. */
export function craneUp(from: Vec3Like, to: Vec3Like, lookFrom: Vec3Like, lookTo: Vec3Like, k: number): RigFrame {
  return crane(from, to, lookFrom, lookTo, k);
}

/** cutIn(subj, faceY, camPos, opts) — reaction cut-in: it is a CUT, not a
 *  move — use as the new shot's static frame (or with a ≤ 5% slowPush
 *  after). faceY = the subject's group.y + headY(height, kid). Beat: the
 *  listener's answer face immediately after a reveal. Duration 0.8–1.5 s.
 *  FOV 36 (44 for a two-face cut-in), distance 2.2–3.0 m. Anchor: the cut
 *  itself lands at cut(i) or at a Wd() boundary — no clock needed. */
export function cutIn(subj: Vec3Like, faceY: number, camPos: Vec3Like, opts: { dist?: number; fov?: number } = {}): RigFrame {
  const dist = opts.dist ?? 2.6;
  let dx = camPos.x - subj.x, dz = camPos.z - subj.z;
  const len = Math.hypot(dx, dz) || 1; dx /= len; dz /= len;
  return { pos: { x: subj.x + dx * dist, y: faceY + 0.05, z: subj.z + dz * dist }, look: { x: subj.x, y: faceY, z: subj.z }, fov: opts.fov ?? 36 };
}

/** wideGag(centre, camPos, k, opts) — the deadpan wide gag: a crane from
 *  camPos to a point 3–5% closer to centre — deliberately near-static; the
 *  restraint IS the staging. Beat: a fall, a chase, a scale joke the
 *  audience must take in whole; a moving camera would step on the joke.
 *  FOV 55–65, distance 7–11 m (subject ≤ ~35% of frame height). Rule: no
 *  punch, cut-in or overlay move may follow inside the same shot. */
export function wideGag(centre: Vec3Like, camPos: Vec3Like, k: number, opts: { fov?: number } = {}): RigFrame {
  const to = { x: centre.x + (camPos.x - centre.x) * 0.96, y: centre.y + (camPos.y - centre.y) * 0.96, z: centre.z + (camPos.z - centre.z) * 0.96 };
  const f = crane(camPos, to, centre, centre, k);
  return { ...f, fov: opts.fov ?? 58 };
}

/** Catalog metadata — library/cameras.md tables and the catalog preview
 *  sheet are generated from this array; if the docs and the array disagree,
 *  the array wins and the doc is the bug. dist = metres, camera→subject
 *  (or the entry's own contract distance, per its doc comment). */
export interface ShotCatalogEntry { name: string; helper: string; beat: string; fov: [number, number]; dist: [number, number]; anchor: 'word' | 'start' | null }
export const SHOT_CATALOG: ShotCatalogEntry[] = [
  { name: 'hook-smash-in', helper: 'smashIn', beat: 'shot 1 hook / punchline return — crash push that lands and holds; at most twice per film', fov: [36, 44], dist: [3.5, 9], anchor: 'start' },
  { name: 'slow-push', helper: 'slowPush', beat: 'explanation, intimacy build — the serious tone’s default move; 15–30% push over ≥ 2 s', fov: [36, 44], dist: [4, 5.5], anchor: null },
  { name: 'whip-pan', helper: 'whipPan', beat: 'intra-line change of place in one continuous space ("meanwhile…"), comic teleport', fov: [44, 55], dist: [4, 7], anchor: 'start' },
  { name: 'dolly-reveal', helper: 'reveal', beat: 'the hidden person/thing/name-drop — fully revealed exactly on its word', fov: [36, 50], dist: [3, 5], anchor: 'word' },
  { name: 'punch-in', helper: 'punchIn', beat: 'reveal-word emphasis; the deadpan micro-beat after the biggest reveal', fov: [36, 36], dist: [2.2, 5], anchor: 'word' },
  { name: 'over-shoulder', helper: 'overShoulder', beat: 'dialogue exchange, "he showed him…", confrontation — fg figure cropped at the lower corner', fov: [44, 55], dist: [2.5, 5], anchor: null },
  { name: 'dutch', helper: 'dutch', beat: 'unease, a lie, the villain’s beat — 3–7° roll via the shot look; never the hook or punchline', fov: [36, 44], dist: [0, 0], anchor: null },
  { name: 'orbit', helper: 'orbitShot', beat: 'wonder, a new place revealed, scale around a subject', fov: [36, 44], dist: [4.5, 7], anchor: null },
  { name: 'crane-down', helper: 'craneDown', beat: 'establishing hook that lands on the subject (6 m → 1.7 m)', fov: [40, 48], dist: [4, 9], anchor: null },
  { name: 'crane-up', helper: 'craneUp', beat: 'endings, scale reveals, the loop-return finale', fov: [36, 44], dist: [4, 9], anchor: null },
  { name: 'reaction-cut-in', helper: 'cutIn', beat: 'the listener’s answer face immediately after a reveal — a cut, not a move', fov: [36, 44], dist: [2.2, 3.0], anchor: null },
  { name: 'wide-gag', helper: 'wideGag', beat: 'deadpan wide — a fall, a chase, a scale joke taken in whole; camera restraint is the staging', fov: [55, 65], dist: [7, 11], anchor: null },
];
