# FILMJSON-W41 — F1 verification (film.json entry) + F2 asset audit

Seat W41, Wave 2 Batch 5 (gap-fix batch). Verified at HEAD `f6616aa`
(Batch 4 central commit; `src/film.rs` as read at this head). F1 itself
landed in `aadc193` (lead, gpucore site 5/5 commit). Method: source read
first-hand; local binary runs in §2.3 are era evidence only (see the era
bracket there). Nothing in `src/` was edited; the only other files this
seat wrote are the fixtures in §2.1.

---

## 1. F1 verification — entry-name resolution in `load_film_data`

### 1.1 The resolution code (src/film.rs:63–75, verbatim)

```rust
pub(crate) fn load_film_data(dir: &Path) -> Result<FilmData, String> {
    // F1 (gap-fix packet): film.json is the format's generic
    // entry name; onoda.json is this film's historical data file
    // and stays the fallback, so the existing film data and the
    // bridge packages load unchanged. --help says the same.
    let path = if dir.join("film.json").exists() {
        dir.join("film.json")
    } else {
        dir.join("onoda.json")
    };
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read film data: {e}"))?;
    let root = crate::scene::parse_json(&text)?;
```

**Resolution order, exactly:** (1) if `<DIR>/film.json` exists → read it;
(2) else → read `<DIR>/onoda.json`. There is no merge and no warning
either way. `--help` now says the same thing (src/main.rs:63–68:
"film.json — the generic entry; onoda.json accepted for this film").

### 1.2 Case analysis

| Case | Behaviour | Evidence |
|---|---|---|
| Only `film.json` | Read. Generic projects load. | film.rs:68–70 |
| Only `onoda.json` | Read (fallback). Existing film data + bridge packages unchanged. | film.rs:70–72 |
| **Both exist** | **`film.json` wins, unconditionally.** `onoda.json` is never opened; no diagnostic. | film.rs:68 (`if/else` on existence, chosen before any read) |
| **Neither exists** | Path resolves to `onoda.json`; the read fails and the error is terminal: `cannot read film data: No such file or directory (os error 2)`. The message names **neither file** — it does not say "film.json missing", and in `--film` mode it does not even name the file that was tried (only the directory, via the `TaleError` path — §1.3). | film.rs:71–74 |
| `film.json` exists but unreadable / is a directory | `exists()` is true, so it is chosen; `read_to_string` fails (`Is a directory (os error 21)` etc.) and the error is terminal — **no fallback to `onoda.json`**. The choice is made on existence, before the read. | film.rs:68–74 |
| `film.json` is a broken symlink | `Path::exists()` follows symlinks and returns false → falls back to `onoda.json`. | `exists()` semantics at film.rs:68 |

After a successful read, key demands run in this order — `anchors`,
`lines`, `captions`, then `duration` (film.rs:77–108) — each missing key
fails with `film data: missing {k}` (film.rs:77). JSON parse errors
propagate unwrapped from `scene::parse_json` (film.rs:75).

### 1.3 Exit paths — the two modes diverge

`load_film_data` itself returns `Result<FilmData, String>`; the wrapping
differs by caller:

**`--film` (render):** `run_film` (film.rs:3401) wraps the error:
`load_film_data(dir).map_err(|e| TaleError::data_load(dir, e))?`.
`main` dispatches through `exit_on_err` (main.rs:286, 156–161), which
prints `error: {e}` to stderr and exits with `e.exit_code()`.
`DataLoad` → **exit 2** (errors.rs:183); its Display with a path is
`failed to load {path}: {source}` (errors.rs:212–215). So the
neither-file case prints, exactly:

```
error: failed to load <DIR>: cannot read film data: No such file or directory (os error 2)
```

**`--film-telemetry`:** `run_film_telemetry` (film.rs:3525) calls the
**same** `load_film_data` with a plain `?` — the entry logic (names,
order, both/neither behaviour) is identical, there is no second
implementation. `main` (main.rs:272–281) maps `Err(e)` to
`eprintln!("film telemetry failed: {e}")` + **exit 3** (could-not-run,
EXIT-CODES-W02 contract; the verdict line is untouched by load errors
because no verdict is reached). Neither-file case prints, exactly:

```
film telemetry failed: cannot read film data: No such file or directory (os error 2)
```

Note the telemetry message carries no directory context at all.

**Verdict: F1 is correctly landed at HEAD** — generic name first,
historical fallback, shared by both modes, errors on the established
exit-code contract (2 render / 3 telemetry), no panics on this path.

---

## 2. Acceptance tests for CI

### 2.1 Fixtures (this seat, new files)

```
tools/film/fixtures/entry/filmjson/film.json    — film.json layout
tools/film/fixtures/entry/onodajson/onoda.json  — onoda.json layout
```

