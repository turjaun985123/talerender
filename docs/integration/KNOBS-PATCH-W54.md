# KNOBS-PATCH-W54 — F4 knobs implementation patch pack

Seat W54, Wave 2 Batch 6. Patch plan + acceptance gate — no src/
file edited, nothing committed. The lead implements in src/ from
this pack; the gate is `tools/ci/knobs_gate.py` (this seat's
second deliverable, self-test PASS — §10).

Spec of record: KNOBS-W43 (§§1–4.2 as committed; see the
truncation disclosure in §0.3). All line numbers below are at
HEAD `aa92cee` and were re-read first-hand — W43's numbers were
at `f6616aa` and have drifted (§0.1).

**The law (unchanged):** defaults are full quality and a run at
defaults is **byte-identical** to the pre-knob binary, in frames
and in stdout. A knob the adapter cannot honour is a hard error,
never a quiet clamp in either direction. A draft labels itself
in every line it prints. `--film-telemetry` and `--telemetry`
are CPU/design-space and are **not touched by any hunk here**.

---

## 0. Survey deltas vs W43 (read at `aa92cee`)

### 0.1 Line drift, f6616aa → aa92cee

Most W43 cites moved by −2 in scenefile.rs (1574 → 1576) and not
at all in film.rs/ab1.rs/dof.rs (the errors-step-2 edits changed
signatures, not the target/factory regions). Every cite in this
pack is the `aa92cee` number.

### 0.2 Sites W43 did not name (all folded into the hunks below)

- **scenefile has a depth twin**: `composer-scene-depth` at
  scenefile.rs:1578 also carries the literal `4`. W43 listed
  only the colour target. A colour/depth sample mismatch is a
  wgpu validation error — the pair is hunk S1, both lines.
- **Two more resolve sites**: the scenefile composer pass
  (scenefile.rs:1973) and the dof scene pass (dof.rs:412) set
  `resolve_target: Some(...)` exactly like film/ab1. §3.3 of the
  spec makes `--aa` live in both modes, so both need the §4.2
  branch — hunks S3 and D3. (The task brief named film + ab1;
  the same pseudo-diff is prescribed for all four passes, one
  shape, four copies.)
- **dof has the twin too**: dof.rs:240 colour + dof.rs:242 depth.
- **ab1's overlay basic exists**: `basic_overlay` at ab1.rs:972
  is the `make_basic(..., 1)` call that must stay 1 (hunk A4
  names it as a non-change). Film has no equivalent call — its
  overlay-variant basics live inside `composer::PostChain`,
  which no hunk touches.

### 0.3 ⚠️ The committed KNOBS-W43.md is truncated on disk

The file ends at line 264 with a literal `...[truncated 3075
chars]` marker, mid-§4.2 — its §4.3 (shadow), §5 (acceptance)
and §6 (risks) never reached the repo. This pack reconstructs
those parts from W43's delivered seat report (defaults gating
via byte_probes' ranges, the --draft differ-smoke, captions
scale with the frame, the --aa 8 adapter-risk flag) and from
§§1–4.2 as committed, which are complete through the resolve
branch's first half. Where this pack extends the spec (the
attachment-construction shape in §4, the tag-placement rule in
§2.5, last-writer-wins in §9.2) it says so. Recommendation to
the lead: restore W43's tail from the seat report when the
knobs land, or let this pack stand as the §4.3+ record.

---

## 1. The Knobs type and its helpers (new code, main.rs)

Insert after `parse_u32` (main.rs:145–153), before `exit_on_err`.
One type, threaded by value (it is `Copy`) into every GPU entry
point — positional `u32` parameters for samples/shadow side by
side would be transposable, and a transposition at defaults
(4 ↔ 2048) fails loudly but a transposition of the *derived*
values inside a constructor might not.

```rust
/// Draft/preview quality knobs (KNOBS-W43; patch pack
/// KNOBS-PATCH-W54). Defaults are full quality: a run at
/// `Knobs::FULL` must be byte-identical to the pre-knob binary,
/// frames and stdout alike. Knobs exist for drafts only.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Knobs {
    /// Render scale, 0 < res <= 1 (1 = full size).
    pub res: f32,
    /// MSAA in the user's spelling: 0 | 2 | 4 | 8 (0 = single).
    pub aa: u32,
    /// Key shadow map size: 512 | 1024 | 2048 | 4096.
    pub shadow: u32,
}

impl Knobs {
    pub(crate) const FULL: Knobs = Knobs { res: 1.0, aa: 4, shadow: 2048 };

    /// The wgpu sample count: the user's 0 is single-sample.
    pub(crate) fn samples(self) -> u32 {
        if self.aa == 0 { 1 } else { self.aa }
    }

    /// Any knob off its default makes the run a draft.
    pub(crate) fn is_draft(self) -> bool {
        self.res != 1.0 || self.aa != 4 || self.shadow != 2048
    }

    /// The in-band draft tag (KNOBS-W43 §3.4), with the
    /// effective values so any log snippet carries provenance.
    pub(crate) fn tag(self) -> String {
        format!("[DRAFT res={} aa={} shadow={} — NOT FOR DELIVERY]",
                self.res, self.aa, self.shadow)
    }
}
```

(`f32` Display prints 0.5 as `0.5`, 0.6 as `0.6` — the tag is
provenance for humans and for the gate's substring check, not a
parse target.)

Validation helpers, same insertion point, in `cli_error` house
style (exit 2, message + help hint, before any GPU work):

