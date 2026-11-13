# M6 telemetry — gaps and scope notes

Everything in the agreement table matches exactly. What follows
is what the proof does **not** cover, or covers by a declared
mapping — recorded so nobody has to re-derive it.

## 1. Caption metrics enter as data (the one real gap)

The v5 caption audit consumes per-chunk metrics (rawW, fit,
renderedW) that the template's `OP.captions()` measures inside
the page. This film's project is v4-era: its `OP.captions()`
returns only `{s, e, text}` and its caption engine exposes no
metrics. The metrics in `ab1-captions.json` were therefore
measured in the harness page with the **project's own caption
engine definitions** — Canvas2D `measureText` under captions.ts's
FONT (`900 88px Inter`, letterSpacing `0px`), `fit = min(1,
960/width)`, the v5 `captionMetrics()` rounding — and `auto`
decided exactly as `buildCaptions` decides it (a chunk is auto
iff its script line authors no `caps`; all 12 lines here author
theirs, so 0 of 66 chunks are auto). The numbers are first-party;
only the *measurement step* ran in Chrome. A native Inter-900
text measurer is contract §2.4 future work; until it exists, the
native audit takes metrics as input data — the same posture
capture.ts itself has toward the page.

## 2. The verdict's `bad` and `logs` terms are 0 by construction

The v5 expression is `failed = bad>0 || logs.length>0 ||
overflow>0 || continuity violations>0 || strict-caption
fails>0`. `bad` (frames the page failed to render) and `logs`
(page console errors) are browser-runtime terms. The native
telemetry has no page: a sample either computes — in which case
both terms are genuinely zero — or the run fails before the
audit with a non-verdict error. The mapping is declared here
rather than silently assumed; the remaining three terms are
computed exactly as transcribed.

## 3. Continuity scope: one scene vs the whole film

The native modes audit a single staged sample, so CONTINUITY
tracks 1 named character; the real page's cast spans 5 (its
scenes register every character's look). This is a sampling-
scope difference, not a field difference: the signature value
for the character both sides stage is byte-identical, and the
violation rule (first-seen signature per id; a later different
signature is a violation) is transcribed with its dedupe keys.
Film-wide native continuity arrives with multi-scene sampling
in the integration phase, not by inflating this proof.

## 4. Overflow counts are rasteriser-dependent in general

Chrome counts Skia-antialiased pixels (alpha > 32) in the border
strips; the native side counts the M4 painter's signed-distance
supersampled canvas. On both stagings proven here the counts
agreed exactly (0 on the base; right = 1028 on the variant) —
reported as measured. A staging whose paint lands within a
pixel or two of the strip boundary could legitimately differ by
small counts between rasterisers; the gate semantics (any
non-zero side fails, dedupe by id+pose+sides, first-seen counts)
are identical regardless.

## 5. The other ten cutout items

The page's payload carries 11 items: the staged onoda, its
invisible JG companions, and the other sets' toons (box null —
not in the rendered scene — and overflow zero from their
last-painted or blank canvases). The native ab1 scene contains
one toon; reproducing the film's full registry is project
sampling, not telemetry semantics, and is out of this proof's
scope. The one field pattern worth naming: in the page payload
the *staged* onoda is identifiable as the only entry carrying a
box, and `compare.py` selects it on exactly that rule.

## 6. The variant is a synthetic staging

Lean 42° at group (1.9, −0.55, 0.6) is a construction chosen to
breach the canvas strip, the rail and the frame edge at once; it
is not a film frame. Its evidentiary value is that both gates,
given identical staging constants through their own real code
paths, produce identical telemetry and the same failing
verdict — which is what the contract's §3 asks the variant to
show.
