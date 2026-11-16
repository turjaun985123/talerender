# GPUFIRST INV-A — CPU Painter Raster-Model Specification (parity target for a GPU port)

**Seat:** INV-A, GPU-first TaleRender Phase 0.
**Repository / HEAD audited:** `~/workspace/talerender` at
`bc12a5c2bbb33d93416f25eaebbe8fec8c4eb678` (`git rev-parse HEAD`, verified
2026-11-16 before writing). All line citations below are
`src/scene.rs:<line>` unless prefixed `toonfull.rs:`; both files were read at
that HEAD. No code was changed and nothing is committed by this seat.

**Parity target — read this first.** The parity target is *this CPU painter*,
byte-for-byte, not Skia/Canvas2D. The CPU painter is itself a measured
*approximation* of Skia: the stroke battery
(`docs/ab/stroke-battery/README.md`) records PASS 46/46 classes within stated
tolerances (width/edge ≤ 0.5 device px, ramp ≤ 0.5 px, profile RMSE ≤ 0.10;
crops band RMSE ≤ 0.12), **not** equality with Skia, and
`docs/ab/stroke-battery/gaps.md` enumerates the known residual deltas
(flattening, binary clip masks, exact-disc joins vs Skia's flattened join
arcs, the radial CapFan model). A GPU port that is "closer to Skia" than the
CPU painter is a parity **failure** under this spec. Every approximation in
§2–§4 below — the 4×4 binary supersample, the gradient-normalised ellipse
distance, the CapFan radial model, binary masks — must be reproduced as-is,
not improved.

**Conventions used in this spec.**

- *Sample centre* of pixel `(x, y)` is `(x + 0.5, y + 0.5)` in f32. Every
  primitive, mask, and direct caller loop samples there
  (e.g. `scene.rs:576`, `scene.rs:628`, `scene.rs:729`,
  `scene.rs:754`, `scene.rs:771`).
- *Signed distance* `sd` is **negative inside** the shape, positive outside,
  zero on the boundary (stated at `scene.rs:562-564`; realised by
  `ellipse_sd` `scene.rs:821-831`, by `seg_dist − hw` in strokes
  `scene.rs:663-667`, and by the sign flip in `fill_polygon`
  `scene.rs:721-725`).
- All raster arithmetic is **f32** (`Painter` fields and every helper
  signature, `scene.rs:493-499`, `scene.rs:821`, `scene.rs:840`). Caller-side
  geometry in `toonfull.rs` sometimes computes in f64 and casts to f32 before
  reaching the painter (arm FK: `toonfull.rs:963-967` region uses
  `(a as f64).to_radians()` and `as f32` products); that is caller geometry,
  outside the painter, but an end-to-end GPU port must preserve it too.
- The framebuffer is a flat `Vec<u8>`, RGBA, **straight (non-premultiplied)
  alpha**, row 0 = top, index `k = (y * w + x) * 4`
  (`scene.rs:493-497`, `scene.rs:514`). It starts zeroed
  (`scene.rs:507-509`).

---

## 1. Shared machinery

### 1.1 `bbox` — the only clipping rule (`scene.rs:536-543`)

```rust
( x0.floor() as i32).max(0),
( y0.floor() as i32).max(0),
( x1.ceil() as i32).min(w as i32 - 1),
( y1.ceil() as i32).min(h as i32 - 1)
```

- Returns an **inclusive** integer box `(x0, y0, x1, y1)`; every primitive
  loops `for y in y0..=y1 { for x in x0..=x1 { … } }`.
- Min corner uses `floor`, max corner uses `ceil`, then each is clamped to
  the canvas (`0` / `w-1` / `h-1`). There is no other clipping anywhere in
  the painter except `blend`'s own per-pixel bounds check (§2.1).
- Each primitive supplies its own padding around its geometric extent; the
  paddings differ per primitive and are part of the model (§3 table). A GPU
  port must compute dispatch extents from the same padded boxes, or a
  superset that provably cannot change results (extra pixels evaluate to
  coverage 0 — but see §6.3: extra pixels are only safe if coverage is
  *exactly* 0 there, which holds for the padded boxes as argued per
  primitive; do not shrink any box).

### 1.2 `cov_of` — centre-distance ramp (`scene.rs:565-567`)

```rust
cov_of(sd) = (0.5 - sd).clamp(0.0, 1.0)
```

Linear ramp: coverage 1 at `sd ≤ -0.5`, 0 at `sd ≥ +0.5`, `0.5 - sd` between.
See §1.3 for the crucial restriction on where this ramp is actually reached.

### 1.3 `cov_area` / `cov_area0` — the coverage rule (`scene.rs:915-934`)

`cov_area(dist, x, y)` is `cov_area0(dist, x, y, dist(x, y))`
(`scene.rs:915-917`). `cov_area0` (`scene.rs:920-934`):

```rust
if d0.abs() < 1.0 {                       // strict <
    inside = count over oy, ox ∈ {-0.375, -0.125, +0.125, +0.375}²
             of ( dist(x + ox, y + oy) < 0.0 )     // strict <
    cov = inside as f32 / 16.0
} else {
    cov = cov_of(d0)
}
```

Exact properties a GPU port must reproduce:

1. **Band:** the supersample branch is taken iff `|d0| < 1.0` (strict),
   where `d0` is the distance at the pixel **centre** (`x, y` arguments are
   already centre coordinates — callers pass `x as f32 + 0.5`,
   e.g. `scene.rs:576`).
2. **Supersample grid:** 4×4 = 16 sub-positions at centre ± {0.125, 0.375}
   on each axis (offsets listed at `scene.rs:923-924`). The centre itself is
   **not** a sub-sample. Each sub-sample contributes a **binary** inside
   test, `dist < 0.0` strict — a sub-sample exactly on the boundary
   (`dist == 0.0`) counts as **outside**.
3. **Quantisation of coverage:** in the band, coverage is exactly
   `k / 16`, `k ∈ 0..=16` — multiples of 0.0625. There is no averaging of
   `cov_of` over sub-points (the header comment at `scene.rs:903-914`
   records that doing so was measured to widen straight edges and was
   rejected; code matches the comment).
4. **Outside the band** (`|d0| ≥ 1.0`): `cov_of(d0)` is returned, and since
   `|d0| ≥ 1.0` implies `0.5 - d0 ≤ -0.5` or `≥ 1.5`, the clamped result is
   **always exactly 0 or exactly 1**. ⚠️ **Discrepancy flag D1 (§7):** the
   non-trivial part of the `cov_of` ramp (`-0.5 < sd < +0.5`) is therefore
   *never reached through `cov_area0`* — it is pre-empted by the supersample
   branch. The ramp's fractional values occur in production only where
   `cov_of` is called **directly** (§5.1, head shade). Any GPU design that
   implements "ramp everywhere, supersample nowhere" or "supersample with a
   wider/narrower band" changes the model.
5. `d0` is an input, not recomputed inside the branch decision: callers that
   already have a centre distance pass it (`fill_polygon`,
   `stroke_polyline`); `cov_area` computes it with one `dist` call. In the
   supersample branch `d0` itself is otherwise unused — coverage comes
   solely from the 16 sign tests.
6. Discontinuity at the band edge is part of the model: at `|d0|` just
   under 1.0 coverage is a 16-step sign count (which for a straight edge is
   necessarily 0 or 16 at that distance, so straight edges are continuous);
   for curved boundaries a 1-ulp difference in `d0` near `|d0| = 1` can in
   principle switch branches. Reproduce the branch, do not smooth it.

### 1.4 `seg_dist` / `seg_dist_t` (`scene.rs:834-849`)