```rust
fn parse_res(args: &[String], i: usize) -> f32 {
    let raw = value(args, i, "--res");
    let v: f32 = raw.parse().unwrap_or_else(|_| {
        cli_error(&format!(
            "invalid value for --res: '{raw}' (expected 0 < scale <= 1)"))
    });
    // Negated conjunction on purpose: it also rejects NaN,
    // which parses as an f32 but satisfies no ordering.
    if !(v > 0.0 && v <= 1.0) {
        cli_error(&format!(
            "invalid value for --res: '{raw}' (expected 0 < scale <= 1)"));
    }
    v
}

fn parse_aa(args: &[String], i: usize) -> u32 {
    let v = parse_u32(args, i, "--aa");
    if !matches!(v, 0 | 2 | 4 | 8) {
        cli_error(&format!(
            "invalid value for --aa: '{}' (expected 0, 2, 4 or 8)",
            value(args, i, "--aa")));
    }
    v
}

fn parse_shadow(args: &[String], i: usize) -> u32 {
    let v = parse_u32(args, i, "--shadow");
    if !matches!(v, 512 | 1024 | 2048 | 4096) {
        cli_error(&format!(
            "invalid value for --shadow: '{}' (expected 512, 1024, 2048 or 4096)",
            value(args, i, "--shadow")));
    }
    v
}
```

Effective size (KNOBS-W43 §3.1), same insertion point:

```rust
/// Effective frame size: --width/--height x --res, computed
/// once in main() before dispatch. At res == 1 the base size is
/// returned exactly (the defaults identity law); below 1 the
/// size is rounded to even — the skill's draft encode is
/// yuv420 and needs even dimensions — with a floor of 2.
fn effective_size(width: u32, height: u32, res: f32) -> (u32, u32) {
    if res == 1.0 {
        return (width, height);
    }
    let scale = |v: u32| -> u32 {
        let raw = (v as f64 * res as f64).round() as u32;
        (raw.max(2) & !1).max(2)
    };
    (scale(width), scale(height))
}
```

Checks: 1080×1920 at 0.5 → 540×960; at 0.6 → 648×1152; an odd
base (1081) at 0.5 → 540 (round 541, mask to even). The gate's
`spec_effective` mirrors this arithmetic exactly
(knobs_gate.py) and the self-test pins these three cases.

**Risk:** `res` is compared to `1.0` for the identity branch.
`--res 1` and `--res 1.0` parse to exactly 1.0f32 ✓. A value
like `0.99999999` also parses to 1.0f32 and takes the identity
branch — harmless (it *is* 1.0 as a float). `1.0000001` parses
to > 1.0f32 and is rejected ✓.

---

## 2. main.rs hunks

### H1 — locals (main.rs:174–175)

Current:

```rust
    let mut width: u32 = 1080;
    let mut height: u32 = 1920;
```

Add after them:

```rust
    let mut knobs = Knobs::FULL;
```

### H2 — parse arms (insert after the `--to` arm, main.rs:~243–247, before `other`)

```rust
            "--res" => {
                i += 1;
                knobs.res = parse_res(&args, i);
            }
            "--aa" => {
                i += 1;
                knobs.aa = parse_aa(&args, i);
            }
            "--shadow" => {
                i += 1;
                knobs.shadow = parse_shadow(&args, i);
            }
            "--draft" => {
                knobs.res = 0.5;
                knobs.aa = 0;
                knobs.shadow = 512;
            }
```

