# FORMAT15-DRAFT-W31 — paste-ready scene-format §15 (prefabs + clips)

**Seat:** TaleRender Wave 2, W31 (Batch 4 — format fold-in draft).
**Date:** 2026-11-14. **Status:** DRAFT for the lead. This file is
not part of the spec; the text between the two `---` rules below
is the complete §15 the lead pastes into `docs/scene-format.md`
verbatim (immediately after §14). Nothing in
`docs/scene-format.md` was edited by this seat, and nothing was
committed.

**Sources folded in, semantics unchanged:** W14's prefab spec
(`FORMAT-EXT-PREFABS-W14.md`, validator
`tools/format/check_prefabs.py`, fixtures `scenes/fixtures/prefabs/`)
and W15's clips spec (`FORMAT-EXT-CLIPS-W15.md`, validator
`tools/format/check_clips.py`, fixtures `scenes/fixtures/clips/`).
Fixture transcripts and verification runs stay in those two
documents; §15 cites them instead of repeating them.

**Reconciliations applied (§15.9 of the paste text):** the four
open cross-spec items are stated there as decided rules — the
colour/step evaluator lane, the ease-direction bridge, wrap/
duration placement, and euler-vs-quaternion rotation — following
the lead's recorded decisions in `WAVE2-RECORD.md` §9 (items 1,
2, 6) and, for rotation, the surveyed scene data. Every
`anim.rs` identifier quoted in the paste text was verified
against `src/anim.rs` at HEAD: `Easing::{Linear, Step,
Smootherstep, EaseOutCubic, EaseInOutCubic, EaseOutBack}`,
`Easing::from_name`, `Easing::DEFAULT` (= `Smootherstep`),
`Wrap::{Clamp, Loop, PingPong}`, the trait `AnimValue` (impls
for `f32`, `[f32; 3]`, `Quat`), `Quat(pub [f32; 4])`,
`Key { t, value, ease }`, `Track::with_duration`,
`Clip::add_f32` / `add_vec3` / `add_quat`, `Clip::sample_f32` /
`sample_vec3` / `sample_quat`.

---

## 15. Format v2 extensions: prefabs and clips

Two additive extensions ride the existing format version
(`"version": 2`), under the §10 rule: neither adds a field to an
existing construct's meaning, neither changes an existing field,
and each one's fully processed form is an ordinary v2 document.
What they add is **reserved keys**: the top-level `"prefabs"`
and `"clips"` blocks, the node-level `"prefab"` key, `"pid"`
(meaningful only inside prefab templates), and the instance
fields `"clip"` / `"clipTime"` on nodes and on the `camera`,
`lights`, `post`, `fog` blocks. Prefabs (§15.2–15.5) are a
load-time macro over node subtrees. Clips (§15.6–15.8) are named
keyframe data evaluated at the file's `time` (§14.1). §15.1 is
the rule that makes both safe to reserve; §15.9 records the
reconciliation rulings where the two extensions meet the
evaluator (`src/anim.rs`); §15.10 is the adoption order.

### 15.1 The reservation guard (fail loud, never silently ignore)

A v2 loader that does not implement one of these extensions
must **hard-error, naming the unsupported extension**, when it
encounters that extension's reserved keys — a top-level
`"prefabs"` or `"clips"` block, a `"prefab"` key in a node
position, or a `"clip"` field on any node or block. Silently
dropping them is forbidden. This is a deliberate carve-out from
§1's "unknown fields are ignored", and it exists because §1's
rule is unsafe for exactly these keys: ignoring a `prefabs`
block while honouring its instances is incoherent, ignoring an
instance deletes scene content, and ignoring a `clip` field
silently freezes animated staging at its static values. The
precedent is already in the format: a v1 file carrying v2
fields is *rejected*, not reinterpreted (§14), and a v1 file
carrying `prefabs` or `clips` is rejected the same way.

