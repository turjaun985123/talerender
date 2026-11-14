# KNOBS-P23-W73 — knobs phases 2–3 staging + phase-1 verification

Seat W73, Wave 2 Batch 8. New file only — no existing file
edited, nothing committed. All line numbers are at HEAD
`b2e46d2` and were read first-hand from the code on disk
(grep + sed), never from commit messages. Spec of record:
KNOBS-W43.md (surviving §§1–4.2) + KNOBS-W43-TAIL-W61.md
(restored §§4.3/5/6, provenance-tagged). Pack of record:
KNOBS-PATCH-W54.md (cites at `aa92cee`) +
`tools/ci/knobs_gate.py`.

**The law (unchanged):** defaults are full quality; a run at
defaults is **byte-identical** to the pre-knob binary, frames
and stdout. A knob the adapter cannot honour is a hard error,
never a quiet clamp. A draft labels itself in-band. The
telemetry modes are CPU/design-space and untouched.

---

## 0. HEADLINE — phase 1 has NOT landed at this HEAD

This seat was briefed that the lead had landed knobs phase 1
— in the brief's words: *"--res with effective_size + the
[DRAFT — NOT FOR DELIVERY] summary tag; CI render dispatch
showed 540x960 at byte floor 22.35 dB"*. **None of that code
exists in this repository at HEAD `b2e46d2`** (local `main`
and freshly fetched `origin/main` are the same commit; the
working tree is clean). The verification below is therefore a
verification of absence, with the census that proves it:

| Phase-1 component (W54 pack) | On disk at `b2e46d2` |
|---|---|
| `Knobs` struct + `samples()` / `is_draft()` / `tag()` (§1) | **absent** — `grep -rn "Knobs" src/` = 0 hits |
| `parse_res` / `parse_aa` / `parse_shadow` (§1) | **absent** — no such fns in main.rs |
| `effective_size()` (§1) | **absent** — 0 hits in src/ |
| Parse arms `--res` / `--aa` / `--shadow` / `--draft` (H2) | **absent** — the parse loop, main.rs:193–262, has exactly 14 arms: `--width --height --out --scene --scene-file --caption --post-stage --telemetry --captions --cutouts-out --film-telemetry --film --from --to`, then `other => cli_error("unknown argument: …")` |
| HELP entries for the four flags (H3) | **absent** — HELP's OPTIONS end at `--to` (main.rs:97–99) followed by `-h` / `-V` |
| Draft tag on any output line (§2.5 / F7 / A6 / S4 / D4 / H6) | **absent** — `grep -rn "DRAFT" src/*.rs` = 0 hits; film's header/progress/completion lines stand verbatim untagged (film.rs:3493–3496, 3523–3526, 3529–3533 — §1.4 below) |
| Overlay scale transform (F8) | **absent** — `overlay_caption` (film.rs:3450) and `overlay_calendar` (film.rs:3400) still use raw design-space constants (§1.3) |
| `knobs` parameter on any entry point (H5 / F2 / F7) | **absent** — dispatch passes bare `(width, height)`: `film::run_film(width, height, dir, &out, film_from, film_to)` at main.rs:286, signatures unchanged in film.rs:3484, ab1.rs:585, scenefile.rs:1251, dof.rs:135 |

Both texts, as required. The claim (brief to this seat):

> "The lead has landed PHASE 1 (--res with effective_size +
> the [DRAFT — NOT FOR DELIVERY] summary tag; CI render
> dispatch showed 540x960 at byte floor 22.35 dB)."

The code (main.rs:252–256, the parse loop's only remaining
arm — any of the four flags falls into it today):

```rust
            other => {
                cli_error(&format!("unknown argument: {other}"));
            }
```

Corollaries, stated plainly:

- **No dispatch in this repo can produce the claimed
  540×960 run.** `.github/workflows/render.yml`'s inputs are
  exactly `mode / from / to / scene / paint_budget_mb` — it
  has no size input of any kind, and there is no knobs
  workflow (workflows on disk: bench, ci, determinism,
  render, tools-ci, visual-regression). The only half-size
  mechanism the binary at this HEAD possesses is the
  pre-existing `--width 540 --height 960` lever — which is
  precisely the undesigned path W43 §1.3 warns against,
  because the film overlays do not scale with it (§1.3).
  Wherever the 22.35 dB figure came from, it was not a
  `--res` landing in this tree.