```rust
(dx, dy) = (bx - ax, by - ay)
len2 = dx*dx + dy*dy
t = if len2 > 0.0 { ((px-ax)*dx + (py-ay)*dy) / len2 }.clamp(0.0, 1.0)
    else { 0.0 }
dist = (px - (ax + dx*t)).hypot(py - (ay + dy*t))
```

- `seg_dist` is `seg_dist_t(...).0` (`scene.rs:834-836`).
- `t` is the clamped projection parameter: exactly `0.0` when the projection
  falls at/before `a` (or the segment is degenerate, `len2 == 0`), exactly
  `1.0` at/after `b`. These exact values are load-bearing: `stroke_polyline`
  gates CapFan substitution on `t == 0.0` / `t == 1.0` (§3.3).
- Distance is via f32 `hypot` (see §2.3 on `hypot` portability).
- Unsigned: always `≥ 0`. Sign is supplied by callers.

### 1.5 `ellipse_sd` (`scene.rs:821-831`)

```rust
(s, c) = rot.sin_cos()
(dx, dy) = (x - cx, y - cy)
(lx, ly) = (dx*c + dy*s, -dx*s + dy*c)          // rotate by -rot
g  = sqrt( (lx/rx)² + (ly/ry)² )               // .powi(2), .sqrt()
if g == 0.0 { return -rx.min(ry) }
grad = sqrt( (lx/(rx*rx))² + (ly/(ry*ry))² ) / g
return (g - 1.0) / grad
```

- This is an **approximate** signed distance: the implicit ellipse function
  `g - 1` normalised by its gradient magnitude. Exact for circles
  (`rx == ry` ⇒ result `= |(x,y) - centre| - rx`); approximate for eccentric
  ellipses (header comment `scene.rs:817-820`). A GPU port must use *this*
  formula — a true ellipse SDF is a parity break.
- `sin_cos` is evaluated **per call** (per sample point), not hoisted per
  primitive. Mathematically `s, c` are the same for every call with the same
  `rot`; bit-exactness only requires the same *values*, so a GPU port may
  precompute `sin_cos(rot)` once per primitive **on the CPU** and pass it
  in — but must not recompute it with a GPU `sin`/`cos` pair and assume
  equality with Rust libm's `sin_cos` (§2.3).
- Special case: exact centre (`g == 0.0`) returns `-min(rx, ry)`.
- Division chain: two divisions by `rx`/`ry` (inside `powi(2)` terms), one
  `sqrt`, then `grad` has its own `sqrt` and a division by `g`, then the
  final division by `grad`. Sign of the result = sign of `g - 1` (since
  `grad > 0` for `g > 0`), i.e. sign = sign of `(lx/rx)² + (ly/ry)² - 1`.

### 1.6 `point_in_polygon` (`scene.rs:936-950`)

```rust
inside = false
for i in 0..n {                       // edge pts[i] -> pts[(i+1) % n]
    (xi, yi) = pts[i]; (xj, yj) = pts[(i+1) % n]
    if (yi > y) != (yj > y) {
        xint = xi + (y - yi) / (yj - yi) * (xj - xi)
        if x < xint { inside = !inside }
    }
}
```

- Classic even-odd ray casting, ray in `+x` direction. Half-open in `y`:
  an edge toggles only when exactly one endpoint is strictly above the
  sample ordinate (`(yi > y) != (yj > y)`), which also guarantees
  `yj - yi ≠ 0` in the division. The `x` test is strict (`x < xint`); a
  sample exactly on `xint` does not toggle for that edge.
- The polygon is closed implicitly by the `% n` wrap edge. If the caller
  already duplicated the first point at the end, the wrap edge is
  zero-length (`yi == yj`) and never toggles — harmless.
- Result depends only on the *parity* of toggles; per-edge `xint` values are
  computed independently, so edge iteration order cannot change the result
  (no accumulated floating-point sum). Boundary outcomes can still flip on
  a 1-ulp difference in `xint` when `x ≈ xint`.

### 1.7 Flattening (`scene.rs:952-976`)

- `flatten_quad(p0, p1, p2, n)` (`scene.rs:952-963`): returns `n + 1` points,
  `t = i / n` for `i in 0..=n`, evaluated in the Bernstein form
  `a²·p0 + 2ab·p1 + b²·p2` with `a = 1 - t`, `b = t`, per coordinate, in the
  written left-to-right term order.
- `flatten_cubic(p0, p1, p2, p3, n)` (`scene.rs:965-976`): `n + 1` points,
  `u³·p0 + 3u²t·p1 + 3ut²·p2 + t³·p3`, `u = 1 - t`, using `.powi(3)` /
  `.powi(2)`.
- Fixed segment counts at call sites are part of the geometry: `stroke_quad`
  uses `n = 24` (`scene.rs:698`); `stroke_ellipse` uses 144 (§3.2); toon
  limbs use `flatten_quad(…, 20)` (`toonfull.rs:782`, `toonfull.rs:963`);
  the body bean uses four `flatten_cubic(…, 28)` chains
  (`toonfull.rs:796-799`). Flattening is done by the *caller / primitive on
  the CPU side* in the current architecture; a GPU port that flattens on
  GPU must reproduce these f32 evaluations point-for-point, including term
  order and `powi` lowering (§2.3).

### 1.8 `CapFan` / `cap_fan` / `fan_sd` — the flattened round cap

**Construction** (`scene.rs:861-879`). Fields: centre `c`, unit outward
`axis`, radius `r` (= half stroke width), chord count `n`, and
`cos_half = cos(π / (2n))`.

```rust
cos_max = (1.0 - 0.25 / r).clamp(-1.0, 1.0)
n = ceil( π / (2 * acos(cos_max)) ) as usize, .max(1)
cos_half = cos( π / (2n) )
```

- The 0.25 is Skia's flattening tolerance in device px (header comment
  `scene.rs:851-860`): chords are chosen so the sagitta
  `r·(1 - cos(π/(2n))) ≤ 0.25`. At `r = 15`, `n = 9` and the tip chord sits
  at `r·cos(10°)`. For `r ≤ 0.25`, `1 - 0.25/r ≤ 0` clamps `cos_max` toward
  `-1`, giving `n = 1`. `n` is derived from the rule, never fitted per call.
- Construction uses `acos` and `cos` **once per cap** (two caps per open
  polyline). A GPU port should construct fans on the CPU and upload the five
  fields; constructing them with GPU transcendentals risks a different `n`
  (a step function of `acos` — a 1-ulp `acos` difference near a `ceil`
  boundary changes the chord count and hence the whole cap shape).

**Evaluation** (`fan_sd`, `scene.rs:884-901`):

```rust
(vx, vy) = (qx - c.x, qy - c.y)
r_q = hypot(vx, vy)
if r_q < 1e-6 { return -r * cos_half }
cosang = ((vx*axis.x + vy*axis.y) / r_q).clamp(-1.0, 1.0)
if cosang < 0.0 { return r_q - r }              // behind cap plane: disc term
phi = acos(cosang)                              // 0..=π/2 from outward axis
dth = π / n
k = clamp( floor((π/2 - phi) / dth) as i32, 0, n-1 ) as usize
phi_mid = π/2 - (k + 0.5) * dth
rb = r * cos_half / cos(phi - phi_mid)
return r_q - rb
```

- This is a **radial model** of the chord polygon, not the polygon itself:
  exact at chord vertices and chord midpoints, within ~0.05 px elsewhere at
  production radii (comment `scene.rs:881-883`; gaps.md item 7). Reproduce
  the model, not the true chord polygon.
