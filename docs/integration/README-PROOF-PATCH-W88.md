# README-PROOF-PATCH-W88 — paste-ready patch proposal for the integration README

Seat W88, 2026-11-14. Prepared against `docs/integration/README.md`
at HEAD `743d248` (123 lines). **The README is lead-owned and was
NOT edited by this seat.** Every item below quotes the current
text with its line numbers at that HEAD, gives the exact
replacement text to paste, and cites the evidence behind it.

Classes: **CORRECTION** = the current text asserts something
false or unproven. **COMPLETION** = the current text carries a
TBD/pointer that the record now resolves (in whole or part).
Where evidence is still pending, the proposed text says PENDING
and names the precise discharge condition — no replacement below
claims a success the record does not carry.

Counts: **6 patch items — 4 CORRECTIONS (5 paste sites) and
2 COMPLETIONS.** The pending-evidence list is consolidated at
the end.

---

## Item 1 — COMPLETION (mixed: 1 discharged, 3 PENDING/PARTIAL) — Proof section, lines 72–78

### Current text

```
## Proof (this seat)

TBD-PROOF: telemetry verdict; per-shot A/B PSNR table headline;
MP4 ffprobe + loudness; determinism byte-compare; format v2
proof render. Renderer of record for every image on this VM:
**llvmpipe/lavapipe (software Vulkan)** — labelled as such;
no performance claims are made from it.
```

### Replacement text

```
## Proof (this seat)

Status of the proof items, against the record:

- **Per-shot A/B — MEASURED.** The full film rendered natively
  in CI at `381ff47` (10 dispatch runs, 1,811 frames tiling
  [0, 1811)); 60/60 sampled pairs measured against the Chrome
  review stills with `tools/ab/measure_ab.py` film mode:
  overall PSNR min 18.378 / mean 21.836 dB (native CI lavapipe
  vs Chrome SwiftShader, q0.93 JPEG references — reported per
  contract §5 (B), never gated as equality). The §28 anchor
  pair (t = 1.50) measures 24.094 dB; the lowest segment is
  s11 (mean 18.523 dB). Full table, per-hypothesis
  dispositions and worst-3 images: `docs/ab/film/RESULTS.md`.
- **Telemetry verdict — PENDING.** No `FILM TELEMETRY:`
  verdict line for the rendered head is in the record yet.
  Discharge: a run of `talerender --film-telemetry` on a
  current binary whose stdout verdict line (`FILM TELEMETRY:
  CHECK OK`, or `FILM TELEMETRY: CHECK FAILED (…)` with its
  itemisation) is recorded in `docs/ab/film/RESULTS.md`.
  The exit code is 0 on pass, fail and error alike; only
  the line counts.
- **MP4 ffprobe + loudness — PENDING.** No finished MP4
  exists yet. Discharge: `film/finish.sh` over the 1,811
  frames + the project's `data/mix.wav`, with
  `docs/ab/film/MP4-EVIDENCE.md` filled from the file
  (ffprobe streams/format, counted frames = 1811, mux
  loudness against the mix's ≈ −14.0 LUFS).
- **Determinism byte-compare — PARTIAL.** Measured on the
  production frames themselves (see the resume note under
  Driver CLI): overlap frames 1200–1204 are byte-identical
  across invocations; frames 1195–1199 differ at 72.34 dB
  past the paint-cache cap. The determinism pack's formal
  discharge — one recorded `DETERMINISM: PASS` per renderer
  of record (`tools/determinism/`) — is still PENDING.
- **Format v2 proof render — PARTIAL.** The loader consumes
  the full v2 surface (loader-status addendum,
  `docs/scene-format.md` §14.5: lights rig, character node
  incl. attachments, camera keys, canonical grade keys, the
  top-level `fog` block, all 13 texture generators), and v2
  scene files have rendered through it under verification
  (the fog chart, ≤1 sRGB code at every panel). PENDING:
  the fx-textures acceptance render and a recorded render
  of `scenes/character-v2.scene.json`. Discharge: a
  `render.yml` `mode=scene-file` dispatch (or a CI-binary
  run) at ≥ `c74cf0c` with the PNG and outcome recorded.

Renderer of record for every image on this VM:
**llvmpipe/lavapipe (software Vulkan)** — labelled as such;
no performance claims are made from it. (The film frames
above are CI lavapipe, same labelling.)
```

### Evidence

- A/B: `docs/ab/film/RESULTS.md` — headline "pairs measured:
  60; missing: 0; overall PSNR min 18.378 / mean 21.836",
  per-segment table (s01 min 24.094 at the anchor pair; s11
  mean 18.523, the lowest), run/artifact record for the 10
  `381ff47` dispatch runs tiling [0, 1811).
