# GPUFIRST-FREQ — How often does the painter actually run, and on what?

**Seat:** FREQ, GPU-first TaleRender Phase 0.
**Repo:** `turjaun985123/talerender`, measured at HEAD `bc12a5c2bbb33d93416f25eaebbe8fec8c4eb678`
(verified by `git rev-parse HEAD` in the working copy before any measurement).
**Status:** evidence document. Nothing in this seat is committed; this file is delivered
uncommitted for the integrator.
**Raw evidence:** `~/workspace/talerender-evidence/gpufirst-phase0/freq-raw-stats.txt`
(verbatim `paint-cache:` / `film render complete:` lines from both CI logs).

Two state-frequency profiles are modelled:

- **Profile A (staged-pose)** — the Onoda film as staged by hand-written shot functions in
  `src/film.rs`. **Measured** (census CI logs). This is ground truth.
- **Profile B (per-frame-sampled)** — a native project whose character poses are baked as
  per-frame pose samples in scene JSON, evaluated through `filmseq` + `scenefile` + `eval15`.
  **Modelled**, not measured (§5). Formulas are given so Phase 1 can plug in a real sampled
  project's numbers.

---

## 1. Profile A — full-film paint statistics (measured)

### 1.1 Sources

| Run | Workflow | headSha | Created (UTC) | Conclusion |
|---|---|---|---|---|
| `43867793062` | Census (dispatch) | `2ef6f28fe2d3bde89dc183d8e7519593515e69eb` | 2026-11-16T12:34:26Z | success |
| `43808791603` | Census (dispatch) | `47d9ca22b802ce3c1e12917be02502cd884576ce` | 2026-11-16T05:13:13Z | success |