Measured state of today's loader (`src/scenefile.rs`), so the
guard's work is named exactly: a top-level `"prefabs"` block is
never read — silently ignored, per §1 — while an instance node
already fails loud, because `walk_nodes` requires a `type` on
every node, but with a misleading missing-`type` message that
never mentions prefabs. The guard replaces both behaviours with
one named error per extension ("prefab extension not
supported", "clips extension not supported"). The guard applies
to every consumer that walks nodes, including `film.rs`'s own
node walk, whose behaviour must not be assumed to match the
still loader's. Until the guard and the corresponding
implementation land together (§15.10), the two reference
validators are the only consumers that understand these
extensions, and no production scene or film data may use them.

### 15.2 Prefabs: grammar

A **prefab** is a named, parameterised node subtree:

```jsonc
"prefabs": {
  "<name>": {
    "description": "optional, documentary, ignored",
    "params": { "<slot>": <declaration>, ... },   // optional
    "node": <node>                                // required
  }
}
```

- The block may appear **inline** in a scene file, and/or in a
  **library file** — a JSON object `{"format":
  "talerender-scene", "version": 2, "prefabs": {…}}` with no
  `nodes`, which is data for the scope and is never rendered. A
  consumer assembles one merged scope from the inline block
  plus every library it is given. **No shadowing:** a name
  defined by two sources is a hard error naming both sources —
  the same file must mean the same thing everywhere.
- A prefab has a **single root node**, so an instance always
  occupies exactly one node position. Multi-object sets are a
  `group` root (the base format's own idiom, §4.2).
- Prefab names are any non-empty JSON object key. Names carry
  no path semantics (no `/`, no imports); a library is included
  whole or not at all.

**Param declarations** are a closed key set — `type`
(required), `required` (optional boolean), `default`
(optional), `description` (optional, documentary):

```jsonc
"params": {
  "height":   { "type": "number", "required": true },
  "capColor": { "type": "color",  "default": "#C23B2E" },
  "label":    { "type": "string", "default": "" }
}
```

Every param is exactly one of: `required: true` (the instance
must supply it) or carrying a `default` (the instance may). A
required param with a default, and a param that is neither, are
both definition errors — a slot must always expand to a value.
A `default` must type-check against its own declaration and
must be **literal**: it may not contain a `$param` slot
anywhere inside it.

**Slots.** A slot is an object with exactly one key,
`{ "$param": "<name>" }`, standing for a **whole JSON value**
anywhere a value may appear inside the template subtree: as a
field value, as an array element (one component of a `size` or
`translate` vector), as a whole array/object, and as the value
of a nested instance's `params` entry (forwarding, §15.3).
Slots are lexically scoped — a slot inside prefab P's template
names a param of P, never of a caller or a nested prefab — and
a slot anywhere in a scene file outside a template is an
error. A slot may not be an object key, may not be the value of
`"prefab"` (instantiation targets are literal strings, so the
reference graph of §15.5 is static), and may not appear inside
a param declaration or a default. A slot-shaped object with
any additional key is not a slot; it is an ordinary object and
fails base validation downstream if it lands somewhere it
makes no sense.

**Instances.** An instance is an object in a node position
carrying `"prefab"`:

```jsonc
{ "prefab": "marker-post",
  "params":    { "height": 1.2, "postY": 0.6, "capY": 1.24,
                 "capColor": "#2E5FC2" },
  "overrides": { "cap": { "material": { "roughness": 0.35 } } },
  "transform": [ { "translate": [2.5, 0, -2] } ],
  "name": "east gate post" }
```

The key set is closed: `prefab` (required, literal string),
`params`, `overrides` (§15.4), `transform` (a base-format
transform list), `name`. **Any other key is a hard error** — a
deliberate departure from §1, mirroring §1's own other half
(unknown *values in discriminating positions* are already hard
errors): an instance is a *call*, and a misspelled key in a
call would otherwise be silently dropped while the render
proceeds with defaults. An instance has **no `"type"` key** —
`"prefab"` is its discriminator. Instances may appear anywhere
a node may appear (the top-level `nodes` array, `group`
children, and — after substitution — in `node`/`nodes` param
positions); they may not appear where the base format does not
take nodes (`attachments`, `geometry`, `material`, `textures`,
`lights` are not node positions). Slots, being value-level,
*may* appear inside those sub-objects within a template.

**`pid`.** Any node inside a template may carry
`"pid": "<local-id>"`, existing only so `overrides` can
address the node (§15.4). A `pid` is not an `id` — the base
format's `id` fields name entries in closed, loader-level
registries and are never touched by this extension. Pids are
scoped to their prefab's **direct scope** (the template
subtree, not descending into a nested instance's interior),
must be unique within that scope, and are **stripped during
expansion** — a `pid` never reaches the renderer, a saved file,
or another prefab.

### 15.3 Prefab params: typing, substitution, and the f32 law

Param types are the JSON shapes the base format already
carries — nothing is coerced, converted, or computed:

| type | value must be | notes |
|---|---|---|
| `number` | a JSON number (never a boolean) | subject to the f32 law below |
| `string` | a JSON string | verbatim |
| `color` | a string matching `#RRGGBB` | the base format's hex colour-spec string (§4.1); the `{hex, shade}` object form is an `object` param, not a `color` |
| `boolean` | `true` / `false` | |
| `vec3` | an array of exactly 3 numbers | positions, sizes, targets |
| `array` | any JSON array | element shape is the author's responsibility; the expanded result is base-validated |
| `object` | a JSON object that is not a slot and not an instance | e.g. a material or colour spec |
| `node` | one node object (`type` or `prefab` present) | enables wrapper prefabs |
| `nodes` | an array of node objects | enables container prefabs |

There is deliberately no `integer` type (JSON has one number
type), no enum type (closed value sets belong to the base
validators of the expanded output), and no expression type.

**Substitution is verbatim — no arithmetic, no string
interpolation, no expressions.** A slot is replaced by a deep
copy of the effective param value, bit for bit, with no
re-rounding; there is no operation in this extension that
could produce a new number. Consequently the §11 f32 data law
needs no extension and applies **after expansion, unchanged**:
a value that lands, after expansion, under `lights`, `camera`,
or a character node is subject to exactly the same exact-f32 /
source-literal rule as if it had been written inline, and
`tools/film/validate_film.py` enforces it on the expanded
document unchanged. The cost is stated plainly: substitution
cannot derive values — a template needing both a post's height
and its centre (`height / 2`) declares **two params**, staged
accordingly. The moment slots could compute, param values
would stop being "numbers exactly as the base format carries
them", the f32 law would need an expression semantics, and
expansion would stop being auditable by reading the file.
Authors who need derived staging compute it where staging is
authored — as they do today — and pass the results as params.

**Forwarding.** A nested instance inside a template may supply
a slot as a param value, forwarding the outer prefab's param
inward. The forwarded slot's declared type in the outer prefab
must match the inner param's declared type exactly, with one
widening: a `color` may feed a `string` (every colour is a
string; the reverse is false and is an error). A template must
be **self-contained**: a nested instance that omits an inner
required param is a definition error, not something a
scene-level caller can repair.

### 15.4 Prefab overrides and transforms

There are exactly two override channels, with different
mechanics on purpose: **params replace wholesale; overrides
deep-merge.**

**Params** (§15.2): the effective value is the instance's
supplied value if present, else the declared default. A
supplied `object` value is *not* merged into its default field
by field — a half-merged material spec is invisible in the
file and unauditable. Per-field variation of an object is
separate params, or belongs in `overrides`.

**Overrides** are pid-addressed partial node objects,
deep-merged onto the template node carrying that `pid`
**before** param substitution:

```
merge(dst, src):
  for each key k in src:
    if k is forbidden (below)           -> hard error
    if src[k] is null                   -> hard error (no deletion)
    if dst[k] is a $param slot          -> hard error
    if dst[k] and src[k] are both plain objects (neither a slot)
                                        -> merge(dst[k], src[k])
    else                                -> dst[k] = deep copy of src[k]
                                           (arrays and scalars replace
                                           wholesale; arrays are never
                                           concatenated or merged by index)
```

Arrays replace wholesale because the base format's arrays are
positional or ordered — vectors, transform op lists (§5
composes left to right), camera keys. An override may **add**
keys the template node does not carry (adding `roughness` to a
material that omitted it is legitimate restyling); added keys
are still subject to the forbidden list, and the merged result
is base-validated after expansion like any authored node.
Overrides address only the target prefab's direct scope — the
interior of a nested instance is not addressable from outside;
restyle inside a nested prefab by calling that prefab directly
or exposing a param.

**What may NOT be overridden** (at any depth inside the
partial object):

| Forbidden | Why |
|---|---|
| `type` | The node's discriminating value — changing it changes *what the node is*, not how it is staged; that is a different prefab. |
| `kind` (geometry, attachment) | Same: a closed-vocabulary discriminator. |
| `generator` (texture) | Same: names a code painter from a closed registry. |
| `id` | Registry identity (texture ids, code-ref ids); the loader resolves textures eagerly in array order before walking nodes, so renaming by instance would make that table context-dependent. |
| `prefab` | Re-targeting an instance would make the §15.5 reference graph dynamic and uncheckable. |
| `pid` | The addressing mechanism itself; re-labelling during the merge that uses the labels is incoherent. |
| `transform` | Transforms compose (§15.4 below); they are never replaced. |
| any key whose template value is a `$param` slot | Two channels writing one position is an ordering ambiguity; slot positions are written by `params` only. |
| deletion (a `null` value) | The base format has no null semantics anywhere; variants that omit a field are separate prefabs. |

**Transforms compose; they never replace.** A template root's
own `transform` is part of the prefab's authored staging and
is protected by the forbidden list. An instance's `transform`
is its placement in the parent space. When an instance carries
a `transform`, the expanded root is therefore wrapped:

```jsonc
{ "type": "group",
  "transform": <the instance's transform>,
  "children": [ <expanded template root, its own transform intact> ] }
```

World = parent × instance × root-local, by the base format's
ordinary group rule — no new transform semantics anywhere.
When an instance carries no `transform`, no wrapper is
emitted. An instance's `name`, when present, is set on the
outermost expanded node (the wrapper if there is one); it is
documentary, like the scene `name`, never an identity.

### 15.5 Prefab expansion and cycles (normative)

Expansion happens at load, before any node is walked. Prefabs
are a load-time macro: the renderer, the telemetry, and the
film driver only ever see expanded plain v2 nodes. A
conforming consumer performs these steps, in order:

1. **Assemble the scope** (§15.2): inline `prefabs` + all
   supplied libraries. Duplicate name → hard error.
2. **Validate the library eagerly** — every definition, used
   or not: declaration grammar, defaults literal and
   type-correct, every slot declared, no slot naming a prefab,
   pids unique per direct scope. (Eager validation follows the
   loader's existing precedent: textures are resolved eagerly,
   in array order, at load.)
3. **Build the reference graph** (definition → names its
   template instantiates, literally) and reject cycles (below).
4. **Validate every instance** in the scene and in templates:
   closed key set, known target, required params present, no
   undeclared params, values type-check (§15.3), overrides
   legal (§15.4).
5. **Expand** depth-first, in document order. For each
   instance: (a) effective params = defaults overridden
   wholesale by supplied params; (b) deep-copy the template
   root; (c) apply overrides by pid; (d) substitute every slot
   with a deep copy of its effective param value (the same
   param used twice yields two independent copies); (e)
   recursively expand instances in the resulting subtree
   (nested instances, and instances arriving via `node`/`nodes`
   params); (f) strip every `pid`; (g) wrap for the instance
   transform and set `name` (§15.4).
6. **Proceed with the base format exactly as if the expanded
   document had been authored**: the expanded tree contains no
   `prefab`, no `pid`, no `$param` — the consumer asserts
   this — and the top-level `prefabs` block is consumed and
   dropped (it does not propagate into any saved or derived
   document).

Document order is preserved throughout (instances expand in
place), so painter's order, texture resolution order, and
every other order-sensitive behaviour of the base format are
unaffected.

**Cycles.** The reference graph of step 3 is **static**:
instantiation targets are literal strings (§15.2 forbids slots
there), so every edge is visible without expanding anything. A
cycle — a prefab whose template transitively instantiates
itself — is a hard error, and the error prints the cycle path
(`loop-a -> loop-b -> loop-a`). Instance data cannot create a
cycle: a `node` param value is a finite JSON tree; it can
contain instances, but each is expanded against the acyclic
definition graph, so expansion terminates. A depth guard is
still required in implementations (the reference validator
uses 64) as belt-and-braces: a consumer that expands without
running the graph check first must fail, not hang or overflow,
on adversarial data.

### 15.6 Clips: the data model

A **clip** is a named set of keyframe tracks. Clips generalise
the §14.4 camera-key shape — the engine's one keyframe form
that is already data — to every channel the engine animates,
and let any number of nodes/blocks instance a clip with a time
mapping. All additions are additive: a top-level `clips`
block, plus `clip` / `clipTime` fields on things that already
exist. Numbers everywhere obey the §11 f32 data law.

```jsonc
"clips": {
  "<clip-name>": {                       // name: any JSON object key
    "duration": 5.907,                   // seconds, > 0, REQUIRED
    "tracks": [ Track, ... ],            // REQUIRED, non-empty
    "uses": [ "<clip-name>", ... ]       // OPTIONAL layering
  }
}
```

A **track**:

```jsonc
{ "target": { "node": "camera", "channel": "eye" },
  "keys":   [ Key, ... ] }               // REQUIRED, at least one key
```

`target.node` is one of the **reserved targets** `camera`,
`lights`, `post`, `fog` — the scene-level blocks — or the
**`name` of a node in the same file** (searched recursively
through `children`). Node names colliding with a reserved
target are rejected by the validator: the node would be
unaddressable.

A **key**:

```jsonc
{ "t": 2.6, "value": 0.4, "ease": "easeOutCubic" }
```

- `t` — clip-local seconds; `0 ≤ t ≤ duration`; strictly
  increasing within the track.
- `value` — shaped by the channel kind (§15.7): a number
  (scalar), `[x, y, z]` (vec3), `"#RRGGBB"` (colour), or an
  enum string (step channels: `mood`, `legs`). The field name
  is `value` — the name in the evaluator's `Key { t, value,
  ease }` and in the film's existing object-form keys alike.
- `ease` — OPTIONAL, default `smoother`. Names the curve of
  the segment **ending at this key** (the incoming segment) —
  the same placement as §14.4, where the loader reads the ease
  from the segment's end key, and the same direction as
  poseTrack, whose move is governed by the key being moved
  *into*. The first key's `ease` has no incoming segment and is
  ignored, exactly as the §14.4 loader ignores it. (The
  evaluator stores easing the other way round; the loader
  remap is a ruling, §15.9.)
- A single key is a constant track. Before the first key and
  after the last, the end values **hold** — the §14.4 camera
  law and poseTrack's hold behaviour agree on this.

**Layering (`uses`).** A clip may name other clips in `uses`;
an instance of the clip then applies its own tracks **plus**
the used clips' tracks, transitively. The `uses` graph must be
acyclic (self-use counts), every name must resolve, and the
layered set may not define the same target+channel twice (no
silent precedence).

**Instantiation.** Any node (recursively), and the `camera`,
`lights`, `post`, `fog` blocks, may carry:

```jsonc
"clip": "look-around",
"clipTime": { "offset": 0.5, "rate": 1.0, "loop": "ping-pong" }
```

- `clip` — the clip name; must be defined in the same file.
  The same clip may be instanced by many nodes — that reuse is
  the point of naming clips.
- `clipTime` — OPTIONAL; every field defaults: `offset`
  (default `0.0`, seconds added to the mapped time — the clip
  is entered `offset` seconds in), `rate` (default `1.0`, must
  be `> 0`, time scale), `loop` (default `"clamp"` — what
  happens outside `[0, duration]`; §15.8 gives the formulas).
  The three `loop` values are the kebab spellings of the
  evaluator's `Wrap::{Clamp, Loop, PingPong}`.
- **Conflict rule:** `camera` may not carry both `clip` and
  `keys` (one camera, one time source). Which form a shot uses
  is the author's choice; carrying both is data rot, and no
  precedence is defined.
- What an instance *means* against static fields: the clip's
  tracks **replace** the target's static value at evaluation
  time (the static `state` / `eye` / rig remains the at-rest
  staging and the t-before/after reference, exactly as §14.4's
  static `eye`/`target` remain the required base/fallback next
  to `keys`).

### 15.7 Clips: the channel vocabulary

Every channel below is a field the engine animates today,
cited to the code that consumes it (the survey lives in
`FORMAT-EXT-CLIPS-W15.md` §3). A channel not in these tables
is an unknown channel and fails validation. Kinds: **S**
scalar, **V** vec3, **C** colour hex, **Step** enum string (no
interpolation).

**camera** (target `"camera"`): `eye` (V), `target` (V),
`fovY` (S) — the §14.4 keys.

**lights** (target `"lights"`) — the §14.2 rig, carrying
post-atmosphere values per **R1**:

| channel | kind | | channel | kind |
|---|---|---|---|---|
| `key.intensity` | S | | `hemisphere.intensity` | S |
| `key.color` | C | | `hemisphere.sky` | C |
| `key.position` | V | | `hemisphere.ground` | C |
| `key.target` | V | | `envIntensity` | S |
| `rim.intensity` | S | | `points[i].intensity` | S |
| `rim.color` | C | | `points[i].color` | C |
| `rim.position` | V | | `points[i].position` | V |
| | | | `points[i].distance` / `points[i].decay` | S |

(`i` is the index into the file's `lights.points` array, as
written: `points[0].intensity`.) Not channels: `key.shadow*` /
`bias` — shadow *configuration*, never time-varied anywhere in
the engine.

**post** (target `"post"`):

| channel | kind | | channel | kind |
|---|---|---|---|---|
| `bokeh.focus` | S | | `grade.sat` | S |
| `bokeh.aperture` | S | | `grade.warm` | S |
| `bokeh.maxblur` | S | | `grade.exposure` | S |
| `bloom.threshold` | S | | `grade.vignette` | S |
| `bloom.strength` | S | | `grade.grain` | S |
| `bloom.radius` | S | | | |

**`grade.time` is deliberately NOT a channel.** **R3** pins
the GRADE grain time to 0 for the whole film (§14.5); a track
on it would animate grain away from Chrome on every frame. A
track targeting `post.grade.time` is rejected **by name**,
citing the ruling.

**fog** (target `"fog"`): `near` (S), `far` (S), `color` (C)
— the top-level fog block (§14.5).

**character nodes** (target = the node's `name`) — the §14.3
`state` fields. Pair fields split into components, because the
film's own data already tracks them separately:

| channel | kind | | channel | kind |
|---|---|---|---|---|
| `position` | V | | `lean` | S |
| `armL.0` / `armL.1` | S | | `bob` | S |
| `armR.0` / `armR.1` | S | | `squash` | S |
| `look.0` / `look.1` | S | | `headTilt` | S |
| `blink` | S | | `headTurn` | S |
| `talk` | S | | `camPitch` | S |
| `phase` | S | | `sitOn` | S |
| `height` | S | | `mood` | Step: the 17 strings of `Mood::from_str` |
| | | | `legs` | Step: `stand walk run sit jump` |

Strictness note: the loader's mood parsing silently falls back
to neutral on an unknown string; clip validation **fails** an
unknown `mood`/`legs` value instead — clips are new data, and
a typo'd mood should be caught at the gate, not rendered as
neutral. The same posture applies to unknown channels: a
channel name is a discriminating value, and §1 already
hard-errors on unknown discriminators.

**other nodes** (target = the node's `name`) — the transform
channels the film's data already keys: `translate` (V),
`scale` (V), `rotate.x` / `rotate.y` / `rotate.z` (S). Rotation
is spelled as euler scalars — see §15.9.

**Easing names.** The spellings are exactly the set the
evaluator's `Easing::from_name` accepts; the validator's list
is copied from it:

| name (canonical) | accepted aliases | engine curve |
|---|---|---|
| `smoother` (default) | `smootherstep` | the engine's smootherstep — camK, poseTrack, §14.4 camera keys |
| `linear` | — | raw lerp — the one ease name v2 already spells (§14.4) |
| `easeOutCubic` | `eo` | film `eo()` |
| `easeInOutCubic` | `eio` | film `eio()` |
| `easeOutBack` | `eob` | film `eob()` |
| `step` | — | hold — discrete channels; **required** on `mood`/`legs` keys |

### 15.8 Clips: evaluation semantics

For an instance at scene time `t`, with clip duration `D`:

```
raw = t * rate + offset
clamp:     tau = min(max(raw, 0), D)
loop:      tau = raw mod D            (result in [0, D))
ping-pong: m = raw mod 2D;  tau = m if m <= D else 2D - m
```

Per track at `tau`: find the bracketing keys `k0 (t0, value0)`,
`k1 (t1, value1)`; outside the key range, hold the end value.
Inside: `u = (tau - t0) / (t1 - t0)`, eased by `k1.ease` (the
arriving key's ease, §15.6), then:

- **scalar:** `value0 + (value1 - value0) * eased` (computed in
  f64, cast to f32 — the film's motion formulas are f64
  throughout; the evaluator's f32-native evaluation of the same
  law can differ by ~1 ulp, which is why evaluator adoption is
  A/B-gated per site, §15.10);
- **vec3:** componentwise, same law;
- **colour:** convert both hex values to linear (the engine's
  colour pipeline, §4.1/§8), lerp componentwise in linear, as
  the film's colour mixing does — never lerp sRGB bytes;
- **step:** the value of the latest key with `t <= tau` (no
  easing, whatever the key says — validation already requires
  `step`).

A clip is a pure function of `t` — no state, no frame history
— matching the format's stance that animation is `frame(t)`.
Loop modes do not change this (the formulas above are
closed-form).

**What must NOT migrate into clips** (the adoption map's
negative half; the positive half — `camera.keys`, poseTrack
channels via the conversion below, object-form node keys,
step data — is surveyed in `FORMAT-EXT-CLIPS-W15.md` §5.1):

- **`post.grade.time`** — R3 is a law, not data (§15.7).
- The fire-flicker formula over the **R1 basis 4.6** —
  a procedural law, not keys; baking it into keys would sample
  a continuous law into data and break the R1 cross-check.
  (Keying the fire's *level* envelope over a shot would be
  legitimate new authoring; it is not this film's data.)
- `walkPhase` / `walkBob` / `segLerp` position drivers —
  derived from travelled distance / shared drivers; they are
  functions of another channel's integral, not independent key
  data. The *position endpoints* migrate; the gait derivation
  stays code.
- `blink` directors, the `talk()` waveform, `settle()` shakes,
  `camPitchOf` — seeded or closed-form procedural systems; a
  seed is not a key table, and keying them would freeze today's
  sampling of a law.
- Pose bundles, stage swaps, and film-manifest segment timing
  (`startS`/`endS`, caption chunks) — pose data, structural
  swaps, and driver bookkeeping respectively, not scene
  animation.

**The poseTrack conversion (exact, mechanical).** A poseTrack
moves into key `k` over the `moveSecs` *before* `t_k` and
holds otherwise; a clip track interpolates across the *whole*
segment. So each poseTrack key `[t, v, m]` with `m > 0`
converts to **two** clip keys: a hold key `{t: t - m, v:
<previous value>}` (unless one already sits there) and the
arrival key `{t: t, v: v, ease: "smoother"}`. Keys with no
moveSecs convert the same way with the poseTrack default
`m = 0.35`. The converted track evaluates **identically** to
`pose_track()` at every `t`; this is a re-spelling, not a
re-timing.

### 15.9 Reconciliation rulings

Four points where the extensions meet the evaluator
(`src/anim.rs`) were left open by the source specs. They are
decided here, so the format text and the code agree from the
start.

**Colour and step channels get their own evaluator lanes.**
*Rationale: the evaluator's `AnimValue` lanes at the §15
landing cover `f32`, `[f32; 3]`, and `Quat` — scalar and vec3
channels map onto them directly, but colour and enum-string
values must never route through numeric interpolation.*
Decided: the evaluator gains a colour lane, `Color`, whose
values are **linear-space `[f32; 3]`**. The loader converts
each colour key's authored `#RRGGBB` to linear by the format's
existing hex→linear law (§15.8) **before** the track carries
it, so a `Color` track never holds an sRGB value; sampling
interpolates componentwise in linear, and consumers receive
linear values exactly as the film's folded light colours are
carried today. Step channels (`mood`, `legs`) are sampled by a
**step sampling mode**: hold the leaving key's value, take the
latest key with `t <= tau`, no interpolation. Their string
values ride a **side channel keyed like the numeric tracks**
(same target+channel names) — they never enter a numeric
track. This extension lands with the clips loader adoption
(§15.10); until then no evaluator lane is claimed to exist.

**Ease is stored on the arriving key; the loader remaps it.**
*Rationale: the format's placement is the §14.4 law (ease on
the key a segment ends at), while the evaluator's `Key.ease`
is easing-to-next (the segment a key leaves) — one of them
must move at the boundary, and the format's data is the side
that already exists.* Decided: a clip key's `ease` names the
segment **ending at that key**, exactly as in §14.4. When the
loader builds evaluator tracks it remaps **format
`key[i+1].ease` → anim `key[i].ease`** for every segment; the
format's first key has no incoming segment, so its `ease` is
dropped, and the last evaluator key's `ease` is never used.
This is the identical rule the evaluator's adoption map
already states for §14.4 camera keys, applied to clips
unchanged.