Byte-identical content in both: exactly the root keys the loader
consumes, shaped like the real `film/onoda.json` — `duration` (2.0),
`anchors` (1 entry), `lines` (1 × `{i, start, end, text}`), `captions`
(1 × `{line, s, e, text, width, fit, strip}`). (The real file also
carries `film`/`fps`/`frames`/`project`, which the loader ignores —
F3 surface, deliberately not mirrored.) Neither fixture ships an
`assets/` dir on purpose: a successful data load is proven by the
failure moving to the *asset* stage (`plates.json: …`), which is the
crisp, GPU-free signal — both modes load all assets before any frame
or verdict work (film.rs:3401–3402, 3525–3527), so these tests need no
GPU and can run in any post-build CI step.

### 2.2 The tests (expected results derived from HEAD source — CI-PENDING, §2.3)

Run from the repo root; `BIN` = the CI-built binary at the tested head.

```bash
# A. film.json layout is accepted as the entry (render mode)
$BIN --film tools/film/fixtures/entry/filmjson --out /tmp/w41-a
#   stderr: error: failed to load tools/film/fixtures/entry/filmjson: plates.json: No such file or directory (os error 2)
#   exit: 2        (data load SUCCEEDED — failure is at the asset stage)

# B. onoda.json fallback still accepted (render mode)
$BIN --film tools/film/fixtures/entry/onodajson --out /tmp/w41-b
#   stderr: error: failed to load tools/film/fixtures/entry/onodajson: plates.json: No such file or directory (os error 2)
#   exit: 2

# C. Neither file present (the fixtures' parent holds neither name)
$BIN --film tools/film/fixtures/entry --out /tmp/w41-c
#   stderr: error: failed to load tools/film/fixtures/entry: cannot read film data: No such file or directory (os error 2)
#   exit: 2

# D. Telemetry shares the entry logic — film.json layout
$BIN --film-telemetry tools/film/fixtures/entry/filmjson
#   stderr: film telemetry failed: plates.json: No such file or directory (os error 2)
#   exit: 3

# E. Telemetry — neither file
$BIN --film-telemetry tools/film/fixtures/entry
#   stderr: film telemetry failed: cannot read film data: No such file or directory (os error 2)
#   exit: 3

# F. Precedence discriminator: film.json present but key-incomplete,
#    onoda.json present and valid → film.json is the one read
D=$(mktemp -d); printf '{"duration": 1.0}' > "$D/film.json"
cp tools/film/fixtures/entry/onodajson/onoda.json "$D/onoda.json"
$BIN --film "$D" --out /tmp/w41-f
#   stderr: error: failed to load $D: film data: missing anchors
#   exit: 2        (had onoda.json been read, the error would be the plates stage)
```

### 2.3 Local binary — era bracket and first-hand results

`~/talerender-bin/talerender` (19,548,480 B, sha256 `bb1f5951…`,
mtime 2026-11-14 16:50Z) **predates F1** — behaviourally bracketed
between `aa10b13` and `ba88596`: telemetry already exits 3 on load
errors (post-`aa10b13`), but `--film` still **panics (exit 101)** on
load errors (pre-`ba88596` errors wiring) and knows no `film.json`.
So no local result can prove F1; §2.2 is **CI-pending** by design.
What the local runs *do* prove first-hand: the fixtures are
structurally valid (the onoda-layout fixture passes data load on the
old loader too), and the pre-F1 behaviour was exactly as the gap
analysis described. Verbatim:

```
--film filmjson fixture        → panic at src/film.rs:3456: "film data: cannot read film data: No such file or directory (os error 2)", exit 101   (film.json never tried)
--film onodajson fixture       → panic at src/film.rs:3457: "film assets: plates.json: No such file or directory (os error 2)", exit 101        (data load OK → asset stage)
--film-telemetry filmjson      → "film telemetry failed: cannot read film data: …", exit 3
--film-telemetry onodajson     → "film telemetry failed: plates.json: …", exit 3
--film fixtures/entry (neither)→ panic, "film data: cannot read film data: …", exit 101
```

---

## 3. F2 audit — every asset the film driver loads (src/film.rs at HEAD)

### 3.1 Completeness method

The film driver touches the filesystem in exactly three places:
`load_film_data` (film.rs:73, the entry file — §1), `load_png`
(film.rs:2048–2052, the only `image::open`), and `load_assets`
(film.rs:2054–2089, the only other `read_to_string`, film.rs:2056).
Every `Path::join` in the file is enumerated below — the list is
complete by construction, not by sampling. Both modes call the same
`load_assets` (film.rs:3402 render, film.rs:3527 telemetry), and it
runs **before** any rendering or audit work, so every load below is
**blocking**: a missing file stops the whole film/telemetry even if
the asset is never used on screen.

