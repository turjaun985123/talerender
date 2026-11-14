# TABLES-IMPL-W85 — G7 telemetry tables: binary-integration spec

**Seat:** Wave 2 W85 (Batch 9). **Scope:** the implementation spec for
building Batch 8's proven G7 design (TELEMETRY-TABLES-W75.md +
`tools/film/telemetry_tables.py`) into the `talerender` binary, for
the lead to execute. This seat writes no code and changes no existing
file; every cite below was verified against the source at the HEAD
named next, not copied from W75.

**Verified at:** HEAD `021ca01` (film.rs 3,761 lines, telemetry.rs
1,619 lines, main.rs 980 lines).

**The design being implemented (W75, settled):** an opt-in `--tables`
flag on `--film-telemetry`; the three tables (shot table, full
caption table, cast list) print to **stdout after the verdict line**;
default output byte-identical; exit codes unchanged (0 OK / 1 FAILED
/ 3 could-not-run); tables add no failure condition of their own.

---

## 0. Decisions this spec makes (and where each is argued)

| # | Decision | § |
|---|----------|---|
| D1 | The tables printer lives in `src/telemetry.rs`, beside `print_caption_audit`; `run_film_telemetry` (film.rs) supplies the inputs. No new module. | 1.0 |
| D2 | Cast presence = the audit's own 0.25 s **sampled presence**, accumulated in the telemetry sample loop — NOT the prototype's shot-granularity counts, and NOT new per-frame sampling. The cast header's honesty clause changes to say so (the one deliberate header delta). | 1.3 |
| D3 | Shot frame ranges derive from the telemetry `shots` vec by the frame-attribution rule (frame n belongs to the shot `shot_id_at(n/30)` names) — the audit's own boundaries, which can differ by 1–2 frames from the `scenes/film/film.json` manifest tiling the prototype printed. Total still tiles [0, 1811). | 1.1 |
| D4 | `set` / `atmosphere` columns are carried over, sourced from a per-shot const transcription in the printer (staging fn cited per shot), fenced exactly like the beat bindings — they have no data source in the telemetry path. | 1.1 |
| D5 | Shot ids print **verbatim** from the binary's `ShotInfo.id` (`S05a`, `S08b`), not the prototype's uppercased manifest ids (`S05A`, `S08B`) — the tables sit under audit sections that cite the same strings. | 4.4 |
| D6 | No RECONCILIATION trailer in the binary (the prototype's two lines are a development aid; W75 §7 already rules them out of the binary's behaviour). | 4.4 |
| D7 | A beat binding that fails verification prints a `note:` line; it never changes the exit code and never suppresses the cast row. | 1.3 |

---

## 1. Data-seam map

### 1.0 Where the printer lives

`print_caption_audit` (telemetry.rs:1037) is the house pattern: a
pure function `(&[CaptionInfo], …, &mut Vec<String>) -> usize` over
already-loaded data, unit-tested in telemetry.rs's own test module
(telemetry.rs:1437, with a `CaptionInfo` fixture test at
telemetry.rs:1599). The tables printer follows it:

```rust
// src/telemetry.rs, beside print_caption_audit (telemetry.rs:1037)
pub(crate) struct FilmTables<'a> {
    pub shots: &'a [ShotInfo],          // film.rs:3643–3658 vec
    pub duration: f64,                  // FilmData.duration
    pub caps: &'a [CaptionInfo],        // film.rs:3706–3735 vec
    pub presence: &'a [PresenceRow],    // accumulated in the sample loop, §1.3
    pub anchors: &'a HashMap<String, f64>, // FilmData.anchors
}
pub(crate) fn print_film_tables(t: &FilmTables, out: &mut Vec<String>)
```

Reasons: `ShotInfo` (telemetry.rs:512–515) and `CaptionInfo`
(telemetry.rs:518–527) are telemetry.rs types; the shot-attribution
rule is telemetry.rs's own (`Audit::shot_id_at`,
telemetry.rs:710–723); the `%.2f` formatter is telemetry.rs's private
`fixed2` (telemetry.rs:63) and integer-float formatting is `js_num`
(telemetry.rs:47) — both reachable with no visibility change from
inside the module; and the unit tests land in the existing test
module (telemetry.rs:1437). film.rs keeps what it already owns: the
loads, the sample loop, the caption-audit construction, and the
verdict.

### 1.1 Shot table

One row per shot, in film order. Column sources:

