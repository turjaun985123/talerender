# The q0.93 JPEG generation — decision brief (contract §5)

**Status: OPEN — the decision is the user's. This brief takes
none.** It assembles the measurements, lays out the two options
with their consequences, and ends with a recommendation and its
reasoning, as the M6 work order requires.

## The question

Every frame the Chrome pipeline masters has passed through one
JPEG generation: capture.ts pulls frames with
`canvas.toDataURL('image/jpeg', 0.93)`, so the golden sheet,
every `docs/ab/` reference produced from page frames, and the
shipped projects' review frames all carry that generation's
artifacts as part of their pixel values. The native renderer
emits clean PNG. When native masters replace Chrome masters,
one of two things must be chosen deliberately:

- **(A)** native frames are passed through the same q0.93 JPEG
  generation before they become masters/references, or
- **(B)** native frames stay clean, and the JPEG generation
  remains a property of the legacy corpus only.

## Evidence

### 1. The floor on smooth content (M3 dof-chart, reused)

The dof-chart reference is a JPEG-carrying Chrome frame; the
native chart render is clean. In the chart's flat step zones —
where the two renders are otherwise in closest agreement — the
per-panel PSNRs land at **50.78–57.71 dB** (ten panels, focus
10 m; docs/ab/dof-chart/metrics.json). That band is what one
JPEG generation costs on smooth content: two otherwise-identical
images, differing only by the reference's generation, agree no
better than ~51–58 dB there. The chart's full-frame verdict
(47.38 dB) was reached with that handicap already present on
one side.

### 2. Direct measurement on a detailed frame (this seat)

The clean M5 native frame (`docs/ab/composer/native.png`,
1080×1920, foliage-heavy) encoded at q0.93 through the
pipeline's own encoder — Chromium `canvas.toDataURL('image/jpeg',
0.93)`, headless Chrome 157, the same call capture.ts makes
(artifact: `jpeg-gen1.jpg`; numbers: `jpeg-metrics.json`):

| comparison | PSNR | mean \|Δ\| | px > 10 | px changed | max Δ |
|---|---|---|---|---|---|
| clean → generation 1 | **42.31 dB** | 1.225 | 0.786 % | 94.47 % | 48 |
| clean → generation 2 | 42.04 dB | 1.247 | 0.933 % | 94.58 % | 48 |
| generation 1 → generation 2 | 56.10 dB | 0.082 | 0.002 % | 8.80 % | 14 |

Three facts matter:

- **The floor is content-dependent**: ~51–58 dB on flat/step
  content, ~42 dB on a detailed frame. There is no single
  "JPEG noise number"; there is a band, and detailed frames sit
  at its expensive end.
- **The first generation does nearly all the damage.**
  Generation 2 costs almost nothing further (gen1→gen2 at
  56.10 dB): JPEG artifacts are largely a fixed point of the
  encoder. The legacy masters are exactly one generation from
  their canvas; matching their *state* means matching that one
  generation, no more.
- **The cost is not subtle at edges**: max channel delta 48,
  and 94.5 % of all pixels change value. It is small compared
  to today's real renderer differences (M5's 23.85 dB A/B), but
  it is first-order at the accuracy the gates are now reaching —
  M4's character-alpha verdict was 47.88 dB and the dof chart's
  47.38 dB, both inside the band where a one-sided JPEG
  generation is a material term of the measurement.

## The options

### (A) Native masters deliberately pass through q0.93 JPEG

- Maximum comparability with everything that already exists:
  the golden sheet, all legacy references, shipped-project
  review frames. A/B numbers before and after adoption measure
  the same quantity.
- The cost is permanent and universal: every native master
  carries the generation-1 loss (≈42 dB on detailed content,
  edge ringing to Δ48) **forever**, including frames nobody
  will ever A/B. The loss cannot be undone later; if a future
  milestone wants clean masters, they must be re-rendered.
- It also obliges the native pipeline to keep a Chromium-grade
  JPEG encode step (or a bit-faithful equivalent) in its own
  chain indefinitely — the encoder becomes part of the format.

### (B) Native frames stay clean

- Masters are the best the renderer produces; no quality is
  spent on comparability. This matches the standing user law
  for this project family: no quality loss, ever, as the price
  of convenience.
- Every A/B against a legacy reference carries the JPEG floor
  as a **known, measured, one-directional bias**: it can only
  make the native side look worse than it is (the reference's
  artifacts count as native error), never better. Each future
  A/B package must state the floor for its content type, as
  this package states both ends of the band.
- The golden sheet must be re-baselined natively when the
  skill adopts the native renderer (a re-baseline is not
  unprecedented: the v6 work copy already carries a deliberate
  golden move for the sit-solver fix). From adoption onward,
  references are clean and mutually exact, and the bias
  disappears from new comparisons.
- Comparability with the legacy corpus remains recoverable on
  demand: passing a *copy* of a clean native frame through one
  q0.93 generation reproduces the legacy state to the measured
  band (and generation convergence means the comparison is
  stable), without touching the master.

## Recommendation

**(B) — keep native frames clean.** The asymmetry decides it:
option A's cost is irreversible quality loss applied to every
master, while option B's cost is a bounded, measured bias in
comparisons against a fixed legacy corpus — a bias that points
in the conservative direction, shrinks to zero for all
post-adoption references, and can be neutralised per-comparison
by JPEG-ing a copy when a legacy reference must be matched
exactly. Paying permanent quality for temporary comparability
trades the wrong way, and it trades against the user's explicit
standing instruction for this pipeline.

**The decision itself is the user's** (contract §5). Until it
is taken, the native renderer continues to emit clean PNG and
every A/B package states the JPEG floor it measured against.
