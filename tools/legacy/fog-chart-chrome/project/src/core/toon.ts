// Toon people: flat 2D cartoon characters (thick ink outlines, big round heads, bean bodies, stick limbs) that live
// inside the 3D world as camera-facing cut-outs with real shadows.
//
//   drawToon(g, look, st)   draws one character into any 2D context, in a 512 x 1024 box (feet at y 1000)
//   new Toon(look, { height })  a 3D cut-out:  .set(st) every frame, .face(camera), .group (position it), .tint(col)
//
// CANVAS MARGIN (v3): drawToon still draws in its unchanged 512 x 1024 stage (feet at stage y 1000), but the
// Toon canvas is 1280 x 1408 with the stage anchored centre-x inside it — stage offset OX=384, OY=256
// (see STAGE below), so in canvas coords the stage box is x 384..896, y 256..1280: margins of 384 units
// to each side, 256 above the head and 128 BELOW the feet. Anything drawToon paints outside the stage
// used to vanish at the old 512 x 1024 canvas edge: raised arms, a tophat crown (stage y ≈ -96) and held
// props (e.g. the tip of a long held prop reaching stage y ≈ -12 at its raise peak, clipped to a nub).
// The margins give that overflow somewhere to land; the below-feet margin exists because lean rotates
// the figure about its feet, dipping a foot a few units under the stage floor, and the wider side
// margins absorb the sideways shift lean gives a raised hand (both found by the universal stress
// reel). scanCanvasOverflow / Toon#overflow report anything that STILL reaches the new canvas edge.
// The plane keeps the old unit scale (H/1024 per logical unit) and the same feet anchor, so a rest
// pose renders pixel-identical in placement and size — only previously-clipped pixels can newly appear.
//
// V4 ADDITIONS (all opt-in; with every new option/state at its default the output is pixel-identical
// to v3): head control (st.headTilt / st.headTurn) and five new moods in drawToon/drawFace; the POSES
// starting bundles live in core/poses.ts; perspective-aware presentation (Toon { perspective } pitches
// the plane in face(), camPitchOf() + st.camPitch compensate in the drawing); the multiPlane prototype
// (Toon { multiPlane } draws the head on a second, slightly forward plane via drawToonLayer);
// ground contact (groundY + sitOn); environmental colour integration (Toon { envMatch } + matchEnv);
// and attach3D (attach/detach real 3D props to the drawn hands).
//
// look = { body: '#E8734A', shade?, pattern: 'plain'|'check'|'stripes'|'overalls'|'vest'|'apron', patCol,
//          hat: 'none'|'cap'|'bowler'|'boater'|'beanie'|'visor'|'tophat'|'bun'|'flower', hatCol, band,
//          neck: 'none'|'bowtie'|'tie'|'scarf'|'collar', neckCol, dots,
//          extra: ['mustache','glasses','freckles','blush','pencil','monocle'], head: '#F7F7FA' }
// st   = { mood: 'neutral'|'smug'|'happy'|'laugh'|'shock'|'worried'|'angry'|'sad'|'sly'|'proud'|'tired'|'dizzy',
//          look: [dx, dy] (-1..1 pupils), blink 0..1, talk 0..1 (mouth open), sweat, tears,
//          armL / armR: [shoulder°, elbow°]  (0 = hanging down, + = outward/up, elbow + = bends up),
//          legs: 'stand'|'walk'|'run'|'sit'|'jump', phase (walk cycle, radians), lean°, bob (px), squash,
//          holdL / holdR: (g, x, y, angle) => draws a prop in that hand,  flip (mirror left/right) }
import * as THREE from 'three';
import type { LibraryItemMeta } from './library';

export const INK = '#1F1C33';
const LW = 9;
const R2D = Math.PI / 180;


export type Mood = 'neutral' | 'smug' | 'happy' | 'laugh' | 'shock' | 'worried' | 'angry' | 'sad' | 'sly' | 'proud' | 'tired' | 'dizzy' | 'determined' | 'embarrassed' | 'awe' | 'suspicious' | 'relieved' | 'puzzled' | 'deadpan';
export type LegPose = 'stand' | 'walk' | 'run' | 'sit' | 'jump';
export type HatKind = 'none' | 'cap' | 'bowler' | 'boater' | 'beanie' | 'visor' | 'tophat' | 'bun' | 'flower' | 'bow' | 'mailcap';
export type NeckKind = 'none' | 'bowtie' | 'tie' | 'scarf' | 'collar';
// Runtime name lists mirroring the unions above (v5, D4 §1.1): sheets, docs and tests iterate
// THESE instead of hardcoding names, so a union member added without a list entry is caught by
// tests/acting.probe.ts instead of silently drifting (the v4 style sheet listed only 12 of 17
// moods). Keep each list in union order.
export const MOODS: readonly Mood[] = ['neutral', 'smug', 'happy', 'laugh', 'shock', 'worried', 'angry', 'sad', 'sly', 'proud', 'tired', 'dizzy', 'determined', 'embarrassed', 'awe', 'suspicious', 'relieved', 'puzzled', 'deadpan'];
export const HATS: readonly HatKind[] = ['none', 'cap', 'bowler', 'boater', 'beanie', 'visor', 'tophat', 'bun', 'flower', 'bow', 'mailcap'];
export const NECKS: readonly NeckKind[] = ['none', 'bowtie', 'tie', 'scarf', 'collar'];
export type PatternKind = 'plain' | 'check' | 'stripes' | 'overalls' | 'vest' | 'apron' | 'stamps';
/** draws a prop held in a hand: (context, hand x, hand y, hand angle in radians, side -1|+1) */
export type PropFn = (g: CanvasRenderingContext2D, x: number, y: number, angle: number, side: number) => void;
export interface ToonLook {
  name?: string; body?: string; shade?: string; pattern?: PatternKind; patCol?: string; patCol2?: string;
  hat?: HatKind; hatCol?: string; band?: string; neck?: NeckKind; neckCol?: string; dots?: number;
  extra?: string[]; head?: string; kid?: boolean; stamps?: number; mustacheCol?: string; hairCol?: string;
}
export interface ToonState {
  mood?: Mood; look?: [number, number]; blink?: number; talk?: number; sweat?: number; tears?: number | boolean;
  armL?: [number, number]; armR?: [number, number]; legs?: LegPose; phase?: number; lean?: number; bob?: number;
  squash?: number; holdL?: PropFn | null; holdR?: PropFn | null; flip?: boolean;
  /** head rotation in degrees about the neck pivot (+ = head top tips toward screen-right); default 0 */
  headTilt?: number;
  /** face turn -1..1 (+ = face turns toward screen-right); shifts features/hat with parallax; default 0 */
  headTurn?: number;
  /** camera pitch compensation -1..1 (from camPitchOf); subtle head-size/feature shift; default 0 */
  camPitch?: number;
  /** seat height in METRES above the feet line (legs 'sit'); Toon#set converts it to sitDrop */
  sitOn?: number;
  /** seat height in STAGE units above the feet line, for direct drawToon callers */
  sitDrop?: number;
  // shots attach extra cache-busting keys (e.g. `v: 'open1'`) that only feed the repaint key in Toon#set
  [key: string]: unknown;
}
export type ResolvedLook = ToonLook & { body: string; head: string; hat: HatKind; neck: NeckKind; pattern: PatternKind; extra: string[] };
export interface ResolvedState {
  mood: Mood; look: [number, number]; blink: number; talk: number; sweat?: number; tears?: number | boolean;
  armL: [number, number]; armR: [number, number]; legs: LegPose; phase: number; lean: number; bob: number;
  squash: number; holdL?: PropFn | null; holdR?: PropFn | null; flip?: boolean;
  headTilt: number; headTurn: number; camPitch: number; sitDrop?: number;
}
export type Hands = { L?: [number, number]; R?: [number, number] };
export interface OverflowCounts { top: number; right: number; bottom: number; left: number }
export interface PropsMap {
  parcels: PropFn; book: (title?: string, open?: number) => PropFn; ticket: PropFn; purse: PropFn;
  letters: PropFn; rubberStamp: PropFn; stamp: PropFn; cash: (n?: number) => PropFn; bill: PropFn;
  stack: PropFn; paper: (text?: string) => PropFn; mirror: PropFn; cup: PropFn; key: PropFn;
  sign: (text?: string, col?: string) => PropFn;
  // v5 (additive): the D5 prop wave — reach of every entry ≤190 units from the grip
  axe: PropFn; hammer: PropFn; wrench: PropFn; shovel: PropFn; apple: PropFn; bottle: PropFn;
  sandwich: PropFn; fish: PropFn; flower: PropFn; egg: PropFn; feather: PropFn; ball: PropFn;
  trophy: PropFn; phone: PropFn; camera: PropFn; radio: PropFn; umbrella: PropFn; lantern: PropFn;
  map: (text?: string) => PropFn; rope: PropFn; medal: PropFn;
}
export interface ToonOptions { height?: number; res?: number; castShadow?: boolean; perspective?: boolean; multiPlane?: boolean; envMatch?: boolean }
/** environmental colour grade for a toon cut-out, produced by an atmosphere preset (core/env.ts) */
export interface EnvTint { sat: number; warm: number; exposure: number; fogColor: string; fogMix: number }
interface AttachEntry { obj: THREE.Object3D; hand: 'L' | 'R'; offset: THREE.Vector3; rotQ: THREE.Quaternion }
const clamp = (v: number, a: number, b: number) => Math.max(a, Math.min(b, v));

export function shadeOf(hex: string, k = 0.78) {
  const n = parseInt(hex.slice(1), 16), r = (n >> 16) & 255, gg = (n >> 8) & 255, b = n & 255;
  const f = (v: number) => Math.max(0, Math.min(255, Math.round(v * k)));
  return '#' + ((1 << 24) + (f(r) << 16) + (f(gg) << 8) + f(b)).toString(16).slice(1);
}

// ---------- stage / canvas geometry (v3) ----------
// W x H = the drawToon stage (unchanged); CW x CH = the Toon canvas; (OX, OY) = where the stage's
// top-left corner sits on the canvas: stage (0,0) -> canvas (384,256), stage feet y 1000 -> canvas
// y 1256, stage bottom 1024 -> canvas y 1280 (128 units of margin below the feet, 384 to each side).
export const STAGE = { W: 512, H: 1024, CW: 1280, CH: 1408, OX: 384, OY: 256 };

/** every constructed Toon, in construction order (Toon#overflow / capture checks scan this list) */
export const TOONS: Toon[] = [];

/**
 * A canvas registered for overflow auditing. The built-in Toon registers itself; a project-local
 * canvas-drawn character (any custom class that paints a figure into its own canvas) registers
 * via registerOverflowCanvas so the capture check/sheet overflow gate covers it too — the
 * detector is character-agnostic, not Toon-specific.
 */
export interface OverflowCanvasEntry {
  canvas: HTMLCanvasElement;
  /** canvas pixels per logical unit (the border strip is measured in logical units) */
  res: number;
  label: string;
  /** optional live pose description, printed alongside any overflow report for this canvas */
  pose?: () => string;
}
export const OVERFLOW_CANVASES: OverflowCanvasEntry[] = [];

/**
 * Register ANY canvas-drawn character's canvas for the overflow gate. Call once, where the
 * canvas is created:
 *   registerOverflowCanvas(myCanvas, 'my-character', { res: 2, pose: () => `arm=${arm}` });
 * `res` = canvas pixels per logical unit of the drawing (default 1: the strip is then measured
 * in raw pixels). Built-in Toons register automatically; registering the same canvas twice is
 * a no-op (identity check), so a Toon canvas never needs — and never gets — a second entry.
 */
export function registerOverflowCanvas(canvas: HTMLCanvasElement, label: string, opts: { res?: number; pose?: () => string } = {}): void {
  if (!canvas || OVERFLOW_CANVASES.some((e) => e.canvas === canvas)) return;
  OVERFLOW_CANVASES.push({ canvas, res: opts.res ?? 1, label, pose: opts.pose });
}