| Column | Source (all in hand during `run_film_telemetry`) |
|--------|---------------------------------------------------|
| shot | `ShotInfo.id` — the `shots` vec built at film.rs:3643–3658, passed to `Audit::new` at film.rs:3659. The vec must be **cloned** before that call (`ShotInfo` already derives `Clone`, telemetry.rs:511) or the Audit must expose it; cloning 14 two-field structs is the smaller diff. |
| time (s) | start = `ShotInfo.at`; end = next shot's `at`; last shot's end = `FilmData.duration` (film.rs:45, loaded at film.rs:3638). The `at` values are: line-start − 0.06 via the `l0` closure (film.rs:3642) for 11 shots, and anchor-based for three — S05b `anchors["thought4"] − 0.12` (film.rs:3649), S07 `anchors["decades6"] − 0.25` (film.rs:3651), S08b `anchors["refused7"] − 0.1` (film.rs:3653). |
| dur | end − start, `{:.2}`. |
| frames | **Derived, D3.** `FilmData` carries **no fps and no frame count** (struct at film.rs:44–54: duration, anchors, captions, lines, calendar fields only — the data file's `fps`/`frames` keys are deliberately not loaded; they survive in the file for the Python tools, F3-a-kept). The binary's own frame rule is in `run_film`: total = `(data.duration * 30.0).ceil()` (film.rs:3517) and frame *n* is staged at `t = n / 30.0` (film.rs:3534). The table's per-shot range is the attribution tiling: frame *n* belongs to the shot `shot_id_at(n/30.0)` names (rule at telemetry.rs:710–723: the last shot with `at <= t + 1e-9`, defaulting to the first shot before its `at`). Closed form: shot *i*'s first frame = `ceil((at_i − 1e-9) * 30)` clamped to ≥ the previous shot's end and, for shot 1, to 0; its last frame = next shot's first − 1; the last shot runs to total − 1. |
| set, atmosphere | **Transcribed, D4.** No source exists in the telemetry path: `FilmData` has neither field, and the staging functions (film.rs:1447–2003) apply atmospheres as numeric re-dressing (film.rs:406, 1240–1241), never by name; the names (`JG`/`SH`/`BK`, `noon`/`fog`/`golden`/`dusk`) are `scenes/film/film.json` vocabulary, a file the binary never loads (the telemetry entry receives only the film data dir, main.rs:391–406). The printer therefore carries a const table `SHOT_DRESSING: [(&str, &str, &str); 14]` = (shot id, set, atmosphere), one row per shot, each row's comment citing its staging fn: s01 film.rs:1447, s02 1465, s03 1509, s04 1544, s05a 1613, s05b 1623, s06 1658, s07 1737, s08 1784, s08b 1824, s09 1853, s10 1888, s11 1942, s12 2003. This is the same epistemic shape as the beat bindings (§1.3): a transcription of code knowledge, fenced by a comment saying the staging is the source of truth. Loading the manifest instead is rejected: it would couple a CPU gate to CWD/repo layout (bridge packages ship the film dir without `scenes/`), breaking W75 §1's one-load rule in the other direction. |
| characters | From the presence accumulator (§1.3): ids sampled in the shot, in first-sampled order, with ` x{n}` when the shot's max simultaneous instance count for the id is n > 1 (S11's Onoda swap). `(none)` when the accumulator has no id for the shot (S05a stages nobody — the sample loop will simply never record it). |

**D3's disclosed delta, worked example.** The prototype's ranges are
the manifest's `startFrame` tiling (S01 0–177, S02 from 178). The
audit's S02 starts at `lines[1].start − 0.06 = 5.94 − 0.06 = 5.88`,
so the attribution tiling gives S02's first frame =
`ceil(5.88 × 30) = 177` and S01 = 0–176. Neither is wrong: the
manifest tiles the *render*, the `shots` vec tiles the *audit*
(its sections above the tables already attribute overflow and
continuity by these ids and boundaries — `Audit::sample` calls
`shot_id_at` at telemetry.rs:729–730). The table must describe the
audit it accompanies; §4's header states the rule so a reader
diffing against the manifest is not surprised. The totals agree:
14 shots, frames tile [0, 1811), 1811 = `ceil(60.36 × 30)` by
film.rs:3517's own formula.

### 1.2 Caption table

One row per chunk, by index. **Every value is already computed**
before the verdict is assembled — the table is a re-print of the
`caps` vec built at film.rs:3706–3735, whose fields are
`CaptionInfo` (telemetry.rs:518–527):

