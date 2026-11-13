// DOF CHART — the TaleRender M3-gate chart scene (compatibility contract §1.2).
// One static shot: textured panels at known depths, rendered through the real
// engine composer with the production bokeh defaults. The chart spec below is
// shared verbatim with the native scene (talerender src/dof.rs) — both sides
// derive geometry and texture from these constants, never by eyeballing.
import * as THREE from 'three';
import { camera, type Engine } from './core/engine';
import type { Shot, Cue } from './core/shots';

export type { Shot, Cue } from './core/shots';

// ---- shared chart spec ------------------------------------------------------
export const FOCUS = 10, APERTURE = 0.0015, MAXBLUR = 0.006;
// [ndcX, ndcY, depthM] — NDC y up. The last panel (2 m) overlaps the bottom of
// the focal panel (10 m) on purpose: the foreground-bleed test case.
export const PANELS: Array<[number, number, number]> = [
  [-0.62, 0.66, 30], [0, 0.66, 14], [0.62, 0.66, 11],
  [-0.62, 0, 12], [0, 0, 10], [0.62, 0, 8],
  [-0.62, -0.66, 7], [0, -0.66, 5], [0.62, -0.66, 3.5],
  [0, -0.30, 2],
];
export const PANEL_NDC_W = 0.56, PANEL_NDC_H = 0.36, BLEED_NDC_H = 0.30;
export const TEX_SIZE = 512;

// Chart pattern as a function of (u, w): u rightward, w DOWNWARD from the
// panel's top edge. Zones: white frame / checkerboard (u<0.5, cells of 0.1) /
// mid-grey separator (0.5..0.58) / one big step edge at u=0.79.
export function pattern(u: number, w: number): [number, number, number] {
  if (u < 0.015 || u > 0.985 || w < 0.015 || w > 0.985) return [255, 255, 255];
  if (u < 0.5) {
    const p = (Math.floor(u * 10) + Math.floor(w * 10)) % 2;
    return p === 0 ? [238, 238, 238] : [20, 20, 20];
  }
  if (u < 0.58) return [128, 128, 128];
  return u < 0.79 ? [20, 20, 20] : [238, 238, 238];
}

function chartTexture(): THREE.CanvasTexture {
  const S = TEX_SIZE;
  const cv = document.createElement('canvas'); cv.width = S; cv.height = S;
  const g = cv.getContext('2d')!;
  const img = g.createImageData(S, S);
  for (let y = 0; y < S; y++) for (let x = 0; x < S; x++) {
    const [r, gg, b] = pattern((x + 0.5) / S, (y + 0.5) / S);
    const k = (y * S + x) * 4;
    img.data[k] = r; img.data[k + 1] = gg; img.data[k + 2] = b; img.data[k + 3] = 255;
  }
  g.putImageData(img, 0, 0);
  const tex = new THREE.CanvasTexture(cv);
  tex.colorSpace = THREE.SRGBColorSpace;
  // No mipmaps: the chart is measured at ~1:1 texel:pixel, and the native
  // sampler is a single-level linear sampler — one less divergence source.
  tex.minFilter = THREE.LinearFilter;
  tex.magFilter = THREE.LinearFilter;
  tex.generateMipmaps = false;
  return tex;
}

export function buildShots(E: Engine): { shots: Shot[]; cues: Cue[] } {
  const scene = new THREE.Scene();
  const cam = camera(30); // fov 30, near 0.05, far 800 (engine defaults)
  cam.position.set(0, 0, 0);
  cam.lookAt(0, 0, -1);
  const tex = chartTexture();
  const tanH = Math.tan(THREE.MathUtils.degToRad(15));
  for (const [nx, ny, d] of PANELS) {
    const hh = d * tanH, hw = hh * (1080 / 1920);
    const hNdc = d === 2 ? BLEED_NDC_H : PANEL_NDC_H;
    const geo = new THREE.PlaneGeometry(PANEL_NDC_W * hw, hNdc * hh);
    const m = new THREE.Mesh(geo, new THREE.MeshBasicMaterial({ map: tex }));
    m.position.set(nx * hw, ny * hh, -d);
    scene.add(m);
  }
  const draw = (_lt: number, _d: number, _t: number, blit: () => void) => {
    // Real composer chain; bloom/grade neutralised through the look object so
    // the comparison isolates the bokeh pass (OutputPass still runs).
    E.render(scene, cam, {
      focus: FOCUS, aperture: APERTURE, maxblur: MAXBLUR,
      bloom: 0, exposure: 1, vignette: 0, grain: 0, warm: 0, sat: 1,
    });
    blit();
  };
  return { shots: [{ id: 'dof-chart', at: 0, note: 'DoF chart: panels at known depths, focus 10 m', draw }], cues: [] };
}
