// looks.ts — age/body variation catalog + crowd look generator (v5, D4 §5.3).
//
// NO new geometry: any change to drawToon's body math is a regression risk against the
// golden projects, so variation is expressed ONLY through mechanisms that already exist
// and already render identically at their defaults — `kid` (drawToon), the Toon
// constructor's `height`, and pose bundles for posture. Age is POSTURE, never a new face
// (channel identity rule: a character keeps their face and proportions; only age, height,
// clothing and posture evolve).
import { rnd } from './lib';
import { HATS, NECKS, type ToonLook } from './toon';
import type { LibraryItemMeta } from './library';

export interface LookPreset { look: Partial<ToonLook>; height?: number; pose?: string; note: string }

export const LOOK_PRESETS: Record<string, LookPreset> = {
  kid: {
    look: { kid: true }, height: 1.35,
    note: 'short legs are drawn by drawToon (kid); pair stride 0.49 in walkPhase',
  },
  adult: {
    look: {}, height: 1.75,
    note: 'the default; every v4 project is this',
  },
  tall: {
    look: {}, height: 1.95,
    note: 'same drawing, taller plane — check headroom vs caption zone',
  },
  elder: {
    look: {}, height: 1.68, pose: 'elderStoop',
    note: 'age is POSTURE, never a new face (channel identity rule)',
  },
  stocky: {
    look: {}, height: 1.66,
    note: 'height-only illusion of breadth; body width is not a look field — do not fake it with squash',
  },
};

/**
 * crowdLook(i, palette) — deterministic crowd picker: the same i always returns the
 * same look (pure, built on lib rnd). Body colour cycles `palette`; hat and neck cycle
 * the EXISTING HATS/NECKS kinds; `kid` is true for ~1 in 5. Feed the results to a
 * set's extras and give each member blinkDirector(i) + crowdReact(t, hit, i) — a crowd
 * becomes a library call, not per-project improvisation.
 * Usage: `const look = crowdLook(i, ['#C9603F', '#6FA3B8', '#7FA36A']);`
 */
export function crowdLook(i: number, palette: string[]): Partial<ToonLook> {
  const look: Partial<ToonLook> = {
    hat: HATS[Math.floor(rnd(i * 3 + 1) * HATS.length) % HATS.length],
    neck: NECKS[Math.floor(rnd(i * 3 + 2) * NECKS.length) % NECKS.length],
    kid: rnd(i * 3 + 3) < 0.2,
  };
  if (palette && palette.length) look.body = palette[((i % palette.length) + palette.length) % palette.length];
  return look;
}

// Skill-library metadata (D4 §5.1): one row per preset, declared next to the data.
export const LOOKS_LIBRARY: LibraryItemMeta[] = (Object.keys(LOOK_PRESETS) as Array<keyof typeof LOOK_PRESETS>).map((name) => {
  const p = LOOK_PRESETS[name];
  return {
    id: `look.${name}`, kind: 'look' as const, name: `${name} — ${p.note}`,
    snippet: `new Toon({ ...look, ...LOOK_PRESETS.${name}.look }, { height: LOOK_PRESETS.${name}.height })`,
    captionSafe: 'height changes headroom vs the caption zone — tall presets need the framing checked',
    preview: `library/previews/look-${name}.jpg`,
    optIn: `LOOK_PRESETS.${name} (look + height${p.pose ? ` + POSES.${p.pose}` : ''})`, since: '5.0',
  };
});