**Duration lives on the clip; wrap lives on the instance.**
*Rationale: one duration bound per clip is what validation
needs (a key beyond it is a single named failure), and two
instances of one clip may legitimately map time differently,
so the wrap mode cannot live on the clip or the track data.*
Decided: the format keeps `duration` on the clip and the wrap
mode on the instance (`clipTime.loop`), and the loader bridges
the placement mechanically — the clip's `duration` becomes
each built track's duration (the evaluator's
`Track::with_duration`), and the instance's `loop` becomes
each track's `Wrap` (`"clamp"` → `Wrap::Clamp`, `"loop"` →
`Wrap::Loop`, `"ping-pong"` → `Wrap::PingPong`). The
evaluator's per-`Track` duration/wrap remains its internal
shape; no format file ever spells them per-track.

**Rotation channels are euler scalars; there is no quaternion
channel.** *Rationale: the format spells what the scene data
carries and what the engine consumes — node rotation in the
data is euler components, and the math layer's rotation path
is Euler → matrix.* Decided: clip rotation channels are
`rotate.x` / `rotate.y` / `rotate.z` (kind S, §15.7). The
evaluator's quaternion lane (`Track<Quat>`, sampled via
`Clip::sample_quat`) remains an evaluator-internal capability
for code-side use; §15 defines no `rotation` channel, and a
future quaternion channel would be a separate amendment with
its own data evidence — not something authors may assume from
the evaluator's type list.

