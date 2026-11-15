// The Tall Tale world: a sunny, saturated, slightly old-timey cartoon world built in 3D with painted canvas textures.
//   sky(scene, { top, horizon, sunset }) · sunlight(scene, opts) · clouds(scene, list)
//   mainStreet(E, opts) -> { scene, sun, people: [...] }      a small-town street with shops, poles, wires, hills
//   interior(E, opts)  -> { scene, sun, counter, window }      a wallpapered shop/office room with a counter and a window
//   meadow(E, opts)    -> { scene, sun }                       grass, hills, fence, trees, flowers (day or sunset)
// Textures: planks, wallpaper, brick, dirt, grass, siding, facade(name, ...), sign(text) — all drawn in code.
import * as THREE from 'three';
import { lightRig, type Engine, type LightRig } from './engine';
import { Toon, type Mood, type ToonLook } from './toon';
import { barn, boat, house, lander, tent, sandbags, watchtower, type HouseGroup } from './props';

const rnd = (i: number) => { const x = Math.sin(i * 127.1 + 311.7) * 43758.5453; return x - Math.floor(x); };

export type ShopEntry = readonly [string, string, string, string | null];
export interface BuildingInfo { name: string; side: number; z: number; w: number; h: number; x: number }
export interface MainStreetOptions { len?: number; people?: boolean; cars?: boolean; dusk?: boolean; shops?: ReadonlyArray<ShopEntry> }
export interface MainStreetSet { scene: THREE.Scene; sun: LightRig; people: Toon[]; cars: THREE.Group[]; buildings: BuildingInfo[]; faceAll: (cam: THREE.Camera) => void }
export interface InteriorOptions { w?: number; depth?: number; h?: number; wall?: string; motif?: string; floor?: string; ceiling?: string; view?: THREE.Texture | null; bulbs?: boolean; signText?: string | null; picture?: boolean; counterOn?: boolean; litter?: boolean; preset?: 'shop' | 'home' | 'office' | 'classroom' }
export interface InteriorSet { scene: THREE.Scene; sun: { key: THREE.DirectionalLight; fills: THREE.PointLight[] }; counter: THREE.Group; window: THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial>; w: number; depth: number; h: number }
export interface MeadowOptions { sunset?: boolean; flowers?: boolean; fenceOn?: boolean; treesOn?: boolean }
export interface MeadowSet { scene: THREE.Scene; sun: LightRig }
export interface FacadeOptions { wall?: string; kind?: string; trim?: string; sign?: string; signText?: string; awning?: string | null; w?: number; h?: number; seed?: number }
export interface SignBoardOptions { w?: number; h?: number; bg?: string; fg?: string; font?: string }
export interface SkyOptions { top?: string; horizon?: string; ground?: string; r?: number }
export interface SunlightOptions { col?: string; i?: number; pos?: [number, number, number]; target?: [number, number, number]; box?: number; sky?: string; ground?: string; hemi?: number; env?: number }

const TEX: Record<string, THREE.CanvasTexture> = {};
export function canvasTex(key: string, w: number, h: number, draw: (g: CanvasRenderingContext2D, w: number, h: number) => void, rep: [number, number] = [1, 1]): THREE.Texture {
  let t = TEX[key];
  if (!t) {
    const c = document.createElement('canvas'); c.width = w; c.height = h; draw(c.getContext('2d')!, w, h);
    t = new THREE.CanvasTexture(c); t.colorSpace = THREE.SRGBColorSpace; t.wrapS = t.wrapT = THREE.RepeatWrapping; t.anisotropy = 8; TEX[key] = t;
  }
  const u = t.clone(); u.repeat.set(rep[0], rep[1]); u.needsUpdate = true; return u;
}
const shade = (hex: string, k: number): string => { const c = new THREE.Color(hex); c.multiplyScalar(k); return '#' + c.getHexString(); };
// Shared caches (a02/r04): keyed by every parameter that reaches the shader/geometry.
// Geometry: no code in world.js/props.js (or anywhere in template/src) ever mutates a geometry
// after creation — transforms live on the mesh — so one geometry per exact size is safe to share.
// Materials: ONLY flat specs are cached — no `map` and every value a primitive. Textured materials
// are always built fresh, because three call sites mutate their texture after creation
// (boardwalk map rotation in mainStreet, floor map rotation + side-wall map repeat in interior);
// a fresh material + its per-call texture clone keeps those mutations private to that one mesh.
const MATCACHE: Record<string, THREE.MeshStandardMaterial> = {};
export const mat = (o: THREE.MeshStandardMaterialParameters = {}): THREE.MeshStandardMaterial => {
  if (o.map || Object.values(o).some((v) => v !== null && typeof v === 'object')) return new THREE.MeshStandardMaterial({ roughness: 0.85, metalness: 0, ...o });
  const eff = { roughness: 0.85, metalness: 0, ...o };
  const key = Object.keys(eff).sort().map((k) => k + '=' + String(eff[k as keyof THREE.MeshStandardMaterialParameters])).join('|');
  return MATCACHE[key] || (MATCACHE[key] = new THREE.MeshStandardMaterial(eff));
};
const GEOCACHE: Record<string, THREE.BufferGeometry> = {};
const boxGeo = (w: number, h: number, d: number) => { const k = 'b|' + w + '|' + h + '|' + d; return GEOCACHE[k] || (GEOCACHE[k] = new THREE.BoxGeometry(w, h, d)); };
const cylGeo = (r1: number, r2: number, h: number, seg: number) => { const k = 'c|' + r1 + '|' + r2 + '|' + h + '|' + seg; return GEOCACHE[k] || (GEOCACHE[k] = new THREE.CylinderGeometry(r1, r2, h, seg)); };
const box = (w: number, h: number, d: number, m: THREE.Material | THREE.Material[], x = 0, y = 0, z = 0, parent?: THREE.Object3D) => { const b = new THREE.Mesh(boxGeo(w, h, d), m); b.position.set(x, y, z); b.castShadow = b.receiveShadow = true; if (parent) parent.add(b); return b; };
const cyl = (r1: number, r2: number, h: number, m: THREE.Material | THREE.Material[], x: number, y: number, z: number, parent?: THREE.Object3D, seg = 12) => { const c = new THREE.Mesh(cylGeo(r1, r2, h, seg), m); c.position.set(x, y, z); c.castShadow = c.receiveShadow = true; if (parent) parent.add(c); return c; };

// ---------- painted textures ----------
export const T = {
  planks: (col = '#A8693E', rep: [number, number] = [1, 1]) => canvasTex('planks' + col, 512, 512, (g, w, h) => {
    const n = 8, ph = h / n;
    for (let i = 0; i < n; i++) {
      g.fillStyle = shade(col, 0.86 + rnd(i * 3) * 0.24); g.fillRect(0, i * ph, w, ph);
      g.strokeStyle = shade(col, 0.7); g.globalAlpha = 0.35; g.lineWidth = 2;
      for (let k = 0; k < 4; k++) { g.beginPath(); const y = i * ph + 8 + rnd(i * 7 + k) * (ph - 16); g.moveTo(0, y); g.bezierCurveTo(w * 0.3, y + 6, w * 0.6, y - 6, w, y + 2); g.stroke(); }
      g.globalAlpha = 1; g.fillStyle = shade(col, 0.45); g.fillRect(0, i * ph, w, 4);
      const jx = rnd(i * 11) * w; g.fillRect(jx, i * ph, 4, ph);
      g.fillStyle = shade(col, 0.5); [jx - 14, jx + 18].forEach((x) => { g.beginPath(); g.arc(x, i * ph + ph / 2, 3, 0, 7); g.fill(); });
    }
  }, rep),
  wallpaper: (col = '#9E2E3A', motif = '#B8414C', rep: [number, number] = [1, 1]) => canvasTex('wp' + col + motif, 256, 256, (g, w, h) => {
    g.fillStyle = col; g.fillRect(0, 0, w, h); g.fillStyle = motif; g.strokeStyle = motif; g.lineWidth = 5;
    const fleur = (x: number, y: number, s: number) => { g.save(); g.translate(x, y); g.scale(s, s); g.beginPath(); g.moveTo(0, -40); g.bezierCurveTo(18, -20, 18, 0, 0, 14); g.bezierCurveTo(-18, 0, -18, -20, 0, -40); g.fill(); g.beginPath(); g.arc(-22, 4, 12, 0, 7); g.arc(22, 4, 12, 0, 7); g.fill(); g.fillRect(-3, 10, 6, 26); g.restore(); };
    [[64, 64], [192, 192], [192, 64 - 256], [64, 192]].forEach(([x, y], i) => i < 2 && fleur(x, y, 0.9));
    g.globalAlpha = 0.5; for (let i = 0; i < 4; i++) { g.beginPath(); g.arc(i % 2 ? 192 : 64, i < 2 ? 192 : 64, 6, 0, 7); g.fill(); } g.globalAlpha = 1;
  }, rep),
  brick: (col = '#B5553C', rep: [number, number] = [1, 1]) => canvasTex('brick' + col, 512, 512, (g, w, h) => {
    g.fillStyle = shade(col, 0.6); g.fillRect(0, 0, w, h); const bh = 32, bw = 85;
    for (let r = 0; r < h / bh; r++) for (let c = -1; c < w / bw + 1; c++) { const x = c * bw + (r % 2) * bw / 2; g.fillStyle = shade(col, 0.85 + rnd(r * 31 + c) * 0.3); g.fillRect(x + 3, r * bh + 3, bw - 6, bh - 6); }
  }, rep),
  siding: (col = '#6FA3B8', rep: [number, number] = [1, 1]) => canvasTex('siding' + col, 256, 256, (g, w, h) => {
    for (let i = 0; i < 8; i++) { const y = i * 32; const gr = g.createLinearGradient(0, y, 0, y + 32); gr.addColorStop(0, shade(col, 1.06)); gr.addColorStop(1, shade(col, 0.86)); g.fillStyle = gr; g.fillRect(0, y, w, 32); g.fillStyle = shade(col, 0.6); g.fillRect(0, y + 30, w, 2); }
  }, rep),
  dirt: (col = '#C8763F', rep: [number, number] = [1, 1], tracks = true) => canvasTex('dirt' + col + tracks, 512, 512, (g, w, h) => {
    g.fillStyle = col; g.fillRect(0, 0, w, h);
    for (let i = 0; i < 2600; i++) { g.fillStyle = shade(col, 0.75 + rnd(i) * 0.5); g.globalAlpha = 0.5; g.beginPath(); g.arc(rnd(i * 2.1) * w, rnd(i * 3.7) * h, 1 + rnd(i * 5.3) * 2.5, 0, 7); g.fill(); }
    g.globalAlpha = 0.18; g.fillStyle = shade(col, 0.6);
    if (tracks) [0.3, 0.38, 0.62, 0.7].forEach((x) => g.fillRect(x * w, 0, 14, h));
    g.globalAlpha = 1;
  }, rep),
  grass: (col = '#6DBB4A', rep: [number, number] = [1, 1]) => canvasTex('grass' + col, 512, 512, (g, w, h) => {
    g.fillStyle = col; g.fillRect(0, 0, w, h);
    for (let i = 0; i < 5000; i++) { const x = rnd(i * 1.3) * w, y = rnd(i * 2.9) * h; g.strokeStyle = shade(col, 0.7 + rnd(i * 7.7) * 0.6); g.lineWidth = 2; g.beginPath(); g.moveTo(x, y); g.lineTo(x + (rnd(i) - 0.5) * 6, y - 6 - rnd(i * 9) * 8); g.stroke(); }
  }, rep),
  sand: (col = '#E3C078', rep: [number, number] = [1, 1]) => canvasTex('sand' + col, 512, 512, (g, w, h) => {
    g.fillStyle = col; g.fillRect(0, 0, w, h);
    for (let i = 0; i < 3200; i++) { g.fillStyle = shade(col, 0.78 + rnd(i) * 0.45); g.globalAlpha = 0.5; g.beginPath(); g.arc(rnd(i * 2.3) * w, rnd(i * 3.1) * h, 1 + rnd(i * 5.7) * 2, 0, 7); g.fill(); }
    g.globalAlpha = 0.16; g.strokeStyle = shade(col, 0.68); g.lineWidth = 3;
    for (let r = 0; r < 7; r++) { g.beginPath(); for (let x = 0; x <= w; x += 16) { const y = 30 + r * 70 + Math.sin(x * 0.05 + r * 2.2) * 7; x === 0 ? g.moveTo(x, y) : g.lineTo(x, y); } g.stroke(); }
    g.globalAlpha = 1;
  }, rep),
  snow: (col = '#F4F8FC', rep: [number, number] = [1, 1]) => canvasTex('snow' + col, 512, 512, (g, w, h) => {
    g.fillStyle = col; g.fillRect(0, 0, w, h);
    for (let i = 0; i < 2400; i++) { g.fillStyle = shade(col, 0.9 + rnd(i) * 0.09); g.globalAlpha = 0.55; g.beginPath(); g.arc(rnd(i * 1.9) * w, rnd(i * 2.7) * h, 1 + rnd(i * 4.3) * 3, 0, 7); g.fill(); }
    g.globalAlpha = 0.10; g.fillStyle = '#9FB8D8'; for (let i = 0; i < 260; i++) { g.beginPath(); g.arc(rnd(i * 3.3) * w, rnd(i * 6.1) * h, 1.5 + rnd(i * 2.2) * 2.5, 0, 7); g.fill(); }
    g.globalAlpha = 1; g.fillStyle = '#FFFFFF'; for (let i = 0; i < 130; i++) g.fillRect(rnd(i * 7.7) * w, rnd(i * 8.3) * h, 2.5, 2.5);
  }, rep),
  felt: (col = '#1F5A45') => canvasTex('felt' + col, 256, 256, (g, w, h) => { g.fillStyle = col; g.fillRect(0, 0, w, h); for (let i = 0; i < 3000; i++) { g.fillStyle = shade(col, 0.85 + rnd(i) * 0.3); g.fillRect(rnd(i * 3) * w, rnd(i * 7) * h, 2, 2); } }),
};