/**
 * Counts opaque-ish pixels (alpha > 32) in the four border strips of a canvas — the overflow
 * signal: a non-zero count means drawing reached that canvas edge and may be clipped there.
 * Reads ONLY the strips (strip logical units wide, via one getImageData per strip), never the
 * whole canvas. Corners belong to two strips and are counted in both. Returns { top, right, bottom, left }.
 * The default strip is the outermost 1 logical unit — paint actually CUT at the edge. A wider
 * strip false-positives on legal paint that merely comes close: the seated knee's stroke
 * legitimately reaches stage y ≈ 1021.5 of 1024 without clipping (found by the v3 template check).
 */
export function scanCanvasOverflow(canvas: HTMLCanvasElement | null, res = 1, strip = 1): OverflowCounts {
  const counts: OverflowCounts = { top: 0, right: 0, bottom: 0, left: 0 };
  if (!canvas || !canvas.width || !canvas.height) return counts;
  const g = canvas.getContext('2d');
  if (!g) return counts;
  const W = canvas.width, Hh = canvas.height;
  const s = Math.max(1, Math.round(strip * res));
  const scan = (x: number, y: number, w: number, h: number) => {
    if (w <= 0 || h <= 0) return 0;
    const d = g.getImageData(x, y, w, h).data;
    let n = 0;
    for (let i = 3; i < d.length; i += 4) if (d[i] > 32) n++;
    return n;
  };
  counts.top = scan(0, 0, W, Math.min(s, Hh));
  counts.bottom = scan(0, Hh - Math.min(s, Hh), W, Math.min(s, Hh));
  counts.left = scan(0, 0, Math.min(s, W), Hh);
  counts.right = scan(W - Math.min(s, W), 0, Math.min(s, W), Hh);
  return counts;
}

// ---------- the drawing ----------
export function drawToon(g: CanvasRenderingContext2D, look: ToonLook = {}, st: ToonState = {}): Hands {
  return drawToonLayer(g, look, st, 'full');
}
/**
 * drawToon with a layer split for the multiPlane prototype: 'full' is exactly drawToon;
 * 'nohead' draws everything except the head; 'headonly' applies only the flip/lean
 * transforms and draws the head (returning no hands).
 */
export function drawToonLayer(g: CanvasRenderingContext2D, look: ToonLook = {}, st: ToonState = {}, layer: 'full' | 'nohead' | 'headonly'): Hands {
  const L: ResolvedLook = { body: '#E8734A', head: '#F7F7FA', hat: 'none', neck: 'none', pattern: 'plain', extra: [], ...look };
  const S: ResolvedState = { mood: 'neutral', look: [0, 0], blink: 0, talk: 0, armL: [8, 6], armR: [8, 6], legs: 'stand', phase: 0, lean: 0, bob: 0, squash: 0, ...st, headTilt: st.headTilt ?? 0, headTurn: st.headTurn ?? 0, camPitch: st.camPitch ?? 0 };
  g.save();
  g.lineCap = 'round'; g.lineJoin = 'round';
  if (S.flip) { g.translate(512, 0); g.scale(-1, 1); }
  // whole-body lean pivots at the feet
  g.translate(256, 1000); g.rotate((S.lean || 0) * R2D); g.translate(-256, -1000);
  const sit = S.legs === 'sit', jump = S.legs === 'jump';
  const sitS = S.sitDrop ?? 220.2;   // implied hip height at the default seat: hipY = 574 + 0.98*210 = 779.8, feet 1000 -> 220.2
  const by = (sit ? (426 - sitS) / 0.98 : 0) + (L.kid ? 190 : 0) + (S.bob || 0) + (jump ? -60 : 0);       // how far the upper body drops (kids: short legs); (426 - 220.2)/0.98 = 210 exactly at the default seat
  // body box
  const bx = 256, top = 330 + by, bot = 600 + by * 0.98, bw = 92;     // half width at the hips
  const hipY = bot - 26;
  // head geometry (drawn later by drawHeadPart; the neck pivot drops with the body exactly like hy)
  const hx = 256, hy = 178 + by * 0.99, r = 138, pivotY = 318 + by * 0.99;
  if (layer === 'headonly') { drawHeadPart(g, L, S, hx, hy, r, pivotY); g.restore(); return {}; }

  // ---- legs (behind the body) ----
  g.strokeStyle = INK; g.lineWidth = 15;
  const leg = (sx: number, side: number) => {
    let kx: number, ky: number, fx: number, fy: number;
    if (sit) {                                           // thighs come toward us (short), shins hang down; feet dangle above the floor on a high seat, plant on a low one
      const kx2 = sx + side * 30, ky2 = hipY + 44; fx = sx + side * 34; fy = Math.min(1000, hipY + 220.2);
      g.beginPath(); g.moveTo(sx, hipY); g.lineTo(kx2, ky2); g.lineTo(fx, fy); g.stroke();
      g.fillStyle = INK; g.beginPath(); g.ellipse(fx + side * 10, fy - 4, 20, 10, 0, 0, Math.PI * 2); g.fill(); return;
    }
    else if (S.legs === 'walk' || S.legs === 'run') {
      const amp = S.legs === 'run' ? 1.0 : 0.55, a = Math.sin(S.phase + (side > 0 ? Math.PI : 0)) * amp;
      const lift = Math.max(0, -Math.cos(S.phase + (side > 0 ? Math.PI : 0))) * (S.legs === 'run' ? 70 : 34);
      kx = sx + a * 22 + side * 6; ky = hipY + 200 - lift * 0.6; fx = sx + a * 34 + side * 8; fy = 1000 - lift;
    } else if (jump) { kx = sx + side * 40; ky = hipY + 170; fx = sx + side * 18; fy = 940; }
    else { kx = sx + side * 4; ky = hipY + 210; fx = sx + side * 10; fy = 1000; }
    g.beginPath(); g.moveTo(sx, hipY); g.quadraticCurveTo(kx, ky, fx, fy); g.stroke();
    g.fillStyle = INK; g.beginPath(); g.ellipse(fx + side * 10, fy - 4, 20, 10, 0, 0, Math.PI * 2); g.fill();
  };
  leg(bx - 42, -1); leg(bx + 42, 1);

  // ---- body (a bean) ----
  const body = new Path2D();
  body.moveTo(bx - 58, top + 10);
  body.bezierCurveTo(bx - 80, top + 40, bx - bw - 6, bot - 120, bx - bw, bot - 50);
  body.bezierCurveTo(bx - bw + 4, bot + 6, bx + bw - 4, bot + 6, bx + bw, bot - 50);
  body.bezierCurveTo(bx + bw + 6, bot - 120, bx + 80, top + 40, bx + 58, top + 10);
  body.bezierCurveTo(bx + 30, top - 8, bx - 30, top - 8, bx - 58, top + 10);
  body.closePath();
  g.fillStyle = L.body; g.fill(body);
  g.save(); g.clip(body);
  drawPattern(g, L, bx, top, bot, bw);
  g.fillStyle = L.shade || shadeOf(L.body, 0.8); g.globalAlpha = 0.85;
  g.beginPath(); g.ellipse(bx + bw + 40, (top + bot) / 2 + 20, 70, 260, 0, 0, Math.PI * 2); g.fill();
  g.globalAlpha = 0.25; g.fillStyle = '#FFFFFF'; g.beginPath(); g.ellipse(bx - 50, top + 90, 16, 60, 0.2, 0, Math.PI * 2); g.fill();
  g.globalAlpha = 1; g.restore();
  g.strokeStyle = INK; g.lineWidth = LW; g.stroke(body);
  drawNeckwear(g, L, bx, top);

  // ---- arms (in front of the body) ----
  const hands: Hands = {};
  const arm = (side: number, [a, e]: [number, number], hold: PropFn | null | undefined) => {
    const sx = bx + side * 66, sy = top + 46;
    const u = 118, lo = 112;
    const a1 = a * R2D, a2 = (a + e) * R2D;
    const ex = sx + side * Math.sin(a1) * u, ey = sy + Math.cos(a1) * u;
    const hx = ex + side * Math.sin(a2) * lo, hy = ey + Math.cos(a2) * lo;
    g.strokeStyle = INK; g.lineWidth = 14;
    g.beginPath(); g.moveTo(sx, sy); g.quadraticCurveTo(ex, ey, hx, hy); g.stroke();
    if (hold) { g.save(); hold(g, hx, hy, side * (a + e) * R2D, side); g.restore(); }
    g.fillStyle = INK; g.beginPath(); g.ellipse(hx, hy, 15, 17, 0, 0, Math.PI * 2); g.fill();
    hands[side < 0 ? 'L' : 'R'] = [hx, hy];
  };
  arm(-1, S.armL, S.holdL); arm(1, S.armR, S.holdR);

  // ---- head ----
  if (layer === 'full') drawHeadPart(g, L, S, hx, hy, r, pivotY);
  g.restore();
  return hands;
}

/**
 * The head block of drawToon, extracted verbatim (v4) so drawToonLayer can draw it on its own
 * plane. v4 head control wraps it: headTilt rotates about the neck pivot (256, pivotY) in an
 * identity transform sandwich at tilt 0; headTurn shifts the face features by turn*30, the
 * shading arc likewise, and the hat/bun by turn*18 (parallax lag), while the head outline stays
 * centred on hx; camPitch scales the head radius by (1 + 0.04*camPitch) and slides the features
 * — compensation for high/low camera angles, not a cure (characters read correctly to ~±75° of
 * facing). At headTilt 0 / headTurn 0 / camPitch 0 every value below equals the pre-v4 code.
 */
function drawHeadPart(g: CanvasRenderingContext2D, L: ResolvedLook, S: ResolvedState, hx: number, hy: number, r: number, pivotY: number) {
  const turn = S.headTurn || 0;
  const rp = r * (1 + 0.04 * (S.camPitch || 0));
  const fx = hx + turn * 30;
  g.save(); g.translate(256, pivotY); g.rotate((S.headTilt || 0) * R2D); g.translate(-256, -pivotY);
  g.save(); g.translate(hx, hy); g.scale(1 + (S.squash || 0) * -0.3, 1 + (S.squash || 0)); g.translate(-hx, -hy);
  if (L.hat === 'bun') { g.fillStyle = L.hatCol || '#6B4A3A'; g.beginPath(); g.arc(hx + 10 + turn * 18, hy - rp - 8, 46, 0, Math.PI * 2); g.fill(); g.strokeStyle = INK; g.lineWidth = LW; g.stroke(); }
  const head = new Path2D(); head.ellipse(hx, hy, rp, rp * 0.97, 0, 0, Math.PI * 2);
  g.fillStyle = L.head; g.fill(head);
  g.save(); g.clip(head);
  g.fillStyle = shadeOf(L.head, 0.9); g.beginPath(); g.arc(hx + 52 + turn * 30, hy + 44, rp, 0, Math.PI * 2); g.rect(hx - 400, hy - 400, 800, 800); g.fill('evenodd');
  g.restore();
  g.strokeStyle = INK; g.lineWidth = LW + 1; g.stroke(head);
  drawFace(g, L, S, fx, hy);
  drawHat(g, L, hx + turn * 18, hy, rp);
  g.restore();
  g.restore();
}

