// Captions: shown in CHUNKS. A script line may set "caps": "First part | second part | third part"
// to author its chunks (2–3 words each is the house style); otherwise chunks are made automatically
// (one word per chunk, a short word riding along with the next when combined ≤ 11 chars:
// "a single", "the whole"). Authored chunks are anchored by their FIRST word: each chunk starts
// at the first timed word after the previous chunk whose normalised text (lowercase, punctuation
// stripped) equals the chunk's first word — so a chunk's first word must match a spoken word
// exactly, or the chunk silently falls back to the word after the previous anchor. A chunk stays
// up until the next chunk starts; across a line break longer than 0.6 s it holds 0.25 s past its
// own line's end. White heavy sans (Inter 900) with a black outline and a soft drop shadow,
// centre of the screen, fit-scaled to CAP.maxW, pop-in ramp on entry. No colour highlight.
// Words keep their punctuation.
import { ctx, clamp, lerp, seg, eob } from './lib';
import { T, lineWords } from './timing';

export const CAP = { y: 1340, size: 88, maxW: 960 };
const FONT = `900 ${CAP.size}px Inter`;
const fitCache = new Map<string, { w: number; fit: number }>();   // text -> {raw width, fit}: both are pure functions of text under FONT + letterSpacing '0px'
/** Measure a caption text once (under the same font state captionText draws with) and cache its raw width + fit. */
function measure(text: string): { w: number; fit: number } {
  let m = fitCache.get(text);
  if (!m) { const w = ctx.measureText(text).width; m = { w, fit: Math.min(1, CAP.maxW / w) }; fitCache.set(text, m); }
  return m;
}
export interface CaptionChunk { line: number; s: number; e: number; text: string; auto: boolean }
export interface ScriptData { lines: Array<{ text: string; caps?: string }> }
let caps: CaptionChunk[] = [];
const SHORT = /^(a|an|the|to|of|in|on|at|by|and|but|or|so|is|it|he|she|we|i|his|her|my|no|up|for|was)$/i;

function autoChunks(ws: ReadonlyArray<string>): Array<[number, number]> {
  const out: Array<[number, number]> = [];
  for (let i = 0; i < ws.length; i++) {
    const w = ws[i], n = ws[i + 1], bare = w.replace(/[^A-Za-z']/g, '');
    if (n && SHORT.test(bare) && !/[,.!?;:]$/.test(w) && (w + n).length <= 11) { out.push([i, i + 1]); i++; } else out.push([i, i]);
  }
  return out;
}

export function buildCaptions(script: ScriptData) {
  caps = [];
  script.lines.forEach((ln, i) => {
    const ws = lineWords(i); if (!ws.length) return;
    const textWords = ln.text.split(/\s+/).filter(Boolean);
    const spoken = ws.length === textWords.length ? textWords : ws.map((w) => w.w);
    if (ln.caps) {
      const nm = (w: string) => String(w).toLowerCase().replace(/[^a-z0-9']+/g, '');
      let from = 0;
      ln.caps.split('|').map((c) => c.trim()).filter(Boolean).forEach((c, j) => {
        const first = nm(c.split(/\s+/)[0]);
        let idx = j === 0 ? 0 : ws.findIndex((w, k) => k > from && nm(w.w) === first);
        if (idx < 0) idx = Math.min(ws.length - 1, from + 1);
        from = idx; caps.push({ line: i, s: ws[idx].s - 0.03, e: 0, text: c.replace(/\*/g, ''), auto: false });
      });
    } else autoChunks(spoken).forEach(([a, b]) => caps.push({ line: i, s: ws[a].s - 0.03, e: 0, text: spoken.slice(a, b + 1).join(' '), auto: true }));
  });
  caps.forEach((c, k) => { const next = caps[k + 1], lineEnd = T.lines[c.line].end; c.e = next ? (next.s - lineEnd > 0.6 && next.line !== c.line ? lineEnd + 0.25 : next.s) : T.dur; });
  return caps;
}
export interface CaptionMetric { line: number; s: number; e: number; text: string; auto: boolean; rawW: number; fit: number; renderedW: number }
/** v5 (D8 §1b): per-chunk measurements for the capture-check caption audit — the SAME
 *  measureText measurement the renderer itself uses (shared cache), never a second
 *  measurer. rawW = width at 88 px unscaled; fit = min(1, CAP.maxW / rawW);
 *  renderedW = rawW × fit (peak width; the 0.86→1 pop-in ramp is ignored). */
export function captionMetrics(): CaptionMetric[] {
  ctx.save(); ctx.font = FONT; ctx.letterSpacing = '0px';
  const out = caps.map((c) => { const m = measure(c.text); return { line: c.line, s: c.s, e: c.e, text: c.text, auto: c.auto, rawW: +m.w.toFixed(1), fit: +m.fit.toFixed(3), renderedW: +(m.w * m.fit).toFixed(1) }; });
  ctx.restore();
  return out;
}
export interface CaptionListEntry extends CaptionChunk { rawW: number; fit: number; renderedW: number; words: Array<{ txt: string; hi?: boolean }> }
export const captionList = (): CaptionListEntry[] => { const ms = captionMetrics(); return caps.map((c, k) => ({ ...c, rawW: ms[k].rawW, fit: ms[k].fit, renderedW: ms[k].renderedW, words: [{ txt: c.text }] })); };

/** draw one caption text at x, y (also used by the style sheet) */
export function captionText(text: string, x = 540, y = CAP.y, sc = 1) {
  ctx.save(); ctx.font = FONT; ctx.letterSpacing = '0px';
  // measureText runs only on a cache miss, under the same font/letterSpacing state as before;
  // fonts are loaded before any captionText call in both entry points (main.js, sheet.js).
  const fit = measure(text).fit;
  ctx.translate(x, y); ctx.scale(sc * fit, sc * fit); ctx.textAlign = 'center'; ctx.textBaseline = 'middle'; ctx.lineJoin = 'round';
  ctx.fillStyle = 'rgba(0,0,0,0.45)'; ctx.fillText(text, 3, 9);
  ctx.lineWidth = 15; ctx.strokeStyle = '#0B0B10'; ctx.strokeText(text, 0, 0);
  ctx.fillStyle = '#FFFFFF'; ctx.fillText(text, 0, 0);
  ctx.restore();
}
export function drawCaption(t: number) {
  const c = caps.find((c) => t >= c.s && t < c.e); if (!c) return;
  const lt = t - c.s; captionText(c.text, 540, CAP.y, lt < 0.1 ? lerp(0.86, 1, eob(seg(lt, 0, 0.1))) : 1);
}
