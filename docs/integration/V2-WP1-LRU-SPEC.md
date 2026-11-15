# V2-WP1-LRU-SPEC — WP1.1 paint-cache LRU patch spec + WP1.4 stats taxonomy

Seat: TaleRender V2, Batch 2, Seat 1 (WP1.1 + WP1.4 spec half).
Authority: `V2-CONTRACTS.md` §1 (lead applies all patches to
existing files), §5 Batch 2 seat 1; master prompt §5.1 (paint-cache
eviction) and §5.4 (resource lifetimes — the code half is
`src/lifetimes.rs`, delivered with this spec).
Status of this document: PATCH SPEC ONLY. No existing file was
edited by the seat. The lead applies Patches 1–5 below, in order,
in one landing, under the §4 minimal gates (contracts §4).

Code verified against the live tree while writing this spec:
`src/film.rs` `PaintCache` (lines 2267–2321), `src/paintbudget.rs`
in full, `src/bufferpool.rs` in full, `tools/perf/vram_report.py`,
`tools/perf/lab/bench_lab.py`. Line numbers are as of that read;
anchors are quoted text, which is what the lead should match on.

---

## 0. What the code actually does today (verified, not assumed)

`PaintCache` (`src/film.rs:2277-2281`):

```rust
pub(crate) struct PaintCache {
    map: HashMap<String, std::rc::Rc<Painted>>,
    order: std::collections::VecDeque<String>,
    budget: crate::paintbudget::PaintBudget,
}
```

