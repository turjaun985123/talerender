# FORMAT-EXT-PREFABS-W14 — scene-format extension: prefabs & templates

**Seat:** TaleRender Wave 2, W14 (scene-format extensions I — Phases 4/5).
**Date:** 2026-11-14. **Status:** SPEC + reference validator + fixtures.
The Rust loader does **not** implement prefabs — §9 states exactly what
remains. This document is written to be folded into
`docs/scene-format.md` as its §15 by the lead/integrator; nothing in
`docs/scene-format.md` was edited by this seat.

**Deliverables of this seat**

| Piece | Path |
|---|---|
| This spec | `docs/integration/FORMAT-EXT-PREFABS-W14.md` |
| Reference validator + expansion | `tools/format/check_prefabs.py` |
| Fixtures (1 library, 1 valid scene, 9 invalid cases) | `scenes/fixtures/prefabs/` |

The external audit asks for prefabs/templates as **versioned
extensions of the existing format**, not a new format and not a
serialisation swap. That is what this is: one new top-level block,
one new node-position form, one slot syntax — everything else is
base v2, and the output of expansion is a plain v2 document.

---

## 1. Decision summary

| Question | Decision |
|---|---|
| Where do prefabs live? | A top-level `"prefabs"` block, inline in the scene file and/or in library files that carry only that block (§2). One merged scope; a name defined twice is an error, never shadowing. |
| What is a prefab? | `{ "params": {…}, "node": <one node subtree> }` — a single root node, so an instance always occupies exactly one node position. Multi-object sets are a `group` root (the base format's own idiom, §4.2 of the spec). |
| How are values parameterised? | Slots: an object with exactly one key, `{"$param": "<name>"}`, standing for a whole JSON value. Substitution only — **no arithmetic, no string interpolation, no expressions** (§3). |
| How is a prefab used? | An instance object in any node position: `{"prefab": "<name>", "params": {…}, "overrides": {…}, "transform": […], "name": "…"}` — a closed grammar (§2.3). |
| What can params override? | The declared default, **wholesale per param** — no merging (§4.1). |
| What can overrides change? | Per-sub-node restyling addressed by prefab-local `pid`, deep-merged under strict legality rules (§4.2). Identity keys (`type`, `kind`, `generator`, `id`, …), transforms, deletions, and slot positions are **never** overridable (§4.3). |
| How do instance transforms behave? | They **compose**, never replace: the expanded root is wrapped in an implicit group carrying the instance transform (§4.4). |
| When does expansion happen? | At load, before any node is walked. Prefabs are a load-time macro: the renderer, the telemetry, and the film driver only ever see expanded plain v2 nodes (§5). |
| Can prefabs recurse? | No. The static reference graph over definitions must be acyclic; cycles are a load error with the cycle path printed (§6). |
| What version are prefab files? | `"version": 2`. The extension **rides the existing format version** as a reserved-key extension; a v2 loader without the extension must **fail loud** on the reserved keys, never silently drop them (§7). |

---

## 2. Grammar

### 2.1 The `prefabs` block and library files

```jsonc
"prefabs": {
  "<name>": {
    "description": "optional, documentary, ignored",   // the only
                                                       // documentary key
    "params": { "<slot>": <declaration>, ... },        // optional;
                                                       // absent = no params
    "node": <node>                                     // required: the
                                                       // template subtree
  },
  ...
}
```

- The block may appear **inline** in a scene file, and/or in a
  **library file**: a JSON object `{"format": "talerender-scene",
  "version": 2, "prefabs": {…}}` with no `nodes` (a library file is
  never rendered; it is data for the scope). A consumer assembles
  one scope from the scene's inline block plus every library it is
  given.
- **No shadowing.** If the same name is defined by two sources
  (inline vs library, or library vs library), that is a hard error
  naming both sources. Rationale: the film validator's whole posture
  is that the same file must mean the same thing everywhere; silent
  precedence rules are how two machines end up expanding the same
  scene differently.
- Prefab names are the file's own namespace: any non-empty string
  that is a valid JSON object key. Names carry no path semantics
  (no `/`, no imports) — a library is included whole or not at all.

### 2.2 Param declarations

```jsonc
"params": {
  "height":   { "type": "number", "required": true },
  "capColor": { "type": "color",  "default": "#C23B2E" },
  "label":    { "type": "string", "default": "", "description": "…" }
}
```

- Declaration keys are a closed set: `type` (required), `required`
  (optional boolean), `default` (optional), `description`
  (optional, documentary).
- **Every param is exactly one of:** `required: true` (the instance
  must supply it) or carrying a `default` (the instance may). A
  required param with a default, and a param that is neither, are
  both definition errors — there is no third "optional, may be
  absent" state, because a slot must always expand to a value.
- A `default` must type-check against its own declaration, and must
  be **literal**: it may not contain a `$param` slot anywhere inside
  it. Defaults are the prefab's contract with its callers; a default
  that itself needed parameters would make the contract recursive.

### 2.3 Slots

A **slot** is an object with exactly one key:

```jsonc
{ "$param": "height" }
```

- A slot stands for a **whole JSON value** and may appear anywhere
  a value may appear inside the template subtree: as a field value,
  as an array element (e.g. one component of a `size` or a
  `translate` vector), as a whole array/object (for `array`/`object`
  params), and as the value of a nested instance's `params` entry
  (forwarding, §3.3).
- Slots are **lexically scoped**: a slot inside prefab P's template
  names a param of P — never of a caller, never of a nested prefab.
  A slot anywhere in a scene file outside a template is an error.
- A slot may **not**: be an object key; be the value of `"prefab"`
  (instantiation targets are literal strings, so the reference
  graph of §6 is static and checkable before any expansion); appear
  inside a param declaration or a default.
- A slot object with any additional key is not a slot — it is an
  ordinary object (and will fail base validation downstream if it
  lands somewhere it makes no sense). The one-key shape is what
  keeps the syntax unambiguous against the base format's own
  single-key objects (transform ops, colour specs).

### 2.4 Instances

An **instance** is an object in a node position carrying `"prefab"`:

```jsonc
{ "prefab": "marker-post",
  "params":    { "height": 1.2, "postY": 0.6, "capY": 1.24,
                 "capColor": "#2E5FC2" },
  "overrides": { "cap": { "material": { "roughness": 0.35 } } },
  "transform": [ { "translate": [2.5, 0, -2] } ],
  "name": "east gate post" }
```

- Closed key set: `prefab` (required, literal string), `params`
  (optional object), `overrides` (optional object, §4.2),
  `transform` (optional, base-format transform list), `name`
  (optional string). **Any other key is a hard error.** This is a
  deliberate departure from the base format's "unknown fields are
  ignored" (§1 of the spec): an instance is a *call*, and a
  misspelled key in a call (`"param:"`, `"overide"`) would otherwise
  be silently dropped while the render proceeds with defaults —
  the exact failure §1's rule accepts for descriptive fields and
  must not accept here. It mirrors §1's own other half: unknown
  *values in discriminating positions* are already hard errors.
- An instance has **no `"type"` key** — `"prefab"` is its
  discriminator. (This is also what makes today's loader fail loud
  on instances; see §7.)
- Instances may appear **anywhere a node may appear**: the
  top-level `nodes` array, `group` children, and (after
  substitution) in `node`/`nodes` param positions. Instances may
  **not** appear where the base format does not take nodes:
  `attachments`, `geometry`, `material`, `textures`, `lights` are
  not node positions. Slots, being value-level, *may* appear inside
  those sub-objects within a template (a material colour slot is
  the fixtures' main example).

### 2.5 `pid` — prefab-local node labels

Any node inside a template may carry `"pid": "<local-id>"`. A `pid`
is **not** an `id`: the base format's `id` fields name entries in
closed, loader-level registries (texture ids, code-ref ids) and are
never touched by this extension. A `pid` is scoped to its prefab,
exists only so `overrides` can address the node (§4.2), and is
**stripped during expansion** — it never reaches the renderer, a
saved file, or another prefab. Pids must be unique within a
prefab's **direct scope**: the template subtree, not descending
into a nested instance's interior (that interior is the inner
prefab's own scope; §4.2).

---

## 3. Parameter typing and the f32 law

### 3.1 Types

Param types are the JSON shapes the base format already carries —
nothing is coerced, converted, or computed:

| type | value must be | notes |
|---|---|---|
| `number` | a JSON number (never a boolean) | subject to §3.2 |
| `string` | a JSON string | verbatim |
| `color` | a string matching `#RRGGBB` | the base format's hex colour-spec string (§4.1 of the spec); the `{hex, shade}` object form is an `object` param, not a `color` |
| `boolean` | `true` / `false` | |
| `vec3` | an array of exactly 3 numbers | positions, sizes, targets |
| `array` | any JSON array | element shape is the author's responsibility; the expanded result is base-validated |
| `object` | a JSON object that is not a slot and not an instance | e.g. a material or colour spec |
| `node` | one node object (`type` or `prefab` present) | enables wrapper prefabs |
| `nodes` | an array of node objects | enables container prefabs |

There is deliberately no `integer` type (JSON has one number type;
the base format's counts are numbers), no enum type (closed value
sets belong to the base validators of the expanded output), and no
expression type.

### 3.2 The f32 law (§11) applies *after* expansion, unchanged

Params are numbers and strings **exactly as the base format carries
them**. Substitution copies the supplied JSON value verbatim into
the template — bit for bit, no re-rounding, no arithmetic (there is
no operation in this extension that could produce a new number).
Consequently the §11 f32 data law needs no extension: a value that
lands, after expansion, under `lights`, `camera`, or a character
node is subject to exactly the same exact-f32 / source-literal rule
as if it had been written inline, and `tools/film/validate_film.py`
enforces it on the expanded document unchanged. The practical
consequence for authors is symmetric: a prefab that will be staged
from transcribed f32 values should be *called* with those exact
values, just as an inline node would be written with them.

### 3.3 Forwarding and its type rule

A nested instance inside a template may supply a slot as a param
value — this is how an outer prefab forwards its own params
inward:

```jsonc
{ "prefab": "marker-post",
  "params": { "height": { "$param": "height" } } }
```

The forwarded slot's declared type in the **outer** prefab must
match the inner param's declared type exactly, with one widening:
a `color` may feed a `string` (every colour is a string; the
reverse is false and is an error). Literal values in a template's
nested instance type-check exactly as scene-level values do. A
template must be **self-contained**: a nested instance that omits
an inner required param is a definition error, not something a
scene-level caller can repair — encapsulation is the point of the
construct.

### 3.4 The cost of "no arithmetic", stated plainly

Substitution cannot derive values. If a template's geometry needs
both a post's height and its centre (`height / 2`), those are **two
params**, and the fixture library says so in its own description
and is staged accordingly. This is a real ergonomic cost and it is
accepted deliberately: the moment slots could compute, param
values would stop being "numbers exactly as the base format
carries them", the f32 law would need an expression semantics, and
expansion would stop being auditable by reading the file. Authors
who need derived staging compute it where staging is authored —
as they do today — and pass the results as params.

---

## 4. Override semantics

There are exactly two override channels, with different mechanics
on purpose: **params replace wholesale; overrides deep-merge.**

### 4.1 Params: wholesale replacement of defaults

The effective value of a param is: the instance's supplied value
if present, else the declared default. Supplied values replace the
default **entirely** — an `object` param's supplied object is *not*
merged into its default object field by field. Rationale: a
half-merged material spec is invisible in the file and unauditable;
if an author wants per-field variation of an object, the fields
should be separate params (as the fixture library does with its
colours) or the variation belongs in `overrides`.

### 4.2 Overrides: pid-addressed deep merge

```jsonc
"overrides": { "<pid>": <partial node object>, ... }
```

The partial object is deep-merged onto the template node carrying
that `pid`, **before** param substitution, by these rules:

```
merge(dst, src):
  for each key k in src:
    if k is forbidden (§4.3)            -> hard error
    if src[k] is null                   -> hard error (no deletion)
    if dst[k] is a $param slot          -> hard error (§4.3)
    if dst[k] and src[k] are both plain objects (neither a slot)
                                        -> merge(dst[k], src[k])
    else                                -> dst[k] = deep copy of src[k]
                                           (arrays and scalars replace
                                           wholesale; arrays are never
                                           concatenated or merged by index)
```

- Arrays replace wholesale because the base format's arrays are
  positional or ordered — vectors, transform op lists (§5 of the
  spec composes left to right), camera keys. Element-wise merge of
  a transform list would silently rewrite staging order; an author
  who needs a different transform uses §4.4, not an override.
- An override may **add** keys the template node does not carry
  (the fixtures add nothing, but e.g. adding `roughness` to a
  material that omitted it is legitimate restyling). Added keys are
  still subject to the forbidden list, and the merged result is
  base-validated after expansion like any authored node.
- Overrides address only the target prefab's **direct scope**
  (§2.5). The interior of a nested instance is not addressable from
  outside — to restyle inside a nested prefab, call that prefab
  directly or expose a param. This keeps a prefab's internals
  refactorable without breaking its callers.

### 4.3 What may NOT be overridden — and why

An override (at any depth inside the partial object) may not touch:

| Forbidden | Why |
|---|---|
| `type` | The node's discriminating value. Changing it changes *what the node is*, not how it is staged — that is a different prefab, not an override. (§1 of the spec already treats discriminating values as hard-error territory.) |
| `kind` (geometry, attachment) | Same: a closed-vocabulary discriminator. A post that becomes a cylinder is a variant prefab. |
| `generator` (texture) | Same: names a code painter from a closed registry. |
| `id` | Registry identity (texture ids, code-ref ids). Renaming by instance would make the eagerly-resolved texture table context-dependent — the loader resolves textures in array order before walking nodes. |
| `prefab` | Re-targeting an instance from an override would make the §6 reference graph dynamic and uncheckable. |
| `pid` | The addressing mechanism itself; re-labelling during the merge that uses the labels is incoherent. |
| `transform` | See §4.4 — transforms compose; they are never replaced. |
| any key whose template value is a `$param` slot | Two channels writing one position is an ordering ambiguity. Slot positions are written by `params` only; overrides that need to reach "through" a slot mean the prefab author should restructure (put the pid on a concrete sibling field, or expose a param). |
| deletion (a `null` value) | The base format has no null semantics anywhere; "remove this field" by override would make the template's meaning caller-dependent. Variants that omit a field are separate prefabs. |

### 4.4 Transforms compose; they never replace

A template root may carry its own `transform` — that transform is
part of the prefab's authored staging and is protected by §4.3. An
instance's `transform` is its **placement in the parent space**.
Expansion therefore does not merge the two lists (list merge is
forbidden by §4.2 anyway, and concatenation would entangle the
association question the base spec's §5 already had to settle);
instead, when an instance carries a `transform`, the expanded root
is wrapped:

```jsonc
{ "type": "group",
  "transform": <the instance's transform>,
  "children": [ <expanded template root, its own transform intact> ] }
```

World = parent × instance × root-local, by the base format's
ordinary group rule — no new transform semantics anywhere. When an
instance carries no `transform`, no wrapper is emitted. An
instance's `name`, when present, is set on the outermost expanded
node (the wrapper if there is one); it is documentary, like the
scene `name`, never an identity.

---

## 5. Expansion algorithm (normative)

A conforming consumer performs these steps at load, in order:

1. **Assemble the scope** (§2.1): inline `prefabs` + all supplied
   libraries. Duplicate name → hard error.
2. **Validate the library eagerly** — every definition, used or
   not: declaration grammar (§2.2), defaults literal and
   type-correct, every slot declared, no slot naming a prefab,
   pids unique per direct scope. Eager validation follows the
   loader's existing precedent: textures are resolved eagerly, in
   array order, at load (`scenefile.rs`, fixtures pack) — a broken
   unused entry already fails a file today.
3. **Build the reference graph** (definition → names its template
   instantiates, literally) and reject cycles (§6).
4. **Validate every instance** in the scene and in templates:
   closed key set, known target, required params present, no
   undeclared params, values type-check (§3), overrides legal (§4).
5. **Expand** depth-first, in document order. For each instance:
   (a) effective params = defaults overridden wholesale by supplied
   params; (b) deep-copy the template root; (c) apply overrides by
   pid (§4.2); (d) substitute every slot with a deep copy of its
   effective param value (the same param used twice yields two
   independent copies); (e) recursively expand instances in the
   resulting subtree (nested instances, and instances arriving via
   `node`/`nodes` params); (f) strip every `pid`; (g) wrap for the
   instance transform and set `name` (§4.4).
6. **Proceed with the base format exactly as if the expanded
   document had been authored**: the expanded tree contains no
   `prefab`, no `pid`, no `$param` — the consumer asserts this —
   and the top-level `prefabs` block is consumed and dropped (it
   does not propagate into any saved or derived document).

Document order is preserved throughout (instances expand in place),
so painter's order, texture resolution order, and every other
order-sensitive behaviour of the base format are unaffected.

---

## 6. Cycles

The reference graph of step 3 is **static**: instantiation targets
are literal strings (§2.3 forbids slots there), so every edge is
visible without expanding anything. A cycle — a prefab whose
template transitively instantiates itself — is a hard error, and
the error prints the cycle path (`loop-a -> loop-b -> loop-a`, see
§8). Two clarifications:

- **Instance data cannot create a cycle.** A `node` param value is
  a finite JSON tree written in the file; it can contain instances,
  but each is expanded against the acyclic definition graph, so
  expansion terminates. There are no references, pointers, or
  names in the format that could make a finite file denote an
  infinite tree — the definition graph is the only recursion, and
  it is checked.
- **A depth guard is still required** in implementations (the
  reference validator uses 64) as belt-and-braces: a consumer that
  expands without running the graph check first must fail, not
  hang or overflow, on adversarial data.

---

## 7. Versioning and the fail-loud rule

- Prefab files declare `"format": "talerender-scene", "version": 2`.
  The extension **rides the existing version**: it adds no field to
  any existing construct, changes no existing field's meaning, and
  its fully expanded form is an ordinary v2 document — the three
  things §10 of the spec reserves version bumps for are all absent.
  What it adds is two **reserved keys**: the top-level `"prefabs"`
  block and the node-level `"prefab"` key (plus `pid`, meaningful
  only inside templates).
- **Required behaviour of a v2 loader that does not implement this
  extension:** encountering `"prefabs"` at the top level, or
  `"prefab"` in a node position, is a **hard error** naming the
  unsupported extension — never a silent drop. This is a carve-out
  from §1's "unknown fields are ignored", and it exists because
  §1's rule is unsafe for exactly these keys: ignoring a `prefabs`
  block while honouring its instances is incoherent, and ignoring
  an instance deletes scene content. (Precedent inside the base
  format: a v1 file carrying v2 fields is *rejected*, §14 — the
  format already refuses to reinterpret what it cannot honour.)
- **What today's loader actually does** (measured against
  `src/scenefile.rs` at this seat's tree, not assumed):
  - Top-level `"prefabs"`: never read — **silently ignored**, per
    §1's rule. A file that defines prefabs but (through some other
    consumer's bug) reaches this loader with its instances already
    gone would render without them and say nothing. This is the
    hazard the reservation rule removes.
  - An instance node: `walk_nodes` requires `type` of every node
    (`str_field(node, "type", …)`, `scenefile.rs:508`), so an
    instance **already fails loud today** — but with a misleading
    message (`'type' is required`-style) that never mentions
    prefabs.
- **Consequence for adoption order:** no production scene/film data
  may use prefabs until the lead lands, together, (a) the
  reservation guard (one check on each reserved key, producing a
  named "prefab extension not supported" error) and (b) the
  expansion itself — or data and guard land in the same commit.
  Until then, `tools/format/check_prefabs.py` is the gate: it is
  the only consumer that understands the extension, and data that
  has not passed it must not be written.

---

## 8. The validator

```
python3 tools/format/check_prefabs.py [--library LIB.json ...] SCENE.json [SCENE ...]
```

Exit codes follow the repo's tool convention (`validate_film.py`,
`make_golden.py`): **0** = every check PASS, **1** = any check FAIL,
**2** = input error (unreadable / invalid JSON / non-object input —
one-line `ERROR` on stderr, no report). The validator renders
nothing and checks **only the prefab layer**; the expanded output's
base-v2 correctness remains the loader's and `validate_film.py`'s
job.

| # | Check | Catches |
|---|---|---|
| 1 | parse | duplicate JSON keys (parsers disagree on duplicates; the repo's own parsers must not be left to chance), `format` value, prefab use under a non-2 version |
| 2 | library | definition grammar, declaration grammar, required/default exclusivity, literal + type-correct defaults, undeclared slots, slot-as-prefab-name, duplicate pids |
| 3 | instances | unknown prefab, closed instance key set, missing required params, undeclared (extra) params, param type mismatches, forwarding type mismatches, slots in scene position |
| 4 | overrides | unknown pid, forbidden keys at any depth, null deletion, merges through a `$param` slot |
| 5 | cycles | any cycle in the static reference graph, printed as a path |
| 6 | expansion | full reference expansion of the scene; counts instantiations and expanded nodes; asserts zero `prefab`/`pid`/`$param` residue. SKIPPED (not passed) when an earlier check failed |

Checks 2–5 validate **all** definitions in scope, including ones
the scene never instantiates — the eager rule of §5 step 2.

---

## 9. Fixtures and verification

All fixtures live in `scenes/fixtures/prefabs/` and are kept tiny
(one feature or one failure each). The library defines two prefabs:

- **`marker-post`** — a `group` of two boxes. Params: `height`
  (number, required; drives the post's `size` via a slot in array
  position), `postY` / `capY` (numbers, defaults staged for
  `height` 1.8 — the no-arithmetic law of §3.4 made visible),
  `postColor` / `capColor` (colors, slotted into the two materials).
  The cap node carries `pid: "cap"` and a `roughness` the valid
  scene's override restyles.
- **`post-pair`** — a `group` of two `marker-post` instances:
  param forwarding (`height`, `capColor` passed through as slots),
  a `gap` param slotted into a `translate` vector, and per-instance
  transforms that compose with (§4.4), never replace, the inner
  template roots.

Plus one **inline** prefab in the valid scene (`ground-disc-marker`,
a disc whose `radius` is a defaulted param), proving inline and
library definitions share one scope.

Runs below are verbatim (paths trimmed to the fixture directory).
Validator: `tools/format/check_prefabs.py` at this seat's tree.

### 9.1 The valid scene — PASS, exit 0

```
$ python3 tools/format/check_prefabs.py --library library.prefabs.json valid.scene.json
check_prefabs.py — valid.scene.json (3 definitions in scope: ground-disc-marker, marker-post, post-pair)

[1 parse] PASS — 3 prefab definitions in scope (ground-disc-marker, marker-post, post-pair), 4 scene instances, version 2

[2 library] PASS — 3 definitions checked, 11 slots, 1 pids

[3 instances] PASS — 6 instances checked (4 in the scene, 2 in templates)

[4 overrides] PASS — 1 override blocks checked

[5 cycles] PASS — reference graph: 3 definitions, 1 edges, acyclic

[6 expansion] PASS — 4 scene instances expanded (of 6 total instantiations incl. nested), 18 typed nodes in the expanded tree, 0 residual keys

OVERALL: PASS (6/6 checks passed)
exit=0
```

### 9.2 Expansion, inspected directly

Check 6's counts were verified by running the reference expansion
standalone and printing the overridden instance (scene node 1, the
`marker-post` with `capColor #2E5FC2`, `height 1.2`, and the
`cap.material.roughness` override). Every spec claim is visible in
the output: slots substituted (`size[1] = 1.2`, translates `0.6` /
`1.24` from `postY`/`capY`), the default colour surviving where no
param overrode it (`#8A6B42`), the deep-merge override landed
(`roughness: 0.35`, the slot-fed colour untouched beside it), the
`pid` stripped, and the instance transform present as a wrapper
group — composed, not merged into the template root:

```json
{
 "children": [
  {
   "children": [
    {
     "geometry": {"kind": "box", "size": [0.12, 1.2, 0.12]},
     "material": {"color": "#8A6B42"},
     "transform": [{"translate": [0, 0.6, 0]}],
     "type": "mesh"
    },
    {
     "geometry": {"kind": "box", "size": [0.2, 0.08, 0.2]},
     "material": {"color": "#2E5FC2", "roughness": 0.35},
     "transform": [{"translate": [0, 1.24, 0]}],
     "type": "mesh"
    }
   ],
   "type": "group"
  }
 ],
 "transform": [{"translate": [2.5, 0, -2]}],
 "type": "group"
}
```

### 9.3 The invalid fixtures — each FAILs with its specific message, exit 1

One file per failure class; each output below is the failing check
only (the other checks pass or, for check 6, report SKIPPED — a
skip is never counted as a pass).

**Unknown prefab** — `--library library.prefabs.json invalid-unknown-prefab.scene.json`
```
[3 instances] FAIL — 3 instances checked (1 in the scene, 2 in templates)
    nodes[0]: unknown prefab 'ghost-fence' (in scope: marker-post, post-pair)
OVERALL: FAIL (4/5 checks passed)
```

**Missing required param** — `invalid-missing-param.scene.json`
```
[3 instances] FAIL — 3 instances checked (1 in the scene, 2 in templates)
    nodes[0]: missing required param 'height' for prefab 'marker-post'
OVERALL: FAIL (4/5 checks passed)
```

**Extra (undeclared) param** — `invalid-extra-param.scene.json`
```
[3 instances] FAIL — 3 instances checked (1 in the scene, 2 in templates)
    nodes[0].params.weight: prefab 'marker-post' declares no param 'weight' (declared: capColor, capY, height, postColor, postY)
OVERALL: FAIL (4/5 checks passed)
```

**Param type mismatch** — `invalid-param-type.scene.json`
```
[3 instances] FAIL — 3 instances checked (1 in the scene, 2 in templates)
    nodes[0].params.height: value 'tall' does not satisfy declared type 'number' of prefab 'marker-post' (got string)
OVERALL: FAIL (4/5 checks passed)
```

**Override touches a forbidden key** — `invalid-override-forbidden.scene.json`
```
[4 overrides] FAIL — 1 override blocks checked
    nodes[0].overrides.cap.type: override touches forbidden key 'type' (identity keys and transforms may not be overridden — spec §4)
OVERALL: FAIL (4/5 checks passed)
```

**Override carries a transform** — `invalid-override-transform.scene.json`
```
[4 overrides] FAIL — 1 override blocks checked
    nodes[0].overrides.cap.transform: override touches forbidden key 'transform' (identity keys and transforms may not be overridden — spec §4)
OVERALL: FAIL (4/5 checks passed)
```

**Override addresses an unknown pid** — `invalid-override-unknown-pid.scene.json`
```
[4 overrides] FAIL — 1 override blocks checked
    nodes[0].overrides.roof: prefab 'marker-post' has no pid 'roof' in its direct scope (declared: cap)
OVERALL: FAIL (4/5 checks passed)
```

**Prefab cycle** — run with `--library invalid-cycle.prefabs.json`
(the bad library is used *instead of* the good one):
`invalid-cycle.scene.json`
```
[5 cycles] FAIL — reference graph: 2 definitions, 2 edges, CYCLE PRESENT
    prefab cycle: loop-a -> loop-b -> loop-a — expansion would not terminate (spec §6)
OVERALL: FAIL (4/5 checks passed)
```

**Template references an undeclared slot** — run with
`--library invalid-slot-undeclared.prefabs.json`:
`invalid-slot-undeclared.scene.json`
```
[2 library] FAIL — 1 definitions checked, 1 slots, 0 pids
    prefabs.broken-post (invalid-slot-undeclared.prefabs.json).node.geometry.size[1]: $param 'widht' is not declared by prefab 'broken-post' (declared: height)
OVERALL: FAIL (4/5 checks passed)
```

**Input error** — the exit-2 contract, per repo convention:
```
$ python3 tools/format/check_prefabs.py /nonexistent.scene.json
ERROR: check_prefabs.py: scene: cannot read /nonexistent.scene.json (No such file or directory)
exit=2
```

---

## 10. What remains — honest status

- **The Rust loader does not implement any of this.** No file in
  `src/` was touched; `talerender --scene-file` on any fixture in
  `scenes/fixtures/prefabs/` behaves exactly as §7 describes
  (instances hard-error on the missing `type`; a lone `prefabs`
  block is ignored). **This seat's deliverable changes no render,
  no pixel, and no existing file's meaning.** It makes the format
  decision, pins the semantics, and proves the data model end to
  end in a reference implementation.
- **For the lead to adopt** (in this order): (1) fold this spec
  into `docs/scene-format.md` as §15, striking nothing above;
  (2) land the §7 reservation guard in `scenefile.rs` — and in
  `film.rs`'s own node walk, which this seat did not survey and
  which must not be assumed to share the still loader's behaviour;
  (3) port §5's expansion into the loader ahead of `walk_nodes`
  (the Python reference is a direct transliteration target);
  (4) turn the fixtures into loader tests: the valid scene must
  load and render, each invalid fixture must fail with the
  corresponding named error.
- **Validator limits, declared:** duplicate JSON keys are reported
  by key name, not by path (Python's `object_pairs_hook` is
  bottom-up and path-blind); `array`/`object` param internals are
  not schema-checked beyond their type (the expanded document's
  base validation is the backstop, by design); the validator does
  not check the §11 f32 law, camera/geometry/texture validity, or
  anything else outside the prefab layer; a library file passed
  positionally as a "scene" is reported as having no `nodes` and
  its expansion is SKIPPED, not passed.
- **Known open questions for the fold-in**, flagged rather than
  decided here: whether film segment files (`scenes/film/*.scene.json`,
  consumed by `film.rs`, not the still loader) should permit
  prefabs in the same landing or a later one; and whether a future
  `nodes`-typed param needs a per-element pid scope (today pids
  inside param-supplied subtrees are stripped at expansion but are
  not addressable by overrides — the subtrees arrive after the
  merge step, §5 step 5c/5e ordering).