Both headShas are ancestors of the measured HEAD `bc12a5c` (which only re-labels the
determinism pack's adapter line). Fetched with
`gh run view <id> --repo turjaun985123/talerender --log` and grepped for `paint-cache:`.

The census renders the film in 8 chunk processes
(`.github/workflows/census.yml`: `for range in "0 250" … "1750 1811"`, one
`./target/release/talerender --film film` invocation per chunk, default cache settings).
Each chunk prints one completion line ending in its `paint-cache:` stats segment
(format defined at `src/paintbudget.rs:299`, `PaintBudget::stats_line`).
**The two runs' paint statistics are identical in every field of every chunk** — the
cache behaviour is deterministic across runs; only elapsed times and peak RSS differ
slightly. All totals below are therefore stated once and hold for both runs.

### 1.2 Per-chunk stats (both runs identical)

| Chunk (frames) | Frames | Hits | Misses (= CPU paints) | Requests (hits+misses) | Evictions (byte / cap) | Rerenders | Hit rate |
|---|---|---|---|---|---|---|---|
| 0..250 | 250 | 37 | 357 | 394 | 283 / 0 | 0 | 9.3 % |
| 250..500 | 250 | 275 | 562 | 837 | 488 / 0 | 0 | 32.8 % |
| 500..750 | 250 | 20 | 419 | 439 | 345 / 0 | 0 | 4.5 % |
| 750..1000 | 250 | 65 | 441 | 506 | 367 / 0 | 0 | 12.8 % |
| 1000..1250 | 250 | 147 | 326 | 473 | 252 / 0 | 0 | 31.0 % |
| 1250..1500 | 250 | 214 | 147 | 361 | 73 / 0 | 0 | 59.2 % |
| 1500..1750 | 250 | 172 | 308 | 480 | 234 / 0 | 0 | 35.8 % |
| 1750..1811 | 61 | 70 | 52 | 122 | 0 / 0 | 0 | 57.3 % |
| **Total** | **1,811** | **1,000** | **2,612** | **3,612** | **2,042 / 0** | **0** | **27.7 %** |

Arithmetic: hits 37+275+20+65+147+214+172+70 = **1,000**; misses
357+562+419+441+326+147+308+52 = **2,612**; requests = 1,000 + 2,612 = **3,612**;
hit rate = 1,000 / 3,612 = **27.69 %**; evictions 283+488+345+367+252+73+234+0 = **2,042**,
all byte-budget evictions, zero entry-cap evictions. The log's own verdict line confirms
the frame total: `CENSUS TOTAL FRAMES: 1811 (expected 1811)`.

Derived rates:

- **Character-paint requests per frame:** 3,612 / 1,811 = **1.994** (≈ 2 characters/frame).
- **Actual CPU paints executed per frame:** 2,612 / 1,811 = **1.442**.
- **Misses as a share of requests:** 2,612 / 3,612 = **72.3 %** — nearly three of every four
  character-paint requests are a full CPU paint, even with the cache live.
- Every full chunk ends at `entries=74 bytes=2133852160` against
  `budget_bytes=2147483648` (see §2 for why 74 is the ceiling).

### 1.3 The telemetry line is a separate sample — not a ninth chunk

Each run's log also contains one earlier `paint-cache:` line from the *Film telemetry*
step, run as `TALERENDER_PAINT_CACHE=24 ./target/release/talerender --film-telemetry film`
(`.github/workflows/census.yml:53`). It is a **different measurement** and is excluded from
the totals above:

```
paint-cache: entries=24 bytes=692060160 hits=96 misses=386 evictions=362 rerenders=0
peak_bytes=720896000 budget_bytes=692060160 hit_rate_permille=199 evictions_byte=0 evictions_cap=362
```

The legacy telemetry gate (`src/film.rs:4105` `run_film_telemetry`, cache constructed at
`src/film.rs:4140`) samples the film at 0.25 s intervals (`while t < data.duration`,
`t += 0.25` — 242 samples over 60.357 s), evaluating the same `eval_frame` staging through
a PaintCache capped at 24 entries (hence budget = 24 cards = 692,060,160 B, and all 362
evictions are entry-cap evictions). Its 482 requests (96 + 386) over 242 samples is again
≈ 1.99 characters per sample, consistent with §1.2. Its value to this seat is in §1.5.

### 1.4 The chunking caveat — direction and size

The census cache **resets at every chunk boundary** (a fresh `PaintCache::new()` per
process, `src/film.rs:3869`). The summed 2,612 misses therefore **overstate** the misses of
a hypothetical single-process full-film run, by two mechanisms:

1. **Cross-chunk duplicate first-touches.** A distinct state that occurs in *k* different
   chunks is painted (and counted as a miss) once per chunk, but only once in a
   single-process run (if never evicted in between).
2. **Cold starts.** Each chunk re-pays the compulsory misses for whatever working set it
   touches first.

One partially offsetting term exists and is **unmeasured**: in a single process, a state
evicted early could be re-requested much later (a *rerender*), adding misses the chunked
sum does not contain in that form. §1.5 bounds how large the correction can be.

### 1.5 Bounding the single-process distinct-state count D

The logs support an exact per-chunk statement and a bounded whole-film statement:

- **`rerenders=0` in every chunk of both runs.** In `PaintBudget`
  (`src/paintbudget.rs`), `on_miss` increments `rerenders` iff the missed key is in
  `evicted_keys`, and a key can only leave the cache by eviction
  (`PaintCache::get_or_paint`, `src/film.rs:2421-2468`). So no miss in any chunk was a
  re-paint of a previously evicted key — **every miss was the first touch of a distinct
  state within that chunk process. Per-chunk misses = per-chunk distinct states touched,
  exactly** (357, 562, 419, 441, 326, 147, 308, 52).
- Therefore the whole film's distinct-state count **D** (distinct `state_key` values,
  `src/film.rs:2457`) satisfies:

  **562 ≤ D ≤ 2,612**

  (lower bound = largest single-chunk distinct count, chunk 250..500; upper bound = the
  sum over chunks, reached iff no state recurs across chunks).
- The telemetry sample corroborates the lower region independently: in one process, over
  the 242 sampled times, **386 distinct states** occur among 482 sampled
  character-instances (386 misses, `rerenders=0`, same argument) — i.e. **80.1 % of sampled
  character-instances are already distinct states** at 0.25 s sampling density.
- A single-process run's miss count M₁ satisfies **D ≤ M₁**; M₁ = D + (single-process
  rerenders). The rerender term is unmeasured for a full single-process run, but the
  observed evidence constrains it: re-touch-after-eviction never happened inside any
  250-frame window, so a rerender requires a state to recur after eviction *and* across a
  chunk-scale gap. **2,612 is an upper bound on D, not a proven upper bound on M₁**; the
  honest statement is that the chunked sum overstates D by a factor 2,612/D ∈ [1.00, 4.65],
  and M₁ is expected near D, not near 2,612 — Phase 1 can measure M₁ directly with one
  unchunked run's stats line.

---

## 2. Card economics

### 2.1 One painted card

