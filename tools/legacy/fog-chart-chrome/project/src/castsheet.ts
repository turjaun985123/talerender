// Cast sheet page (v5 Phase 3): stages ONE library item per CAST.draw(i) call on a
// plain set — poses, moods, looks and held props, all drawn with core/toon's
// drawToon — so `tools/capture.mjs castsheet` can render every library preview
// from code alone (canvas items on a plain backdrop; no three.js scene, no film).
// Preview files are named <kind>-<id>.jpg after the library/index.json convention.
import { drawToon, MOODS, PROPS, type Mood, type PropFn, type ToonLook, type ToonState } from './core/toon';
import { POSES, POSE_NAMES } from './core/poses';
import { LOOK_PRESETS } from './core/looks';

export interface CastItem { kind: string; id: string; name: string }

const cv = document.getElementById('cv') as HTMLCanvasElement;
const g = cv.getContext('2d')!;

const BASE_LOOK: ToonLook = { body: '#7A5FBF', hat: 'none' };
const ALL_PROPS = PROPS as unknown as Record<string, (...a: never[]) => unknown>;
function propFn(id: string): PropFn {
  const raw = ALL_PROPS[id];
  // Factories (book/cash/sign/map — declared with <= 2 params) RETURN the PropFn;
  // direct PropFns take (g, x, y, ...) — 3+ declared params. Defaults apply.
  return (raw.length <= 2 ? (raw as () => PropFn)() : raw) as PropFn;
}

export const CAST_ITEMS: CastItem[] = [
  ...POSE_NAMES.map((id) => ({ kind: 'pose', id: id as string, name: `pose ${id}` })),
  ...MOODS.map((id) => ({ kind: 'mood', id: id as string, name: `mood ${id}` })),
  ...Object.keys(LOOK_PRESETS).map((id) => ({ kind: 'look', id, name: `look ${id}` })),
  ...Object.keys(PROPS).map((id) => ({ kind: 'held', id, name: `prop ${id}` })),
];

const FLOOR_Y = 1560;
function stage(look: ToonLook, st: ToonState, heightM = 1.75): void {
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.fillStyle = '#EFE7D6'; g.fillRect(0, 0, cv.width, cv.height);
  g.fillStyle = '#DCCBA6'; g.fillRect(0, FLOOR_Y, cv.width, cv.height - FLOOR_Y);
  g.fillStyle = 'rgba(60,45,20,0.18)';
  g.beginPath(); g.ellipse(cv.width / 2, FLOOR_Y + 18, 280, 40, 0, 0, Math.PI * 2); g.fill();
  // drawToon paints in raw stage coords (512x1024 box, figure centred at x=256,
  // feet at y=1000): scale the stage so the feet land on FLOOR_Y, centred.
  const s = 1.35 * (heightM / 1.75);
  g.translate(cv.width / 2 - 256 * s, FLOOR_Y - 1000 * s);
  g.scale(s, s);
  drawToon(g, look, st);
  g.setTransform(1, 0, 0, 1, 0, 0);
}

function draw(i: number): void {
  const it = CAST_ITEMS[i];
  if (!it) return;
  if (it.kind === 'pose') stage(BASE_LOOK, { ...POSES[it.id as keyof typeof POSES] });
  else if (it.kind === 'mood') stage(BASE_LOOK, { mood: it.id as Mood });
  else if (it.kind === 'look') {
    const p = LOOK_PRESETS[it.id as keyof typeof LOOK_PRESETS];
    stage({ ...BASE_LOOK, ...p.look }, p.pose ? { ...POSES[p.pose as keyof typeof POSES] } : {}, p.height);
  } else stage(BASE_LOOK, { holdR: propFn(it.id), armR: [72, -38], armL: [14, 8] });
}

function label(i: number): void {
  const it = CAST_ITEMS[i];
  if (!it) return;
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.fillStyle = '#000'; g.fillRect(0, 0, cv.width, 92);
  g.fillStyle = '#fff'; g.font = '700 54px sans-serif'; g.textAlign = 'left'; g.textBaseline = 'alphabetic';
  g.fillText(`${it.kind} — ${it.id}`, 22, 62);
}

const CAST = { ready: false, bootError: null as string | null, items: () => CAST_ITEMS, draw, label };
(window as unknown as { CAST: typeof CAST }).CAST = CAST;
try { draw(0); } catch (e) { CAST.bootError = e instanceof Error ? e.message : String(e); }
// Previews must not render in fallback fonts (held-prop lettering is Inter):
// ready only once the font set settles.
if (typeof document.fonts?.ready?.then === 'function') document.fonts.ready.then(() => { CAST.ready = true; });
else CAST.ready = true;
