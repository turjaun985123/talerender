// library.ts — the SHARED skill-library metadata format (v5, D4 §5.1, proposal of record
// for the whole wave). Types only — no runtime imports, no behaviour.
//
// Every library item (pose, mood, hat, world, prop, shot, …) is declared ONCE, in code,
// next to the data it describes, with a copy-paste snippet and a preview path, so the
// generated library/*.md docs can never drift from the code. Rules:
//   (1) an item does not exist for the library until its meta row exists — builders add
//       the row in the same patch as the feature;
//   (2) preview paths follow ONE convention for all squads: 'library/previews/<kind>-<id>.jpg'
//       (the exact name castsheet writes),
//       and a preview is ALWAYS a tool render (castsheet / world sheet / capture sheet),
//       never generated art (Hard Law 2); '' only for non-visual kinds ('sound'/'caption');
//   (3) wording lives in these rows, in code review — the doc generator is a dumb table
//       printer over the LIBRARY arrays (poses.ts POSES_LIBRARY, toon.ts MOODS_LIBRARY /
//       HATS_LIBRARY / NECKS_LIBRARY / EXTRAS_LIBRARY, looks.ts LOOKS_LIBRARY, and the
//       worlds/props/cameras arrays in their own modules, same shape).
export type LibraryKind = 'pose' | 'mood' | 'hat' | 'neck' | 'extra' | 'look'
  | 'world' | 'prop' | 'shot' | 'sound' | 'caption';
export interface LibraryItemMeta {
  id: string;          // dotted, stable: 'pose.react', 'mood.puzzled', 'hat.crown', 'world.farm', 'prop.crate'
  kind: LibraryKind;
  name: string;        // display name: 'React (shock-hold)'
  snippet: string;     // ONE copy-paste line of real code, e.g. "p.set({ ...POSES.react, look: [0.4, 0] })"
  captionSafe: string; // composition note: what the item does near the caption zone
                       // (frame y 1340–1550) / edges, e.g. 'arms stay inside stage x 60..452; safe over captions'
  preview: string;     // repo-relative path to the item's preview image, ALWAYS a tool render,
                       // e.g. 'library/previews/pose-react.jpg'; '' only for kind 'sound'/'caption'
  optIn: string;       // the exact enabling switch/field, verbatim: "spread POSES.react into set()",
                       // "mood: 'puzzled'", "new Toon(look, { multiPlane: true })", 'none — data only' where true
  since: string;       // '5.0' for everything added in the v5 wave
}