/** a painted shopfront (front face of a building) */
export function facade(name: string, { wall = '#C9603F', kind = 'brick', trim = '#F2E6CF', sign = '#2E4E3F', signText = '#F2D37A', awning = null, w = 5, h = 7, seed = 1 }: FacadeOptions = {}): THREE.Texture {
  const px = 110, cw = Math.round(w * px), ch = Math.round(h * px);
  return canvasTex('fac' + name + wall + w + h + seed, cw, ch, (g) => {
    if (kind === 'brick') { const b = T.brick(wall).image; for (let y = 0; y < ch; y += 512) for (let x = 0; x < cw; x += 512) g.drawImage(b, x, y); }
    else { const b = T.siding(wall).image; for (let y = 0; y < ch; y += 256) for (let x = 0; x < cw; x += 256) g.drawImage(b, x, y); }
    // cornice
    g.fillStyle = trim; g.fillRect(0, 0, cw, 40); g.fillStyle = shade(trim, 0.7); g.fillRect(0, 40, cw, 10);
    for (let x = 10; x < cw; x += 34) { g.fillStyle = trim; g.fillRect(x, 50, 16, 16); }
    // upper windows
    const nw = Math.max(2, Math.round(w / 1.6)), ww = cw / nw;
    for (let i = 0; i < nw; i++) {
      const x = i * ww + ww * 0.22, y = ch * 0.16, wd = ww * 0.56, ht = ch * 0.22;
      g.fillStyle = trim; g.fillRect(x - 10, y - 14, wd + 20, ht + 28);
      const gr = g.createLinearGradient(x, y, x + wd, y + ht); gr.addColorStop(0, '#7FB3D8'); gr.addColorStop(0.5, '#3F6E9A'); gr.addColorStop(1, '#2B4A70'); g.fillStyle = gr; g.fillRect(x, y, wd, ht);
      g.fillStyle = 'rgba(255,255,255,0.35)'; g.beginPath(); g.moveTo(x + wd * 0.1, y + ht); g.lineTo(x + wd * 0.45, y); g.lineTo(x + wd * 0.6, y); g.lineTo(x + wd * 0.25, y + ht); g.fill();
      g.fillStyle = trim; g.fillRect(x + wd / 2 - 4, y, 8, ht); g.fillRect(x, y + ht / 2 - 4, wd, 8);
      if (rnd(seed * 13 + i) > 0.5) { g.fillStyle = shade(wall, 0.8); g.fillRect(x, y, wd, ht * 0.35); }
    }
    // sign board
    const sy = ch * 0.47, sh = ch * 0.1;
    g.fillStyle = sign; g.fillRect(cw * 0.06, sy, cw * 0.88, sh); g.strokeStyle = signText; g.lineWidth = 5; g.strokeRect(cw * 0.06 + 8, sy + 8, cw * 0.88 - 16, sh - 16);
    g.fillStyle = signText; g.textAlign = 'center'; g.textBaseline = 'middle'; let fs = sh * 0.56; g.font = `700 ${fs}px Caladea, Georgia, serif`;
    while (g.measureText(name).width > cw * 0.78 && fs > 10) { fs -= 2; g.font = `700 ${fs}px Caladea, Georgia, serif`; }
    g.fillText(name, cw / 2, sy + sh / 2 + 2);
    // shop window + door
    const by = sy + sh + 16, bh = ch - by;
    g.fillStyle = shade(trim, 0.9); g.fillRect(0, by, cw, bh);
    const dw = cw * 0.2, dx = rnd(seed) > 0.5 ? cw * 0.08 : cw * 0.72;
    g.fillStyle = '#5A3A26'; g.fillRect(dx, by + 14, dw, bh - 14); g.fillStyle = '#3F6E9A'; g.fillRect(dx + 12, by + 30, dw - 24, bh * 0.4);
    g.fillStyle = '#E8C060'; g.beginPath(); g.arc(dx + dw - 18, by + bh * 0.62, 6, 0, 7); g.fill();
    const wx = dx < cw / 2 ? dx + dw + 20 : cw * 0.06, wwid = cw * 0.66 - 20;
    const gr = g.createLinearGradient(wx, by, wx + wwid, by + bh); gr.addColorStop(0, '#9CC7E4'); gr.addColorStop(0.45, '#4C7FA8'); gr.addColorStop(1, '#2A4565');
    g.fillStyle = '#4A2E1E'; g.fillRect(wx - 8, by + 14, wwid + 16, bh * 0.7 + 16); g.fillStyle = gr; g.fillRect(wx, by + 22, wwid, bh * 0.7);
    for (let i = 0; i < 5; i++) { g.fillStyle = ['#E7B04A', '#D8584A', '#7DB36A', '#F2E6CF', '#8A6FC0'][(i + seed) % 5]; g.fillRect(wx + 16 + i * (wwid - 32) / 5, by + bh * 0.55, (wwid - 32) / 5 - 10, bh * 0.2); }
    g.fillStyle = 'rgba(255,255,255,0.3)'; g.beginPath(); g.moveTo(wx + wwid * 0.15, by + 22 + bh * 0.7); g.lineTo(wx + wwid * 0.4, by + 22); g.lineTo(wx + wwid * 0.52, by + 22); g.lineTo(wx + wwid * 0.27, by + 22 + bh * 0.7); g.fill();
    if (awning) { const ay = by - 4; for (let x = 0, i = 0; x < cw; x += 40, i++) { g.fillStyle = i % 2 ? '#F4EFE4' : awning; g.fillRect(x, ay, 40, 46); g.beginPath(); g.arc(x + 20, ay + 46, 20, 0, Math.PI); g.fill(); } }
  });
}

/** a flat painted sign (plank board with text) as a plane mesh */
export function signBoard(text: string, { w = 2, h = 0.5, bg = '#2E4E3F', fg = '#F2D37A', font = 'Caladea' }: SignBoardOptions = {}): THREE.Mesh {
  const t = canvasTex('sign' + text + bg + fg + w + h, Math.round(w * 200), Math.round(h * 200), (g, cw, ch) => {
    g.fillStyle = bg; g.fillRect(0, 0, cw, ch); g.strokeStyle = fg; g.lineWidth = 6; g.strokeRect(10, 10, cw - 20, ch - 20);
    g.fillStyle = fg; g.textAlign = 'center'; g.textBaseline = 'middle'; let fs = ch * 0.5; g.font = `700 ${fs}px ${font}, Georgia, serif`;
    while (g.measureText(text).width > cw * 0.86) { fs -= 2; g.font = `700 ${fs}px ${font}, Georgia, serif`; }
    g.fillText(text, cw / 2, ch / 2 + 3);
  });
  const m = new THREE.Mesh(new THREE.BoxGeometry(w, h, 0.05), [mat({ color: shade(bg, 0.7) }), mat({ color: shade(bg, 0.7) }), mat({ color: shade(bg, 0.7) }), mat({ color: shade(bg, 0.7) }), mat({ map: t }), mat({ color: shade(bg, 0.7) })]);
  m.castShadow = true; return m;
}

// ---------- sky, sun, clouds ----------
export function sky(scene: THREE.Scene, { top = '#3D8EE0', horizon = '#BFE4F7', ground = '#BFE4F7', r = 380 }: SkyOptions = {}): THREE.Mesh {
  const m = new THREE.ShaderMaterial({ side: THREE.BackSide, depthWrite: false, fog: false, toneMapped: false,
    uniforms: { top: { value: new THREE.Color(top) }, hor: { value: new THREE.Color(horizon) }, gnd: { value: new THREE.Color(ground) } },
    vertexShader: 'varying vec3 vP; void main(){ vP = normalize(position); gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }',
    fragmentShader: 'uniform vec3 top, hor, gnd; varying vec3 vP; void main(){ float y = vP.y; vec3 c = y > 0.0 ? mix(hor, top, pow(clamp(y * 1.6, 0.0, 1.0), 0.7)) : gnd; gl_FragColor = vec4(c, 1.0); }' });
  const s = new THREE.Mesh(new THREE.SphereGeometry(r, 32, 16), m); s.renderOrder = -10; scene.add(s); return s;
}
let cloudT: THREE.CanvasTexture | null = null;
function cloudTex(): THREE.CanvasTexture {
  if (cloudT) return cloudT;
  const c = document.createElement('canvas'); c.width = 512; c.height = 256; const g = c.getContext('2d')!;
  const puffs = [[110, 175, 58], [185, 140, 80], [285, 118, 96], [380, 150, 70], [250, 178, 70], [440, 180, 44], [70, 190, 36]];
  g.filter = 'blur(3px)';
  g.fillStyle = '#B9CCE6'; puffs.forEach(([x, y, r]) => { g.beginPath(); g.arc(x + 4, y + 14, r, 0, 7); g.fill(); });
  g.fillStyle = '#FFFFFF'; puffs.forEach(([x, y, r]) => { g.beginPath(); g.arc(x - 2, y - 2, r * 0.92, 0, 7); g.fill(); });
  g.globalCompositeOperation = 'destination-out'; g.filter = 'blur(10px)'; g.fillRect(0, 214, 512, 60);
  cloudT = new THREE.CanvasTexture(c); cloudT.colorSpace = THREE.SRGBColorSpace; return cloudT;
}
export function clouds(scene: THREE.Scene, list: ReadonlyArray<readonly [number, number, number, number]>, tint = '#FFFFFF'): THREE.Sprite[] {
  const out: THREE.Sprite[] = [];
  list.forEach(([x, y, z, s]) => { const sp = new THREE.Sprite(new THREE.SpriteMaterial({ map: cloudTex(), color: tint, fog: false, toneMapped: false, depthWrite: false })); sp.position.set(x, y, z); sp.scale.set(s * 2, s, 1); scene.add(sp); out.push(sp); });
  return out;
}
export function sunlight(scene: THREE.Scene, E: Engine, { col = '#FFF0D6', i = 2.6, pos = [6, 10, 6], target = [0, 0, 0], box = 12, sky = '#CFE6FF', ground = '#C48A5A', hemi = 1.25, env = 0.25 }: SunlightOptions = {}): LightRig {
  scene.environment = E.envMap; scene.environmentIntensity = env;
  const r = lightRig(scene, { key: col, keyI: i, keyPos: pos, target, shadowBox: box, rimCol: '#FFFFFF', rimI: 0.4, sky, ground, hemiI: hemi });
  r.key.shadow.camera.far = 80; r.key.shadow.bias = -0.0006; r.key.shadow.camera.updateProjectionMatrix();
  return r;
}