function drawPattern(g: CanvasRenderingContext2D, L: ResolvedLook, bx: number, top: number, bot: number, bw: number) {
  const pc = L.patCol || shadeOf(L.body, 0.7);
  if (L.pattern === 'check') {
    g.strokeStyle = pc; g.globalAlpha = 0.55; g.lineWidth = 6;
    for (let x = bx - 120; x < bx + 120; x += 34) { g.beginPath(); g.moveTo(x, top - 20); g.lineTo(x, bot + 20); g.stroke(); }
    for (let y = top; y < bot + 20; y += 34) { g.beginPath(); g.moveTo(bx - 130, y); g.lineTo(bx + 130, y); g.stroke(); }
    g.lineWidth = 2; g.globalAlpha = 0.5; g.strokeStyle = L.patCol2 || '#B5372E';
    for (let x = bx - 103; x < bx + 120; x += 68) { g.beginPath(); g.moveTo(x, top - 20); g.lineTo(x, bot + 20); g.stroke(); }
    g.globalAlpha = 1;
  } else if (L.pattern === 'stripes') {
    g.fillStyle = pc; for (let x = bx - 130; x < bx + 130; x += 36) g.fillRect(x, top - 20, 14, bot - top + 40);
  } else if (L.pattern === 'overalls') {
    g.fillStyle = L.patCol || '#3E5FA8';
    g.beginPath(); g.roundRect(bx - 62, top + 110, 124, 110, 14); g.fill(); g.fillRect(bx - 130, top + 190, 260, 200);
    g.strokeStyle = shadeOf(g.fillStyle as string, 0.75); g.lineWidth = 4; g.setLineDash([8, 7]); g.strokeRect(bx - 48, top + 124, 96, 80); g.setLineDash([]);
    g.fillStyle = L.patCol || '#3E5FA8'; g.fillRect(bx - 58, top, 16, 120); g.fillRect(bx + 42, top, 16, 120);
    g.fillStyle = '#F1C24B'; [[-50, 118], [50, 118]].forEach(([dx, dy]) => { g.beginPath(); g.arc(bx + dx, top + dy, 8, 0, 7); g.fill(); });
  } else if (L.pattern === 'vest') {
    g.fillStyle = L.patCol || '#3B3346';
    g.beginPath(); g.moveTo(bx - 140, top - 10); g.lineTo(bx - 8, top + 120); g.lineTo(bx - 8, bot + 20); g.lineTo(bx - 140, bot + 20); g.fill();
    g.beginPath(); g.moveTo(bx + 140, top - 10); g.lineTo(bx + 8, top + 120); g.lineTo(bx + 8, bot + 20); g.lineTo(bx + 140, bot + 20); g.fill();
    g.fillStyle = '#E9C46A'; [150, 195, 240].forEach((dy) => { g.beginPath(); g.arc(bx + 26, top + dy, 6, 0, 7); g.fill(); });
  } else if (L.pattern === 'stamps' || L.stamps) {
    const n = L.stamps ?? 6; const spots = [[-50, 70, -0.2], [30, 60, 0.15], [-10, 140, 0.05], [50, 150, -0.12], [-60, 180, 0.18], [10, 215, -0.1], [-30, 100, 0.3], [60, 105, 0.08], [20, 175, 0.25], [-45, 240, -0.05]];
    spots.slice(0, n).forEach(([dx, dy, r], i) => { g.save(); g.translate(bx + dx, top + dy); g.rotate(r); const cols = ['#D8343F', '#3C6FB6', '#2E8A5A', '#E2B33C', '#8C6FC9']; g.fillStyle = '#FFFDF4'; g.strokeStyle = INK; g.lineWidth = 3;
      g.beginPath(); g.rect(-22, -27, 44, 54); g.fill(); g.stroke(); g.fillStyle = cols[i % 5]; g.fillRect(-15, -20, 30, 32); g.fillStyle = '#FFFDF4'; g.font = '900 13px Inter'; g.textAlign = 'center'; g.fillText(i % 2 ? '5¢' : '2¢', 0, 22);
      g.beginPath(); g.arc(0, -6, 8, 0, 7); g.fill(); g.restore(); });
  } else if (L.pattern === 'apron') {
    g.fillStyle = L.patCol || '#F4EFE4'; g.beginPath(); g.roundRect(bx - 70, top + 90, 140, bot - top, 18); g.fill();
    g.strokeStyle = shadeOf(L.patCol || '#F4EFE4', 0.8); g.lineWidth = 4; g.beginPath(); g.roundRect(bx - 40, top + 190, 80, 60, 8); g.stroke();
  }
}

function drawNeckwear(g: CanvasRenderingContext2D, L: ResolvedLook, bx: number, top: number) {
  const c = L.neckCol || '#D8343F';
  g.strokeStyle = INK; g.lineWidth = 6;
  if (L.neck === 'bowtie') {
    const y = top + 24, bt = new Path2D();
    bt.moveTo(bx, y); bt.bezierCurveTo(bx - 30, y - 30, bx - 64, y - 26, bx - 62, y); bt.bezierCurveTo(bx - 64, y + 26, bx - 30, y + 30, bx, y);
    bt.bezierCurveTo(bx + 30, y - 30, bx + 64, y - 26, bx + 62, y); bt.bezierCurveTo(bx + 64, y + 26, bx + 30, y + 30, bx, y);
    g.fillStyle = c; g.fill(bt);
    if (L.dots) { g.save(); g.clip(bt); g.fillStyle = '#FFFFFF'; for (let i = -3; i <= 3; i++) for (let j = -1; j <= 1; j++) { g.beginPath(); g.arc(bx + i * 18 + (j & 1) * 9, y + j * 15, 4.5, 0, 7); g.fill(); } g.restore(); }
    g.stroke(bt); g.fillStyle = shadeOf(c, 0.75); g.beginPath(); g.roundRect(bx - 13, y - 13, 26, 26, 6); g.fill(); g.stroke();
  } else if (L.neck === 'tie') {
    g.fillStyle = c; g.beginPath(); g.moveTo(bx - 14, top + 10); g.lineTo(bx + 14, top + 10); g.lineTo(bx + 20, top + 140); g.lineTo(bx, top + 170); g.lineTo(bx - 20, top + 140); g.closePath(); g.fill(); g.stroke();
  } else if (L.neck === 'scarf') {
    g.fillStyle = c; g.beginPath(); g.moveTo(bx + 20, top + 20); g.lineTo(bx + 58, top + 130); g.lineTo(bx + 24, top + 138); g.lineTo(bx + 6, top + 26); g.closePath(); g.fill(); g.stroke();
    g.beginPath(); g.moveTo(bx - 70, top + 4); g.quadraticCurveTo(bx, top + 44, bx + 70, top + 4); g.lineTo(bx + 66, top + 34); g.quadraticCurveTo(bx, top + 76, bx - 66, top + 34); g.closePath(); g.fill(); g.stroke();
    g.strokeStyle = shadeOf(c, 0.75); g.lineWidth = 4; g.beginPath(); g.moveTo(bx - 50, top + 26); g.quadraticCurveTo(bx, top + 56, bx + 50, top + 26); g.stroke();
  } else if (L.neck === 'collar') {
    g.fillStyle = '#FFFFFF';
    [-1, 1].forEach((s) => { g.beginPath(); g.moveTo(bx, top + 18); g.lineTo(bx + s * 56, top + 2); g.lineTo(bx + s * 40, top + 46); g.closePath(); g.fill(); g.stroke(); });
  }
}

function eye(g: CanvasRenderingContext2D, x: number, y: number, S: ResolvedState, side: number, L: ResolvedLook) {
  const [lx, ly] = S.look; const m = S.mood; const bl = S.blink || 0;
  g.fillStyle = INK; g.strokeStyle = INK;
  if (m === 'happy' || m === 'laugh' || m === 'relieved' || (m === 'proud' && side > 0 && false)) {
    g.lineWidth = 10; g.beginPath(); g.arc(x, y + 10, 20, Math.PI * 1.15, Math.PI * 1.85); g.stroke(); return;
  }
  if (m === 'dizzy') { g.lineWidth = 6; g.beginPath(); for (let a = 0; a < 12; a += 0.3) { const rr = a * 1.6; g.lineTo(x + Math.cos(a * side) * rr, y + Math.sin(a * side) * rr); } g.stroke(); return; }
  if (m === 'angry' && false) return;
  if (m === 'shock') {
    g.fillStyle = '#FFFFFF'; g.lineWidth = 6; g.beginPath(); g.ellipse(x, y, 26, 30, 0, 0, 7); g.fill(); g.stroke();
    g.fillStyle = INK; g.beginPath(); g.arc(x + lx * 8, y + ly * 8, 7, 0, 7); g.fill(); return;
  }
  if (m === 'awe') {           // wide like shock, but a bigger sparkling pupil
    g.fillStyle = '#FFFFFF'; g.lineWidth = 6; g.beginPath(); g.ellipse(x, y, 26, 30, 0, 0, 7); g.fill(); g.stroke();
    g.fillStyle = INK; g.beginPath(); g.arc(x + lx * 8, y + ly * 8, 9, 0, 7); g.fill();
    g.fillStyle = '#FFFFFF'; g.beginPath(); g.arc(x + lx * 8 - 3, y + ly * 8 - 3, 3, 0, 7); g.fill(); return;
  }
  if (bl > 0.6) { g.lineWidth = 8; g.beginPath(); g.moveTo(x - 14, y + 4); g.quadraticCurveTo(x, y + 10, x + 14, y + 4); g.stroke(); return; }
  // pupil (suspicious: the viewer's-left eye is narrowed to a slit)
  const px = x + lx * 9, py = y + ly * 8;
  g.beginPath(); g.ellipse(px, py, 13, 17 * (1 - bl * 0.6) * (m === 'suspicious' && side < 0 ? 0.55 : 1) * (m === 'deadpan' ? 0.8 : 1), 0, 0, 7); g.fill();
  if (m === 'smug' || m === 'tired' || m === 'sly' || m === 'proud' || m === 'determined' || m === 'deadpan') {           // heavy lid over the top half
    const lid = m === 'tired' ? 6 : m === 'deadpan' ? 8 : 0;
    g.fillStyle = L.head; g.fillRect(x - 24, y - 28, 48, 26 + lid);
    g.lineWidth = 7; g.beginPath(); g.moveTo(x - 20, y - 2 + lid); g.lineTo(x + 20, y - 2 + lid); g.stroke();
  }
  if (m === 'sad' || m === 'worried') { g.fillStyle = '#FFFFFF'; g.beginPath(); g.arc(px - 3, py - 5, 3.5, 0, 7); g.fill(); }
}