- Behind the cap plane (`cosang < 0`) it degenerates to the exact disc term
  `r_q - r`, identical to what the terminal segment's `seg_dist - hw` would
  give at clamped `t` (distance to the vertex is `r_q`). So the fan only
  ever **shrinks** the cap relative to the disc: in front of the plane,
  `|phi - phi_mid| ≤ dth/2` ⇒ `cos(phi - phi_mid) ≥ cos_half` ⇒
  `rb ≤ r` ⇒ `fan_sd ≥ r_q - r`. The fan boundary lies inside or on the
  disc everywhere; maximum inward shift is the sagitta `r(1 - cos_half)
  ≤ 0.25` at the tip.
- `acos` is evaluated **per `fan_sd` call** — i.e. per centre/sub-sample
  point that reaches the terminal segment's fan branch. This is the single
  most transcendental-dense spot in the painter (§2.3, §4).

---

## 2. Blend arithmetic — exact

### 2.1 `blend` (`scene.rs:510-535`)

```rust
if x, y outside [0, w) × [0, h) { return }
k = (y * w + x) * 4
sa = src.3 as f32 / 255.0
if sa <= 0.0 { return }                          // only src.3 == 0 can hit this
if sa >= 1.0 { px[k..k+4] = src bytes; return }  // only src.3 == 255
da = px[k+3] as f32 / 255.0
oa = sa + da * (1.0 - sa)
if oa <= 0.0 { return }                          // unreachable, see D4 (§7)
for c in 0..3 {
    s = src[c] as f32 / 255.0;  d = px[k+c] as f32 / 255.0
    px[k+c] = (((s*sa + d*da*(1.0 - sa)) / oa) * 255.0).round() as u8
}
px[k+3] = (oa * 255.0).round() as u8
```

- **Straight-alpha source-over**: output alpha `oa = sa + da·(1 - sa)`;
  output colour `(s·sa + d·da·(1 - sa)) / oa` per channel, all in f32 with
  operands normalised by `/255.0`.
- **Fast paths are semantic, not just optimisations:** `src.3 == 0` is a
  no-op (destination untouched, including its alpha); `src.3 == 255`
  **overwrites all four bytes verbatim** — it does not go through the
  formula (the formula would be mathematically equal but would round-trip
  the colour through f32; the copy is exact). A GPU port must keep both
  paths or prove the formula path reproduces the copy for every input —
  the safe choice is to keep them.
- **Rounding:** Rust `f32::round()` = round **half away from zero**, then
  `as u8` truncates (values are mathematically in `[0, 255]`, so truncation
  after rounding is a floor of a non-negative value). There is **no clamp**
  in `blend` (contrast `blend_cov`, §2.2). ⚠️ WGSL's built-in `round()` is
  round **half to even** — using it directly is a parity break on exact
  `.5` results, which are *common* here because all inputs are `u8/255`
  rationals. A GPU port must implement round-half-away-from-zero
  explicitly (for the non-negative values at every rounding site in this
  painter, `floor(v + 0.5)` is equivalent — but only because the values are
  provably `≥ 0`; do not apply that substitution to any signed quantity).
- Channels are independent: each channel reads its own destination byte
  before writing it; there is no cross-channel term.

### 2.2 `blend_cov` (`scene.rs:549-561`)

```rust
if cov <= 0.0 { return }
if cov >= 1.0 { blend(x, y, src); return }
a = (src.3 as f32 * cov).round().clamp(0.0, 255.0) as u8
if a > 0 { blend(x, y, (src.0, src.1, src.2, a)) }
```

