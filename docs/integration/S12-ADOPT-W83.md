# S12-ADOPT-W83 — s12 bake adoption readiness (re-verified at HEAD)

**Seat:** TaleRender Wave 2, W83 (Batch 9).
**Repo HEAD at verification:** `021ca01` (Batch-8 close). The working
tree carried the lead's unrelated in-flight edits (`src/ab1.rs`
staged, `WAVE2-RECORD.md` modified); every file this note cites —
`src/film.rs`, `scenes/film/s12.scene.json`, the staging pack — was
clean at HEAD.
**Method:** first-hand re-runs only. `bake_s12.py` writes beside
itself, so it was run from a scratch mirror of the repo layout
(`/tmp/w83/…`, unmodified script, current source file); every other
tool was run read-only against the repo with outputs in `/tmp`.
Nothing in the repo was edited; this note is the only new file.

## Verdict up front

**READY — no drift since W52.** The whole proof chain reproduces
at `021ca01` exactly as recorded at `aa92cee`: the drift guard
passes against the current source and regenerates the staged file
byte-identically; the equivalence check returns FAITHFUL with the
same numbers; the original s12 still fails conversion naming the
same two records (the baseline the adoption removes); the baked
copy converts PASS 4/4 and validates PASS 5/5; the full census
re-runs 13/13 on the other files, so with the bake adopted the
standing totals remain **14/14 files, 65/65 key-carrying records**.
The adoption commit's shape, retirement target, and verification
legs are specified in §2; the frozen-factor risk and its
documentation are in §3.

## 1. Proof chain, re-run at `021ca01`

### 1.1 Drift guard — PASS, output byte-identical

`bake_s12.py` verifies the two `apply` records in
`scenes/film/s12.scene.json` against its recorded originals (keys +
`apply` text, census of exactly two) and refuses without writing
on any drift. Run against the current source:

```
baked lean     x 0.6  -> keys [[0, 0], [2.5, 2.1, 0.45]] (f32 arrival: 2.0999999046325684)
baked headTilt x -1.0 -> keys [[0, 0], [2.5, -3.5, 0.45]] (f32 arrival: -3.5)
BAKE: OK — wrote /tmp/w83/tools/format/staging/s12-baked/s12.scene.json
exit 0
```

The regenerated file is **byte-identical** to the committed
staging (`tools/format/staging/s12-baked/s12.scene.json`, `cmp`
clean). Corroborating: `git log aa92cee..HEAD` over the source
file, the staging pack, and both format tools shows only
`3a3a427` (the Batch-6 central commit that landed the pack itself)
— the source records have not moved since the bake was recorded.
The staged-vs-live diff is still exactly W52's 10 lines: two key
values (3.5 → 2.1, 3.5 → −3.5), two `apply` fields removed, two
`law` fields rewritten with bake provenance.

### 1.2 Equivalence — FAITHFUL, same numbers

`equivalence_check.py` (reads the committed staged file; writes
only to a tempfile) exits 0:

```
VERDICT: FAITHFUL — (a) key-time f32 bit-identity (modulo sign-of-zero): True;
         (b) bake's own contribution <= 1 ulp everywhere: True;
         (c) baked total within the conversion envelope + 1 ulp: True
```

The sampled tables match W52 §5 line for line: key times bit-exact
(lean arrival 2.0999999046325684, bits 0x40066666, both routes);
A−B ≤ 1 ulp everywhere (max 2.98e-8, lean t = 2.2); worst total
deviation 3.576e-7 (headTilt t = 2.2), all of it the §5.2/§11
conversion cost the other 63 records already pay. The one
disclosed imperfection stands as recorded: headTilt key 1 spells
`0` where the code's negation yields −0.0 (== under every
comparison; no consumer reads the sign of an exact zero).

### 1.3 Conversion + validation — baseline and baked, both as recorded

Original file (the pre-adoption state, unchanged):

```
stage_clips.py — scenes/film/s12.scene.json (2 clips, 4 tracks, 8 keys staged; 2 records code-only)
[2 channels] FAIL — 2 failure(s)
    nodes[Onoda (JG onoda2)].channels.lean: record carries an 'apply' transform ('lean = bowA * 0.6') …
    nodes[Onoda (JG onoda2)].channels.headTilt: record carries an 'apply' transform ('headTilt = -bowA') …
OVERALL: FAIL (3/4 checks passed) — no output written     (exit 1)
```

Baked copy:

```
stage_clips.py — tools/format/staging/s12-baked/s12.scene.json (2 clips, 6 tracks, 14 keys staged; 2 records code-only)
OVERALL: PASS (4/4 checks passed)                          (exit 0)
check_clips.py: [1 structure] PASS [2 channels] PASS [3 keys] PASS
                [4 references] PASS [5 f32] PASS
OVERALL: PASS (5/5 checks passed)                          (exit 0)
```

### 1.4 Full census re-run

`stage_clips.py` + `check_clips.py` over the other 13 scene files
at HEAD: **13/13 convert PASS, validator 5/5 each** (s01–s11,
s05a, s05b, s08b). With baked s12 substituted: **14/14 files,
65/65 key-carrying records** — W52's closing census reproduces
unchanged. s12's remaining records (mood step, camera, the two
§5.3 code-only blink directors) convert in the same run, as
before.