- Telemetry PENDING: RESULTS.md, "Still pending for this
  package" — "**FILM TELEMETRY verdict line** for head
  `381ff47` — not run by this seat … the gate verdict is the
  lead's to record". No `FILM TELEMETRY: CHECK OK` line exists
  in any committed record (tree-wide grep, this seat).
- MP4 PENDING: RESULTS.md pending list ("no finished MP4
  exists yet") + `docs/ab/film/MP4-EVIDENCE.md` still an
  unfilled template (every measured field `_pending_`).
- Determinism PARTIAL: RESULTS.md chunk-overlap check (see
  Item 2) + `tools/determinism/README.md` §"How the verdict
  discharges the TBD item" (discharge requires the recorded
  PASS lines; only stub-binary PASSes exist so far — W67/W78).
- Format v2 PARTIAL: commits `c74cf0c` (loader lane: 6
  painters + `solid` arm + exposure), `69952ab` (fog wiring),
  `5b0889d` (§14.5 loader-status addendum; fog chart ≤1 code);
  `docs/integration/LOADER-ACCEPTANCE-W71.md` (13/13 texture
  generators armed; fx-textures "passable in principle", the
  acceptance render itself not yet run).

---

## Item 2 — CORRECTION (W50 finding C4) — Driver CLI resume claim, lines 44–47

### Current text

```
    Resume: frames already present in the output dir are skipped,
    so a chunked/interrupted run loses nothing; chunking never
    changes a frame (frame(t) is pure — proven by re-rendering a
    chunk boundary twice and byte-comparing).
```

W50's C4: the same file's Proof section listed "determinism
byte-compare" as TBD-PROOF — the CLI section claimed as proven
what the Proof section admitted was outstanding, and no
byte-compare evidence existed anywhere in the tree. Evidence
now exists, and it is two-part: it supports byte-identity in
the tested ranges and *refutes* the unqualified "never" past
the paint-cache cap. The replacement states exactly that, no
stronger, no weaker.

### Replacement text

```
    Resume: frames already present in the output dir are skipped,
    so a chunked/interrupted run loses nothing. On purity:
    chunk-boundary byte-identity is proven for the ranges the
    determinism pack tests (`tools/determinism/`), and on the
    production render itself the overlap frames 1200–1204 are
    byte-identical across invocations. One measured exception,
    stated exactly: frames 1195–1199, rendered deep into a
    chunk (past the 160-entry paint-cache cap), differed
    across invocations at PSNR 72.34 dB — 1,787 of 2,073,600
    px, 86% of them at |Δ| = 1 (leading hypothesis: a paint
    re-render after FIFO cache eviction is not bit-identical
    to its first render). So within a cache-resident working
    set chunking never changes a frame; past the cap a
    re-rendered frame can differ by ±1 code on a fraction of
    a percent of pixels. Evidence: `docs/ab/film/RESULTS.md`,
    chunk-overlap check.
```

Companion tweak (optional, same item): the bridge diagram's
gloss at line 33, "frame(t) is a pure function", is the design
statement and can stand, but for consistency append to it:
"(see the resume note under Driver CLI for the one measured
exception)".

### Evidence

- `docs/ab/film/RESULTS.md`, "Chunk-overlap consistency
  check": frames 1200–1204 **byte-identical** (sha256 match,
  pair-mode PSNR = ∞); frames 1195–1199 differ at **PSNR
  72.34 dB**, 1,787 of 2,073,600 px (0.086%), 86% at |Δ| = 1,
  max |Δ| 28, clustered in the lower-left foliage; the chunk
  invocation had rendered 195 frames when it reached 1195,
  the probe 0–4, cache cap 160 entries (`1475b66`, which
  recorded the *assumption* that eviction is pixel-neutral).
- `tools/determinism/` pack: stub-proven in both directions
  (PASS on identical bytes, FAIL naming the frame on a flipped
  byte, stale-binary refusal), currency-verified 8/8 against
  current source by W78 — but no real-binary PASS recorded
  yet, which is why the replacement claims proof only "for
  the ranges the determinism pack tests" via the production
  overlap measurement, and Item 1 keeps the pack's formal
  discharge PENDING.

---

## Item 3 — CORRECTION (overclaim; W50 finding C2's logic) — header, lines 3–6

### Current text

```
How a Tall Tale project becomes a complete native film in
TaleRender, proven end-to-end on **"THE SOLDIER WHO DIDN'T KNOW
THE WAR WAS OVER"** (the Onoda project, 60.357 s, 12 shots /
14 segments, 1,811 frames at 1080×1920 @ 30 fps).
```