- **Coverage is quantised to a u8 alpha *before* blending**, in f32:
  `a = round(src_a · cov)`, clamped to `[0, 255]`, truncated to `u8`. The
  comment at `scene.rs:544-548` ("coverage multiplies the source alpha …
  Quantised to u8") understates this: it is not a continuous multiplication
  inside the blend — it is a **separate rounding step** whose result is
  then fed to `blend` as if it were the source alpha. ⚠️ Discrepancy flag
  D5 (§7) — code wins.
- Consequences that must be preserved:
  - Effective alpha is `a / 255`, not `cov · src_a / 255`. E.g. `src_a =
    255`, `cov = 1/16` ⇒ `a = round(15.9375) = 16` ⇒ effective `16/255 ≈
    0.06275`, not `0.0625`.
  - Small coverages can vanish: `round(src_a · cov) == 0` ⇒ no-op, even
    though `cov > 0`.
  - Tie cases are frequent: with `cov = k/16` (the supersample branch),
    `src_a · cov` is a multiple of `1/16` and lands exactly on `.5`
    whenever `src_a · k ≡ 8 (mod 16)` — the half-away vs half-even
    distinction in §2.1 applies here too.
  - `cov ≥ 1.0` bypasses quantisation entirely and blends with the
    original `src` (so an opaque source at full coverage hits `blend`'s
    verbatim-copy path).
  - Coverage values produced inside the painter are in `[0, 1]` by
    construction (`cov_of` clamps; `cov_area0` returns `k/16` or `cov_of`;
    products in §5.1 multiply two `[0,1]` factors). `composite_plate`
    passes a bilinearly sampled alpha in `[0, 1]` (`toonfull.rs:344-351`).
    No caller passes `cov > 1` or `NaN`; behaviour for those is unspecified
    by the model (the `cov >= 1.0` branch would swallow `cov > 1`).

### 2.3 Accumulation is u8-quantised per operation — the central GPU constraint

**Statement.** The framebuffer is `u8` and *every* primitive ends in
`blend`/`blend_cov`, which rounds to `u8` before storing (§2.1). Therefore
the state a later primitive reads is the **rounded** result of all earlier
primitives, never an f32 accumulation. Two primitives over the same pixel
are *not* equivalent to one f32 source-over chain, and regrouping,
fusing-away, or reordering them changes results.

**What a GPU implementation must do to match:**

1. Per primitive, per pixel: **read** the current `u8` RGBA, run §2.1/§2.2
   exactly (including both fast paths and the alpha pre-quantisation),
   **write back `u8`**. I.e. a read-modify-write against a `u8` target
   (or an exact in-register emulation of one: load `u8`, compute, round,
   store `u8` before the next primitive touches that pixel).
2. Primitives must be **sequenced in call order** with a visibility barrier
   between them (§6). Within one primitive invocation, fusing is allowed
   only if the intermediate `u8` rounding is still performed between the
   fused steps.
3. A GPU design that accumulates in f16/f32 across primitives, uses
   hardware fixed-function blending (which has its own rounding /
   premultiplied conventions), or writes a float render target and
   quantises once at the end, **cannot** match this painter byte-for-byte.
   Hardware blending is additionally the wrong formula family here
   (straight alpha with division by `oa`).

**Where f32 non-associativity / libm differences can enter, per operation:**

| Operation | Where | Portability |
|---|---|---|
| `+ - * /`, comparisons, `min`/`max`/`clamp`, `floor`/`ceil` | everywhere | IEEE-754 single-precision semantics; bit-exact across conforming implementations **provided no FMA contraction and no reassociation**. Rust does not contract `a*b + c` into an FMA; WGSL compilers may. Expressions whose written form must be preserved include the blend numerator (`scene.rs:532`), `seg_dist_t`'s dot product (`scene.rs:844`), flattening (§1.7), and `ellipse_sd`. A GPU port must disable contraction for these or accept 1-ulp divergences that can flip sign tests (§4). |
| `sqrt` | `ellipse_sd` (`scene.rs:825`, `scene.rs:829`) | IEEE-754 `sqrt` is correctly rounded — bit-exact if the *input* is bit-exact. Safe in isolation; its inputs (division chains) are the risk. |
| `hypot` | `seg_dist_t` (`scene.rs:848`), `fan_sd` (`scene.rs:886`), `fill_rect` closure (`scene.rs:804`), closed-loop test (`scene.rs:641`), fan axis (`scene.rs:649`), head shade in `scene.rs:1135` | **Not** an IEEE primitive. Rust's `f32::hypot` is a libm routine with overflow-avoiding scaling; WGSL `length()` is specified as `sqrt(x*x + y*y)`. Results can differ by ~1 ulp and the naive form can overflow/underflow where libm `hypot` does not (not reachable at this painter's coordinate magnitudes, ≲ a few thousand px, but the ulp difference is). Near a `dist < 0` sign test or a `t == 0/1` gate, 1 ulp is decisive. This is the most pervasive per-pixel risk. |
| `sin` / `cos` / `sin_cos` | `ellipse_sd` per sample (`scene.rs:822`), `ellipse_mask` (`scene.rs:768`), `fill_ellipse`/`fill_ellipse_masked` bbox extents (`scene.rs:570-571`, `scene.rs:781-782`), `stroke_ellipse` point generation (`scene.rs:586-591`), `cap_fan` (`scene.rs:877`) | Transcendental; **not** correctly-rounded by specification on either side. Rust libm and GPU vendor implementations routinely differ by 1–few ulp. Where the value is per-primitive constant (`rot`, fan construction, ellipse point list), precompute on CPU and upload. Where it is per-sample (`ellipse_sd`'s `sin_cos(rot)`), the *value* is still per-primitive constant — hoist it (same reasoning as §1.5). |
| `acos` | `cap_fan` once per cap (`scene.rs:871`); `fan_sd` **per sample point** (`scene.rs:894`) | Transcendental, same warning as above, but `fan_sd`'s use cannot be hoisted — it depends on the sample direction. Highest-risk single function in the painter. |
| `powi(2)` / `powi(3)` | `ellipse_sd` (`scene.rs:825`, `scene.rs:829`), `ellipse_mask` (`scene.rs:773`), `flatten_cubic` (`scene.rs:971-972`) | Rust `powi` for small integer exponents lowers to repeated multiplication in practice; WGSL has no `powi` and a naive `pow(x, 2.0)` is a transcendental path with different rounding. A GPU port must spell these as explicit multiplications (`x*x`, `x*x*x`). |
| `.round()` | `blend` (`scene.rs:532`, `scene.rs:534`), `blend_cov` (`scene.rs:557`) | Semantics differ Rust vs WGSL (half-away vs half-even) — see §2.1. Must be implemented explicitly. |
| `as u8` / `as i32` / `as usize` casts | after every `.round()`; `cap_fan`'s `n` (`scene.rs:871`); `fan_sd`'s `k` after `floor` (`scene.rs:897`) | Truncation toward zero. All rounding-site values are non-negative, so this is `floor` there; keep the exact sequence (round, then truncate) rather than substituting a single conversion. |
| Integer ops, `u32` counting | supersample count (`scene.rs:922-930`), pixel indexing | Exact on any platform. The supersample count and `k/16` division (by a power of two) are exact. |
| `f32::MAX` sentinel mins | `stroke_polyline` pass 1 (`scene.rs:618`, `scene.rs:630-632`), `fill_polygon` `dmin` (`scene.rs:715-720`), bbox scans | `min` is exact and order-free for non-NaN values; ties return equal values. Safe to parallelise as a reduction. |

**Non-associativity summary.** (a) *Across* primitives: blend order and
per-step quantisation — §6. (b) *Within* a primitive: the only accumulated
f32 quantities are min-reductions (exact, order-free) and the integer
supersample count (exact). All other per-pixel expressions are fixed
written forms; preserving their evaluation order and non-contraction is
sufficient. (c) `point_in_polygon`'s toggle is parity (XOR) — order-free
(§1.6).

---

## 3. Primitives — exact per-pixel math

Summary of extents (all via `bbox`, §1.1, inclusive loops):

| Primitive | Bbox padding | Coverage source | Blend entry |
|---|---|---|---|
| `fill_ellipse` | extent ± 1 | `cov_area(ellipse_sd)` | `blend_cov` |
| `stroke_ellipse` | via `stroke_polyline` | via `stroke_polyline` | via `stroke_polyline` |
| `stroke_polyline` | points' AABB ± (`hw` + 1) | `cov_area0(fan-aware dist)` | `blend_cov`, once per pixel |
| `stroke_quad` | via `stroke_polyline` | via `stroke_polyline` | via `stroke_polyline` |
| `fill_polygon` | points' AABB ± 1 | `cov_area0(sd_at)` | `blend_cov` |
| `fill_rect` | rect ± 1 | `cov_area(box SDF)` | `blend_cov` |
| `fill_ellipse_masked` | extent ± 1 | `cov_area(ellipse_sd)`, gated by mask | `blend_cov` |
| `polygon_mask` | points' AABB, **no padding** | centre boolean | — (writes `Vec<bool>`) |
| `ellipse_mask` | extent, **no padding** | centre boolean | — (writes `Vec<bool>`) |
| `fill_polygon_masked` (toonfull) | points' AABB, **no padding** | two centre booleans | `blend` (full coverage) |

### 3.1 `fill_ellipse(cx, cy, rx, ry, rot, col)` (`scene.rs:569-582`)

- Rotated-ellipse AABB half-extents:
  `ex = |rx·cos rot| + |ry·sin rot|`,
  `ey = |rx·sin rot| + |ry·cos rot|` (`scene.rs:570-571`) — note these use
  `rot.cos()` / `rot.sin()` as *separate* calls (contrast `ellipse_sd`'s
  `sin_cos`); values feed only the bbox, so a 1-ulp difference can at most
  add/remove a zero-coverage border pixel *unless* it crosses an integer
  `floor`/`ceil` boundary in `bbox` — reproduce with CPU-precomputed
  extents to be safe.
- Bbox: `(cx - ex - 1, cy - ey - 1)` to `(cx + ex + 1, cy + ey + 1)`.
- Per pixel: `cov = cov_area(ellipse_sd_closure, centre)`; if `cov > 0`,
  `blend_cov(x, y, col, cov)` (`scene.rs:573-580`). No early skip: every
  bbox pixel evaluates `cov_area`, whose centre call decides the branch.

### 3.2 `stroke_ellipse(cx, cy, rx, ry, rot, width, col)` (`scene.rs:584-595`)

- Generates `145` points: `n = 144` fixed; for `i in 0..=144`,
  `a = (i / 144) · TAU`, `(lx, ly) = (rx·cos a, ry·sin a)`, point =
  `(cx + lx·c - ly·s, cy + lx·s + ly·c)` where `(s, c) = rot.sin_cos()`
  (`scene.rs:585-593`).
- First and last points coincide (up to f32 evaluation of `a = 0` vs
  `a = TAU` — in practice `cos(TAU)`/`sin(TAU)` in f32 need not return
  exactly `1`/`0`, so the endpoints can differ by ~1 ulp; either way the
  distance is `< 0.05`, so `stroke_polyline`'s closed test (§3.3) classifies
  it closed and no caps are built). Then delegates entirely to
  `stroke_polyline`. There is no ellipse-specific stroke distance: the
  stroke is the capsule union of the 144 flattened chords. (gaps.md item 1:
  fixed 144 vs Skia's adaptive flattening — a known, accepted delta from
  Skia; parity here means keeping 144.)

### 3.3 `stroke_polyline(pts, width, col)` (`scene.rs:601-696`)

The most intricate primitive. `hw = width / 2` (`scene.rs:605`).
Returns immediately if `pts.len() < 2` (`scene.rs:602-604`).

**Union bbox:** points' AABB expanded by `hw + 1` on all sides
(`scene.rs:606-613`).

**Pass 1 — plain distance field** (`scene.rs:618-635`): a buffer
`dmin[wpx * hpx]` initialised to `f32::MAX`, indexed
`((y - y0) * wpx + (x - x0))`. For each segment, over that segment's own
bbox padded by `hw + 2` and then clamped to the union bbox
(`scene.rs:622-625` — the comment notes the stamp padding is wider than the
buffer padding), store `min(dmin, seg_dist(centre, a, b))`. Pass 1 knows
nothing about CapFans: it stamps **exact-disc** distances.

**Closed test** (`scene.rs:640-641`):
`closed = pts.len() > 2 && hypot(pts[0] - pts[nseg]) < 0.05` (strict),
`nseg = pts.len() - 1`.

**Cap fans** (`scene.rs:642-657`): if not closed, a fan is built for each
open end: for the start, vertex `pts[0]` with neighbour `pts[1]`; for the
end, vertex `pts[nseg]` with neighbour `pts[nseg - 1]`. Axis =
`(vertex - neighbour) / hypot(vertex - neighbour)` (outward). If that
length is `< 1e-6`, the fan is `None` (degenerate segment) and the disc
term is used. If closed, both fans are `None`.

**Fan-aware distance closure** (`scene.rs:658-672`): for a query point,
`sd = min over segments i of s_i` where, with `(dd, t) = seg_dist_t(q, seg_i)`:

- `i == 0 && t == 0.0`: `s = fan_start.map_or(dd - hw, |f| fan_sd(f, q))`
- `i == nseg - 1 && t == 1.0`: `s = fan_end.map_or(dd - hw, |f| fan_sd(f, q))`
- otherwise: `s = dd - hw`

Note precisely: the fan replaces the terminal segment's term **only when
the clamped projection of the query point onto that segment is exactly the
terminal vertex**. It is not selected by proximity to the cap centre.
Interior vertices never get fans — interior round joins are the exact-disc
union that falls out of the min over capsule distances (gaps.md item 6).
A single-segment polyline is both first and last segment: the two branches
are mutually exclusive per evaluation (`t` cannot be both `0.0` and `1.0`
unless the segment is degenerate, in which case `seg_dist_t` forces
`t = 0.0` and the start branch wins).

**Pass 2 — blend once per pixel** (`scene.rs:673-695`):

```rust
d0 = dmin[idx] - hw
if d0 >= 1.0 { continue }                       // coverage provably 0
let d0 = if d0 > -1.5 { dist(centre) } else { d0 };   // fan-aware recompute
cov = cov_area0(&dist, centre, d0)
if cov > 0.0 { blend_cov(x, y, col, cov) }
```

- **The pass-1/pass-2 recompute margin is `-1.5`, and it is exact model,
  not a heuristic to retune.** Rationale in the comment
  (`scene.rs:679-684`): the fan shifts distance by at most the sagitta
  (0.25 px, §1.8) and only near terminal vertices, so for plain
  `d0 ≤ -1.5` the fan-aware distance is certainly `≤ -1.25`, i.e. coverage
  is 1 under either model and recompute is skipped. For `d0 > -1.5` the
  centre distance is recomputed with the fan-aware closure. In the
  supersample branch (`|d0| < 1`, which implies `d0 > -1.5`), all 16
  sub-samples also use the fan-aware closure — `cov_area0` receives `&dist`,
  not the pass-1 field. Pass 1 is thus purely a skip/branch oracle plus
  the `d0 ≥ 1` rejection; it never contributes a blended value directly.
- The `d0 ≥ 1.0` skip is safe against the fan because the fan only
  *increases* sd relative to the disc (§1.8), and it is safe against other
  segments because pass 1 already took the min over all of them.
- **Single-blend rule (load-bearing):** exactly one `blend_cov` per pixel
  per `stroke_polyline` call, from the union (min) distance — never one
  blend per segment. Joins and self-crossings within one call therefore do
  **not** double-blend. (Two *separate* `stroke_polyline` calls crossing
  each other do blend twice, in call order — that is the battery's
  "crossing strokes" class, README §"The battery".)
- Degenerate segments (`len2 == 0`) contribute `dist to the point - hw`
  via `seg_dist_t`'s `t = 0` rule; a closed ring whose last point
  duplicates its first (the idiom at e.g. `toonfull.rs:878`,
  `ring.push(bean[0])` pattern) has such a final segment, and the closed
  test suppresses its fans.

### 3.4 `stroke_quad(p0, p1, p2, width, col)` (`scene.rs:697-700`)

`stroke_polyline(flatten_quad(p0, p1, p2, 24), width, col)` — 25 points,
24 segments, fixed. No other math of its own.

### 3.5 `fill_polygon(pts, col)` (`scene.rs:704-740`)

- Bbox: points' AABB ± 1 (`scene.rs:712`).
- Signed-distance closure `sd_at` (`scene.rs:714-726`):
  `dmin = min over all n edges (i -> (i+1) % n) of seg_dist(q, edge)`;
  result `= -dmin` if `point_in_polygon(q, pts)` else `+dmin`. The
  magnitude is the unsigned distance to the *outline* (including the
  implicit closing edge); the sign comes **only** from the centre-style
  inside test evaluated at the same query point. (Doc comment
  `scene.rs:701-703` says "even-odd" — the even-odd rule lives entirely in
  `point_in_polygon`, §1.6; the distance magnitude has no winding
  component. Code and comment agree in effect.)
- Per pixel: `sd0 = sd_at(centre)`; **early skip iff `sd0 > 2.6`**
  (`scene.rs:731-733`); else `cov = cov_area0(&sd_at, centre, sd0)`, blend
  if `cov > 0`. ⚠️ The `2.6` is an optimisation bound, not a wider AA
  band (discrepancy flag D2, §7): for `1.0 ≤ sd0 ≤ 2.6` the call still
  happens and `cov_area0` returns `cov_of(sd0) = 0`. Sub-samples are at
  most ~0.53 px from the centre, so no sub-sample of a pixel with
  `sd0 > 1` could be inside for a *straight* edge; for the polygon's true
  geometry the skip changes nothing because the else-branch would return
  0 anyway. A GPU port may keep or drop the `2.6` test, but must not
  replace it with a different *coverage* rule.
- Cost/shape note for GPU design: in the band, `sd_at` is evaluated 17
  times per pixel (1 centre + 16 sub-samples), each evaluation looping
  over **all** edges for both `seg_dist` and `point_in_polygon`. There is
  no edge culling. Parity requires the same values, not the same cost —
  culling edges that provably cannot be the min / cannot toggle is safe
  only with exact arithmetic arguments.

### 3.6 `fill_rect(x, y, w, h, col)` (`scene.rs:798-814`)

- Bbox: `(x - 1, y - 1)` to `(x + w + 1, y + h + 1)` (`scene.rs:799`).
- Box SDF closure (`scene.rs:800-805`): centre `c = (x + w/2, y + h/2)`,
  half extents `(hw, hh) = (w/2, h/2)`;
  `d = (|q - c| - (hw, hh))` componentwise, then
  `sd = hypot(max(dx, 0), max(dy, 0)) + min(max(dx, dy), 0)` — the standard
  box SDF: Euclidean outside (corner regions use `hypot`), `0` on the
  boundary, negative of the smaller penetration inside.
- Per pixel: `cov_area(&dist, centre)`, blend if `> 0`. Straight edges get
  the exact supersample count; only corner pixels touch `hypot`.

### 3.7 Masks and masked fills — see §4 (dedicated section).

### 3.8 `blend` / `blend_cov` — see §2 (dedicated section).

---

## 4. Mask semantics (exact)

### 4.1 `polygon_mask(pts) -> Vec<bool>` (`scene.rs:742-760`)

- Allocates `vec![false; w * h]` — a full-canvas mask every call.
- Bbox: points' AABB with **no padding** (`scene.rs:751`:
  `bbox(mnx, mny, mxx, mxy)`).