### 15.10 Adoption order

The extensions land in this order, and no step is skipped:

1. **Reservation guard first** (§15.1): the named hard errors
   on the reserved keys, in `scenefile.rs` and in `film.rs`'s
   own node walk. Gate: until this lands, `check_prefabs.py`
   and `check_clips.py` are the only consumers of the
   extensions, and **no production data uses them**.
2. **Expansion and evaluation next**: the §15.5 expansion
   ported into the loader ahead of its node walk; the clips
   loader and the §15.9 evaluator lanes (`Color`, the step
   side channel) with the §15.9 remaps. Gate: the fixture sets
   become loader tests — the valid fixtures load (and the
   valid prefab scene renders its expanded tree), each invalid
   fixture fails with its corresponding named error
   (`check_prefabs.py`: 6 checks over `scenes/fixtures/prefabs/`;
   `check_clips.py`: 5 checks over `scenes/fixtures/clips/`,
   each bad fixture failing exactly one). Evaluator adoption at
   existing call sites (§14.4 camera keys first) is separately
   A/B-gated per site, because the evaluator's f32-native
   evaluation can differ from the current f64 paths by ~1 ulp
   (§15.8).
3. **Production data last**: a production scene or film file
   may use prefabs or clips only after steps 1–2 have landed,
   and both validators run over it in tools-ci (exit 0 = pass,
   1 = a check failed, 2 = input error — the repo tool
   convention), with `validate_film.py` still enforcing the
   §11 f32 law on the expanded result.

