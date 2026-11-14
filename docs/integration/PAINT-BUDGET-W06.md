# Paint-cache byte budget + stats — W06 design note

**Seat:** Wave 2 W06. **Delivered:** `src/paintbudget.rs` (new file,
uncommitted — the lead wires it). **Nothing else touched.** The module
is verified standalone: `rustc --edition 2021 --test
src/paintbudget.rs` → **12 passed, 0 failed**. (Two dead-code
warnings appear in the standalone build only — `from_env`,
`budget_bytes`, `entry_cap`, `DEFAULT_ENTRY_CAP` are the API surface
the wiring below consumes; they disappear once film.rs calls them.)

## 1. Why

The audit's memory finding, in the repo's own numbers: film.rs's
`PaintCache` enforces an ENTRY cap (default 160, env
`TALERENDER_PAINT_CACHE`). One painted canvas is 2560×2816 RGBA =
**28,835,840 bytes** (27.5 MiB), so the default cap's worst case is
160 × 28,835,840 = **4,613,734,400 B ≈ 4.6 GB** — on the target PC's
~6.7 GiB of usable RAM, next to the renderer, the assets and the OS.
PERF.md §7 already records that worst case as a genuine OOM risk.
Counting entries made the risk invisible in the code; counting
bytes makes it a policy. PERF.md §9 item 2 separately asks for the
missing signal: no hits/misses/evictions are printed anywhere, so
the `TALERENDER_PAINT_CACHE` time-for-memory trade cannot be
quantified from output.

## 2. What the module is

`PaintBudget` — accounting + eviction policy for one `PaintCache`
lifetime (film.rs constructs a fresh cache per render / per
telemetry run, so counters are per-run by construction). It mirrors
film.rs's real types, no parallel ones: keys are the cache's own
`String` state keys, values are accounted by `entry_bytes(&painted.0)`
= the pixel buffer's length. API:

| Method | Called from (wiring §4) |
|---|---|
| `PaintBudget::from_env(entry_cap, CANVAS_BYTES)` | `PaintCache::new()` |
| `on_hit()` | `get_or_paint` hit path |
| `on_miss(key)` | `get_or_paint` miss path, before painting |
| `on_insert(bytes)` | after `map.insert` |
| `on_evict(key, bytes)` | per FIFO eviction |
| `should_evict() -> bool` | the eviction loop condition |
| `admit(bytes) -> bool` | optional pre-insert check (see §5) |
| `stats_line() -> String` | the two end-of-run print sites |

Counters: current bytes, peak bytes, entries, hits, misses,
evictions, and **rerenders** — misses whose key was evicted earlier
in the run, i.e. the actual re-paint cost the budget causes. (They
are tracked via a `HashSet` of evicted keys, consulted only on a
miss; growth is bounded by distinct evicted states at ~200 B/key —
even 50k evictions ≈ 10 MB, two orders below one canvas.)

Accounting excludes the Rc control block, the tuple scalars and the
key string: together they are ~0.001% of a 27.5 MiB entry, and the
budget is a ceiling with GiB of headroom, not an RSS predictor
(PERF.md §4 reads RSS against exactly this canvas math).

## 3. The default budget, derived

```
default = min(entry_cap × CANVAS_BYTES, 2048 MiB)
```

- At the default cap: min(4.61 GB, 2 GiB) = **2 GiB** ≈ 74 canvases.
  2 GiB keeps the cache near 30% of the PC's 6.7 GiB usable RAM —
  the renderer (composer targets, textures), the film assets and
  the OS keep the rest; GitHub's ubuntu-22.04 runners (7 GB) sit in
  the same envelope. It is >3× the memory-tight telemetry setting
  (cap 24 ≈ 0.69 GB) that already audits the whole film, and still
  far above the live working set of any single shot (film.rs's own
  characterisation of the 160 cap).
- A smaller operator cap scales the budget down with it (cap 24 →
  24 × canvas ≈ 0.69 GB): the byte budget never silently exceeds
  the chosen entry cap in byte terms.

Overrides, `TALERENDER_PAINT_BUDGET_MB`:

| Value | Meaning |
|---|---|
| unset / unparseable | derived default above |
| `N` (> 0) | budget = N MiB |
| `0` | byte enforcement **disabled** — entry cap alone (legacy behaviour) |

## 4. Wiring sketch (film.rs — lead's edit, ~25 lines)

**main.rs:** add `mod paintbudget;` after `mod math;`. (Optional,
same commit or later: add `TALERENDER_PAINT_BUDGET_MB` to the
`--help` ENVIRONMENT section next to `TALERENDER_PAINT_CACHE`.)

**film.rs `PaintCache` struct + `new()`** — the entry-cap env read
moves from per-insert (current code re-reads
`TALERENDER_PAINT_CACHE` inside `get_or_paint` on every miss) to
construction. Env vars are process-static, so this is
behaviour-identical; it just reads once:

```rust
pub(crate) struct PaintCache {
    map: HashMap<String, std::rc::Rc<Painted>>,
    order: std::collections::VecDeque<String>,
    budget: crate::paintbudget::PaintBudget,
}

fn new() -> PaintCache {
    let cap: usize = std::env::var("TALERENDER_PAINT_CACHE").ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(crate::paintbudget::DEFAULT_ENTRY_CAP);
    PaintCache {
        map: HashMap::new(),
        order: std::collections::VecDeque::new(),
        budget: crate::paintbudget::PaintBudget::from_env(
            cap,
            crate::paintbudget::CANVAS_BYTES,
        ),
    }
}

fn stats_line(&self) -> String {
    self.budget.stats_line()
}
```