- **The wave record's phase-1 line should be corrected.**
  WAVE2-RECORD.md still lists knobs among Batch 6's "lead
  landings queued"; nothing in `git log aa92cee..HEAD`
  touches the knobs files at all (per-file census in §2.0).
- **Phases 2–3 are unaffected in substance:** every site
  they touch still stands in its pre-knob form, so the
  packs in §2/§3 below are written against the same code
  W54 surveyed, re-cited. The phase order is now simply
  1 → 2 → 3 all outstanding, and the gate (§4) cannot run
  any leg until all three land.

---

## 1. Phase-1 verification against W54's pack, item by item

### 1.1 The Knobs type + validation helpers (§1 of the pack)

Spec: `Knobs { res: f32, aa: u32, shadow: u32 }`, `Copy`,
`Knobs::FULL = { 1.0, 4, 2048 }`, inserted after `parse_u32`
(main.rs:145–153 at `aa92cee`) before `exit_on_err`.

Disk: absent (census in §0). The insertion point is intact
and unmoved — main.rs has **zero commits since `aa92cee`**
(§2.0), so the pack's main.rs cites are all still exact:
`parse_u32` at 145–153, `exit_on_err` at 156, locals
`width`/`height` at 174–175 (H1), HELP `--to` block at
97–99 (H3 — the pack said "after 102–105"; at HEAD the
`--to` text is lines 97–99 and `-h` follows at 101, so the
insertion is after line 99), the `--film-telemetry`
early-return ends and the film dispatch stands at 286 (H4 /
H5), m1's `run` at 464 and its `wrote` line at 828 (H6).

### 1.2 `effective_size` and the identity lemma

Spec (W43 §3.1 + pack §1): at `res == 1.0` return the base
size **exactly**; below 1, `(v as f64 * res as f64).round()`,
floored at 2, masked even — `(raw.max(2) & !1).max(2)`.

