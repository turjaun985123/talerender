# A4 — Animation, Camera & Shot Systems Inventory (Tall Tale v5.0)

Seat A4, TaleRender Milestone 0 audit. Read-only inventory of:

| File | Lines | Role |
|---|---|---|
| `template/src/core/motion.ts` | 540 | easing/timing/follow-through, camera rigs, acting cues |
| `template/src/core/poses.ts` | 58 | pose vocabulary (data) |
| `template/src/core/shots.ts` | 226 | shot grammar, `shoot()` orchestrator, named-shot catalog |
| `template/src/core/timing.ts` | 26 | narration-clock data + word/line anchors |
| `template/src/core/library.ts` | 31 | library metadata types (types only, zero runtime) |
| `template/src/shots.ts` | 389 | the shipped example film ("The Girl Who Was Mailed to Grandma", 21 shots) — a *project* file, audited as the reference consumer of the five core files |

All citations are `file:line` against the v5.0 tree at `~/workspace/custom-skills/tall-tale-shorts/template/src/`.

---

## 1. The animation evaluation model

### 1.1 The model in one paragraph

Evaluation is **pure, closed-form, and timestamp-driven — there is no simulation anywhere in these files.** The engine's stated law is at the top of `core/motion.ts:3-9`: *"draw(t) in this engine is a PURE function of film time t… NO stateful integrators, NO closures that mutate across frames, NO 'previous value' anywhere. Follow-through and lag are ANALYTIC."* A timestamp becomes character state by composition:

1. **Anchors first.** All action times derive from the narration clock: `timing.ts` loads `data/lines.json` + `data/words.json` into the module singleton `T` (`core/timing.ts:9`, populated by `loadTiming()` at `core/timing.ts:12`), and exposes pure lookups — `L(i)` line start (`:18`), `Le(i)` line end (`:19`), `Wd(word, line?)` first word-start matching a normalised prefix, throws if absent (`:20-25`), `lineWords(i)` (`:26`). Shot cut points are `cut(i) = L(i) − 0.06` (`core/shots.ts:36`).
2. **Shot dispatch.** A shot is `{ id, at, note, draw(lt, d, t, blit) }` (`core/shots.ts:26`): the host calls `draw` with shot-local time `lt`, shot duration `d`, and absolute film time `t`. Each `draw` computes all character state for that instant from scratch and calls `toon.set({...})` — state is *assigned*, never accumulated.
3. **Curves map time → scalars.** Easing/progress functions (§1.2) turn `lt`/`t` into normalised progress `u` or `k`; `lerp`/`seg` (from `core/lib.ts`, §1.3) map progress onto angles, positions, scales.
4. **State bundles compose by spread.** A character's per-frame state is a plain `Partial<ToonState>` object assembled as `{ ...POSES.x, ...reactCue(...), mood: mc.mood, blink: max(...), phase: walkPhase(dist), ... }` — poses (§5), acting cues (§1.4) and motion helpers all return plain data; `Toon#set` (in `core/toon.ts`, outside this seat) consumes it.
5. **Walk cycles are distance-driven, not time-driven.** `walkPhase(distance, stride = 0.62) = distance / stride × π` (`core/motion.ts:229-231`), explicitly so feet cannot slide when speed and stride disagree; `walkBob(phase, amp = 6) = amp × (1 − |sin(phase)|)` (`:242-244`).

Because capture/QA may draw arbitrary times in arbitrary order, every helper is seek-safe by construction: the same `(t, anchors, seeds)` always yields the same state (proven in §3).

### 1.2 Easing / curve inventory — every easing function

**Defined in `core/motion.ts`** (all take normalised `u ∈ [0,1]` and clamp it themselves, unless noted):