// ---------- reusable bits ----------
export function hills(scene: THREE.Scene, { col = '#79C25A', z = -90, list }: { col?: string; z?: number; list?: Array<[number, number, number, number]> } = {}): THREE.Group {
  const g = new THREE.Group(); scene.add(g);
  (list || [[-40, z, 34, 12], [10, z - 10, 46, 16], [60, z + 4, 36, 11], [-90, z + 10, 40, 10]]).forEach(([x, zz, r, h], i) => {
    const m = new THREE.Mesh(new THREE.SphereGeometry(1, 32, 16), mat({ color: shade(col, 0.92 + rnd(i) * 0.16), map: T.grass(col, [6, 3]) }));
    m.scale.set(r, h, r * 0.6); m.position.set(x, -h * 0.25, zz); g.add(m);
    for (let k = 0; k < 6; k++) { const a = rnd(i * 9 + k) * 2 - 1; const t = tree(0.5 + rnd(k) * 0.4); t.traverse((o) => { if ((o as THREE.Mesh).isMesh) (o as THREE.Mesh).castShadow = false; }); t.position.set(x + a * r * 0.6, h * (0.75 - Math.abs(a) * 0.45) - 1, zz + r * 0.25); t.scale.multiplyScalar(2.2); g.add(t); }
  });
  return g;
}
export function tree(s = 1, col = '#4E9A3C', kind = 'round'): THREE.Group {
  const g = new THREE.Group();
  cyl(0.12 * s, 0.18 * s, 1.6 * s, mat({ color: '#7A5234' }), 0, 0.8 * s, 0, g);
  const lm = mat({ color: col });
  if (kind === 'pine') { // v5: trunk + 3 stacked cones (snowfield/mountain); pass a pale col for a snow-dusted read
    [[1.15, 1.9], [0.85, 2.75], [0.55, 3.55]].forEach(([r, y], i) => { const c = new THREE.Mesh(new THREE.ConeGeometry(r * s, 1.7 * s, 10), mat({ color: shade(col, 0.88 + i * 0.09) })); c.position.y = y * s; c.castShadow = true; g.add(c); });
    return g;
  }
  if (kind === 'cypress') { const c = new THREE.Mesh(new THREE.SphereGeometry(1, 16, 12), lm); c.scale.set(0.7 * s, 2.6 * s, 0.7 * s); c.position.y = 3.2 * s; c.castShadow = true; g.add(c); }
  else [[0, 2.3, 0, 1.0], [0.6, 2.0, 0.2, 0.75], [-0.6, 2.0, -0.1, 0.8], [0.1, 2.9, -0.2, 0.75], [0.2, 2.2, 0.6, 0.7]].forEach(([x, y, z, r], i) => {
    const m = new THREE.Mesh(new THREE.IcosahedronGeometry(r * s, 2), mat({ color: shade(col, 0.9 + rnd(i) * 0.2) })); m.position.set(x * s, y * s, z * s); m.castShadow = true; g.add(m);
  });
  return g;
}
export function pole(h = 7): THREE.Group {
  const g = new THREE.Group(), w = mat({ color: '#6E4E36' });
  cyl(0.09, 0.12, h, w, 0, h / 2, 0, g, 8); box(1.8, 0.1, 0.1, w, 0, h - 0.6, 0, g); box(1.2, 0.08, 0.08, w, 0, h - 1.1, 0, g);
  [-0.8, -0.3, 0.3, 0.8].forEach((x) => { const ins = cyl(0.03, 0.03, 0.12, mat({ color: '#CFE3E8' }), x, h - 0.5, 0, g, 6); ins.castShadow = false; });
  return g;
}
export function wire(a: THREE.Vector3, b: THREE.Vector3, sag = 0.6, col = '#2A2A33'): THREE.Mesh {
  const pts = []; for (let i = 0; i <= 20; i++) { const t = i / 20; const p = new THREE.Vector3().lerpVectors(a, b, t); p.y -= Math.sin(t * Math.PI) * sag; pts.push(p); }
  const m = new THREE.Mesh(new THREE.TubeGeometry(new THREE.CatmullRomCurve3(pts), 20, 0.015, 4), new THREE.MeshBasicMaterial({ color: col })); return m;
}
export function lamp(): THREE.Group {
  const g = new THREE.Group(), m = mat({ color: '#2E3A35', metalness: 0.3, roughness: 0.5 });
  cyl(0.05, 0.07, 3.2, m, 0, 1.6, 0, g, 8); cyl(0.18, 0.1, 0.3, m, 0, 3.3, 0, g, 8);
  const b = new THREE.Mesh(new THREE.SphereGeometry(0.14, 12, 8), new THREE.MeshStandardMaterial({ color: '#FFF3C8', emissive: '#FFE7A0', emissiveIntensity: 0.6 })); b.position.y = 3.15; g.add(b);
  return g;
}
export function barrel(): THREE.Group { const g = new THREE.Group(); cyl(0.33, 0.33, 0.8, mat({ color: '#9A6337', map: T.planks('#9A6337') }), 0, 0.4, 0, g, 16); [0.12, 0.68].forEach((y) => cyl(0.345, 0.345, 0.05, mat({ color: '#3A3A3F', metalness: 0.4 }), 0, y, 0, g, 16)); return g; }
export function crate(s = 0.6): THREE.Group { const g = new THREE.Group(); box(s, s, s, mat({ map: T.planks('#B07A48', [0.5, 0.5]) }), 0, s / 2, 0, g); return g; }
export function car(col = '#2C3A4A'): THREE.Group {
  const g = new THREE.Group(), m = mat({ color: col, roughness: 0.4, metalness: 0.2 }), dark = mat({ color: '#1E1E24', roughness: 0.6 });
  box(1.5, 0.5, 3.2, m, 0, 0.75, 0, g); box(1.3, 0.8, 1.4, m, 0, 1.4, -0.3, g);
  box(1.2, 0.5, 0.05, mat({ color: '#9CC7E4', roughness: 0.1 }), 0, 1.5, 0.41, g);
  [[-0.78, 1.1], [0.78, 1.1], [-0.78, -1.1], [0.78, -1.1]].forEach(([x, z]) => { const w = cyl(0.36, 0.36, 0.22, dark, x, 0.36, z, g, 16); w.rotation.z = Math.PI / 2; const hub = cyl(0.12, 0.12, 0.24, mat({ color: '#D9D2C2' }), x, 0.36, z, g, 10); hub.rotation.z = Math.PI / 2; });
  [-0.5, 0.5].forEach((x) => { const l = new THREE.Mesh(new THREE.SphereGeometry(0.12, 10, 8), mat({ color: '#FFF3C8', emissive: '#FFE7A0', emissiveIntensity: 0.3 })); l.position.set(x, 0.95, 1.62); g.add(l); });
  return g;
}
export function fence(len = 10, col = '#F4EFE4'): THREE.Group {
  const g = new THREE.Group(), m = mat({ color: col });
  for (let x = 0; x <= len; x += 1.5) box(0.1, 1.0, 0.1, m, x, 0.5, 0, g);
  [0.45, 0.85].forEach((y) => box(len, 0.08, 0.06, m, len / 2, y, 0, g));
  return g;
}
export function bunting(a: THREE.Vector3, b: THREE.Vector3, cols: string[] = ['#D8343F', '#F4EFE4', '#3C6FB6', '#F2C230']): THREE.Group {
  const g = new THREE.Group(); g.add(wire(a, b, 0.5, '#3A2A2A'));
  const n = Math.round(a.distanceTo(b) / 0.5);
  for (let i = 1; i < n; i++) {
    const t = i / n, p = new THREE.Vector3().lerpVectors(a, b, t); p.y -= Math.sin(t * Math.PI) * 0.5;
    const tri = new THREE.Mesh(new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(-0.16, 0, 0), new THREE.Vector3(0.16, 0, 0), new THREE.Vector3(0, -0.32, 0)]), new THREE.MeshBasicMaterial({ color: cols[i % cols.length], side: THREE.DoubleSide }));
    tri.position.copy(p); tri.lookAt(p.x + (b.z - a.z), p.y, p.z - (b.x - a.x)); g.add(tri);
  }
  return g;
}
const LOOKS: ToonLook[] = [
  { body: '#5C8FD6', hat: 'bowler', hatCol: '#3B3346' }, { body: '#E8734A', hat: 'cap', hatCol: '#6E8A4F' }, { body: '#F2B8C6', hat: 'flower', pattern: 'apron' },
  { body: '#9BC46A', hat: 'boater', hatCol: '#E8C77A' }, { body: '#8C6FC9', hat: 'tophat', hatCol: '#2A2638' }, { body: '#F2C230', hat: 'beanie', hatCol: '#D8343F', pattern: 'overalls' },
  { body: '#4FB3A9', hat: 'none', neck: 'tie' }, { body: '#D8584A', hat: 'cap', hatCol: '#3B3346', pattern: 'stripes' },
];
/** cheap background extras: small cut-outs at low resolution */
export function extras(scene: THREE.Scene, spots: ReadonlyArray<readonly [number, number, (number | undefined)?, (Mood | undefined)?]>, moodFn?: unknown): Toon[] {
  return spots.map(([x, z, k, mood], i) => { const p = new Toon(LOOKS[(k ?? i) % LOOKS.length], { height: 1.7, res: 0.5 }); p.set({ mood: mood || 'neutral', armL: [8, 6], armR: [8, 6] }); p.at(x, 0, z); scene.add(p.group); return p; });
}

// ---------- SETS ----------
const SHOPS: ShopEntry[] = [['BAKERY', '#C9603F', 'brick', '#D8343F'], ['GENERAL STORE', '#6FA3B8', 'wood', null], ['BARBER', '#E3C16F', 'wood', '#3C6FB6'], ['GRAND HOTEL', '#9E4A3A', 'brick', null],
  ['POST OFFICE', '#7FA36A', 'wood', null], ['TAILOR', '#B98ACF', 'wood', '#2E4E3F'], ['HARDWARE', '#B5553C', 'brick', '#F2C230'], ['CAFE', '#E58A6A', 'wood', '#2E4E3F'],
  ['BANK', '#C7B79A', 'brick', null], ['PHARMACY', '#5FA39A', 'wood', '#D8343F'], ['SHOES', '#D9A34A', 'wood', null], ['THEATER', '#A3485A', 'brick', '#F2C230']];

