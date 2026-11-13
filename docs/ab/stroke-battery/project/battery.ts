// M4 stroke battery — REFERENCE side. Draws the battery cards with
// the real Canvas2D painter (Skia, in Chrome): the primitive ops use
// the same ctx call forms drawToonLayer uses (lineCap/lineJoin
// 'round' set once per pass), and the character/face cards call the
// template's own drawToon under the same transform Toon#set applies
// (scale(res) then translate(OX, OY)).
// The spec is the package's spec.json, copied next to this page as
// battery-spec.json by run-reference.sh — the same bytes the native
// binary embeds. Nothing here is redrawn or approximated: this page
// IS the production painter, driven with battery inputs.
import { drawToon, STAGE } from './core/toon';

const spec = await (await fetch('battery-spec.json')).json();
const RES: number = spec.canvas.res;

function makeCanvas(): HTMLCanvasElement {
  const cv = document.createElement('canvas');
  cv.width = spec.canvas.w * RES;
  cv.height = spec.canvas.h * RES;
  return cv;
}

async function send(name: string, cv: HTMLCanvasElement): Promise<void> {
  const blob = await new Promise<Blob | null>((r) => cv.toBlob(r, 'image/png'));
  if (!blob) throw new Error('toBlob failed for ' + name);
  await fetch('save/' + name, { method: 'POST', body: await blob.arrayBuffer() });
}

// ---- prims card: raw Canvas2D, one shared spec ----
{
  const cv = makeCanvas();
  const g = cv.getContext('2d')!;
  g.setTransform(RES, 0, 0, RES, 0, 0);
  g.lineCap = 'round';
  g.lineJoin = 'round';
  for (const op of spec.prims) {
    const col: string = op.col || spec.ink;
    if (op.op === 'line' || op.op === 'polyline') {
      g.strokeStyle = col;
      g.lineWidth = op.w;
      g.beginPath();
      op.p.forEach(([x, y]: number[], i: number) => (i ? g.lineTo(x, y) : g.moveTo(x, y)));
      g.stroke();
    } else if (op.op === 'quad') {
      g.strokeStyle = col;
      g.lineWidth = op.w;
      g.beginPath();
      g.moveTo(op.p[0][0], op.p[0][1]);
      g.quadraticCurveTo(op.p[1][0], op.p[1][1], op.p[2][0], op.p[2][1]);
      g.stroke();
    } else if (op.op === 'stroke_ellipse') {
      g.strokeStyle = col;
      g.lineWidth = op.w;
      g.beginPath();
      g.ellipse(op.c[0], op.c[1], op.rx, op.ry, 0, 0, Math.PI * 2);
      g.stroke();
    } else if (op.op === 'fill_ellipse') {
      g.fillStyle = col;
      g.beginPath();
      g.ellipse(op.c[0], op.c[1], op.rx, op.ry, 0, 0, Math.PI * 2);
      g.fill();
    } else if (op.op === 'arc') {
      g.strokeStyle = col;
      g.lineWidth = op.w;
      g.beginPath();
      g.arc(op.c[0], op.c[1], op.r, op.a0, op.a1);
      g.stroke();
    } else {
      throw new Error('unknown prim op ' + op.op);
    }
  }
  await send('prims.png', cv);
}

// ---- character + face cards: the real drawToon ----
function toonCard(look: unknown, state: unknown): HTMLCanvasElement {
  const cv = makeCanvas();
  const g = cv.getContext('2d')!;
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.scale(RES, RES);
  g.translate(STAGE.OX, STAGE.OY);
  drawToon(g, look as never, state as never);
  return cv;
}

await send('toon.png', toonCard(spec.toon.look, spec.toon.state));
for (const mood of spec.faces.moods) {
  await send(`face-${mood}.png`, toonCard(spec.faces.look, { ...spec.faces.state, mood }));
}

// ---- provenance + done ----
let renderer = 'unknown';
try {
  const gl = document.createElement('canvas').getContext('webgl');
  const ext = gl?.getExtension('WEBGL_debug_renderer_info');
  if (gl && ext) renderer = String(gl.getParameter(ext.UNMASKED_RENDERER_WEBGL));
} catch {
  /* keep 'unknown' */
}
await fetch('done', {
  method: 'POST',
  body: JSON.stringify({ userAgent: navigator.userAgent, webglRenderer: renderer }),
});