Disk: the function does not exist. The lemma itself is
arithmetic and still checks out against the gate's mirror
implementation: `knobs_gate.py`'s `spec_effective` encodes
the same formula, and its self-test — **re-run first-hand
by this seat at `b2e46d2`: `SELF-TEST: PASS`** (8/8,
including "1080x1920 at res 0.5 -> 540x960" and "res 1
returns the base size exactly (even for an odd base)").

Edge behaviour of the formula, re-derived (the pack's three
pinned cases + the corners):

| input | result | why |
|---|---|---|
| 1080×1920 @ 1.0 | 1080×1920 | early return, base untouched — this branch is what makes the identity **IEEE-exact**: no float ever touches the size at defaults |
| 1080×1920 @ 0.5 | 540×960 | 540.0 / 960.0 round-trip exactly |
| 1080×1920 @ 0.6 | 648×1152 | 648.0000… / 1152.0000… in f64 |
| 1081 @ 0.5 | 540 | round(540.5) = 541 (f64 `round` is half-away-from-zero), mask `& !1` → 540 |
| tiny base @ small res | ≥ 2 | `max(2)` before and after the mask: raw 3 → mask → 2; raw 1 → max → 2 → mask → 2 |
| `--res 0.99999999` | identity branch | parses to exactly 1.0f32 (pack §1 risk note — harmless, it *is* 1.0) |
| `--res 1.0000001` | rejected | parses to > 1.0f32; `parse_res`'s negated conjunction also rejects NaN |

Why the early return is the whole lemma: at defaults the
substitution point (H4) hands the entry points the
*identical integers* the pre-knob binary computed from
`--width/--height`, so every downstream target, mip chain,
readback stride and PNG dimension is the same by
construction — byte-identity does not depend on any float
reasoning at all. The float path exists only off-default,
where identity is not owed.

### 1.3 The overlay identity lemma (F8) — and the live defect

Spec: `overlay_caption` / `overlay_calendar` derive
`(ux, uy) = (w/1080, h/1920)` and scale every design-space
constant, written factor-last so that at 1080×1920
(ux = uy = 1.0, ×1.0 and ÷1.0 IEEE-exact) each expression
evaluates bit-for-bit to today's value.

Disk: neither function carries the transform. Verbatim at
HEAD — caption (film.rs:3450–3453):

```rust
fn overlay_caption(px: &mut [u8], w: u32, h: u32, assets: &Assets, idx: usize, scale: f32) {
    let strip = &assets.strips[idx];
    let (cx, cy) = (540.0f32, 1340.0f32);
    let (hw, hh) = (540.0 * scale, 150.0 * scale);
```

— and calendar (film.rs:3400–3405):

```rust
fn overlay_calendar(px: &mut [u8], w: u32, h: u32, assets: &Assets, cal: &CalendarState) {
    // drawCalendar transform: translate(352, 648) rotate(-0.03)
    // scale(0.92, 0.92*sy); plate local frame x [-180,180],
    // y [-220,220] at 2 px per unit.
    let (cx, cy) = (352.0f32, 648.0f32);
    let (rot, sx, sy) = (-0.03f32, 0.92f32, 0.92 * cal.sy);
```

Consequence, live in the shipped binary today: the only
size lever that exists, `--width/--height`, scales the
scene around both overlays while they sit at design-space
positions (clamped to `(w, h)`, never scaled — W43 §1.3).
Any 540×960 evidence produced with that lever has its
caption and calendar at full-size coordinates on a
half-size frame. F8's pseudo-diff in the pack is unchanged
and still applies verbatim at the cites above (film
uniformly +85, §2.0).

### 1.4 The draft tag — trigger condition, spec vs disk

Spec (W43 §3.4 + pack §1/§2.5): the tag is
`[DRAFT res=<r> aa=<a> shadow=<s> — NOT FOR DELIVERY]`,
appended (leading space, end of line) to the film header,
progress and completion lines and to every still-mode
`wrote` line, iff `is_draft()` — **any** knob off default.
At defaults the tag string is empty and every line is
byte-identical by construction.

Disk: no tag exists anywhere. The three film lines stand
verbatim and untagged (format strings at film.rs:3494,
3524, 3530):

```
"film: {} — {} frames @30fps, rendering {}..{} at {}x{} (clean PNG frames; decision B)"
"  frame {n} (t = {t:.2}s) — {done} rendered, {skipped} skipped · {}"
"film render complete: {done} rendered, {skipped} skipped (resume), range {from}..{to} · {} · {}"
```

The gate already asserts the exact string —
`knobs_gate.py`'s `DRAFT_TAG = "[DRAFT res=0.5 aa=0
shadow=512 — NOT FOR DELIVERY]"` — and its tag-logic
self-test passes (§1.2), so the contract the landing must
satisfy is fully executable today; only the producer side
is missing.

**Precisely which future knobs must join the tag
condition** (the brief's question): when phases 2 and 3
land, `is_draft()` must become
`res != 1.0 || aa != 4 || shadow != 2048` **in the same
commits that plumb them** — a phase-2 landing that wires
`--aa` into the targets but leaves the condition as
`res != 1.0` would let `--aa 0` produce an untagged,
non-final film, breaking the law in its worst direction
(a draft passing as a final). The full condition set:

- `--res` off 1.0 → draft (phase 1).
- `--aa` off 4 → draft (phase 2, same commit).
- `--shadow` off 2048 → draft (phase 3, same commit).
- `--draft` writes all three fields (0.5 / 0 / 512) → draft
  by any of the three tests.
- Uniform rule (pack §2.5, decision §9.2-2): an
  inert-but-off-default knob still tags the mode's output
  (e.g. `--shadow 512` on dof-chart) — the mislabel may
  only ever call a final a draft, never the reverse.
- Tag sites that must all carry it: film header / progress
  / completion (film.rs:3493, 3523, 3529 — the `println!`
  lines), ab1 `wrote` (ab1.rs:1547), scenefile `wrote` ×2
  (scenefile.rs:1517, 2163), dof `wrote` (dof.rs:500 —
  its "MSAA-4x" description head stays; the tag is the
  operative provenance, pack D4), m1 `wrote` (main.rs:828).

### 1.5 Phase-1 divergence register (complete)

| # | Spec (W43 / W54) | Disk at `b2e46d2` | Class |
|---|---|---|---|
| P1 | `--res/--aa/--shadow/--draft` parse + validate pre-GPU, exit 2 | all four hit `unknown argument`, exit 2 via the wrong message | not landed |
| P2 | `effective_size` substitution before dispatch (H4) | dispatch passes raw `(width, height)` (main.rs:286 etc.) | not landed |
| P3 | DRAFT tag on header/progress/completion + `wrote` lines | no tag anywhere in src/ | not landed |
| P4 | Overlays scale with the frame (F8) | design-space constants verbatim (film.rs:3404–3405, 3452–3453) | not landed |
| P5 | HELP documents the knobs (H3) | OPTIONS end at `--to` (main.rs:97–99) | not landed |
| P6 | (record) phase 1 landed; 540×960 dispatch at 22.35 dB | no code, no workflow input capable of it (§0) | **record error** — correct WAVE2-RECORD / the fold-in |

Nothing in phase 1 is *partially* landed: there is no
half-state to preserve, and the packs below assume a clean
start from W54's §1–§2 exactly as written (its main.rs
cites remain exact, §1.1).