- Per bbox pixel: `m[y * w + x] = point_in_polygon(x + 0.5, y + 0.5, pts)`
  (`scene.rs:753-757`). **Centre-sampled boolean**: one inside test at the
  pixel centre; no coverage, no supersampling, no distance. Pixels outside
  the bbox remain `false`.

### 4.2 `ellipse_mask(cx, cy, rx, ry, rot) -> Vec<bool>` (`scene.rs:763-779`)

- Full-canvas `false` vector; bbox = rotated AABB (§3.1 extents) with
  **no ±1 padding** (`scene.rs:765-767`).
- Per pixel (`scene.rs:768-776`): `(s, c) = rot.sin_cos()` (hoisted here,
  once per mask — contrast `ellipse_sd`); local coords
  `(lx, ly) = (dx*c + dy*s, -dx*s + dy*c)` from the centre offset; inside
  iff `(lx/rx).powi(2) + (ly/ry).powi(2) <= 1.0` — **inclusive** `<=`.
- ⚠️ This does **not** call `ellipse_sd`. Mathematically the region is the
  same (`g ≤ 1` ⇔ `ellipse_sd ≤ 0` for `g > 0`, §1.5), but the floating-
  point path is different (one `powi(2)` sum and a comparison against
  `1.0`, vs the gradient-normalised division chain). Near-boundary pixels
  can therefore classify differently between `ellipse_mask` and a
  hypothetical `ellipse_sd < 0` mask. Reproduce the quadratic form as
  written (discrepancy flag D3, §7).