Ordering semantics (this pack's prescription, extending W43
§2's "flags after --draft override it"): the loop is
last-writer-wins in command-line order, and `--draft` is a
writer of all three fields — `--aa 2 --draft` therefore yields
aa=0, while `--draft --aa 2` yields aa=2. W43 specified only
the second direction; the lead confirms the symmetric rule
when landing (§9.2).

Missing values fall out of the existing `value()` behaviour
(exit 2, `missing value for --aa`) ✓ — gate leg C covers it.

### H3 — HELP text (insert into OPTIONS after the `--to` lines, main.rs:102–105)

W43 §2's text, verbatim spelling:

```
    --res <SCALE>           Render scale for drafts: 0 < SCALE <= 1,
                            applied to --width/--height (default: 1
                            = full size). The whole frame scales,
                            captions included. Draft only.
    --aa <0|2|4|8>          MSAA samples for the HDR scene pass
                            (default: 4 = full quality; 0 =
                            single-sample). Draft only below 4.
    --shadow <SIZE>         Key shadow map size: 512 | 1024 | 2048 |
                            4096 (default: 2048 = full quality).
    --draft                 Draft preset: --res 0.5 --aa 0
                            --shadow 512. Any of the three flags
                            given after --draft overrides it.
                            Draft runs label every output line
                            DRAFT — never use for delivery.
```

### H4 — the substitution point (the plumbing order)

Insert after the `--film-telemetry` early-return block
(main.rs:271–284) and before `if let Some(dir) = &film_dir`
(main.rs:285):

```rust
    // F4 knobs (KNOBS-W43 §3.1): the effective size is computed
    // once, here, before any GPU dispatch. Every entry point
    // below receives (ew, eh) in place of (width, height), so
    // film's Renderer::new (film.rs:3406) is built at the
    // effective size — the substitution cannot land after it.
    let (ew, eh) = effective_size(width, height, knobs.res);
```

Why here and nowhere else: the two telemetry early-returns
above this point are CPU-only and must not see knob arithmetic
at all; every GPU dispatch below it takes the effective pair.
The call chain the brief flagged —
main.rs:286 → `film::run_film(ew, eh, …)` (film.rs:3399) →
`Renderer::new(width, height, …)` at film.rs:3406 — is safe
*structurally*: run_film passes its own parameters straight
through, so substituting before dispatch substitutes before
`Renderer::new`. There is no second size anywhere in run_film:
its header print, its overlay calls and its `RgbaImage` all
read the same two parameters.

### H5 — dispatch arms (main.rs:286–311)

| line | current | becomes |
|---|---|---|
| 286 | `film::run_film(width, height, dir, &out, film_from, film_to)` | `film::run_film(ew, eh, dir, &out, film_from, film_to, knobs)` |
| 292 | `scenefile::run_scene_file(width, height, &out, path)` | `scenefile::run_scene_file(ew, eh, &out, path, knobs)` |
| 296 | `run(width, height, &out)` (m1) | `run(ew, eh, &out, knobs)` |
| 308 | `ab1::run_ab1(width, height, &out, &caption, stage)` | `ab1::run_ab1(ew, eh, &out, &caption, stage, knobs)` |
| 310 | `dof::run_dof(width, height, &out)` | `dof::run_dof(ew, eh, &out, knobs)` |
| 311 | `battery::run_battery(&out)` | **unchanged** — the battery takes no size and consumes no knob |

`run` (m1, main.rs:464) gains the `knobs: Knobs` parameter but
consumes only the tag (§2.5). Per-mode applicability is W43
§3.3's table; an inert knob is accepted silently everywhere.

### H6 — still-mode tag site in main.rs (m1's `wrote`, main.rs:828)

Current: `println!("wrote {out} ({width}x{height})");`
Becomes: `println!("wrote {out} ({width}x{height}){tag}");`
with `tag` computed as in §2.5. (In `run`, the parameters are
the effective size, so the printed size is the rendered size —
at defaults, unchanged.)

### 2.5 The tag-placement rule (all modes)

One rule, prescribed here (W43 §3.4 names the lines, not the
mechanics): each consuming function computes once —

```rust
    let tag = if knobs.is_draft() {
        format!(" {}", knobs.tag())
    } else {
        String::new()
    };
```

— and appends `{tag}` at the **end** of each existing format
string. At defaults `tag` is empty, so every line is
byte-identical to today by construction. End-appending keeps
every documented line *head* intact (the film progress head is
quoted in PERF.md §2; the telemetry verdict lines are not in
these functions at all). The uniform `is_draft()` test is
deliberately conservative: a mode where a knob is inert (say
`--shadow` on dof-chart) still tags its output if that knob is
off default. The mislabel can only ever call a final a draft,
never the reverse.

---

## 3. gpucore.rs — no hunk

`gpucore::make_target` (gpucore.rs:141–160) and
`make_target_viewed` (165–178) already take `samples: u32` as a
parameter. The whole --aa plumbing is call-site work:

| file | target call sites (colour + depth twin) |
|---|---|
| film.rs | 2545 + 2551 (§5, hunk F3) |
| ab1.rs | inline descriptors at 846 + 859 (§6, hunk A3) |
| scenefile.rs | 1576 + 1578 (§7, hunk S1) |
| dof.rs | 240 + 242 (§8, hunk D1) |

---

## 4. The resolve branch — one shape, four copies

The one structural change (W43 §4.2). Both scene passes set the
resolve target unconditionally today; wgpu forbids a resolve
target on a single-sample attachment, so at `samples == 1` the
pass must render directly into the resolved view. Construction
shape prescribed for all four passes (film §5 F6, ab1 §6 A5,
scenefile §7 S3, dof §8 D3): build the attachment value before
`begin_render_pass`, choosing view and resolve together —

```rust
let scene_attachment = wgpu::RenderPassColorAttachment {
    view: if samples > 1 { &msaa_view } else { &resolved_view },
    resolve_target: if samples > 1 { Some(&resolved_view) } else { None },
    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
};
// in the descriptor:
color_attachments: &[Some(scene_attachment)],
```

At `samples > 1` the constructed value is field-for-field
today's attachment (same view, `Some` resolve, same ops), so
the default path is unchanged — the identity law rests on
this, and gate leg A is the check. The depth attachment is
**not** branched: the depth twin is created with the same
`samples` (§3 table), so it matches the colour attachment's
count in both arms.

Downstream is branch-free by design: every consumer reads the
resolved texture (film's bokeh bind group binds
`&self.hdr_view` at film.rs:3077; scenefile/dof read their
`scene_view`), which is the render target in the 1-sample arm
and the resolve target in the MSAA arm — same view either way.

**Risk (named in W43, restated):** at `samples == 1` the MSAA
texture is still created (with count 1) and simply never bound
— 16.6 MB of fp16 at 1080×1920, created and dropped. Skipping
its creation is *not* prescribed: `hdr_msaa_view` is a struct
field in film and a bound local in the other three, and making
it conditional introduces an Option through four files to save
draft-mode-only memory. Drafts are the only runs that pay it.

---

## 5. film.rs hunks

### F1 — Renderer fields (struct at film.rs:2338)

Add beside `width`/`height` (2341–2342):

```rust
    samples: u32,
    shadow_size: u32,
```

and the matching assignments in the `Ok(Renderer { … })`
literal (film.rs:~2730–2761; `hdr_view` is assigned at 2746):
`samples,` and `shadow_size,`.

### F2 — Renderer::new signature (film.rs:2383)

Current:

```rust
    pub(crate) async fn new(width: u32, height: u32, sign: &(Vec<u8>, u32, u32)) -> crate::errors::TaleResult<Renderer> {
```

Becomes (knobs appended; the single caller is run_film,
film.rs:3406):

```rust
    pub(crate) async fn new(width: u32, height: u32, sign: &(Vec<u8>, u32, u32), knobs: crate::Knobs) -> crate::errors::TaleResult<Renderer> {
```

Early in the body (before the targets at 2528):

```rust
        let samples = knobs.samples();
        let shadow_size = knobs.shadow;
```

### F3 — targets (film.rs:2528–2551)

Current (four lines, verbatim):

```rust
        let shadow_map = crate::gpucore::make_target(&device, 2048, 2048, wgpu::TextureFormat::Rgba8Unorm, 1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, "key shadow map");
        let shadow_depth = crate::composer::make_rt(&device, 2048, 2048, wgpu::TextureFormat::Depth32Float, "shadow depth");
        let hdr_msaa = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Rgba16Float, 4, wgpu::TextureUsages::RENDER_ATTACHMENT, "hdr msaa");
        let hdr_depth = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Depth32Float, 4, wgpu::TextureUsages::RENDER_ATTACHMENT, "hdr depth");
```

(2528, 2531, 2545, 2551 respectively; the packed-depth and
prepass targets between them stay at sample count 1 — §9.)

Replacements: `2048, 2048` → `shadow_size, shadow_size` in the
first two; the literal `4` → `samples` in the last two. Both
shadow targets and both MSAA targets move together — the
shadow pair is the size knob, the HDR pair is the AA twin
rule (§3).

### F4 — pipeline factories (film.rs:2606, 2647–2648)

- `make_pbr` body (closure at 2593), current line 2606:

  ```rust
                multisample: wgpu::MultisampleState { count: 4, ..Default::default() },
  ```

  → `count: samples`. (The closure captures the local by
  reference; it is called three times — `vs_colour`, `vs_tex`,
  `vs_map` — and all three pipelines write the scene target,
  so all three take the knob.)

- `make_basic` calls, current 2647–2648:

  ```rust
        let basic_write = make_basic(wgpu::TextureFormat::Rgba16Float, depth_state(true), 4);
        let basic_nowrite = make_basic(wgpu::TextureFormat::Rgba16Float, depth_state(false), 4);
  ```

  → trailing `4` → `samples` in both. (These basics draw into
  the HDR scene target — sky, cards, characters — so they are
  scene pipelines, not overlay pipelines. Film's post/overlay
  pipelines live in `composer::PostChain` and are untouched.)

- **Non-changes in the same region:** `make_depth`'s
  `MultisampleState::default()` (2670 — prepass + shadow
  pipelines, single-sample targets) and the bokeh pipeline's
  `MultisampleState::default()` (2713 — post) stay exactly as
  they are. Plumbing the AA value into either is a wgpu
  validation error against their targets.

### F5 — shadow_misc uniform lockstep (film.rs:2838, inside `Renderer::render`, fn at 2795)

Current:

```rust
        vecs.push([1.0 / 2048.0, 2048.0, 0.0, 0.0]);
```

Becomes:

```rust
        vecs.push([1.0 / self.shadow_size as f32, self.shadow_size as f32, 0.0, 0.0]);
```

This slot is the *only* channel by which the size reaches
`shader_pbr.wgsl` (the `shadow_misc` field; its comment at
shader_pbr.wgsl:51 still quotes the baked-in numbers — §9.3).
Identity check: at 2048, `1.0 / 2048.0f32` is exact (power of
two) and `2048 as f32` is exact, so the default uniform is
bit-identical to the literal.

### F6 — the scene pass resolve branch (film.rs:3175–3178)

Current (the `"film scene pass"` attachment):

```rust
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.hdr_msaa_view,
                    resolve_target: Some(&self.hdr_view),
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
                })],
```

After (attachment built just above `begin_render_pass`, using
the §4 shape with `self.samples`, `&self.hdr_msaa_view`,
`&self.hdr_view`, and this pass's `clear`):

```rust
            let scene_attachment = wgpu::RenderPassColorAttachment {
                view: if self.samples > 1 { &self.hdr_msaa_view } else { &self.hdr_view },
                resolve_target: if self.samples > 1 { Some(&self.hdr_view) } else { None },
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("film scene pass"),
                color_attachments: &[Some(scene_attachment)],
```

The pass's other three attachments (3091 shadow, 3131
prepass, 3232 final — all `resolve_target: None` against
single-sample targets) are untouched.

### F7 — run_film: signature, Renderer::new call, tags (film.rs:3399–3447)

Signature (3399) gains `, knobs: crate::Knobs`; the
`Renderer::new` call (3406) becomes
`Renderer::new(width, height, &assets.sign, knobs)`.

Tag (§2.5 rule), computed at the top of run_film. The three
current lines, verbatim, and their tagged forms:

Header (3408–3411):

```rust
    println!(
        "film: {} — {} frames @30fps, rendering {}..{} at {}x{} (clean PNG frames; decision B)",
        "THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER", frames_total, from, to, width, height
    );
```
→ format string gains `{tag}` after `(clean PNG frames; decision B)`.

Progress (3438–3441):

```rust
            println!(
                "  frame {n} (t = {t:.2}s) — {done} rendered, {skipped} skipped · {}",
                clock.timing_suffix((done + skipped) as u64, (to - from) as u64)
            );
```
→ `{tag}` appended after the final `{}`. (The suffix itself —
runlog.rs:150–152, `elapsed {MM:SS} · eta {MM:SS|--:--}` — is
not edited; runlog.rs has no hunks.)

Completion (3444–3447):

```rust
    println!(
        "film render complete: {done} rendered, {skipped} skipped (resume), range {from}..{to} · {} · {}",
        clock.summary_suffix(),
        cache.stats_line()
    );
```
→ `{tag}` appended after the final `{}` (summary_suffix is
runlog.rs:164–169, `elapsed … · peak RSS … MB`; the
composition order — counts, time/memory, cache stats — is the
RUNLOG-W09 contract and is preserved; the tag rides after it).

### F8 — the CPU overlays scale (film.rs:3315, 3365)

The designed path for `--res` (W43 §4.1): captions scale with
the frame. Both overlays already receive the frame's `(w, h)`
— post-substitution the effective size — so the transform is
computed inside each function from its own arguments, with no
signature change:

```rust
    let ux = w as f32 / 1080.0;
    let uy = h as f32 / 1920.0;
```

**Identity lemma (why this is safe at defaults):** at
1080×1920, `ux == uy == 1.0` exactly, and multiplication and
division by 1.0 are IEEE-exact for every finite value — so
every expression below evaluates to today's value bit-for-bit
at the default size, provided each scaled constant is written
as `today's expression * u` (same evaluation order, factor
last). Gate leg A is the empirical check; this is the reason
it can pass.

**F8a — `overlay_caption` (film.rs:3365–3386).** Current core:

```rust
fn overlay_caption(px: &mut [u8], w: u32, h: u32, assets: &Assets, idx: usize, scale: f32) {
    let strip = &assets.strips[idx];
    let (cx, cy) = (540.0f32, 1340.0f32);
    let (hw, hh) = (540.0 * scale, 150.0 * scale);
    ...
            let su = cx + (x as f32 - cx) / scale;
            let sv = cy + (y as f32 - cy) / scale;
            let src_x = su;
            let src_y = sv - (1340.0 - 150.0);
```

Transformed (centre and extents scale; the inverse map
divides by the *combined* scale; the strip is still sampled
in design coordinates, so `src_x`/`src_y` and the strip
bounds checks are unchanged):

```rust
    let (ux, uy) = (w as f32 / 1080.0, h as f32 / 1920.0);
    let (cx, cy) = (540.0f32 * ux, 1340.0f32 * uy);
    let (hw, hh) = (540.0 * scale * ux, 150.0 * scale * uy);
    ...
            let su = 540.0 + (x as f32 - cx) / (scale * ux);
            let sv = 1340.0 + (y as f32 - cy) / (scale * uy);
            let src_x = su;
            let src_y = sv - (1340.0 - 150.0);
```

The loop bounds (`ix0..ix1` from centre ± half-extents,
clamped to `(w, h)`) keep today's formulas — they read the
scaled centre/extents and the real frame size, so they are
correct in both spaces. Sampling note: the strip sample is
the pre-existing fit-scale law (`scale` already did exactly
this kind of point-mapped bilinear sampling before --res
existed); folding the frame scale into the same expressions
changes placement and size, not the sampling kind. The
caption *audit* stays design-space — `run_film_telemetry`
(film.rs:3523) has no hunks.

**F8b — `overlay_calendar` (film.rs:3315–3363).** Current
transform head:

```rust
    // drawCalendar transform: translate(352, 648) rotate(-0.03)
    // scale(0.92, 0.92*sy); plate local frame x [-180,180],
    // y [-220,220] at 2 px per unit.
    let (cx, cy) = (352.0f32, 648.0f32);
    let (rot, sx, sy) = (-0.03f32, 0.92f32, 0.92 * cal.sy);
```

Transformed:

```rust
    let (ux, uy) = (w as f32 / 1080.0, h as f32 / 1920.0);
    let (cx, cy) = (352.0f32 * ux, 648.0f32 * uy);
    let (rot, sx, sy) = (-0.03f32, 0.92f32 * ux, 0.92 * cal.sy * uy);
```

Everything downstream is formula-identical and reads the
primed values: `fwd` (rotate/scale about the centre), the
corner bounds, the inverse map (`(x − cx, y − cy)`,
un-rotate, divide by `(sx, sy)`), the local-frame range
checks `[-180, 180]` / `[-220, 220]`, and the plate sample
`(pu, pv) = ((lx + 180.0) * 2.0, (ly + 220.0) * 2.0)` — the
"2 px per unit" is *plate* space, not frame space, and does
not scale (W43 §4.1's warning was that one factor must cover
both overlays; here both derive `(ux, uy)` from the same
`(w, h)` arguments the same way, so they cannot disagree).
The dark-plate constants (`[30.0, 25.0, 18.0]`, the `0.22`
factor) are colours, not geometry — untouched.

**Disclosed behaviour change:** at a non-default
`--width/--height` *without* `--res`, the overlays now scale
to the frame instead of sitting at design-space positions
(the pre-existing misplacement W43 §1.3 documented). That is
the designed path the spec orders, it is identity at the
default size, and the landing commit message should say it
out loud.

---

## 6. ab1.rs hunks

`run_ab1` (ab1.rs:585) gains `, knobs: crate::Knobs`. At the
top of the body:

```rust
    let samples = knobs.samples();
    let shadow_size = knobs.shadow;
```

### A1 — shadow_misc lockstep (ab1.rs:652)

Current (inside the frame-uniform array):

```rust
        [1.0 / 2048.0, 2048.0, 0.0, 0.0],
```

→ `[1.0 / shadow_size as f32, shadow_size as f32, 0.0, 0.0],`
— bit-identical at 2048 by the same power-of-two argument as
F5.

### A2 — shadow target pair (ab1.rs:811, 820)

Current: the inline `create_texture` for `"key shadow map"`
with `size: wgpu::Extent3d { width: 2048, height: 2048, depth_or_array_layers: 1 }`
(811), and

```rust
    let shadow_depth = composer::make_rt(&device, 2048, 2048, wgpu::TextureFormat::Depth32Float, "shadow depth");
```

→ `width: shadow_size, height: shadow_size` in the Extent3d;
`shadow_size, shadow_size` in the make_rt call.

### A3 — the inline MSAA pair (ab1.rs:846, 859)

The two inline descriptors (`"hdr msaa"` at 842–852,
`"hdr depth"` at 855–865) each carry `sample_count: 4,` →
`sample_count: samples,`. Prescribed as an in-place
parameterisation (option A). Option B — replacing both blocks
with `gpucore::make_target_viewed`, as ab1 already does for
its final target at 870–878 — closes the last inline-creation
leftover W43 noted, but it is a larger diff through the
wave's regression oracle; take it as a separate later cleanup,
not inside the knobs landing. Risk on A: the two descriptors
must stay twins in everything except format — only the
`sample_count` line changes in each.

### A4 — pipeline factories (ab1.rs:927, 970–972)

- `make_pbr` (closure at 914), line 927:
  `multisample: wgpu::MultisampleState { count: 4, ..Default::default() },`
  → `count: samples`.
- `make_basic` calls:

  ```rust
    let basic_write = make_basic(wgpu::TextureFormat::Rgba16Float, depth_state(true), 4);
    let basic_nowrite = make_basic(wgpu::TextureFormat::Rgba16Float, depth_state(false), 4);
    let basic_overlay = make_basic(wgpu::TextureFormat::Rgba8Unorm, None, 1);
  ```

  → `samples` in the first two; **`basic_overlay` keeps its
  literal 1** — it writes the single-sample final target (the
  caption composite), and plumbing the knob into it is a
  validation error. This line is the ab1 trap of the patch;
  it is one line below the two that change.

- Non-changes: `make_depth`'s `MultisampleState::default()`
  (996) and the bokeh pipeline's (1062).

### A5 — the scene pass resolve branch (ab1.rs:1416–1419)

Current (`"ab1 scene pass"`):

```rust
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &hdr_msaa_view,
                resolve_target: Some(&hdr_view),
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }), store: wgpu::StoreOp::Store },
            })],
```

→ the §4 shape with the locals (`samples`, `&hdr_msaa_view`,
`&hdr_view`, this pass's literal clear colour). ab1's views
are function locals, so the branch reads the local `samples`
directly — no struct field needed here (contrast F1/F6).

### A6 — wrote line (ab1.rs:1547)

`println!("wrote {out} ({width}x{height})");` → append
`{tag}` (§2.5; `tag` computed in run_ab1).

---

## 7. scenefile.rs hunks

`run_scene_file` (scenefile.rs:1220) gains
`, knobs: crate::Knobs`, forwarded to `run_composer`
(scenefile.rs:1522), which gains the same parameter and
derives `let samples = knobs.samples();`. (`--shadow` is
inert in both scene-file paths — no shadow target exists —
and `--res` arrives via the size parameters, per §3.3.)

### S1 — targets (scenefile.rs:1576, 1578)

```rust
    let scene_msaa = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Rgba16Float, 4, wgpu::TextureUsages::RENDER_ATTACHMENT, "composer-scene-msaa");
    let scene_depth = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Depth32Float, 4, wgpu::TextureUsages::RENDER_ATTACHMENT, "composer-scene-depth");
```

→ `samples` for both literals. The resolved target between
them (1577) stays at 1 — it is the branch's destination, not
a knob site. (W43 named only the colour line; the depth twin
at 1578 is the §0.2 catch.)

### S2 — scene pipelines (scenefile.rs:1637, 1681)

Both `multisample: wgpu::MultisampleState { count: 4, ..Default::default() },`
— the composer scene tex pipeline (1637) and the composer
scene colour pipeline (1681) — → `count: samples`.
Non-changes in this function: the depth-prepass pipeline
(1730), fog (1782), bokeh (1832) and output (1879)
`MultisampleState::default()` states.

### S3 — the composer scene pass resolve branch (scenefile.rs:1967–1973)

Current head of the pass (views are locals, created at
1967–1968):

```rust
        let msaa_view = scene_msaa.create_view(&wgpu::TextureViewDescriptor::default());
        let sdepth_view = scene_depth.create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composer scene pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &msaa_view,
                resolve_target: Some(&scene_view),
```

→ the §4 shape with `&msaa_view` / `&scene_view` and this
pass's `clear` (the `scene.clear`-derived `wgpu::Color`).
Depth attachment (`&sdepth_view`) unchanged. At
`samples == 1`, `msaa_view` is created but unbound — the §4
risk note applies; no warning arises since the view is still
constructed and dropped normally.

### S4 — wrote lines (scenefile.rs:1486, 2132)

Both `println!("wrote {out} ({width}x{height})");` (1486 =
simple path, 2132 = composer path) → append `{tag}`. The
simple path consumes only `--res`; the tag rule is the
uniform §2.5 one.

### S5 — simple path: no other hunks

Its targets (scenefile.rs:1110, 1420, 1431) are
`sample_count: 1` end to end and its pass (1448) already has
`resolve_target: None`. There is no MSAA stage to knob —
`--aa` is accepted and inert there, by spec.

---

## 8. dof.rs hunks

`run_dof` (dof.rs:135) gains `, knobs: crate::Knobs` and
derives `let samples = knobs.samples();`. (`--shadow` inert:
no shadow map in this path.)

### D1 — targets (dof.rs:240, 242)

```rust
    let scene_msaa = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Rgba16Float, 4, wgpu::TextureUsages::RENDER_ATTACHMENT, "dof-scene-msaa");
    let scene_depth = crate::gpucore::make_target(&device, width, height, wgpu::TextureFormat::Depth32Float, 4, wgpu::TextureUsages::RENDER_ATTACHMENT, "dof-scene-depth");
```

→ `samples` in both (the resolved target at 241 stays 1).

### D2 — scene pipeline (dof.rs:281)

`multisample: wgpu::MultisampleState { count: 4, ..Default::default() },`
(the `"dof-scene-pipeline"`) → `count: samples`.
Non-changes: depth (307), bokeh (366), output (399) defaults.

### D3 — the scene pass resolve branch (dof.rs:406–412)

Current (`"dof-scene-pass"`, views are locals at 406–407):

```rust
        let msaa_view = scene_msaa.create_view(&wgpu::TextureViewDescriptor::default());
        let sdepth_view = scene_depth.create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("dof-scene-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &msaa_view,
                resolve_target: Some(&scene_view),
```

→ the §4 shape, clear colour `wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }`.

### D4 — wrote line (dof.rs:500)

The long descriptive line —

```rust
        "wrote {out} ({width}x{height}) — scene dof-chart: panels at 2..30 m, focus {FOCUS} m, aperture {APERTURE}, maxblur {MAXBLUR}; chain: HalfFloat MSAA-4x scene -> packed-depth prepass -> transcribed r170 bokeh -> Neutral+sRGB output"
```

→ append `{tag}` at the very end. Note the line *describes*
the chain as "MSAA-4x": at an off-default `--aa` the
description and the tag disagree, and the tag is the
operative provenance (W43 §3.4's rule is that the tag lists
effective values). Rewording the description is not
prescribed — the line is quoted as provenance in M3-era
evidence, and changing its head at defaults would break the
identity law for stdout. Flagged here so the lead decides
with eyes open; the conservative reading is: tag wins.

---

## 9. Explicit non-sites, and the remaining risks

### 9.1 Do-not-plumb table (changing any of these breaks the identity law or wgpu validation)

| site | why it stays |
|---|---|
| `composer::make_rt` (composer.rs:72–88) + all PostChain pipelines | single-sample by architecture; the resolved texture is their input |
| film.rs:2670 (`make_depth`), 2713 (bokeh) — `MultisampleState::default()` | prepass/shadow/post pipelines against single-sample targets |
| ab1.rs:996 (`make_depth`), 1062 (bokeh) — same | same |
| ab1.rs:972 `basic_overlay` literal `1` | writes the single-sample final target (the A4 trap) |
| scenefile.rs:1730 / 1782 / 1832 / 1879 defaults | prepass / fog / bokeh / output |
| dof.rs:307 / 366 / 399 defaults | depth / bokeh / output |
| packed-depth + prepass-depth targets everywhere (film 2539, scenefile 1579–1580, dof 245–246) | `sample_count: 1` data targets, not the scene colour pair |
| film.rs:2425 (texture-upload closure) | uploaded images, not render targets |
| shadow *depth* pipelines (`make_depth` family) | they render into the shadow targets at count 1 — the shadow map itself is single-sample at every knob value; only its *size* is knobbed |
| `run_film_telemetry` (film.rs:3523), `telemetry::run` | CPU, design-space; the 850 px caption audit is never scaled (W43 §1.3) |
| paint canvas / paint cache (2560×2816) | CPU-side; a draft does not get cheaper paints (W43 §1.3) |
| `battery::run_battery` | no size, no knobs |

### 9.2 Decisions this pack takes that the lead confirms at landing

1. **Last-writer-wins** for `--draft` vs the individual flags
   in both orders (§2, H2). W43 specified only
   flags-after-draft.
2. **Uniform tag rule** (§2.5): an inert-but-off-default knob
   still tags a mode's output. Conservative by design.
3. **Attachment construction** (§4) instead of an if/else
   around the whole descriptor — field-for-field identical at
   samples > 1.
4. **MSAA texture still created at samples == 1** (§4 risk).
5. **dof's "MSAA-4x" description line** keeps its head; the
   tag carries the truth (§8, D4).

### 9.3 Comment-only optional hunk

`src/shader_pbr.wgsl:51` —

```wgsl
    // x: shadow texel size (1/2048), y: shadow map size (2048)
```

→ `// x: shadow texel size (1/size), y: shadow map size (size)`.
Comment only; no shader semantics. (W43 §1.2 asked for this
rewording when the knob lands.)

### 9.4 `--aa 8` adapter risk (from W43's report, restated)

MSAA 8× on `Rgba16Float` exceeds some adapters' limits. When
it does, texture/pipeline creation fails as a wgpu validation
error — a hard, visible failure, which is the lawful outcome
(the no-silent-change law cuts both ways: never clamp 8 → 4).
The gate does not exercise `--aa 8`; the PC evidence session
should record whether RADV takes it. Same for `--shadow
4096` on downlevel-class limits — visible failure, no clamp.

---

## 10. Acceptance — tools/ci/knobs_gate.py

The gate (this seat's second deliverable) implements W43 §5's
acceptance as three legs, all driven off the standing probe
ranges and `byte_probes.py` **imported** (the house importlib
pattern, same as frame_hash_compare.py) — never forked:

- **Leg A — defaults identity.** The three standing ranges
  (195-204, 795-814, 1195-1204 — `byte_probes.DEFAULT_RANGES`)
  rendered twice with the same binary: no knob flags, then
  `--res 1 --aa 4 --shadow 2048`. Requires `BYTE PROBES: PASS`
  (40/40) and no `DRAFT` substring in either run's stdout.
  This is the per-knob gate as well as the combined one: all
  three defaults are spelled in the second run, so a plumbing
  slip in any single knob (a shadow default of 1024, an aa
  default of 2) changes bytes and fails this leg.
- **Leg B — draft smoke.** Frames 195–219 with `--draft`
  (25 frames, so exactly one progress line prints). Asserts:
  exit 0; the exact tag `[DRAFT res=0.5 aa=0 shadow=512 — NOT
  FOR DELIVERY]` in the header line and the completion line;
  a DRAFT tag in the progress line; frames 195–204 **differ**
  from leg A's plain frames (byte_probes must FAIL them —
  identical output would mean the knobs are decorative); and
  the draft frame's IHDR size equals the spec arithmetic on
  the plain size (1080×1920 → 540×960).
- **Leg C — CLI validation.** Ten bad-value/missing-value
  cases against `--scene m1` (validation fires in the parse
  loop, before any GPU work): exit 2, `error:` on stderr, and
  **not** the `unknown argument` refusal — which would mean
  the flag doesn't exist. This leg is what keeps a pre-knobs
  binary from passing the gate by accident.

Verdict discipline mirrors determinism.yml: report written to
`<work>/knobs-results.txt`, final line exactly
`KNOBS GATE: PASS` / `KNOBS GATE: FAIL`, exits 0/1, and exit
2 with **no verdict line** for refusals (missing binary/film
dir, a pre-populated run dir — the driver's resume-skip would
poison a comparison, so the gate refuses rather than cleans).

CI wiring sketch (a new dispatch workflow, determinism.yml's
shape — for the lead to add; workflows are not this seat's
lane):

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

**Which legs are CI-only, exactly:** legs A, B and C drive a
real binary, so all three are CI-only until this pack lands —
no knobs binary exists before that — and the film legs
additionally cannot run on this development VM at all (its
llvmpipe segfaults compiling shader_pbr.wgsl; see the
render.yml header). They run where film renders run: CI
lavapipe (ubuntu-22.04) or the user's RADV PC. What runs
here, and did:

```
$ python3 tools/ci/knobs_gate.py --self-test
  ok: identity: frames4 vs itself passes byte_probes
  ok: difference: one flipped byte fails byte_probes
  ok: png_size reads frames4 frame 195 as 1080x1920
  ok: spec arithmetic: 1080x1920 at res 0.5 -> 540x960
  ok: spec arithmetic: res 1 returns the base size exactly (even for an odd base)
  ok: tag logic: defaults stdout is clean
  ok: tag logic: drafted stdout passes every check
  ok: tag logic: untagged film stdout fails draft checks
SELF-TEST: PASS
```

The self-test exercises the imported comparison path (frames4
vs itself over the real probe ranges, plus a one-byte-flip
difference), the IHDR size reader against a real frames4
frame, the effective-size arithmetic, and the stdout tag
logic against synthetic header/progress/completion lines. It
makes no claim about the binary legs.

**Residual gap, stated plainly:** the gate exercises the
knobs at their defaults (leg A), all-off-default together
(leg B), and invalid values (leg C). A defect that manifests
only at one intermediate value alone (say `--aa 2` with the
rest at defaults) would pass. That matrix belongs to the PC
evidence session as spot checks; it is not gated here, and
this pack does not pretend otherwise.

No performance claims are made anywhere in this pack or the
gate: what the knobs save is measured later, on PC/CI
evidence (PERF.md discipline).

---

## 11. Hunk index

| file | hunks | lines at `aa92cee` |
|---|---|---|
| src/main.rs | Knobs type + parse_res/parse_aa/parse_shadow + effective_size (new code, §1) | insert after 153 |
| src/main.rs | H1 locals | 174–175 |
| src/main.rs | H2 parse arms | after ~247 |
| src/main.rs | H3 HELP text | after 105 |
| src/main.rs | H4 substitution point | before 285 |
| src/main.rs | H5 dispatch | 286, 292, 296, 308, 310 (311 unchanged) |
| src/main.rs | H6 m1 wrote + `run` signature | 828; 464 |
| src/gpucore.rs | — none — | — |
| src/film.rs | F1 fields + constructor literal | 2341–2342; ~2730–2761 |
| src/film.rs | F2 Renderer::new signature + locals | 2383; before 2528 |
| src/film.rs | F3 targets | 2528, 2531, 2545, 2551 |
| src/film.rs | F4 factories | 2606; 2647–2648 (2670, 2713 unchanged) |
| src/film.rs | F5 shadow_misc | 2838 |
| src/film.rs | F6 scene-pass resolve branch | 3175–3178 |
| src/film.rs | F7 run_film signature/call/tags | 3399; 3406; 3408–3411; 3438–3441; 3444–3447 |
| src/film.rs | F8 overlays | 3315 (calendar), 3365 (caption) |
| src/ab1.rs | A1 shadow_misc | 652 |
| src/ab1.rs | A2 shadow targets | 811, 820 |
| src/ab1.rs | A3 inline MSAA pair | 846, 859 |
| src/ab1.rs | A4 factories | 927; 970–972 (972 unchanged) |
| src/ab1.rs | A5 scene-pass resolve branch | 1416–1419 |
| src/ab1.rs | A6 wrote | 1547 |
| src/ab1.rs | run_ab1 signature + locals | 585 |
| src/scenefile.rs | S1 targets | 1576, 1578 |
| src/scenefile.rs | S2 pipelines | 1637, 1681 |
| src/scenefile.rs | S3 composer resolve branch | 1967–1973 |
| src/scenefile.rs | S4 wrote ×2 | 1486, 2132 |
| src/scenefile.rs | S5 simple path — no hunks | (1110, 1420, 1431, 1448 confirmed) |
| src/scenefile.rs | signatures | 1220, 1522 |
| src/dof.rs | D1 targets | 240, 242 |
| src/dof.rs | D2 pipeline | 281 |
| src/dof.rs | D3 scene-pass resolve branch | 406–412 |
| src/dof.rs | D4 wrote | 500 |
| src/dof.rs | run_dof signature + locals | 135 |
| src/shader_pbr.wgsl | §9.3 comment (optional) | 51 |
| src/runlog.rs, src/composer.rs | — none — | — |