/** a small-town main street running along -z. The road is x -4..4; boardwalks x ±4..6; shop fronts at x ±6. */
export function mainStreet(E: Engine, { len = 70, people = true, cars = true, dusk = false, shops = SHOPS }: MainStreetOptions = {}): MainStreetSet {
  const buildings: BuildingInfo[] = [];
  const scene = new THREE.Scene();
  sky(scene, dusk ? { top: '#3B3F8F', horizon: '#F59C6B' } : {});
  const sun = sunlight(scene, E, dusk ? { col: '#FFB27A', i: 2.0, pos: [-8, 5, -14], sky: '#B9A2D8', ground: '#7A4A3A', hemi: 0.9 } : { pos: [9, 13, 5], box: 16 });
  clouds(scene, [[-30, 26, -100, 16], [20, 34, -110, 20], [55, 22, -90, 12], [-60, 30, -80, 14], [5, 20, -70, 9]], dusk ? '#FFC9B0' : '#FFFFFF');
  hills(scene, { z: -len - 30 });
  const road = new THREE.Mesh(new THREE.PlaneGeometry(8, len + 40), mat({ map: T.dirt('#C98A52', [2, (len + 40) / 8]) })); road.rotation.x = -Math.PI / 2; road.position.set(0, 0, -len / 2 + 10); road.receiveShadow = true; scene.add(road);
  const ground = new THREE.Mesh(new THREE.PlaneGeometry(300, 300), mat({ map: T.grass('#79C25A', [60, 60]) })); ground.rotation.x = -Math.PI / 2; ground.position.y = -0.02; ground.receiveShadow = true; scene.add(ground);
  [-1, 1].forEach((s) => {
    const walk = box(2.2, 0.18, len + 20, mat({ map: T.planks('#A8693E', [(len + 20) / 3, 1]) }), s * 5.1, 0.09, -len / 2 + 5, scene);
    (walk.material as THREE.MeshStandardMaterial).map!.rotation = Math.PI / 2;
    let z = 8, k = s > 0 ? 0 : 5;
    while (z > -len) {
      const [name, wall, kind, aw] = shops[k % shops.length]; const w = 5 + rnd(k) * 2.5, h = 6 + rnd(k * 3) * 3.5; k++;
      const front = mat({ map: facade(name, { wall, kind, awning: aw, w, h, seed: k }) }), side = mat({ color: shade(wall, 0.85) });
      const b = new THREE.Mesh(new THREE.BoxGeometry(6, h, w), s < 0 ? [front, side, side, side, side, side] : [side, front, side, side, side, side]);
      b.position.set(s * 9.2, h / 2, z - w / 2); b.castShadow = b.receiveShadow = true; scene.add(b); buildings.push({ name, side: s, z: z - w / 2, w, h, x: s * 6.2 });
      if (aw) { const a = box(1.4, 0.06, w - 0.4, mat({ color: aw }), s * 5.6, 2.9, z - w / 2, scene); a.rotation.z = s * 0.35; }
      z -= w + 0.15;
    }
    for (let zz = 4; zz > -len; zz -= 12) { const p = pole(7.5); p.position.set(s * 4.4, 0, zz); scene.add(p); }
    for (let zz = -2; zz > -len; zz -= 16) { const l = lamp(); l.position.set(s * 4.3, 0.18, zz); scene.add(l); }
  });
  for (let zz = 4; zz > -len + 12; zz -= 12) [-1, 1].forEach((s) => [-0.8, -0.3, 0.3, 0.8].forEach((dx) => scene.add(wire(new THREE.Vector3(s * 4.4 + dx, 6.95, zz), new THREE.Vector3(s * 4.4 + dx, 6.95, zz - 12), 0.45))));
  for (let zz = -6; zz > -len; zz -= 22) scene.add(bunting(new THREE.Vector3(-4.4, 6.2, zz), new THREE.Vector3(4.4, 6.2, zz - 2)));
  [[-4.6, -3], [4.7, -9], [-4.8, -20], [4.6, -27]].forEach(([x, z], i) => { const o = i % 2 ? barrel() : crate(); o.position.set(x, 0.18, z); scene.add(o); });
  const carsList: THREE.Group[] = [];
  if (cars) ([[2, -14, '#2C3A4A'], [-2, -30, '#7A2E2E'], [2, -44, '#2E5A44']] as Array<[number, number, string]>).forEach(([x, z, c]) => { const v = car(c); v.position.set(x, 0, z); if (x < 0) v.rotation.y = Math.PI; scene.add(v); carsList.push(v); });
  const crowd = people ? extras(scene, [[-5.3, -6], [5.0, -10, 2], [-5.0, -15], [5.4, -18, 4], [-5.4, -24, 3], [4.9, -31, 5], [-5.0, -36, 6], [5.3, -42, 7], [-5.2, -48, 1]]) : [];
  if (dusk) crowd.forEach((p) => p.tint('#D9B9C9'));
  return { scene, sun, people: crowd, cars: carsList, buildings, faceAll(cam: THREE.Camera) { crowd.forEach((p) => p.face(cam)); } };
}

/** interior() preset defaults (v5): a preset supplies values ONLY for fields the caller left
 *  unset — it changes no geometry code path. 'shop' is exactly the v4 defaults, so a call with
 *  no preset (or preset:'shop' and the same explicit fields) renders byte-identically to v4. */
export const INTERIOR_PRESETS: Record<'shop' | 'home' | 'office' | 'classroom', { wall: string; motif: string; floor: string; signText: string | null; counterOn: boolean; picture: boolean }> = {
  shop: { wall: '#9E2E3A', motif: '#B8414C', floor: '#A8693E', signText: 'NO CREDIT · CASH ONLY', counterOn: true, picture: true },
  home: { wall: '#7FA36A', motif: '#93B584', floor: '#8A5A36', signText: null, counterOn: false, picture: true },
  office: { wall: '#5F7A8A', motif: '#72909E', floor: '#A8693E', signText: 'OFFICE', counterOn: true, picture: false },
  classroom: { wall: '#C9A86A', motif: '#D8BC82', floor: '#A8693E', signText: null, counterOn: false, picture: true },
};

/** a shop / office interior. Back wall z = -depth, window in it; counter along the right side. */
export function interior(E: Engine, opts: InteriorOptions = {}): InteriorSet {
  const P = INTERIOR_PRESETS[opts.preset ?? 'shop'];
  const w = opts.w ?? 8, depth = opts.depth ?? 7, h = opts.h ?? 4.2, ceiling = opts.ceiling ?? '#24524A';
  const view = opts.view ?? null, bulbs = opts.bulbs ?? true, litter = opts.litter ?? true;
  const wall = opts.wall ?? P.wall, motif = opts.motif ?? P.motif, floor = opts.floor ?? P.floor;
  const signText = opts.signText !== undefined ? opts.signText : P.signText;
  const picture = opts.picture !== undefined ? opts.picture : P.picture;
  const counterOn = opts.counterOn !== undefined ? opts.counterOn : P.counterOn;
  const scene = new THREE.Scene(); scene.background = new THREE.Color('#1A1418');
  scene.environment = E.envMap; scene.environmentIntensity = 0.35;
  const fl = new THREE.Mesh(new THREE.PlaneGeometry(w, depth + 2), mat({ map: T.planks(floor, [w / 2.5, (depth + 2) / 2.5]) })); fl.rotation.x = -Math.PI / 2; fl.position.z = -depth / 2 + 1; fl.receiveShadow = true; scene.add(fl);
  (fl.material as THREE.MeshStandardMaterial).map!.rotation = Math.PI / 2;
  const wp = (len: number) => mat({ map: T.wallpaper(wall, motif, [len / 1.4, h / 1.4]) });
  const wain = (len: number) => mat({ map: T.planks('#6B3E26', [len / 2, 0.5]) });
  // back wall with a window hole: 4 boxes around the opening
  const ww = 3.2, wh = 1.9, wy = 1.1;
  const back = new THREE.Group(); back.position.z = -depth; scene.add(back);
  box((w - ww) / 2, h, 0.2, wp((w - ww) / 2), -(w + ww) / 4, h / 2, 0, back); box((w - ww) / 2, h, 0.2, wp((w - ww) / 2), (w + ww) / 4, h / 2, 0, back);
  box(ww, wy, 0.2, wain(ww), 0, wy / 2, 0, back); box(ww, h - wy - wh, 0.2, wp(ww), 0, h - (h - wy - wh) / 2, 0, back);
  const frameM = mat({ color: '#2F5C52' });
  box(ww + 0.2, 0.14, 0.3, frameM, 0, wy, 0.05, back); box(ww + 0.2, 0.14, 0.3, frameM, 0, wy + wh, 0.05, back); box(0.12, wh, 0.3, frameM, -ww / 2, wy + wh / 2, 0.05, back); box(0.12, wh, 0.3, frameM, ww / 2, wy + wh / 2, 0.05, back); box(0.06, wh, 0.1, frameM, 0, wy + wh / 2, 0.05, back);
  const viewPlane = new THREE.Mesh(new THREE.PlaneGeometry(ww * 2.6, wh * 2.6), new THREE.MeshBasicMaterial({ color: '#BFE4F7', toneMapped: false })); viewPlane.position.set(0, wy + wh / 2, -2.5); back.add(viewPlane);
  if (view) { viewPlane.material.map = view; viewPlane.material.color.set('#FFFFFF'); }
  [-1, 1].forEach((s) => { const sw = box(0.2, h, depth + 2, wp(depth + 2), s * w / 2, h / 2, -depth / 2 + 1, scene); (sw.material as THREE.MeshStandardMaterial).map!.repeat.set((depth + 2) / 1.4, h / 1.4); });
  [-1, 1].forEach((s) => box(0.24, 1.0, depth + 2, wain(depth + 2), s * (w / 2 - 0.02), 0.5, -depth / 2 + 1, scene));
  box((w - ww) / 2, 1.0, 0.24, wain(2), -(w + ww) / 4, 0.5, -depth + 0.02, scene); box((w - ww) / 2, 1.0, 0.24, wain(2), (w + ww) / 4, 0.5, -depth + 0.02, scene);
  const ceil = box(w, 0.2, depth + 2, mat({ map: T.wallpaper(ceiling, shade(ceiling, 1.25), [w / 1.2, (depth + 2) / 1.2]) }), 0, h + 0.1, -depth / 2 + 1, scene);
  box(w, 0.16, 0.3, mat({ color: '#E8DCC0' }), 0, h - 0.08, -depth + 0.1, scene);
  // counter on the right
  const counter = new THREE.Group(); counter.position.set(w / 2 - 1.3, 0, -depth / 2 + 0.6); if (counterOn) scene.add(counter);
  box(1.0, 1.1, depth - 1.5, mat({ map: T.planks('#7A3E24', [1, 2]) }), 0, 0.55, 0, counter);
  box(1.2, 0.08, depth - 1.3, mat({ map: T.felt('#1F5A45') }), 0, 1.14, 0, counter);
  box(0.1, 0.06, depth - 1.3, mat({ color: '#C9A13B', metalness: 0.6, roughness: 0.3 }), -0.62, 1.12, 0, counter);
  // picture frame + sign
  const pic = canvasTex('pic', 256, 200, (g) => { g.fillStyle = '#E9DCC0'; g.fillRect(0, 0, 256, 200); g.strokeStyle = '#3A2A22'; g.lineWidth = 6; g.beginPath(); g.moveTo(40, 140); g.quadraticCurveTo(90, 60, 140, 120); g.quadraticCurveTo(170, 80, 220, 140); g.stroke(); g.fillStyle = '#C9603F'; g.beginPath(); g.arc(190, 60, 22, 0, 7); g.fill(); });
  if (picture) { const pf = box(0.9, 0.7, 0.05, [mat({ color: '#C9A13B' }), mat({ color: '#C9A13B' }), mat({ color: '#C9A13B' }), mat({ color: '#C9A13B' }), mat({ map: pic }), mat({ color: '#C9A13B' })], -w / 2 + 0.13, 2.4, -depth / 2, scene); pf.rotation.y = Math.PI / 2; }
  if (signText) { const sb = signBoard(signText, { w: 2.2, h: 0.4, bg: '#3A2A2A', fg: '#F2E6CF' }); sb.position.set(w / 2 - 0.13, 2.9, -depth / 2 + 0.6); sb.rotation.y = -Math.PI / 2; scene.add(sb); }
  // light: warm sun through the window + bulbs
  const sun = new THREE.DirectionalLight('#FFE4B8', 2.4); sun.position.set(-2, 6, -depth - 6); sun.target.position.set(0.5, 0, -1); sun.castShadow = true;
  sun.shadow.mapSize.set(2048, 2048); Object.assign(sun.shadow.camera, { left: -8, right: 8, top: 8, bottom: -8, near: 0.5, far: 30 }); sun.shadow.bias = -0.0006; sun.shadow.camera.updateProjectionMatrix(); sun.shadow.radius = 4;
  scene.add(sun, sun.target); scene.add(new THREE.HemisphereLight('#FFE9D2', '#5A3A2A', 1.1));
  const fill = new THREE.PointLight('#FFD9A8', 6, 12, 1.4); fill.position.set(0, 3.2, -1); scene.add(fill);
  const bulbLights: THREE.PointLight[] = [];
  if (bulbs) [[-1.5, -2.2], [1.6, -4.5]].forEach(([x, z]) => {
    const cord = cyl(0.008, 0.008, 1.0, new THREE.MeshBasicMaterial({ color: '#C9A13B' }), x, h - 0.5, z, scene, 4);
    const b = new THREE.Mesh(new THREE.SphereGeometry(0.1, 12, 8), new THREE.MeshStandardMaterial({ color: '#FFF6DA', emissive: '#FFE7A0', emissiveIntensity: 2.2 })); b.position.set(x, h - 1.05, z); scene.add(b);
    const pl = new THREE.PointLight('#FFD9A0', 3, 6, 1.6); pl.position.set(x, h - 1.15, z); scene.add(pl); bulbLights.push(pl);
  });
  // floor litter (betting slips / paper) for life
  const slips = new THREE.InstancedMesh(new THREE.PlaneGeometry(0.16, 0.1), new THREE.MeshStandardMaterial({ color: '#F2E6CF', side: THREE.DoubleSide }), 140);
  const m4 = new THREE.Matrix4(), q = new THREE.Quaternion(), e = new THREE.Euler();
  for (let i = 0; i < 140; i++) { e.set(-Math.PI / 2, 0, rnd(i * 3) * 6.3); q.setFromEuler(e); m4.compose(new THREE.Vector3((rnd(i) - 0.5) * (w - 1), 0.005 + i * 0.00005, -rnd(i * 7) * depth + 0.8), q, new THREE.Vector3(1, 1, 1)); slips.setMatrixAt(i, m4); slips.setColorAt(i, new THREE.Color(['#F2E6CF', '#E9D7A5', '#F4C7B0', '#CFE3D0'][i % 4])); }
  slips.receiveShadow = true; if (litter) scene.add(slips);
  // v5: the rig now also exposes its point-light fills (env.ts AtmosphereRig.fills) so an
  // atmosphere pass can dim the bulbs at 'night'. Return-shape addition only — .key is as before.
  return { scene, sun: { key: sun, fills: [fill, ...bulbLights] }, counter, window: viewPlane, w, depth, h };
}

