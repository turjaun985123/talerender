// Environment matching + atmosphere presets (WP4): one call re-dresses a whole set — sky, clouds,
// fog, light rig and the 2D characters in it — so a Toon standing in the scene visibly belongs there.
//   applyAtmosphere(scene, 'dusk', { rig: sun })       re-dress a set built by world.ts
//   const preset = applyAtmosphere(scene, 'night', { rig });  E.render(scene, cam, { sat: preset.grade.sat, warm: preset.grade.warm, exposure: preset.grade.exposure })
//   applyAtmosphere(scene, 'noon', { rig, sky: false })       the A/B-safe adoption call: noon is
//                                                             neutral and sky:false leaves a set's
//                                                             own sky/clouds untouched.
// A preset bundles the sky-dome colours, the cloud tint, the scene fog, the key / rim / hemisphere
// values for a LightRig, a render `grade` and a character `tint` (EnvTint, see toon.ts).
//
// HONESTY NOTES — what this can and cannot do:
//   · Fog DOES reach the Toon cut-outs: a Toon's plane uses MeshBasicMaterial, and MeshBasicMaterial
//     honours scene.fog like every standard material in the set, so people and world fade with
//     distance consistently. (The sky dome and the cloud sprites opt out of fog in world.ts —
//     they are re-coloured directly here instead.)
//   · `tint` / matchEnv is a MULTIPLY approximation: it can mute and colour a character toward the
//     scene (dim blue at night, grey in a storm), but a multiply cannot truly desaturate a painted
//     texture — a saturated red coat stays recognisably red, just darker / cooler. True
//     desaturation would need a shader or a repaint, which this pass deliberately is not.
//   · `grade` is ADVISORY: applyAtmosphere never touches the renderer. Shots feed preset.grade
//     into their own E.render(..., look) call, so a shot can override any part of it.
import * as THREE from 'three';
import type { EnvTint } from './toon';

export type AtmosphereName = 'dawn' | 'noon' | 'golden' | 'dusk' | 'night' | 'storm' | 'fog' | 'snow' | 'underwater' | 'dust';

export interface AtmospherePreset {
  name: AtmosphereName;
  skyTop: string;
  skyHor: string;
  skyGnd: string;
  cloudTint: string;
  fog: { color: string; near: number; far: number } | null;
  keyCol: string;
  keyI: number;
  keyPos?: [number, number, number];
  rimI: number;
  hemiSky: string;
  hemiGround: string;
  hemiI: number;
  grade: { sat: number; warm: number; exposure: number };
  tint: EnvTint;
}

/**
 * Structural view of a set's light rig — the world sets return a full LightRig
 * ({ key, rim, group }), which satisfies this, and the interior set returns a
 * { key }-only object, which also satisfies it. Every field is optional and
 * every use is guarded, so a partial rig can never crash applyAtmosphere.
 */
export interface AtmosphereRig {
  key?: THREE.DirectionalLight;
  rim?: THREE.DirectionalLight;
  group?: THREE.Group;
  /** interior point-light fills (bulbs, room fill) — applyAtmosphere step 3b scales/lerps them toward the preset, so 'night' indoors dims the bulbs too. Absent on every v4 rig: step 3b is then a no-op and pixels are unchanged. */
  fills?: THREE.PointLight[];
}