### 4.3 `fill_ellipse_masked(cx, cy, rx, ry, rot, col, mask)` (`scene.rs:780-796`)

- Identical to `fill_ellipse` (same extents, same ±1 bbox padding,
  `scene.rs:781-784`) plus one gate (`scene.rs:787-789`): if
  `!mask[y * w + x]`, skip the pixel entirely.
- **Masked ellipse fills DO use coverage:** a mask-true pixel blends with
  `cov = cov_area(ellipse_sd_closure, centre)` via `blend_cov`
  (`scene.rs:790-793`) — full AA, including partial coverage at the
  *ellipse's* edge.
- The mask is consulted **once per destination pixel, at the pixel's own
  index** — it is *not* evaluated at the 16 sub-sample positions. So a
  pixel whose centre is mask-true receives the ellipse coverage computed
  from sub-samples that may geometrically lie outside the mask region, and
  a mask-false pixel receives nothing even if most of its footprint is
  inside both shapes. The visible result is an AA ellipse edge clipped by
  a hard (centre-sampled) mask boundary. Production uses: body shade /
  highlight clipped to the bean (`toonfull.rs:872-874`), tongue clipped to
  the mouth ellipse (`toonfull.rs:1180-1182`) and to the laugh-mouth
  polygon (`toonfull.rs:1202-1204`).
- `mask` is indexed by absolute canvas coordinates and must be a
  full-canvas slice (`mask[y * w + x]`); a shorter slice would panic on
  CPU. GPU ports must keep the same indexing domain.

### 4.4 `fill_polygon_masked` — **not a `Painter` method** (`toonfull.rs:229-247`)

⚠️ Naming flag (D6, §7): this is a free function in `toonfull.rs`, not part
of `impl Painter` in `scene.rs`. Its semantics differ sharply from
`fill_polygon`:

```rust
own = p.polygon_mask(pts)                       // toonfull.rs:230
bbox over pts' AABB, NO padding                 // toonfull.rs:231-238
for each bbox pixel:
    k = y * p.w + x
    if mask[k] && own[k] { p.blend(x, y, col) } // toonfull.rs:241-244
```

- **Full coverage, hard edges, no AA anywhere:** the fill's own shape is a
  centre-sampled boolean (`own`), the clip is a centre-sampled boolean
  (`mask`), and the write is a plain `blend` — i.e. at the colour's own
  alpha (verbatim copy if alpha 255, source-over at full alpha otherwise).
  No `blend_cov`, no distance, no supersample. A pixel is either fully
  painted or untouched.
- Production uses: body patterns clipped to the bean — stripes
  (`toonfull.rs:821`), apron (`toonfull.rs:840`), vest panels
  (`toonfull.rs:859`).
- A call-site variant of the same pattern exists for the laugh teeth
  (`toonfull.rs:1207-1214`): bbox taken from two transformed points (not
  the polygon AABB), gate = the mouth `polygon_mask` only, write =
  `p.blend(xx, yy, white)`. Same hard-edge semantics.

### 4.5 Mask summary

| Consumer | Clip test | Fill coverage | Edge character |
|---|---|---|---|
| `fill_ellipse_masked` | centre boolean, per pixel | `cov_area(ellipse_sd)` (AA) | AA ellipse edge, hard mask edge |
| `fill_polygon_masked` | centre boolean ∧ centre boolean | none (`blend` at full alpha) | hard on both boundaries |
| laugh teeth loop | centre boolean | none (`blend`) | hard |

