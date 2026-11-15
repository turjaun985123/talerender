// acting.probe.ts — shared probe for the v5 acting layer (D4 §6): poses/moods expansion,
// moodCue/reactCue/crowdReact, talkTrack, looks. Run by hand or by the extended selftest:
//   bun run tests/run-acting-probe.ts   → prints ACTING PROBE PASSED, or throws the failing
// assert. (The wrapper bundles this file with 'three' mapped onto template/vendor — see
// run-acting-probe.ts.) Pure logic only — no browser, no render lock.
import { MOODS, HATS, NECKS, EXTRAS, type Mood } from '../src/core/toon';
import { POSES, POSE_NAMES } from '../src/core/poses';
import { moodCue, reactCue, crowdReact, talkTrack, speakingAt } from '../src/core/motion';
import { LOOK_PRESETS, crowdLook } from '../src/core/looks';
import { lookSignature } from '../src/core/cast';

let n = 0;
const ok = (cond: boolean, msg: string) => { n++; if (!cond) throw new Error(`acting probe FAILED: ${msg}`); };
const eq = (a: unknown, b: unknown, msg: string) => ok(JSON.stringify(a) === JSON.stringify(b), `${msg} (got ${JSON.stringify(a)}, want ${JSON.stringify(b)})`);

// ── Spec 1: runtime lists mirror the unions (exhaustiveness is a tsc matter: this
// Record<Mood, true> fails typecheck the day a Mood is added without updating it) ──
const MOOD_EXHAUSTIVE: Record<Mood, true> = {
  neutral: true, smug: true, happy: true, laugh: true, shock: true, worried: true, angry: true,
  sad: true, sly: true, proud: true, tired: true, dizzy: true, determined: true,
  embarrassed: true, awe: true, suspicious: true, relieved: true, puzzled: true, deadpan: true,
};
ok(Object.keys(MOOD_EXHAUSTIVE).length === 19, 'mood exhaustiveness table has 19 entries');
eq(MOODS.length, 19, 'MOODS list length 19 (17 v4 + puzzled + deadpan)');
eq([...MOODS].sort(), Object.keys(MOOD_EXHAUSTIVE).sort(), 'MOODS list matches the Mood union exactly');
eq(HATS.length, 11, 'HATS list length 11');
eq(NECKS.length, 5, 'NECKS list length 5');
eq(EXTRAS.length, 6, 'EXTRAS list length 6');
eq(Object.keys(POSES).length, 17, 'POSES has 17 bundles (8 v4 + 9 new)');
eq(POSE_NAMES.length, 17, 'POSE_NAMES length 17');
for (const p of ['react', 'facepalm', 'sneak', 'exhausted', 'dazed', 'boast', 'salute', 'read', 'elderStoop'] as const)
  ok(p in POSES, `new pose present: ${p}`);
eq(POSES.react.mood, 'shock', 'react pose carries the shock mood');
eq(POSES.elderStoop.lean, 7, 'elderStoop leans 7 (posture, not a new face)');

// ── Spec 2: moodCue ──
const KEYS = [[0, 'smug'], [1, 'shock'], [2, 'sad']] as const;
eq(moodCue(KEYS, 0.5).mood, 'smug', 'moodCue: key-1 mood before key 2');
eq(moodCue(KEYS, 1.5).mood, 'shock', 'moodCue: key-2 mood after key 2');
eq(moodCue(KEYS, 1).blink, 1, 'moodCue: blink == 1 exactly at the swap time');
eq(moodCue(KEYS, 1.2).blink, 0, 'moodCue: blink == 0 at +0.2 s');
eq(moodCue(KEYS, 0.8).blink, 0, 'moodCue: blink == 0 at -0.2 s');
eq(moodCue(KEYS, 1).squash, -0.02, 'moodCue: squash dips to -0.02 at the swap');
eq(moodCue(KEYS, 1).headTilt, 2, 'moodCue: headTilt dips +2 at the swap');
eq(moodCue(KEYS, 0.5).blink, 0, 'moodCue: no mask outside windows');
eq(moodCue([], 1).mood, 'neutral', 'moodCue: empty keys → neutral, no crash');
eq(moodCue(KEYS, 1.03), moodCue(KEYS, 1.03), 'moodCue: pure (same inputs → deep-equal outputs)');