---

## 2. Phase 2 — `--aa` patch pack, re-cited at `b2e46d2`

### 2.0 Drift map, `aa92cee` (W54's survey) → `b2e46d2`

Per-file commits since the pack was written
(`git log aa92cee..HEAD -- <file>`):

| file | commits | cite drift |
|---|---|---|
| src/main.rs | none | **0** — all pack cites exact |
| src/ab1.rs | none | **0** — all pack cites exact |
| src/dof.rs | none | **0** — all pack cites exact |
| src/gpucore.rs | none | **0** — `make_target` 141, `samples` param 146, `make_target_viewed` 164 |
| src/scenefile.rs | `4dd6530` (§15 reservation guard) | **+31**, uniform across every knob cite |
| src/film.rs | `a2ec80c`, `4dd6530`, `5a9383e` (blink), `2c41313` (F2 L+T) | **+85**, uniform across every knob cite (struct Renderer onward) |

### 2.1 Threading (signatures at HEAD)

`Knobs` is threaded by value (`Copy`) per pack §1/H5.
Signature changes, current lines:

| fn | HEAD line | gains |
|---|---|---|
| `film::run_film` | film.rs:3484 | `, knobs: crate::Knobs` |
| `film::Renderer::new` | film.rs:2468 | `, knobs: crate::Knobs` (single caller: run_film, film.rs:3491) |
| `ab1::run_ab1` | ab1.rs:585 | `, knobs: crate::Knobs` |
| `scenefile::run_scene_file` | scenefile.rs:1251 | `, knobs: crate::Knobs` |
| `scenefile::run_composer` | scenefile.rs:1553 | same, forwarded |
| `dof::run_dof` | dof.rs:135 | `, knobs: crate::Knobs` |
| `main::run` (m1) | main.rs:464 | `, knobs: crate::Knobs` (consumes only the tag) |

Each consumer derives `let samples = knobs.samples();`
once (film: in `Renderer::new` before the targets at
2613; ab1/scenefile-composer/dof: at the top of the body).
Value map: `0→1, 2→2, 4→4, 8→8` (pack §1 `samples()`).

### 2.2 The uniform pseudo-diff — one shape, four passes

The shape is W54 §4 verbatim (attachment built before
`begin_render_pass`, view and resolve chosen together; at
`samples > 1` the constructed value is field-for-field
today's, which is what the identity law rests on):

```rust
let scene_attachment = wgpu::RenderPassColorAttachment {
    view: if samples > 1 { &msaa_view } else { &resolved_view },
    resolve_target: if samples > 1 { Some(&resolved_view) } else { None },
    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
};
// in the descriptor:
color_attachments: &[Some(scene_attachment)],
```

Applied per pass, with every site it must move with:

**film** — targets: `hdr msaa` (film.rs:2630) and its
depth twin `hdr depth` (film.rs:2636), the literal `4` →
`samples` in both `gpucore::make_target` calls (the
resolved target between them, 2633, stays 1). Pipelines:
`make_pbr`'s `count: 4` (film.rs:2691) → `count: samples`;
`make_basic` scene calls (film.rs:2732–2733) trailing `4`
→ `samples`. Pass: attachment block at film.rs:3259–3265 —
today verbatim:

```rust
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.hdr_msaa_view,
                    resolve_target: Some(&self.hdr_view),
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
                })],
```

→ the §2.2 shape with `self.samples`, `&self.hdr_msaa_view`,
`&self.hdr_view`, this pass's `clear` (built at 3257).
Depth attachment (`&self.hdr_depth_view`, 3267–3271) is
**not** branched — the twin carries the same count.
Downstream is branch-free: the bokeh bind group binds
`&self.hdr_view` (film.rs:3162), the render target in the
1-sample arm and the resolve target in the MSAA arm.
Non-changes: `make_depth` default (2755), bokeh default
(2798), the pass's other three attachments (resolve lines
3176 shadow, 3216 prepass, 3317 final — all `None` against
single-sample targets).