---

## W31 notes for the lead (not part of the paste text)

**Verification.** Every `anim.rs` identifier quoted in §15 was
checked against `src/anim.rs` at HEAD (`df89ad6`): `Easing`
variants and `from_name`'s ten accepted spellings, `Wrap`
variants, `AnimValue` impls (`f32`, `[f32; 3]`, `Quat`),
`Key { t, value, ease }` with its easing-to-next doc comment
(the remap sentence in §15.9 is the comment's own mapping),
`Track::with_duration`, and `Clip`'s three lanes with
`add_*` / `sample_*` names. §15 quotes no `anim.rs` line
numbers — GATE2-W20 §4.1 recorded ~3 lines of citation drift
in W15's note; names are the stable interface.

**Flags — things the source documents left in tension, and how
the draft handled them (nothing beyond the four §15.9 rulings
was silently resolved):**

1. **Both source specs claim §15.** W14 §10 says "fold into
   §15" for prefabs; W15 §7(a) says "fold §2–§4 as a new §15"
   for clips. The draft makes §15 one combined section
   (guard §15.1, prefabs §15.2–15.5, clips §15.6–15.8,
   rulings §15.9, adoption §15.10), following GATE2-W20
   §5.3's fold-in order (W14 first, W15 next). If the lead
   prefers §15/§16, the internal cross-references renumber
   mechanically.
2. **`uses` layering is still unratified.** WAVE2-RECORD §9
   records decisions for the colour/step lane, the ease
   bridge, and wrap/duration — but not for W15 §7 flag 3
   (keep-or-drop `uses`). The draft keeps W15's layering
   semantics unchanged (§15.6), as the task required unchanged
   semantics outside the four reconciliations. If the lead
   drops layering, §15.6's `uses` paragraph and check 4's
   cycle half go with it; nothing else in §15 depends on it.
3. **W14 §10's two open questions are carried, not decided:**
   (a) whether film segment files (`scenes/film/*.scene.json`,
   consumed by `film.rs`, not the still loader) permit prefabs
   in the same landing — §15.1 makes the *guard* cover
   `film.rs`'s walk either way; (b) pids inside
   param-supplied subtrees are stripped at expansion but not
   addressable by overrides (they arrive after the merge
   step) — §15.4's "direct scope" wording is consistent with
   this but does not spell out the param-subtree case.
4. **The colour lane's name and shape follow the lead's
   recorded decision, not the obvious guess.** WAVE2-RECORD
   §9.1 decides `AnimValue::Color`, linear-space `[f32; 3]` —
   US spelling, three components (no colour channel anywhere
   in the rig carries alpha; the film's folded light colours
   are RGB triplets). At HEAD `AnimValue` is a trait, so the
   record's phrase is lane shorthand; §15.9 phrases it as a
   lane named `Color` and states plainly that it lands with
   the clips adoption — no existing code is claimed.
5. **poseTrack has two conversion descriptions, and they
   agree.** W15 §5.2 converts poseTrack data into *clip keys*
   (hold key + arrival key carrying `ease: "smoother"`);
   ANIM-W11 §4.3 encodes poseTrack semantics into *evaluator
   keys* (the arrival pair plus `Step` on the preceding key).
   Composed through §15.9's remap they produce the same
   evaluator track: easing on a segment whose endpoints are
   equal (the hold) is inert, so the missing `Step` in the
   clip spelling changes no value. Flagged so the lead can
   confirm at adoption rather than discover it.
6. **f64-vs-f32 evaluation is a disclosed hazard, not a spec
   contradiction.** W15 §4 states the film's scalar law as
   f64-computed/cast-to-f32; `anim.rs` evaluates natively in
   f32 (~1 ulp differences, ANIM-W11 §4.1). §15.8 carries the
   note and §15.10 keeps evaluator adoption A/B-gated per
   site.