/** grass, hills, a fence, trees, flowers. sunset: true = pink/orange evening */
export function meadow(E: Engine, { sunset = false, flowers = true, fenceOn = true, treesOn = true }: MeadowOptions = {}): MeadowSet {
  const scene = new THREE.Scene();
  sky(scene, sunset ? { top: '#5B4FA8', horizon: '#FF9E6E' } : { top: '#3D8EE0', horizon: '#CBEAF8' });
  const sun = sunlight(scene, E, sunset ? { col: '#FFB27A', i: 2.4, pos: [-12, 4, -18], sky: '#C9A6E0', ground: '#6A4A3A', hemi: 1.0, box: 14 } : { pos: [8, 12, 6], box: 14 });
  clouds(scene, [[-30, 22, -90, 14], [25, 30, -100, 18], [60, 18, -80, 11], [-5, 16, -60, 8]], sunset ? '#FFB8A0' : '#FFFFFF');
  const geo = new THREE.PlaneGeometry(200, 200, 100, 100); const p = geo.attributes.position as THREE.BufferAttribute;
  for (let i = 0; i < p.count; i++) { const x = p.getX(i), y = p.getY(i); p.setZ(i, Math.sin(x * 0.08) * 0.6 + Math.cos(y * 0.06 + 1) * 0.8 + (Math.abs(x) < 6 && Math.abs(y) < 10 ? 0 : 0)); }
  geo.computeVertexNormals();
  const ground = new THREE.Mesh(geo, mat({ map: T.grass(sunset ? '#7DB55A' : '#6DBB4A', [50, 50]) })); ground.rotation.x = -Math.PI / 2; ground.position.y = -0.5; ground.receiveShadow = true; scene.add(ground);
  const flat = new THREE.Mesh(new THREE.CircleGeometry(12, 48), mat({ map: T.grass(sunset ? '#7DB55A' : '#6DBB4A', [6, 6]) })); flat.rotation.x = -Math.PI / 2; flat.receiveShadow = true; scene.add(flat);
  hills(scene, { z: -70, col: sunset ? '#86B860' : '#79C25A' });
  (treesOn ? ([[-7, -9, 'round'], [8, -14, 'round'], [-14, -20, 'cypress'], [-11, -22, 'cypress'], [14, -26, 'cypress'], [17, -25, 'cypress']] as Array<[number, number, string]>) : ([] as Array<[number, number, string]>)).forEach(([x, z, k], i) => { const t = tree(1.1 + rnd(i) * 0.4, '#4E9A3C', k); t.position.set(x, 0, z); scene.add(t); });
  if (fenceOn) { const f = fence(16); f.position.set(-8, 0, -6); scene.add(f); }
  if (flowers) {
    const fl = new THREE.InstancedMesh(new THREE.IcosahedronGeometry(0.05, 0), new THREE.MeshStandardMaterial({ roughness: 0.6 }), 1600), m4 = new THREE.Matrix4();
    for (let i = 0; i < 1600; i++) { const a = rnd(i) * 6.28, r = 2 + rnd(i * 3.3) * 22; m4.makeTranslation(Math.cos(a) * r, 0.1, Math.sin(a) * r - 4); fl.setMatrixAt(i, m4); fl.setColorAt(i, new THREE.Color(['#F2D23A', '#F29BB5', '#FFFFFF', '#B98ACF', '#F28C4A'][i % 5])); }
    scene.add(fl);
    const tufts = new THREE.InstancedMesh(new THREE.ConeGeometry(0.025, 0.16, 3), mat({ color: sunset ? '#6FA650' : '#5DA83E' }), 4000);
    for (let i = 0; i < 4000; i++) { const a = rnd(i * 1.7) * 6.28, r = 1 + rnd(i * 5.1) * 14; m4.makeTranslation(Math.cos(a) * r, 0.07, Math.sin(a) * r - 3); tufts.setMatrixAt(i, m4); }
    scene.add(tufts);
  }
  return { scene, sun };
}

/** render a set into a texture once (for windows, picture-in-picture, spyglass views) */
export function snapshot(E: Engine, scene: THREE.Scene, cam: THREE.Camera, w = 1024, h = 768): THREE.Texture {
  const rt = new THREE.WebGLRenderTarget(w, h, { samples: 4, type: THREE.HalfFloatType });
  const r = E.renderer; const prev = r.getRenderTarget(); r.setRenderTarget(rt); r.shadowMap.needsUpdate = true; r.render(scene, cam); r.setRenderTarget(prev);
  return rt.texture;
}

