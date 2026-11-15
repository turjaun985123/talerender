// Boot: fonts → script + timing → 3D engine + sets → captions → shots. draw(t) renders film time t (pure function of t):
// the shot renders its 3D frame into the hidden WebGL canvas, which is copied onto the 2D canvas, then the 2D layer
// (labels, counters) and the captions go on top. window.OP is the API tools/capture.mjs drives.
import { W, H, setCtx, ctx, clamp } from './core/lib';
import { T, loadTiming } from './core/timing';
import { buildCaptions, drawCaption, captionList } from './core/captions';
import { makeEngine, RES, AA } from './core/engine';
import { TOONS, OVERFLOW_CANVASES, scanCanvasOverflow, registerOverflowCanvas } from './core/toon';
import * as THREE from 'three';
import { buildShots, type Shot, type Cue } from './shots';
import { CAST, lookSignature } from './core/cast';

export interface CutoutBox { x0: number; y0: number; x1: number; y1: number; behind?: boolean }
export interface CaptureOP {
  ready: boolean; _shots?: Shot[]; _cam?: THREE.Camera; _scene?: THREE.Scene; script?: unknown;
  duration?: () => number; draw?: (t: number) => void; frame?: (t: number, q?: number) => string;
  shots?: () => Array<{ id: string; at: number; end: number; note: string }>;
  captions?: () => Array<{ s: number; e: number; text: string; line: number; auto: boolean; rawW: number; fit: number; renderedW: number }>;
  cues?: () => Cue[]; cutouts?: () => Array<{ id: string; desc: string; pose: string; overflow: unknown; box: CutoutBox | null; sig: string }>;
  cast?: () => string[];
  bootError?: string;
}
const cv = document.getElementById('cv') as HTMLCanvasElement, GL = document.getElementById('gl') as HTMLCanvasElement;
setCtx(cv.getContext('2d')!);
const OP: CaptureOP = ((window as unknown as { OP: CaptureOP }).OP = { ready: false });

function draw(t: number) {
  t = clamp(t, 0, T.dur - 1e-4);
  ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.globalAlpha = 1; ctx.letterSpacing = '0px';
  const S = OP._shots!; let i = S.length - 1; while (i > 0 && S[i].at > t) i--;
  const sh = S[i], end = i < S.length - 1 ? S[i + 1].at : T.dur, d = end - sh.at, lt = t - sh.at;
  const blit = () => ctx.drawImage(GL, 0, 0, W, H);
  ctx.save(); sh.draw(lt, d, t, blit); ctx.restore();
  ctx.globalAlpha = 1; ctx.letterSpacing = '0px';
  const fade = Math.min(clamp(t / 0.25), clamp((T.dur - t) / 0.4));
  if (fade < 1) { ctx.fillStyle = '#000'; ctx.globalAlpha = 1 - fade; ctx.fillRect(0, 0, W, H); ctx.globalAlpha = 1; }
  drawCaption(t);
}

