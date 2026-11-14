# Incremental re-render planner — W27 (TaleRender Wave 2, Phases 3/6)

**Deliverable.** `tools/film/rerender_plan.py` — given two snapshots
of the film data (OLD = as rendered, NEW = as modified), it computes
the minimal frame ranges to delete and re-render, a reason per
range, and the ranges that must NOT be touched. Tests:
`tools/film/fixtures/rerender/run_tests.py` over the miniature
fixture film in `tools/film/fixtures/rerender/mini-*` plus two
real-history reconstructions — **17/17 checks pass** (§5).

This answers audit §6's "incremental rendering" ask at the data
level: the driver already skips existing frames (resume), but until
now nothing computed *which* frames a data change affects — Wave 2's
own re-chunk priced its impact by hand (W92/W01: "≈10 of the 60 A/B
pairs"). This tool reproduces that analysis mechanically, exactly,
and for arbitrary data diffs.

---

## 1. Frame arithmetic (the driver's own — off-by-one discipline)

- fps is read from the data (30; `src/film.rs` hardcodes 30 and
  evaluates frame *n* at `t = n / 30.0`).
- A caption chunk is on screen for frame *n* iff `s <= n/fps < e`
  — `eval_frame`'s own test (`t >= c.s && t < c.e`, first match).
  Windows are computed by performing that comparison in float
  arithmetic, not by rounding formulas, so boundary frames are
  exact by construction. There is no epsilon anywhere in the
  window path; the only epsilon in the tool is the validator's
  `-1e-9` in the fallback segment-frame derivation, used only when
  a manifest omits `startFrame`/`endFrame`.
- Caption windows are **global** (film time), never clipped to the
  owning segment — chunks bleed across cuts (chunk 51 ends 0.03 s
  into S10; chunk 41 bleeds 1.16 s into S07) and the driver does
  not clip either. The caption's 0.1 s scale-in lives inside the
  window (`lt = t - s`), so it adds no frames.
- Segment ranges come from the manifest (`startFrame`/`endFrame`,
  the tiling the validator checks). **Caveat:** the driver's
  in-code shot cuts for s05b / s07 / s08b sit at anchor-derived
  times (word anchor − 0.12 / 0.25 / 0.1 s) up to ~0.25 s *before*
  the manifest boundary, so a segment-range plan can under-cover
  by up to ~8 frames at those three cuts. A lead widening a
  segment range by a few frames into the preceding segment at
  those cuts is being exact, not sloppy. Caption windows are
  unaffected (global arithmetic).
- A chunk whose `s·fps` or `e·fps` lands within 1e-6 of an integer
  is flagged in the notes as boundary-exact: the driver's JSON
  parser is hand-rolled, and a 1-ulp parse difference against
  Python's could flip that single frame — verify by telemetry if
  it ever fires (it does not fire on the current film data).

## 2. Change rules

| Change | Plan |
|---|---|
| Caption chunk's text / s / e / fit differs | Union of the old and new chunk windows. Chunks are matched by **content**, never by index — a pure renumbering after a split is pixel-identical and yields **no** frames. |
| Scene file changes outside `captions` | The segment's full range (see limits: sub-segment pixel dependency is all-or-nothing). |
| Scene `captionsNote`, manifest `captionChunks` re-tile | **Note only.** `captionsNote` is prose (nothing in `src/` reads it); `captionChunks` is derived — pixel impact comes from the chunk diff itself. |
| Manifest timing fields differ at all | Union of the segment's old and new ranges. Motion is continuous in *t*, so even a sub-frame shift re-evaluates every frame of the segment. |
| New film shorter | Plus a **delete-only stale tail** `[newTotal, oldTotal)` — not hygiene: `finish.sh`'s image2 demuxer would encode leftover tail frames into the MP4. |
| Manifest set / atmosphere / filmLook changes; filmLook definition changes in `atmospheres.json`; scene dressing changes | Segment's full range, **expanded through the inheritance graph** (§3). |
| `film/onoda.json` changes outside `captions` (lines / anchors / duration) | **Full film.** Shot boundaries and staging derive from lines+anchors *in code*; not decomposable from data, and the planner says so instead of guessing. |
| Asset bytes change | Segments whose scene data references the asset (scene files name their plate files: leaflet→s05b, coconut→s07, paper-map→s08/s08b/s09, book→s10). `plates.json` resolves per plate. An asset nothing references → full film, flagged conservative. |
| Caption strips `cap_*.png`, strip census `manifest.json` | **Excluded from byte diffing, deliberately.** A strip is a pure function of its chunk's (text, fit) — both already in the chunk key — and the generator is byte-deterministic (W01 §1/§4). Name-based strip diffs after a renumbering would report ~59 false changes. |

## 3. Atmosphere inheritance (encoded facts)

`s08`, `s08b`, `s09` inherit **s07's golden dressing**: each shot
function in `src/film.rs` sets `sky_jg = TEX_SKY_GOLDEN` with the
comment *"inherits S07's golden dressing: noon passes sky:false
and never restores dome, clouds, or the key position (golden
keyPos [−8, 4.5, 5] persists; noon colours/intensities DO apply)"*
(lead fix `1fc6fab`; also recorded in `docs/integration/README.md`).
So a dressing change to s07 expands the plan to s08 + s08b + s09.

The graph is data, layered: the built-in table `{s07: [s08, s08b,
s09]}` → overlaid by `inheritance.json` next to the manifest (or at
the snapshot root) when present → overlaid by `--inherit
s07:s08,s08b,s09`. Expansion is transitive. **Only dressing causes
expand** (scene non-caption change, manifest dressing change,
filmLook change): the inherited state is dome/cloud/key-position,
not time or text, so timing and caption changes to s07 do *not*
drag the inheritors in.

## 4. Deletion-safety contract + pipeline integration

- **The tool never deletes, renders, or writes anything.** It
  prints a plan. `--print-rm FRAMES_DIR` prints *only* the `rm -f`
  commands (40 files per line) for a human/lead to review and run
  — verified in testing: pointed at a nonexistent frames dir, it
  emitted 296 file names and touched nothing.
- Workflow: run the planner → review the ranges and reasons → run
  the `--print-rm` output (or delete the ranges by hand) → re-run
  the driver over the film dir: resume skips every surviving frame
  and re-renders exactly the deleted ones (or drive single ranges
  with `--from A --to B`) → re-run `film/finish.sh FRAMES MIX OUT`
  → re-run the film telemetry gate before believing the result.
- Exit codes follow the repo tool convention (validate_film.py):
  0 = plan computed (possibly empty), 2 = input error, one line on
  stderr, no plan.

## 5. Verification (all first-hand, this seat)

Synthetic suite (miniature 3-segment film, 240 frames): identical
snapshots → empty plan; caption-text change → exactly `[75, 120)`;
caption split → exactly the outer window `[75, 120)`; renumber-only
→ empty; scene-lights change → exactly segment b `[60, 150)`;
timing shift → union `[60, 255)` with segment a untouched;
fixture inheritance b→c → `[60, 240)` with the inheritance reason
named; stale tail → `[210, 240)` flagged delete-only; driver-only
snapshot: caption change → `[30, 60)`, anchors change → full film.
**10/10 synthetic case groups pass (13 checks).**

**The real case — full agreement with the record.** Reconstructed
dir-vs-dir from git history (`git archive`):

- Scenes side, `3a36f1f^` → `3a36f1f`: the plan is **exactly the
  seven split windows** — `[201, 256)`, `[649, 679)`, `[947, 990)`,
  `[1114, 1154)`, `[1321, 1362)`, `[1392, 1434)`, `[1563, 1608)` —
  **296 of 1,811 frames (16.3%)**, each range's reasons naming the
  split chunk and its two pieces. Nothing else: the probe confirms
  that commit changed only `captions`/`captionsNote` keys in the
  scene files and only `captionChunks` in the manifest, and the
  planner prices all of that at zero extra frames.
- Cross-check against W92/W01's manual analysis: **exactly 10** of
  the 60 one-second A/B sample stills fall inside the plan — the
  same ten stills (t = 7.50 … 53.50) the record lists. Several
  windows *just* exclude their neighbouring stills (frame 945 at
  t = 31.50 vs window start 947; frame 1155 at t = 38.50 vs window
  end 1154), which is the off-by-one discipline working as
  designed.
- Driver side, `5e90d7e^` → `5e90d7e` (`film/onoda.json` +
  `film/assets` only): the identical seven windows — the two data
  sides agree, as they must after the lead's regeneration.

No disagreement with the record was found on either side.

## 6. Limits (stated, not hidden)

1. **Sub-segment pixel dependency is all-or-nothing.** A one-light
   change inside a segment re-renders the whole segment. Finer
   granularity would need dependency data the scene format does
   not carry (which nodes a light touches, etc.).
2. **Code changes are out of scope.** The planner diffs *data*
   snapshots. A change to `src/film.rs` staging invalidates
   whatever it touches in ways no data diff can see — after a
   renderer change, re-render what the change's own record says,
   or everything.
3. **Driver non-caption data is a full-film trigger** (anchors /
   lines / duration), because shot boundaries are computed from
   them in code. Narrowing this needs the boundary expressions
   exported as data.
4. **Unreferenced assets fall back to full film** (conservative).
   Every plate in the current film *is* referenced from scene
   data, so the fallback does not fire today.
5. **The three anchor-offset cuts** (§1) can make a segment range
   under-cover by up to ~8 frames; widen manually there.
6. Windows assume the snapshots' fps agree; an fps change is a
   full-film trigger by construction (every frame's *t* moves).

---

*Scope compliance: new files only — `tools/film/rerender_plan.py`,
`tools/film/fixtures/rerender/` (fixtures + `run_tests.py`), this
note. No `src/`, scene, or other-tool edits; nothing committed.*