// fx = the FEATURE centre x (head centre + headTurn*30, passed by drawHeadPart); every feature
// below is drawn around fx while the head outline/shading stay centred on the head centre.
// Eye spacing compresses with |headTurn| and the features slide down the face with camPitch —
// both are exact no-ops (ex 46, ey hy+4) at headTurn 0 / camPitch 0.
function drawFace(g: CanvasRenderingContext2D, L: ResolvedLook, S: ResolvedState, fx: number, hy: number) {
  const m = S.mood, ex = 46 * (1 - 0.18 * Math.abs(S.headTurn || 0)), ey = hy + 4 + (S.camPitch || 0) * 8;
  // eyebrows
  const brow = { neutral: [4, 0], smug: [-8, 10], happy: [8, 0], laugh: [10, 0], shock: [24, 0], worried: [10, -16], angry: [-10, 22],
    sad: [8, -18], sly: [-6, 12], proud: [6, 4], tired: [-2, 0], dizzy: [14, 0],
    determined: [-4, 16], embarrassed: [12, -10], awe: [26, 0], suspicious: [-2, 6], relieved: [14, -4],
    puzzled: [6, 4], deadpan: [2, 0] }[m] || [4, 0];
  g.strokeStyle = INK; g.lineWidth = 12;
  [-1, 1].forEach((s) => {
    let lift = brow[0], tilt = brow[1] * s;           // tilt > 0: inner end lower (angry)
    if ((m === 'smug' || m === 'sly') && s > 0) { lift = 16; tilt = -4; }
    if (m === 'suspicious' && s < 0) { lift = 18; tilt = -6; }   // one brow cocked up
    if (m === 'puzzled') { if (s < 0) { lift = 16; tilt = -2; } else { lift = -2; tilt = 8; } }   // one brow up, one brow down — the whole read
    const y = ey - 44 - lift;
    g.beginPath(); g.moveTo(fx + s * (ex - 26), y + tilt * 0.5); g.quadraticCurveTo(fx + s * ex, y - 7, fx + s * (ex + 26), y - tilt * 0.5);
    g.stroke();
  });
  eye(g, fx - ex, ey, S, -1, L); eye(g, fx + ex, ey, S, 1, L);
  if (L.extra.includes('glasses')) { g.strokeStyle = INK; g.lineWidth = 5; [-1, 1].forEach((s) => { g.beginPath(); g.arc(fx + s * ex, ey, 30, 0, 7); g.stroke(); }); g.beginPath(); g.moveTo(fx - 16, ey - 4); g.lineTo(fx + 16, ey - 4); g.stroke(); }
  if (L.extra.includes('monocle')) { g.strokeStyle = '#C9A13B'; g.lineWidth = 5; g.beginPath(); g.arc(fx + ex, ey, 30, 0, 7); g.stroke(); g.beginPath(); g.moveTo(fx + ex + 22, ey + 22); g.quadraticCurveTo(fx + ex + 40, ey + 90, fx + ex + 10, ey + 140); g.stroke(); }
  if (L.extra.includes('blush') || m === 'laugh' || m === 'proud' || m === 'embarrassed') { g.fillStyle = 'rgba(240,120,130,0.35)'; [-1, 1].forEach((s) => { g.beginPath(); g.ellipse(fx + s * 78, ey + 40, 22, 12, 0, 0, 7); g.fill(); }); }
  if (L.extra.includes('freckles')) { g.fillStyle = '#C98A64'; [-1, 1].forEach((s) => [[0, 0], [12, 8], [-10, 10]].forEach(([dx, dy]) => { g.beginPath(); g.arc(fx + s * (74 + dx), ey + 36 + dy, 3.5, 0, 7); g.fill(); })); }
  // mouth
  const my = ey + 62, t = S.talk || 0;
  g.strokeStyle = INK; g.fillStyle = INK; g.lineWidth = 8;
  const openMouth = (w: number, h: number, tongue = true) => {
    g.beginPath(); g.ellipse(fx, my + h * 0.3, w, Math.max(4, h), 0, 0, 7); g.fillStyle = '#3A1A2A'; g.fill(); g.stroke();
    if (tongue && h > 12) { g.save(); g.clip(); g.fillStyle = '#E2697A'; g.beginPath(); g.ellipse(fx, my + h * 1.1, w * 0.7, h * 0.6, 0, 0, 7); g.fill(); g.restore(); }
  };
  if (m === 'shock') openMouth(16, 20 + t * 6);
  else if (m === 'laugh') { g.beginPath(); g.moveTo(fx - 40, my - 6); g.quadraticCurveTo(fx, my - 2, fx + 40, my - 6); g.quadraticCurveTo(fx + 34, my + 52, fx, my + 52); g.quadraticCurveTo(fx - 34, my + 52, fx - 40, my - 6); g.closePath(); g.fillStyle = '#3A1A2A'; g.fill(); g.save(); g.clip(); g.fillStyle = '#E2697A'; g.beginPath(); g.ellipse(fx, my + 50, 24, 18, 0, 0, 7); g.fill(); g.fillStyle = '#FFFFFF'; g.fillRect(fx - 40, my - 10, 80, 12); g.restore(); g.stroke(); }
  else if (t > 0.05) openMouth(14 + t * 10, 6 + t * 18);
  else if (m === 'happy' || m === 'proud') { g.beginPath(); g.moveTo(fx - 28, my); g.quadraticCurveTo(fx, my + 26, fx + 28, my); g.stroke(); }
  else if (m === 'smug' || m === 'sly') { g.beginPath(); g.moveTo(fx - 22, my + 6); g.quadraticCurveTo(fx + 4, my + 16, fx + 24, my - 4); g.stroke(); }
  else if (m === 'worried') { g.beginPath(); g.moveTo(fx - 26, my + 8); g.bezierCurveTo(fx - 12, my - 4, fx + 8, my + 14, fx + 26, my + 2); g.stroke(); }
  else if (m === 'angry') { g.beginPath(); g.moveTo(fx - 30, my + 12); g.quadraticCurveTo(fx, my - 8, fx + 30, my + 12); g.stroke(); g.lineWidth = 5; g.fillStyle = '#FFFFFF'; g.beginPath(); g.roundRect(fx - 24, my - 2, 48, 14, 6); g.fill(); g.stroke(); }
  else if (m === 'sad') { g.beginPath(); g.moveTo(fx - 24, my + 12); g.quadraticCurveTo(fx, my - 4, fx + 24, my + 12); g.stroke(); }
  else if (m === 'tired' || m === 'dizzy' || m === 'deadpan') { g.beginPath(); g.moveTo(fx - 18, my + 6); g.lineTo(fx + 18, my + 6); g.stroke(); }
  else if (m === 'determined') { g.beginPath(); g.moveTo(fx - 24, my + 5); g.lineTo(fx + 24, my + 1); g.stroke(); }
  else if (m === 'embarrassed') { g.beginPath(); g.moveTo(fx - 17, my + 4); g.quadraticCurveTo(fx - 8, my - 2, fx, my + 4); g.quadraticCurveTo(fx + 8, my + 10, fx + 17, my + 4); g.stroke(); }
  else if (m === 'awe') openMouth(11, 13, false);
  else if (m === 'suspicious') { g.beginPath(); g.moveTo(fx - 22, my); g.quadraticCurveTo(fx, my + 2, fx + 24, my + 9); g.stroke(); }
  else if (m === 'puzzled') { g.beginPath(); g.moveTo(fx - 16, my + 5); g.quadraticCurveTo(fx, my + 1, fx + 16, my - 1); g.stroke(); }
  else if (m === 'relieved') { g.beginPath(); g.moveTo(fx - 20, my); g.quadraticCurveTo(fx, my + 18, fx + 20, my); g.stroke(); g.fillStyle = '#3A1A2A'; g.beginPath(); g.ellipse(fx, my + 16, 5, 3, 0, 0, 7); g.fill(); }
  else { g.beginPath(); g.moveTo(fx - 20, my + 2); g.quadraticCurveTo(fx, my + 14, fx + 20, my + 2); g.stroke(); }
  if (L.extra.includes('mustache')) {
    g.fillStyle = L.mustacheCol || '#3A2A22'; g.strokeStyle = INK; g.lineWidth = 4;
    g.beginPath(); g.moveTo(fx, my - 18); g.bezierCurveTo(fx - 20, my - 32, fx - 60, my - 22, fx - 64, my - 2); g.bezierCurveTo(fx - 44, my - 8, fx - 20, my - 4, fx, my - 10);
    g.bezierCurveTo(fx + 20, my - 4, fx + 44, my - 8, fx + 64, my - 2); g.bezierCurveTo(fx + 60, my - 22, fx + 20, my - 32, fx, my - 18); g.fill(); g.stroke();
  }
  if (L.extra.includes('pencil')) { g.save(); g.translate(fx + 132, hy - 50); g.rotate(-0.5); g.fillStyle = '#F2C230'; g.fillRect(-8, -60, 16, 120); g.fillStyle = '#E58A9A'; g.fillRect(-8, -70, 16, 14); g.strokeStyle = INK; g.lineWidth = 4; g.strokeRect(-8, -70, 16, 130); g.restore(); }
  if (S.sweat || m === 'worried') { const k = S.sweat ?? 1; g.globalAlpha = Math.min(1, k); g.fillStyle = '#8FD3F4'; g.strokeStyle = INK; g.lineWidth = 4; g.beginPath(); const sx = fx + 104, sy = hy - 70; g.moveTo(sx, sy - 26); g.quadraticCurveTo(sx + 18, sy, sx, sy + 12); g.quadraticCurveTo(sx - 18, sy, sx, sy - 26); g.fill(); g.stroke(); g.globalAlpha = 1; }
  if (S.tears || m === 'sad') { g.fillStyle = '#8FD3F4'; [-1, 1].forEach((s) => { g.beginPath(); g.ellipse(fx + s * (ex + 6), ey + 40, 7, 14, 0, 0, 7); g.fill(); }); }
}

function drawHat(g: CanvasRenderingContext2D, L: ResolvedLook, hx: number, hy: number, r: number) {
  const c = L.hatCol || '#7A5236', top = hy - r * 0.97;
  g.strokeStyle = INK; g.lineWidth = LW; g.fillStyle = c;
  const band = (y: number, w: number, col: string) => { g.fillStyle = col; g.fillRect(hx - w, y - 12, w * 2, 24); g.strokeRect(hx - w, y - 12, w * 2, 24); };
  if (L.hat === 'cap') {                                  // newsboy cap
    g.beginPath(); g.ellipse(hx + 6, top + 34, r * 0.98, 62, -0.05, Math.PI, 0); g.closePath(); g.fill(); g.stroke();
    g.fillStyle = shadeOf(c, 0.75); g.beginPath(); g.ellipse(hx - 24, top + 40, r * 0.78, 22, 0.05, 0, Math.PI); g.fill(); g.stroke();
    g.fillStyle = shadeOf(c, 1.15); g.beginPath(); g.arc(hx + 6, top - 26, 12, 0, 7); g.fill(); g.stroke();
  } else if (L.hat === 'bowler') {
    g.beginPath(); g.ellipse(hx, top + 26, r * 1.02, 22, 0, 0, 7); g.fill(); g.stroke();
    g.beginPath(); g.ellipse(hx, top + 14, r * 0.66, 80, 0, Math.PI, 0); g.closePath(); g.fill(); g.stroke();
    band(top + 4, r * 0.64, L.band || shadeOf(c, 0.6));
  } else if (L.hat === 'boater') {
    g.fillStyle = c; g.beginPath(); g.ellipse(hx, top + 30, r * 1.3, 26, 0, 0, 7); g.fill(); g.stroke();
    g.beginPath(); g.roundRect(hx - r * 0.66, top - 46, r * 1.32, 76, [16, 16, 4, 4]); g.fill(); g.stroke();
    band(top + 8, r * 0.66, L.band || '#3C6FB6');
    g.strokeStyle = shadeOf(c, 0.82); g.lineWidth = 3; for (let x = -r * 0.6; x < r * 0.6; x += 16) { g.beginPath(); g.moveTo(hx + x, top - 40); g.lineTo(hx + x + 6, top - 6); g.stroke(); }
  } else if (L.hat === 'beanie') {
    g.beginPath(); g.ellipse(hx, top + 50, r * 0.95, 96, 0, Math.PI, 0); g.closePath(); g.fill(); g.stroke();
    g.fillStyle = shadeOf(c, 0.8); g.beginPath(); g.roundRect(hx - r * 0.98, top + 30, r * 1.96, 40, 18); g.fill(); g.stroke();
    g.fillStyle = L.band || '#F4EFE4'; g.beginPath(); g.arc(hx, top - 52, 22, 0, 7); g.fill(); g.stroke();
  } else if (L.hat === 'visor') {
    g.fillStyle = 'rgba(70,190,120,0.85)'; g.beginPath(); g.moveTo(hx - r * 1.0, top + 70); g.quadraticCurveTo(hx, top + 10, hx + r * 1.0, top + 70); g.lineTo(hx + r * 1.15, top + 98); g.quadraticCurveTo(hx, top + 54, hx - r * 1.15, top + 98); g.closePath(); g.fill(); g.stroke();
    g.fillStyle = c; g.beginPath(); g.moveTo(hx - r * 0.96, top + 64); g.quadraticCurveTo(hx, top + 4, hx + r * 0.96, top + 64); g.lineTo(hx + r * 0.9, top + 80); g.quadraticCurveTo(hx, top + 24, hx - r * 0.9, top + 80); g.closePath(); g.fill(); g.stroke();
  } else if (L.hat === 'tophat') {
    g.beginPath(); g.ellipse(hx, top + 24, r * 1.0, 20, 0, 0, 7); g.fill(); g.stroke();
    g.beginPath(); g.roundRect(hx - r * 0.6, top - 140, r * 1.2, 164, 8); g.fill(); g.stroke();
    band(top - 4, r * 0.6, L.band || '#B5372E');
  } else if (L.hat === 'bow') {
    const fx = hx + r * 0.5, fy = top + 26, c2 = L.hatCol || '#D8343F'; g.fillStyle = c2; g.lineWidth = 6;
    [-1, 1].forEach((s2) => { g.beginPath(); g.moveTo(fx, fy); g.bezierCurveTo(fx + s2 * 30, fy - 46, fx + s2 * 66, fy - 30, fx + s2 * 60, fy); g.bezierCurveTo(fx + s2 * 66, fy + 30, fx + s2 * 30, fy + 40, fx, fy); g.fill(); g.stroke(); });
    g.beginPath(); g.arc(fx, fy, 15, 0, 7); g.fill(); g.stroke();
    g.fillStyle = L.hairCol || '#6B4A3A'; [-1, 1].forEach((s2) => { g.beginPath(); g.ellipse(hx + s2 * (r + 4), hy + 10, 22, 50, s2 * -0.2, 0, 7); g.fill(); g.stroke(); });
  } else if (L.hat === 'mailcap') {
    g.fillStyle = c; g.beginPath(); g.roundRect(hx - r * 0.72, top - 40, r * 1.44, 70, [20, 20, 4, 4]); g.fill(); g.stroke();
    g.fillStyle = INK; g.beginPath(); g.ellipse(hx - 10, top + 34, r * 0.82, 18, 0.04, 0, Math.PI); g.fill();
    g.fillStyle = '#E2B33C'; g.beginPath(); g.arc(hx, top - 6, 13, 0, 7); g.fill(); g.lineWidth = 4; g.stroke();
  } else if (L.hat === 'flower') {
    const fx = hx + r * 0.62, fy = top + 40;
    for (let i = 0; i < 6; i++) { const a = i / 6 * Math.PI * 2; g.fillStyle = L.hatCol || '#F29BB5'; g.beginPath(); g.arc(fx + Math.cos(a) * 18, fy + Math.sin(a) * 18, 14, 0, 7); g.fill(); g.lineWidth = 4; g.stroke(); }
    g.fillStyle = '#F6C945'; g.beginPath(); g.arc(fx, fy, 11, 0, 7); g.fill(); g.stroke();
  }
}