(async () => {
  try {
    await document.fonts.load('900 100px Inter'); await document.fonts.load('800 100px Inter');
    const script = await (await fetch('script.json', { cache: 'no-store' })).json();
    await loadTiming();
    buildCaptions(script);
    GL.width = Math.round(1080 * RES); GL.height = Math.round(1920 * RES);
    const E = makeEngine(GL, { w: GL.width, h: GL.height, samples: AA });
    const { shots, cues } = buildShots(E);
    // WP2: remember the last camera passed to E.render so OP.cutouts() can project
    // toon planes into screen space. buildShots captured E, but its shot draw
    // functions look E.render up on the object at call time, so reassigning the
    // property here (before OP.ready) intercepts every later draw.
    const _render = E.render.bind(E);
    E.render = (scene, cam, look) => { OP._cam = cam; OP._scene = scene; return _render(scene, cam, look); };
    shots.sort((a, b) => a.at - b.at);
    OP._shots = shots; OP.script = script;
    OP.duration = () => T.dur;
    OP.draw = draw;
    OP.frame = (t, q = 0.92) => { draw(t); return cv.toDataURL('image/jpeg', q); };
    OP.shots = () => shots.map((s, i) => ({ id: s.id, at: +s.at.toFixed(3), end: +(i < shots.length - 1 ? shots[i + 1].at : T.dur).toFixed(3), note: s.note || '' }));
    OP.captions = () => captionList().map((c) => ({ s: +c.s.toFixed(3), e: +c.e.toFixed(3), text: c.words.map((w) => (w.hi ? '*' : '') + w.txt).join(' '), line: c.line, auto: c.auto, rawW: c.rawW, fit: c.fit, renderedW: c.renderedW }));
    OP.cues = () => cues.slice().sort((a, b) => a.t - b.t);
    // WP2: per-toon audit data for tools/capture.mjs — canvas overflow (painted
    // pixels in the texture border strips, from Seat A's toon.js contract) plus
    // the toon plane's screen-space box, projected with the last render camera.
    // The plane is wider/taller than the figure (safety margin), so `box` is the
    // PLANE bounds. That is not what the edge-touch warning measures: capture
    // converts `box` into the drawn figure's box (figBox in capture.ts — fixed
    // stage fractions, x 384..896 / y 256..1280 of the canvas) and tests THAT
    // against the frame, so a centred figure whose plane overhangs the frame
    // no longer warns spuriously (v5, Phase 3). Canvas overflow keeps the
    // plane/canvas pixel semantics exactly.
    OP.cutouts = () => {
      const items = TOONS.map((t) => {
      let box: CutoutBox | null = null;
      // Edge-touch only makes sense for toons in the scene that was just
      // rendered (every shot set builds its own cast; other sets' toons would
      // project as garbage against this camera). Overflow is scene-independent
      // and is reported for every toon regardless.
      let inScene = !OP._scene;
      if (!inScene) { let n: THREE.Object3D | null = t.group; while (n) { if (n === OP._scene) { inScene = true; break; } n = n.parent; } }
      if (inScene && t.group.visible !== false && OP._cam) {
        const cam = OP._cam;
        cam.updateMatrixWorld();
        t.mesh.updateWorldMatrix(true, false);
        const geo = t.mesh.geometry;
        if (!geo.boundingBox) geo.computeBoundingBox();
        const bb = geo.boundingBox!;
        const corners = [
          [bb.min.x, bb.min.y], [bb.min.x, bb.max.y],
          [bb.max.x, bb.min.y], [bb.max.x, bb.max.y],
        ];
        let behind = false;
        const xs: number[] = [], ys: number[] = [];
        for (const [cx, cy] of corners) {
          const v = new THREE.Vector3(cx, cy, bb.min.z).applyMatrix4(t.mesh.matrixWorld);
          // Behind the camera: in view space the camera looks down -Z, so a
          // corner with view-space z > 0 sits behind the camera plane.
          const viewZ = v.clone().applyMatrix4(cam.matrixWorldInverse).z;
          if (viewZ > 0) behind = true;
          v.project(cam);
          if (!Number.isFinite(v.x) || !Number.isFinite(v.y) || !Number.isFinite(v.z)) { behind = true; v.x = Number.isFinite(v.x) ? v.x : 0; v.y = Number.isFinite(v.y) ? v.y : 0; v.z = Number.isFinite(v.z) ? v.z : 2; }
          if (v.z > 1) behind = true;
          // Screen px (frame is 1080x1920). Clamp only to a wide sane range so a
          // behind/far projection stays finite — NOT to the frame, or edge-touch
          // detection (box outside 0..1080 / 0..1920) could never fire.
          const clampPx = (n: number) => Math.max(-10000, Math.min(10000, n));
          xs.push(clampPx((v.x + 1) / 2 * 1080));
          ys.push(clampPx((1 - v.y) / 2 * 1920));
        }
        box = {
          x0: Math.round(Math.min(...xs)), y0: Math.round(Math.min(...ys)),
          x1: Math.round(Math.max(...xs)), y1: Math.round(Math.max(...ys)),
        };
        if (behind) box.behind = true;
      }
      return { id: t.name, desc: t.describe(), pose: t.poseSummary(), overflow: t.overflow(), box, sig: lookSignature(t.look) };
      });
      // Project-local canvas characters registered via registerOverflowCanvas
      // (core/toon): overflow-audited exactly like toons, but they own no
      // engine plane, so there is no screen box to project (box stays null —
      // no edge-touch line for them). Toon canvases are skipped here; they
      // are audited above, with their boxes.
      const toonCanvases = new Set(TOONS.map((t) => t.canvas));
      for (const e of OVERFLOW_CANVASES) {
        if (toonCanvases.has(e.canvas)) continue;
        items.push({ id: e.label, desc: 'registered canvas', pose: e.pose ? e.pose() : '', overflow: scanCanvasOverflow(e.canvas, e.res), box: null, sig: '' });
      }
      return items;
    };
    OP.cast = () => [...CAST.keys()];
    // Project code that never imports the engine modules can still register a
    // canvas-drawn character for the overflow gate.
    (window as unknown as { __registerOverflowCanvas?: typeof registerOverflowCanvas }).__registerOverflowCanvas = registerOverflowCanvas;
    OP.ready = true;
    const q = new URLSearchParams(location.search);
    if (!q.has('capture')) {
      if (q.has('t')) draw(+q.get('t')!);
      else {
        const audio = new Audio('data/mix.wav'); let t0: number | null = null; const btn = document.getElementById('play') as HTMLButtonElement; btn.hidden = false;
        btn.onclick = () => { btn.hidden = true; t0 = performance.now(); audio.currentTime = 0; audio.play().catch(() => {}); };
        const loop = (now: number) => { const t = t0 == null ? 0 : (now - t0) / 1000; draw(t); if (t0 != null && t > T.dur) { t0 = null; btn.hidden = false; } requestAnimationFrame(loop); };
        requestAnimationFrame(loop);
      }
    }
  } catch (e) { OP.bootError = String(e instanceof Error ? e.stack : e); console.error(e); }
})();