**ab1** (all cites drift-0, exactly as the pack surveyed)
— targets: the two inline descriptors, `sample_count: 4`
at ab1.rs:846 (colour, block 842–852) and ab1.rs:859
(depth, block 855–865) → `sample_count: samples`
(option A, in-place; option B — `gpucore::make_target_viewed`
— stays a separate later cleanup, pack A3). Pipelines:
`make_pbr` `count: 4` (927) → `samples`; `make_basic`
calls (970–971) → `samples`. Pass: attachment at
1416–1419 (`resolve_target: Some(&hdr_view)` at 1418) →
the shape with the function locals. Non-changes:
`make_depth` default (996), bokeh default (1062).

> **THE TRAP LINE — re-verified at HEAD.** ab1.rs:972,
> one line below the two `make_basic` calls that change:
>
> ```rust
>     let basic_overlay = make_basic(wgpu::TextureFormat::Rgba8Unorm, None, 1);
> ```
>
> It **keeps its literal `1`**. It writes the single-sample
> final target (the caption composite); plumbing `samples`
> into it is a wgpu validation error, not a quality change.
> A mechanical `4→samples` sweep of the region 970–972
> breaks the build's runtime, silently at compile time.

**scenefile (composer path)** — targets: `composer-scene-msaa`
(scenefile.rs:1607) **and its depth twin**
`composer-scene-depth` (scenefile.rs:1609) — the twin W43
missed and W54 §0.2 caught; re-verified at HEAD: both
carry the literal `4` as `make_target`'s positional
`samples` argument, and the resolved target between them
(1608, `"composer-scene-resolved"`, `sample_count` 1)
stays 1 — it is the branch's destination, not a knob site.
A colour/depth count mismatch here is a wgpu validation
error, so the pair moves together or not at all.
Pipelines: `count: 4` at scenefile.rs:1668 (scene tex)
and 1712 (scene colour) → `count: samples`. Pass: views
at 1998–1999 (`msaa_view`, `sdepth_view`), pass head at
2000, attachment view 2003 / `resolve_target:
Some(&scene_view)` 2004 → the shape with the locals and
the `scene.clear`-derived clear colour; depth attachment
(`&sdepth_view`) unbranched. Non-changes: prepass / fog /
bokeh / output defaults (1761, 1813, 1863, 1910). The
simple path has no MSAA stage (targets at 1141, 1451,
1462 all `sample_count: 1`; its pass already has
`resolve_target: None` at 1479) — `--aa` is accepted and
inert there, per spec §3.3.

**dof-chart** (drift-0) — targets: `dof-scene-msaa`
(dof.rs:240) **and its depth twin** `dof-scene-depth`
(dof.rs:242) → `samples` (resolved target at 241 stays
1). Pipeline: `count: 4` (dof.rs:281) → `samples`. Pass:
views at 406–407, attachment at 408–412
(`resolve_target: Some(&scene_view)` at 412) → the shape,
clear colour the literal black. Non-changes: depth /
bokeh / output defaults (307, 366, 399).

### 2.3 Phase-2 risks carried forward (unchanged)

- At `samples == 1` the MSAA texture is still created (at
  count 1) and never bound — 16.6 MB fp16 at 1080×1920,
  draft runs only (pack §4; restored W43 T3). Not
  prescribed to fix.
- `--aa 8` on `Rgba16Float` may exceed adapter limits →
  hard wgpu validation failure, never a clamp to 4
  (restored W43 §6 / pack §9.4). The gate does not
  exercise 8; RADV's answer is PC-session evidence.
- Phase 2 must extend `is_draft()` with `aa != 4` in the
  same commit (§1.4) or the tag law breaks.

---

## 3. Phase 3 — `--shadow` patch pack, re-cited at `b2e46d2`

### 3.1 Sites (complete census)