## 2. The adoption commit — exact shape

W52's law: the baked data lands in the **same commit** that
retires the code path; a baked copy and an unbaked original must
never coexist as parallel sources of truth.

### 2.1 Data side

- **Target file: `scenes/film/s12.scene.json`** — the only s12
  scene file in the repo (`find scenes -name "*s12*"` returns
  exactly this path; it is the manifest copy the tools read).
- **Change: exactly the staged transform** — adopt
  `tools/format/staging/s12-baked/s12.scene.json` over it, or apply
  the equivalent 10-line diff (2 key values, 2 `apply` removals,
  2 `law` rewrites; §1.1). The staged copy's byte format is the
  live file's own (1-space indent, no trailing newline; W52
  verified a load/dump round-trip of the original is
  byte-identical), so adopting the copy whole changes nothing
  outside the two records.

### 2.2 Code side — what retires in `src/film.rs`

`fn s12` (film.rs:2003–2095). The entire `apply` evaluation is
three lines:

```rust
// film.rs:2021
let bow = pose_track(&[key(0.0, 0.0), keym(w_loyal - st0 - 0.1, 3.5, 0.45)], lt);
// film.rs:2022
onoda.lean = (0.6 * bow) as f32;
// film.rs:2023
onoda.head_tilt = (-bow) as f32;
```

`bow` has **no other consumer** in the function (grep of the full
fn body: the definition + these two uses only), so retiring
2021–2023 removes the whole computation. `w_loyal`
(`data.anchors["loyal11"]` = 58.12) stays — it also drives
`onoda.mood` (film.rs:2027) and `blink_of` (film.rs:2028), which
are not part of the bake. In the adopted state, `lean`/`head_tilt`
for this shot come from the baked clip channels (the §15 lane the
staging feeds) instead of the in-code track evaluation — the seam
itself is the §15 adoption's, per W52 §6; this pack certifies the
data and names the lines.

### 2.3 Everything else that references the `apply` form (census)

Grepped at HEAD over `scenes/`, `src/`, `tools/`, `docs/`:

| where | what | adoption consequence |
|---|---|---|
| `scenes/film/s12.scene.json` (lines 10541, 10557) | the two `apply` records themselves | removed by §2.1; **only occurrence in any scene file** (corpus-wide, as W52 verified) |
| `tools/format/stage_clips.py` | the rejecter: check 2 names any record carrying `apply` | no change; post-adoption it simply never fires on this corpus |
| `tools/format/staging/s12-baked/bake_s12.py` | the regenerator: reads the records, guards drift | see §3 — its role ends at adoption |
| `docs/integration/ANIM-ADOPT-W32.md` (§4, census table) | the historical record of the block ("FAIL (apply ×2)", "Blocked: 2 records") | historical; the adoption commit should note the status change (a pointer line or the commit message — §3) |
| `docs/integration/S12-BAKE-W52.md` + this note | the proof + readiness record | none |
| `src/**` | **no consumer**: no `"apply"` field handling anywhere in the Rust tree (`apply_channels`/`apply_gaze`/`apply_fade` are unrelated identifiers) | nothing to retire outside film.rs:2021–2023 |
| `tools/format/check_format15.py` | no `apply` handling — the §15 conformance checker does not special-case it | none |
| `docs/scene-format.md`, FORMAT15 docs | "apply" hits are prefab-override vocabulary (`apply overrides by pid`), unrelated to poseTrack `apply` | none |

### 2.4 Post-adoption verification (for the adoption commit)

1. **Conversion census re-run** (the §1.4 commands against the
   live tree): the live `scenes/film/s12.scene.json` now converts
   PASS 4/4, its clips validate 5/5, census stands 14/14, 65/65.
2. **Telemetry:** `FILM TELEMETRY: CHECK OK`, exit 0 — the
   standing film-lane gate must still pass after the commit.
3. **Byte probes on s12's rendered range.** From
   `scenes/film/film.json`: s12 = **frames 1666–1811**
   (startFrame 1666, endFrame 1811, 145 frames; 55.52–60.357 s).
   The standing probe sets do **not** cover it — `byte_probes.py`
   defaults are 195-204 / 795-814 / 1195-1204, and the E7
   determinism pack window is [100,130) — so the adoption needs
   explicit ranges against a pre-adoption render of the same
   frames:
   - `--ranges 1666-1810` (the full shot), and within it the bow
     move: lt 2.05–2.5 ⇒ film t 57.57–58.02 ⇒ **frames 1727–1741**
     (arrival key lt 2.5 at t 58.02, frame ≈1740.6).
   - Expectations, from the §1.2 arithmetic: hold frames
     (1666–1726, both paths exactly 0) and arrival/hold frames
     (≥1741, key values bit-exact) should be **byte-identical**;
     move frames (1727–1740) sit inside the conversion envelope
     (channel deviation ≤ 3.58e-7) — judge them at the house
     80 dB band and report any byte differences with their PSNR
     rather than assuming identity.