The unqualified "proven end-to-end" asserts the verdicts the
Proof section itself lists as outstanding. The render half of
it is now true (1,811 frames, CI, `381ff47`, A/B measured);
the telemetry verdict and MP4 finish are not in the record.
The replacement keeps the true half and defers to the Proof
section for the rest.

### Replacement text

```
How a Tall Tale project becomes a complete native film in
TaleRender, carried end-to-end on **"THE SOLDIER WHO DIDN'T
KNOW THE WAR WAS OVER"** (the Onoda project, 60.357 s,
12 shots / 14 segments, 1,811 frames at 1080×1920 @ 30 fps).
The film has rendered natively end-to-end (CI, `381ff47`)
and its per-shot A/B is measured; the telemetry verdict and
the MP4 finish are still outstanding — the Proof section
below is the status of record, item by item.
```

### Evidence

- Render + A/B: `docs/ab/film/RESULTS.md` (10 dispatch runs
  at `381ff47`, artifacts tiling [0, 1811); 60/60 pairs).
- Outstanding items: RESULTS.md "Still pending for this
  package" (telemetry verdict line, MP4 finish + evidence,
  golden staging, PC RADV evidence).

---

## Item 4 — CORRECTION (W50 finding C5) — Fog limit bullet, lines 92–96

### Current text

```
- **Fog** is implemented in the PBR/basic shaders as an append
  (three.js Fog smoothstep over view depth); the ab1 proof path
  keeps fog disabled bit-exactly, and the fog path itself is
  validated by the film's S06 (fog set) A/B frames, not by a
  separate gate.
```

W50's C5: wrong on both halves at the time — no S06 A/B frames
existed yet, and a separate gate (the analytic fog chart) did
exist. Both halves have since resolved, in opposite
directions: the S06 A/B frames now exist and measure well,
*and* the separate gate has run on both sides. The current
text is additionally stale on scope: since `69952ab` the
scene-file loader consumes fog too, which the bullet (written
for the film lane only) does not mention.

### Replacement text

```
- **Fog** is implemented in the PBR/basic shaders as an append
  (three.js Fog smoothstep over view depth); the ab1 proof path
  keeps fog disabled bit-exactly. The scene-file loader also
  consumes the v2 `fog` block (`69952ab`): on the composer
  path, three's fog_fragment is applied to the resolved HDR
  before bokeh, from the packed-depth prepass; the direct
  (non-composer) path stays fog-free. Validation is by a
  separate gate — the analytic fog chart (`tools/fog-chart/`):
  the Chrome side confirms smoothstep within 1 sRGB code at
  every panel depth, and the native loader render verifies
  to ≤1 sRGB code at every panel (`5b0889d`) — and by the
  film's S06 (fog set) A/B pairs, mean 24.77 dB with no
  depth-growing delta signature (`docs/ab/film/RESULTS.md`).
```

### Evidence

- Loader wiring: commit `69952ab` (scenefile parses the v2
  fog block {color, near, far}; new `shader_fog.wgsl`;
  run_composer applies fog_fragment pre-bokeh via
  perspectiveDepthToViewZ over the packed-depth prepass;
  direct path fog-free).
- Separate gate, Chrome side: `tools/fog-chart/chrome/
  VERIFY.md` (W66 — harness run; basic panels within 1 code
  of the analytic smoothstep at all five depths).