| Column | Source |
|--------|--------|
| idx | position in `caps` (= position in `data.captions`; the vec is built in data order, film.rs:3707). |
| line | `CaptionInfo.line + 1`. The stored field is **0-based**: it is the `rposition` over `data.lines` for the last line with `start <= c.s + 0.05` (film.rs:3720–3724). The prototype prints 1-based; `print_caption_audit`'s own CAPTIONS line also converts (`(i + 1)`, telemetry.rs:1086). |
| shot | the shot owning the chunk's **start**, by the same attribution rule as §1.1 (`shot_id_at` semantics over the `shots` vec at `t = c.s`). This is the rule W75 §4.1 verified the manifest tiling encodes (start-ownership; a midpoint rule fails 3 chunks). |
| window (s) | `CaptionInfo.s` – `CaptionInfo.e`, `{:.2}`. |
| fit | `CaptionInfo.fit`, `{:.2}`. |
| rend | `CaptionInfo.rendered_w` — the ink-bbox width measured by the audit block itself (scan at film.rs:3709–3719: columns holding any pixel with alpha > 32, film.rs:3713; `rendered = x1 − x0 + 1`, film.rs:3719) over `assets.strips[i]` (`Assets.strips: Vec<(Vec<u8>, u32, u32)>`, film.rs:2090; loaded by `load_assets`, film.rs:2102, called at film.rs:3640). **The table must not re-measure**: printing the audit's own value is what makes the W75 parity claim (73/73 vs the prototype) structural rather than a second measurement that could drift. |
| raw | `CaptionInfo.raw_w` (= rendered / fit when fit > 0, film.rs:3731). |
| flag | `WIDTH` if `rendered_w > 850.0`, `FIT` if `fit < 0.68`, `WIDTH+FIT` if both (prototype join order: WIDTH then FIT, joined with `+`). These are **the same two predicates** `print_caption_audit` evaluates at telemetry.rs:1094–1095. The printer uses the same literals, cross-commented both ways; do NOT hoist them into shared consts in this change (behaviour-freeze on the verdict path — a const refactor is a separate, separately-gated diff). |
| text | `CaptionInfo.text`, quoted. |

**Straddle notes** (§4.3) derive from the same values: a chunk
straddles when `c.e` exceeds its owning shot's end (next shot's
`at`, or `duration` for S12's chunks — none straddle there on this
film). Overrun = `c.e − shot_end`, `{:.2}`.

### 1.3 Cast list

**Presence — the D2 decision.** Three candidate sources:

1. *The Audit's retained state* — insufficient by itself.
   `Audit::sample` (telemetry.rs:729) aggregates overflow (by
   id+pose+sides), signatures (by id), violations (id+shot), edge
   crossings (shot+id+side, telemetry.rs:772–790) and composition
   entries (shot+id+kind, telemetry.rs:797–810); its per-shot
   aggregate `CompShotAgg` (telemetry.rs:560–565) counts
   composition sample kinds, **not presence**. No plain
   per-(shot, id) presence count survives aggregation.