- Canvas: **2560 × 2816 RGBA8** = **28,835,840 bytes = 27.5 MiB exactly**.
  Source: `src/paintbudget.rs:38-40` (`CANVAS_BYTES = 2560 * 2816 * 4`, matching
  `paint_toon_full`'s canvas in `toonfull.rs`); cache-side note at `src/film.rs:2387-2396`.
- Cross-check from the logs: every stats line in both runs is an exact multiple —
  2,133,852,160 / 74 = 1,499,463,680 / 52 = 692,060,160 / 24 = **28,835,840 B/entry**.
  There is no size variance: every character, look, and state paints onto the same canvas.

### 2.2 The 2 GiB budget → resident ceiling

- Default byte budget: `BUDGET_CEILING_BYTES = 2048 * 1024 * 1024 = 2,147,483,648 B`
  (`src/paintbudget.rs:50-64`), derived as min(entry_cap × canvas, 2 GiB). The nominal
  entry cap is 160 (`DEFAULT_ENTRY_CAP`, `src/paintbudget.rs:46-48`) — 160 cards would be
  ≈ 4.6 GB, so **the byte budget binds first; the entry cap never binds in the census**
  (0 cap evictions in all chunks).
- Resident ceiling: ⌊2,147,483,648 / 28,835,840⌋ = ⌊74.47⌋ = **74 cards**
  (74 × 28,835,840 = 2,133,852,160 B ≤ budget; 75 cards = 2,162,688,000 B > budget).
  The logs show exactly this dynamics: every full chunk's `peak_bytes=2162688000`
  (75 cards, transiently, at the insert that triggers eviction) settling to
  `entries=74 bytes=2133852160`. Eviction-reason precedence is entry-cap first
  (`src/paintbudget.rs` `eviction_reason`), but with 74 < 160 only the byte reason fires.

### 2.3 Coverage of the film's distinct states

| Denominator | Resident 74 cards cover |
|---|---|
| D = 562 (lower bound) | 74 / 562 = **13.2 %** |
| D = 2,612 (upper bound) | 74 / 2,612 = **2.8 %** |
| Telemetry sampled distinct = 386 | 74 / 386 = 19.2 % |

So the cache can hold **at most ~13 %, and possibly as little as ~3 %, of the film's
distinct painted states at once**. The 27.7 % hit rate is achieved despite this because
reuse in Profile A is *local*: held poses, blink plateaus and bit-exact walk-phase
recurrence recur within a short window (the `state_key` comment, `src/film.rs:2459-2468`,
notes walk-cycle phases recur bit-exactly under exact f32-bit keys). Holding the **entire**
film resident would need D × 28,835,840 B = **16.2 GB (D = 562) to 75.3 GB (D = 2,612)** —
not a machine that exists in this programme's envelope (~6.7 GiB usable RAM on the user's
PC, `src/paintbudget.rs:53-58`).

---

## 3. Profile A — state-space of the film (what actually varies)

Film data: `film/onoda.json` — duration 60.357 s, 30 fps, 1,811 frames. Staging:
`eval_frame` (`src/film.rs:2227-2247`) dispatches to 14 segment evaluators
`s01, s02, s03, s04, s05a, s05b, s06, s07, s08, s08b, s09, s10, s11, s12`
(`src/film.rs:1559-2245`), each building `FullState`s from `fs()` = `FullState::default()`
plus explicit writes, resolved through `resolve_chars` (`src/film.rs:2510`) into the
PaintCache under `state_key` (look index + every `FullState` field as f32 bit patterns +
held-plate names + sweat/tears, `src/film.rs:2457-2505`).

### 3.1 Looks (registry)

`look_registry()` (`src/film.rs:598-649`) defines **11 instances but only 6 distinct
looks**, because the key prefixes the *instance index*, not the look content:

| Index | Instance | Look (body / hat / extras) | Perspective | Blink seed |
|---|---|---|---|---|
| 0 | ONODA (jungle) | #5C6644 / cap #4C5638, shade #878E6E | yes | 3.0 |
| 1 | COMPA | #68744E / cap #525C3C | no | 7.0 |
| 2 | COMPB | #59613F / cap, scarf #8A7A52 | no | 11.0 |
| 3 | SUZUKI | #C96A42 / cap #E8B83A, glasses | no | 5.0 |
| 4 | TANI | #4A4560 / bowler #2E2A3A, vest, tie, glasses, mustache | no | 9.0 |
| 5 | SH_ONODA (shore) | = index 0's colours, no shade | no | 3.0 |
| 6, 7 | SH_COMPA / SH_COMPB | = indices 1 / 2 | no | 7.0 / 11.0 |
| 8, 9 | SH_TANI / BK_TANI | = index 4 | no | 9.0 |
| 10 | ONODA2 (dusk swap) | = index 0 (incl. shade + perspective) | yes | 3.0 |

All registry looks have `kid=false` and none of the CR-B3-06 extras
(blush/freckles/pencil/monocle/hair) — `fl()` at `src/film.rs:572-596` defaults them off,
so those painter inputs never vary in this film.

### 3.2 Segments: characters, and a cross-validation of §1

Segment starts are the shot table's (`src/film.rs:2227-2247`): line starts − 0.06 s, with
S05b at `thought4 − 0.12`, S07 at `decades6 − 0.25`, S08b at `refused7 − 0.1`
(anchors from `film/onoda.json`). Frames are `t = n/30` (`src/film.rs:3891`).