// ---------- v5 pieces (additive) ----------
/** a palm: trunk of stacked segments leaning as it climbs, painted-frond planes + coconuts at the crown */
export function palm(s = 1, lean = 0.18): THREE.Group {
  const g = new THREE.Group();
  const bark = mat({ color: '#8A6A4A', map: T.planks('#8A6A4A', [0.4, 1]) });
  const segH = 0.62 * s;
  let px = 0, py = 0;
  for (let i = 0; i < 5; i++) {
    const ang = lean * (i / 4);
    const seg = cyl(0.085 * s, 0.11 * s, segH + 0.05, bark, px + Math.sin(ang) * segH / 2, py + Math.cos(ang) * segH / 2, 0, g, 8);
    seg.rotation.z = -ang;
    px += Math.sin(ang) * segH; py += Math.cos(ang) * segH;
  }
  const frondT = canvasTex('palmfrond', 256, 128, (c) => {
    c.clearRect(0, 0, 256, 128); c.fillStyle = '#4E8A3C';
    c.beginPath(); c.moveTo(6, 64); c.quadraticCurveTo(110, 18, 250, 58); c.quadraticCurveTo(120, 84, 6, 64); c.fill();
    c.strokeStyle = '#3A6B2E'; c.lineWidth = 5; c.beginPath(); c.moveTo(8, 64); c.quadraticCurveTo(120, 40, 248, 58); c.stroke();
    c.lineWidth = 3; for (let i = 0; i < 7; i++) { const x = 40 + i * 28; c.beginPath(); c.moveTo(x, 56); c.lineTo(x + 12, 76); c.stroke(); }
  });
  for (let i = 0; i < 6; i++) {
    const fg = new THREE.PlaneGeometry(2.3 * s, 0.72 * s); fg.translate(1.15 * s, 0, 0);
    const f = new THREE.Mesh(fg, mat({ map: frondT, side: THREE.DoubleSide, transparent: true, alphaTest: 0.35 }));
    f.position.set(px, py, 0); f.rotation.order = 'YZX'; f.rotation.y = i / 6 * Math.PI * 2 + rnd(i * 3) * 0.5; f.rotation.z = -0.42 - rnd(i * 7) * 0.25; f.castShadow = true; g.add(f);
  }
  [[0.13, -0.1], [-0.14, 0.08]].forEach(([dx, dz]) => { const nut = new THREE.Mesh(new THREE.SphereGeometry(0.11 * s, 10, 8), mat({ color: '#6B4A2E' })); nut.position.set(px + dx * s, py - 0.12 * s, dz * s); nut.castShadow = true; g.add(nut); });
  return g;
}
/** a cartoon saguaro: column + cap + two arms (one low, one high) */
export function cactus(s = 1, col = '#4E7A3A'): THREE.Group {
  const g = new THREE.Group(), m = mat({ color: col, roughness: 0.7 });
  cyl(0.26 * s, 0.3 * s, 2.2 * s, m, 0, 1.1 * s, 0, g, 12);
  const cap = new THREE.Mesh(new THREE.SphereGeometry(0.26 * s, 12, 8), m); cap.position.y = 2.2 * s; cap.castShadow = true; g.add(cap);
  ([[-1, 0.95, 0.55], [1, 1.35, 0.7]] as Array<[number, number, number]>).forEach(([sd, ay, ah]) => {
    const out = cyl(0.09 * s, 0.09 * s, 0.42 * s, m, sd * 0.42 * s, ay * s, 0, g, 8); out.rotation.z = Math.PI / 2;
    cyl(0.09 * s, 0.1 * s, ah * s, m, sd * 0.63 * s, (ay + ah / 2) * s, 0, g, 8);
    const tip = new THREE.Mesh(new THREE.SphereGeometry(0.09 * s, 10, 8), m); tip.position.set(sd * 0.63 * s, (ay + ah) * s, 0); tip.castShadow = true; g.add(tip);
  });
  return g;
}
/** a backdrop mountain: grey cone + optional snow cap. Backdrop only — never a stage. */
export function peak(w = 18, h = 14, snow = true): THREE.Group {
  const g = new THREE.Group();
  const rock = new THREE.Mesh(new THREE.ConeGeometry(w / 2, h, 5), mat({ color: '#7A7D84', roughness: 1 }));
  rock.position.y = h / 2; rock.rotation.y = 0.4; g.add(rock);
  if (snow) { const cap = new THREE.Mesh(new THREE.ConeGeometry(w * 0.2, h * 0.4, 5), mat({ color: '#F4F8FC', roughness: 0.9 })); cap.position.y = h * 0.8; cap.rotation.y = 0.4; g.add(cap); }
  return g;
}
/** dome stars for spaceLite / night skies: points on the upper hemisphere, fog-free */
export function stars(n = 400, r = 360): THREE.Points {
  const pos = new Float32Array(n * 3);
  for (let i = 0; i < n; i++) {
    const y = 0.06 + rnd(i * 3.7) * 0.94, th = rnd(i * 7.3) * Math.PI * 2, rxz = Math.sqrt(Math.max(0, 1 - y * y));
    pos[i * 3] = Math.cos(th) * rxz * r; pos[i * 3 + 1] = y * r; pos[i * 3 + 2] = Math.sin(th) * rxz * r;
  }
  const geo = new THREE.BufferGeometry(); geo.setAttribute('position', new THREE.BufferAttribute(pos, 3));
  const pts = new THREE.Points(geo, new THREE.PointsMaterial({ color: '#FFFFFF', size: 1.6, sizeAttenuation: false, fog: false, toneMapped: false, transparent: true, opacity: 0.9, depthWrite: false }));
  return pts;
}
/** the openSky cloud deck: a soft painted cloud plane whose alpha fades at the edges. receiveShadow is FALSE on purpose — nothing casts from 55 m up in the shadow box. */
export function cloudDeck({ size = 500, col = '#FFFFFF' }: { size?: number; col?: string } = {}): THREE.Group {
  const g = new THREE.Group();
  const t = canvasTex('clouddeck', 1024, 1024, (c, w, h) => {
    c.clearRect(0, 0, w, h);
    for (let i = 0; i < 240; i++) {
      const x = rnd(i * 1.3) * w, y = rnd(i * 2.9) * h, r = 26 + rnd(i * 5.1) * 74;
      const gr = c.createRadialGradient(x, y, 0, x, y, r);
      gr.addColorStop(0, 'rgba(255,255,255,0.55)'); gr.addColorStop(1, 'rgba(255,255,255,0)');
      c.fillStyle = gr; c.beginPath(); c.arc(x, y, r, 0, 7); c.fill();
    }
    c.globalCompositeOperation = 'destination-out';
    const eg = c.createRadialGradient(w / 2, h / 2, w * 0.18, w / 2, h / 2, w * 0.52);
    eg.addColorStop(0, 'rgba(0,0,0,0)'); eg.addColorStop(1, 'rgba(0,0,0,1)');
    c.fillStyle = eg; c.fillRect(0, 0, w, h);
    c.globalCompositeOperation = 'source-over';
  });
  const m = new THREE.Mesh(new THREE.PlaneGeometry(size, size), new THREE.MeshStandardMaterial({ map: t, transparent: true, roughness: 1, color: col, depthWrite: false }));
  m.rotation.x = -Math.PI / 2; m.receiveShadow = false; g.add(m);
  return g;
}
/** foreground framing: a dark leaf-cluster arc (~2.2 m across) the SHOT parks just off a frame edge for depth (the engine's DoF blurs it). Origin at the cluster's base centre. */
export function fgLeaves(col = '#1E3320'): THREE.Group {
  const g = new THREE.Group();
  const br = cyl(0.03, 0.045, 2.3, mat({ color: '#3A2A20' }), 0, 0.55, -0.1, g, 6); br.rotation.z = Math.PI / 2 - 0.18;
  ([[-1.0, 0.75, 0.5], [-0.55, 0.95, 0.62], [0, 1.05, 0.7], [0.55, 0.95, 0.62], [1.0, 0.7, 0.52]] as Array<[number, number, number]>).forEach(([x, y, r], i) => {
    const m = new THREE.Mesh(new THREE.IcosahedronGeometry(r, 1), mat({ color: shade(col, 0.85 + rnd(i) * 0.3) }));
    m.scale.set(1, 0.62, 0.55); m.position.set(x, y, 0); g.add(m);
  });
  return g;
}

// ---------- v5 SETS (additive) — every outdoor set returns its stage (C-4): a known-flat disc at the origin ----------
export interface OpenSkyOptions { deckY?: number; deckOn?: boolean; cloudCol?: string; sunAlt?: number }
export interface OpenSkySet { scene: THREE.Scene; sun: LightRig; deck: THREE.Group | null; cloudsNear: THREE.Sprite[] }
/** THE ALTITUDE SET: sky dome + sun + a cloud deck far below + free-floating cloud sprites. There is NO ground and NO stage — action is staged around the origin and the WORLD moves (see the falling staging contract in library/worlds.md). */
export function openSky(E: Engine, { deckY = -55, deckOn = true, cloudCol = '#FFFFFF', sunAlt }: OpenSkyOptions = {}): OpenSkySet {
  const scene = new THREE.Scene();
  sky(scene, {});
  const sun = sunlight(scene, E, { pos: [6, sunAlt ?? 10, 6], box: 8 });
  let deck: THREE.Group | null = null;
  if (deckOn) { deck = cloudDeck({ col: cloudCol }); deck.position.y = deckY; scene.add(deck); }
  const list: Array<readonly [number, number, number, number]> = [];
  for (let i = 0; i < 14; i++) { const a = rnd(i * 3.1) * Math.PI * 2, r = 16 + rnd(i * 5.7) * 24, y = (rnd(i * 7.9) - 0.5) * 26; list.push([Math.cos(a) * r, y, Math.sin(a) * r, 5 + rnd(i * 11.3) * 7] as const); }
  const cloudsNear = clouds(scene, list, cloudCol);
  return { scene, sun, deck, cloudsNear };
}

export interface FarmOptions { sunset?: boolean; crops?: boolean; barnCol?: string; houseOn?: boolean }
export interface FarmSet { scene: THREE.Scene; sun: LightRig; stage: { y: number; r: number }; barn: THREE.Group; house: HouseGroup | null; field: THREE.Group }
/** a farmyard: dirt-yard stage at the origin, barn + farmhouse, fences, crop rows behind */
export function farm(E: Engine, { sunset = false, crops = true, barnCol, houseOn = true }: FarmOptions = {}): FarmSet {
  const scene = new THREE.Scene();
  sky(scene, sunset ? { top: '#5B4FA8', horizon: '#FF9E6E' } : {});
  const sun = sunlight(scene, E, sunset ? { col: '#FFB27A', i: 2.4, pos: [-12, 4, -18], sky: '#C9A6E0', ground: '#6A4A3A', hemi: 1.0, box: 14 } : { pos: [8, 12, 6], box: 14 });
  clouds(scene, [[-28, 22, -85, 13], [24, 28, -95, 17], [55, 18, -75, 10]], sunset ? '#FFB8A0' : '#FFFFFF');
  const ground = new THREE.Mesh(new THREE.PlaneGeometry(220, 220), mat({ map: T.grass(sunset ? '#7DB55A' : '#6DBB4A', [55, 55]) }));
  ground.rotation.x = -Math.PI / 2; ground.position.y = -0.02; ground.receiveShadow = true; scene.add(ground);
  const yard = new THREE.Mesh(new THREE.CircleGeometry(9, 40), mat({ map: T.dirt('#A9805A', [4, 4], false) }));
  yard.rotation.x = -Math.PI / 2; yard.position.y = 0.005; yard.receiveShadow = true; scene.add(yard);
  const b = barn(barnCol ? { col: barnCol } : {}); b.position.set(-7.5, 0, -8); b.rotation.y = 0.55; scene.add(b);
  let hs: HouseGroup | null = null;
  if (houseOn) { hs = house(); hs.position.set(8.5, 0, -10.5); hs.rotation.y = -0.45; scene.add(hs); }
  const f1 = fence(12, '#E8DFC8'); f1.position.set(-8.5, 0, 3.5); f1.rotation.y = 0.25; scene.add(f1);
  const f2 = fence(9, '#E8DFC8'); f2.position.set(2.5, 0, 5.5); f2.rotation.y = -0.15; scene.add(f2);
  ([[-13, -16], [13, -20], [-18, -6]] as Array<[number, number]>).forEach(([x, z], i) => { const t = tree(1.2 + rnd(i) * 0.4); t.position.set(x, 0, z); scene.add(t); });
  const field = new THREE.Group(); scene.add(field);
  if (crops) {
    const soil = new THREE.Mesh(new THREE.PlaneGeometry(26, 9), mat({ map: T.dirt('#8A5A36', [8, 3], false) }));
    soil.rotation.x = -Math.PI / 2; soil.position.set(0, 0.005, -19); soil.receiveShadow = true; field.add(soil);
    const cones = new THREE.InstancedMesh(new THREE.ConeGeometry(0.16, 0.5, 6), mat({ color: '#5DA83E' }), 120);
    const m4 = new THREE.Matrix4(); let n = 0;
    for (let row = 0; row < 5; row++) for (let k = 0; k < 24; k++) { m4.makeTranslation(-11.5 + k + (row % 2) * 0.5, 0.25, -22.6 + row * 1.8); cones.setMatrixAt(n++, m4); }
    cones.count = n; field.add(cones);
  }
  return { scene, sun, stage: { y: 0, r: 9 }, barn: b, house: hs, field };
}

