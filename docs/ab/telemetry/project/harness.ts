// M6 telemetry harness — runs inside a scratch copy of the Onoda
// project and imports the project's own core modules throughout.
// Two jobs:
//
// 1. CAPTION METRICS for the film's chunk list, in the v5
//    OP.captions() field shape (line, s, e, text, auto, rawW, fit,
//    renderedW). The v4 project's captionList() exposes only
//    {line, s, e, text}; the metrics are produced here with the
//    caption engine's own measurer — Canvas2D measureText under
//    captions.ts's FONT ('900 88px Inter', letterSpacing '0px'),
//    fit = min(1, CAP.maxW / w), renderedW = w * fit, rounded as
//    the v5 captionMetrics() rounds them (rawW/renderedW to 1 dp,
//    fit to 3 dp). `auto` is decided exactly as buildCaptions
//    decides it: a chunk is auto iff its script line has no
//    authored "caps" (every line of this film authors its caps).
//
// 2. THE OVERFLOW VARIANT: the real Onoda toon — registerCast
//    look (shots.ts LOOK.onoda), person()'s construction
//    (perspective + envMatch, shade '#878E6E') — staged at the
//    shared variant numbers (group (1.9, -0.55, 0.6), lean 42,
//    S01's t=1.50 camera), set + faced + rendered through the real
//    engine with main.ts's E.render interception, then audited by
//    OP.cutouts copied verbatim from src/main.ts (only change: the
//    OP holder is local). The native telemetry mode
//    `ab1-variant` stages the identical numbers.
//
// Outputs are POSTed to the harness server: captions.json,
// variant-cutouts.json, provenance.json (via /done).
import { loadTiming } from './core/timing';
import { buildCaptions, captionList, CAP } from './core/captions';
import { makeEngine, RES, AA, camera } from './core/engine';
import { Toon, TOONS, OVERFLOW_CANVASES, scanCanvasOverflow, camPitchOf } from './core/toon';
import { registerCast, castLook, lookSignature } from './core/cast';
import * as THREE from 'three';

const post = async (name: string, body: string) => {
  await fetch('/save/' + name, { method: 'POST', body });
};