// ---------- props held in a hand: (g, x, y, angle, side) ----------
export const PROPS: PropsMap = {
  parcels: (g, x, y) => { [[0, 0, 150, 90], [10, -88, 120, 80], [-6, -160, 96, 64]].forEach(([dx, dy, w, h], i) => { g.fillStyle = ['#C8935A', '#B9844E', '#D3A26A'][i]; g.strokeStyle = INK; g.lineWidth = 6; g.beginPath(); g.rect(x - w / 2 + dx - 40, y - h + dy, w, h); g.fill(); g.stroke(); g.fillStyle = '#E9D7A5'; g.fillRect(x + dx - 46, y - h + dy, 12, h); g.fillStyle = '#D8343F'; g.fillRect(x + dx + w / 2 - 70, y - h + dy + 10, 18, 22); }); },
  book: (title = 'RULES', open = 0) => (g, x, y) => { g.save(); g.translate(x, y - 70); g.strokeStyle = INK; g.lineWidth = 6;
    if (open > 0.5) { g.fillStyle = '#F4EFE4'; g.beginPath(); g.rect(-110, -70, 110, 140); g.rect(0, -70, 110, 140); g.fill(); g.stroke(); g.fillStyle = '#9B958A'; for (let i = 0; i < 6; i++) { g.fillRect(-96, -50 + i * 20, 82, 6); g.fillRect(14, -50 + i * 20, 82, 6); } }
    else { g.fillStyle = '#8A2E2E'; g.beginPath(); g.roundRect(-80, -100, 160, 200, 8); g.fill(); g.stroke(); g.fillStyle = '#E2B33C'; g.font = '900 30px Inter'; g.textAlign = 'center'; g.fillText(title, 0, -20); g.fillRect(-50, 10, 100, 6); }
    g.restore(); },
  ticket: (g, x, y) => { g.save(); g.translate(x + 10, y - 40); g.rotate(-0.2); g.fillStyle = '#F2D37A'; g.strokeStyle = INK; g.lineWidth = 5; g.beginPath(); g.roundRect(-70, -40, 140, 80, 6); g.fill(); g.stroke(); g.fillStyle = INK; g.font = '900 24px Inter'; g.textAlign = 'center'; g.fillText('TICKET', 0, -4); g.font = '900 30px Inter'; g.fillStyle = '#B5372E'; g.fillText('$$$', 0, 30); g.restore(); },
  purse: (g, x, y) => { g.fillStyle = '#7A5236'; g.strokeStyle = INK; g.lineWidth = 5; g.beginPath(); g.ellipse(x, y - 30, 44, 34, 0, 0, 7); g.fill(); g.stroke(); g.strokeStyle = '#C9A13B'; g.beginPath(); g.moveTo(x - 30, y - 56); g.lineTo(x + 30, y - 56); g.stroke(); g.fillStyle = '#C9C2B5'; g.beginPath(); g.arc(x + 30, y - 6, 4, 0, 7); g.fill(); },
  letters: (g, x, y) => { for (let i = 0; i < 3; i++) { g.save(); g.translate(x - 10 + i * 10, y - 40 - i * 8); g.rotate(-0.3 + i * 0.2); g.fillStyle = i === 1 ? '#F2E6CF' : '#FFFDF4'; g.strokeStyle = INK; g.lineWidth = 4; g.beginPath(); g.rect(-50, -30, 100, 60); g.fill(); g.stroke(); g.beginPath(); g.moveTo(-50, -30); g.lineTo(0, 6); g.lineTo(50, -30); g.stroke(); g.fillStyle = '#D8343F'; g.fillRect(28, -24, 14, 16); g.restore(); } },
  rubberStamp: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 5; g.fillStyle = '#7A5236'; g.beginPath(); g.roundRect(x - 14, y - 90, 28, 60, 8); g.fill(); g.stroke(); g.fillStyle = '#B5372E'; g.beginPath(); g.roundRect(x - 40, y - 34, 80, 28, 4); g.fill(); g.stroke(); g.fillStyle = '#7A5236'; g.beginPath(); g.arc(x, y - 100, 20, 0, 7); g.fill(); g.stroke(); },
  stamp: (g, x, y) => { g.save(); g.translate(x, y - 40); g.fillStyle = '#FFFDF4'; g.strokeStyle = INK; g.lineWidth = 4; g.beginPath(); g.rect(-24, -30, 48, 60); g.fill(); g.stroke(); g.fillStyle = '#D8343F'; g.fillRect(-16, -22, 32, 36); g.restore(); },
  cash: (n = 1) => (g, x, y) => { for (let i = 0; i < n; i++) bill(g, x - 30, y - 34 - i * 12, 0.06 * (i % 2 ? -1 : 1)); },
  bill: (g, x, y) => bill(g, x - 10, y - 50, -0.5),
  stack: (g, x, y) => { for (let i = 0; i < 5; i++) { g.fillStyle = i % 2 ? '#7FBF7A' : '#8FD08A'; g.strokeStyle = INK; g.lineWidth = 5; g.beginPath(); g.roundRect(x - 60, y - 40 - i * 16, 120, 18, 4); g.fill(); g.stroke(); } g.fillStyle = '#E9D7A5'; g.fillRect(x - 12, y - 120, 24, 82); g.strokeRect(x - 12, y - 120, 24, 82); },
  paper: (text = 'NEWS') => (g, x, y) => { g.save(); g.translate(x, y - 70); g.rotate(-0.08); g.fillStyle = '#F2EDE0'; g.strokeStyle = INK; g.lineWidth = 5; g.beginPath(); g.rect(-70, -90, 140, 180); g.fill(); g.stroke(); g.fillStyle = INK; g.font = '900 30px Inter'; g.textAlign = 'center'; g.fillText(text, 0, -50); g.fillStyle = '#9B958A'; for (let i = 0; i < 6; i++) g.fillRect(-56, -26 + i * 18, i === 2 ? 60 : 112, 7); g.restore(); },
  mirror: (g, x, y) => { g.strokeStyle = '#B8862F'; g.lineWidth = 12; g.beginPath(); g.moveTo(x, y); g.lineTo(x, y - 60); g.stroke(); g.fillStyle = '#E6B850'; g.strokeStyle = INK; g.lineWidth = 6; g.beginPath(); g.arc(x, y - 110, 54, 0, 7); g.fill(); g.stroke(); g.fillStyle = '#BFE6F5'; g.beginPath(); g.arc(x, y - 110, 40, 0, 7); g.fill(); g.fillStyle = 'rgba(255,255,255,0.8)'; g.beginPath(); g.ellipse(x - 14, y - 124, 9, 18, 0.6, 0, 7); g.fill(); },
  cup: (g, x, y) => { g.fillStyle = '#F4EFE4'; g.strokeStyle = INK; g.lineWidth = 6; g.beginPath(); g.roundRect(x - 30, y - 70, 60, 70, [4, 4, 14, 14]); g.fill(); g.stroke(); g.beginPath(); g.arc(x + 34, y - 36, 16, -1.4, 1.4); g.stroke(); g.fillStyle = '#7A4A2A'; g.fillRect(x - 24, y - 64, 48, 10); },
  key: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 5; g.fillStyle = '#E6B850'; g.beginPath(); g.arc(x, y - 60, 22, 0, 7); g.fill(); g.stroke(); g.fillRect(x - 5, y - 40, 10, 50); g.strokeRect(x - 5, y - 40, 10, 50); },
  sign: (text = 'SALE', col = '#F2EDE0') => (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#9A6A43'; g.fillRect(x - 6, y - 150, 12, 160); g.strokeRect(x - 6, y - 150, 12, 160); g.fillStyle = col; g.beginPath(); g.roundRect(x - 100, y - 250, 200, 110, 10); g.fill(); g.stroke(); g.fillStyle = INK; g.font = '900 44px Inter'; g.textAlign = 'center'; g.textBaseline = 'middle'; g.fillText(text, x, y - 194); },
  // ---------- v5 props (additive) — reach = farthest painted point from the grip (0,0), measured from the draw code; contract: ≤190 ----------
  axe: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#9A6A43'; g.beginPath(); g.roundRect(x - 8, y - 135, 16, 150, 7); g.fill(); g.stroke(); g.fillStyle = '#C9CDD4'; g.beginPath(); g.moveTo(x + 4, y - 138); g.lineTo(x + 18, y - 95); g.lineTo(x + 2, y - 88); g.quadraticCurveTo(x - 30, y - 104, x - 45, y - 150); g.quadraticCurveTo(x - 22, y - 140, x + 4, y - 138); g.closePath(); g.fill(); g.stroke(); g.fillStyle = '#8A8D91'; g.fillRect(x - 4, y - 140, 12, 46); },
  hammer: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#9A6A43'; g.beginPath(); g.roundRect(x - 7, y - 100, 14, 120, 6); g.fill(); g.stroke(); g.fillStyle = '#B9BEC7'; g.beginPath(); g.roundRect(x - 52, y - 128, 104, 32, 6); g.fill(); g.stroke(); g.fillStyle = '#8A8D91'; g.fillRect(x - 52, y - 118, 14, 12); g.fillRect(x + 38, y - 118, 14, 12); },
  wrench: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#B9BEC7'; g.beginPath(); g.roundRect(x - 9, y - 95, 18, 113, 8); g.fill(); g.stroke(); g.beginPath(); g.arc(x, y - 104, 19, 0, 7); g.fill(); g.stroke(); g.fillStyle = '#FFFFFF'; g.beginPath(); g.moveTo(x, y - 104); g.lineTo(x + 20, y - 122); g.lineTo(x + 22, y - 96); g.closePath(); g.fill(); },
  shovel: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#9A6A43'; g.fillRect(x - 7, y - 140, 14, 200); g.strokeRect(x - 7, y - 140, 14, 200); g.lineWidth = 7; g.beginPath(); g.roundRect(x - 20, y - 168, 40, 34, 10); g.stroke(); g.fillStyle = '#8A8D91'; g.beginPath(); g.moveTo(x - 13, y + 52); g.lineTo(x + 13, y + 52); g.lineTo(x + 26, y + 128); g.quadraticCurveTo(x + 24, y + 168, x, y + 186); g.quadraticCurveTo(x - 24, y + 168, x - 26, y + 128); g.closePath(); g.fill(); g.stroke(); },
  apple: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#D8343F'; g.beginPath(); g.arc(x, y - 26, 24, 0, 7); g.fill(); g.stroke(); g.strokeStyle = '#6B4A3A'; g.lineWidth = 5; g.beginPath(); g.moveTo(x, y - 44); g.quadraticCurveTo(x + 4, y - 54, x + 8, y - 56); g.stroke(); g.fillStyle = '#5DA83E'; g.strokeStyle = INK; g.lineWidth = 4; g.beginPath(); g.ellipse(x + 14, y - 50, 11, 6, -0.5, 0, 7); g.fill(); g.stroke(); g.fillStyle = 'rgba(255,255,255,0.55)'; g.beginPath(); g.ellipse(x - 9, y - 34, 6, 10, 0.4, 0, 7); g.fill(); },
  bottle: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#3E7C59'; g.beginPath(); g.roundRect(x - 17, y - 80, 34, 80, 8); g.fill(); g.stroke(); g.beginPath(); g.moveTo(x - 17, y - 80); g.quadraticCurveTo(x - 8, y - 92, x - 7, y - 100); g.lineTo(x + 7, y - 100); g.quadraticCurveTo(x + 8, y - 92, x + 17, y - 80); g.fill(); g.fillRect(x - 7, y - 104, 14, 8); g.fillStyle = '#C9A13B'; g.fillRect(x - 9, y - 112, 18, 9); g.strokeRect(x - 9, y - 112, 18, 9); g.fillStyle = '#F2E6CF'; g.fillRect(x - 12, y - 62, 24, 30); },
  sandwich: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#EFBE7D'; g.beginPath(); g.moveTo(x - 40, y - 40); g.lineTo(x + 40, y - 40); g.lineTo(x, y + 6); g.closePath(); g.fill(); g.stroke(); g.strokeStyle = '#5DA83E'; g.lineWidth = 5; g.beginPath(); g.moveTo(x - 30, y - 26); for (let i = 0; i < 4; i++) g.quadraticCurveTo(x - 22 + i * 16, y - 34, x - 14 + i * 16, y - 26); g.stroke(); g.fillStyle = '#D8343F'; g.beginPath(); g.arc(x - 8, y - 18, 7, 0, 7); g.fill(); g.fillStyle = '#F6C945'; g.beginPath(); g.moveTo(x - 24, y - 12); g.lineTo(x + 16, y - 12); g.lineTo(x - 4, y + 2); g.closePath(); g.fill(); },
  fish: (g, x, y) => { g.save(); g.translate(x + 22, y - 38); g.rotate(-0.6); g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#E8873A'; g.beginPath(); g.ellipse(0, 0, 55, 24, 0, 0, 7); g.fill(); g.stroke(); g.beginPath(); g.moveTo(-50, 0); g.lineTo(-78, -20); g.lineTo(-78, 20); g.closePath(); g.fill(); g.stroke(); g.fillStyle = '#FFFFFF'; g.beginPath(); g.arc(34, -6, 8, 0, 7); g.fill(); g.stroke(); g.fillStyle = INK; g.beginPath(); g.arc(36, -6, 4, 0, 7); g.fill(); g.strokeStyle = '#C25E1E'; g.lineWidth = 4; [-16, 0, 16].forEach((dx) => { g.beginPath(); g.arc(dx, 0, 12, -0.9, 0.9); g.stroke(); }); g.restore(); },
  flower: (g, x, y) => { g.strokeStyle = '#3E7C43'; g.lineWidth = 6; g.beginPath(); g.moveTo(x, y + 10); g.quadraticCurveTo(x - 4, y - 30, x, y - 70); g.stroke(); g.fillStyle = '#5DA83E'; g.strokeStyle = INK; g.lineWidth = 4; g.beginPath(); g.ellipse(x - 13, y - 38, 12, 6, -0.6, 0, 7); g.fill(); g.stroke(); for (let i = 0; i < 6; i++) { const a = i / 6 * Math.PI * 2; g.fillStyle = '#F29BB5'; g.beginPath(); g.arc(x + Math.cos(a) * 15, y - 86 + Math.sin(a) * 15, 12, 0, 7); g.fill(); g.lineWidth = 4; g.stroke(); } g.fillStyle = '#F6C945'; g.beginPath(); g.arc(x, y - 86, 10, 0, 7); g.fill(); g.stroke(); },
  egg: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#F7F1E3'; g.beginPath(); g.ellipse(x, y - 28, 21, 27, 0, 0, 7); g.fill(); g.stroke(); g.strokeStyle = '#C9BFA8'; g.lineWidth = 4; g.beginPath(); g.moveTo(x - 20, y - 30); g.lineTo(x - 8, y - 22); g.lineTo(x + 2, y - 31); g.lineTo(x + 12, y - 23); g.lineTo(x + 20, y - 29); g.stroke(); },
  feather: (g, x, y) => { g.save(); g.translate(x, y); g.rotate(0.3); g.strokeStyle = INK; g.lineWidth = 5; g.fillStyle = '#BFD9EA'; g.beginPath(); g.moveTo(0, 10); g.bezierCurveTo(-24, -20, -20, -70, 6, -104); g.bezierCurveTo(26, -66, 22, -18, 0, 10); g.closePath(); g.fill(); g.stroke(); g.strokeStyle = '#7FA3BC'; g.lineWidth = 3; g.beginPath(); g.moveTo(0, 10); g.quadraticCurveTo(2, -50, 6, -102); g.stroke(); g.restore(); },
  ball: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#F4EFE4'; g.beginPath(); g.arc(x, y - 36, 34, 0, 7); g.fill(); g.stroke(); g.fillStyle = '#D8343F'; g.beginPath(); g.arc(x, y - 36, 34, 0.9, 2.2); g.lineTo(x, y - 36); g.closePath(); g.fill(); g.beginPath(); g.arc(x, y - 36, 34, 3.6, 4.9); g.lineTo(x, y - 36); g.closePath(); g.fill(); g.fillStyle = '#3C6FB6'; g.beginPath(); g.arc(x, y - 36, 34, 5.6, 6.9); g.lineTo(x, y - 36); g.closePath(); g.fill(); },
  trophy: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#E6B850'; g.beginPath(); g.moveTo(x - 30, y - 95); g.lineTo(x + 30, y - 95); g.quadraticCurveTo(x + 28, y - 48, x + 8, y - 40); g.lineTo(x - 8, y - 40); g.quadraticCurveTo(x - 28, y - 48, x - 30, y - 95); g.closePath(); g.fill(); g.stroke(); g.beginPath(); g.arc(x - 32, y - 72, 13, Math.PI * 0.5, Math.PI * 1.6); g.stroke(); g.beginPath(); g.arc(x + 32, y - 72, 13, -Math.PI * 0.6, Math.PI * 0.5); g.stroke(); g.fillRect(x - 6, y - 40, 12, 20); g.strokeRect(x - 6, y - 40, 12, 20); g.fillRect(x - 22, y - 20, 44, 14); g.strokeRect(x - 22, y - 20, 44, 14); g.fillStyle = '#FFF3C4'; g.beginPath(); g.arc(x, y - 72, 9, 0, 7); g.fill(); },
  phone: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#2A2A33'; g.beginPath(); g.roundRect(x - 21, y - 78, 42, 76, 8); g.fill(); g.stroke(); g.fillStyle = '#BFE6F5'; g.fillRect(x - 15, y - 64, 30, 48); g.fillStyle = '#8A8D91'; g.beginPath(); g.arc(x, y - 70, 3, 0, 7); g.fill(); g.fillStyle = '#F4EFE4'; g.beginPath(); g.arc(x, y - 9, 4, 0, 7); g.fill(); },
  camera: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.fillStyle = '#4A4A52'; g.beginPath(); g.roundRect(x - 46, y - 78, 92, 72, 8); g.fill(); g.stroke(); g.fillStyle = '#6E6A6A'; g.fillRect(x - 14, y - 92, 24, 14); g.strokeRect(x - 14, y - 92, 24, 14); g.fillStyle = '#B9BEC7'; g.beginPath(); g.arc(x, y - 42, 20, 0, 7); g.fill(); g.stroke(); g.fillStyle = '#2B4A70'; g.beginPath(); g.arc(x, y - 42, 12, 0, 7); g.fill(); g.fillStyle = 'rgba(255,255,255,0.7)'; g.beginPath(); g.arc(x - 4, y - 46, 4, 0, 7); g.fill(); g.fillStyle = '#D8343F'; g.beginPath(); g.arc(x + 36, y - 68, 5, 0, 7); g.fill(); },
  radio: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.beginPath(); g.moveTo(x + 26, y - 90); g.lineTo(x + 64, y - 140); g.stroke(); g.fillStyle = '#D8343F'; g.beginPath(); g.arc(x + 64, y - 140, 5, 0, 7); g.fill(); g.stroke(); g.fillStyle = '#7A5236'; g.beginPath(); g.roundRect(x - 52, y - 96, 82, 100, 8); g.fill(); g.stroke(); g.fillStyle = '#3A2A22'; for (let r = 0; r < 3; r++) for (let c = 0; c < 3; c++) { g.beginPath(); g.arc(x - 34 + c * 13, y - 62 + r * 15, 4, 0, 7); g.fill(); } g.fillStyle = '#F2D37A'; g.beginPath(); g.arc(x + 8, y - 52, 13, 0, 7); g.fill(); g.stroke(); g.fillStyle = '#F4EFE4'; g.fillRect(x - 44, y - 88, 66, 14); },
  umbrella: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.beginPath(); g.moveTo(x, y - 100); g.lineTo(x, y + 4); g.stroke(); g.beginPath(); g.arc(x + 8, y + 6, 8, 0, Math.PI); g.stroke(); g.fillStyle = '#3C6FB6'; g.beginPath(); g.moveTo(x - 100, y - 56); g.quadraticCurveTo(x, y - 128, x + 100, y - 56); g.quadraticCurveTo(x + 66, y - 42, x + 33, y - 56); g.quadraticCurveTo(x, y - 42, x - 33, y - 56); g.quadraticCurveTo(x - 66, y - 42, x - 100, y - 56); g.closePath(); g.fill(); g.stroke(); g.fillStyle = '#2A2A33'; g.fillRect(x - 3, y - 116, 6, 14); },
  lantern: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 6; g.beginPath(); g.arc(x, y - 96, 16, Math.PI, 0); g.stroke(); g.fillStyle = '#3A3A3F'; g.fillRect(x - 20, y - 96, 40, 12); g.strokeRect(x - 20, y - 96, 40, 12); g.fillStyle = '#FFDF8A'; g.beginPath(); g.roundRect(x - 22, y - 84, 44, 60, 6); g.fill(); g.stroke(); g.fillStyle = '#FFB84D'; g.beginPath(); g.ellipse(x, y - 52, 8, 13, 0, 0, 7); g.fill(); g.fillStyle = '#3A3A3F'; g.fillRect(x - 24, y - 24, 48, 12); g.strokeRect(x - 24, y - 24, 48, 12); },
  map: (text = '') => (g, x, y) => { g.save(); g.translate(x, y - 58); g.rotate(-0.05); g.fillStyle = '#F2E6CF'; g.strokeStyle = INK; g.lineWidth = 5; g.beginPath(); g.rect(-75, -55, 150, 110); g.fill(); g.stroke(); g.strokeStyle = '#C9BFA8'; g.lineWidth = 3; g.beginPath(); g.moveTo(-25, -55); g.lineTo(-25, 55); g.moveTo(25, -55); g.lineTo(25, 55); g.stroke(); g.strokeStyle = '#D8343F'; g.lineWidth = 4; g.setLineDash([8, 6]); g.beginPath(); g.moveTo(-58, 34); g.quadraticCurveTo(-10, -6, 30, -18); g.stroke(); g.setLineDash([]); g.lineWidth = 6; g.beginPath(); g.moveTo(38, -34); g.lineTo(56, -16); g.moveTo(56, -34); g.lineTo(38, -16); g.stroke(); if (text) { g.fillStyle = INK; g.font = '900 20px Inter'; g.textAlign = 'center'; g.fillText(text, 0, 46); } g.restore(); },
  rope: (g, x, y) => { g.strokeStyle = '#9A6A43'; g.lineWidth = 9; [-16, -34, -52].forEach((cy) => { g.beginPath(); g.ellipse(x, y + cy, 40, 15, 0, 0, 7); g.stroke(); }); g.strokeStyle = INK; g.lineWidth = 3; [-16, -34, -52].forEach((cy) => { g.beginPath(); g.ellipse(x, y + cy, 40, 15, 0, 0, 7); g.stroke(); }); g.strokeStyle = '#9A6A43'; g.lineWidth = 8; g.beginPath(); g.moveTo(x + 34, y - 20); g.quadraticCurveTo(x + 48, y + 6, x + 30, y + 26); g.stroke(); },
  medal: (g, x, y) => { g.strokeStyle = INK; g.lineWidth = 5; g.fillStyle = '#3C6FB6'; g.beginPath(); g.moveTo(x - 4, y); g.lineTo(x - 18, y + 36); g.lineTo(x - 6, y + 40); g.lineTo(x, y + 10); g.closePath(); g.fill(); g.stroke(); g.fillStyle = '#D8343F'; g.beginPath(); g.moveTo(x + 4, y); g.lineTo(x, y + 10); g.lineTo(x + 6, y + 40); g.lineTo(x + 18, y + 36); g.closePath(); g.fill(); g.stroke(); g.fillStyle = '#E6B850'; g.beginPath(); g.arc(x, y + 58, 23, 0, 7); g.fill(); g.stroke(); g.fillStyle = '#FFF3C4'; g.beginPath(); for (let i = 0; i < 5; i++) { const a = -Math.PI / 2 + i * (Math.PI * 4 / 5); g.lineTo(x + Math.cos(a) * 12, y + 58 + Math.sin(a) * 12); } g.closePath(); g.fill(); },
};
function bill(g: CanvasRenderingContext2D, x: number, y: number, rot: number) {
  g.save(); g.translate(x, y); g.rotate(rot); g.fillStyle = '#9ED39A'; g.strokeStyle = INK; g.lineWidth = 5;
  g.beginPath(); g.roundRect(-10, -26, 100, 52, 6); g.fill(); g.stroke();
  g.strokeStyle = '#4F8F55'; g.lineWidth = 3; g.beginPath(); g.roundRect(-2, -18, 84, 36, 4); g.stroke();
  g.fillStyle = '#4F8F55'; g.beginPath(); g.arc(40, 0, 11, 0, 7); g.fill(); g.restore();
}