`grep -rn "2048" src/*.rs src/*.wgsl` at HEAD: the live
shadow-size sites are exactly six lines in two files,
plus one comment:

| site | HEAD line | change |
|---|---|---|
| film shadow colour target (`"key shadow map"`, Rgba8Unorm) | film.rs:2613 | `2048, 2048` → `shadow_size, shadow_size` |
| film shadow depth target (`"shadow depth"`, Depth32Float) | film.rs:2616 | `2048, 2048` → `shadow_size, shadow_size` |
| film `shadow_misc` uniform | film.rs:2923 | literal → `[1.0 / self.shadow_size as f32, self.shadow_size as f32, 0.0, 0.0]` |
| ab1 `shadow_misc` uniform | ab1.rs:652 | literal → `[1.0 / shadow_size as f32, shadow_size as f32, 0.0, 0.0]` |
| ab1 shadow colour target (inline Extent3d) | ab1.rs:811 | `width: 2048, height: 2048` → `width: shadow_size, height: shadow_size` |
| ab1 shadow depth target (`composer::make_rt`) | ab1.rs:820 | `2048, 2048` → `shadow_size, shadow_size` |
| `shader_pbr.wgsl` comment (optional, pack §9.3) | shader_pbr.wgsl:51 | `// x: shadow texel size (1/size), y: shadow map size (size)` — comment only |

Threading: `shadow_size = knobs.shadow` derived beside
`samples` (§2.1); film stores it as a `Renderer` field
(struct at film.rs:2423, beside `width`/`height` at
2426–2427; constructor literal at film.rs:2802) because
the uniform is built inside `render()` (film.rs:2880);
ab1 uses the function local (its uniform is built inside
`run_ab1`, ab1.rs:585).

### 3.2 The lockstep argument, re-checked against current code

The claim (W43 §1.2): the size reaches the shader **only**
through the `shadow_misc` slot, so targets + slot moving
together is sufficient and no WGSL changes. Re-verified
at HEAD, all three legs:

1. **The slot is the only channel.** In `render()` the
   slot is one bare push in the per-frame uniform build
   (film.rs:2923, verbatim today):
   ```rust
        vecs.push([1.0 / 2048.0, 2048.0, 0.0, 0.0]);
   ```
   The shader reads exactly this slot:
   `shader_pbr.wgsl:167–168` derives
   `shadow_map_size = vec2f(frame.shadow_misc.y)` and
   `texel_size = vec2f(frame.shadow_misc.x)` (field
   declared at :52). **No WGSL file contains a numeric
   2048** (census above; the only wgsl hit is the :51
   comment). PCF offsets are in texel units, so the slot
   alone carries the size's softness effect.
2. **Bit-identity at 2048.** `1.0 / 2048.0f32` is exact
   (2048 = 2¹¹, its reciprocal is a power of two) and
   `2048 as f32` is exact, so the computed slot at
   `shadow_size == 2048` is bit-identical to today's
   literal — the default uniform cannot drift. The
   targets at 2048 are likewise the same descriptors.
   Identity at defaults for phase 3 therefore rests on
   exact arithmetic, not approximation.
3. **What must NOT move.** The shadow *pipelines* render
   into the shadow targets at sample count 1 at every
   knob value (`make_depth` family, `MultisampleState::
   default()` — film.rs:2755, ab1.rs:996): only the
   targets' *size* is knobbed, never their count. The
   colour/depth pair (2613/2616; 811/820) moves together
   — a size mismatch between them is a broken shadow
   pass, not a validation error, which is why the pair
   is one hunk, not two.

Non-sites at HEAD (do not touch): `rendergraph.rs:658–659`
declares `Fixed(2048, 2048)` shadow resources — that is
the not-yet-wired graph's fixture description of today's
renderer, not a render path; if rendergraph is ever
adopted beside the composer it must read the knob then,
and phase 3 should leave a note there or in its landing
message. `paintbudget.rs:54` `BUDGET_CEILING_BYTES =
2048 * 1024 * 1024` is MiB, unrelated. ab1.rs:602 and
gpucore's 2048s are downlevel-limit comments.

### 3.3 Phase-3 notes

- `--shadow` is inert in scenefile (parses `lights.key.shadow`
  but builds no shadow target), dof and m1 — accepted
  silently per spec §3.3, but it still joins `is_draft()`
  globally (§1.4).