`get_or_paint` (`src/film.rs:2301-2317`), verbatim:

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
        self.budget.on_evict(&old, crate::paintbudget::entry_bytes(&old_v.0));
    }
    v
}
```

Facts the design must respect:

1. **Eviction today is pure FIFO.** Insert does
   `order.push_back(key)` (newest at the back); eviction does
   `order.pop_front()` (oldest insertion first). The hit path
   (`film.rs:2302-2305`) never touches `order`. Nothing else in
   the tree mutates `order`. So "LRU" is exactly one missing
   operation: a recency touch on hit. The deque's orientation
   already matches the landed LRU pattern in
   `src/bufferpool.rs` (front = least-recent, back =
   most-recent; `push_back` on arrival, `pop_front` on evict) —
   this spec changes no orientation and no eviction end.
2. **`should_evict()` returns a bare `bool`** (`paintbudget.rs:172`):

   ```rust
   pub(crate) fn should_evict(&self) -> bool {
       self.entries > self.entry_cap
           || (self.budget_bytes > 0 && self.bytes > self.budget_bytes && self.entries > 1)
   }
   ```

   There is **no reason channel** anywhere: `on_evict`
   (`paintbudget.rs:196`) is `on_evict(&mut self, key: &str,
   bytes: u64)` and increments a single `evictions` counter.
   The eviction-reason taxonomy in §3 therefore cannot be
   "read off" the existing code — it needs the small
   `PaintBudget` API addition specified in Patch 2. Anyone
   claiming the reasons are already tracked is wrong.
3. **Every production entry costs the same bytes.**
   `entry_bytes` is `Painted.0.len()`, and `paint_toon_full`
   paints a fixed 2560×2816 RGBA canvas
   (`paintbudget::CANVAS_BYTES = 28,835,840`). Byte pressure and
   entry pressure are therefore proportional in production —
   but they still bind at *different points*, because the byte
   budget is the entry cap expressed in bytes **clamped to the
   2 GiB ceiling** (`default_budget_bytes`, `paintbudget.rs`):
   at the default cap 160 the byte budget binds at ~74 entries,
   long before the cap; at cap 24 the derived budget is exactly
   `24 × CANVAS_BYTES`, so both predicates trip on the same
   insert and a precedence rule (§3.2) is load-bearing, not
   decorative. Operator overrides (`TALERENDER_PAINT_BUDGET_MB`,
   including `0` = byte enforcement disabled) decouple them
   arbitrarily. The taxonomy is real; do not collapse it.
4. **The stats line today** (`paintbudget.rs:242-255`) is:

   ```
   paint-cache: entries={} bytes={} hits={} misses={} evictions={} rerenders={} peak_bytes={} budget_bytes={}
   ```

   pinned by `stats_line_format_is_stable`
   (`paintbudget.rs:407-421`) with the expectation
   `"paint-cache: entries=1 bytes=250 hits=2 misses=3 evictions=1 rerenders=1 peak_bytes=350 budget_bytes=1000"`.
5. **Call sites of `on_evict` — exactly seven.** One production
   site (`film.rs:2314`) and six unit tests in
   `paintbudget.rs` (lines 280, 312, 327, 352, 373, 415).
   Patch 2 changes the signature, so all seven are listed in
   §3.4 with the reason each test should pass. There are no
   other callers (grep-verified over `src/`).

---

## 1. Patch 1 — the recency change (WP1.1 core, `src/film.rs`)

### 1.1 The change

Anchor (current, `film.rs:2301-2305`):

```rust
    fn get_or_paint(&mut self, key: String, look: &FullLook, st: &FullState) -> std::rc::Rc<Painted> {
        if let Some(v) = self.map.get(&key) {
            self.budget.on_hit();
            return v.clone();
        }
```

Replacement:

```rust
    fn get_or_paint(&mut self, key: String, look: &FullLook, st: &FullState) -> std::rc::Rc<Painted> {
        if let Some(v) = self.map.get(&key).cloned() {
            self.budget.on_hit();
            // LRU touch (WP1.1): a hit makes `key` the
            // most-recently-used entry — move it to the back of
            // `order`, the end inserts already use and the end
            // opposite eviction's pop_front. `map` and `order`
            // hold the same key set exactly once each (insert
            // below is the only other writer), so the position
            // exists; if it somehow does not, returning the hit
            // without touching is the safe degradation — never
            // push a duplicate key.
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                let k = self.order.remove(pos).expect("cache order");
                self.order.push_back(k);
            }
            return v;
        }
```

Notes the lead should keep:

- **Why `.cloned()` on the `map.get`.** Today's shape holds
  `v: &Rc<Painted>` borrowed from `self.map` across
  `self.budget.on_hit()` (a disjoint-field borrow, which is
  why it compiles). The touch mutates `self.order` while the
  hit value is still needed; cloning the `Rc` up front
  (`.cloned()` on `Option<&Rc<Painted>>`) ends the map borrow
  at the semicolon-equivalent point and makes the field
  borrows obviously disjoint. Cost: one `Rc` refcount bump —
  exactly the `v.clone()` the old code already paid on return.
- **Cost of the touch.** `position` + `remove` on a
  `VecDeque<String>` is O(n) in entries (≤ entry cap; ~74 at
  the byte-bound default, 160 at the entry cap). That is a few
  hundred pointer/length moves against a hit that just saved a
  ~29 MB canvas paint. No new data structure, no new
  dependency, no timestamp/clock (deque position *is* the
  recency, as in `bufferpool.rs` — two runs issuing the same
  lookup sequence make the same eviction decisions).
- **Already-at-back is handled naturally**: `position` finds
  the last index, `remove` + `push_back` re-inserts the same
  key at the same end; the deque's key sequence is unchanged.
  (A `pos + 1 != self.order.len()` early-out is a valid
  micro-optimisation; it is not required and the unconditional
  form above is the one to land, for symmetry with the
  existing `expect("cache order")` style.)
- **Nothing else in `get_or_paint` changes.** The miss path,
  the insert `push_back`, and the eviction `pop_front` are
  byte-for-byte as today (except the loop head in Patch 3,
  §3.3, which does not change *which* entry is evicted — the
  front — only how the reason is obtained).
- **Invariant, stated for the reviewer:** before and after
  every `get_or_paint` call, `map.keys()` and `order` contain
  the same set of keys, each exactly once; `order` is sorted
  by last-use ascending (front = least-recently used). FIFO
  maintained this invariant with "last-use = insertion";
  Patch 1 extends last-use to hits. That is the entire
  semantic change.

### 1.2 Comment-only updates in the same patch

- `film.rs:2267`: `/// Painted-card cache with a FIFO cap.` →
  `/// Painted-card cache with an LRU cap.` The rest of that
  doc block (pure function / exact key / budget governance)
  is unchanged and remains true.
- `paintbudget.rs` module header, call-contract bullet
  (line ~102): `- each FIFO eviction: on_evict(key, bytes).`
  → `- each eviction: on_evict(key, bytes, reason).`
  (see Patch 2).

---

## 2. Patch 4 (specified before Patch 2/3 it depends on narratively) — the stats change

*(Numbering follows landing order in §5; this section is placed
here because the task charter orders it (b) — the lead applies
the patches in §5's order, not this section order.)*

### 2.1 New fields, appended at the END, and only at the end

`PaintBudget::stats_line` (`paintbudget.rs:242`) gains three
fields **after** `budget_bytes`:

```
paint-cache: entries={} bytes={} hits={} misses={} evictions={} rerenders={} peak_bytes={} budget_bytes={} hit_rate_permille={} evictions_byte={} evictions_cap={}
```

- `hit_rate_permille` — `hits × 1000 / (hits + misses)`,
  integer division; **`0` when `hits + misses == 0`** (a run
  with no lookups has no rate; 0 is the defined sentinel, and
  the field's doc comment must say so, so nobody reads it as
  "0 % success" for an empty run). Permille, not a float
  percent: every other field on the line is an integer, the
  line stays greppable and locale-free, and there is no float
  formatting to drift between Rust versions. It is a **derived
  value computed inside `stats_line`** from the existing
  counters — no new stored field, so it cannot drift from
  `hits`/`misses`:

  ```rust
  fn hit_rate_permille(&self) -> u64 {
      let total = self.hits.saturating_add(self.misses);
      if total == 0 {
          0
      } else {
          self.hits.saturating_mul(1000) / total
      }
  }
  ```

  (Private helper beside `stats_line`; `saturating_*` matches
  the house style in this file and in `bufferpool.rs`.)
- `evictions_byte` / `evictions_cap` — the Patch 2 reason
  counters (§3). Invariant, to be asserted in tests:
  `evictions_byte + evictions_cap == evictions` at every
  observable point.

The format-string replacement in `stats_line` is the obvious
one: same eight leading `{}` in the same order, then
` hit_rate_permille={} evictions_byte={} evictions_cap={}`
with `self.hit_rate_permille(), self.evictions_byte,
self.evictions_cap` appended to the argument list. The
doc comment above `stats_line` ("The first six fields are the
stable contract…") must be updated to say the first **eight**
fields are the stable prefix and the three appended fields
are the WP1.1 extension — the append-only rule is the
contract now: future fields go after `evictions_cap`, never
in the middle.

### 2.2 Proof the appended fields cannot break `vram_report.py`

Checked against the actual file, `tools/perf/vram_report.py`:

1. The grammar (lines 58–61) is

   ```python
   PAINT_CACHE_RE = re.compile(
       r"paint-cache: entries=(\d+) bytes=(\d+) hits=(\d+) misses=(\d+) "
       r"evictions=(\d+) rerenders=(\d+) peak_bytes=(\d+) budget_bytes=(\d+)"
   )
   ```

   It ends at `budget_bytes=(\d+)`. There is **no `$`, no
   `\Z`, no end anchor of any kind**, and it is applied with
   `PAINT_CACHE_RE.search(line)` (line 80) — a substring
   search, not `match`/`fullmatch` (grep-verified: no
   `fullmatch` anywhere in the file).
2. Therefore the regex matches the **prefix** of the extended
   line and ignores everything after the `budget_bytes`
   digits. The one subtlety — could `(\d+)` over-consume? —
   is settled by the separator: in the new line the character
   immediately after the `budget_bytes` value is a space
   (` budget_bytes=1000 hit_rate_permille=…`), and `\d+`
   cannot cross a space. Capture group 8 is byte-identical
   to today, as are groups 1–7 (their fields, order and
   separators are untouched).
3. Backward compatibility runs the other way too: logs in the
   **old** format (including the tool's own fixture,
   `tools/perf/fixtures/vram/film-run-tail.log`, whose
   expectations are pinned in the tool's `EXPECTED` table)
   still match the same regex, because the change is purely
   additive at the tail. `vram_report.py --self-test` needs
   **no edit** and must still pass unchanged — that is part of
   the landing's evidence (§4).
4. The only other consumer of the line in `tools/` is
   `tools/perf/lab/bench_lab.py`, whose grammar (line 191) is
   `PAINT_CACHE_RE = re.compile(r"paint-cache:.*$")` — it
   captures the line **verbatim** as an opaque string, so the
   appended fields simply ride along in its records; its
   synthetic fixture `FAKE_PAINT_CACHE` (old format) still
   matches. No other anchored parser of this line exists in
   `tools/` or `src/` (grep-verified for `paint-cache` /
   `budget_bytes`). The in-binary prints
   (`film.rs` `cache.stats_line()` at the completion and
   telemetry sites) print the string whole; nothing splits it.

Conclusion: appending at the END is safe **because** the
prefix is untouched and both real parsers are prefix- or
verbatim-based. Appending in the middle, renaming a field, or
changing a separator would break parser (1) — the spec
forbids all three.

### 2.3 The format-stability test expectation (updated)

`stats_line_format_is_stable` (`paintbudget.rs:407-421`)
builds: budget 1000, cap 160; miss `a`, insert 100; hit, hit;
miss `b`, insert 250; evict `a` (100); miss `a` (a rerender).
Counters at assertion time: entries 1, bytes 250, hits 2,
misses 3, evictions 1, rerenders 1, peak 350, budget 1000.

Under this spec the eviction call gains an explicit reason
(Patch 2); the scenario's eviction is synthetic (see §3.4),
and the test should pass `EvictReason::EntryCap`. Derived
rate: `2 × 1000 / (2 + 3) = 400`. The updated expectation is
therefore exactly:

```rust
assert_eq!(
    b.stats_line(),
    "paint-cache: entries=1 bytes=250 hits=2 misses=3 evictions=1 rerenders=1 peak_bytes=350 budget_bytes=1000 hit_rate_permille=400 evictions_byte=0 evictions_cap=1"
);
```

No other existing assertion in the file changes its expected
*values*; the six `on_evict` call sites change only by the
added reason argument (§3.4).

---

## 3. Patches 2 + 3 — the eviction-reason taxonomy

### 3.1 The contradiction, stated plainly

The charter asks for "eviction-reason tracking (byte-budget
vs entry-cap)". The code as it stands **cannot supply it**:
`should_evict()` folds both causes into one `bool`, and the
film loop consumes that bool without recording which clause
fired. Any spec that pretended the reason could be recovered
after the fact (e.g. by re-deriving it in `stats_line`, or by
diffing counters) would be guessing — by end of run the state
that caused an early eviction is gone. The reason must be
captured **at eviction time, from the pre-eviction state**,
which is exactly what Patches 2–3 do. This is the minimal
API addition the code actually supports; nothing larger is
needed.

### 3.2 Taxonomy and precedence

Two reasons, disjoint, exhaustive over every eviction the
loop can perform:

| Reason | Meaning | Predicate (pre-eviction state) |
|---|---|---|
| `EntryCap` | the cache holds more entries than the entry cap | `entries > entry_cap` |
| `ByteBudget` | within the entry cap, but resident bytes exceed the byte budget | `budget_bytes > 0 && bytes > budget_bytes && entries > 1` |

- **Precedence: `EntryCap` wins when both predicates hold.**
  Rationale: (i) the cap clause is the legacy, exact clause
  and carries no live-entry guard — when the count is over
  cap the cache is over its hard bound regardless of bytes;
  (ii) it makes `ByteBudget` mean the operationally
  interesting thing — "evicted *purely* for bytes while the
  count was legal" — which is the WP1.1 signal (with the
  derived default budget this is the common production case:
  bytes bind at ~74 entries, cap is 160); (iii) disjointness
  gives the checkable invariant
  `evictions_byte + evictions_cap == evictions`.
  The rejected alternative — counting both causes for one
  eviction — breaks that invariant and makes the fields
  non-additive for every downstream reader.
- The byte predicate **keeps the live-entry guard**
  (`entries > 1`) verbatim from today's `should_evict`: a
  single entry larger than the whole budget is never
  byte-evicted (it would thrash without saving live memory).
  Consequently a lone oversize entry yields **no** reason and
  no eviction — same behaviour as today.
- With `budget_bytes == 0` (byte enforcement disabled), the
  byte predicate is always false and **every** eviction is
  `EntryCap` — the legacy behaviour, correctly labelled.

### 3.3 Where each reason is counted (Patch 2, `src/paintbudget.rs`)

All counting lives in `PaintBudget` — the module that already
owns every other paint-cache counter. `film.rs` only *asks*
for the reason and *hands it back*; it keeps no counters of
its own. Changes:

1. New type, near the top of `paintbudget.rs` (after the
   constants):

   ```rust
   /// Why one eviction happened (WP1.1). Captured at eviction
   /// time from the pre-eviction state; see `eviction_reason`.
   #[derive(Clone, Copy, PartialEq, Eq, Debug)]
   pub(crate) enum EvictReason {
       ByteBudget,
       EntryCap,
   }
   ```

2. Two new fields beside `evictions` (line 113),
   initialised to `0` in `new` (beside line 135):

   ```rust
   evictions_byte: u64,
   evictions_cap: u64,
   ```

3. `should_evict` is **reimplemented in terms of** a new
   `eviction_reason`, so the loop condition and the recorded
   reason can never drift apart (today's body moves, verbatim,
   into the two predicates):

   ```rust
   /// The reason the cache must evict now, or None. Cap takes
   /// precedence when both bounds are exceeded (V2-WP1-LRU-SPEC
   /// §3.2). `should_evict()` is exactly `eviction_reason().is_some()`.
   pub(crate) fn eviction_reason(&self) -> Option<EvictReason> {
       if self.entries > self.entry_cap {
           Some(EvictReason::EntryCap)
       } else if self.budget_bytes > 0 && self.bytes > self.budget_bytes && self.entries > 1 {
           Some(EvictReason::ByteBudget)
       } else {
           None
       }
   }

   pub(crate) fn should_evict(&self) -> bool {
       self.eviction_reason().is_some()
   }
   ```

   The existing `should_evict` doc comment (the two deliberate
   semantics: cap checked exactly as before; byte condition
   never evicts the last entry) moves onto `eviction_reason`
   with it — those semantics are unchanged, only relocated.
   Every existing `should_evict` unit test must pass
   unmodified; they pin this equivalence.
4. `on_evict` gains the reason and counts it
   (`paintbudget.rs:196`):

   ```rust
   pub(crate) fn on_evict(&mut self, key: &str, bytes: u64, reason: EvictReason) {
       self.entries = self.entries.saturating_sub(1);
       self.bytes = self.bytes.saturating_sub(bytes);
       self.evictions += 1;
       match reason {
           EvictReason::ByteBudget => self.evictions_byte += 1,
           EvictReason::EntryCap => self.evictions_cap += 1,
       }
       self.evicted_keys.insert(key.to_string());
   }
   ```

   Call-contract doc (line ~102) updated per §1.2.
5. Getters beside the existing ones (`evictions()` etc.):
   `evictions_byte()` and `evictions_cap()` returning the two
   fields — needed by the new tests and by any future reader;
   `stats_line` reads the fields directly, as it already does
   for the other counters.

Patch 3 is the film-loop half (`film.rs:2311-2315`). Current:

```rust
        while self.budget.should_evict() {
            let old = self.order.pop_front().expect("cache order");
            let old_v = self.map.remove(&old).expect("cache order");
            self.budget.on_evict(&old, crate::paintbudget::entry_bytes(&old_v.0));
        }
```

Replacement:

```rust
        while let Some(reason) = self.budget.eviction_reason() {
            let old = self.order.pop_front().expect("cache order");
            let old_v = self.map.remove(&old).expect("cache order");
            self.budget.on_evict(&old, crate::paintbudget::entry_bytes(&old_v.0), reason);
        }
```

The reason is read **before** the pop, from the same state
`should_evict()` would have seen — per iteration, so a run of
evictions that crosses from over-cap to over-bytes (or vice
versa) is attributed iteration-by-iteration, not once for the
whole drain. Which entry leaves (`pop_front`, the LRU end) is
unchanged. Both `expect("cache order")`s stay.

### 3.4 The six test call sites (signature-change fallout)

Each existing `on_evict` test call gains a reason. The
reasons below are chosen to match each scenario's own setup,
so the tests also become classification evidence:

| Test (paintbudget.rs) | Call today | Reason to pass | Why it matches the scenario |
|---|---|---|---|
| `accounting_tracks_bytes_entries_and_peak` (~280) | `on_evict("k1", 100)` | `EvictReason::EntryCap` | Synthetic accounting call; budget 0, no predicate is live — the test's subject is bytes/peak, and §3.5 covers why a synthetic reason is honest here. |
| `byte_budget_triggers_eviction_need` (~312) | `on_evict("k1", C)` | `EvictReason::ByteBudget` | Budget `2*C`, cap 160, 3 entries resident: cap predicate false, byte predicate true. |
| `single_oversize_entry_is_never_self_evicted` (~327) | `on_evict("k1", C)` | `EvictReason::ByteBudget` | Budget `C/2`, 2 entries of `C`: within cap, over bytes, `entries > 1`. |
| `zero_budget_disables_bytes_but_not_entry_cap` (~352) | `on_evict("k1", C)` | `EvictReason::EntryCap` | Budget 0 (byte clause dead), 3 entries vs cap 2. |
| `rerenders_counted_only_after_eviction` (~373) | `on_evict("walk-a", C)` | `EvictReason::EntryCap` | Budget 0, cap 1. |
| `stats_line_format_is_stable` (~415) | `on_evict("a", 100)` | `EvictReason::EntryCap` | Drives the §2.3 expectation (`evictions_cap=1`). |

New tests to add in the same landing (in-module, `paintbudget.rs`):

- `eviction_reason_classification`: byte-only → `ByteBudget`;
  cap-only → `EntryCap`; both (e.g. cap 1, budget `C/2`, two
  canvases) → `EntryCap` (precedence); neither → `None`;
  budget 0 → never `ByteBudget`; single oversize entry →
  `None` (live-entry guard survives the refactor).
- `eviction_reason_matches_should_evict`: across a small
  table of (budget, cap, inserts) states,
  `eviction_reason().is_some() == should_evict()` — the
  anti-drift pin.
- `eviction_counters_sum_to_evictions`: mixed sequence of
  byte and cap evictions; assert both counters and the sum
  invariant.
- `hit_rate_permille_edges`: no lookups → 0; all hits → 1000;
  2 hits / 5 lookups → 400; all misses → 0 (and distinguish
  from the no-lookup sentinel only via hits/misses, as
  documented).

### 3.5 One honest wart, disclosed

Two tests above pass a reason for an eviction that
`eviction_reason()` would not have produced in that synthetic
state (the accounting test and, partly, the stats-line test:
budget 1000 vs 350 resident bytes, cap 160 vs 2 entries — no
predicate holds). That is inherent to unit-testing
`on_evict` in isolation: it is a *reporting* entry point, and
the production caller (Patch 3) can only ever pass a reason
that `eviction_reason()` just returned. The explicit-parameter
design was chosen over the alternative — classifying *inside*
`on_evict` from its own state — precisely because the
internal version would have to invent a fallback bucket for
exactly these calls, silently mislabelling synthetic (and any
future manual-invalidation) evictions. With the parameter,
the label is always the caller's stated cause, and in
production the caller is the loop that just asked. If a
future manual invalidation path appears, it should get its
own `EvictReason` variant and its own stats field (appended,
per §2.1) rather than borrowing one of these two.

---

## 4. Pixel-neutrality — by construction, and the proof protocol

### 4.1 Why no frame byte can change

1. **Paint is a pure function of the exact key.** `film.rs`'s
   own doc (lines 2270–2272) and `paintbudget.rs`'s header
   state it, and `state_key` (`film.rs:2323…`) makes it
   precise: the key is built from **f32 bit patterns**
   (`v.to_bits()`, since `cabcc30`) plus the discrete fields —
   key equality is state equality for everything the painter
   reads. Hence for every key, `paint_toon_full` returns the
   same `Painted` bytes on every call, cache or no cache.
2. **The hit path returns the same value under both
   policies.** A hit returns an `Rc` clone of the entry stored
   under the exact key — i.e. bytes produced by (1) for that
   identical state. Patch 1 changes only the *position of the
   key string* inside `order: VecDeque<String>`. It does not
   touch `map`, does not touch any `Painted`, and cannot
   change which `Rc` a hit returns.
3. **Eviction changes only repaint frequency.** LRU and FIFO
   differ in *which* keys are resident at a given lookup, so a
   lookup that hit under FIFO may miss under LRU (and vice
   versa). On a miss the entry is repainted — and by (1) the
   repaint is byte-identical to the evicted entry. Induction
   over the lookup sequence: under both policies, every
   `get_or_paint` call returns `paint(exact state)`. The
   frames consume only those returned values. ∎
4. **The stats changes are log-only.** `eviction_reason`,
   the two counters and `hit_rate_permille` feed exactly one
   sink: the `stats_line` string. No return value from
   `PaintBudget` reaches a pixel, a verdict or a gate — the
   same argument `runlog.rs`'s header makes for its own
   numbers. (The loop-condition refactor, `should_evict` →
   `eviction_reason().is_some()`, is behaviour-identical by
   the §3.4 anti-drift test: same predicates, same order of
   evaluation, same guard.)
5. **The caveat that makes (1) load-bearing — stated, not
   buried.** This argument *fails* for approximate keys: with
   the pre-`cabcc30` quantised keys, two different states
   shared a key, a hit returned the *representative's* paint,
   and eviction order therefore **did** select pixels (the
   measured 72.34 dB chunk-boundary divergence at frames
   1195–1199). Exact keys are what demote eviction policy to a
   pure performance choice. This landing does not touch
   `state_key`; if it ever did, this section's proof would
   have to be re-derived, not assumed.

Expected *non-pixel* differences after landing, so nobody
misreads them as regressions: `hits`/`misses` split for a
given film may shift (that is the point of LRU — recurring
character states, e.g. walk-cycle phases, should hit more),
`rerenders` should fall or hold, eviction counts may move,
and wall-clock may improve. The lookup *sequence* is fixed by
the film, so `hits + misses` for a run is invariant across
the two policies and is a cheap sanity check in the landing
evidence.

### 4.2 Proof protocol (the landing's gates — contracts §4, nothing deferred)

All of the following, on the landing commit, reported as
they land:

1. **CI build SUCCESS** + **full test suite green**,
   including the updated `stats_line_format_is_stable`, the
   four new `paintbudget` tests (§3.4), and every pre-existing
   `should_evict` test passing unmodified. (A failure caused
   solely by the artifact-quota upload step is reported as
   such, per contracts §4, and does not block.)
2. **Determinism byte probes 40/40** against the §3 baseline
   (`~/workspace/talerender-evidence/hashes-post.txt`, ranges
   195–204, 795–814, 1195–1204). Range 1195–1204 is the
   sensitive one for this change: it straddles the old
   paint-cache cap boundary, so it is where an eviction-order
   bug would surface first. **Any differing byte stops the
   landing** and is reported as a user decision (master
   prompt §2/§5.1 hard law) — never re-pinned over.
3. **FILM TELEMETRY: CHECK OK** (probe workflow).
4. **Golden verify 17 OK** — local-only act (pins and images
   never go to GitHub; hashes/metrics as text), against the
   17 surviving pins (s28 retired 2026-11-15).
5. **`tools/perf/vram_report.py --self-test` PASS, unedited**,
   plus one parse of a post-landing log line through the same
   tool showing the eight original fields extracted with the
   same values — the §2.2 proof, executed rather than argued.
6. **Stats-line before/after** for one telemetry run in the
   landing note: old-format line, new-format line, and the
   `hits + misses` invariance check from §4.1.

---

## 5. Landing order for the lead (all five patches, one landing)

1. **Patch 2** — `paintbudget.rs`: `EvictReason`, the two
   counters + getters, `eviction_reason`, `should_evict`
   reimplemented on it, `on_evict` signature, call-contract
   doc bullet. Update the six test call sites (§3.4) and add
   the classification / anti-drift / counter-sum tests.
2. **Patch 4** — `paintbudget.rs`: `hit_rate_permille`
   helper, `stats_line` tail extension + doc comment, the
   §2.3 expectation update, `hit_rate_permille_edges` test.
   (Lands with Patch 2 in the same commit sequence; ordered
   second only so a bisect lands on a compiling tree at every
   step — Patch 2 alone compiles once its test call sites are
   updated, Patch 4 alone would too, but the stats line should
   never be observable with reason fields whose counters do
   not yet exist.)
3. **Patch 3** — `film.rs` eviction loop head (§3.3).
4. **Patch 1** — `film.rs` hit-path LRU touch + the
   `FIFO cap` → `LRU cap` doc line (§1). Last, deliberately:
   it is the only patch that changes runtime *behaviour*
   (residency), and landing it after the accounting means the
   very first LRU run already reports its reasons and hit
   rate.
5. **Patch 5** — run §4.2's protocol; record the verdicts in
   the landing note. No other files change. In particular:
   `vram_report.py`, `bench_lab.py`, fixtures and
   `runlog.rs` are **not** edited — §2.2 is the reason they
   do not need to be.

Out of scope, explicitly: any change to `state_key`, to
`paint_toon_full`, to the budget derivation or the 2 GiB
ceiling, to the entry-cap default, or to `admit()` (unused by
the primary wiring today; left exactly as is). WP1.4's code
half is `src/lifetimes.rs` (separate deliverable); nothing in
this spec wires it — its delivery note carries its own
anchors.