- Separate gate, native side: commit `5b0889d` (§14.5
  loader-status addendum — "verified against the analytic
  fog chart to ≤1 sRGB code at every panel").
- S06 A/B: RESULTS.md segment table (s06: 4 pairs, PSNR mean
  24.77 dB) + hypothesis disposition ("s06 fog risk refuted:
  no depth-growing delta signature").
- Superseded status note: `docs/integration/
  FOG-STATUS-W74.md` recorded verdict (c) "ignored entirely"
  at `3781eb2`; `69952ab` landed after it and closed the item
  (a closure note for that doc is a separate wave item).

---

## Item 5 — CORRECTION (W50/W52 item B1) — telemetry verdict string, two sites

The binary's verdict line is `FILM TELEMETRY: CHECK OK` /
`FILM TELEMETRY: CHECK FAILED (…)`, and the exit code is 0 on
pass, fail, and error alike — so the line, in its full form,
is the only machine- and human-readable verdict. The README
twice prints the truncated form "CHECK OK". Any reader (or
script) matching the README's string against real output will
miss the verdict; the skill adapter's gate in fact matches
the exact full line.

### Site A — Driver CLI, lines 52–54

Current text:

```
    the strict caption audit. Prints CHECK OK or itemises every
    failure. No GPU needed.
```

Replacement text:

```
    the strict caption audit. Prints the verdict line
    `FILM TELEMETRY: CHECK OK` — or `FILM TELEMETRY:
    CHECK FAILED (…)`, itemising every failure. The exit
    code is 0 on pass, fail and error alike; the line is
    the verdict. No GPU needed.
```

### Site B — Runbook step 3, line 119

Current text:

```
3. `talerender --film-telemetry film` — expect CHECK OK.
```

Replacement text:

```
3. `talerender --film-telemetry film` — expect the verdict
   line `FILM TELEMETRY: CHECK OK`.
```

### Evidence

- `docs/runbook/PERF.md` (telemetry outcome table): FAILED =
  exit **0**, stdout line `FILM TELEMETRY: CHECK FAILED
  (<n> overflow groups, <n> continuity violations,
  <n> caption failures)`.
- `docs/runbook/USER-PC.md`:78–81 — the expected line printed
  in full (`FILM TELEMETRY: CHECK OK`); failure form likewise.
- `docs/integration/RED-TEAM-W70.md` — the adapter gate
  matches the exact full line (FAILED tested before OK;
  lines `trimEnd()`ed); a binary printing only "CHECK OK"
  would not pass it.
- `docs/ab/film/README.md`:125 — the proof package's own
  charter already states "the verdict is the printed line:
  FILM TELEMETRY: CHECK OK".

---

## Item 6 — COMPLETION (pointer) — end of the Format v2 section, after line 69

The Format v2 section (lines 58–69) describes the v2 surface
as specified; since `c74cf0c` / `69952ab` / `5b0889d` the
loader's consumption of that surface is a matter of record,
but the README never points at it. This is an addition, not a
replacement — insert after line 69 (the section's last line,
"nodes for character and rifle."), before the Proof heading.

### Text to insert

```
Loader consumption of everything this section declares is
recorded in the §14.5 loader-status addendum in
`docs/scene-format.md` (post-`c74cf0c` / `69952ab`): the
loader consumes the `lights` rig, the character node incl.
`attachments`, camera `keys`, `post.grade` under the
canonical keys sat / warm / exposure / time, the top-level
`fog` block, and all 13 texture generators.
```

### Evidence

- `docs/scene-format.md` §14.5 loader-status addendum
  (commit `5b0889d`), quoted in full in that file.
- `docs/integration/LOADER-ACCEPTANCE-W71.md` §1 — census:
  13/13 generators armed in `resolve_texture`
  (`src/scenefile.rs:399`), the 7 previously-unarmed ones
  being exactly the arms `c74cf0c` added.

---

## Pending-evidence list (consolidated)

Nothing below is claimed anywhere in the proposed texts;
each is carried as PENDING/PARTIAL with its discharge
condition stated in Item 1 (and Item 2 for determinism).

1. **Telemetry verdict line** for the rendered head
   (≥ `381ff47`). Discharge: a `--film-telemetry` run on a
   current binary with the stdout verdict line recorded in
   `docs/ab/film/RESULTS.md`. (An independent run at
   `TALERENDER_PAINT_CACHE=24` is in flight in this batch —
   W85 — but no verdict line is in the committed record as
   of this proposal.)
2. **MP4 finish + evidence.** Discharge: `film/finish.sh`
   over the 1,811 CI frames + `data/mix.wav`;
   `docs/ab/film/MP4-EVIDENCE.md` filled from the file
   (ffprobe fields, counted frames = 1811, mux loudness vs
   the mix's ≈ −14.0 LUFS). (Finish in flight — W84.)
3. **Determinism pack formal PASS per renderer of record**
   (CI lavapipe **and** PC RADV), per
   `tools/determinism/README.md`'s discharge criteria: the
   results file's header block, both `section: 30/30
   identical` lines, and the `DETERMINISM: PASS` line, cited
   in the proof package. Interim evidence (production-frame
   overlap, Item 2) does not discharge this.
4. **fx-textures acceptance render** and a **recorded
   render of `scenes/character-v2.scene.json`**.
   Discharge: `render.yml` `mode=scene-file` dispatch (or
   CI-binary run) at ≥ `c74cf0c` with PNG + outcome recorded.
   (Local evidence in flight — W86.)
5. **PC RADV evidence** (all native numbers so far are CI /
   VM lavapipe; the runbook's RADV route is unexercised).
   Not a README claim today; listed because Items 1–2's
   renderer labelling depends on the distinction.

Also noted, not a patch item: golden staging/pinning for the
14 film mid-frames remains outstanding (hashes staged in
RESULTS.md; pinning is a human act, and four mids sit at
chunk-invocation offsets ≥ 160, which interacts with Item
2's cache-cap finding). The README makes no golden claim, so
no text is proposed for it.