Not film-dir loads (checked, excluded): the PMREM room environment is
compile-time embedded (`include_bytes!` in src/composer.rs:103,
called from film.rs:2446) — not read from the film directory. The
`assets/caption.png`, `assets/room`, `toon.png`, `prims.png` strings
the gap analysis found belong to the ab1/battery lanes, not to
`--film`; film.rs reads no scene files either (the segment staging is
in-code; the `scenes/film/` clause in `--help` describes the project
layout, not a driver load).

### 3.2 The list

`adir` = `<DIR>/assets` (film.rs:2055). Classes: **U** = unconditional
(hardcoded name/set, loaded regardless of any data), **D** =
data-referenced (named by film data).

| # | Asset | Loaded at | Class | Notes / what SHOULD govern it |
|---|---|---|---|---|
| 1 | `assets/plates.json` | film.rs:2056 | **U** | Read first; its absence fails everything. Its *contents* only supply plate origins (see #2). In a generic film this file should BE the plate manifest: its key set should define which plates load. |
| 2 | `assets/paper-map.png` | film.rs:2060–2061 | **U** | Hardcoded name in a 5-name array. Origin `[x0,y0]` looked up in plates.json by name; a missing entry silently defaults to `(0, 0)`. Should be governed by the plate manifest / the staging's actual plate references (plates are consumed by name at film.rs:2224). |
| 3 | `assets/leaflet.png` | film.rs:2060–2061 | **U** | Same as #2. |
| 4 | `assets/book-open.png` | film.rs:2060–2061 | **U** | Same as #2. |
| 5 | `assets/book-closed.png` | film.rs:2060–2061 | **U** | Same as #2. |
| 6 | `assets/coconut.png` | film.rs:2060–2061 | **U** | Same as #2. |
| 7 | `assets/<captions[].strip>` (real film: `cap_000.png`…`cap_072.png`, 73 files) | film.rs:2073–2076 | **D** | The one correctly data-governed class: one PNG per caption chunk, named by the film data's `captions[].strip`. Caveat: `strip` defaults to `""` when the field is absent (film.rs:98), so a strip-less caption fails as `cannot load <DIR>/assets: …` — loud, but the message names no caption index. |
| 8 | `assets/calendar-1945.png` | film.rs:2078–2080 | **U** | One of a hardcoded year array `["1945", "1955", "1965", "1974"]`. The year set is a property of this film's calendar sequence; it should come from film data (a declared calendar list), not a source array. |
| 9 | `assets/calendar-1955.png` | film.rs:2078–2080 | **U** | Same as #8. |
| 10 | `assets/calendar-1965.png` | film.rs:2078–2080 | **U** | Same as #8. |
| 11 | `assets/calendar-1974.png` | film.rs:2078–2080 | **U** | Same as #8. |
| 12 | `assets/calendar-dark.png` | film.rs:2085 | **U** | Hardcoded single. The dark variant belongs with the calendar set — should be declared alongside #8–11 in data. |
| 13 | `assets/sign.png` | film.rs:2086 | **U** | Hardcoded single. Also load-bearing beyond assets: `Renderer::new` is seeded from `assets.sign` (film.rs:3406). A set/prop texture like the plates — should be declared by the data/manifest that uses it. |

### 3.3 Silent-fallback traps inside the unconditional loads

- **Non-object `plates.json` → zero plates, silently.** The plate
  loop runs only `if let Value::Obj(m) = &origins` (film.rs:2059). A
  plates.json of `[]` or `"x"` parses fine, loads **no** plate PNGs,
  and raises no error; staging plate lookups then just miss.
  (First flagged by W28; line confirmed at HEAD.)
- **Missing origin entry → `(0, 0)`, silently** (film.rs:2062–2070):
  the PNG still loads; only its placement defaults.
- **A 6th plate in plates.json is ignored**: keys are never enumerated
  — the manifest cannot extend the plate set today.

### 3.4 Cross-check against the gap analysis (rev 2)

Named by the analysis and confirmed in source: `paper-map.png` ✓,
`calendar-1945.png` ✓, `plates.json` ✓, `leaflet` ✓, `book-open` /
`book-closed` ✓, `coconut` ✓, `calendar-dark` ✓.

**Missed by the analysis:** `sign.png` (#13 — and it seeds the
renderer), the other three calendar years `calendar-1955/1965/1974.png`
(#9–11), the blocking gate role of `plates.json` itself (it is not
just demanded, everything behind it is), and the two silent fallbacks
in §3.3. The analysis's remaining film-mode strings (`caption.png`,
`room`, `toon.png`, `prims.png`) are **not** film-driver loads — §3.1.

**F2 work list in one line:** make `plates.json` the manifest it
already half-is (load its keys, not a hardcoded five), move the
calendar year set + `calendar-dark` + `sign` into film data, and keep
the caption strips as the model — they are the only loads a generic
film already governs by data.