2. *Shot-granularity from data* (the prototype's method) —
   available but strictly worse inside the binary: it credits a
   character with every frame of a shot he appears in at all
   (W75 §4.3's example: Taniguchi enters S10 mid-shot and the
   prototype credits him all 159 S10 frames).
3. *The sample loop's own `cuts`* — **chosen.** Each iteration of
   the loop in `run_film_telemetry` builds the full `Cutout` vec
   for sample time `t` (film.rs:3679–3699; each cutout's `id` is
   `INSTANCE_IDS[ci.look]`, film.rs:3684, registry at
   film.rs:3632–3634) immediately before `audit.sample(t, &cuts)`
   (film.rs:3700). The staging work is already paid; recording
   presence costs one pass over a ≤ 4-element vec per sample
   (~242 samples over the film).

**The accumulator** (film.rs, beside the loop; the only new state
the feature adds):

```rust
// Per (shot, identity) sampled presence, in first-appearance order.
struct PresenceRow {
    id: String,          // INSTANCE_IDS identity
    shot: String,        // ShotInfo.id the sample attributes to
    samples: u32,        // samples in which the id was staged
    max_simul: u32,      // max simultaneous instances in one sample (S11 onoda = 2)
    first: f64, last: f64, // first/last sample times (kept for the note checks; not printed)
}
```

Per sample (after `cuts` is built, film.rs:3699, before or after
`audit.sample` — order between them is immaterial): attribute the
sample with the Audit's own rule. `Audit::shot_id_at` is private
(telemetry.rs:710); the printer-side attribution in §1.1/§1.2 is
over the `shots` slice, so implement one free helper in
telemetry.rs — `fn shot_at<'a>(shots: &'a [ShotInfo], t: f64) ->
&'a ShotInfo` — carrying the identical rule (last `at <= t + 1e-9`,
default first), have the accumulator call it via a pub(crate)
re-export, and pin its equivalence to `Audit::shot_id_at` with a
unit test (§5). Do not change `Audit` itself. Then, for each
distinct id in `cuts` (counting multiplicity): create-or-update
the (shot, id) row — `samples += 1`, `max_simul = max(max_simul,
count_in_this_sample)`, `first`/`last` updated.

**Columns from the accumulator**, per identity, ordered by first
appearance (first sample time, ties broken by the id's order
within that sample's `cuts`):

| Column | Source |
|--------|--------|
| id | the identity string. Expected order on the real film: onoda, compA, compB, tani, suzuki (tani's first sample is in S03 ≈ 13.06 s, suzuki's in S08 ≈ 34.21 s — same order as the prototype's scene-file traversal). |
| shots | count of distinct shots with a row for the id. |
| frames | **sampled-presence frames** = `samples × 7.5` over all the id's rows (0.25 s sample step, film.rs:3701, × the 30 fps frame rule, film.rs:3534), printed as an integer with round-half-up: `(samples * 15 + 1) / 2` in integer arithmetic. This is the D2 measure; §4.2's header says so. It is *not* expected to equal the prototype's shot-granularity numbers (1557/592/592/537/334): expect ≤ those + one sample (7 frames) for shot-long presences, and visibly less where presence is partial — tani's total should land **below** 537 precisely because of the S10 mid-shot entrance. |
| beats | the id's verified authored beats (below). |
| shot list | the id's shots in film order, binary ids verbatim (D5). |

**Authored blink beats.** Beat *times* are data:
`FilmData.anchors` (film.rs:46, loaded film.rs:96–101). The
anchor → (identity, shot) binding exists only in the staging
code, so the printer carries the transcription — the same table
the prototype fenced as `BEAT_BINDINGS`
(tools/film/telemetry_tables.py), re-cited at this HEAD:
`years0` → (onoda, S01), call site film.rs:1459 (in `s01`,
film.rs:1447, seed 3); `never2` → (onoda, S03), film.rs:1528
(`s03`, 1509); `message3` → (onoda, S04), film.rs:1565 (`s04`,
1544); `found7` → (suzuki, S08), film.rs:1803 (`s08`, 1784);
`commander8` → (onoda, S09), film.rs:1870 (`s09`, 1853);
`bookseller9` → (tani, S10), film.rs:1911 (`s10`, 1888);
`loyal11` → (onoda, S12), film.rs:2028 (`s12`, 2003). The calls
feed `blink_of` (film.rs:1443); the beat-frame count per beat is
the count of frames *n* with `0 <= n/30.0 − beat < 0.12` — the
`blink_at` window (film.rs:256–264) under the frame rule
(film.rs:3534) — i.e. 3 or 4 frames per beat, computed, never
hardcoded.

**Verification, D7.** Each binding is checked against the run's
own data before printing: the anchor exists in `data.anchors`
(lookup by `.get()` — a missing anchor is a note, never a panic);
the beat time falls in its shot's `[at, end)` window (audit
boundaries, §1.1); the identity has a presence row for that shot
(samples > 0). A binding that fails any check prints its beat
entry suffixed ` (unverified)` and one `note:` line after the
cast list naming the failed check — and nothing else changes:
no exit-code effect, no suppressed rows (§2). On the real film
all seven verify (W75 §4.3 verified the same bindings against
the scene data; the presence check here is against sampled
presence, a superset test at shot granularity).

---

## 2. Flag plumbing

### 2.1 main.rs — parse

Beside the existing telemetry state (main.rs:292):

```rust
let mut tables = false;
```

One arm in the parse loop (main.rs:296–386), next to the
`--film-telemetry` arm (main.rs:338–341). `--tables` is
value-less; the shape precedent is the `--draft` arm
(main.rs:370–379):

```rust
"--tables" => {
    tables = true;
}
```

Like `--from`/`--to` outside `--film`, `--tables` is accepted
anywhere and only consulted by the `--film-telemetry` dispatch;
it is not an error with other modes. It has no interaction with
the F4 knobs: the telemetry dispatch (main.rs:391–406) returns
before the knobs substitution (main.rs:408–412, whose comment
already records that the telemetry early-returns are CPU-only
and never see it).

### 2.2 main.rs — dispatch and the one signature change

```rust
match film::run_film_telemetry(std::path::Path::new(dir), tables) {
```
(main.rs:397; the exit mapping at main.rs:398–405 is untouched.)

`run_film_telemetry` (film.rs:3636) gains the parameter:
`pub(crate) fn run_film_telemetry(dir: &Path, tables: bool) ->
Result<bool, String>`. main.rs:397 is its only call site.

### 2.3 HELP

One line under the `--film-telemetry` entry (main.rs:70), in the
house two-space style:

```
    --tables            With --film-telemetry: also print the
                        shot, caption and cast tables after the
                        verdict line. Default output is unchanged.
```

The usage synopsis (main.rs:50) and the exit-code entries
(main.rs:132, 135) do not change — no new mode, no new code.

### 2.4 Print order inside `run_film_telemetry`

Today's stdout, in order, and where the tables go:

1. **stderr** progress (`film-telemetry: sample N …`, every 20
   samples, film.rs:3665–3667) — untouched; tables add nothing
   to stderr.
2. **stdout** paint-cache stats line (`println!` at
   film.rs:3703, immediately after the sample loop) — untouched,
   still first on stdout.
3. The legacy `out` block (built film.rs:3736–3751: header,
   `print_audit` film.rs:3738, `print_continuity` 3739,
   `print_composition` 3740, `print_caption_audit` 3741, verdict
   line pushed last at 3743–3751) printed by the loop at
   film.rs:3752–3754 — **byte-for-byte untouched**, verdict line
   still the block's last line.
4. **NEW, only when `tables` is true:** one blank line, then the
   three tables, printed from a separate `Vec<String>` built by
   `print_film_tables` (§1.0) after step 3's loop. The tables
   are never pushed into `out` — `out`'s contents and order are
   a frozen surface (the adapter's verdict scan and every
   recorded output shape read it).

If the run cannot proceed (any `Err` from `load_film_data`
film.rs:3638 / `load_assets` film.rs:3640), nothing of steps
2–4 prints, with or without `--tables` — there is no
tables-only failure mode (W75 §2).

### 2.5 Exit codes — unchanged, and why they cannot move

The return at film.rs:3759 (`Ok(bad == 0)`) and main's mapping
(0 OK / 1 FAILED / 3 could-not-run, main.rs:398–405) are not
modified. The tables code is infallible by construction: it
formats values already in hand (`shots`, `caps`, `presence`,
`anchors`); its only lookups are `anchors.get()` (§1.3) and
slice indexing over vecs whose lengths the audit already
trusted. It contains no `?`, no `unwrap`/`expect` on data
paths, and no I/O beyond the final `println!` loop it shares
with step 3. A flagged caption row, an unverified beat, a
straddle — all are printed data, never verdict inputs (W75 §2).

---

## 3. Byte-stability proof plan

The claim to prove: **default output is byte-identical before
and after the change, and `--tables` output is the default
output as a strict prefix.** Builds are CI-only in this repo
(no local cargo), so every step names the CI binary of a stated
commit, mirroring how W62/W64 obtained theirs.

0. **Determinism pre-check.** With the pre-change CI binary
   (the green run at the tables change's parent commit), run
   `talerender --film-telemetry film` twice from the repo root;
   the two stdouts must already be identical (`diff` empty,
   exit 0 both). This establishes that the surface being frozen
   is itself stable: stdout carries the paint-cache stats line
   (counters, film.rs:3703) and the `out` block — no wall-clock,
   no RSS, no paths. (Progress goes to stderr and is out of
   scope.) If this pre-check fails, stop: the proof would be
   chasing noise, and the instability is a finding in its own
   right.
1. **Default identity.** Post-change CI binary, same command,
   no flag: stdout B. `diff A B` (A = either pre-change run)
   must be empty and the exit codes equal (0). This is the
   headline proof — the legacy block, stats line included, is
   untouched bytes.
2. **Prefix property.** Post-change binary with `--tables`:
   stdout C. `head -n "$(wc -l < A)" C | diff - A` must be
   empty: C's first |A| lines are A exactly, so the verdict
   line sits at the same line number in both, and everything
   after it in C is the tables block (blank separator + three
   tables). Exit code still 0.
3. **FAILED direction.** On a scratch copy of the film data
   with one caption strip replaced by a widened variant
   (rendered > 850 px — e.g. a horizontally stretched copy of
   chunk 50's strip, already 844 px), pre- and post-change
   binaries without the flag must print the identical
   `FILM TELEMETRY: CHECK FAILED (…)` line and exit 1; with
   `--tables`, the same line at the same position, exit 1,
   and the caption table's row for that chunk carries the
   `WIDTH` flag. (Do not mutate the repo's `film/` tree —
   copy it; the telemetry entry takes a DIR argument.)
4. **Could-not-run direction.** `--film-telemetry` on a
   missing dir (and on W62's strip-mismatch fixture dir,
   tools/ci/fixtures/adapter_reproof/project-strip-mismatch,
   whose failure is a load error): exit 3, the `film telemetry
   failed:` line on stderr, and **no** table header anywhere
   in stdout — with and without `--tables`.
5. **Adapter grid.** Re-run W62's grid
   (tools/ci/fixtures/adapter_reproof/run_reproof.sh) once
   with the post-change binary and `--tables` on for the
   telemetry cells: expected 8/8 unchanged (W75 §7's
   belt-and-braces proof that additive output below the
   verdict does not disturb the gate).

Record all five results in the landing commit's message or a
short verification note beside it, with the two commit hashes
(pre/post) named — the proof is only citable with its binaries
identified.

---

## 4. Format spec

The prototype's layouts are normative. Each block below quotes
the prototype's format code (tools/film/telemetry_tables.py)
and gives the Rust equivalent. Rust's `format!` width/align
specifiers mirror Python's for every field used here
(`{:<6}`, `{:>6}`, `{:.2}`, `{:.0}`); text is always the last
field so long values never shift a column (W75 §3.1). The
em-dash in headers is U+2014, as in the prototype and in the
binary's own telemetry header (film.rs:3737).

### 4.1 Shot table

Prototype (telemetry_tables.py, `shot_table`):

```python
f"SHOT TABLE — {len(segments)} shots, {film['totalFrames']} frames @ {film['fps']} fps, {fmt_t(film['durationS'])} s"
f"{'shot':<6}{'frames':<13}{'time (s)':<16}{'dur':>6}  {'set':<5}{'atmosphere':<11}characters"
f"{seg_id:<6}{fr:<13}{tm:<16}{dur:>6.2f}  {set:<5}{atmosphere:<11}{chars}"
# fr = f"{startFrame}-{endFrame - 1}", tm = f"{startS:.2f}-{endS:.2f}"
```

Rust (values per §1.1):

```rust
out.push(format!("SHOT TABLE — {} shots, {} frames @ 30 fps, {} s",
    shots.len(), total_frames, fixed2(duration)));
out.push(format!("{:<6}{:<13}{:<16}{:>6}  {:<5}{:<11}characters",
    "shot", "frames", "time (s)", "dur", "set", "atmosphere"));
// per shot:
out.push(format!("{:<6}{:<13}{:<16}{:>6.2}  {:<5}{:<11}{}",
    id, fr, tm, dur, set, atmosphere, chars));
```

`total_frames` = `(duration * 30.0).ceil() as u32` — film.rs:3517's
own expression. `fixed2` is telemetry.rs:63. The fps in the
header is the literal `30` of the binary's frame rule
(film.rs:3517, 3534) — there is no fps value to read (§1.1).

### 4.2 Cast-list header delta (applies here, stated once)

The prototype's shot-table header and all three column headers
carry over byte-for-byte. The single exception is the cast
header's parenthetical (§4.5), where the honesty clause must
name the D2 measure instead of the prototype's.

### 4.3 Caption table + straddle notes

Prototype (telemetry_tables.py, `caption_table` + `main`):

```python
f"CAPTION TABLE — {len(rows)} chunks (audit bars: rendered > 850 px = WIDTH fail, fit < 0.68 = FIT fail; strict)"
f"{'idx':<5}{'line':<6}{'shot':<6}{'window (s)':<17}{'fit':>5}{'rend':>6}{'raw':>7}  {'flag':<10}text"
f"{i:<5}{line:<6}{shot:<6}{win:<17}{fit:>5.2f}{rend:>6.0f}{raw:>7.0f}  {flag:<10}\"{text}\""
# win = f"{s:.2f}-{e:.2f}"
f"note: chunk {i} ({sid}) straddles the cut — its window runs {over:.2f} s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)"
```

Rust equivalents are one-to-one (`{:.2}`/`{:.0}` for the
float fields; the bar numbers in the header are the literals
of telemetry.rs:1094–1095, §1.2). Notes print in chunk order
immediately after the last caption row, with no blank line
between the table and its notes (prototype `main` appends
them before the blank separator). `eval_frame` in the note
text cites film.rs:2043 at this HEAD.

**Worked example — chunk 41.** `"for decades,"`, window
30.29–31.55, owned by S06; S06's end is S07's `at` =
`anchors["decades6"] − 0.25` = 30.39 (film.rs:3651), so the
overrun is 31.55 − 30.39 = **1.16 s** — the note must read
exactly:

```
note: chunk 41 (S06) straddles the cut — its window runs 1.16 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)
```

This is the film's one long straddle (the other twelve are
≤ 0.47 s): S06 is fog, S07 is golden, so the caption plays
across the atmosphere change for over a second (W75 §6.1).
The audit-boundary computation reproduces the prototype's
manifest-based 1.16 exactly, because S07's audit start and
the manifest's S06 endS are the same instant (30.39).

### 4.4 The disclosed deltas (complete list)

Everything else in the binary's tables matches the
prototype's output shape; the following are the **only**
differences, each forced by §1 and each to be named in the
landing commit message:

1. **Shot ids verbatim** (D5): `S05a`/`S08b` where the
   prototype prints `S05A`/`S08B` — in the shot table, the
   caption table's shot column, straddle notes, and cast
   shot lists alike.
2. **Shot frame ranges** by audit attribution (D3): e.g.
   S01 `0-176`, S02 from 177 (prototype: `0-177`, 178).
   Time ranges and durations are the audit's `at`-derived
   values (S01 `0.21-5.88` — the first shot's printed start
   is its `at`, 0.21, not 0.00; its frame range still starts
   at frame 0 by the default rule, telemetry.rs:711–715).
3. **Cast frames** are sampled-presence frames (D2), with
   the §4.5 header wording.
4. **Cast/shot character order** is first-sampled order
   (the prototype used scene-file node order; e.g. S03's
   row may order its four ids differently). Deterministic:
   the sample grid and staging are fixed, so run-to-run
   diffs still read as data changes (W75 §3.2's law holds).
5. **No RECONCILIATION lines** (D6).

### 4.5 Cast list

Prototype (telemetry_tables.py, `cast_list`):

```python
f"CAST LIST — {len(order)} identities (frame counts are shot-granularity: full frames of every shot the identity appears in)"
f"{'id':<9}{'shots':>6}{'frames':>8}{'beats':>7}  shot list / authored blink beats"
f"{cid:<9}{nshots:>6}{frames:>8}{nbeats:>7}  {', '.join(shots)}"
f"{'':<9}{'':>6}{'':>8}{'':>7}  beats: {beat_s}"
# beat_s entries: f"{anchor} @ {t:.2f} ({shot}, {n}f)", joined "; "; "-" when none
```

Binary header (the §4.2 delta):

```
CAST LIST — {n} identities (frame counts are sampled presence: 0.25 s audit samples x 7.5 frames per sample)
```

Column header, row, and beats-continuation formats carry
over byte-for-byte, including the 30-column indent of the
beats line and the `beats: -` empty form. Beat entries use
the binary shot id verbatim (D5) and the §1.3 frame counts.

### 4.6 Block structure

With `--tables`, stdout after the verdict line is exactly:

```
<blank>
SHOT TABLE …
<shot rows>
<blank>
CAPTION TABLE …
<caption rows>
<straddle notes, if any>
<blank>
CAST LIST …
<cast rows + beats lines>
<unverified-beat notes, if any>
```

No trailing summary line, no second blank at the end beyond
the final row's newline.

---

## 5. Worked expectations on the real film (acceptance inputs)

Derived from the data and rules cited in §1 — these are the
numbers the first real run must be checked against (the full
acceptance block is §6.2):

- **Shot table:** header `SHOT TABLE — 14 shots, 1811 frames
  @ 30 fps, 60.36 s` (duration 60.357 → `fixed2` → `60.36`);
  14 rows; frame ranges tile [0, 1811) with no gaps or
  overlaps; S01 = frames 0–176, time 0.21–5.88 (D3 example);
  S12 ends at frame 1810 / 60.36 s; S05a's characters cell is
  `(none)`; S11's contains `onoda x2`.
- **Caption table:** 73 rows, **0 flags**; max rend **844**
  at chunk 50 (`"Onoda still refused"`); min fit **1.00**;
  chunk 0 rend 208, chunk 6 rend 60; and the parity check —
  every row's rend equals the prototype's recorded value for
  that idx in TELEMETRY-TABLES-W75 §5 (73/73; that block is
  the recorded evidence the parity claim is checked
  against, since both print the one audit measurement, §1.2).
- **Straddle notes:** exactly **13**; overrun values
  0.03–0.47 s for twelve of them and **1.16 s** for chunk 41
  (§4.3's verbatim line).
- **Cast list:** 5 identities in the order onoda, compA,
  compB, tani, suzuki; shots **12 / 4 / 4 / 4 / 3**; beats
  **5 / 0 / 0 / 1 / 1** = 7 bound, all verified (no
  `(unverified)` suffixes, no beat notes); beat-frame counts
  per anchor — years0 4f, never2 3f, message3 4f, found7 3f,
  commander8 3f, bookseller9 4f, loyal11 4f — totalling
  **25 beat-frames** (W58's independent enumeration, via the
  W75 prototype); frames column = the D2 sampled measure
  (first run records the actuals; expectations: each within
  one sample — 7 frames — above its shot-granularity
  counterpart 1557/592/592/537/334 is impossible since
  sampled ⊆ staged, so each ≤ counterpart + 7 for
  shot-long presences, and tani strictly below 537).

---

## 6. Test plan

### 6.1 Unit tests (telemetry.rs test module, telemetry.rs:1437)

Following the existing `CaptionInfo`-fixture pattern
(telemetry.rs:1599):

1. **Caption row rendering.** Three `CaptionInfo` fixtures
   modelled on the film's named extremes — chunk 0 `"This"`
   (line 0, s 0.27, e 0.43, fit 1.00, rendered_w 208,
   raw_w 208 → row shows line **1**, rend 208, no flag);
   chunk 50 `"Onoda still refused"` (rendered_w 844 → still
   no flag: the bar is strict `>`, telemetry.rs:1094);
   chunk 6 `"II"` (rendered_w 60). Plus two synthetic
   predicate cases: rendered_w 851 → `WIDTH`; fit 0.67 →
   `FIT`; both → `WIDTH+FIT`. This pins the table's flags to
   `print_caption_audit`'s predicates without touching them.
2. **Shot attribution equivalence.** Build the real 14-shot
   vec (same construction as film.rs:3643–3658, from a
   fixture lines/anchors set or the literal `at` values) and
   assert the new `shot_at` helper agrees with
   `Audit::shot_id_at` (via `Audit::new(shots)`) at: t = 0,
   each shot's `at` exactly, `at − 1e-6`, `at + 1e-6`, and
   duration. Include the first-shot default (t < shots[0].at
   → S01) and chunk 41's start (30.29 → S06) and end-side
   boundary (30.39 → S07).
3. **Frame tiling.** From the same fixture: the §1.1 closed
   form tiles [0, 1811) for duration 60.357; S01 first frame
   0 despite `at` = 0.21; S02 first frame 177.
4. **Beat-frame counting.** For beat times 4.20 and 14.78,
   the counter's result equals
   `#{n in 0..200 : blink_at(n/30.0, &[b]) == 1.0}` computed
   by calling film.rs's own `blink_at` (film.rs:257) — 4 and
   3 respectively — tying the new counter to the transcribed
   rule instead of restating it.
5. **Presence accumulator.** Synthetic sample sequence:
   an id present in 3 samples of one shot → samples 3,
   frames `(3*15+1)/2 = 23`; two same-id cutouts in one
   sample → `max_simul` 2 and samples +1 (not +2); an id
   absent from a shot has no row for it (the S05a `(none)`
   case at unit scale).

### 6.2 Acceptance run (CI binary, real film)

`talerender --film-telemetry film --tables` from the repo
root, checked against §5 in full:

- exit 0; §3 steps 1–2 already proven for the same binary
  (default identity + prefix property), so the verdict line
  in this output is at the default run's line number;
- shot table: header line, 14 rows, tiling, S01/S11/S05a
  cells as §5;
- caption table: 73 rows, 0 flags, max rend 844 (chunk 50),
  min fit 1.00, and rend parity 73/73 against
  TELEMETRY-TABLES-W75 §5;
- exactly 13 straddle notes; chunk 41's note byte-equal to
  §4.3's quoted line;
- cast list: order, shot counts, beat counts and per-anchor
  frame counts as §5 (7 bound / 25 beat-frames, none
  unverified); frames column recorded (no fixed expectation
  — first run's values become the regression baseline);
- §3 steps 3–5 (FAILED direction, could-not-run, adapter
  grid 8/8 with `--tables`).

### 6.3 What is explicitly not tested by this feature

The verdict path itself (already gated: telemetry exit
codes, W62's grid, the determinism and golden dispatches)
and the manifest tiling in `scenes/film/film.json` (the
tables deliberately do not read it, D3/D4). A future
manifest-vs-audit boundary diff is a tools-side check, not
part of this landing.

---

## 7. Landing checklist for the lead (ordered)

1. telemetry.rs: `shot_at` helper (§1.3) + `PresenceRow`
   input struct + `print_film_tables` with the two const
   tables (`SHOT_DRESSING` §1.1, beat bindings §1.3), each
   carrying its source-of-truth comment. Unit tests §6.1
   in the same commit.
2. film.rs: clone `shots` before `Audit::new`
   (film.rs:3659); presence accumulator in the sample loop
   (after film.rs:3699); signature gain of `tables: bool`
   (film.rs:3636); tables block after the `out` print loop
   (film.rs:3752–3754) per §2.4.
3. main.rs: `tables` state (beside main.rs:292), `--tables`
   arm (beside main.rs:338–341), dispatch argument
   (main.rs:397), HELP line (§2.3).
4. CI: build both binaries (parent + landing commit) and
   execute §3 steps 0–5, then §6.2; record the results and
   the cast frames actuals in the landing record.
5. Commit message names: the §4.4 disclosed deltas (all
   five), the D2 measure change, and the §3/§6.2 verdicts.
   WAVE2-RECORD fold-in notes F4-style: the tables are an
   opt-in print widening; the verdict line, exit codes and
   default stdout are byte-frozen, with §3's diffs as the
   evidence.