| Function | Line | Formula / behaviour |
|---|---|---|
| `smootherstep(u)` | `motion.ts:30` | Quintic smootherstep `u³(u(6u−15)+10)`; zero slope at both ends; the house default ease |
| `camK(lt, d, hold = 0.1)` | `motion.ts:42` | Camera progress: holds 0 for the first `hold` fraction of the shot, `smootherstep` through the middle, holds 1 for the last `hold` fraction (hold clamped ≤ 0.45) |
| `anticipate(u, dip = 0.18)` | `motion.ts:57` | Ease-back anticipation: descends to `−dip` over `u ∈ [0, 0.3]`, then smootherstep leg to 1; returns negative values early *by design* |
| `overshoot(u, c = 1.35)` | `motion.ts:71` | easeOutBack variant: `1 + (c+1)(u−1)³ + c(u−1)²`; house `c = 1.35` (gentler than lib's `eob`) |
| `snap(u)` | `motion.ts:82` | easeOutQuart strike: `1 − (1−u)⁴` |
| `softLand(u)` | `motion.ts:94` | Gentle ease-out: `1 − (1−u)⁵`, no overshoot |
| `settle(t, t0, freq = 6, decay = 5)` | `motion.ts:111` | **Follow-through channel**, not a u-curve: damped oscillation after event `t0`: `sin((t−t0)·freq·2π) · exp(−(t−t0)·decay)`, 0 for `t ≤ t0`. Analytic — no integration |
| `springStep(t, t0, {freq = 2.2, decay = 3.5})` | `motion.ts:129` | Analytic underdamped step response `1 − exp(−decay·dt)(cos(w·dt) + (decay/w)·sin(w·dt))`, `w = 2π·freq`; 0 before `t0`, rings to exactly 1 |

**Interpolation / track primitives in `core/motion.ts`:**

| Function | Line | Behaviour |
|---|---|---|
| `arc(p0, p1, lift, u)` | `motion.ts:144` | Quadratic Bézier between 2D points; control point = midpoint raised by `lift` in −y (canvas space) |
| `arcV(a, b, lift, u, up = 'y')` | `motion.ts:162` | Same for plain `{x,y,z}` objects (Y-up world space); returns a new object, inputs unmutated |
| `poseTrack(keys, t)` | `motion.ts:189` | **Hold-move-hold** keyframe track. Keys `[time, value, moveSecs?, curve?]` (`PoseKey`, `:188`); the move *into* a key occupies the `moveSecs` (default 0.35) *ending* at that key's time, using that key's curve (default `smootherstep`); holds the first/last value outside the key range |
| `beatStart(wordTime, framesEarly = 4, fps = 30)` | `motion.ts:213` | `wordTime − framesEarly/fps` — starts an action so its peak lands on the spoken word |

**Inherited from `core/lib.ts`** (imported by motion.ts at `:22` and used pervasively by the example film; lib.ts itself is outside this seat but these five are part of the de-facto easing vocabulary):

| Function | Line (lib.ts) | Formula |
|---|---|---|
| `clamp(v, a = 0, b = 1)` | `lib.ts:19` | `max(a, min(b, v))` |
| `lerp(a, b, t)` | `lib.ts:20` | `a + (b−a)·t` |
| `seg(t, a, b)` | `lib.ts:21` | Normalised segment progress `clamp((t−a)/(b−a))` — the standard way raw time becomes `u` |
| `eo(t)` | `lib.ts:22` | easeOutCubic `1 − (1−t)³` |
| `eio(t)` | `lib.ts:23` | easeInOutCubic (4t³ / mirrored) |
| `eob(t)` | `lib.ts:24` | easeOutBack, classic constants `c1 = 1.70158, c3 = c1 + 1` |
| `rnd(i)` | `lib.ts:25` | **Seeded hash, not randomness:** `frac(sin(i·127.1 + 311.7) · 43758.5453)` — see §3 |

There are **no other easing functions** in the six audited files, and no keyframe/cubic-bezier *spline* system: all interpolation is scalar easing + `lerp`, 2D/3D quadratic Bézier (`arc`/`arcV`), and `poseTrack`'s piecewise hold-move-hold.

### 1.3 Where interpolation actually happens

- Scalar channels (arm angles, lean, bob, scale…): `lerp(a, b, curve(seg(...)))` inline in each shot's `draw` (e.g. template `shots.ts:206, 215, 259`).
- Camera position/look: `lerpV` component-wise lerp on plain vectors (`motion.ts:324-326`), or `THREE.Vector3.clone().lerp` inside `createShoot`'s `mix` (`core/shots.ts:56`) and the example's own `mix` (`shots.ts:19`).
- Discrete channels (mood, legs, held prop) are **switched, never interpolated** — faces are discrete drawings; mood changes are masked swaps (§1.4).

### 1.4 Acting cues (in `core/motion.ts`) — the word-anchored layer

| Function | Line | Behaviour |
|---|---|---|
| `blinkDirector(seed = 0)` | `motion.ts:261` | Returns a pure `blink(t)` (1 while eyes shut, 0.12 s width). Interval pattern `BLINK_IV = [2.7, 3.9, 3.1, 4.4, 2.9, 3.6]` (`motion.ts:248`) is rotated by the seed (`startIdx = floor(seed·7) mod 6`, `:264`) and the clock offset by `seed × 1.37` s, so cast members never blink in sync. All loop state is local to the call (`:271-274`) |
| `blinkAt(t, beats)` | `motion.ts:286` | 1 if `t` is within 0.12 s after any listed beat time (deliberate blink on a thought beat) |
| `moodCue(keys, t)` | `motion.ts:438` | Masked mood track over keys `[time, mood, maskSecs?]` (`MoodKey`, `:423`). Returns `{ mood, blink, squash, headTilt }` (`MoodCue`, `:424`): mood = last key ≤ t; around each later key a mask window (default 0.16 s) where `blink` is a triangle peaking at exactly 1 *at* the swap instant, `squash` dips to −0.02, `headTilt` +2°. Sorts a copy of the keys (`:440`) — input never mutated |
| `reactCue(t, tHit, opts)` | `motion.ts:474` | The double-take, anchored on a word time. Returns a `Partial<ToonState>` timeline relative to `dt = t − tHit`: gaze drifts away for `dt ∈ [−0.10, 0)`, `{ blink: 1 }` masked flip for `dt ∈ [0, 0.06)`, reaction hold (mood default `'shock'`, gaze snap, squash ringing via `settle(t, tHit, 2.7, 5.5)`) after; `takes: 2` adds a second gaze flick at `dt ∈ [0.34, 0.46]`; `hold` defaults to ∞ |
| `crowdReact(t, tHit, seed, opts)` | `motion.ts:498` | `reactCue` delayed by `rnd(seed) × 0.18` s with squash × 0.6 — a crowd ripples instead of snapping in unison; the only consumer of `rnd()` in the audited files (`:499`) |
| `talkTrack(words, t, opts)` | `motion.ts:520` | Lip-sync-lite from word *timings* only: inside a word `[s, e]`, syllable pulses at one per `rate` (default 0.14 s), `gain` (default 0.85) × `|sin(π·u·beats)|` × `sin(π·u)^0.5` envelope, output quantised to 0.05 steps; 0 in gaps. Optional `line` filter |
| `speakingAt(words, t)` | `motion.ts:537` | True while `t` is inside any word's `[s, e]` |

### 1.5 The example film's local helpers

The shipped example defines two private helpers instead of using `core/motion.ts` equivalents (`shots.ts:20-21`): `talk(t, on)` = `0.25 + 0.75·|sin(13t)|·(0.6 + 0.4·sin(5.3t))`, and `blink(t, off)` = `((t + off) mod 3.3) < 0.12`. Both are pure functions of `t`. (A migration note, not a defect: two blink/talk idioms exist — the example's fixed-period ones and motion.ts's seeded ones.)

---

## 2. The camera system

### 2.1 The frame vocabulary: `RigFrame`

All camera motion in the core is expressed as a **`RigFrame`** — a plain data triple defined at `core/motion.ts:322`:

```ts
export interface RigFrame { pos: Vec3Like; look: Vec3Like; fov?: number }
```

`pos` = camera position, `look` = aim point, `fov` present only when the move changes field of view. Vectors are **`Vec3Like` — a plain `{ x, y, z }` interface defined at `core/motion.ts:161`** — deliberately *not* `THREE.Vector3`. Rigs are pure: *"same k in, same frame out, inputs never mutated, every returned object newly allocated"* (`motion.ts:302-306`). House law: rigs are always driven by a `camK`-eased `k`, never raw shot progress (`motion.ts:308-311`).

### 2.2 Camera rigs (in `core/motion.ts`)

| Rig | Line | Parameters / behaviour |
|---|---|---|
| `dollyZoom(from, to, look, k, fov0, fov1)` | `motion.ts:340` | Vertigo pair: pos lerps from→to, fov lerps fov0→fov1, look fixed (copy returned) |
| `orbit(centre, radius, height, a0, a1, k, lookAt?)` | `motion.ts:361` | Angle lerps a0→a1 (radians); `pos = centre + (sin a · radius, height, cos a · radius)` — `height` is absolute world Y; look = `lookAt ?? centre` |
| `crane(from, to, lookFrom, lookTo, k)` | `motion.ts:381` | The general move: pos and look both lerp linearly on k — *"exactly the from/to + at/at2 pattern shots already hand-roll, named"* |
| `trackAhead(subjFrom, subjTo, offset, lead, k)` | `motion.ts:399` | Tracking with look-ahead: camera rides at `subj(k) + offset` (world axes), aims at `subj(clamp(k + lead))`; lead ≈ 0.08–0.15 typical |

### 2.3 `shoot()` — where a RigFrame becomes a real camera (the only three.js camera code in the core files)

`createShoot(E, cam, { baseFov = 36 })` (`core/shots.ts:54-67`) builds the `ShootFn` every shot calls. Its type (`core/shots.ts:38-42`) is the three.js boundary:

```ts
export type ShootFn = (
  scene: THREE.Scene, people: Toon[], from: THREE.Vector3, to: THREE.Vector3,
  at: THREE.Vector3, at2: THREE.Vector3 | null, k: number, look: ShotLook,
  focus?: number,
) => void;
```

Per call, in order (`core/shots.ts:57-66`): set `cam.fov` to `look.fov ?? baseFov` (36 — the house default, *not* the engine camera factory's 30) and `updateProjectionMatrix()` if changed; `cam.position.copy(from.clone().lerp(to, k))`; target = `at2 ? at.clone().lerp(at2, k) : at`; `cam.lookAt(target)`; if `look.roll`, `cam.rotateZ(look.roll)` (opt-in Dutch angle, `ShotLook` at `core/shots.ts:32`); `people.forEach(p => p.face(cam))` — every character re-yaws to the camera; `E.render(scene, cam, { ...look, focus: focus ?? cam.position.distanceTo(target) })`.

Callers convert a `RigFrame` to `THREE.Vector3`s at the call site (the pattern is documented at `core/shots.ts:9-13`: `shoot(scene, cast, V3(f.pos), V3(f.pos), V3(f.look), null, 1, { ...LOOK, ...(f.fov ? { fov: f.fov } : {}) })`).

Supporting helpers in `core/shots.ts`:

- `anchored(t, wordAt, dur = 0.3, curve = snap)` (`:73-75`) — progress that is 0 until `wordAt − dur`, rises on `curve`, and arrives at exactly 1 *on* the word.
- `headY(height, kid = false)` (`:80-82`) — world height of the head centre: `(kid ? 634 : 822) × (height/0.96) / 1024` stage units.

### 2.4 The named-shot catalog — every entry, with parameters

12 named helpers in `core/shots.ts`, each returning a `RigFrame` (except `dutch`, which returns a look fragment), and a machine-readable catalog `SHOT_CATALOG` at `core/shots.ts:213-226` (`ShotCatalogEntry { name, helper, beat, fov: [lo,hi], dist: [lo,hi], anchor }`, `:212`) from which the docs/preview sheet are generated. FOV/dist below are the catalog's own ranges (metres, camera→subject); behavioural parameters are from each helper's signature + doc comment.

| # | Catalog name | Helper (line) | Anchor | FOV | Dist (m) | Parameters / contract |
|---|---|---|---|---|---|---|
| 1 | hook-smash-in | `smashIn(from, to, look, lt, landSecs = 0.5)` (`shots.ts:90`) | start | 36–44 | 3.5–9 | Crash push: `crane` driven by `snap(lt/landSecs)` internally; lands by `landSecs` then *holds*; typical travel 7–9 m → 3.5–4.5 m; at most twice per film |
| 2 | slow-push | `slowPush(from, to, look, k)` (`:99`) | — | 36–44 | 4–5.5 | `crane` passthrough; caller passes `k = camK(lt, d, 0.15)`; end/start distance ratio 0.70–0.85; shot ≥ 2 s |
| 3 | whip-pan | `whipPan(centre, radius, height, a0, a1, lt, dur = 0.25)` (`:110`) | start | 44–55 | 4–7 | Orbit geometry with angle driven internally by `snap(lt/dur)`; swing 0.5–1.2 rad completes inside `dur`, then holds; radius 4–7 m |
| 4 | dolly-reveal | `reveal(from, to, lookFrom, lookTo, t, wordAt, dur = 0.55)` (`:120`) | word | 36–50 | 3–5 | `crane` on `anchored(t, wordAt, dur, smootherstep)`; subject fully revealed exactly on its word; ends 3–5 m with ≥ 1.2 m lateral travel |
| 5 | punch-in | `punchIn(from, to, at, t, wordAt, dur = 0.28)` (`:128`) | word | 36 (fixed) | 2.2–5 | `crane` on `anchored(t, wordAt, dur, snap)`; end = 0.55–0.70 × start distance |
| 6 | over-shoulder | `overShoulder(fg, target, opts)` (`:142`) | — | 44–55 | 2.5–5 (fg→target) | Static frame, no k/t. Defaults: `side = 1`, `back = 1.15` (camera 1.0–1.6 m behind fg figure), `across = 0.55`, `camY = 1.35`, `lookY = 1.3`. Directions computed from the fg→target axis. Documented limitation: a Toon has no back-of-head — the fg figure must be excluded from `shoot()`'s people list or it will `face()` the lens |
| 7 | dutch | `dutch(deg = 5)` (`:158`) | — | 36–44 | — (0,0 in catalog) | Returns `{ roll: deg·π/180 }` to spread into the shot look; working range 3–7°; requires createShoot's roll branch |
| 8 | orbit | `orbitShot(centre, k, opts)` (`:166`) | — | 36–44 | 4.5–7 | Defaults: `radius = 5.5`, `height = 2.2`, `swing = 0.7` rad, `from = −swing/2`; delegates to `orbit()`; \|swing\| ≤ 1.0 rad |
| 9 | crane-down | `craneDown(from, to, lookFrom, lookTo, k)` (`:173`) | — | 40–48 | 4–9 | `crane` passthrough; start height 4–7 m → end 1.5–2.2 m; `k = camK` |
| 10 | crane-up | `craneUp(from, to, lookFrom, lookTo, k)` (`:179`) | — | 36–44 | 4–9 | `crane` passthrough, reverse of crane-down |
| 11 | reaction-cut-in | `cutIn(subj, faceY, camPos, opts)` (`:189`) | — | 36 (44 two-face) | 2.2–3.0 | A cut, not a move: static frame at `dist` (default 2.6) from subject along the camPos→subj direction, at `faceY + 0.05`, looking at `(subj.x, faceY, subj.z)`; returns `fov` (default 36) in the RigFrame |
| 12 | wide-gag | `wideGag(centre, camPos, k, opts)` (`:202`) | — | 55–65 | 7–11 | Deliberately near-static `crane` from camPos to a point 4% closer to centre (×0.96); returns `fov` default 58 |

Unanchored helpers never ease internally (caller supplies `camK` k); the exceptions — `smashIn`, `whipPan`, and the word-anchored `reveal`/`punchIn` — carry their timing inside, as documented per entry (`core/shots.ts:14-17`).

### 2.5 Three.js camera usage inside these files

- `core/shots.ts`: type-only import (`import type * as THREE`, `:19`) used in `ShootFn`/`createShoot` signatures; the runtime camera operations are `cam.fov` + `updateProjectionMatrix()`, `cam.position.copy`, `cam.lookAt`, `cam.rotateZ`, `cam.position.distanceTo` (`:54-66`) and `Vector3.clone().lerp` (`:56`). No camera is *constructed* here.
- `template/src/shots.ts` (example): runtime `import * as THREE` (`:5`); constructs cameras directly — the main camera via the engine factory `camera(36)` (`:40`, engine.ts's helper), and a `new THREE.PerspectiveCamera(42, 4/3, 0.1, 800)` for snapshot/view textures (`:55`); `V3()` factory = `new THREE.Vector3` (`:18`); `localToWorld` for grandma's-house coordinates (`:80`).
- `core/motion.ts`, `core/poses.ts`, `core/timing.ts`, `core/library.ts`: **no three.js camera usage at all** (motion.ts has no three import of any kind).

---

## 3. Determinism audit

**Verdict: PROVEN — within the six audited files, evaluation is a pure function of (t, anchors, seeds). No source of nondeterminism was found.**

Evidence:

1. **Forbidden-API sweep is empty.** `grep -n 'Math\.random\|Date\.now\|performance\.now\|new Date\|crypto\.'` over all six files returns **no matches**. There is no wall-clock read, no RNG, no entropy source anywhere in the animation/camera layer.
2. **The only "random-looking" function is a seeded hash.** `rnd(i)` (`core/lib.ts:25`, imported by motion.ts) is `frac(sin(i·127.1 + 311.7) · 43758.5453)` — a pure function of its integer argument. Its single use in the audited files is `crowdReact`'s per-member delay `rnd(seed) × 0.18` (`motion.ts:499`).
3. **Seed mechanisms are explicit and frozen at construction.** `blinkDirector(seed)` derives both its pattern rotation (`floor(seed·7) mod 6`, `motion.ts:264`) and its clock offset (`seed × 1.37`, `:263`) from the seed; the returned closure captures only those two derived constants and the frozen `BLINK_IV` table (`:248`) — its scan loop keeps all state in locals (`:271-274`).
4. **No hidden state.** `poseTrack`/`moodCue`/`talkTrack`/`reactCue` read only their arguments; `moodCue` sorts a *copy* of its keys (`motion.ts:440`) with a numeric comparator (stable, input-order-independent result for distinct times); rigs allocate fresh output objects and never mutate inputs (`motion.ts:324-326`, `:340-406`).
5. **Iteration order is deterministic.** All iteration is over arrays (index order) — `poseTrack`'s key scan, `talkTrack`'s word scan (first matching word wins, `:524-534`), `blinkAt`'s beat scan. The one object-key enumeration, `Object.keys(POSES)` → `POSE_NAMES` (`poses.ts:33`) and `Object.entries(LOOK)` for cast registration (example `shots.ts:32`), relies on ES string-key insertion order, which is spec-defined and stable for a fixed source file.
6. **The anchors are data, not clocks.** `timing.ts`'s `L/Le/Wd/lineWords` are pure lookups over the loaded `T` arrays. Caveat (loading, not evaluation): `T` is a module-level mutable singleton populated by the async `loadTiming()` (`timing.ts:9-16`), which uses the browser `fetch` API — evaluation is pure *after* load; the native port must inject the same JSON as data instead.

**Boundary caveats (outside the pure core, stated honestly):**

- **The example film's `draw` functions mutate three.js scene state** — `toon.set()`, `restyle()` guarded by current-value checks (e.g. `shots.ts:110, 138`), `group.visible` flags, `scale.set()`, texture `offset.x` (`shots.ts:290`), `tr.update(t)` / `chicks.update(t)` (props.ts updaters, outside this seat). The *math* is a pure function of `(lt, d, t)`; the *application* is stateful but convergent — guards compare against current values, so re-drawing a timestamp re-derives the same state rather than accumulating. TaleRender's runtime must preserve this evaluate-then-apply shape (or make application idempotent) to keep arbitrary-order capture correct.
- **Floating-point portability note:** all curves are `f64` `Math.sin/pow/exp` compositions. JS engines and Rust's `libm` may differ in the last ulp on transcendentals; visually irrelevant, but byte-identical frame hashes across a JS↔Rust boundary should not be promised without a tolerance.

---

## 4. Three.js coupling & portability classification

What each module actually depends on:

| Module | three.js math types | Other three.js | Verdict |
|---|---|---|---|
| `core/motion.ts` | **None.** No three import at all (grep: zero `THREE.` / `from 'three'`). Own `Vec3Like` plain interface (`:161`). Only imports: `clamp/lerp/rnd` from lib (`:22`) and *type-only* `Mood, ToonState` from toon (`:23`) | None | **PURE-TS** — ports mechanically |
| `core/poses.ts` | None. Data literals typed as `Partial<ToonState>` (type-only import, `:6`) + `LibraryItemMeta` (type-only, `:7`) | None | **PURE-TS** — pure data; ports as a data registry |
| `core/timing.ts` | None | None (no imports at all). Loader uses browser `fetch` (`:13`) | **PURE-TS** for evaluation; the *loader* is browser-coupled — native port supplies lines/words JSON directly |
| `core/library.ts` | None | None — *"Types only — no runtime imports, no behaviour"* (`:1-2`) | **PURE-TS** — types only |
| `core/shots.ts` | `THREE.Vector3` in `ShootFn`/`createShoot` signatures and the internal `mix` lerp (`:38-42`, `:56`) | `THREE.Scene`, `THREE.PerspectiveCamera` (all **type-only**, `import type * as THREE`, `:19`); runtime camera ops `lookAt/rotateZ/updateProjectionMatrix` delegated through the typed camera object; render delegated to `Engine.render` | **SPLIT:** the 12 catalog helpers + `anchored`/`cut`/`headY` are **PURE-TS** (Vec3Like/RigFrame in-out, no three values); `createShoot`/`ShootFn` are **THREE-COUPLED** — the single seam where the compat layer must provide Scene/Vector3/PerspectiveCamera semantics |
| `template/src/shots.ts` (example film) | Runtime `THREE.Vector3` everywhere (`V3` factory `:18`, `mix` `:19`, `localToWorld` `:80`) | Runtime: `THREE.Scene`, `THREE.Group[]` (`:52`), `THREE.PerspectiveCamera` construction (`:55`), `THREE.Texture`, `THREE.RepeatWrapping` (`:55`). **Canvas 2D:** imports `ctx` from lib (`:6`) and draws post-`blit()` overlay stamps with the full CanvasRenderingContext2D API (save/translate/rotate/scale/fillRect/fillText/roundRect) at `:349-353` and `:380-383` | **THREE-COUPLED + CANVAS-COUPLED** — but this is a *project* file, not engine code: it ports by re-authoring against the compat layer + native 2D layer, not by mechanical translation |

Summary for the migration plan: **the entire animation/camera *evaluation* layer (motion.ts, poses.ts, timing.ts, library.ts, and shots.ts's catalog half) is plain-number TS with zero three.js values** — `Vec3Like`/`RigFrame` were designed as the three-free boundary. Three.js coupling concentrates in exactly one core function (`createShoot`) plus the project files that consume it.

---

## 5. Pose & mood vocabularies (native-side registries)

### 5.1 Poses — 17, defined in `core/poses.ts`

`POSES` (`poses.ts:9-28`), typed `Record<string, Partial<ToonState>>` — pure data bundles of arm angles `[shoulder°, elbow°]`, lean, bob, headTilt/headTurn, look, squash, sweat and a mood name; *"poses contain no functions, so they are safe in the repaint key"* (`poses.ts:4-5`). Runtime name list `POSE_NAMES = Object.keys(POSES)` (`:33`); per-pose library metadata `POSES_LIBRARY` (`:40-58`) in the shared `LibraryItemMeta` format (§5.3).

The 17: `point`, `wave`, `shrug`, `think`, `celebrate`, `mourn`, `explain`, `listen` (pre-5.0) and `react`, `facepalm`, `sneak`, `exhausted`, `dazed`, `boast`, `salute`, `read`, `elderStoop` (added in v5, marked `'5.0'` in their meta rows, `poses.ts:47-57`).

### 5.2 Moods — 19, defined in `core/toon.ts` (outside this seat; cited because the audited files consume the type)

The `Mood` type is imported type-only by `motion.ts:23` and used as data by `poses.ts` and `moodCue`/`reactCue`. The vocabulary lives in `core/toon.ts:45`:

```ts
export type Mood = 'neutral' | 'smug' | 'happy' | 'laugh' | 'shock' | 'worried' | 'angry' | 'sad'
  | 'sly' | 'proud' | 'tired' | 'dizzy' | 'determined' | 'embarrassed' | 'awe' | 'suspicious'
  | 'relieved' | 'puzzled' | 'deadpan';
```

19 moods, with the runtime list `MOODS` (`core/toon.ts:53`) and metadata `MOODS_LIBRARY` (`core/toon.ts:775`). Moods are **discrete face drawings, never interpolated** — transitions happen only via `moodCue`'s blink-masked swap (§1.4). Default mood in `moodCue`'s empty-keys fallback and `reactCue`: `'neutral'` / `'shock'` respectively (`motion.ts:439, 475`).

### 5.3 Registry format

Both vocabularies already ship with a registry schema: `LibraryItemMeta` (`core/library.ts:16-27`) — `{ id (dotted, e.g. 'pose.react'), kind, name, snippet, captionSafe, preview, optIn, since }`, with `LibraryKind` (`:14-15`) covering `'pose' | 'mood' | 'hat' | 'neck' | 'extra' | 'look' | 'world' | 'prop' | 'shot' | 'sound' | 'caption'`. The shot catalog has its own parallel registry (`ShotCatalogEntry`, `core/shots.ts:212`). A native port can ingest `POSES`/`POSES_LIBRARY`, `MOODS`/`MOODS_LIBRARY` and `SHOT_CATALOG` as data tables unchanged — all three are plain JSON-shaped literals.

---

## Appendix — file/dependency map for the port

```
lib.ts ──clamp/lerp/seg/eo/eio/eob/rnd──► motion.ts ──► (rigs, cues) ──► core/shots.ts ──► src/shots.ts (film)
timing.ts ──L/Le/Wd anchors────────────► core/shots.ts (cut)              ▲ type-only: toon.ts (Mood/ToonState)
library.ts ──LibraryItemMeta (types)───► poses.ts                         ▲ type-only + runtime: three (ShootFn only)
```

- Mechanical port (plain numbers in/out): **motion.ts, poses.ts, timing.ts (evaluation), library.ts, shots.ts catalog helpers.**
- Compat-layer seam: **`createShoot`/`ShootFn`** (one function) + the `Engine.render` call it wraps.
- Re-authoring (project content): **template/src/shots.ts** and its siblings — they consume the above through THREE objects and a 2D canvas overlay pass.
