// Timing: everything on screen is placed on the narration's clock.
//   data/lines.json  { duration, lines: [{ i, text, start, end }] }   (voice.py, or estimate.py for captions-only)
//   data/words.json  [{ w, line, s, e }]                                (words.py, or estimate.py)
// L(i) = line i start · Le(i) = line i end · Wd(word, line) = start of the first word in that line matching
// the prefix (case/punctuation ignored; throws if it isn't there, so a rewritten script can't silently drift).
export interface TimingLine { i: number; text: string; start: number; end: number; caps?: string }
export interface TimingWord { w: string; line: number; s: number; e: number }
export interface TimingData { dur: number; lines: TimingLine[]; words: TimingWord[] }
export const T: TimingData = { dur: 0, lines: [], words: [] };
const norm = (w: string) => String(w).toLowerCase().replace(/[^a-z0-9']+/g, '');

export async function loadTiming() {
  const get = async <X>(f: string): Promise<X> => { const r = await fetch(f, { cache: 'no-store' }); if (!r.ok) throw new Error(`missing ${f}: run voice.py + words.py (or estimate.py)`); return r.json() as Promise<X>; };
  const lines = await get<{ duration: number; lines: TimingLine[] }>('data/lines.json'), words = await get<TimingWord[]>('data/words.json');
  T.lines = lines.lines; T.words = words; T.dur = lines.duration;
  return T;
}
export const L = (i: number) => { const l = T.lines[i]; if (!l) throw new Error(`no line ${i}`); return l.start; };
export const Le = (i: number) => { const l = T.lines[i]; if (!l) throw new Error(`no line ${i}`); return l.end; };
export function Wd(word: string, line?: number | null, which: 's' | 'e' = 's') {
  const n = norm(word);
  const hit = T.words.find((w) => (line == null || w.line === line) && norm(w.w).startsWith(n));
  if (!hit) throw new Error(`word "${word}" not found${line != null ? ` in line ${line}` : ''}`);
  return hit[which];
}
export const lineWords = (i: number) => T.words.filter((w) => w.line === i);