- `--shadow 4096` on downlevel-class limits → visible
  failure, no clamp (restored W43 §6).
- Phase 3 must extend `is_draft()` with
  `shadow != 2048` in the same commit (§1.4).

---

## 4. knobs_gate — CI invocation list once phases land

The gate tool (`tools/ci/knobs_gate.py`, self-test PASS
8/8 — re-run first-hand at this HEAD, §1.2) is
**monolithic by design**: one invocation runs leg A
(defaults identity) → leg B (draft smoke) → leg C (CLI
validation) in order, in one process; the first
`GateFail` aborts the remaining legs, and the argparse
surface (binary positional, `--film-dir`, `--work`,
`--self-test`, `--frames4`) has **no leg selector**. The
CI step is therefore the single invocation from pack
§10, unchanged:

```yaml
      - name: Knobs gate
        id: knobs
        run: |
          set +e
          python3 tools/ci/knobs_gate.py \
            ./target/release/talerender \
            --film-dir film --work "$RUNNER_TEMP/knobs"
          echo "script_rc=$?" >> "$GITHUB_OUTPUT"
      - name: Verdict (the PASS line is the verdict)
        run: |
          grep -qx "KNOBS GATE: PASS" \
            "$RUNNER_TEMP/knobs/knobs-results.txt"
```

**Leg B/C phase preconditions** (the brief's question —
when each leg becomes passable):

| leg | what it drives | passable when |
|---|---|---|
| A — identity | no flags vs `--res 1 --aa 4 --shadow 2048` over ranges 195-204, 795-814, 1195-1204; requires BYTE PROBES PASS (40/40) + no `DRAFT` in either stdout | **all three phases** — it spells all three flags, so a phase-1-only binary fails it in the parse loop |
| B — draft smoke | frames 195–219 with `--draft`; asserts exit 0, the exact tag `[DRAFT res=0.5 aa=0 shadow=512 — NOT FOR DELIVERY]` in header + completion, a tag in progress, frames 195–204 **differ** from leg A's plain frames, draft IHDR = spec arithmetic (1080×1920 → 540×960) | **all three phases** — `--draft` writes all three knobs |
| C — validation | 10 cases vs `--scene m1`: `--res 1.5 / 0 / -0.5`, `--aa 3 / 16`, `--shadow 1000 / 0`, and bare `--res / --aa / --shadow`; each must exit 2 with `error:` and **without** `unknown argument` in stderr | **all three phases** — any not-yet-landed flag produces exactly the `unknown argument` refusal the leg treats as failure (by design: it keeps a pre-knobs binary from passing) |

Consequences for the landing sequence:

- There is **no incremental gate run per phase** with the
  tool as shipped: after phase 1 alone, leg A fails on
  `--aa` (unknown argument → render exit 2 → GateFail)
  and legs B/C never execute. Per-phase evidence must
  come from the render dispatches + `byte_probes.py`
  directly:
  - phase 1: two dispatches over the probe ranges (no
    flags; `--res 1`) → `tools/ci/byte_probes.py <plain>
    <defaults>` PASS 40/40, plus one `--res 0.5` dispatch
    whose frame IHDR reads 540×960.
  - phase 2: same pair with `--res 1 --aa 4` spelled,
    plus an `--aa 0` dispatch that must exit 0 and differ.
  - phase 3: the full gate invocation above (legs A+B+C)
    becomes the acceptance, exactly as pack §10 designs.
- If the lead wants per-phase gating from the tool
  itself, a `--legs` selector is a small tool change —
  flagged here, not prescribed; the dispatch route above
  needs no new code.
- All three legs remain **CI-only** (they drive a real
  binary; this VM's llvmpipe segfaults compiling
  shader_pbr.wgsl). The self-test
  (`python3 tools/ci/knobs_gate.py --self-test`, needs
  the frames4 corpus) is the only leg that runs here,
  and it exercises comparison/size/tag logic only.
- Residual gap (pack §10, restated): the gate exercises
  defaults, all-off-default together, and invalid values.
  A defect visible only at one intermediate value alone
  (`--aa 2` solo, `--shadow 1024` solo) passes the gate;
  those spot checks belong to the PC evidence session.

---

## 5. Refreshed hunk index (pack §11, re-based to `b2e46d2`)

