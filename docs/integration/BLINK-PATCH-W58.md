# BLINK-PATCH-W58 — Blink adoption patch pack: charanim smooth blink, beats maxed back in

**Seat:** Wave 2, Batch 6, W58. **Surveyed at HEAD `aa92cee`** (all
line numbers are that head's). **Deliverables:** this pack +
`tools/perf/blink_census.py` (the expected-difference census, §8).
New files only; nothing edited, nothing committed.

**What this prepares:** replacing film.rs's binary `Blinker` with
W12's charanim blink channel, in the form W33's measured decision
recommends — **smooth, with the seven authored word-beat blinks
maxed back in** (`BLINK-DECISION-W33.md`: smooth ceiling +219
paints / 6.06% of lookups; beats preserved at +0 measured cost).
The variant choice is the lead's at adoption; §9 gives the
one-line quantized swap if the lead rules the other way.

---

## 1. Wiring state — verified at HEAD

- **Registered:** `src/main.rs:23` — `mod charanim;` sits in the
  crate-root module list (Batch 2 registration, `2328e6f`).
- **Zero call sites — CONFIRMED:** `grep -rn "charanim::" src/`
  outside `src/charanim.rs` itself returns no matches. Nothing in
  film.rs, scenefile.rs, or main.rs references the module; the
  Batch-5 matrix's "registered, zero call sites" is still exact.
- **Surface this patch uses:** `pub fn blink_amount(id: u64,
  t: f64) -> f32` (`src/charanim.rs:155`). Nothing else.
- **Deliberately NOT used:** `channels_at(id, t)`
  (`src/charanim.rs:615`). It returns blink *bundled with idle
  deltas* (bob / lean / head_tilt / head_turn / squash,
  `CharStateDelta`, charanim.rs:604-613). Adopting blink through
  it would smuggle the idle channel into the same patch — a
  separate decision with its own cost surface (W33 §6 scope
  note). This patch calls `blink_amount` directly so the diff
  touches the blink field and nothing else.

## 2. The composition — the beats law, kept structurally

Today (`src/film.rs:1400-1402`), verbatim:

```rust
fn blink_of(seed: f64, t: f64, beats: &[f64]) -> f32 {
    blink_max(Blinker { seed }.at(t), blink_at(t, beats)) as f32
}
```

with `blink_at` at film.rs:213-219 ("1 while 0 <= t - beat <
0.12") and `blink_max` at film.rs:221-223. The patch changes
**one expression inside `blink_of`** — the scheduled channel —
and leaves the signature, the beats argument, and the `max`
untouched:

```rust
fn blink_of(seed: f64, t: f64, beats: &[f64]) -> f32 {
    blink_max(crate::charanim::blink_amount(seed as u64, t) as f64, blink_at(t, beats)) as f32
}
```

Why this exact form:

- `seed as u64` is the mapping charanim itself prescribes
  (charanim.rs:152-155: "The film's per-character blink seeds …
  map to ids by `seed as u64`"). All five seeds in staging are
  exact integer-valued literals (3, 5, 7, 9, 11), so the cast is
  lossless.
- The census (§8) models the adopted value as
  `f32(max(blink_amount_f64(id, t), blink_at))` — channel in f64,
  maxed, then cast. The patch widens the channel's f32 result
  back to f64 before the max. The two are **bit-identical**:
  `blink_at` only ever returns 0.0 or 1.0; at beat = 1.0 both
  forms give exactly 1.0 (channel ≤ 1), and at beat = 0.0 both
  give the channel's f32 value unchanged (f32→f64 widening and
  the final cast are both exact for f32-representable values).
- Because the composition stays inside `blink_of`, the seven
  authored beats survive **structurally** — no call site can
  lose its beats argument, which is the failure mode W33 warned
  against ("adopting the channel as a bare replacement silently
  deletes all seven"). The census proves the outcome frame by
  frame (§8: 25 beat character-frames, 0 changed).

`crate::charanim::` needs no import plumbing: film.rs already
addresses sibling modules by crate path (e.g.
`crate::paintbudget::entry_bytes`, film.rs PaintCache impl).

## 3. Call-site catalogue — 28 assignments, 2 shapes

### 3a. The seven `blink_of` sites — NO change at the site

Only `blink_of`'s body changes (§2); these lines stay byte-for-
byte as they are. Quoted so the patcher can verify none is
missed:

| Line | Shot | Character (seed) | Verbatim |
|---|---|---|---|
| 1416 | s01 | Onoda (3) | `st.blink = blink_of(3.0, t, &[data.anchors["years0"]]);` |
| 1485 | s03 | Onoda (3) | `onoda.blink = blink_of(3.0, t, &[w_never]);` |
| 1522 | s04 | Onoda (3) | `onoda.blink = blink_of(3.0, t, &[data.anchors["message3"]]);` |
| 1755 | s08 | Suzuki (5) | `suz.blink = blink_of(5.0, t, &[w_found]);` |
| 1822 | s09 | Onoda (3) | `onoda.blink = blink_of(3.0, t, &[w_comm]);` |
| 1863 | s10 | Taniguchi (9) | `tani.blink = blink_of(9.0, t, &[w_books]);` |
| 1980 | s12 | Onoda (3) | `onoda.blink = blink_of(3.0, t, &[w_loyal]);` |

### 3b. The 21 bare `Blinker` sites — one-line replacement each

Current shape (all 21): `X.blink = Blinker { seed: S }.at(t) as f32;`
Replacement shape: `X.blink = crate::charanim::blink_amount(S, t);`
— no `as f32` (the channel already returns f32), no seed
variable: the integer literal *is* the charanim id.

| Line | Shot | Character (seed → id) | Current line (verbatim) |
|---|---|---|---|
| 1448 | s02 | Onoda (3) | `so.blink = Blinker { seed: 3.0 }.at(t) as f32;` |
| 1454 | s02 | compA (7) | `sa.blink = Blinker { seed: 7.0 }.at(t) as f32;` |
| 1460 | s02 | compB (11) | `sb.blink = Blinker { seed: 11.0 }.at(t) as f32;` |
| 1479 | s03 | Taniguchi (9) | `tani.blink = Blinker { seed: 9.0 }.at(t) as f32;` |
| 1490 | s03 | compA (7) | `ca.blink = Blinker { seed: 7.0 }.at(t) as f32;` |
| 1495 | s03 | compB (11) | `cb.blink = Blinker { seed: 11.0 }.at(t) as f32;` |
| 1509 | s04 | compA (7) | `ca.blink = Blinker { seed: 7.0 }.at(t) as f32;` |
| 1515 | s04 | compB (11) | `cb.blink = Blinker { seed: 11.0 }.at(t) as f32;` |
| 1603 | s05b | Onoda (3) | `onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;` |
| 1643 | s06 | compA (7) | `ca.blink = Blinker { seed: 7.0 }.at(t) as f32;` |
| 1646 | s06 | compB (11) | `cb.blink = Blinker { seed: 11.0 }.at(t) as f32;` |
| 1662 | s06 | Onoda (3) | `onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;` |
| 1723 | s07 | Onoda (3) | `onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;` |
| 1770 | s08 | Onoda (3) | `onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;` |
| 1790 | s08b | Onoda (3) | `onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;` |
| 1795 | s08b | Suzuki (5) | `suz.blink = Blinker { seed: 5.0 }.at(t) as f32;` |
| 1830 | s09 | Suzuki (5) | `suz.blink = Blinker { seed: 5.0 }.at(t) as f32;` |
| 1925 | s11 | Onoda (3), walk-in branch | `onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;` |
| 1938 | s11 | Onoda (3), post-drop branch | `onoda.blink = Blinker { seed: 3.0 }.at(t) as f32;` |
| 1949 | s11 | Taniguchi (9) | `tani.blink = Blinker { seed: 9.0 }.at(t) as f32;` |
| 1986 | s12 | Taniguchi (9) | `tani.blink = Blinker { seed: 9.0 }.at(t) as f32;` |

Notes for the patcher:

- **s05a has no site** — it stages no characters at all (plane +
  leaflets only). Absence is correct, not a miss.
- **s11's two Onoda sites are both converted.** They are the two
  branches of the drop_t split; same character, same seed, so
  presence (and the schedule) is continuous across the split —
  W33's presence model assumes exactly this.
- **s05b's site converts but changes nothing:** no schedule —
  today's or the channel's — fires inside s05b's 47 frames
  (census §8: 0 changed frames; W33 observed the same). Do not
  read a byte-identical s05b as a missed conversion.
- Cross-check after patching: `grep -n "Blinker {" src/film.rs`
  must return only the struct definition and its impl (or
  nothing, per §6), and `grep -c "blink_amount" src/film.rs`
  must be 22 (21 sites + `blink_of`).

## 4. Beat data source — identified in staging (no gap)

The brief required citing where `blink_at(beats)` gets its data.
It is fully identified; no new plumbing is needed:

- **Storage:** `FilmData.anchors: HashMap<String, f64>`
  (`src/film.rs:44-46`), parsed from the film data file's
  top-level `"anchors"` object (`src/film.rs:77-80`) — the word
  timings generated with the rest of the film data
  (`film/onoda.json`; header comment film.rs:7). Values at
  HEAD: years0 4.2 · never2 14.78 · message3 20.7 · found7
  38.48 · commander8 44.04 · bookseller9 48.16 · loyal11
  58.12 (seconds, film time).
- **Staging reads:** five beats are lifted into locals at the
  top of their shot functions — `w_never` (film.rs:1468, s03),
  `w_found` (:1737, s08), `w_comm` (:1807, s09), `w_books`
  (:1842, s10), `w_loyal` (:1957, s12) — and two are read
  inline at the call site (`data.anchors["years0"]` :1416,
  `data.anchors["message3"]` :1522). All seven reach
  `blink_of` as its `beats` argument today; the patch does not
  touch any of these lines.
- **Beat frames** (frames where `blink_at` = 1 for the beat
  character; the census asserts all are bit-unchanged):
  years0 → 126-129 · never2 → 444-446 · message3 → 621-624 ·
  found7 → 1155-1157 · commander8 → 1322-1324 · bookseller9 →
  1445-1448 · loyal11 → 1744-1747. **25 character-frames.**
  Caution for gate-writers: beats are a *per-character* law —
  film frame 1747, e.g., is a loyal11 beat frame for Onoda
  (unchanged) but legitimately changes because Taniguchi's
  channel blink fires there (census, s12). Never assert
  whole-frame identity from the beat list alone.

## 5. Schedule determinism — the seed specification

The charanim schedule has **no seed input to source**: it is a
pure function of (character id, t), and the patch must keep it
that way.

- **Mechanism:** `blink_start(id, n)` (charanim.rs:131-141)
  builds start times by fixed-order f64 accumulation: first
  start `0.6 + 1.4·u₀`, each gap `2.6 + 1.8·uₖ`, where
  `u = seeded_unit(id, k, salt)` (charanim.rs:72-79) is the top
  53 bits of a splitmix64 finalizer (charanim.rs:61-68) over
  integer inputs only. Salts: `SALT_BLINK_FIRST = 0xB111_0001`,
  `SALT_BLINK_GAP = 0xB111_0002` (charanim.rs:80-81).
  `blink_amount` (charanim.rs:155-176) walks that schedule and
  evaluates a smootherstep down/up (0.12 s + 0.12 s) — f64
  add/mul and a polynomial, **no transcendentals, no stored
  state**, so it is bit-exact across platforms and runs.
- **The id is the seed, and it comes from staging:** the only
  per-character input is the id, which this patch takes from
  the seed literals already assigned in film.rs staging — the
  same constants that drive today's `Blinker` (seed registry
  documented at film.rs:443-447, "(look, perspective, blink
  seed) per instance index"). Adoption changes no seed
  assignment anywhere; each character keeps its identity and
  gains the smooth curve on its own schedule.
- **No clock, by construction and by grep:** `grep -n
  "SystemTime|Instant|rand" src/charanim.rs` returns nothing at
  HEAD. The patch must not introduce any: no wall-clock seeding,
  no per-run nonce, no frame-count-derived phase. If a future
  format wants per-film schedule variation, the knob is the id
  space (per-character constants in staging), never time.
- **Consequence for the cache:** the value that enters
  `state_key` (`src/film.rs:2161`; blink component
  `q(st.blink)` at :2176, exact f32 bit pattern per the
  cabcc30 law) is a deterministic function of (id, film t) —
  the same frame rendered twice, or on two fleets, keys
  identically. Gate 3 in §7 proves this on changed frames.

## 6. Residue — `Blinker` and `blink_max` after the patch

- `blink_max` (film.rs:221-223) stays: `blink_of` (§2 form)
  still calls it.
- `Blinker` (struct film.rs:184, `at()` :192-211) loses all 21
  call sites. Options:
  **(i) delete it in the same patch** — clean tree, but it is
  the in-tree transcription of motion.ts's blinkDirector that
  `tools/perf/blink_cost.py`'s header cites as its port source;
  or **(ii) keep it** as the reference transcription with a
  one-line comment ("superseded by charanim::blink_amount,
  W58; retained as the motion.ts reference"). This pack
  recommends (ii) for one release cycle — the census and cost
  tools' citations stay true while the adoption beds in — and
  deletion as a named follow-up, at which point the tools'
  header citations re-point to git history. Lead's call; the
  patch is complete under either.

## 7. The A/B gate — identity where promised, difference where expected

Baseline = full film render at the pre-patch head; candidate =
full film render at the patched head; **same renderer both
sides** (byte_probes' own law: both CI lavapipe, or both PC
RADV — never mixed). Frame dirs use the driver's
`frame_{n:05d}.png` naming (byte_probes default pattern).

**Gate 1 — the standing probe ranges, trimmed to blink-clean
frames (HARD, must PASS):**

```
python3 tools/ci/byte_probes.py BASELINE_DIR CANDIDATE_DIR \
    --ranges 195-203,795-813,1195-1204
```

The census (§8) intersects the changed set with the standing
ranges (DETERMINISM-CONTRACT-W04 §5) as follows:

| Standing range | Changed frames inside | Assertable sub-range |
|---|---|---|
| 195-204 (s02) | **204** — Onoda: today 1.0, channel 0.0 (REMOVED) | 195-203 |
| 795-814 (s06) | **814** — Onoda: today 0.0, channel 0.0272 (OPENED) | 795-813 |
| 1195-1204 (s08) | none | **1195-1204, whole range** |

1195-1204 passing whole matters beyond blink: it is the range
that straddles the old 160-entry cache cap where the quantised
state_key defect measured 72.34 dB (contract E2) — the patch
must not re-open it while adding ~7 distinct values per event.

**Gate 2 — no difference outside the expected set (HARD):**
diff all 1,811 frames byte-wise; the differing set must be a
**subset** of the census's 345 changed film frames
(`tools/perf/blink_census.py --json` emits the exact set,
per segment and in total). Any frame outside the set differing
means the patch touched more than blink — or nondeterminism —
and stops the adoption. The converse is *not* hard: a census
frame whose blink delta is tiny (frame 814's 0.0272 changes
eye height by 17·0.6·0.0272 ≈ 0.28 stage units) may
legitimately rasterise identically. Triage rule for a
shortfall: a missing difference where either model reaches
≥ 0.6 (the painter's closed-line threshold, toonfull.rs:593)
is a missed call site until proven otherwise; a shortfall
concentrated in sub-0.05 tail deltas is painter quantisation.

**Gate 3 — determinism on the new values (HARD):** render the
candidate twice; the changed frames must reproduce bit-
exactly:

```
python3 tools/ci/byte_probes.py CAND1_DIR CAND2_DIR \
    --ranges 48-54,560-568,1031-1059,1737-1753
```

(ranges chosen from the census's densest changed runs,
including the RESHAPED pair at 565-566).

**Gate 4 — cost and telemetry (MEASURED, reported):**

- The candidate render's `paint-cache:` line (W06 budget
  wiring): realised extra distinct states = (misses −
  rerenders) delta vs the baseline render. Must land **≤
  +219** (W33's r = 1 ceiling); W33 expects well under, since
  only 17.6% of blink frames fall in camera-hold windows.
- `FILM TELEMETRY: CHECK OK` with exit 0 on the patched build.
  Caveat, stated once: if the telemetry cutout model reads
  blink from this same staging, its expectation moves with the
  patch by design — resolve any telemetry delta against the
  census's per-character values (the ground truth for adopted
  blink), not by reverting the patch.

## 8. The census — the expected-difference set

`tools/perf/blink_census.py` (delivered with this pack)
enumerates, per staged character per frame, whether the f32
blink value entering `state_key` changes under smooth-with-
beats adoption vs today's binary blink. Models are imported
from `blink_cost.py` (no re-port — the tools cannot drift);
presence is re-transcribed at HEAD `aa92cee` and asserted
equal to W33's table. Reproduce:

```
python3 tools/perf/blink_census.py                 # tables on stdout
python3 tools/perf/blink_census.py --json out.json # full frame lists
python3 tools/perf/blink_census.py --against quant # vs blink_amount_q(·,8)+beats
```

**Reconciliation with W33 — all asserted in-tool, all PASS:**
lookups 3,612 = W33 · today's blink>0 frames 159 = W33 ·
adopted blink>0 frames 279 = W33 · decomposition identity
(279 = 27 both-on + 252 opened; 159 = 27 both-on + 132
removed) · **beats: 25 beat character-frames, 0 changed.**

**Film totals:** **386 changed character-frames** of 3,612
lookups (10.69%) — OPENED 252 (channel blinks where today has
none), REMOVED 132 (today's binary blinks the channel schedule
doesn't make), RESHAPED 2. **345 of 1,811 film frames** change
(at least one staged character). The only RESHAPED frames in
the film are **565 and 566** (s04, compB): today holds a full
1.0 shut while the channel is already reopening (0.487, 0.070)
— the two schedules' single overlap, and the cleanest possible
illustration of what adoption buys. Per-character changed
totals: Onoda 164 · compA 60 · compB 58 · Suzuki 46 ·
Taniguchi 58.

**Per-segment changed film frames** (the Gate-2 allowlist;
per-character lists are the tool's stdout / `--json`):

| Seg | Changed | Frames |
|---|---|---|
| s01 | 22 | 9-12, 48-54, 96-99, 162-168 |
| s02 | 56 | 204-207, 242-252, 257-263, 274-277, 285-288, 293-299, 331-333, 345-352, 355-362 |
| s03 | 34 | 402-405, 422-428, 435-441, 448-455, 468-475 |
| s04 | 38 | 495-498, 513-519, 541-543, 560-568, 596-606, 627-630 |
| s05a | 0 | — (nobody staged) |
| s05b | 0 | — (converted site; no schedule fires in-window) |
| s06 | 38 | 784-787, 814-825, 868-870, 872-878, 892-895, 901-908 |
| s07 | 11 | 922-928, 1020-1023 |
| s08 | 34 | 1031-1034, 1046-1059, 1113-1116, 1131-1142 |
| s08b | 12 | 1220-1223, 1241-1248 |
| s09 | 25 | 1255-1261, 1332-1335, 1337-1340, 1342-1351 |
| s10 | 21 | 1366-1372, 1392-1394, 1469-1476, 1485-1487 |
| s11 | 33 | 1520-1524, 1576-1582, 1617-1619, 1621-1627, 1638-1641, 1658-1664 |
| s12 | 21 | 1704-1706, 1731-1734, 1737-1743, 1747-1753 |

**Frames vs paints — the reconciliation made explicit:**
W33's +219 counts *distinct values per event* (extra paints
at r = 1): Σ(D−1) over the 42 adopted events, i.e. ΣD = 261
distinct blink values sampled across the film. This census
counts *frames*: those 261 distinct values live inside the
279 adopted on-frames (261 ≤ 279 necessarily — an event's
distinct values cannot exceed its frames), and the census's
on-frame totals match W33's independently simulated ones
exactly (159 / 279, asserted in-tool). The 386 changed
character-frames vs 219 extra paints are likewise
consistent, not interchangeable: every extra paint lands on
a changed frame, but a changed frame whose remaining state
is unique that frame anyway costs no paint.

**Quantized cross-check** (`--against quant`, for §9): smooth
vs quantized+beats differ on 254 character-frames — RESHAPED
199 (quantization snapping the same curve), OPENED 55
(tail frames the quantizer rounds to 0), REMOVED 0 (the
quantized on-set is a strict subset of the smooth on-set:
same schedule, snapped values).

## 9. If the lead adopts QUANTIZED instead

One-expression change at the same 22 places: `blink_amount_q(
seed as u64, t, 8)` in place of `blink_amount(seed as u64, t)`
(bare sites: `crate::charanim::blink_amount_q(S, t, 8)`).
Everything else in this pack — composition, beats, seed
specification, gates — is unchanged; the census to regenerate
the expected set for that pairing is the same tool family
(W33's quant+beats on-set: 224 frames; ceiling +123 / 3.41%).

## 10. Scope notes and risks

- **Blink only.** The patch changes no other `FullState`
  field; the painter needs no change (it already renders
  partial blink: eye height `1.0 − blink·0.6` at
  toonfull.rs:609, closed-eye line above 0.6 at :593).
- **Cost is real but bounded** (W33): up to +219 distinct
  states against a default paint budget of ≈2 GiB ≈ 74
  resident canvases — on long frozen shots the extra states
  can evict entries that later re-render. Rerenders are
  already a tracked budget line; Gate 4 measures the realised
  figure on the adoption render.
- **Channels are evaluated at film time t**, as today's call
  sites are (`Blinker.at(t)`, never `lt`) — the patch
  preserves that exactly. Feeding shot-local `lt` instead
  would shift event phases; W33 §2 records that the ceiling's
  scale would survive but the census's exact integers would
  not. Do not "tidy" t → lt while landing this.
- **No headline gap:** the beat data source (§4) and the seed
  source (§5) were both identified in staging, as the brief
  required; nothing in this pack rests on an unverified
  assumption about either.