export interface DesertOptions { oasis?: boolean; bones?: boolean; sunset?: boolean }
export interface DesertSet { scene: THREE.Scene; sun: LightRig; stage: { y: number; r: number } }
/** open desert: sand stage, dunes behind, cacti + rocks; optional oasis and a cartoon ribcage (dressing, no gore) */
export function desert(E: Engine, { oasis = false, bones = false, sunset = false }: DesertOptions = {}): DesertSet {
  const scene = new THREE.Scene();
  sky(scene, sunset ? { top: '#5B4FA8', horizon: '#FF9E6E' } : { top: '#3D8EE0', horizon: '#F2DFB8' });
  const sun = sunlight(scene, E, sunset ? { col: '#FFB27A', i: 2.4, pos: [-12, 4, -18], sky: '#C9A6E0', ground: '#8A5A3A', hemi: 1.0, box: 14 } : { col: '#FFF3D0', i: 3.0, pos: [10, 13, 5], ground: '#C89A62', hemi: 1.1, box: 14 });
  clouds(scene, [[-30, 24, -90, 12], [30, 30, -100, 15]], sunset ? '#FFB8A0' : '#FFFFFF');
  const ground = new THREE.Mesh(new THREE.PlaneGeometry(240, 240), mat({ map: T.sand('#E3C078', [60, 60]) }));
  ground.rotation.x = -Math.PI / 2; ground.position.y = -0.02; ground.receiveShadow = true; scene.add(ground);
  // dunes: hills()'s flattened-sphere idiom with the sand map (hills() itself stays grass-only, additive-only discipline)
  ([[-38, -55, 30, 7], [8, -70, 42, 10], [52, -50, 28, 6], [-80, -40, 34, 8]] as Array<[number, number, number, number]>).forEach(([x, z, r, h], i) => {
    const m = new THREE.Mesh(new THREE.SphereGeometry(1, 24, 12), mat({ color: shade('#E0B26A', 0.92 + rnd(i) * 0.14), map: T.sand('#E0B26A', [6, 3]) }));
    m.scale.set(r, h, r * 0.6); m.position.set(x, -h * 0.25, z); m.receiveShadow = true; scene.add(m);
  });
  ([[-5, -7, 1.1], [6.5, -12, 0.9], [-11, -16, 1.3]] as Array<[number, number, number]>).forEach(([x, z, s], i) => { const c = cactus(s); c.position.set(x, 0, z); c.rotation.y = rnd(i * 7) * 3; scene.add(c); });
  ([[3, -4, 0.5], [-7.5, -3, 0.35], [9, -6, 0.65]] as Array<[number, number, number]>).forEach(([x, z, s], i) => { const r = new THREE.Mesh(new THREE.DodecahedronGeometry(s, 0), mat({ color: shade('#8A8578', 0.9 + rnd(i) * 0.2), roughness: 1 })); r.position.set(x, s * 0.4, z); r.castShadow = r.receiveShadow = true; scene.add(r); });
  if (oasis) {
    const pond = new THREE.Mesh(new THREE.CircleGeometry(3.4, 28), mat({ color: '#3E7CA6', roughness: 0.15 }));
    pond.rotation.x = -Math.PI / 2; pond.position.set(8, 0.01, -8); pond.receiveShadow = true; scene.add(pond);
    ([[5.2, -5.5, 1.15], [11, -10.5, 0.95]] as Array<[number, number, number]>).forEach(([x, z, s]) => { const p = palm(s); p.position.set(x, 0, z); scene.add(p); });
  }
  if (bones) for (let i = 0; i < 3; i++) { const rib = new THREE.Mesh(new THREE.TorusGeometry(0.85 - i * 0.14, 0.085, 8, 14, Math.PI), mat({ color: '#EFE6D2', roughness: 0.9 })); rib.position.set(-4.2 + i * 0.55, 0.06, -5.5); rib.rotation.y = 0.5; rib.castShadow = true; scene.add(rib); }
  return { scene, sun, stage: { y: 0, r: 10 } };
}

export interface SnowfieldOptions { blizzard?: boolean; cabin?: boolean; pond?: boolean }
export interface SnowfieldSet { scene: THREE.Scene; sun: LightRig; stage: { y: number; r: number } }
/** a snowfield: white stage, drifts, pines; optional snow-capped cabin and a frozen pond. blizzard only retints the clouds — pair with atmosphere 'snow'/'fog' for the weather itself */
export function snowfield(E: Engine, { blizzard = false, cabin = false, pond = false }: SnowfieldOptions = {}): SnowfieldSet {
  const scene = new THREE.Scene();
  sky(scene, { top: '#7FA8D8', horizon: '#EAF2FA' });
  const sun = sunlight(scene, E, { col: '#FFF4E0', i: 2.4, pos: [5, 8, 4], sky: '#DCE9F7', ground: '#B9C4CE', hemi: 1.35, box: 14 });
  clouds(scene, [[-26, 20, -80, 12], [22, 26, -90, 16], [50, 16, -70, 9]], blizzard ? '#EDF2F7' : '#FFFFFF');
  const ground = new THREE.Mesh(new THREE.PlaneGeometry(220, 220), mat({ map: T.snow('#F4F8FC', [55, 55]) }));
  ground.rotation.x = -Math.PI / 2; ground.position.y = -0.02; ground.receiveShadow = true; scene.add(ground);
  ([[-14, -18, 9, 1.6], [16, -22, 11, 2.0], [-24, -8, 7, 1.2], [26, -10, 8, 1.4]] as Array<[number, number, number, number]>).forEach(([x, z, r, h]) => {
    const m = new THREE.Mesh(new THREE.SphereGeometry(1, 24, 12), mat({ color: '#F4F8FC', roughness: 0.95 }));
    m.scale.set(r, h, r * 0.6); m.position.set(x, -h * 0.3, z); m.receiveShadow = true; scene.add(m);
  });
  ([[-8, -10, 1.2], [9, -13, 1.4], [-15, -20, 1.1], [16, -24, 1.25], [-4, -26, 1.35], [5, -30, 1.15]] as Array<[number, number, number]>).forEach(([x, z, s], i) => { const t = tree(s, i % 2 ? '#4A7A5C' : '#3E6B4F', 'pine'); t.position.set(x, 0, z); scene.add(t); });
  if (cabin) {
    const cb = house({ roof: '#7A4A3A' }); cb.position.set(7.5, 0, -9.5); cb.rotation.y = -0.4; scene.add(cb);
    [-1, 1].forEach((sd) => { const cap = box(3.9, 0.09, 5.3, mat({ color: '#F4F8FC' }), sd * 1.65, 4.36, 0, cb); cap.rotation.z = -sd * Math.atan2(2.1, 3.6); });
  }
  if (pond) { const p = new THREE.Mesh(new THREE.CircleGeometry(3, 28), mat({ color: '#CFE3EE', roughness: 0.2 })); p.rotation.x = -Math.PI / 2; p.position.set(-6, 0.01, -6); p.receiveShadow = true; scene.add(p); }
  return { scene, sun, stage: { y: 0, r: 10 } };
}

export interface BeachOptions { sunset?: boolean; palms?: boolean; boatOn?: boolean }
export interface BeachSet { scene: THREE.Scene; sun: LightRig; stage: { y: number; r: number }; sea: THREE.Group; foam: THREE.Group[] }
/** a beach: dry-sand stage (y 0) sloping to a waterline at z −4; the sea plane sits at y −0.25. The foam strips are RETURNED so the shot can slide them ±0.4 m on a word — the set itself never animates */
export function beach(E: Engine, { sunset = false, palms: palmsOn = true, boatOn = true }: BeachOptions = {}): BeachSet {
  const scene = new THREE.Scene();
  sky(scene, sunset ? { top: '#5B4FA8', horizon: '#FF9E6E' } : {});
  const sun = sunlight(scene, E, sunset ? { col: '#FFB27A', i: 2.4, pos: [-12, 4, -18], sky: '#C9A6E0', ground: '#8A5A3A', hemi: 1.0, box: 14 } : { pos: [8, 12, 6], box: 14 });
  clouds(scene, [[-28, 22, -85, 13], [24, 28, -95, 17]], sunset ? '#FFB8A0' : '#FFFFFF');
  const dry = new THREE.Mesh(new THREE.PlaneGeometry(240, 64), mat({ map: T.sand('#E3C078', [48, 14]) }));
  dry.rotation.x = -Math.PI / 2; dry.position.set(0, 0, 26); dry.receiveShadow = true; scene.add(dry);
  const sea = new THREE.Group(); scene.add(sea);
  const water = new THREE.Mesh(new THREE.PlaneGeometry(240, 170), mat({ color: '#2E7FA8', roughness: 0.25 }));
  water.rotation.x = -Math.PI / 2; water.position.set(0, -0.25, -89); sea.add(water);
  const foamT = canvasTex('foam', 512, 64, (g, w, h) => {
    g.clearRect(0, 0, w, h); g.fillStyle = 'rgba(255,255,255,0.95)';
    g.fillRect(0, h * 0.5, w, h * 0.5);
    for (let x = 0; x < w + 40; x += 36) { g.beginPath(); g.arc(x, h * 0.52, 20, 0, 7); g.fill(); }
    g.globalCompositeOperation = 'destination-out';
    const gr = g.createLinearGradient(0, 0, 0, h * 0.4); gr.addColorStop(0, 'rgba(0,0,0,0.85)'); gr.addColorStop(1, 'rgba(0,0,0,0)');
    g.fillStyle = gr; g.fillRect(0, 0, w, h * 0.4); g.globalCompositeOperation = 'source-over';
  });
  const foam: THREE.Group[] = [];
  [-3.4, -4.6, -6.0].forEach((z, i) => {
    const fg = new THREE.Group();
    const f = new THREE.Mesh(new THREE.PlaneGeometry(46 - i * 6, 1.1), new THREE.MeshBasicMaterial({ map: foamT, transparent: true, depthWrite: false, toneMapped: false, opacity: 0.95 - i * 0.18 }));
    f.rotation.x = -Math.PI / 2; fg.add(f); fg.position.set((i - 1) * 6, -0.16 - i * 0.03, z); scene.add(fg); foam.push(fg);
  });
  if (boatOn) { const bt = boat(); bt.position.set(4.5, -0.25, -8); bt.rotation.y = 0.4; sea.add(bt); }
  if (palmsOn) ([[-6.5, 2.5, 1.2], [7.5, 0.5, 1.0], [-10, -2, 0.9]] as Array<[number, number, number]>).forEach(([x, z, s]) => { const p = palm(s); p.position.set(x, 0, z); scene.add(p); });
  return { scene, sun, stage: { y: 0, r: 8 }, sea, foam };
}

export interface CityStreetOptions { len?: number; night?: boolean; blocks?: number }
export interface CityStreetSet { scene: THREE.Scene; sun: LightRig; stage: { y: number; r: number }; buildings: BuildingInfo[] }
/** mainStreet's grammar at modern scale: tall facade() blocks (h 12–20) set back at x ±11.5, lamps, parked cars. Default cam 8 m / fov 46 — the wider fov keeps the tops in frame */
export function cityStreet(E: Engine, { len = 70, night = false, blocks }: CityStreetOptions = {}): CityStreetSet {
  const buildings: BuildingInfo[] = [];
  const scene = new THREE.Scene();
  sky(scene, night ? { top: '#0E1838', horizon: '#3A4F6B', ground: '#232F45' } : {});
  const sun = sunlight(scene, E, night ? { col: '#9AB8FF', i: 0.55, pos: [-5, 10, 4], sky: '#4A5F8A', ground: '#1E2430', hemi: 0.25, box: 18 } : { pos: [9, 13, 5], box: 18 });
  clouds(scene, [[-30, 30, -100, 15], [25, 38, -110, 18]], night ? '#6A7F9E' : '#FFFFFF');
  const base = new THREE.Mesh(new THREE.PlaneGeometry(300, 300), mat({ color: '#5A5E58' }));
  base.rotation.x = -Math.PI / 2; base.position.y = -0.03; base.receiveShadow = true; scene.add(base);
  const road = new THREE.Mesh(new THREE.PlaneGeometry(10, len + 40), mat({ map: T.dirt('#4A4A52', [2, (len + 40) / 8], false) }));
  road.rotation.x = -Math.PI / 2; road.position.set(0, 0, -len / 2 + 10); road.receiveShadow = true; scene.add(road);
  const NAMES = ['HOTEL', 'BANK', 'CAFE', 'STORE', 'LOFTS', 'GYM'];
  [-1, 1].forEach((s) => {
    box(2.4, 0.16, len + 20, mat({ color: '#9A9A98' }), s * 6.2, 0.08, -len / 2 + 5, scene);
    let z = 8, k = s > 0 ? 0 : 3, n = 0;
    while (z > -len && (blocks === undefined || n < blocks)) {
      const w = 7 + rnd(k * 3) * 2, h = 12 + rnd(k * 7) * 8, name = NAMES[k % NAMES.length]; k++; n++;
      const wall = ['#B5553C', '#6FA3B8', '#C7B79A', '#9E4A3A'][k % 4];
      const front = mat({ map: facade(name, { wall, kind: 'brick', w, h, seed: k }) }), side = mat({ color: shade(wall, 0.85) });
      const b = new THREE.Mesh(new THREE.BoxGeometry(7, h, w), s < 0 ? [front, side, side, side, side, side] : [side, front, side, side, side, side]);
      b.position.set(s * 11.5, h / 2, z - w / 2); b.castShadow = b.receiveShadow = true; scene.add(b);
      buildings.push({ name, side: s, z: z - w / 2, w, h, x: s * 8.3 });
      z -= w + 0.3;
    }
    for (let zz = 2; zz > -len; zz -= 11) { const l = lamp(); l.position.set(s * 5.6, 0.16, zz); scene.add(l); }
  });
  const v1 = car('#7A2E2E'); v1.position.set(-2.2, 0, -16); v1.rotation.y = Math.PI; scene.add(v1);
  const v2 = car('#2E5A44'); v2.position.set(2.4, 0, -34); scene.add(v2);
  return { scene, sun, stage: { y: 0, r: 8 }, buildings };
}