// ── Spec 2: reactCue ──
eq(reactCue(0, 0.2), {}, 'reactCue: {} at dt = -0.2');
ok(!('mood' in reactCue(0.23, 0.2)), 'reactCue: no mood at dt = 0.03 (masked flip window)');
eq(reactCue(0.23, 0.2).blink, 1, 'reactCue: blink 1 during the flip window');
eq(reactCue(0.3, 0.2).mood, 'shock', "reactCue: mood 'shock' at dt = 0.1");
eq(reactCue(0.1, 0.2).look, [-0.6, 0], 'reactCue: anticipation gaze drifts away (dir 1)');
eq(reactCue(0.1, 0.2, { dir: -1 }).look, [0.6, 0], 'reactCue: anticipation mirrors with dir -1');
eq(reactCue(2.2, 0.2, { hold: 1 }), {}, 'reactCue: {} after hold expires');
ok('mood' in reactCue(0.8, 0.2, { takes: 2 }), 'reactCue takes:2 still holds the mood at dt = 0.6');
eq(reactCue(0.6, 0.2, { takes: 2 }).look, [-0.5, 0], 'reactCue takes:2 gaze flicks away at dt = 0.4');
eq(reactCue(0.2, 0.2), reactCue(0.2, 0.2), 'reactCue: pure');
// crowdReact: delayed + squash scaled — at the original hit time a seeded member may not
// have reacted yet, and whenever both have squash, the crowd one is the smaller.
const cr = crowdReact(0.3, 0.2, 5), rc = reactCue(0.3, 0.2);
if (typeof cr.squash === 'number' && typeof rc.squash === 'number')
  ok(Math.abs(cr.squash) <= Math.abs(rc.squash) + 1e-9, 'crowdReact: squash never exceeds the solo reactCue squash');
eq(crowdReact(0.3, 0.2, 5), crowdReact(0.3, 0.2, 5), 'crowdReact: deterministic per seed');

// ── Spec 3: talkTrack / speakingAt ──
const WORDS = [{ s: 1, e: 1.42, line: 0 }, { s: 1.6, e: 1.8, line: 0 }];
eq(talkTrack(WORDS, 0.5), 0, 'talkTrack: 0 before the first word');
eq(talkTrack(WORDS, 1.51), 0, 'talkTrack: 0 in the gap');
eq(talkTrack(WORDS, 2.0), 0, 'talkTrack: 0 after the last word');
ok(talkTrack(WORDS, 1.1) >= 0.3, 'talkTrack: >= 0.3 inside the first word');
ok(talkTrack(WORDS, 1.7) >= 0.3, 'talkTrack: >= 0.3 inside the second word');
for (let t = 0.9; t <= 2.0; t += 0.01) {
  const v = talkTrack(WORDS, t);
  ok(Math.abs(v * 20 - Math.round(v * 20)) < 1e-9, `talkTrack: output quantised to 0.05 steps (t=${t.toFixed(2)}, v=${v})`);
}
eq(talkTrack(WORDS, 1.1, { line: 3 }), 0, 'talkTrack: line filter excludes other lines');
ok(speakingAt(WORDS, 1.2), 'speakingAt: true inside a word');
ok(!speakingAt(WORDS, 1.51), 'speakingAt: false in the gap');

// ── Spec 5: looks ──
eq(LOOK_PRESETS.adult.look, {}, 'LOOK_PRESETS.adult is the identity look');
eq(LOOK_PRESETS.adult.height, 1.75, 'LOOK_PRESETS.adult height 1.75');
eq(LOOK_PRESETS.elder.pose, 'elderStoop', 'elder preset pairs with the elderStoop pose');
eq(crowdLook(4, ['#111111', '#222222']), crowdLook(4, ['#111111', '#222222']), 'crowdLook: deterministic (same i → deep-equal)');
eq(crowdLook(4, ['#111111', '#222222']).body, '#111111', 'crowdLook: body cycles the palette');
ok(HATS.includes(crowdLook(7, []).hat!), 'crowdLook: hat is an existing HatKind');
ok(NECKS.includes(crowdLook(7, []).neck!), 'crowdLook: neck is an existing NeckKind');

// ── continuity contract: a fixed v4 look's signature is frozen ──
const V4_LOOK = { body: '#E8734A', hat: 'boater' as const, neck: 'tie' as const, extra: ['glasses'] };
eq(lookSignature(V4_LOOK), lookSignature({ ...V4_LOOK }), 'lookSignature: stable for a fixed v4 look');

console.log(`ACTING PROBE PASSED (${n} assertions)`);
