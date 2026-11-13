# DoF chart — remaining divergences, ranked

The transcription itself is not on this list: with byte-exact depth
storage it matches the reference to ≤0.05 px at every chart depth
(README.md). What remains, largest first:

## 1. HalfFloat packed-depth storage is driver-sensitive (worked around, deliberately)

BokehPass stores its packed depth in a **HalfFloat** render target.
The pack's top byte travels in the red channel as k/255 (≈0.996),
where one fp16 ulp is 4.9e-4 — and the unpack weights that channel
≈1.0, so a 1-ulp store error becomes ≈0.0005 of depth, which
`perspectiveDepthToViewZ` amplifies to **≈1 m of viewZ at the 10 m
focal plane** (sensitivity ≈2000 per unit depth there, ≈18000 at
30 m).

Measured on this VM (native depth readback, `TALERENDER_DOF_DUMP_DEPTH=1`):

| depth | viewZ recovered (HalfFloat, llvmpipe) | viewZ recovered (RGBA8) | nominal |
|---|---|---|---|
| 11 m | −9.964 | −10.9998 | −11 |
| 12 m | −10.778 | −12.0001 | −12 |
| 10 m | −9.136 | −10.0000 | −10 |
| 8 m | −7.439 | −8.0000 | −8 |
| 7 m | −6.567 | −7.0000 | −7 |
| 5 m | −4.782 | −5.0000 | −5 |

Every stored channel sat consistently ~1 fp16 ulp *below* the
round-to-nearest value (e.g. red: 0.99561 stored vs 0.99609 correct
for byte 254) — llvmpipe's fp32→fp16 render-target conversion
truncates where SwiftShader/ANGLE rounds to nearest. With HalfFloat
storage the native blur curve consequently breaks tolerance in the
transition band (step-width Δ up to **+0.90 px** at 10 m, −0.90 px at
11 m; clamps and focus-adjacent planes unaffected because the curve
saturates or the byte happens to be exact, e.g. byte 255 at 14/30 m).

**Shipped resolution:** the native depth target is `Rgba8Unorm` — the
pack is a byte packing (it predates half-float targets), so byte
storage is lossless and identical on every driver; recovered viewZ is
within 0.001 m of nominal at all ten depths.
`TALERENDER_DOF_DEPTH_FORMAT=halffloat` restores BokehPass's exact
target configuration for comparison. The packing/unpacking arithmetic
is untouched either way.

**Open follow-up:** fp16 store rounding on RADV (the user's PC) is
unverified from this VM. If it rounds to nearest like SwiftShader,
the reference's own HalfFloat noise floor is ≤½ ulp (≈±0.5 m viewZ
worst case at 10 m, ≈±0.25 px blur) — the byte target stays the more
deterministic choice regardless.

## 2. The reference master is a JPEG

`original.png` is converted from `OP.frame`'s JPEG (q ≈ 0.92 — the
§28 finding; contract §5's open master-format question). Its noise
floor bounds every number in the package: the per-panel step-zone
PSNRs (50.8–57.7 dB) and the bleed-strip RMSEs (~0.7–1.1) are at that
floor, and 14.8% of pixels differ at all (0.25% by >10). A lossless
reference capture path would tighten future gates more than any
shader work.

## 3. Shared pre-bokeh baseline width (cancels, but bounds the kernel check)

Both renders carry 0.8–1.6 px of edge width *before* any bokeh
(~1.7× texture minification, single-level linear sampling on both
sides by construction, MSAA-4× resolve). Measured with bokeh disabled
(reference: engine aperture 0 disables the pass; native:
`TALERENDER_DOF_APERTURE=0` makes the gather an exact identity) the
baselines agree to ≤0.04 px at every depth — so the comparison is
clean — but the isolated-kernel prediction column only reconciles
after accounting for it, and the residual checker-median deltas
(≤0.12 px, worst at 8 m) are consistent with SwiftShader-vs-llvmpipe
filtering LSB differences under that minification.

## Not gaps (checked, by design)

- **Bloom / GRADE** are neutralised through the look on both sides;
  the full-strength chain on real scenes remains §28's gap class,
  not this gate's.
- **MSAA**: 4× on both scene targets, matching the composer's
  `samples: 4`.
- **Background**: BokehPass's white depth clear makes open background
  read as the far plane (dofblur −maxblur) in both renders — visible
  as the same soft panel silhouettes against black.
- **The `af` pack channel** is identically zero across the chart band
  in both implementations (fp32 `v·2²⁴` is integral for v ∈ [0.5, 1)).