**`get_or_paint`** — three call-outs and a new loop condition;
painting, key handling and FIFO order are untouched:

```rust
fn get_or_paint(&mut self, key: String, look: &FullLook, st: &FullState) -> std::rc::Rc<Painted> {
    if let Some(v) = self.map.get(&key) {
        self.budget.on_hit();
        return v.clone();
    }
    self.budget.on_miss(&key);
    let v = std::rc::Rc::new(paint_toon_full(look, st));
    self.budget.on_insert(crate::paintbudget::entry_bytes(&v.0));
    self.map.insert(key.clone(), v.clone());
    self.order.push_back(key);
    while self.budget.should_evict() {
        let old = self.order.pop_front().expect("cache order");
        let old_v = self.map.remove(&old).expect("cache order");
        self.budget
            .on_evict(&old, crate::paintbudget::entry_bytes(&old_v.0));
    }
    v
}
```

(The old body declared `let cap …` mid-function and looped on
`self.map.len() > cap`; both go away. The `expect("cache order")`
invariant is preserved verbatim — the tracker counts mirror
map/order exactly because every mutation is reported, so
`should_evict()` agrees with the old condition whenever the byte
budget is disabled or non-binding.)

**Print sites** — one line each, next to the existing end-of-run
lines:

- `run_film`, immediately after
  `println!("film render complete: …")`:
  `println!("{}", cache.stats_line());`
- `run_film_telemetry`, immediately after the
  `while t < data.duration { … }` sampling loop (before the caption
  audit, so the line is emitted even if a later stage fails):
  `println!("{}", cache.stats_line());`

Output shape:

```
paint-cache: entries=41 bytes=1182269440 hits=812 misses=97 evictions=56 rerenders=12 peak_bytes=1268776960 budget_bytes=2147483648
```

## 5. Semantics that were decided, not defaulted

- **Oversize single entry.** A budget smaller than one canvas is
  legal (e.g. an operator forcing maximum re-render on a tiny
  machine). `should_evict` never evicts the LAST remaining entry
  on the byte condition: that entry is live, and evicting it on its
  own insertion would thrash without freeing live memory. The
  entry-cap condition is checked exactly as the legacy code did —
  cap 0 still evicts everything.
- **`admit(bytes)`** answers "would this fit once resident?". The
  primary wiring ignores it (insert-then-evict is simpler and
  matches today's shape); it exists for the alternative wiring
  where a guaranteed-to-exceed entry is painted but not retained.
  The lead picks one; the module supports both.
- **Zero budget ≠ zero cache.** `TALERENDER_PAINT_BUDGET_MB=0`
  disables byte enforcement only. There is no setting in this
  module that disables painting or caching outright — cap 0 via
  `TALERENDER_PAINT_CACHE` already covers that, unchanged.

## 6. Pixel-neutrality argument

1. Painting is a pure function of the exact state key. Since
   cabcc30 the keys are f32 bit patterns, so a key identifies one
   unique painted output; the lead's determinism probes at the
   final head (frames 1195–1204 10/10, 795–814 20/20 byte-identical
   across independent runs, under different cache pressure) are the
   standing proof.
2. The tracker observes four events and advises a loop count. It
   holds no pixel data, computes no key, and cannot reorder paints.
3. Eviction removes the map's `Rc<Painted>`; `Rc`s already handed
   to `resolve_chars` for the frame in flight keep the entry alive
   and unmodified. An eviction mid-frame cannot alter that frame.
4. FIFO order is unchanged (`order.pop_front()`); the budget only
   changes WHEN the loop stops, and at the derived default the byte
   ceiling (≈74 entries) binds after any realistic per-shot working
   set and before the old 160-entry worst case — so default runs
   see strictly fewer evictions than a hypothetical unbounded run
   and the same output as today's capped runs, by (1).

Therefore any budget setting — disabled, 2 GiB, or smaller than one
canvas — changes only how often `paint_toon_full` re-runs, never a
rendered pixel.

## 7. PERF.md §9 discharge

`stats_line()` at the two print sites discharges §9 **item 2**
("No paint-cache statistics") in full: hits, misses, evictions and
final size are printed at end of run in both film and telemetry
modes, plus rerenders, peak bytes and the active budget so the
time-for-memory trade can be read off a single grep. Items 1
(progress-line clock), 3 (peak-RSS self-report), 4 (telemetry exit
codes — owned by the exit-codes work) and 5 (GPU memory blind spot)
are NOT addressed by this module and remain open.

## 8. What the lead must verify post-wiring

1. **CI green**, including the 12 new `paintbudget` tests (they run
   under the crate's normal `cargo test` once the module is in the
   tree) and the existing 71-test suite.
2. **Byte-compare probes, pre/post wiring**, same protocol as the
   determinism proof: frames **1195–1204** and **795–814** rendered
   both ways must be sha256-identical — once at default settings
   and once under forced eviction (`TALERENDER_PAINT_BUDGET_MB=100`,
   i.e. a ~3-canvas budget) to exercise the byte condition, not
   just the entry cap.
3. **Telemetry unchanged**: a `--film-telemetry` run at
   `TALERENDER_PAINT_CACHE=24` prints the same verdict line as
   pre-wiring, with the `paint-cache:` line present in stdout.
4. **Sanity-read one stats line** from a full-film render: entries
   ≤ cap, bytes ≤ max(budget, one canvas), peak_bytes ≥ bytes,
   misses − rerenders ≥ distinct states painted. (No RSS claim is
   made from this VM; the PERF protocol run on the user's PC is
   where the 2 GiB ceiling gets its real-world reading.)