export interface MountainOptions { sunset?: boolean; snowline?: boolean; cabin?: boolean }
export interface MountainSet { scene: THREE.Scene; sun: LightRig; stage: { y: number; r: number } }
/** a mountain valley: displaced rocky ground flattened at the origin stage, peak() backdrop, slope pines planted at the true displaced height */
export function mountain(E: Engine, { sunset = false, snowline = true, cabin = false }: MountainOptions = {}): MountainSet {
  const scene = new THREE.Scene();
  sky(scene, sunset ? { top: '#5B4FA8', horizon: '#FF9E6E' } : {});
  const sun = sunlight(scene, E, sunset ? { col: '#FFB27A', i: 2.4, pos: [-12, 4, -18], sky: '#C9A6E0', ground: '#6A4A3A', hemi: 1.0, box: 16 } : { pos: [8, 12, 6], box: 16 });
  clouds(scene, [[-30, 24, -90, 13], [26, 30, -100, 16]], sunset ? '#FFB8A0' : '#FFFFFF');
  const gy = (x: number, z: number) => { const d = Math.hypot(x, z); const k = THREE.MathUtils.smoothstep(d, 11, 42); return (Math.sin(x * 0.07) * 1.1 + Math.cos((-z) * 0.055 + 1) * 1.3) * k; };
  const geo = new THREE.PlaneGeometry(220, 220, 90, 90); const p = geo.attributes.position as THREE.BufferAttribute;
  for (let i = 0; i < p.count; i++) { const x = p.getX(i), y = p.getY(i); p.setZ(i, gy(x, -y)); }
  geo.computeVertexNormals();
  const ground = new THREE.Mesh(geo, mat({ map: T.dirt('#8A7A5F', [46, 46], false) }));
  ground.rotation.x = -Math.PI / 2; ground.receiveShadow = true; scene.add(ground);
  const flat = new THREE.Mesh(new THREE.CircleGeometry(11, 40), mat({ map: T.grass('#6FA650', [6, 6]) }));
  flat.rotation.x = -Math.PI / 2; flat.position.y = 0.005; flat.receiveShadow = true; scene.add(flat);
  ([[-48, -78, 44, 30], [14, -92, 56, 38], [64, -70, 38, 24], [-14, -105, 48, 32]] as Array<[number, number, number, number]>).forEach(([x, z, w, h]) => { const pk = peak(w, h, snowline); pk.position.set(x, -3, z); scene.add(pk); });
  ([[-9, -13, 1.2], [11, -17, 1.35], [-16, -25, 1.15], [18, -29, 1.25], [-6, -35, 1.4]] as Array<[number, number, number]>).forEach(([x, z, s]) => { const t = tree(s, '#3E6B4F', 'pine'); t.position.set(x, gy(x, z), z); scene.add(t); });
  if (cabin) { const cb = house({ wall: '#C9B48A' }); cb.scale.setScalar(0.85); cb.position.set(6.5, 0, -6.5); cb.rotation.y = -0.5; scene.add(cb); }
  return { scene, sun, stage: { y: 0, r: 10 } };
}

export interface FieldCampOptions { dawn?: boolean; tents?: number; tower?: boolean }
export interface FieldCampSet { scene: THREE.Scene; sun: LightRig; stage: { y: number; r: number } }
/** the serious-tone camp set (Onoda-class stories): tents, sandbags, crates, a watchtower, mud. HARD content rule: NO weapons as set dressing, NO gore, NO bodies — conflict is told by people and words, never by props */
export function fieldCamp(E: Engine, { dawn = true, tents: tentsN = 3, tower = true }: FieldCampOptions = {}): FieldCampSet {
  const scene = new THREE.Scene();
  sky(scene, dawn ? { top: '#4A5AA8', horizon: '#F9C0A8' } : {});
  const sun = sunlight(scene, E, dawn ? { col: '#FFB49A', i: 1.9, pos: [-9, 3.5, 4], sky: '#C9B8E8', ground: '#8A5A48', hemi: 0.85, box: 14 } : { pos: [8, 12, 6], box: 14 });
  clouds(scene, [[-26, 20, -80, 12], [24, 26, -90, 15]], dawn ? '#FFD9C8' : '#FFFFFF');
  const ground = new THREE.Mesh(new THREE.PlaneGeometry(200, 200), mat({ map: T.dirt('#8A6A4A', [44, 44], true) }));
  ground.rotation.x = -Math.PI / 2; ground.position.y = -0.02; ground.receiveShadow = true; scene.add(ground);
  const spots: Array<[number, number, number]> = [[-6.5, -6, 0.5], [6.5, -9, -0.5], [-6.5, -12.5, 0.5], [6.5, -15, -0.5], [-6.5, -19, 0.5]];
  spots.slice(0, Math.max(0, Math.min(tentsN, spots.length))).forEach(([x, z, ry], i) => { const t = tent(i % 2 ? { col: '#8A8558' } : {}); t.position.set(x, 0, z); t.rotation.y = ry; scene.add(t); });
  ([[-3.6, 4.2, 0.25], [3.6, 4.2, -0.25]] as Array<[number, number, number]>).forEach(([x, z, ry]) => { const sb = sandbags(14); sb.position.set(x, 0, z); sb.rotation.y = ry; scene.add(sb); });
  ([[-4.8, -3.5], [5.2, -4.5]] as Array<[number, number]>).forEach(([x, z], i) => {
    const c1 = crate(0.6); c1.position.set(x, 0, z); scene.add(c1);
    const c2 = crate(0.5); c2.position.set(x + 0.1, 0.6, z); c2.rotation.y = 0.4; scene.add(c2);
    const bl = barrel(); bl.position.set(x + (i ? -1 : 1) * 0.9, 0, z + 0.3); scene.add(bl);
  });
  if (tower) { const tw = watchtower(); tw.position.set(-9.5, 0, -14); tw.rotation.y = 0.5; scene.add(tw); }
  const tops: THREE.Vector3[] = [];
  ([[-11, -17], [0, -18.5], [11, -17]] as Array<[number, number]>).forEach(([x, z]) => { const pl = pole(4.2); pl.position.set(x, 0, z); scene.add(pl); tops.push(new THREE.Vector3(x, 3.6, z)); });
  for (let i = 0; i + 1 < tops.length; i++) scene.add(wire(tops[i], tops[i + 1], 0.35));
  return { scene, sun, stage: { y: 0, r: 9 } };
}

export interface SpaceLiteOptions { earth?: boolean; landerOn?: boolean; flagCol?: string }
export interface SpaceLiteSet { scene: THREE.Scene; sun: LightRig; stage: { y: number; r: number } }
/** a moonscape diorama, deliberately "lite": regolith, craters, stars, a code-painted Earth, a lander and a flag. Vacuum light (key 3.0, hemi 0.15) and NO atmosphere preset applies — envMatch: leave OFF, there is no air to match */
export function spaceLite(E: Engine, { earth = true, landerOn = true, flagCol = '#D8343F' }: SpaceLiteOptions = {}): SpaceLiteSet {
  const scene = new THREE.Scene();
  sky(scene, { top: '#05070F', horizon: '#05070F', ground: '#05070F' });
  scene.add(stars(420, 370));
  const sun = sunlight(scene, E, { col: '#FFFFFF', i: 3.0, pos: [7, 11, 5], sky: '#2A3040', ground: '#101216', hemi: 0.15, box: 10 });
  const ground = new THREE.Mesh(new THREE.PlaneGeometry(220, 220), mat({ map: T.dirt('#8A8D91', [48, 48], false) }));
  ground.rotation.x = -Math.PI / 2; ground.position.y = -0.02; ground.receiveShadow = true; scene.add(ground);
  ([[-7, -9, 1.6], [9, -13, 2.3], [-13, -20, 2.8], [4, -26, 1.4], [16, -6, 1.2]] as Array<[number, number, number]>).forEach(([x, z, r]) => {
    const rim = new THREE.Mesh(new THREE.TorusGeometry(r, r * 0.16, 8, 24), mat({ color: '#9A9DA1', roughness: 1 }));
    rim.rotation.x = Math.PI / 2; rim.position.set(x, 0.02, z); rim.receiveShadow = true; scene.add(rim);
    const bowl = new THREE.Mesh(new THREE.CircleGeometry(r * 0.95, 24), mat({ color: '#6E7176', roughness: 1 }));
    bowl.rotation.x = -Math.PI / 2; bowl.position.set(x, 0.005, z); bowl.receiveShadow = true; scene.add(bowl);
  });
  if (earth) {
    const et = canvasTex('earth', 256, 256, (c) => {
      const gr = c.createRadialGradient(108, 100, 10, 128, 128, 126); gr.addColorStop(0, '#7FB8E8'); gr.addColorStop(0.6, '#3E7CC4'); gr.addColorStop(1, '#1E4E7E');
      c.fillStyle = gr; c.beginPath(); c.arc(128, 128, 124, 0, 7); c.fill();
      c.fillStyle = '#69A85E'; ([[86, 96, 30], [160, 140, 36], [116, 176, 22], [176, 84, 18]] as Array<[number, number, number]>).forEach(([x, y, r]) => { c.beginPath(); c.arc(x, y, r, 0, 7); c.fill(); });
      c.fillStyle = 'rgba(255,255,255,0.75)'; ([[70, 70, 16], [150, 110, 20], [100, 190, 14], [190, 160, 12]] as Array<[number, number, number]>).forEach(([x, y, r]) => { c.beginPath(); c.ellipse(x, y, r * 1.7, r * 0.6, 0.5, 0, 7); c.fill(); });
    });
    const sp = new THREE.Sprite(new THREE.SpriteMaterial({ map: et, fog: false, toneMapped: false, depthWrite: false, transparent: true }));
    sp.position.set(-60, 55, -120); sp.scale.set(34, 34, 1); scene.add(sp);
  }
  if (landerOn) { const ld = lander(); ld.position.set(5.5, 0, -4.5); ld.rotation.y = -0.5; scene.add(ld); }
  const fl = new THREE.Group(); fl.position.set(-2.6, 0, -2); scene.add(fl);
  cyl(0.03, 0.04, 1.6, mat({ color: '#C9C9CE', metalness: 0.6, roughness: 0.35 }), 0, 0.8, 0, fl, 8);
  const flagT = canvasTex('flag' + flagCol, 128, 84, (c) => { c.fillStyle = flagCol; c.fillRect(0, 0, 128, 84); c.fillStyle = '#F4EFE4'; c.beginPath(); c.arc(38, 42, 17, 0, 7); c.fill(); c.fillStyle = flagCol; c.beginPath(); c.arc(45, 38, 14, 0, 7); c.fill(); });
  const flag = new THREE.Mesh(new THREE.PlaneGeometry(0.72, 0.46), mat({ map: flagT, side: THREE.DoubleSide }));
  flag.position.set(0.38, 1.32, 0); flag.castShadow = true; fl.add(flag);
  return { scene, sun, stage: { y: 0, r: 9 } };
}
