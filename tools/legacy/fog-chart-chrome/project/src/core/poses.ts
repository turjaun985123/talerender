// POSES — curated starting bundles for Toon#set: spread one into set() first, then override
// freely, e.g. p.set({ ...POSES.point, look: [0.2, 0] }). A pose bundles arm/lean/head/mood
// state only — it is a starting point for acting, not a finished performance. Poses contain
// no functions, so they are safe in the repaint key.
// Arm convention: [shoulder°, elbow°], 0 = hanging down, + = outward/up (see core/toon.ts).
import type { ToonState } from './toon';
import type { LibraryItemMeta } from './library';

export const POSES = {
  point: { mood: 'neutral', armR: [92, -6], armL: [12, 8], lean: 2, headTurn: 0.15 },
  wave: { mood: 'happy', armR: [158, 12], armL: [10, 6] },
  shrug: { mood: 'neutral', armL: [75, -60], armR: [75, -60], bob: -6 },
  think: { mood: 'neutral', armR: [48, -118], armL: [20, -30], headTilt: 6, headTurn: 0.3, look: [0.4, -0.3] },
  celebrate: { mood: 'laugh', armL: [150, 14], armR: [150, 14], bob: -8 },
  mourn: { mood: 'sad', armL: [4, 4], armR: [4, 4], lean: 5, headTilt: -4, bob: 4 },
  explain: { mood: 'happy', armR: [68, -42], armL: [28, -24], look: [0.3, 0] },
  listen: { mood: 'neutral', armR: [118, -105], armL: [10, 8], lean: 3, headTilt: -5, headTurn: -0.2 },
  // ── v5 acting poses (D4 §1.3): starting bundles in the STYLE §8 sense — spread, then direct ──
  react: { mood: 'shock', armL: [34, -26], armR: [34, -26], lean: -3, squash: 0.03 },
  facepalm: { mood: 'embarrassed', armR: [120, 78], armL: [10, 6], headTilt: -6, lean: 4, bob: 2 },
  sneak: { mood: 'sly', armL: [35, -55], armR: [35, -55], lean: 8, bob: 2, look: [0.5, 0] },
  exhausted: { mood: 'tired', armL: [4, 2], armR: [4, 2], lean: 9, headTilt: -7, bob: 5, sweat: 1 },
  dazed: { mood: 'dizzy', armL: [70, -15], armR: [105, -40], lean: -4, headTilt: 9, squash: 0.02, look: [0.3, 0.2] },
  boast: { mood: 'proud', armR: [25, -105], armL: [38, -110], lean: -3, bob: -2 },
  salute: { mood: 'determined', armR: [146, 30], armL: [8, 6], lean: 0, headTilt: 0 },
  read: { mood: 'neutral', armL: [42, -48], armR: [42, -48], look: [0, 0.5], headTilt: 4 },
  elderStoop: { mood: 'neutral', armL: [12, 8], armR: [20, -30], lean: 7, headTilt: -5, bob: 3 },
} satisfies Record<string, Partial<ToonState>>;

export type PoseName = keyof typeof POSES;

/** runtime pose-name list (D4 §1.1): sheets/docs/tests iterate this, never a hardcoded copy */
export const POSE_NAMES = Object.keys(POSES) as PoseName[];

// Skill-library metadata (D4 §5.1): one row per pose, declared next to the data.
const poseMeta = (name: PoseName, display: string, captionSafe: string, since: string): LibraryItemMeta => ({
  id: `pose.${name}`, kind: 'pose', name: display, snippet: `p.set({ ...POSES.${name} })`,
  captionSafe, preview: `library/previews/pose-${name}.jpg`, optIn: `spread POSES.${name} into set()`, since,
});
export const POSES_LIBRARY: LibraryItemMeta[] = [
  poseMeta('point', 'Point', 'raised arm stays inside the stage margins', 'pre-5.0'),
  poseMeta('wave', 'Wave', 'raised arm stays inside the stage margins', 'pre-5.0'),
  poseMeta('shrug', 'Shrug', 'elbows out, hands below the caption zone on a standing figure', 'pre-5.0'),
  poseMeta('think', 'Think', 'hand to chin; face stays clear of the caption zone', 'pre-5.0'),
  poseMeta('celebrate', 'Celebrate', 'both arms up — keep the figure low in frame so hands clear the top edge', 'pre-5.0'),
  poseMeta('mourn', 'Mourn', 'arms hang; no caption-zone impact', 'pre-5.0'),
  poseMeta('explain', 'Explain', 'one arm mid-gesture; safe over captions', 'pre-5.0'),
  poseMeta('listen', 'Listen', 'hand to ear; no caption-zone impact', 'pre-5.0'),
  poseMeta('react', 'React (shock-hold)', 'hands half-raised, body rocked back; the held beat after a reveal — static partner to reactCue', '5.0'),
  poseMeta('facepalm', 'Facepalm', 'forearm folded fully up, hand on forehead, head bowed; swap mood to tired for the weary variant', '5.0'),
  poseMeta('sneak', 'Sneak', 'tiptoe menace/comedy; pair with legs walk + walkPhase(dist, stride*0.6) + walkBob(phase, 3)', '5.0'),
  poseMeta('exhausted', 'Exhausted', 'spent, upright collapse; floor collapse = legs sit + this bundle', '5.0'),
  poseMeta('dazed', 'Dazed', 'uneven windmill arms sell the post-impact wobble; arms stay inside the stage margins', '5.0'),
  poseMeta('boast', 'Boast', 'chest-out brag: fist to chest, other hand on hip, leaning back', '5.0'),
  poseMeta('salute', 'Salute', 'hand snapped to the brow — military/order beats and "yes sir" gags', '5.0'),
  poseMeta('read', 'Read', 'both hands forward-down holding a document; pair with holdL/holdR PROPS.book/paper/letters', '5.0'),
  poseMeta('elderStoop', 'Elder stoop', 'age as POSTURE, never a new face (channel identity rule); right hand cane-ready (holdR a cane/stick prop)', '5.0'),
];
