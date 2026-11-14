# Caption decision pack — the 7 over-floor chunks (W92)

**Analysis only. Nothing in this pack has been applied** — no caption data,
strips, scene files, or film data were modified. It exists so the lead/user
can decide the caption question with the numbers in front of them.

**The question.** The native film-telemetry gate's verdict is

```
FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)
```

Overflow is **none**, continuity is **clean**; the failure is caption widths
alone (run record: [TELEMETRY-W85.md](TELEMETRY-W85.md), independent cache-24
run, agreeing verbatim with the lead's own telemetry4/5 logs). Seven of the
66 chunks render wider than the audit's **850 px floor**. This pack identifies
the seven, explains the rule that produced them, and prices three options:
**A** re-chunk, **B** waive, **C** retarget the fit cap.

**Three widths, three different numbers — read this first.** The same chunk
has up to three widths in the record, and the options turn on which is meant:

| Width | Definition | Who measures it |
|---|---|---|
| `rawW` | Text width at 88 px Inter-Black, unscaled (PIL `getlength`; Skia agrees to ≤0.15 px) | generator manifest, `film/onoda.json` |
| `renderedW` (Chrome) | `rawW × fit`, `fit = min(1, 960/rawW)` — capped at **960** | Chrome audit (`printCaptionAudit`), scenes-side manifest |
| strip ink | Ink bbox (alpha > 32) of the driver-side strip PNG `film/assets/cap_*.png` | **native telemetry audit — the verdict's measure** |

The driver-side strips are rendered **unscaled** (ink ≈ `rawW` + 4–15 px of
stroke across the seven; the fit is applied later, at composite time —
`caption_scale = sc × fit`, film.rs:2014, overlay at film.rs:3467). So the
native audit's width is the strictest of the three: it audits the strip at
scale 1, not the size the caption actually reaches on screen (strip ink ×
fit, §1). All three measures flag **the same 7 chunks** — the count has never
been in dispute, only the widths attached to it.

---

## 1. The seven chunks

Frame width **1080 px**. Captions are centre-anchored (`textAlign = 'center'`,
captions.ts:53; strip text centred in a 1080-wide strip composited at
y = 1340), so side margins are symmetric up to glyph shape — the margins
below are **measured** on the driver strips (ink bbox position), not assumed.

| Chunk | Line | Text | t (s) | rawW | renderedW (Chrome) | **strip ink (audit)** | fit | Ink margins L/R (strip) | On-screen settled width (ink × fit) | On-screen margin/side |
|---|---|---|---|---|---|---|---|---|---|---|
| cap-014 / cap_014.png | 2 | "was a Japanese soldier" | 6.69 | 1024.05 | 960.0 | **1039** | 0.9375 | 20 / 21 | ≈974 | ≈53 |
| cap-026 / cap_026.png | 5 | "Planes dropped leaflets" | 21.61 | 1040.44 | 960.0 | **1051** | 0.9227 | 16 / 13 | ≈970 | ≈55 |
| cap-040 / cap_040.png | 7 | "surviving on coconuts" | 31.55 | 989.95 | 960.0 | **1002** | 0.9697 | 39 / 39 | ≈972 | ≈54 |
| cap-044 / cap_044.png | 8 | "named Norio Suzuki" | 37.13 | 884.11 | 884.11 | **892** | 1.0 | 94 / 94 | 892 | 94 |
| cap-051 / cap_051.png | 9 | "commander ordered it." | 44.01 | 1007.39 | 960.0 | **1015** | 0.9530 | 30 / 35 | ≈967 | ≈56 |
| cap-053 / cap_053.png | 10 | "his old commander," | 46.39 | 867.30 | 867.30 | **871** | 1.0 | 102 / 107 | 871 | ≈104 |
| cap-059 / cap_059.png | 11 | "and Onoda surrendered" | 52.07 | 1040.03 | 960.0 | **1050** | 0.9230 | 14 / 16 | ≈969 | ≈55 |

Full narration lines, for context: line 2 "Hiroo Onoda was a Japanese
soldier sent to a Philippine island in 1944." · line 5 "Planes dropped
leaflets saying the war was over. …" · line 7 "He lived in the jungle for
decades, surviving on coconuts and bananas." · line 8 "In 1974, a young
traveler named Norio Suzuki found him." · line 9 "He said he would only stop
if his own commander ordered it." · line 10 "So Japan found his old
commander, now a bookseller, and flew him to the island." · line 11 "He
finally gave the order, and Onoda surrendered after 29 years."

Facts the table establishes:

- **Nothing touches the frame edge.** Worst strip margin is 13 px
  (cap_026, right); worst on-screen margin ≈ 53 px. The audit's own overflow
  count is 0 and its composition section counted **0 caption-zone findings**
  (W85 run record).
- Four chunks sit exactly on the Chrome-side 960 cap (renderedW = 960.0);
  their strip ink exceeds even the raw text width because the strips are
  unscaled.
- The 850 floor corresponds to a design margin of (1080 − 850)/2 = **115 px
  per side**. The widest on-screen caption keeps ≈ 53 px — inside the frame,
  outside the design margin.

## 2. Why they are wide — the rule that produced them

Three facts, each verified in source:

1. **Chunking never consults width.** The chunk boundaries are the project's
   authored `caps` fields in `script.json`, reproduced exactly by the ported
   `buildCaptions()` (`tools/captions/gen_caption_strips.py`): chunk *j*
   starts at a word boundary chosen by the author; start = word start −
   0.03 s. Neither the authored path nor the `autoChunks()` fallback measures
   text width at any point. (Provenance nuance: `film/onoda.json` carries no
   `caps` record — grep count 0 — so the native audit classifies all 66
   chunks as auto/estimate-grade, "0/12 lines authored", even though their
   text and timings are identical to the authored-caps chunking, cross-checked
   against the M6 first-party list and the scenes manifest. The classification
   differs; the boundaries, and the absence of a width rule behind them, do
   not.)
2. **The only width mechanism is `fit`, and its cap is above the floor.**
   `CAP = { y: 1340, size: 88, maxW: 960 }` (captions.ts:7);
   `fit = min(1, CAP.maxW / rawW)`. The layout system therefore *by design*
   admits rendered widths in (850, 960] — a chunk can be fully "fitted" by
   the system's own rule and still violate the audit floor. The parameter
   that admits the seven is **CAP.maxW = 960 vs the audit's WIDTH_FLOOR =
   850**; they are different constants in different files (layout vs audit),
   and no code reconciles them.
3. **The floor is an audit rule, not a layout rule.** In the Chrome template
   the same audit is **WARN by default**; the violations become FAIL only
   under the strict form (telemetry.rs:1033–1036, transcribing
   `printCaptionAudit`: "WARN by default; under CAP_CHECK=strict the
   violations print as FAIL"). The ≥ 0.33 s/word time floor exists in the
   same audit model and is **not** enforced (ruling R5 — "the width floor
   wins"). The Batch-1 census (WAVE-RECORD.md:123) records that Chrome's own
   audit headline for this film is identical — same 66 chunks, same 7 over
   the floor, min fit 0.9227 — i.e. **the original Chrome pipeline produced
   this exact caption set and reported these exact seven as warnings**.

So the seven are not a native-port defect and not a data corruption: they are
authored chunks that the layout cap (960) accepts and the audit floor (850)
rejects, in both pipelines.

## 3. Option A — re-chunk (split the seven at word boundaries)

Each chunk splits into exactly two word-anchored pieces; timings follow the
generator's own rule (piece start = its first word's start − 0.03 s; piece 1
ends where piece 2 starts). Splits below are the word boundary minimising
the wider piece, computed by measuring **every** candidate split with PIL
against the project's own Inter-Black.otf at 88 px (this seat, first-hand).
Compliance target: every piece `rawW` ≤ 850 makes `renderedW` ≤ 850 under
the Chrome arithmetic; projected strip ink uses the observed ink − rawW
delta over the current seven (+3.7 … +15.0 px, mean +9.9) — worst-case
projection shown.

| Chunk | Split after | Piece 1 (rawW) | Piece 2 (rawW) | New timing | Worst projected ink |
|---|---|---|---|---|---|
| cap-014 | "…Japanese" | "was a Japanese" (699.34), 6.69–7.87 s | "soldier" (307.17), 7.87–8.53 s | piece 2 starts at "soldier" (7.90 − 0.03) | ≈714 |
| cap-026 | "…dropped" | "Planes dropped" (687.14), 21.61–22.09 s | "leaflets" (335.77), 22.09–22.61 s | piece 2 at "leaflets" (22.12 − 0.03) | ≈702 |
| cap-040 | "surviving" | "surviving" (425.88), 31.55–32.01 s | "on coconuts" (546.55), 32.01–32.99 s | piece 2 at "on" (32.04 − 0.03) | ≈562 |
| cap-044 | "…Norio" | "named Norio" (562.33), 37.13–38.07 s | "Suzuki" (304.25), 38.07–38.45 s | piece 2 at "Suzuki" (38.10 − 0.03) | ≈578 |
| cap-051 | "commander" | "commander" (532.11), 44.01–44.43 s | "ordered it." (457.75), 44.43–45.40 s | piece 2 at "ordered" (44.46 − 0.03) | ≈547 |
| cap-053 | "…old" | "his old" (290.33), 46.39–46.93 s | "commander," (559.44), 46.93–47.77 s | piece 2 at "commander," (46.96 − 0.03) | ≈574 |
| cap-059 | "…Onoda" | "and Onoda" (473.73), 52.07–52.99 s | "surrendered" (548.77), 52.99–53.57 s | piece 2 at "surrendered" (53.02 − 0.03) | ≈564 |

- **New total: 66 → 73 chunks.** Every piece passes both audits with ≥ 80 px
  of headroom on the worst piece under the worst-case ink projection. All
  pieces also satisfy the (advisory, unenforced) 0.33 s/word time floor —
  shortest piece is 0.42 s for one word (cap-051 piece 1).
- Two chunks have exactly **one** compliant split point: cap-051 (the other
  boundary leaves a 895.83 px piece) and cap-059 (the other leaves 856.02 px
  before stroke). The table's choices are forced there, not stylistic.
- **Authoring cost:** the splits are changes to the project's `script.json`
  `caps` fields — an authoring edit in the project (lead/user domain), then
  regeneration. Seven lines get one extra caption cut each; caption display
  rhythm changes inside those seven windows (each piece shows for ~0.4–1.2 s).

**Downstream impact (verified, not assumed):**

- *Scenes side:* `tools/captions/gen_caption_strips.py` regenerates the
  manifest + strips from the edited `caps`. Because splits insert chunks,
  **every chunk after each split point renumbers** (+1 after cap-014, +2
  after cap-026, … +7 after cap-059): unchanged chunks keep their strip
  bytes but change filenames from the first insertion onward. The film
  manifest's per-segment caption ranges (which today tile chunks 0–65)
  re-tile to 0–72, and the validator's caption checks must re-pass.
- *Driver side:* `film/onoda.json`'s captions array (text/s/e/width/fit/
  strip per chunk) and the `film/assets/cap_*.png` strips are the lead's
  build — they must be regenerated to the same 73-chunk layout or the two
  sides diverge.
- *Bridge package:* the adapter's bridge set carries the 66 strips; it
  becomes a 73-strip set (mechanical, via the bridge packager).
- *A/B proof — the brief's hopeful claim does NOT hold as stated:*
  captions **are** part of the measured frames on both sides (native:
  `overlay_caption` composites the strip into every film frame,
  film.rs:3467; Chrome: draw order shot → fade → caption). After a re-chunk
  **and** a re-render, pairs sampled inside the seven windows would show
  different caption text on the native side. At the proof's 1 s sampling
  (stills at t = x.50), ≈ **10 of the 60 pairs** fall inside the affected
  windows (t = 7.50, 8.50, 22.50, 32.50, 37.50, 44.50, 46.50, 47.50, 52.50,
  53.50); the other ≈ 50 pairs are caption-identical. The A/B method and the
  remaining pairs are unaffected, and per decision (B) the numbers are
  reported, never gated — but the film A/B table would need a footnote or a
  re-measure of those pairs against a re-chunked Chrome reference that does
  not exist. This is a real cost of A, priced in.

## 4. Option B — waive (accept the seven, document the waiver)

The case, from the record:

- **No caption clips or overflows.** Worst on-screen margin ≈ 53 px; audit
  overflow groups = 0; composition caption-zone findings = 0. The floor is
  a safe-area/readability rule (115 px design margins), not a frame-fit rule.
- **This is the original pipeline's shipped state.** Chrome's audit reports
  the identical seven as **warnings** (its default severity), and the
  delivered Chrome film carries them. The native gate's FAILED comes from
  running the same audit in its strict form — a severity choice, not a new
  defect found by the port.
- **The native measure is conservative.** The verdict's width is the
  unscaled strip ink; the caption as composited on screen is 871–974 px
  (ink × fit). Even on-screen all seven exceed 850 — the waiver should not
  pretend otherwise — but the number the gate fails on (1051) is not a size
  any viewer ever sees.
- **Cost: zero production work.** The waiver is a recorded decision (in the
  integration record / proof package) that the gate's caption section is
  accepted at 7 known over-floor chunks for this film. The verdict line
  stays FAILED unless the lead also adjusts the gate's severity — which is
  itself a decision with precedent cost (the gate would stop failing future
  films' caption widths at WARN→FAIL parity with Chrome… or would diverge
  from it; either way it is the lead's call, not this pack's).

## 5. Option C — retarget the fit cap (CAP.maxW 960 → 850)

**Does a per-chunk scale mechanism exist?** Yes and no. `fit` *is* a
per-chunk scale, and it is already applied at composite time
(`caption_scale = sc × fit`). But `fit` is a **pure function of the chunk
text** (captions.ts:9 — "fit is a pure function of text under FONT"): there
is no per-chunk override field in `script.json`, the scenes manifest, or
`film/onoda.json`. Hand-tuning individual chunks is not a data-only option
today; it would need a new field plus driver support (lead's code). The
available knob is the global one: **CAP.maxW**.

Setting CAP.maxW = 850 (one constant, captions.ts:7, mirrored in
`gen_caption_strips.py`'s `CAP_MAXW`):

| Chunk | New fit (850/rawW) | Chrome renderedW | On-screen ink × new fit |
|---|---|---|---|
| cap-014 | 0.830 | 850.0 | ≈862 |
| cap-026 | 0.817 | 850.0 | ≈859 |
| cap-040 | 0.859 | 850.0 | ≈861 |
| cap-044 | 0.961 | 850.0 | ≈857 |
| cap-051 | 0.844 | 850.0 | ≈857 |
| cap-053 | 0.980 | 850.0 | ≈854 |
| cap-059 | 0.817 | 850.0 | ≈859 |

- All new fits stay well above the 0.68 fit floor (min 0.817) — no new
  violation class appears. Only the seven chunks change (every other chunk
  has rawW ≤ 850 and keeps fit = 1): their text renders at 850/rawW of its
  current on-screen size — 2–18 % smaller (effective font 72–86 px instead
  of 88) for those seven caption windows. No timing, chunking, or numbering
  changes anywhere; strips and `onoda.json` fit/width fields regenerate.
- **The decisive limitation:** the native audit measures **strip ink**, and
  the strips are rendered unscaled — retargeting fit does not change any
  strip by a single pixel. Under the audit as built, **Option C leaves the
  verdict exactly where it is** (`max rendered width 1051 px`, 7 failures),
  while fixing the Chrome-side arithmetic (renderedW = 850.0) and bringing
  on-screen widths to ≈ 854–862 (still marginally over 850 on screen,
  because fit scales the text width, and the ink carries ~10 px of stroke
  on top). C therefore only "works" paired with a lead change to the strip
  build (render strips at fit) or to the audit's measure (audit ink × fit,
  i.e. the on-screen width) — at which point it is no longer a one-constant
  option.

## 6. Side-by-side

| | A — re-chunk | B — waive | C — maxW 960→850 |
|---|---|---|---|
| Native telemetry verdict | **CHECK OK** (both audits pass, ≥80 px headroom) | stays FAILED (or severity change — separate decision) | stays FAILED (audit measures unscaled strip ink) |
| Chrome-side audit | clean (no warnings) | same 7 WARN as shipped | clean (renderedW = 850.0) |
| On-screen result | same text, narrower pieces, +7 caption cuts | unchanged (margins ≈53 px worst) | same chunks, 2–18 % smaller text in 7 windows |
| Work | project `caps` edit + regenerate both strip sets + manifest re-tile + bridge repackage | a recorded decision | one constant + regenerate fits/strips both sides |
| Collateral | ≈10 of 60 A/B pairs change caption content on re-render; chunk renumbering | gate stays red for this film; precedent question | none to numbering/timing; does not achieve the gate |

**Labelled recommendation (the factual part is one-sided):** if the goal is
a green native gate, **A is the only option that achieves it** — it is the
only one that reduces `rawW` itself, so it passes the Chrome arithmetic and
the native strip-ink audit simultaneously, with headroom. C cannot flip the
native verdict on its own, and B explicitly accepts it red. Whether a green
gate is worth A's re-chunk cost (authoring change + renumbering + the A/B
footnote) versus B's zero-cost acceptance of a state the Chrome pipeline
already shipped as warnings — that is the production judgement this pack
exists to inform, and it belongs to the lead/user.

---

*Evidence base (all first-hand for this seat unless noted): the seven FAIL
lines and audit semantics — TELEMETRY-W85.md + src/telemetry.rs:1033–1100;
strip ink bboxes and margins — PIL (alpha > 32) over `film/assets/cap_*.png`,
this seat, matching W85's widths digit-for-digit; rawW/renderedW/fit —
`assets/captions/manifest.json`; chunk timings and split candidates — PIL
`getlength` at 88 px against the project's `fonts/Inter-Black.otf`, every
word-boundary split of all seven chunks measured; word timings —
project `data/words.json`; placement and fit rules — project
`src/core/captions.ts`:7,51–53 and src/film.rs:2014,3401–3467; Chrome census
parity — docs/integration/WAVE-RECORD.md:123. Nothing was modified.*