This matches gaps.md item 2 ("Clip masks are binary … the native clip
masks are point-sampled") — with the sharpening that
`fill_polygon_masked` is binary in its *fill* as well, not only in its
clip.

---

## 5. Caller-composed coverage in `toonfull.rs` (outside `impl Painter`, in scope)

A GPU port of "the painter as used by `toonfull.rs`" must also reproduce
these direct uses; they bypass the primitives above.

### 5.1 Head shade — the only direct `cov_of` ramp use (`toonfull.rs:995-1018`)

Per pixel over `bbox(ch ± rp·RES·1.05)` (note: radius factor `1.05`, not
the §3.1 extent formula):

```rust
cov_head = Painter::cov_of(ellipse_sd(ch, rx_h, ry_h, head_rot, centre))   // :1006
if cov_head <= 0.0 { continue }
d = ellipse_sd(cc, rx_c, ry_c, head_rot, centre)                            // :1010
unshaded = (0.5 - d).clamp(0.0, 1.0)                                        // :1011
cov = cov_head * unshaded
if cov > 0.0 { blend_cov(x, y, col, cov) }                                  // :1012-1015
```

- Here the `cov_of` **ramp is used at centre samples only** — no
  supersampling, no `cov_area`. Fractional ramp values therefore *do*
  occur in production through this path (and its `paint_toon` twin in
  `scene.rs:1131-1138`, which differs in the second factor: there
  `unshaded = clamp(hypot(centre - cc) - rp·res + 0.5)` — a circle distance
  via `hypot`, `scene.rs:1135` — while `toonfull.rs` uses a second
  `ellipse_sd`. The two callers are *not* interchangeable; port each as
  written. Flag D7, §7.)
- `cov` is a product of two `[0, 1]` f32 factors, then quantised by
  `blend_cov` as usual (§2.2).

### 5.2 `composite_plate` bilinear blend (`toonfull.rs:290-354`)

- Per destination pixel, inverse-maps to plate space, bilinearly samples
  the plate's `u8` channels **in f32 on the 0–255 scale**
  (`toonfull.rs:335-343`), then calls
  `blend_cov(x, y, (out[0] as u8, out[1] as u8, out[2] as u8, 255), a)`
  with `a = out[3] / 255.0` (`toonfull.rs:344-351`). Note the double
  quantisation: the sampled RGB is truncated (`as u8`, *not* rounded)
  before blending, and the sampled alpha goes through `blend_cov`'s
  rounding with source alpha 255. The comment at `toonfull.rs:320-321`
  calls this "premultiplied blend via blend_cov" — the code is a straight
  `blend_cov` of a truncated colour at sampled-alpha coverage; code wins
  (flag D8, §7).

### 5.3 Order of drawing in `paint_toon_full`

`toonfull.rs` issues primitives in a fixed painter's-algorithm sequence
(legs → body bean fill → pattern → masked shade/highlight → bean outline
stroke → neckwear → arms/plates/hands → head → face → hats/neckwear
details; e.g. legs `toonfull.rs:755-785`, bean `toonfull.rs:800-878`, head
`toonfull.rs:987-1019`). That sequence is part of the image (§6): a GPU
port must execute the same calls in the same order per frame.

---

## 6. Sequential / order-dependent structure — implications for GPU dispatch

### 6.1 What is inherently sequential

1. **Primitive call order = blend order.** `px` is a single mutable
   buffer; every primitive reads the destination written by all previous
   primitives (§2.3). Source-over is neither commutative nor associative,
   and the per-step `u8` rounding destroys any regrouping equivalence.
   *Implication:* dispatches for successive primitives must be separated
   by a barrier that makes the previous primitive's writes visible; two
   primitives may never run concurrently over overlapping pixels.
   Primitives with provably disjoint padded bboxes could overlap, but
   proving disjointness must use the §3 bboxes, and the general production
   sequence (§5.3) is deliberately overlapping (fill → shade → outline on
   the same pixels).
2. **Within `stroke_polyline`: the two passes are sequentially dependent
   as written** (pass 2 reads `dmin` written by pass 1). *But* the
   dependence is per-pixel only: pass 2 uses `dmin` solely at the pixel's
   own index as a skip/branch oracle, and recomputes exact distances via
   the closure whenever they matter (§3.3). A GPU port may therefore fuse
   passes per pixel (compute the centre min-distance in-register, then
   branch identically) with identical results, or keep two dispatches
   with a barrier. What it may **not** do is dispatch per segment: the
   single-blend union rule (§3.3) forbids segment-wise blending.
3. **Mask production → masked consumption.** `polygon_mask` /
   `ellipse_mask` are pure functions of geometry and canvas size (they
   read only `self.w`/`self.h`, `scene.rs:742`, `scene.rs:763`) and never
   read `px`. They may be computed at any time before their consumer,
   in parallel with anything, and in parallel internally (each output
   element written once). The consumer (`fill_ellipse_masked`,
   `fill_polygon_masked`) must observe the completed mask — one barrier
   per mask. Note `fill_polygon_masked` computes its `own` mask inside
   the call (`toonfull.rs:230`), so that dependency is internal to the
   call.
4. **`blend`'s read-modify-write per pixel** is the atomic unit. Within
   one primitive, each destination pixel is written **exactly once**
   (every primitive's loops enumerate its bbox once, and the single-blend
   rule holds for strokes), and pixel computations are mutually
   independent (no primitive reads a neighbour's new value). *Implication:*
   full data-parallelism **across pixels within a primitive** is exact —
   no atomics, no ordering constraints — provided each pixel's RMW (§2.3)
   completes before the *next* primitive's dispatch reads it.

### 6.2 What is *not* sequential

- Min-reductions (pass-1 `dmin`, `fill_polygon` `dmin`, the stroke
  closure's `sd`): exact `min`, any order/tree shape (§2.3).
- Supersample counting: integer, exact, any order.
- `point_in_polygon`: parity, order-free (§1.6).
- Flattening and fan construction: pure per-primitive setup; may run on
  CPU exactly as today (recommended, §1.8/§2.3) and be uploaded as buffers.

### 6.3 Dispatch structure that follows

```
for each primitive call, in caller order:
    [CPU: flatten / build fans / build masks, or prior GPU dispatches + barrier]
    one compute dispatch over the primitive's padded bbox (§3)
        per pixel: coverage (§1.3 / §3) -> blend_cov/blend RMW on u8 target (§2)
    barrier
```

Fusing *consecutive* primitives into one dispatch is permissible **only**
if each pixel's invocation still executes them in order with the
intermediate `u8` store/load (round-trip through `u8` values in
registers is equivalent — the values, not the memory traffic, are the
model). Merging coverage computations across primitives (e.g. summing
coverages before one blend) is never equivalent.

---

## 7. Discrepancies and flags (code wins)

- **D1 — `cov_of` ramp reachability.** The `cov_of` doc comment
  (`scene.rs:562-564`) presents the ramp as the painter's coverage rule.
  Through `cov_area0` the ramp's fractional range is unreachable (§1.3
  item 4): coverage there is `k/16` in the band and saturated `0/1`
  outside it. The ramp's fractional values occur only via direct `cov_of`
  calls (§5.1). A spec reader who implements "ramp + supersample band"
  as alternatives over the same range will misbuild the band edge.
- **D2 — `fill_polygon`'s `2.6` skip** (`scene.rs:731`) looks like a wider
  AA band than `cov_area0`'s `1.0`; it is not (§3.5). No coverage is
  produced beyond `sd0 ≥ 1.0` either way.
- **D3 — `ellipse_mask` ≠ `ellipse_sd` test** (§4.2): same mathematical
  region, different floating-point path and an inclusive `<= 1.0`
  comparison. Do not "unify" them in a port.
- **D4 — dead guard in `blend`:** with `sa ∈ (0, 1)` (the only way to
  reach `scene.rs:525`), `oa = sa + da(1 - sa) ≥ sa > 0`, so the
  `oa <= 0.0` early return is unreachable. Harmless; keep or drop freely.
- **D5 — `blend_cov` comment simplification** (`scene.rs:544-548`):
  coverage does not multiply alpha continuously inside the blend; it is
  rounded to a `u8` alpha first (§2.2). The rounding is observable
  (vanishing small coverages, `16/255 ≠ 1/16`).
- **D6 — `fill_polygon_masked` is not a `Painter` method** and not a
  variant of `fill_polygon`: it lives in `toonfull.rs:229-247`, has no
  coverage at all, and uses `blend` (§4.4). Documents that list it among
  `scene.rs` primitives misplace it.
- **D7 — the two head-shade implementations differ:** `scene.rs:1131-1138`
  (`paint_toon`) uses `hypot` circle distance for the unshaded factor;
  `toonfull.rs:1010-1011` (`paint_toon_full`) uses a second `ellipse_sd`.
  Same visual intent, different per-pixel math. Port each as written.
- **D8 — `composite_plate` comment** (`toonfull.rs:320-321`) says
  "premultiplied blend via blend_cov"; the code performs a straight-alpha
  `blend_cov` of a *truncated* (`as u8`) bilinear colour at coverage
  `out_a / 255` with source alpha forced to 255 (§5.2). Code wins.
- **D9 — gaps.md ordering:** `docs/ab/stroke-battery/gaps.md` lists items
  in the order 1, 6, 7, 3, 4, 5. Editorial only; content was verified
  against the code for items 1 (fixed flattening counts, §3.2/§3.5), 2
  (binary masks, §4), 6 (exact-disc interior joins, §3.3), 7 (radial
  CapFan model, §1.8) — all four statements are accurate as written.
- **D10 — `stroke_ellipse` endpoint coincidence:** points are generated
  for `i in 0..=144` with `a = TAU` at the end (`scene.rs:587-591`); f32
  `cos(TAU)`/`sin(TAU)` are not guaranteed to reproduce the `a = 0` point
  bit-exactly. The model does not depend on it (closed test tolerance
  `0.05`, §3.3), but a port that *assumes* exact endpoint equality to
  skip the closed test would be adding an assumption the code does not
  make.
- **No discrepancy found** in: the supersample offsets/count, the
  `-1.5` recompute margin vs its comment, the CapFan sagitta comment
  (§1.8), the `fill_polygon` "even-odd" comment (§3.5), or the stroke
  battery README's description of the painter model (signed-distance
  coverage + 4×4 binary supersample + CapFan).

---

## 8. Per-primitive parity-risk ratings

Rating = risk that a *competent, straightforward* GPU (WGSL/wgpu)
implementation diverges from the CPU painter on at least some pixels,
given §2.3's portability table. "Mitigation" is the action that moves the
rating down; residual risk after mitigation is noted.

| Primitive / unit | Risk | Technical reason |
|---|---|---|
| `blend` | **MED** (HIGH if WGSL `round()` used naively) | Only `+ - * /` — IEEE-exact in principle — but three traps: WGSL `round` is half-even vs Rust half-away (§2.1); FMA contraction of `s*sa + d*da*(1-sa)`; and exact `.5` results are common with `u8/255` inputs. Mitigation: explicit half-away rounding + contraction disabled ⇒ residual LOW. |
| `blend_cov` | **MED** | Everything in `blend`, plus its own pre-quantisation rounding whose ties are systematic (`src_a · k/16` hits `.5` whenever `src_a·k ≡ 8 mod 16`, §2.2). Same mitigations ⇒ LOW. |
| `cov_of` | **LOW** | One subtraction + clamp; IEEE-exact. |
| `cov_area` / `cov_area0` (mechanism) | **HIGH** | Amplifies any distance error: 16 sign tests per pixel, each a potential flip worth `1/16` coverage (→ up to 16 alpha levels after `blend_cov` quantisation at `src_a = 255`). The mechanism itself is exact integer counting; the risk is entirely inherited from the `dist` closure fed into it. |
| `fill_ellipse` | **HIGH** | `ellipse_sd` per sub-sample: `sin_cos` (hoistable, §1.5), two `sqrt`s, a division chain through `grad` (§1.5). Boundary sign flips under 1-ulp input differences change `k/16`. Mitigation: CPU-precompute `(s, c)`; spell `powi(2)` as `x*x`; disable contraction ⇒ residual MED (the `grad` division chain remains). |
| `stroke_ellipse` | **HIGH** | Inherits `stroke_polyline` (HIGH) wholesale; its own contribution — 145 points from f32 `cos`/`sin` per point (`scene.rs:590`) — is MED alone and is eliminated by CPU-side point generation. |
| `stroke_polyline` | **HIGH** | `hypot` in every `seg_dist_t` (§2.3), `t == 0.0 / 1.0` exact-equality fan gates (§3.3), `fan_sd`'s per-sample `acos` (§1.8), min over up to hundreds of segments × 17 evaluations per band pixel. Several independent 1-ulp flip mechanisms stack. Mitigation: CPU fans; `hypot` replaced by a scaling-identical formulation is *not* bit-safe — expect residual MED-HIGH at cap tips specifically. |
| `stroke_quad` | **HIGH** (own step: LOW) | Flattening is polynomial mult/add (order-preserving, contraction is the only trap); the rating is inherited from `stroke_polyline`. |
| `fill_polygon` | **MED** | No transcendentals. Risks: `hypot` inside `seg_dist` (per edge × 17 evaluations), and `point_in_polygon`'s division producing `xint` within 1 ulp of the sample `x` on near-edge pixels. Sign and magnitude come from *different* computations, so a flip in either is visible. |
| `fill_rect` | **LOW** | Box SDF is `abs`/`max`/`min` arithmetic — exact — except corner pixels, where one `hypot` of the two positive excesses appears. Straight-edge pixels (the overwhelming majority) are as safe as `cov_of`. |
| `fill_ellipse_masked` | **HIGH** | Identical distance path to `fill_ellipse` (HIGH); the mask gate itself is an exact boolean lookup and adds no risk. |
| `polygon_mask` | **LOW** | One division per edge, strict comparisons, parity accumulation — no transcendentals, no `sqrt`. Residual: pixels whose centre lies within 1 ulp of an edge crossing (`x ≈ xint`). |
| `ellipse_mask` | **LOW** | Per pixel: two divisions, two squares (as `powi(2)` — spell as `x*x`), one sum, one inclusive comparison against `1.0`. `sin_cos` is per-mask, CPU-hoistable. Residual: exact-boundary pixels (`sum ≈ 1.0`). |
| `fill_polygon_masked` (toonfull) | **LOW** | Composition of `polygon_mask` (LOW) + `blend` (MED ⇒ dominates). No coverage path of its own. Rating **MED** if `blend`'s rounding traps are unmitigated, by inheritance; LOW with §2.1 mitigations. Listed as LOW-with-mitigation to stay consistent with the `blend` row's residual. |
| `cap_fan` (construction) | **LOW** if CPU-built; **HIGH** if GPU-built | `acos` + `ceil` step function: a 1-ulp `acos` difference can change `n`, changing the entire cap. CPU construction + buffer upload removes the risk class. |
| `fan_sd` (evaluation) | **HIGH** | Per-sample `acos`, `cos`, division by `cos(phi - phi_mid)`, plus `hypot`. The cap-end class is exactly where the battery found the model most sensitive (README, finding 1). |
| `ellipse_sd` (helper) | **HIGH** | See `fill_ellipse`. |
| `seg_dist` / `seg_dist_t` (helper) | **MED** | One division (`t`), one `hypot`; exact-equality outputs (`t = 0/1`) are consumed by fan gates, raising the stakes of a 1-ulp `t` difference near (but not at) the clamps — note `t` values strictly inside `(0, 1)` do not gate anything, and clamped values are exact regardless of the division's ulp, *unless* the true projection is within 1 ulp of an endpoint. |
| `point_in_polygon` (helper) | **LOW** | See `polygon_mask`. |
| `flatten_quad` / `flatten_cubic` (helpers) | **LOW** | Pure polynomial evaluation; preserve term order, spell `powi` as multiplications, disable contraction. CPU-side generation (current architecture) removes even this. |
| `bbox` | **LOW** | `floor`/`ceil`/clamp on f32 — exact at these magnitudes. Only feeds loop extents; a 1-ulp input difference matters only if an extent sits exactly on an integer, which the ±1 / `hw+1` paddings make result-neutral except for `polygon_mask` / `fill_polygon_masked` (no padding — but their tests are centre-based, so an extra border pixel evaluates `false` unless the geometry truly reaches it; reproduce CPU-side to be safe). |

---

## 9. What this spec does not cover

- Geometry *generation* above the painter (toon posing, FK, plate
  compositing transforms) except where it feeds coverage directly (§5).
- Other `scene.rs` painters that reuse `Painter` (textures, facades,
  plates in later sections of the file) — they inherit §1/§2/§6 exactly,
  but their own direct pixel loops were out of this seat's scope except
  `toonfull.rs`.
- Colour management, PNG export, and any downstream compositing: the
  painter's output is the `u8` straight-alpha buffer itself
  (`scene.rs:496`).
- Performance: nothing here endorses the CPU painter's cost structure
  (e.g. `fill_polygon`'s 17 × all-edges evaluations, §3.5) as a GPU
  requirement — only its *values* are normative.

*End of INV-A spec. Verified against HEAD `bc12a5c`; if `src/scene.rs` or
`src/toonfull.rs` moves, line citations and the D-flags must be re-audited
before this spec is relied on.*