// ---------- the 3D cut-out ----------
let shadowTex: THREE.CanvasTexture | null = null;
function blobTex() {
  if (shadowTex) return shadowTex;
  const c = document.createElement('canvas'); c.width = c.height = 128; const g = c.getContext('2d')!;
  const gr = g.createRadialGradient(64, 64, 0, 64, 64, 64); gr.addColorStop(0, 'rgba(30,20,40,0.75)'); gr.addColorStop(0.6, 'rgba(30,20,40,0.25)'); gr.addColorStop(1, 'rgba(30,20,40,0)');
  g.fillStyle = gr; g.fillRect(0, 0, 128, 128); shadowTex = new THREE.CanvasTexture(c); shadowTex.colorSpace = THREE.SRGBColorSpace; return shadowTex;
}

export class Toon {
  look: ToonLook; res: number; name: string;
  _overflowKey: string | null; _overflowCache: OverflowCounts | null;
  canvas: HTMLCanvasElement; g: CanvasRenderingContext2D; tex: THREE.CanvasTexture;
  mat: THREE.MeshBasicMaterial; mesh: THREE.Mesh; blob: THREE.Mesh;
  group: THREE.Group; pivot: THREE.Group; key: string; H: number;
  _facePos: THREE.Vector3; st: ToonState | undefined; hands: Hands | undefined;
  perspective: boolean; envMatch: boolean; multiPlane: boolean;
  /** world Y of the surface the character stands on (shots raise it for platforms/benches) */
  groundY = 0;
  headCanvas: HTMLCanvasElement | null; headG: CanvasRenderingContext2D | null; headTex: THREE.CanvasTexture | null;
  headMesh: THREE.Mesh | null; headPivot: THREE.Group | null;
  private _attachments: AttachEntry[] = [];
  private _attQ = new THREE.Quaternion(); private _attOff = new THREE.Vector3(); private _attW = new THREE.Vector3();
  private _envC = new THREE.Color(); private _envFog = new THREE.Color();
  /** height = metres from feet to the top of the head (≈1.75); res = canvas scale (2 = 2560x2816, canvas is 1280x1408 logical — see STAGE) */
  constructor(look: ToonLook = {}, { height = 1.75, res = 2, castShadow = true, perspective = false, multiPlane = false, envMatch = false }: ToonOptions = {}) {
    this.look = look; this.res = res;
    this.perspective = perspective; this.multiPlane = multiPlane; this.envMatch = envMatch;
    this.headCanvas = null; this.headG = null; this.headTex = null; this.headMesh = null; this.headPivot = null;
    this.name = look.name || `toon#${TOONS.length}`;
    TOONS.push(this);
    this._overflowKey = null; this._overflowCache = null;      // overflow() scan cache, keyed by this.key
    this.canvas = document.createElement('canvas'); this.canvas.width = STAGE.CW * res; this.canvas.height = STAGE.CH * res;
    OVERFLOW_CANVASES.push({ canvas: this.canvas, res, label: this.name, pose: () => this.poseSummary() });
    this.g = this.canvas.getContext('2d')!;
    this.tex = new THREE.CanvasTexture(this.canvas); this.tex.colorSpace = THREE.SRGBColorSpace; this.tex.anisotropy = 4;
    const H = height / 0.96;                                   // the head top sits ~4% below the stage top
    // Plane: same unit scale as before (H/1024 m per logical unit, both axes) and the same feet anchor,
    // now spanning the full 1280x1408 canvas: width 1280 units = 1.25H, height 1408 units = 1.375H.
    // Centre derivation: canvas row 1256 (feet) must sit at world 0 and the plane centre is canvas
    // row 704, so the centre sits at (1256 - 704) = 552 (in H/1024 units) above the feet. Stage rows
    // 0 / 1000 / 1024 land at exactly the same world heights as always (1000 / 0 / -24, in H/1024
    // units) — the wider/taller plane only adds transparent margin around the same stage picture.
    // PlaneGeometry's default UVs map the full texture across the plane, so the shadow depth material
    // (same texture) stays aligned with the colour pass.
    const planeW = H * 1280 / 1024, planeH = H * 1408 / 1024;
    const geo = new THREE.PlaneGeometry(planeW, planeH); geo.translate(0, H * 552 / 1024, 0);
    this.mat = new THREE.MeshBasicMaterial({ map: this.tex, transparent: true, alphaTest: 0.04, side: THREE.DoubleSide, toneMapped: false });
    this.mesh = new THREE.Mesh(geo, this.mat);
    this.mesh.castShadow = castShadow;
    this.mesh.customDepthMaterial = new THREE.MeshDepthMaterial({ depthPacking: THREE.RGBADepthPacking, map: this.tex, alphaTest: 0.5 });
    const blobW = (H / 2) * 0.9;                               // blob shadow keeps the FIGURE width (H/2), not the new plane width
    this.blob = new THREE.Mesh(new THREE.PlaneGeometry(blobW, (H / 2) * 0.34), new THREE.MeshBasicMaterial({ map: blobTex(), transparent: true, depthWrite: false, opacity: 0.55 }));
    this.blob.rotation.x = -Math.PI / 2; this.blob.position.y = 0.01;
    this.group = new THREE.Group(); this.pivot = new THREE.Group(); this.pivot.rotation.order = 'YXZ'; this.pivot.add(this.mesh); this.group.add(this.pivot, this.blob);
    this.group.userData.toon = this;
    this.key = ''; this.H = H; this._facePos = new THREE.Vector3();
    if (multiPlane) {
      // Head layer (prototype): a second canvas + plane carrying ONLY the head, centred exactly
      // where the main plane draws it and pushed 0.055 m toward the viewer for parallax.
      // Head-canvas mapping: stage (sx, sy) -> head canvas (sx + 256, sy + 160), so the head
      // canvas centre (512, 512) is stage (256, 352), whose main-plane local coords are
      // ((384 + 256 - 640) * s, (1256 - (256 + 352)) * s) = (0, 648 * s) with s = H / 1024.
      const s = H / 1024;
      this.headCanvas = document.createElement('canvas'); this.headCanvas.width = 1024 * res; this.headCanvas.height = 1024 * res;
      this.headG = this.headCanvas.getContext('2d')!;
      this.headTex = new THREE.CanvasTexture(this.headCanvas); this.headTex.colorSpace = THREE.SRGBColorSpace; this.headTex.anisotropy = 4;
      const headMat = new THREE.MeshBasicMaterial({ map: this.headTex, transparent: true, alphaTest: 0.04, side: THREE.DoubleSide, toneMapped: false });
      this.headMesh = new THREE.Mesh(new THREE.PlaneGeometry(1024 * s, 1024 * s), headMat);
      this.headMesh.castShadow = true;
      this.headMesh.customDepthMaterial = new THREE.MeshDepthMaterial({ depthPacking: THREE.RGBADepthPacking, map: this.headTex, alphaTest: 0.5 });
      this.headMesh.renderOrder = this.mesh.renderOrder + 1;
      this.headPivot = new THREE.Group(); this.headPivot.position.set(0, 648 * s, 0.055);
      this.headPivot.add(this.headMesh); this.pivot.add(this.headPivot);
      OVERFLOW_CANVASES.push({ canvas: this.headCanvas, res, label: this.name + ':head', pose: () => this.poseSummary() });
    }
    this.set({});
  }
  /** redraws only when the state changed */
  set(st: ToonState = {}): this {
    // Invisible cut-outs contribute no colour and no shadow (three.js projectObject and the shadow
    // pass both bail on visible === false, and the blob hangs off the same group), so repainting one
    // cannot change a pixel — skip before paying for the key. this.key is deliberately left alone:
    // shots re-call set() every frame for every toon they show, so the first visible frame re-issues
    // its state and repaints iff it differs from the last *painted* key.
    if (this.group.visible === false) return this;
    const key = JSON.stringify(st, (k: string, v: unknown) => (typeof v === 'function' ? (v.name || String(v).length) : typeof v === 'number' ? Math.round(v * 100) / 100 : v));
    if (key === this.key) { this.syncBlob(); return this; }
    this.key = key; this.st = st;
    // sitOn (metres above the feet line) becomes sitDrop (stage units) for drawToon; the repaint
    // key above is computed from the ORIGINAL st, which already contains sitOn.
    const drawSt: ToonState = st.sitOn != null && st.sitDrop == null ? { ...st, sitDrop: st.sitOn * 1024 / this.H } : st;
    const g = this.g; g.setTransform(1, 0, 0, 1, 0, 0); g.clearRect(0, 0, this.canvas.width, this.canvas.height);
    // Anchor the unchanged 512x1024 stage inside the larger canvas. drawToon returns hands in STAGE
    // coordinates: they are computed purely from stage-space numbers inside drawToon and never read the
    // transform back, so this translate shifts only where stage pixels land on the canvas — the returned
    // hand positions stay in stage space, exactly as before.
    g.scale(this.res, this.res); g.translate(STAGE.OX, STAGE.OY);
    if (this.multiPlane && this.headCanvas && this.headG && this.headTex) {
      this.hands = drawToonLayer(g, this.look, drawSt, 'nohead');
      const g2 = this.headG; g2.setTransform(1, 0, 0, 1, 0, 0); g2.clearRect(0, 0, this.headCanvas.width, this.headCanvas.height);
      // head-canvas stage mapping: stage (sx, sy) -> head canvas (sx + 256, sy + 160)
      g2.scale(this.res, this.res); g2.translate(256, 160); drawToonLayer(g2, this.look, drawSt, 'headonly');
      this.headTex.needsUpdate = true;
    } else {
      this.hands = drawToon(g, this.look, drawSt);
    }
    this.tex.needsUpdate = true; this.syncBlob(); return this;
  }
  /**
   * Border-strip overflow of the last painted frame (see scanCanvasOverflow), or null if this toon
   * has never painted (key ''). Cached: rescans only when this.key changed since the last scan.
   */
  overflow(): OverflowCounts | null {
    if (!this.key) return null;
    if (this._overflowKey === this.key && this._overflowCache) return this._overflowCache;
    const r = scanCanvasOverflow(this.canvas, this.res);
    this._overflowKey = this.key; this._overflowCache = r; return r;
  }
  /** short look summary, e.g. `hat=cap body=#7A9A4A pattern=overalls` */
  describe(): string {
    const L = this.look || {};
    return `hat=${L.hat ?? 'none'} body=${L.body ?? '#E8734A'} pattern=${L.pattern ?? 'plain'}`;
  }
  /** short pose summary from the last painted state, e.g. `mood=happy armL=[8,6] armR=[8,6] legs=stand` */
  poseSummary(): string {
    if (!this.st) return 'never painted';
    const s = this.st, fmt = (a: [number, number] | undefined) => `[${(Array.isArray(a) ? a : [8, 6]).join(',')}]`;   // [8,6] = drawToon's arm default when st omits it
    const base = `mood=${s.mood ?? 'neutral'} armL=${fmt(s.armL)} armR=${fmt(s.armR)} legs=${s.legs ?? 'stand'}`;
    const ht = s.headTilt || 0, hn = s.headTurn || 0;
    return ht || hn ? `${base} head=${ht}/${hn}` : base;
  }
  /**
   * Turn the cut-out to face the camera. By default this is yaw only, exactly as before. With
   * { perspective: true } the plane also pitches toward the camera's height: the pivot order is
   * YXZ, so after the yaw a positive rotation.x tips the plane's top toward the camera — and a
   * camera above the character gives pitchAng > 0, hence rotation.x > 0 (top toward camera);
   * a camera below gives a negative pitch (top away). When perspective is off, rotation.x is
   * never written and stays exactly 0.
   */
  face(cam: { position: THREE.Vector3 }): this {
    if (this.group.visible === false) { this.syncAttachments(); return this; }
    const p = this._facePos; this.group.getWorldPosition(p);
    this.pivot.rotation.y = Math.atan2(cam.position.x - p.x, cam.position.z - p.z) - this.group.rotation.y;
    if (this.perspective) {
      const dy = cam.position.y - (p.y + this.H * 0.6), dh = Math.hypot(cam.position.x - p.x, cam.position.z - p.z);
      const pitchAng = dh < 1e-4 ? 0 : Math.atan2(dy, dh);
      this.pivot.rotation.x = clamp(pitchAng * 0.45, -0.38, 0.38);
    }
    if (this.multiPlane && this.headPivot) this.headPivot.rotation.y = (this.st?.headTurn || 0) * -0.22;
    this.syncBlob(); this.syncAttachments(); return this;
  }
  /**
   * Keep the blob shadow glued to the ground: air = height above groundY (clamped 0..3 m).
   * The blob drops to the ground plane, shrinks and fades with height. At air 0 this writes
   * exactly the construction values (y 0.01, scale 1, opacity 0.55).
   */
  private syncBlob(): void {
    const air = clamp(this.group.position.y - this.groundY, 0, 3);
    this.blob.position.y = 0.01 - air;
    const sc = 1 / (1 + air * 0.55);
    this.blob.scale.set(sc, sc, 1);
    (this.blob.material as THREE.MeshBasicMaterial).opacity = 0.55 * Math.exp(-air * 1.1);
  }
  /**
   * Grade the cut-out's material colour from an atmosphere tint (see core/env.ts). A multiply
   * tint can only mute toward grey, so sat < 1 mixes every channel toward 0.92 by (1 - sat) * 0.25
   * (documented approximation; sat > 1 leaves the colour alone). Overrides tint() — last call
   * wins. Never called unless the project opts in ({ envMatch: true }) and applies an atmosphere.
   */
  matchEnv(t: EnvTint): this {
    let r = 1, gg = 1, b = 1;
    r *= 1 + t.warm * 0.6; b *= 1 - t.warm * 0.6;
    if (t.sat < 1) { const k = (1 - t.sat) * 0.25; r += (0.92 - r) * k; gg += (0.92 - gg) * k; b += (0.92 - b) * k; }
    const e = clamp(Math.sqrt(Math.max(t.exposure, 0)), 0, 1.15); r *= e; gg *= e; b *= e;
    const c = this._envC.setRGB(r, gg, b);
    if (t.fogMix > 0) c.lerp(this._envFog.set(t.fogColor), clamp(t.fogMix, 0, 1));
    this.mat.color.copy(c);
    if (this.headMesh) (this.headMesh.material as THREE.MeshBasicMaterial).color.copy(c);
    return this;
  }
  /**
   * Attach a REAL 3D object to a drawn hand: the object must be added to the same scene by the
   * shot; from then on face() poses it at the hand every frame — it occludes, is occluded and
   * casts real shadows. It is not canvas-bound, but keep its size sane in the spirit of the
   * 200-unit canvas prop-reach contract: the hand must visually meet it.
   */
  attach(obj: THREE.Object3D, hand: 'L' | 'R', opts: { offset?: [number, number, number]; rot?: [number, number, number] } = {}): this {
    this._attachments = this._attachments.filter((e) => e.obj !== obj);
    const [rx, ry, rz] = opts.rot ?? [0, 0, 0];
    this._attachments.push({ obj, hand, offset: new THREE.Vector3(...(opts.offset ?? [0, 0, 0])), rotQ: new THREE.Quaternion().setFromEuler(new THREE.Euler(rx, ry, rz)) });
    return this;
  }
  /** detach one attached object, or all of them when called with no argument */
  detach(obj?: THREE.Object3D): this {
    this._attachments = obj ? this._attachments.filter((e) => e.obj !== obj) : [];
    return this;
  }
  private syncAttachments(): void {
    if (!this._attachments.length) return;
    if (this.group.visible === false) { for (const e of this._attachments) e.obj.visible = false; return; }
    const s = this.H / 1024;
    this.mesh.updateWorldMatrix(true, false);
    this.mesh.getWorldQuaternion(this._attQ);
    for (const e of this._attachments) {
      const h = this.hands?.[e.hand]; if (!h) continue;
      const w = this._attW.set((STAGE.OX + h[0] - STAGE.CW / 2) * s, (1256 - (STAGE.OY + h[1])) * s, 0);
      this.mesh.localToWorld(w);
      e.obj.position.copy(w).add(this._attOff.copy(e.offset).applyQuaternion(this._attQ));
      e.obj.quaternion.copy(this._attQ).multiply(e.rotQ);
      e.obj.visible = true;
    }
  }
  tint(col: string): this { this.mat.color.set(col); return this; }
  /** change the look (e.g. more stamps) and force a redraw */
  restyle(patch: Partial<ToonLook>): this { Object.assign(this.look, patch); this.key = ''; return this; }
  at(x: number, y: number, z: number): this { this.group.position.set(x, y, z); this.syncBlob(); return this; }
}