(async () => {
  const errors: string[] = [];
  try {
    await document.fonts.load('900 100px Inter');
    await document.fonts.load('800 100px Inter');
    const script = await (await fetch('script.json', { cache: 'no-store' })).json();
    await loadTiming();
    buildCaptions(script);

    // ---- 1. caption metrics (the caption engine's own measurer) --
    const mc = document.createElement('canvas').getContext('2d')!;
    mc.font = `900 ${CAP.size}px Inter`;
    (mc as unknown as { letterSpacing: string }).letterSpacing = '0px';
    const metrics = captionList().map((c) => {
      const w = mc.measureText(c.text).width;
      const fit = Math.min(1, CAP.maxW / w);
      return {
        line: c.line, s: c.s, e: c.e, text: c.text,
        auto: !script.lines[c.line].caps,
        rawW: +w.toFixed(1), fit: +fit.toFixed(3), renderedW: +(w * fit).toFixed(1),
      };
    });
    await post('captions.json', JSON.stringify(metrics, null, 1));


    // OP.cutouts — copied verbatim from src/main.ts (WP2 block);
    // `OP._cam` / `OP._scene` are the local holder above.
    interface CutoutBox { x0: number; y0: number; x1: number; y1: number; behind?: boolean }
    const cutouts = () => {
      const items = TOONS.map((t) => {
        let box: CutoutBox | null = null;
        let inScene = !OP._scene;
        if (!inScene) { let n: THREE.Object3D | null = t.group; while (n) { if (n === OP._scene) { inScene = true; break; } n = n.parent; } }
        if (inScene && t.group.visible !== false && OP._cam) {
          const cam2 = OP._cam;
          cam2.updateMatrixWorld();
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
            const viewZ = v.clone().applyMatrix4(cam2.matrixWorldInverse).z;
            if (viewZ > 0) behind = true;
            v.project(cam2);
            if (!Number.isFinite(v.x) || !Number.isFinite(v.y) || !Number.isFinite(v.z)) { behind = true; v.x = Number.isFinite(v.x) ? v.x : 0; v.y = Number.isFinite(v.y) ? v.y : 0; v.z = Number.isFinite(v.z) ? v.z : 2; }
            if (v.z > 1) behind = true;
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
      const toonCanvases = new Set(TOONS.map((t) => t.canvas));
      for (const e of OVERFLOW_CANVASES) {
        if (toonCanvases.has(e.canvas)) continue;
        items.push({ id: e.label, desc: 'registered canvas', pose: e.pose ? e.pose() : '', overflow: scanCanvasOverflow(e.canvas, e.res), box: null, sig: '' });
      }
      return items;
    };

        const OP: { _cam?: THREE.Camera; _scene?: THREE.Scene } = {};
    registerCast('onoda', { body: '#5C6644', hat: 'cap', hatCol: '#4C5638' });

    // ---- 1b. base staging (S01 @ t=1.50, shared constants) ------
    // The same numbers the native telemetry stages for `ab1`: the
    // registerCast look, person()'s construction, the pose set()
    // from shots.ts (headTurn the exact f64 the poseTrack produces
    // at t=1.50), the §28 camera constants. If this box differs
    // from the full page's OP.cutouts() box, the difference lives
    // in the shot path (shoot()); if it differs from the native
    // box, it lives in the face()/mesh transform. Debug values are
    // POSTed alongside.
    {
      const GL0 = document.getElementById('gl') as HTMLCanvasElement;
      GL0.width = Math.round(1080 * RES);
      GL0.height = Math.round(1920 * RES);
      const E0 = makeEngine(GL0, { w: GL0.width, h: GL0.height, samples: AA });
      const keep: { _cam?: THREE.Camera; _scene?: THREE.Scene } = {};
      const _r0 = E0.render.bind(E0);
      E0.render = (s: THREE.Scene, c: THREE.Camera, l?: unknown) => { keep._cam = c; keep._scene = s; return _r0(s, c, l as never); };
      const scene0 = new THREE.Scene();
      const look0 = castLook('onoda');
      look0.shade = '#878E6E';
      const t0 = new Toon(look0, { envMatch: true, perspective: true });
      scene0.add(t0.group);
      const EYE0 = new THREE.Vector3(0.83432, 2.64354, 5.28499);
      const TGT0 = new THREE.Vector3(0.35, 1.11477, -0.55);
      t0.at(0.35, 0, -0.55);
      t0.set({ mood: 'determined', armR: [16, -6], armL: [10, 6], lean: 1, headTurn: -0.024999999999999967, camPitch: camPitchOf(t0, EYE0) });
      const cam0 = camera(44);
      cam0.position.copy(EYE0);
      cam0.lookAt(TGT0);
      t0.face(cam0);
      E0.render(scene0, cam0, { fov: 44 });
      const dbg = {
        camPos: cam0.position.toArray(), camQuat: cam0.quaternion.toArray(),
        groupPos: t0.group.position.toArray(),
        meshPos: t0.mesh.position.toArray(),
        meshRot: [t0.mesh.rotation.x, t0.mesh.rotation.y, t0.mesh.rotation.z, t0.mesh.rotation.order],
        groupRot: [t0.group.rotation.x, t0.group.rotation.y, t0.group.rotation.z, t0.group.rotation.order],
        matrixWorld: t0.mesh.matrixWorld.elements,
      };
      await post('base-debug.json', JSON.stringify(dbg, null, 1));
      const savedCam = OP._cam, savedScene = OP._scene;
      OP._cam = keep._cam; OP._scene = keep._scene;
      await post('base-cutouts.json', JSON.stringify(cutouts(), null, 1));
      OP._cam = savedCam; OP._scene = savedScene;
    }

    // ---- 2. the overflow variant ---------------------------------
    const GL = document.getElementById('gl') as HTMLCanvasElement;
    GL.width = Math.round(1080 * RES);
    GL.height = Math.round(1920 * RES);
    const E = makeEngine(GL, { w: GL.width, h: GL.height, samples: AA });
    const _render = E.render.bind(E);
    E.render = (scene: THREE.Scene, cam: THREE.Camera, look?: unknown) => {
      OP._cam = cam; OP._scene = scene;
      return _render(scene, cam, look as never);
    };
    const scene = new THREE.Scene();
    const look = castLook('onoda');
    look.shade = '#878E6E';
    const toon = new Toon(look, { envMatch: true, perspective: true });
    scene.add(toon.group);
    // Shared variant staging — mirrors src/telemetry.rs
    // (VARIANT_POS, VARIANT_LEAN_DEG) and ab1's camera constants.
    const EYE = new THREE.Vector3(0.83432, 2.64354, 5.28499);
    const TGT = new THREE.Vector3(0.35, 1.11477, -0.55);
    toon.at(1.9, -0.55, 0.6);
    toon.set({ mood: 'determined', armR: [16, -6], armL: [10, 6], lean: 42, headTurn: -0.025, camPitch: camPitchOf(toon, EYE) });
    const cam = camera(44);
    cam.position.copy(EYE);
    cam.lookAt(TGT);
    toon.face(cam);
    E.render(scene, cam, { fov: 44, focus: EYE.distanceTo(TGT), bloom: 0.16, aperture: 0.0012, maxblur: 0.007, sat: 0.8, warm: 0.04, vignette: 0.24, grain: 0.012 });

    await post('variant-cutouts.json', JSON.stringify(cutouts(), null, 1));
  } catch (e) {
    errors.push(String(e instanceof Error ? e.stack : e));
  }
  const prov = {
    userAgent: navigator.userAgent,
    webglRenderer: (() => { try { const c = document.createElement('canvas'); const gl = (c.getContext('webgl2') || c.getContext('webgl')) as WebGLRenderingContext | null; if (!gl) return 'unknown'; const ext = gl.getExtension('WEBGL_debug_renderer_info'); return String(ext ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER)); } catch { return 'unknown'; } })(),
    errors,
  };
  await fetch('/done', { method: 'POST', body: JSON.stringify(prov, null, 1) });
})();