| Segment | Time (s) | Frames | Chars/frame | Paint requests (frames × chars) |
|---|---|---|---|---|
| s01 | 0.000–5.907 | 178 | 1 (Onoda) | 178 |
| s02 | 5.907–13.061 | 214 | 3 (Onoda + 2 companions, shore) | 642 |
| s03 | 13.061–15.950 | 87 | 4 (Tani + Onoda + 2 companions) | 348 |
| s04 | 15.950–21.384 | 163 | 3 | 489 |
| s05a | 21.384–24.560 | 95 | **0** (plane + leaflets only) | **0** |
| s05b | 24.560–26.129 | 47 | 1 | 47 |
| s06 | 26.129–30.390 | 128 | 3 | 384 |
| s07 | 30.390–34.209 | 115 | 1 | 115 |
| s08 | 34.209–40.360 | 184 | 2 | 368 |
| s08b | 40.360–41.732 | 41 | 2 | 82 |
| s09 | 41.732–45.365 | 109 | 2 | 218 |
| s10 | 45.365–50.648 | 159 | 1 (Tani, bookshop) | 159 |
| s11 | 50.648–55.520 | 146 | 2 | 292 |
| s12 | 55.520–60.357 | 145 | 2 | 290 |
| **Total** | | **1,811** | | **3,612** |

**The staging-derived request total (3,612) equals the measured log total (3,612)
exactly**, and the frame total equals the census verdict (1,811). The code read and the
log agree — the per-chunk request variation in §1.2 (361–837 per 250 frames) is explained
by characters-per-frame varying 0–4 across segments, not by any hidden paint source.

### 3.3 Moods

`Mood` has 19 variants (`src/toonfull.rs:36-56`). Written in the film staging
(`grep "Mood::" src/film.rs`, staging lines only): **Determined** (s01, s02, s03 ×2, s08,
s08b, s09, s10 late, s11 Tani + Onoda early), **Neutral** (the `FullState::default()` mood
— every companion/background character that never gets an explicit mood, plus s07 Onoda
after 1.2 s), **Happy** (`pose_explain`, s04 CompB), **Laugh** (`pose_celebrate`, s04
CompA), **Sad** (`pose_mourn`, s06 Onoda + CompA), **Tired** (s06 CompB, s07 Onoda
< 1.2 s), **Suspicious** (s04, s05b Onoda), **Shock** (s08 Suzuki after the `found7`
anchor), **Worried** (s08b Suzuki), **Awe** (s10 Tani between `bookseller9` and `flew9`;
s11 Onoda around the rifle drop), **Relieved** (s11 Onoda2 after drop + 0.85 s; s12 Onoda
starts Relieved per its clip data, stepping to Neutral at lt 2.6 —
`scenes/film/s12.scene.json` mood step keys `[[0,'relieved'],[2.6,'neutral']]`, consumed
via the clip fold in s12, `src/film.rs:2161-2205`).

**11 of 19 moods occur** (Neutral, Happy, Laugh, Sad, Tired, Determined, Suspicious,
Shock, Worried, Awe, Relieved). **Never in the film:** Smug, Angry, Sly, Proud, Dizzy,
Embarrassed, Puzzled, Deadpan.

### 3.4 Legs

`Legs` has 5 variants (`src/toonfull.rs:122-128`). Staging writes
(`grep "Legs::" src/film.rs`):

- **Stand** — the default (`FullState::default()`), and the explicit tail state of every
  walk (`wk >= 1.0` / `k > 0.97` conditions at `src/film.rs:1591, 1814, 1901, 1940, 1961,
  2054, 2126`). The large majority of character-frames.
- **Walk** — s02 (all 3 chars), s06 (2 companions), s07 (Onoda), s08 (Suzuki + Onoda),
  s10 (Tani, only while `0 < wk < 1`), s11 (Onoda walk-in). Walk carries the continuously
  varying `phase`/`bob` from `gait_eval` — the main source of distinct states inside a
  segment.
- **Sit** — exactly one assignment in the film: s06 Onoda at `lt >= 3.55`
  (`src/film.rs:1839`), with `sit_drop = Some(0.34 × 1024 / (1.75/0.96))` and a settle
  `squash` sine over 0.3 s.
- **Run, Jump — never assigned anywhere in the film staging** (they occur only in the
  `state_key` string mapping and in scene-file readers).

### 3.5 Held plates (holds)

Plates are named in `film/assets/plates.json` (5 plates: `paper-map`, `leaflet`,
`book-open`, `book-closed`, `coconut`). Hold assignments in staging
(`src/film.rs:1783, 1919, 1954, 1999, 2034, 2071`) — the hold name is part of the paint
key, so each hold change is a new distinct state for that character:

| Plate | Where | Condition |
|---|---|---|
| `leaflet` (hold_l) | s05b Onoda | while `t < 26.150` (the drop) |
| `coconut` (hold_l) | s07 Onoda | from `t >= w_coco − 0.05` to segment end |
| `paper-map` (hold_l) | s08, s08b, s09 Suzuki | whole segments |
| `book-open` → `book-closed` (hold_r) | s10 Tani | switches at `t >= w_flew` |

All other character-frames hold nothing. (The rifle is **not** a plate hold: it is a
`rifle: bool` on `CharDraw` drawn as scene geometry; it does not enter the paint key
except through the arm pose that carries it.)

### 3.6 Sweat / tears — VERIFIED at HEAD `bc12a5c`

The state_key fix record says nothing in the film path sets `sweat`/`tears`. Verified by
exhaustive grep of `src/` for field writes (`.sweat =`, `.tears =`) and field
initialisations (`sweat:`, `tears:`):

**Writes that set them true exist ONLY in test modules:**

- `src/charstate.rs:875, 878, 972, 976, 983` — all inside `#[cfg(test)] mod tests`
  (module starts `src/charstate.rs:777`).
- `src/film.rs:4290, 4302` — the `state_key` regression tests, inside `#[cfg(test)]`
  (module starts `src/film.rs:4268`).

**Every non-test occurrence is a default, a copy, or a JSON-spec reader:**

- `src/toonfull.rs:198-199` — `FullState::default()`: `sweat: false, tears: false`.
  Every film-staging state starts from `fs()` = this default (`src/film.rs:1456`),
  and **no staging function s01–s12 ever writes either field** (no occurrence in
  `src/film.rs` outside the tests and the key/copy sites below).
- `src/film.rs:2526-2527` — `resolve_chars` *copies* `c.state.sweat` / `c.state.tears`
  into the painter state; in the film path the copied value is always the default `false`.