const _camPitchTmp = new THREE.Vector3();
/**
 * The DRAWING-side companion of the perspective pitch in face(): the camera pitch relative to
 * a toon, normalised to -1..1 (pitchAng / 0.9, clamped). Shots pass it as st.camPitch so the
 * drawn head compensates for the same angle the plane pitches by. Uses the same geometry as
 * face(): height reference p.y + H * 0.6, horizontal distance with the same 1e-4 guard.
 */
export function camPitchOf(toon: Toon, camPos: { x: number; y: number; z: number }): number {
  const p = _camPitchTmp; toon.group.getWorldPosition(p);
  const dy = camPos.y - (p.y + toon.H * 0.6), dh = Math.hypot(camPos.x - p.x, camPos.z - p.z);
  const pitchAng = dh < 1e-4 ? 0 : Math.atan2(dy, dh);
  return clamp(pitchAng / 0.9, -1, 1);
}

// ---------- skill-library metadata (v5, D4 §5.1) ----------
// One meta row per library item, declared next to the data it describes; Squad 10's
// library/*.md docs are generated from these rows so the docs cannot drift from the code.
// Previews are always engine renders (castsheet / sheets), never generated art (Hard Law 2).
const moodMeta = (name: Mood, display: string, since: string): LibraryItemMeta => ({
  id: `mood.${name}`, kind: 'mood', name: display, snippet: `p.set({ mood: '${name}' })`,
  captionSafe: 'face-only change; nothing enters the caption zone', preview: `library/previews/mood-${name}.jpg`,
  optIn: `mood: '${name}'`, since,
});
export const MOODS_LIBRARY: LibraryItemMeta[] = [
  moodMeta('neutral', 'Neutral', 'pre-5.0'), moodMeta('smug', 'Smug', 'pre-5.0'), moodMeta('happy', 'Happy', 'pre-5.0'),
  moodMeta('laugh', 'Laugh', 'pre-5.0'), moodMeta('shock', 'Shock', 'pre-5.0'), moodMeta('worried', 'Worried', 'pre-5.0'),
  moodMeta('angry', 'Angry', 'pre-5.0'), moodMeta('sad', 'Sad', 'pre-5.0'), moodMeta('sly', 'Sly', 'pre-5.0'),
  moodMeta('proud', 'Proud', 'pre-5.0'), moodMeta('tired', 'Tired', 'pre-5.0'), moodMeta('dizzy', 'Dizzy', 'pre-5.0'),
  moodMeta('determined', 'Determined', '4.0'), moodMeta('embarrassed', 'Embarrassed', '4.0'), moodMeta('awe', 'Awe', '4.0'),
  moodMeta('suspicious', 'Suspicious', '4.0'), moodMeta('relieved', 'Relieved', '4.0'),
  moodMeta('puzzled', 'Puzzled — one brow up, one down; the thinking-it-over face', '5.0'),
  moodMeta('deadpan', 'Deadpan — heavy lids, flat mouth; the micro-beat after a reveal', '5.0'),
];
const hatMeta = (name: HatKind): LibraryItemMeta => ({
  id: `hat.${name}`, kind: 'hat', name, snippet: `registerCast('hero', { hat: '${name}' })`,
  captionSafe: 'identity field (lookSignature) — choose at registerCast time, never swap mid-film',
  preview: `library/previews/hat-${name}.jpg`, optIn: `hat: '${name}'`, since: 'pre-5.0',
});
export const HATS_LIBRARY: LibraryItemMeta[] = HATS.map(hatMeta);
const neckMeta = (name: NeckKind): LibraryItemMeta => ({
  id: `neck.${name}`, kind: 'neck', name, snippet: `registerCast('hero', { neck: '${name}' })`,
  captionSafe: 'identity field (lookSignature) — choose at registerCast time, never swap mid-film',
  preview: `library/previews/neck-${name}.jpg`, optIn: `neck: '${name}'`, since: 'pre-5.0',
});
export const NECKS_LIBRARY: LibraryItemMeta[] = NECKS.map(neckMeta);
/** the extra strings drawFace/drawPattern actually honour (unknown strings are silently ignored) */
export const EXTRAS: readonly string[] = ['mustache', 'glasses', 'freckles', 'blush', 'pencil', 'monocle'];
export const EXTRAS_LIBRARY: LibraryItemMeta[] = EXTRAS.map((name) => ({
  id: `extra.${name}`, kind: 'extra' as const, name, snippet: `registerCast('hero', { extra: ['${name}'] })`,
  captionSafe: 'identity field (lookSignature) — choose at registerCast time, never swap mid-film',
  preview: `library/previews/extra-${name}.jpg`, optIn: `extra: ['${name}']`, since: 'pre-5.0',
}));
