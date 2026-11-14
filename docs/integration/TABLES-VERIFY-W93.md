# TABLES-VERIFY-W93 — G7 `--tables` implementation diff-check

**Seat:** Wave 2 W93 (Batch 10, final). **Scope:** verify the lead's
`--tables` landing at `3223509` ("G7 tables in the binary
(TABLES-IMPL-W85)") against Batch 9's spec
[TABLES-IMPL-W85.md](TABLES-IMPL-W85.md) — the verification W90's
gate explicitly left owed. This seat writes no code, edits no
existing file, commits nothing, and ran no local renders.

**Method:** the `3223509` diff read first-hand (main.rs, film.rs,
telemetry.rs, +499/−2); sole-caller and return-path greps against
the tree at `3223509`; CI conclusions and logs via `gh`; the seam-4
arithmetic reproduced in IEEE-754 doubles (Python floats, same
semantics as Rust `f64`). CI evidence runs through HEAD `2f5c708`
(the lead's follow-up fix, §A.5).

## Verdict summary

| Spec area | Verdict |
|---|---|
| §1.0–1.2 data seams (printer home, shot table, caption re-print) | **MATCHES** |
| §1.3 cast list / presence / beats | **MATCHES** — two disclosed interpretation points (§C) |
| §2 flag plumbing + print order | **MATCHES** |
| §2.5 exit codes | **MATCHES** — provably unmoved (§D) |
| §4 formats + §4.4 disclosed deltas (all five) | **MATCHES** (§E) |
| §6.1 unit tests, seams 1, 2, 3, 5 | **MATCHES** — seam 2's fixture reduced (§F) |
| §6.1 seam 4, beat-frame test | **DEVIATES — the test is wrong, the implementation is right** (§A, first-class) |
| §3 byte-stability proof (steps 0–5) | **UNVERIFIED** — not executed/recorded; now executable (§F) |
| §5 / §6.2 acceptance numbers | **UNVERIFIED** — not observed; now executable (§F) |
| §7 landing checklist steps 1–3 | **MATCHES** |
| §7 steps 4–5 | **DEVIATES** (partial — §G) |

---

## §A FIRST-CLASS: the seam-4 CI failure — which side deviates

### A.1 Evidence chain

1. The landing commit's own CI never judged it: CI run
   `43629625139` at `3223509` was **cancelled** (concurrency
   supersession — the Batch-9 commit `9380b01` pushed ~2 minutes
   later). Tools CI `43629625142` at `3223509`: **success**.
2. The first CI verdict on the tables code is run `43629817118`
   at `9380b01`, whose `src/` is byte-identical to `3223509`
   (`git diff 3223509 9380b01 -- src/` is empty). Build OK; the
   Test step reports **179 passed / 1 failed**:
   `telemetry::tests::tables_beat_frames_tie_to_blink_at`,
   panicked at `src/telemetry.rs:2062` with
   `assertion left == right failed — left: 0, right: 3`.
3. Line 2062 is the test's **first** assert,
   `assert_eq!(via_blink, want)` — the recount through
   `film::blink_at` itself. The new counter is never reached.

### A.2 The three parties

`blink_at` (film.rs at `3223509`) — the §1 window, verbatim:

```rust
/// blinkAt(t, beats), transcribed: 1 while 0 <= t - beat < 0.12.
pub(crate) fn blink_at(t: f64, beats: &[f64]) -> f64 {
    if beats.iter().any(|b| t - b >= 0.0 && t - b < 0.12) { 1.0 } else { 0.0 }
}
```

`beat_frames` (telemetry.rs:1231–1238 at `3223509`) — the new
counter:

```rust
pub(crate) fn beat_frames(beat: f64, total: u32) -> u32 {
    (0..total)
        .filter(|&n| {
            let d = n as f64 / 30.0 - beat;
            (0.0..0.12).contains(&d)
        })
        .count() as u32
}
```

The predicate is identical: `0.0 <= n/30 − beat < 0.12`.

The test (telemetry.rs:2057–2065 at `3223509`):

```rust
for (beat, want) in [(4.20, 4u32), (14.78, 3u32)] {
    let via_blink = (0..200u32)
        .filter(|&n| crate::film::blink_at(n as f64 / 30.0, &[beat]) == 1.0)
        .count() as u32;
    assert_eq!(via_blink, want);            // line 2062 — panics here
    assert_eq!(beat_frames(beat, 200), want);
}
```

### A.3 The arithmetic

- **Beat 4.20:** `0 <= n/30 − 4.20 < 0.12` ⇔ `n/30 ∈ [4.20, 4.32)`
  ⇔ `n ∈ [126, 129.6)` → n = 126, 127, 128, 129 → **4**. Both
  counters, inside the 0..200 window. ✓
- **Beat 14.78:** `n/30 ∈ [14.78, 14.90)` ⇔ `n ∈ [443.4, 447)`
  → n = 444, 445, 446 → **3** — but the window `0..200` ends at
  t = 199/30 ≈ **6.63 s**, long before the beat occurs. Direct
  evaluation over 0..200 finds no satisfying n (the window opens
  at frame 444: 444/30 − 14.78 = +0.0200…). Over 0..200 **both
  counters return 0** — hence `left: 0`.
- Over the full film range (0..1811) both counters return **3**
  for the 14.78 beat. That is where the constant comes from: 3 is
  §5's full-film expectation for this beat (never2, 3f), not a
  0..200 count.

### A.4 Root cause and verdict

Spec §6.1 item 4 prescribes the construction —
`#{n in 0..200 : blink_at(n/30.0, &[b]) == 1.0}` — and states the
results as "4 and 3 respectively". For the 14.78 beat that
sentence is internally impossible: over 0..200 the `blink_at`
count is 0, not 3. The landed test transcribed the spec's range
and constants literally, so it asserts 0 == 3 and panics on its
own recount.

**Verdict: the TEST deviates — inheriting an error in the spec
sentence it transcribes. The IMPLEMENTATION does not deviate.**
`beat_frames` is an exact transcription of the `blink_at` window
(film.rs:256–264 as cited by §1); the tie seam 4 exists to prove
— counter ≡ `blink_at` — in fact holds at every window: both 0
over 0..200, both 3 over 0..1811, both 4 for the 4.20 beat. No
production path is implicated: `beat_frames` feeds only the cast
list's printed `Nf` beat counts.

### A.5 Resolution (landed by the lead after this evidence was gathered)

`2f5c708` "Fix tables beat test range" — a **test-only** change:
the range 0..200 → 0..600 in both the recount and the counter
call (600 frames = 20 s, covering frame ~443); the implementation
is untouched, and the commit message names the same root cause
("that range stops at t = 6.63 s and cannot reach the second
beat"). CI `43630186620` at `2f5c708`: **success — 180 passed /
0 failed**; Tools CI `43630186614`: success. The fix's shape
independently confirms §A.4: widening the window makes the
original constants (4, 3) pass against the unmodified counter.

**Residual:** TABLES-IMPL-W85 §6.1 item 4 still reads
"0..200 … 4 and 3 respectively". A spec erratum is owed so the
next reader does not re-transcribe it; the fixed test's added
comment already records the correction in code.

---

## §B Check (1) — §2 plumbing: MATCHES

- **State:** `let mut tables = false;` declared in `main()`
  beside the `film_telemetry_dir` state.
- **Parse arm:** `"--tables" => { tables = true; }` immediately
  after the `--film-telemetry` arm; value-less; accepted anywhere
  and consulted only by the telemetry dispatch — §2.1's
  not-an-error-with-other-modes behaviour.
- **Signature + sole caller:** `run_film_telemetry` gains
  `tables: bool` (`Result<bool, String>` unchanged);
  `git grep` at `3223509` shows exactly one call site,
  main.rs:410 — `film::run_film_telemetry(std::path::Path::new(dir), tables)`.
- **HELP:** one entry under `--film-telemetry`, wording verbatim
  against §2.3 ("With --film-telemetry: also print the shot,
  caption and cast tables after the verdict line. Default output
  is unchanged."). The description column aligns with the
  neighbouring entries rather than the spec block's narrower
  indent — cosmetic only. Usage synopsis and the exit-code
  entries are unchanged, as §2.3 requires.
- **Print order (§2.4):** the `if tables` block sits **after**
  the `for line in &out { println!() }` loop; the tables are
  built into a separate Vec (`tlines`, seeded with one blank
  line) by `print_film_tables` and printed from it. Nothing is
  pushed into `out` — the default output is a strict prefix by
  construction. Internal block order matches §4.6: SHOT TABLE,
  blank, CAPTION TABLE + rows + straddle notes, blank, CAST
  LIST + rows, unverified-beat notes last. Stderr progress and
  the paint-cache stats line are untouched by the diff.

## §C Check (3) — §1 data seams: MATCHES, two interpretation points

- **§1.0 printer home:** `print_film_tables` + the `FilmTables`
  input struct live in telemetry.rs beside `print_caption_audit`,
  consuming `(&[ShotInfo], duration, &[CaptionInfo],
  &[PresenceRow], &HashMap<String, f64>)` — the specced shape;
  `fixed2`/`js_num` used in-module, no visibility changes.
- **§1.1 shot table:** `let shots_keep = shots.clone();`
  immediately before `Audit::new(shots)` — the specced clone
  point. Ends are next shot's `at`, last = `FilmData.duration`.
  Frames come from `shot_frame_ranges`: first frame
  `ceil((at − 1e-9) * 30)`, shot 1 = 0, last = next first − 1,
  final shot to `total − 1`, with
  `total = (duration * 30.0).ceil() as u32` (film.rs:3517's own
  expression). One wording nuance: the code clamps each first
  frame to ≥ the previous shot's **first** (`raw.max(firsts[i-1])`)
  where §1.1 says "clamped to ≥ the previous shot's end"; under
  the tiling definition the two coincide for every
  non-decreasing `at` sequence — the only inputs the audit
  constructs — and the unit test pins contiguous tiling. No
  observable difference.
- **§1.1 / D4 dressing:** `SHOT_DRESSING` const, 14 rows, shot
  ids verbatim, header comment naming the staging functions as
  the source of truth. The comment cites the staging order by
  function name, not the per-shot film.rs line numbers §1.1
  enumerated — cite granularity reduced; the values themselves
  are not re-derivable from the diff alone.
- **§1.2 caption table:** a pure re-print of the `caps` vec —
  stored `rendered_w` / `raw_w` / `fit` / `text`, **no
  re-measurement** (the structural-parity requirement). Line
  printed as `c.line + 1` (0-based storage, §1.2). Owner by
  `shot_at(shots, c.s)` — start-ownership. Flag predicates are
  the literals `rendered_w > 850.0` / `fit < 0.68`, duplicated
  with a cross-comment to `print_caption_audit` and deliberately
  **not hoisted**, per the §1.2 verdict-path freeze. Straddle
  notes derive from the same values; the note template is
  byte-identical to §4.3's quoted chunk-41 line.
- **§1.3 presence accumulator:** lives in `run_film_telemetry`'s
  sample loop — one `note_presence` call per sample, after
  `audit.sample(t, &cuts)`, attributed by
  `shot_at(&shots_keep, t)`, ids taken from the `cuts` vec the
  loop already built, **with multiplicity**. `PresenceRow`
  fields (`id`, `shot`, `samples`, `max_simul`, `first`, `last`)
  exactly as specced; the update rule gives each distinct id
  +1 sample per sample and tracks the max simultaneous count.
  The shot table's characters cell reads the rows in
  first-sampled order, prints `x{n}` when `max_simul > 1`, and
  `(none)` when a shot has no row.
- **§1.3 beats:** `BEAT_BINDINGS` const — all seven
  (anchor, identity, shot) triples match §1.3's table
  (years0/onoda/S01, never2/onoda/S03, message3/onoda/S04,
  found7/suzuki/S08, commander8/onoda/S09, bookseller9/tani/S10,
  loyal11/onoda/S12). Beat-frame counts are computed by
  `beat_frames`, never hardcoded. D7 verification is implemented
  as specced: anchor presence via `.get()`, beat time inside
  the shot's `[at, end)` window, identity sampled in that shot
  with `samples > 0`; each failure appends a `note:` line after
  the cast list naming the failed check.
- **Interpretation point 1 (disclosed in the landing commit
  message):** cast frames are `(total_samples * 15 + 1) / 2`
  computed over the identity's **total** samples across all its
  rows — one rounding ("totals-first"). §1.3's formula admits a
  per-row reading; the commit message declares the totals-first
  reading explicitly. §5 fixes no cast-frames expectation (the
  first run's values become the baseline), so nothing checkable
  turns on the choice; the difference is a few frames at most.
- **Interpretation point 2 (wording-level, failure path only):**
  an unverified beat prints as `anchor (unverified)` — the
  suffix §1.3 requires — but without the `@ t (shot, Nf)` body
  a verified entry carries. For a missing anchor no time exists
  to print, so the short form is the only uniform option; D7's
  functional requirements (suffix present, note line naming the
  check, row never suppressed, no exit-code effect) all hold.
  The path never triggers on data whose bindings all verify.

## §D Check (2) — exit codes: MATCHES, provably unmoved

- No changed line in the `3223509` diff touches the return
  `Ok(bad == 0)` (film.rs:3786 at `3223509`), main's mapping
  (`Ok(true)` → 0 / `Ok(false)` → exit 1 / `Err` → exit 3), or
  the HELP exit-code entries — a grep of the diff's `+`/`-`
  lines for those patterns is empty; the hunks show them only
  as unchanged context.
- By construction (§2.5): the tables block runs after the
  verdict bool is determined and the legacy block printed; it
  formats values already in hand; anchor access is `.get()`;
  other lookups resolve through `.unwrap_or` fallbacks; indexing
  is over the audit's own vecs. It introduces no `?` into
  `run_film_telemetry` and cannot alter the returned bool. A
  flagged caption, a straddle, an unverified beat are printed
  data, never verdict inputs.

## §E Check (4) — §4 formats and the §4.4 disclosed deltas: MATCHES

All five disclosed deltas are present as specced:

1. **Shot ids verbatim** — `S05a` / `S08b` flow from
   `ShotInfo.id` into the shot table, the caption table's shot
   column, straddle notes, and cast shot lists; no case
   transform exists anywhere in the new code.
2. **Frame ranges by audit attribution** — S01 `0-176`, S02
   from 177, S01 time `0.21-5.88` (its `at`, not 0.00): the
   tiling function produces them and its unit test asserts
   exactly these values.
3. **Cast frames = sampled presence** — the header is
   byte-identical to §4.5's prescribed wording:
   `CAST LIST — {n} identities (frame counts are sampled presence: 0.25 s audit samples x 7.5 frames per sample)`.
4. **Character order = first-sampled** — presence-vec order in
   the shot rows; cast identities ordered by earliest
   first-sample time, ties by vec order. Deterministic.
5. **No RECONCILIATION trailer** — `print_film_tables` ends
   with the beat notes; no summary lines.

Formats (§4.1 / §4.3 / §4.5): the SHOT TABLE and CAPTION TABLE
header strings, all column format strings, the 30-column
beats-line indent, the `beats: -` empty form, and the
straddle-note template match the spec's quoted forms.

## §F Checks (5) + (6) — CI evidence; what remains unproven

### CI of record

| Head | Run | Conclusion |
|---|---|---|
| `108f33f` (parent) | CI `43629313479` | success — pre-tables baseline |
| `3223509` | CI `43629625139` | **cancelled** (concurrency supersession, §A.1) |
| `3223509` | Tools CI `43629625142` | success |
| `9380b01` (src ≡ `3223509`) | CI `43629817118` | **failure** — 179/1, the seam-4 test (§A) |
| `2f5c708` (test-only fix) | CI `43630186620` | **success — 180 passed / 0 failed** |
| `2f5c708` | Tools CI `43630186614` | success |

### §6.1 seams as landed

All five tests exist in telemetry.rs's test module. Seams 1
(caption rows, flags, line conversion — including the strict-`>`
844 case), 3 (attribution tiling), and 5 (presence accumulator
semantics) match their spec items and pass. Seam 2
(`shot_at` ≡ `Audit::shot_id_at`) passes, but its fixture is a
reduced 3-shot vec (S01 @ 0.21, S06 @ 25.0, S07 @ 30.39) instead
of the specced real 14-shot construction; every named boundary
is still covered — first-shot default at t < at, ±1e-6 probes,
chunk 41's start (30.29 → S06) and cut (30.39 → S07) — so the
fixture's fidelity is reduced while its boundary coverage is
kept. Seam 4 is §A.

### UNVERIFIED — not observed by this seat, claimed nowhere here

- **§3 byte-stability sequence, steps 0–5** (determinism
  pre-check; default-output identity diff; strict-prefix
  property; FAILED direction on a widened-strip scratch copy;
  could-not-run direction incl. W62's strip-mismatch fixture;
  the W62 adapter grid re-run with `--tables`): not executed
  and not recorded as of this check. The landing commit message
  defers the results ("results land in the record"); the wave
  record contains no §3 results.
- **§5 / §6.2 acceptance numbers** — 73 caption rows, 0 flags,
  max rend 844 at chunk 50, min fit 1.00, exactly 13 straddle
  notes with chunk 41's note byte-equal to §4.3's line, cast
  order onoda/compA/compB/tani/suzuki with shots 12/4/4/4/3 and
  beats 5/0/0/1/1 (25 beat-frames, none unverified), and the
  cast-frames actuals: all remain expectations, not results.
- **What it takes now:** both binaries exist as CI artifacts —
  the pre-change binary in green run `43629313479` (`108f33f`)
  and the post-change binary in green run `43630186620`
  (`2f5c708`), each carrying a `talerender-binary` artifact
  (`2f5c708` differs from `3223509` only in the test file, so
  its binary is the landing's binary for output purposes).
  Executing §3 + §6.2 is: download both artifacts, run
  `talerender --film-telemetry film` and
  `talerender --film-telemetry film --tables` from the repo
  root, perform §3's diffs, and check the `--tables` output
  against §5. This seat ran none of it (no local renders), so
  no §5 number is reported here as observed.

## §G Check (7) — §7 landing checklist

- **Steps 1–3** (telemetry.rs: `shot_at`, `PresenceRow`,
  `print_film_tables`, both const tables, §6.1 tests in the
  same commit; film.rs: clone, accumulator, signature, tables
  block; main.rs: state, arm, dispatch, HELP line): landed as
  specced in `3223509`. **MATCHES.**
- **Step 4** (execute §3 steps 0–5 and §6.2 on the CI binaries;
  record the results and the cast-frames actuals): not done as
  of this check. **DEVIATES — open** (see §F).
- **Step 5** (commit message names all five §4.4 deltas, the D2
  measure change, and the §3/§6.2 verdicts): the message names
  deltas 1, 2, 3 and the D2 measure, but not delta 4
  (first-sampled order) or delta 5 (no RECONCILIATION lines),
  and the §3/§6.2 verdicts are deferred rather than stated.
  **DEVIATES — partial.**

## Bottom line

The implementation at `3223509` is a faithful landing of
TABLES-IMPL-W85 §§1, 2, and 4: the plumbing, the data seams, the
formats, all five disclosed deltas, and the exit-code freeze
check out from the diff. The wave's one red CI verdict on this
code traced to the spec's own §6.1 seam-4 sentence — the test
inherited an impossible window/expectation pair, while the
counter it was meant to tie out is an exact transcription of
`blink_at`; the lead's test-only fix (`2f5c708`) has the suite
green at 180/0. Still owed: a §6.1 spec erratum, and the §3
byte-stability + §6.2 acceptance runs — both binaries for them
are now sitting in CI artifacts.
