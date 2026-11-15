// FOG CHART — the TaleRender fog verification chart (shaders/fog/FOG.md §7).
// One static shot: flat panels at known axis depths under the S06 scene fog,
// rendered through the real engine composer. The chart spec below is shared
// verbatim with the native scene files (scenes/fog-chart.scene.json and
// scenes/fog-chart-nofog.scene.json) — both sides derive geometry and
// colours from these constants, never by eyeballing. Analytic expectations
// live in tools/fog-chart/expected.json; the native side is gated against
// them, this Chrome capture is reference (b) (PSNR >= 45 dB, centres only —
// its master is JPEG).
import * as THREE from 'three';
import { camera, type Engine } from './core/engine';
import type { Shot, Cue } from './core/shots';

export type { Shot, Cue } from './core/shots';

// ---- shared chart spec ------------------------------------------------------
export const FOV = 40;
export const FOG_COLOR = 0xC9CFC2, FOG_NEAR = 4.5, FOG_FAR = 20; // S06 (shots.ts:409)
// [depthM, rowNdcY] — panels sit at fixed NDC so every depth measures at the
// same screen size (~335 x 288 px): basic at NDC x -0.45, standard at +0.45,
// panel size 0.62 x 0.30 of the frustum half-extents at its depth.
export const ROWS: Array<[number, number]> = [
  [3, 0.62], [6, 0.31], [10, 0], [15, -0.31], [25, -0.62],
];
export const BASIC_NDC_X = -0.45, STD_NDC_X = 0.45;
export const PANEL_NDC_W = 0.62, PANEL_NDC_H = 0.30;
export const BASIC_COL: Record<number, string> = {
  3: '#D94F30', 6: '#3F6FD9', 10: '#7A8B99', 15: '#2FA89B', 25: '#8C8C46',
};
export const STD_COL: Record<number, string> = {
  3: '#30D94F', 6: '#D9B83F', 10: '#8E44AD', 15: '#C94F7C', 25: '#466B8C',
};
export const GROUND_COL = '#2E4A34';

function flatTexture(hex: string): THREE.CanvasTexture {
  const S = 64;
  const cv = document.createElement('canvas'); cv.width = S; cv.height = S;
  const g = cv.getContext('2d')!;
  g.fillStyle = hex; g.fillRect(0, 0, S, S);
  const tex = new THREE.CanvasTexture(cv);
  tex.colorSpace = THREE.SRGBColorSpace;
  // No mipmaps: panels are flat; one less divergence source (as at M3).
  tex.minFilter = THREE.LinearFilter;
  tex.magFilter = THREE.LinearFilter;
  tex.generateMipmaps = false;
  return tex;
}

export function buildShots(E: Engine): { shots: Shot[]; cues: Cue[] } {
  const scene = new THREE.Scene();
  scene.fog = new THREE.Fog(FOG_COLOR, FOG_NEAR, FOG_FAR);
  scene.background = new THREE.Color(FOG_COLOR); // mirrors the native `clear`
  const cam = camera(FOV); // near 0.05, far 800 (engine defaults)
  cam.position.set(0, 0, 0);
  cam.lookAt(0, 0, -1);

  // The native scene-file colour pipeline lights with a fixed house term,
  // col x (0.35 + 0.65 x max(dot(n, L), 0)), L = normalize(0.5, 0.8, 0.6).
  // This rig reproduces that factor on the fronto-parallel standard panels:
  // ambient 0.35 + directional 0.65 from L's direction. Declared residual:
  // three's GGX specular at roughness 1 and its irradiance conventions are
  // not the house term — the analytic column in expected.json is the gate,
  // this capture is the cross-check.
  scene.add(new THREE.AmbientLight(0xffffff, 0.35));
  const dir = new THREE.DirectionalLight(0xffffff, 0.65);
  dir.position.set(10, 16, 12); // 20 x (0.5, 0.8, 0.6); target is the origin
  scene.add(dir);

  const ground = new THREE.Mesh(
    new THREE.BoxGeometry(80, 0.1, 72),
    new THREE.MeshStandardMaterial({ color: GROUND_COL, roughness: 1, metalness: 0 }),
  );
  ground.position.set(0, -8.05, -29);
  scene.add(ground);

  const tanH = Math.tan(THREE.MathUtils.degToRad(FOV / 2));
  for (const [d, ny] of ROWS) {
    const hw = d * tanH * (1080 / 1920), hh = d * tanH;
    const w = PANEL_NDC_W * hw, h = PANEL_NDC_H * hh, cy = ny * hh;
    const basic = new THREE.Mesh(
      new THREE.PlaneGeometry(w, h),
      new THREE.MeshBasicMaterial({ map: flatTexture(BASIC_COL[d]) }),
    );
    basic.position.set(BASIC_NDC_X * hw, cy, -d);
    scene.add(basic);
    // 0.1 m box seated so the FRONT FACE is at exactly d metres, matching
    // the native file (a plane's face would sit at its centre depth too,
    // but the box also gives the native colour-geometry vocabulary).
    const std = new THREE.Mesh(
      new THREE.BoxGeometry(w, h, 0.1),
      new THREE.MeshStandardMaterial({ color: STD_COL[d], roughness: 1, metalness: 0 }),
    );
    std.position.set(STD_NDC_X * hw, cy, -(d + 0.05));
    scene.add(std);
  }

  const draw = (_lt: number, _d: number, _t: number, blit: () => void) => {
    // Real composer chain; aperture 0 disables the bokeh pass (engine.ts
    // enable rule) and the look is neutral, so the comparison isolates fog
    // (OutputPass still runs).
    E.render(scene, cam, {
      focus: 10, aperture: 0, maxblur: 0.006,
      bloom: 0, exposure: 1, vignette: 0, grain: 0, warm: 0, sat: 1,
    });
    blit();
  };
  return { shots: [{ id: 'fog-chart', at: 0, note: 'Fog chart: basic + standard panels at 3/6/10/15/25 m under S06 fog', draw }], cues: [] };
}