// Design rule for every preset below: fog colour === horizon colour (noon has
// no fog at all), so distant geometry, the horizon band and a fogged character
// all converge on the same colour. Character tint.fogColor follows the same
// colour, scaled by tint.fogMix.
export const ATMOSPHERES: Record<AtmosphereName, AtmospherePreset> = {
  // Low warm-pink key raking in from the side; pale haze on the horizon.
  dawn: {
    name: 'dawn',
    skyTop: '#4A5AA8', skyHor: '#F9C0A8', skyGnd: '#F9C0A8', cloudTint: '#FFD9C8',
    fog: { color: '#F9C0A8', near: 24, far: 115 },
    keyCol: '#FFB49A', keyI: 1.9, keyPos: [-9, 3.5, 4], rimI: 0.55,
    hemiSky: '#C9B8E8', hemiGround: '#8A5A48', hemiI: 0.85,
    grade: { sat: 1.05, warm: 0.06, exposure: 0.98 },
    tint: { sat: 0.96, warm: 0.05, exposure: 0.97, fogColor: '#F9C0A8', fogMix: 0.08 },
  },
  // NEUTRAL — fixed by the wave board, do not improvise: these are exactly the
  // values the sets / engine already ship (world.ts sky + sunlight defaults,
  // engine grade defaults), so adopting applyAtmosphere(..., 'noon') changes
  // no pixel. tint is the identity tint (fogMix 0).
  noon: {
    name: 'noon',
    skyTop: '#3D8EE0', skyHor: '#BFE4F7', skyGnd: '#BFE4F7', cloudTint: '#FFFFFF',
    fog: null,
    keyCol: '#FFF0D6', keyI: 2.6, rimI: 0.4,
    hemiSky: '#CFE6FF', hemiGround: '#C48A5A', hemiI: 1.25,
    grade: { sat: 1.12, warm: 0.02, exposure: 1 },
    tint: { sat: 1, warm: 0, exposure: 1, fogColor: '#FFFFFF', fogMix: 0 },
  },
  // Rich amber key, low from the side; warm haze far off.
  golden: {
    name: 'golden',
    skyTop: '#4A90D9', skyHor: '#FFD98A', skyGnd: '#FFD98A', cloudTint: '#FFE3B0',
    fog: { color: '#FFD98A', near: 30, far: 140 },
    keyCol: '#FFB35C', keyI: 2.3, keyPos: [-8, 4.5, 5], rimI: 0.6,
    hemiSky: '#FFE0B0', hemiGround: '#B07A48', hemiI: 1.0,
    grade: { sat: 1.15, warm: 0.08, exposure: 1.02 },
    tint: { sat: 1.02, warm: 0.08, exposure: 1, fogColor: '#FFD98A', fogMix: 0.06 },
  },
  // Purple-orange evening — the values the sets already use for their dusk
  // variants (world.ts mainStreet / meadow: sky '#3B3F8F' / '#F59C6B', key
  // '#FFB27A' at [-8, 5, -14], hemi '#B9A2D8' / '#7A4A3A', clouds '#FFC9B0').
  dusk: {
    name: 'dusk',
    skyTop: '#3B3F8F', skyHor: '#F59C6B', skyGnd: '#F59C6B', cloudTint: '#FFC9B0',
    fog: { color: '#F59C6B', near: 28, far: 120 },
    keyCol: '#FFB27A', keyI: 2.0, keyPos: [-8, 5, -14], rimI: 0.5,
    hemiSky: '#B9A2D8', hemiGround: '#7A4A3A', hemiI: 0.9,
    grade: { sat: 1.08, warm: 0.06, exposure: 0.97 },
    tint: { sat: 0.97, warm: 0.06, exposure: 0.96, fogColor: '#F59C6B', fogMix: 0.08 },
  },
  // Dim blue moonlight: weak blue key, very low bounce, cool desaturated grade,
  // characters pulled a quarter of the way toward the blue-grey fog colour.
  night: {
    name: 'night',
    skyTop: '#0E1838', skyHor: '#3A4F6B', skyGnd: '#232F45', cloudTint: '#6A7F9E',
    fog: { color: '#3A4F6B', near: 18, far: 90 },
    keyCol: '#9AB8FF', keyI: 0.55, keyPos: [-5, 10, 4], rimI: 0.35,
    hemiSky: '#4A5F8A', hemiGround: '#1E2430', hemiI: 0.25,
    grade: { sat: 0.8, warm: -0.02, exposure: 0.85 },
    tint: { sat: 0.8, warm: -0.02, exposure: 0.8, fogColor: '#3A4F6B', fogMix: 0.25 },
  },
  // Grey flat light: low-contrast key, hemisphere doing most of the work,
  // near grey fog eating the distance.
  storm: {
    name: 'storm',
    skyTop: '#5E6A72', skyHor: '#9AA3A8', skyGnd: '#7A848A', cloudTint: '#8A959C',
    fog: { color: '#9AA3A8', near: 12, far: 65 },
    keyCol: '#C8D2D8', keyI: 1.1, rimI: 0.2,
    hemiSky: '#B8C2C8', hemiGround: '#5E6A62', hemiI: 1.1,
    grade: { sat: 0.85, warm: -0.01, exposure: 0.94 },
    tint: { sat: 0.85, warm: -0.01, exposure: 0.93, fogColor: '#9AA3A8', fogMix: 0.18 },
  },
  // White-grey everything, fog starting almost at the camera: the world
  // dissolves within a few dozen metres and characters fade hardest into it.
  fog: {
    name: 'fog',
    skyTop: '#C8CDD0', skyHor: '#E6E9EA', skyGnd: '#D8DCDE', cloudTint: '#EDEFEF',
    fog: { color: '#E6E9EA', near: 3, far: 38 },
    keyCol: '#EAF0F2', keyI: 0.7, rimI: 0.15,
    hemiSky: '#E6E9EA', hemiGround: '#B8BCBE', hemiI: 0.9,
    grade: { sat: 0.82, warm: 0, exposure: 0.96 },
    tint: { sat: 0.82, warm: 0, exposure: 0.94, fogColor: '#E6E9EA', fogMix: 0.35 },
  },
  // Snow-glare (v5): bright white-blue, high bounce, long pale fog. Pairs with snowfield.
  snow: {
    name: 'snow',
    skyTop: '#7FA8D8', skyHor: '#EAF2FA', skyGnd: '#EAF2FA', cloudTint: '#FFFFFF',
    fog: { color: '#EAF2FA', near: 20, far: 95 },
    keyCol: '#FFF4E0', keyI: 2.4, keyPos: [5, 8, 4], rimI: 0.5,
    hemiSky: '#DCE9F7', hemiGround: '#B9C4CE', hemiI: 1.35,
    grade: { sat: 0.92, warm: 0, exposure: 1.05 },
    tint: { sat: 0.94, warm: 0, exposure: 1.03, fogColor: '#EAF2FA', fogMix: 0.12 },
  },
  // Underwater-lite (v5): fog + tint + top light only — no caustics, no refraction,
  // no bubble system (bubbles are a cue/overlay concern, not env). Beach cutaways.
  underwater: {
    name: 'underwater',
    skyTop: '#0B3B5C', skyHor: '#2E7FA8', skyGnd: '#0E2E44', cloudTint: '#7FB8D4',
    fog: { color: '#2E7FA8', near: 4, far: 42 },
    keyCol: '#9ADBE8', keyI: 1.6, keyPos: [0, 10, 2], rimI: 0.3,
    hemiSky: '#3E7CA6', hemiGround: '#12303F', hemiI: 0.8,
    grade: { sat: 0.95, warm: -0.06, exposure: 0.95 },
    tint: { sat: 0.92, warm: -0.05, exposure: 0.94, fogColor: '#2E7FA8', fogMix: 0.30 },
  },
  // Dust (v5): desert haze / dust-storm-lite — a warm tan wall of air. Desert, fieldCamp.
  dust: {
    name: 'dust',
    skyTop: '#B98A5A', skyHor: '#E8C79A', skyGnd: '#E8C79A', cloudTint: '#EFD3AC',
    fog: { color: '#E8C79A', near: 10, far: 60 },
    keyCol: '#FFD9A3', keyI: 1.8, keyPos: [-7, 6, 3], rimI: 0.25,
    hemiSky: '#E8C79A', hemiGround: '#8A6A4A', hemiI: 1.05,
    grade: { sat: 0.9, warm: 0.07, exposure: 0.97 },
    tint: { sat: 0.9, warm: 0.06, exposure: 0.96, fogColor: '#E8C79A', fogMix: 0.20 },
  },
};