4. **Determinism spot range:** no overlap between s12
   (1666–1811) and any standing determinism probe window (above),
   so no determinism re-run is owed for this adoption beyond leg
   3's same-frames comparison; if the lead wants a spot range
   anyway, 1727–1741 (the move) is the sensitive window.

## 3. Risk note — the frozen factors

Baking trades a live expression for constants. Post-adoption, the
scene file no longer says "0.6 × however the bow track evolves";
it says **2.1** and **−3.5**. If the S12 staging ever changes —
a deeper bow (arrival value ≠ 3.5), a moved anchor (`loyal11`,
hence a different key time), or a factor change (0.6 → anything)
— **the baked data does not follow**. It silently describes the
old bow until someone re-derives the keys. That is the honest
cost W52 recorded; it is accepted because the factors are two
constants, the stored values are the code's own f32 results at
the keys, and the equivalence envelope is already paid by every
other converted record.

Where a future editor actually sees the freeze:

- **In the data itself (primary).** Each baked record's `law`
  field opens with the provenance: *"BAKED (W52): the former
  apply 'lean = bowA * 0.6' is multiplied through these keys
  (film.rs S12: onoda.lean = (0.6 * bow) as f32); pre-bake law:
  …"* — factor, source expression, and the pre-bake law verbatim.
  Anyone editing those keys meets the dependency in the file,
  not just in a side document.
- **In `tools/format/staging/s12-baked/` (the pack).** `bake_s12.py`'s
  `FACTORS` table holds the same factors beside the film.rs lines
  they mirror; `equivalence_check.py` re-proves any re-bake;
  S12-BAKE-W52.md is the full arithmetic.
- **In this note and the adoption commit** (below).

One operational consequence the adoption must make discoverable:
**the regenerator's guard ends its role at adoption.**
`bake_s12.py` reads the live `scenes/film/s12.scene.json`
expecting the two `apply` records; on the adopted file it refuses
with "apply-record census changed" (exit 1, nothing written).
That refusal is correct behaviour — its guarded source no longer
exists — but it means post-adoption regeneration is *not* "run
the bake script". The paths back, in order:

1. Edit the baked keys directly for a pure key change, keeping
   the factors applied by hand (the `law` fields carry them), and
   re-run `equivalence_check.py`'s method against the code path
   if film.rs still evaluates one — after §2.2's retirement it
   does not, so the data is then the sole source and is edited
   as data.
2. For a factor or track change, restore the pre-bake records
   from git (the pre-adoption `s12.scene.json` is in history at
   the adoption commit's parent), update `FACTORS` in
   `bake_s12.py`, and re-bake — the guard then passes against the
   restored source and the whole §1 chain re-runs.

**Documentation the adoption commit should leave behind:**

- The in-file `law` provenance (already in the staged data —
  §2.1 carries it into the live file unchanged). No extra data
  annotation is needed.
- A commit message that names the freeze explicitly: the two
  factors, the W52 pack as the proof of record, and the fact that
  film.rs:2021–2023 retired in the same commit (the atomicity
  law). A future `git log` reader should reach the dependency
  from either side — data or code.
- A status pointer where the block was recorded:
  ANIM-ADOPT-W32.md's census lists s12 as "Blocked: 2 records
  (apply ×2)" — the adoption should add the one-line resolution
  pointer there (or in the wave record's fold-in) so the census
  of record doesn't keep reporting a closed block as open.
- Keep the whole staging pack (`bake_s12.py`,
  `equivalence_check.py`, the W52 note) in the tree: post-adoption
  it is the audit trail and the re-bake path (item 2 above), not
  dead weight.

Residual risks beyond the freeze itself: none found in this
re-verification. The sign-of-zero spelling (headTilt key 1) is
inherited from the source record and disclosed in W52 §3; the
conversion envelope (§1.2) is corpus-standard. The precedent
boundary also stands: this readiness covers records carrying
their **own** pre-transform keys plus a scalar `apply` — the only
such records in the corpus. Derived channels (e.g. s11's headTilt,
defined as −0.8 × another record's track, no keys of its own)
stay code under §5.3 and are untouched by this adoption.

## 4. Summary for the lead

- Proof chain at `021ca01`: **all green, zero drift** — guard
  PASS + byte-identical regeneration (§1.1), FAITHFUL (§1.2),
  baseline FAIL intact / baked PASS 4/4 + 5/5 (§1.3), census
  14/14 + 65/65 reproducible (§1.4).
- Adoption commit = `scenes/film/s12.scene.json` takes the staged
  transform (10 lines) **and** film.rs:2021–2023 (fn s12's `bow`
  evaluation + the two factor lines) retires, same commit (§2.1,
  §2.2). Nothing else in the repo consumes the `apply` form
  (§2.3).
- Verify the adoption with: census re-run, telemetry CHECK OK,
  byte probes `--ranges 1666-1810` against pre-adoption frames
  (move window 1727–1741; standing probe sets don't cover s12)
  (§2.4).
- Leave behind: the in-file `law` provenance (automatic), a
  freeze-naming commit message, and a resolution pointer at
  ANIM-ADOPT-W32.md's blocked census (§3).