- `src/scenefile.rs:727-728` — the scene-file JSON reader:
  `sweat: truthy(st_spec.get("sweat"))`, `tears: truthy(st_spec.get("tears"))`
  (generic scene path, Profile B's reader — §5).
- `src/filmtelemetry.rs:648-649` — the same reader pattern in the *generic* telemetry
  path's scene-spec walk (not the legacy film telemetry, which uses `eval_frame`).
- `src/charstate.rs:327, 330, 380-404` — `CharStateKey` struct fields and its
  `from_state` constructor *reading* `FullState` fields; no `FullState` is written.

**Conclusion: in the film path at `bc12a5c`, `st.sweat` and `st.tears` are `false` for
every one of the 3,612 character-paint requests.** The two key fields added by the fix
are constant in this film — they contribute **zero** distinct states and zero misses.
Sweat/tear *pixels* still appear, but only through the mood side-channels in the painter:
`src/toonfull.rs:1326` (`if st.sweat || m == Mood::Worried` — sweat drop; fires for s08b
Suzuki) and `src/toonfull.rs:1339` (`if st.tears || m == Mood::Sad` — tear ellipses; fires
for s06's Sad characters). (The stale comments at `src/film.rs:2474-2476` cite
toonfull.rs:775/788; the code at HEAD is at 1326/1339.)

### 3.7 Talk and blink

- **Talk** (`st.talk`, a continuous f32 from `talk(t, on)`, `src/film.rs:380-385`) is
  written in exactly **two** places: s03 Tani (`src/film.rs:1652`, on while
  `lt < w_surr − st0 + 0.35`) and s11 Tani (`src/film.rs:2153`, on for
  `w_order − 0.1 < t < w_order + 0.75`). Every other character-frame has `talk = 0`.
- **Blink** is written for **every** character in every segment (28 `.blink =` sites in
  `src/film.rs`): either `charanim::blink_amount(seed, t)` with the per-instance seeds of
  §3.1, or `blink_of(seed, t, &[anchor])` which max-combines the periodic blink with a
  scripted blink at a word anchor (s01, s03, s04, s07 via anchors, s08, s09, s10, s12).
  Blink is a stepped channel (0 outside blink events) — long runs of frames share
  `blink = 0` bit-exactly, which is part of why held poses hit the cache at all.
- The remaining per-frame variation is continuous pose machinery: `pose_track` arm/head
  tracks, `gait_eval` phase/bob, sinusoidal celebrate bob (s04), `settle` head-turn
  overshoots (s05b, s08b), and — for the two perspective instances only (indices 0, 10) —
  `cam_pitch` recomputed from the moving camera in `finish_chars`
  (`src/film.rs:1516-1544`). Non-perspective instances keep `cam_pitch = 0` unless staging
  sets it (s01, s07 set it explicitly).

---

## 4. Paint cost context (bound only — PERF measures paint directly)

The census chunk elapsed times are **whole-pipeline wall times** on the CI runner
(ubuntu-22.04, Vulkan forced to the lavapipe ICD `lvp_icd.x86_64.json` —
`.github/workflows/census.yml`, "Install mold + lavapipe" step): scene evaluation, CPU
paint, wgpu/lavapipe scene render, PNG encode and write, for that chunk. Elapsed is
printed at whole-second resolution (`elapsed MM:SS`).

| Chunk | Frames | Run 43867793062 | ms/frame | Run 43808791603 | ms/frame |
|---|---|---|---|---|---|
| 0..250 | 250 | 07:44 (464 s) | 1,856 | 07:38 (458 s) | 1,832 |
| 250..500 | 250 | 10:07 (607 s) | 2,428 | 09:29 (569 s) | 2,276 |
| 500..750 | 250 | 08:20 (500 s) | 2,000 | 08:04 (484 s) | 1,936 |
| 750..1000 | 250 | 08:59 (539 s) | 2,156 | 08:43 (523 s) | 2,092 |
| 1000..1250 | 250 | 07:16 (436 s) | 1,744 | 07:15 (435 s) | 1,740 |
| 1250..1500 | 250 | 04:38 (278 s) | 1,112 | 05:07 (307 s) | 1,228 |
| 1500..1750 | 250 | 07:22 (442 s) | 1,768 | 07:24 (444 s) | 1,776 |
| 1750..1811 | 61 | 01:25 (85 s) | 1,393 | 01:29 (89 s) | 1,459 |
| **Total / mean** | **1,811** | **3,351 s (55:51)** | **1,850 ms/frame** | **3,309 s (55:09)** | **1,827 ms/frame** |

What this does and does not say about paint:

- **No per-paint timing exists in these logs**, so the paint share of the 1,850 ms/frame
  cannot be extracted from them — any percentage quoted from this data alone would be
  invented. The PERF seat measures `paint_toon_full` directly; this seat supplies only
  the demand side (2,612 paints per film, §1) and the pipeline envelope.
- The only hard bound available: with 1.442 paints/frame inside a 1,850 ms/frame
  pipeline, the mean cost per paint is **≤ 1,850 / 1.442 ≈ 1,283 ms** — the absurd
  ceiling obtained by attributing *all* pipeline time to paint. The true figure is
  strictly below it by everything else the pipeline does (the s05a segment renders 95
  frames with **zero** paint requests and its chunk, 500..750, still costs
  2,000 ms/frame — direct evidence that the non-paint pipeline is the dominant term at
  lavapipe speeds; per-segment elapsed is not printed, so s05a's own rate cannot be
  isolated further).
- Chunk-to-chunk spread (1,112–2,428 ms/frame) tracks scene/render weight (character
  count, segment content), not paint count alone: chunk 1250..1500 has the *fewest*
  misses (147) and the lowest ms/frame, while chunk 250..500 has the most misses (562)
  and the highest ms/frame — correlation consistent with paint being *a* cost, at a
  weight this data cannot separate from the simultaneous change in characters/frame
  (3.35 vs 1.44 requests/frame).

---

## 5. Profile B — per-frame-sampled poses (MODEL, not a measurement)

> Everything in this section is a model built from Profile A's measured constants
> (card size 28,835,840 B, resident ceiling 74, budget 2 GiB, entry cap 160) plus the
> code paths cited. **No sampled project has been measured.** Phase 1 should replace
> the assumed parameters (F, chars/frame, u, S) with a real project's numbers using
> the formulas in §5.4.

### 5.1 How a sampled project reaches the painter

1. **Driver:** `filmseq::run_film_seq` — each segment's scene file is parsed **once**;
   per frame, the parsed root is **cloned**, its `time` is set to the f32 nearest the
   segment-local `lt` (`src/filmseq.rs:20-23` module law; `set_root_time`,
   `src/filmseq.rs:422-433`; per-frame clone + set + load at `src/filmseq.rs:577-589`),
   then evaluated through `scenefile::load_root` + `render_loaded_pixels`.
2. **Fold:** `load_root` first runs `eval15::process_scene(root, &[])`
   (`src/scenefile.rs:959-969`) — top-level `clips` are sampled at the file's `time`
   and folded into node state. Node-level animation is **not** available: a `clip` or
   `clipTime` field on a node is rejected outright in `walk_nodes`
   (`src/scenefile.rs:571-575`: `"clips extension not supported"`). Staged character
   state in the JSON is otherwise static — so acting beats and continuous motion can
   only enter as **clip tracks whose keys are baked per-frame pose samples**: at each
   frame's `lt`, the sampled/interpolated channel values differ, and the folded state
   is (for any moving character) a new state.
3. **Paint:** `walk_nodes`' character branch builds a `FullState` from the folded node's
   `state` object (`src/scenefile.rs:719-754`) and calls
   `paint_toon_full(&look, &state)` **unconditionally** (`src/scenefile.rs:752`).

### 5.2 Which state channels the sampled poses exercise

Clip-animatable character channels (`eval15::channel_kind`,
`src/eval15.rs:1263-1269`), and the `FullState` field each folds into
(`src/scenefile.rs:719-754`):

| Channel(s) | Kind | FullState field | Per-frame effect |
|---|---|---|---|
| `armL.0, armL.1, armR.0, armR.1` | scalar | `arm_l`, `arm_r` | new key per distinct value |
| `look.0, look.1` | scalar | `look` (gaze) | ″ |
| `lean, bob, squash` | scalar | `lean`, `bob`, `squash` | ″ |
| `phase` | scalar | `phase` (gait) | ″ |
| `headTilt, headTurn` | scalar | `head_tilt`, `head_turn` | ″ |
| `blink, talk` | scalar | `blink`, `talk` | ″ |
| `camPitch` | scalar | `cam_pitch` | ″ |
| `sitOn` | scalar | `sit_drop` | ″ |
| `mood, legs` | step | `mood`, `legs` | new key per step change |
| `height` | scalar | — (node height, card scale) | geometry, not paint content |

**Not clip-animatable:** `sweat` and `tears` have no clip channel — in the scene path
they remain static JSON booleans read by the loader (`src/scenefile.rs:727-728`), exactly
as inert as in Profile A (§3.6) unless a scene file sets them. Held plates (`holdL` /
`holdR`) are likewise static state fields, not channels.

Because the paint key is the **exact f32 bit pattern** of every scalar field
(`src/film.rs:2457-2505` states the law for the film path; the same exactness is what
makes a cache a pure optimisation), a pose sampled at 30 fps from any continuous track
(poseTrack-style interpolation over per-frame keys) yields a **bit-distinct FullState on
essentially every frame for every moving character**. There is no quantisation step
anywhere in the path that would collapse neighbouring samples into one state.

### 5.3 The as-built fact: the generic path has no cache at all

Profile B's paint count today is **not** a cache question. The scene path owns **no
PaintCache**: `walk_nodes` paints every character node on every frame
(`src/scenefile.rs:752`), and `filmseq`'s completion line deliberately omits the
paint-cache segment for exactly this reason (`src/filmseq.rs:613-620`). The generic
telemetry path is the same (`src/filmtelemetry.rs:53-55`: "the generic path paints
uncached"). Therefore, **as built**:

- **CPU paints executed = R** (every character in every frame), **hit rate = 0 %**,
  regardless of how repetitive the poses are. Even a perfectly static sampled character
  is repainted 30 times per second.
- At Profile A's scale (R = 3,612) that is **3,612 paints vs Profile A's 2,612 chunked
  misses — ×1.38** — and vs a single-process Profile A run (misses between D and 2,612)
  the factor is ≥ 1.38.

### 5.4 The cached counterfactual — formulas and scenarios

If a PaintCache with the current budget (2 GiB / cap 160 / **K = 74 resident cards**,
§2) were wired into the generic path, the governing quantities are:

- `F` = frames; `c(f)` = characters in frame f; **R = Σ c(f)** (total requests).
- `D` = distinct FullStates touched; `u` = fraction of character-frames whose state
  never recurs; `S` = distinct states in the recurring pool.
  **D ≈ u·R + (1−u)·S**, with **D ≤ R** always.
- Paints executed **M = D + rerenders**, hence **D ≤ M ≤ R**; **hit rate h = 1 − M/R**.
- **Thrash regime:** when a recurring state's reuse distance exceeds **K = 74** distinct
  intervening states, LRU has already evicted it, so its recurrence is a rerender too.
  As the working set grows past K, **M → R and h → 0** — the cache degenerates to a
  74-card delay line. (Profile A escapes this only because its recurrence is local:
  reuse distance is short even though D ≫ K.)
- Residency required to hold everything: **D × 28,835,840 B**. For D = 3,612 that is
  **104,155,054,080 B ≈ 97.0 GiB** — ~49× the budget. K/D at D = 3,612 is **2.0 %**.

Scenario table at Profile A scale (F = 1,811; 1–2 sampled characters/frame, so
R ∈ [1,811, 3,622]; reference R = 3,612 as measured in §3.2; assumed recurring pool
S = 150 distinct states — an assumption, stated, of the order of Profile A's
static-pose pool; K = 74):

| Unique fraction u | D ≈ u·R + (1−u)·S | M (paints) range | Hit rate h range | K/D |
|---|---|---|---|---|
| 0 (fully staged, Profile-A-like) | ≈ 150–2,612 (measured §1.5) | 2,612 chunked (measured) | 27.7 % (measured) | 2.8–13.2 % |
| 0.25 | ≈ 1,016 | 1,016 – 3,612 | 0 – 71.9 % | 7.3 % |
| 0.50 | ≈ 1,881 | 1,881 – 3,612 | 0 – 47.9 % | 3.9 % |
| 0.75 | ≈ 2,747 | 2,747 – 3,612 | 0 – 24.0 % | 2.7 % |
| 1.00 (every pose unique) | = R = 3,612 | = 3,612 | **0 %** | 2.0 % |

Reading the table: the **order-of-magnitude growth** the profile predicts is in D —
from Profile A's hundreds-per-chunk (147–562 distinct per 250 frames; 386 distinct in
242 telemetry samples) to **thousands per film** (D → R ≈ 1.8–3.6 k at this scale, and
D scales linearly with F and chars/frame beyond it). M's *range* within a row is set by
reuse distance vs K: monotonic, never-revisited sampled motion sits at the low end
(M = D — the cache neither helps nor re-paints), while cyclic sampled motion (loops,
returns to earlier poses) with reuse distance > 74 sits at the high end (M → R, pure
thrash). **In no row does the current cache restore Profile A's economics** — at
u ≥ 0.5 the best possible hit rate is under 48 %, and the as-built path (§5.3) is at
0 % by construction.

**Phase 1 plug-in recipe:** for a real sampled project, measure (a) R from its manifest
(Σ segment frames × characters), (b) D and u by hashing the folded `FullState` per
character-frame over one pass (the `state_key` function is reusable verbatim), and
(c) the reuse-distance distribution of recurring states. Then M and h follow from the
formulas above with K = 74 — or with whatever resident ceiling a GPU-resident card
store would change K to, which is precisely the variable a GPU-first design moves:
on GPU, a card is a texture, and the binding constraint becomes VRAM, not the 2 GiB
CPU-RAM budget — but at 27.5 MiB/card, even D = 1,881 needs 54.2 GB of residency, so
**no residency policy makes a sampled-pose film cache-friendly; the paint itself has
to get cheaper.** That is the demand-side case for GPU paint.

---

## 6. What could not be obtained (and why)

- **Single-process full-film miss count M₁** — no unchunked full-film run exists in CI
  (the census is chunked by design, to bound disk). Bounded in §1.5 instead.
- **Per-paint CPU time / paint share of frame time** — the census logs contain no
  per-paint or per-frame timing lines (frame-timing is env-gated off by default,
  `src/film.rs:3873-3878`). Bounded in §4; direct measurement is the PERF seat's.
- **Per-segment elapsed time** — elapsed is printed per chunk only; s05a's zero-paint
  rate (§4) cannot be isolated from its chunk-mates.
- **Exact D for Profile A** — requires instrumenting one run to log distinct keys (or
  an unchunked run plus a rerender count). The [562, 2,612] bound and the 386-state
  telemetry sample are what the existing logs support.
- **Profile B measurements** — no sampled-pose project has been rendered through
  `filmseq` with paint accounting (the path has no cache to account with, §5.3).
  §5 is a model with stated assumptions, by design.

---

## Appendix — sources

- CI logs (verbatim stats lines):
  `~/workspace/talerender-evidence/gpufirst-phase0/freq-raw-stats.txt`
  (runs `43867793062`, `43808791603`; fetched 2026-11-16 via `gh run view --log`).
- Workflow: `.github/workflows/census.yml` (telemetry invocation :53, chunk loop :59-66).
- Cache: `src/paintbudget.rs` (canvas :38-40, cap :46-48, ceiling :50-64, stats :299);
  `src/film.rs` PaintCache :2397-2468, `state_key` :2457-2505, `resolve_chars` :2510-2545.
- Film staging: `src/film.rs` registry :598-649, poses :1456-1494, segments :1559-2245,
  shot table :2227-2247, telemetry :4105-4180; data `film/onoda.json`,
  `film/assets/plates.json`, `scenes/film/s12.scene.json` (mood clip keys).
- Painter: `src/toonfull.rs` Mood :36-56, Legs :122-128, defaults :198-199,
  sweat :1326, tears :1339.
- Generic path: `src/filmseq.rs` :20-23, :422-433, :577-589, :613-620;
  `src/scenefile.rs` character state :712-752, clips guard :571-575, fold :959-969,
  sweat/tears read :727-728; `src/eval15.rs` character channels :1263-1269;
  `src/filmtelemetry.rs` :53-55, :648-649; `src/charstate.rs` tests :777-778.