function setUniformColour(uniform: THREE.IUniform, hex: string): void {
  if (uniform.value instanceof THREE.Color) uniform.value.set(hex);
  else uniform.value = new THREE.Color(hex);
}

/** Re-dress a sky-dome material (ShaderMaterial with top / hor / gnd uniforms); any other material is left alone. */
function dressSkyMaterial(mat: THREE.Material, preset: AtmospherePreset): void {
  if (!(mat instanceof THREE.ShaderMaterial)) return;
  const u = mat.uniforms;
  if (!u.top || !u.hor || !u.gnd) return;
  setUniformColour(u.top, preset.skyTop);
  setUniformColour(u.hor, preset.skyHor);
  setUniformColour(u.gnd, preset.skyGnd);
}

/**
 * Apply an atmosphere preset to a scene: scene fog, sky dome + cloud sprites
 * (unless opts.sky === false), the light rig (if opts.rig is given) and the
 * env-match pass over every Toon in the scene that opted in. Accepts a preset
 * name or a preset object (for a shot's hand-tweaked variant). Returns the
 * preset — shots feed `preset.grade` into their E.render look themselves.
 */
export function applyAtmosphere(scene: THREE.Scene, name: AtmosphereName | AtmospherePreset, opts: { rig?: AtmosphereRig; sky?: boolean } = {}): AtmospherePreset {
  const preset: AtmospherePreset = typeof name === 'string' ? ATMOSPHERES[name] : name;

  // 1. Fog — the one change that reaches characters and world alike (see header).
  scene.fog = preset.fog ? new THREE.Fog(preset.fog.color, preset.fog.near, preset.fog.far) : null;

  // 2. Sky dome + clouds (clouds are the only sprites in these sets).
  if (opts.sky !== false) {
    scene.traverse((obj) => {
      if ((obj as THREE.Sprite).isSprite) {
        (obj as THREE.Sprite).material.color.set(preset.cloudTint);
      }
      const holder = obj as { material?: THREE.Material | THREE.Material[] };
      if (Array.isArray(holder.material)) holder.material.forEach((m) => dressSkyMaterial(m, preset));
      else if (holder.material) dressSkyMaterial(holder.material, preset);
    });
  }

  // 3. Light rig — key colour / intensity (position only when the preset sets
  //    one, so noon never moves a set's sun), rim intensity (colour untouched),
  //    and the hemisphere light found inside the rig's group.
  const rig = opts.rig;
  if (rig) {
    if (rig.key) {
      rig.key.color.set(preset.keyCol);
      rig.key.intensity = preset.keyI;
      if (preset.keyPos) rig.key.position.set(preset.keyPos[0], preset.keyPos[1], preset.keyPos[2]);
    }
    if (rig.rim) rig.rim.intensity = preset.rimI;
    if (rig.group) {
      rig.group.traverse((obj) => {
        if ((obj as THREE.HemisphereLight).isHemisphereLight) {
          const hemi = obj as THREE.HemisphereLight;
          hemi.color.set(preset.hemiSky);
          hemi.groundColor.set(preset.hemiGround);
          hemi.intensity = preset.hemiI;
        }
      });
    }
  }

  // 3b. Interior fills (v5, additive): a rig that exposes its point-light fills
  //     gets them scaled by the preset's grade exposure and lerped halfway
  //     toward the key colour, so 'night' indoors dims/re-colours the bulbs
  //     instead of leaving them burning at full warm. Rigs without fills
  //     (every v4 set) skip this entirely — pixels unchanged.
  if (rig?.fills) {
    const key = new THREE.Color(preset.keyCol);
    rig.fills.forEach((fill) => {
      fill.intensity *= preset.grade.exposure;
      fill.color.lerp(key, 0.5);
    });
  }

  // 4. Env-match pass — structural probe: Toon sets group.userData.toon = this
  //    and the group sits in the scene graph, so traversal finds every toon.
  scene.traverse((obj) => {
    const toon = (obj as any).userData?.toon;
    if (toon && toon.envMatch === true && typeof toon.matchEnv === 'function') {
      toon.matchEnv(preset.tint);
    }
  });

  // 5. The caller owns the render grade.
  return preset;
}
