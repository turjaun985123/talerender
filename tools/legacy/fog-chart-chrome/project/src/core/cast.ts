// Cast registry: define the cast ONCE, build every set's copy of a character
// from castLook, and capture check then proves the same person looks identical
// in every set/shot.
//
//   registerCast('name', look)  — once, at the top of the story's shot setup.
//   castLook('name')            — every time a set needs its own copy of that
//                                 character (each set builds its own Toons, so
//                                 each set asks the registry for a fresh clone
//                                 of the one true look instead of re-typing it).
//
// If a name is registered twice with a different identity, registerCast logs
// a console.error naming the character and the differing fields and keeps the
// FIRST registration — the page's console errors fail capture check, so that
// is the gate working, not a silent restyle.
import type { ToonLook } from './toon';

/** The cast registry: character name -> the one true look for that character. */
export const CAST = new Map<string, ToonLook>();

/**
 * Canonical signature of a look's IDENTITY, as a JSON string.
 *
 * Identity fields only, in this fixed key order:
 *   body, pattern, patCol, hat, hatCol, band, neck, neckCol, head, kid, extra
 *
 * Missing fields become null, `kid` is coerced to a boolean, and `extra` is a
 * sorted copy of the array (so ordering of extras never changes identity).
 *
 * Deliberately EXCLUDED — stamps, dots, shade, patCol2, hairCol, mustacheCol
 * (and `name` itself): these are surface details a story may legitimately
 * restyle mid-film, e.g. stamps accumulating on a coat, so they are not part
 * of who the character is and must not trip the continuity gate.
 */
export function lookSignature(look: ToonLook): string {
  return JSON.stringify({
    body: look.body ?? null,
    pattern: look.pattern ?? null,
    patCol: look.patCol ?? null,
    hat: look.hat ?? null,
    hatCol: look.hatCol ?? null,
    band: look.band ?? null,
    neck: look.neck ?? null,
    neckCol: look.neckCol ?? null,
    head: look.head ?? null,
    kid: Boolean(look.kid),
    extra: [...(look.extra ?? [])].sort(),
  });
}

/**
 * Register a character's one true look under `name`.
 *
 * Stores a frozen-in-time clone `{ ...look, extra: [...(look.extra ?? [])], name }`
 * so later mutation of the caller's object cannot change the registry.
 * If `name` is already registered with a DIFFERENT signature, logs a
 * console.error naming the character and the differing fields and keeps the
 * FIRST registration. Returns the stored clone.
 */
export function registerCast(name: string, look: ToonLook): ToonLook {
  const existing = CAST.get(name);
  if (existing) {
    const prevSig = lookSignature(existing);
    const nextSig = lookSignature(look);
    if (prevSig !== nextSig) {
      const prev = JSON.parse(prevSig) as Record<string, unknown>;
      const next = JSON.parse(nextSig) as Record<string, unknown>;
      const keys = [...Object.keys(prev), ...Object.keys(next).filter((k) => !(k in prev))];
      const fields = keys.filter((k) => JSON.stringify(prev[k]) !== JSON.stringify(next[k]));
      console.error(`registerCast: character "${name}" already registered with a different look — differing fields: ${fields.join(', ')}; keeping the first registration`);
    }
    return existing;
  }
  const stored: ToonLook = { ...look, extra: [...(look.extra ?? [])], name };
  CAST.set(name, stored);
  return stored;
}

/**
 * Return a fresh clone of the registered look for `name` (with `name` set),
 * so a caller can mutate or pass it to a Toon without touching the registry.
 * Throws a clear Error for an unregistered name.
 */
export function castLook(name: string): ToonLook {
  const stored = CAST.get(name);
  if (!stored) throw new Error(`castLook: unregistered character "${name}" — call registerCast("${name}", look) first`);
  return { ...stored, extra: [...(stored.extra ?? [])], name };
}