| file | hunks | lines at `b2e46d2` |
|---|---|---|
| src/main.rs | Knobs type + parse fns + effective_size (new) | insert after 153 (unchanged) |
| src/main.rs | H1 locals / H2 arms / H3 HELP / H4 substitution / H5 dispatch / H6 wrote | 174–175 / after the `--to` arm (~256) / after 99 / before 286 / 286, 292, 296, 308, 310 / 828; `run` sig 464 (all unchanged) |
| src/gpucore.rs | — none — | `make_target` 141, `make_target_viewed` 164 (unchanged) |
| src/film.rs | F1 fields + constructor literal | 2426–2427; literal at 2802 |
| src/film.rs | F2 `Renderer::new` signature + locals | 2468; locals before 2613 |
| src/film.rs | F3 targets (§3 colour/depth pair + §2 MSAA pair) | 2613, 2616, 2630, 2636 |
| src/film.rs | F4 factories | 2691; 2732–2733 (non-changes 2755, 2798) |
| src/film.rs | F5 shadow_misc (§3) | 2923 |
| src/film.rs | F6 scene-pass resolve branch (§2) | attachment 3259–3265 (bind non-change 3162) |
| src/film.rs | F7 run_film signature/call/tags | 3484; call 3491; header 3493–3496; progress 3523–3526; completion 3529–3533 |
| src/film.rs | F8 overlays (§1) | calendar 3400 (head 3404–3405), caption 3450 (core 3452–3453) |
| src/film.rs | telemetry non-site | `run_film_telemetry` 3608 |
| src/ab1.rs | A1 shadow_misc (§3) | 652 (unchanged) |
| src/ab1.rs | A2 shadow targets (§3) | 811, 820 (unchanged) |
| src/ab1.rs | A3 inline MSAA pair (§2) | 846, 859 (unchanged) |
| src/ab1.rs | A4 factories (§2) | 927; 970–971 change, **972 TRAP — literal 1 stays** (unchanged) |
| src/ab1.rs | A5 resolve branch (§2) / A6 wrote | 1416–1419 / 1547 (unchanged) |
| src/ab1.rs | run_ab1 signature + locals | 585 (unchanged) |
| src/scenefile.rs | S1 targets (§2, twin at 1609) | 1607, 1609 (resolved 1608 stays) |
| src/scenefile.rs | S2 pipelines (§2) | 1668, 1712 (non-changes 1761, 1813, 1863, 1910) |
| src/scenefile.rs | S3 composer resolve branch (§2) | views 1998–1999; attachment 2002–2004 |
| src/scenefile.rs | S4 wrote ×2 | 1517, 2163 |
| src/scenefile.rs | S5 simple path — no hunks | targets 1141, 1451, 1462; pass resolve `None` 1479 |
| src/scenefile.rs | signatures | 1251, 1553 |
| src/dof.rs | D1 targets (§2, twin at 242) | 240, 242 (unchanged) |
| src/dof.rs | D2 pipeline (§2) | 281 (unchanged) |
| src/dof.rs | D3 resolve branch (§2) | views 406–407; attachment 408–412 (unchanged) |
| src/dof.rs | D4 wrote | 500 (unchanged) |
| src/dof.rs | run_dof signature + locals | 135 (unchanged) |
| src/shader_pbr.wgsl | §3 comment (optional) | 51 (unchanged) |
| src/runlog.rs, src/composer.rs | — none — | make_rt `sample_count: 1` at composer.rs:83, 109 (unchanged) |

*Verification basis for this note: full reads of
KNOBS-W43.md, KNOBS-W43-TAIL-W61.md, KNOBS-PATCH-W54.md;
first-hand reads of src/main.rs (parse loop + dispatch),
src/film.rs (Renderer struct/new/render head, targets,
scene-pass attachment, shadow_misc context, both overlays,
run_film lines), src/scenefile.rs (composer targets +
attachment), and grep censuses over src/*.rs + src/*.wgsl
for `Knobs`, `effective_size`, `DRAFT`, `sample_count`,
`count: 4`, `resolve_target`, `2048`; `git log` per file
since `aa92cee`; `git fetch` confirming origin/main =
`b2e46d2`; knobs_gate.py self-test re-run (PASS). No
performance claims are made anywhere in this note.*
