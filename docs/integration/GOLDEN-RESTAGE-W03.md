# Golden re-stage from the final head — W03 record

**Seat:** TaleRender Wave 2, W03 (golden re-stage). **Date:** 2026-11-14.
**Deliverables:** the candidate set `docs/golden/frames-f021991/`
(14 PNGs + `RENDERER.txt`) and this record. **Nothing was pinned.**
`make_golden.py --pin` was never run by this seat; the existing
staging in `docs/golden/frames/` is byte-for-byte untouched
(verified after staging: the tool still reads
`4 OK / 0 STALE / 14 MISSING`, current column = the 381ff47 hashes).

## 1. Where the f021991 frames came from (provenance, first-hand)

The frames were **on disk** — no CI download was needed, so artifact
expiry is moot (for the record: the f021991 Render dispatches ran
2026-11-14T09:45–46Z with the usual 7-day artifact retention, i.e.
expiring ≈2026-11-21; the route is named in §5 anyway).

The lead's working tree `~/workspace/film-final/` holds four full
1,811-frame sets with matching A/B tables. I mapped each set to its
head **by hash and by its own A/B table**, not by directory name:

| frame set | A/B overall (its own table) | identification |
|---|---|---|
| `frames/` | n=60, mean 21.44 dB | earliest iteration |
| `frames2/` | n=60, mean 21.84 dB | **381ff47** — its frame_00560/01593 hashes equal the current golden staging's exactly (and equal `.w77-scratch`, the staging's recorded source) |
| `frames3/` | n=60, mean 22.43 dB | dome-fix iteration |
| `frames4/` | n=60, min 19.36, **mean 23.02 dB** | **f021991** — mean/min match the lead's final-head report exactly, and `film-final/onoda-native.mp4` (finished from this set) is byte-identical to the delivered proof `onoda-native-f021991.mp4` (sha256 `dea7bb2c9c376ee9…`) |

Cross-checks on the old side: the 14 staged files in
`docs/golden/frames/` hash **exactly** to the table recorded in
`docs/ab/film/RESULTS.md` ("Golden mid frames (staged, NOT pinned)",
head `381ff47`) — all 14, full-length. So the comparison below is
381ff47-staged vs f021991-rendered, both sides verified.

The 14 mid frames are the same frame numbers as the current
staging (from `scenes/film/film.json` mids, per `RENDERER.txt`):
89, 285, 435, 560, 689, 760, 848, 969, 1119, 1231, 1306, 1440,
1593, 1738.

## 2. Old vs new — hashes and measured differences

PSNR/mean|Δ| are old-staged vs new-candidate, measured with the
house tool (`tools/ab/measure_ab.py --pair`, float64 MSE over all
channels) by this seat. "Cause" names the fix-chain commit(s) the
movement is consistent with, per the lead's completion report —
the A/B-vs-Chrome progression over the same chain was
21.84 → 22.43 → **23.02 dB**, i.e. every step moved *toward* the
reference; these native-vs-native numbers are the size of the
deliberate corrections, not drift.

| item | frame | sha256 @381ff47 (current staging) | sha256 @f021991 (candidate) | verdict | PSNR (dB) | mean \|Δ\| | named cause |
|---|---|---|---|---|---|---|---|
| film-s01 | 89 | `ebcfeba172be55b8ec8a8a56d2b50bef9a8641543d645fa8fd50106729cbc295` | `1608ead9a477764af9cf5958cfc4dfb78b205d3beee705a094aec189bd6bc610` | differs | 48.764 | 0.164 | small — fix chain (3.87 % px move, max Δ 44) |
| film-s02 | 285 | `9be3879c94db211aaf0d2668ad874dd26e9c804aab7217f0a520334f6fa1b73c` | `9649a9b65fa375523023fde3c0bf7c69321f7b788232c34b5408444f94e98a89` | differs | 35.508 | 1.517 | sky dome (0f6c033) |
| film-s03 | 435 | `61c54a42dbde95c814e80bb0416a63f95503869ca712c2ba9b691db407495857` | `0bbdbdd8a93959fb8c145b01cce1791c8a8d0a8dbdfb7553e0ee65c3469eca17` | differs | 32.659 | 3.059 | sky dome (0f6c033) |
| film-s04 | 560 † | `61e2ff32da99814610b695fdccdcdec1d2b1338925e3a594e114c1aaaa85af2f` | `f135462d330c0e2d1a4771513bfe36b37a2e0d2310ea0d246a0df8d717680523` | differs | 36.057 | 1.676 | dome + exact keys (cabcc30) — see §3 |
| film-s05a | 689 | `bcb161f11cf418814c83e3875c1fb9c5732ed75e0f7c660c88a332c4e12f2bde` | `bb0ee0c0d969744a997760d8482b2aaf6c8a5a6d46c7b27d4bd80151aa14685b` | differs | 26.093 | 8.760 | sky dome (0f6c033) — the S05a sky band went 22.66 → 45.53 dB vs Chrome |
| film-s05b | 760 † | `e932ed856dbac45f299211456dbe4b8c259a1190c56cf84e13099a2a8a1b215c` | `471366bffdc72f55988cab97ef8c13f9ffd26e98cabb8f2dd7cce06744c67919` | differs | 45.647 | 0.255 | small — exact keys (cabcc30) — see §3 |
| film-s06 | 848 | `d41bd58e3918fe7018d45ce32e1f8ee3c90b749db921a5ea7aba5f3fe6777289` | `7eff42cdf8c80ff6e5dd06c56298ba045d49f6eeff2c2639e59657bcfcb576fb` | differs | 34.049 | 1.742 | fog dome setter (1fc6fab) — S06 sky band 22.18 → 29.16 dB vs Chrome |
| film-s07 | 969 † | `9eca38032479cfdba282358d67cb313b68781f68e65edbfff5a45b21fe6122a1` | `4bc49c5ccb96a94ff340ae99082e4873fd60ec3dfae00e1a2dbd3e3f8d8cb551` | differs | 43.548 | 0.103 | tiny — 0.49 % px move; exact keys — see §3 |
| film-s08 | 1119 | `7ab95e9d4c94fb0b09e1450214f14649ce551f2eea2dfe8316ba1326f398746a` | `5ebdb083bd0e0a2f1ad2c015dfec784ce6180cdb4a91b6cd63d7916aab11c55c` | differs | 24.737 | 12.620 | atmosphere inheritance (1fc6fab) — golden dressing replaces noon; 93 % px move, as designed |
| film-s08b | 1231 | `7764821c30e1094e27626770857e048528fbf4fcc3f8a3c5a512a18379426f5a` | `fb4e2ba44ad807394f6ff59bf0d48e8742a6080f8f63f6dd49a3df500f710dbc` | differs | 24.849 | 12.292 | atmosphere inheritance (1fc6fab) |
| film-s09 | 1306 | `ebe45d9c7f322e22a858b9fe11e057bed848afa7d23690a3d578ee95f40290a3` | `74b6003d0eda5d82e8ac58b4717c33c4c902aa7f6bf6965ae67ad9411796462d` | differs | 24.881 | 12.213 | atmosphere inheritance (1fc6fab) — S09 cloud warmth now matches Chrome exactly (R−B = +31.1 both sides) |
| film-s10 | 1440 | `59bdb4f2558fa37f34569482f1c650669b96eb8a8fb225c8d2825386b93ebaad` | `59bdb4f2558fa37f34569482f1c650669b96eb8a8fb225c8d2825386b93ebaad` | **IDENTICAL** | ∞ | 0 | interior segment; the whole fix chain leaves this frame byte-identical — the control case proving the differences above are confined to their named causes |
| film-s11 | 1593 † | `eb7b23f4a4be2834b842e30924fe29d2928df5022688f3a66fa05fd9a2a84f30` | `231fc172784c09a16c563440296004628906cfcc4d2c9c32ff38f6c1a10cfbf4` | differs | 21.102 | 7.210 | largest mover — exact keys at a boundary mid (see §3); vs Chrome this segment gained most of any (18.52 → 21.91 dB). Movement is concentrated: 15.2 % px move, but 14.9 % move by >10 (max Δ 98) |
| film-s12 | 1738 | `1978d6e603bcfc83e0764de509116a1e448332671fa6342d57b640c04a91bfe7` | `dfabc09a542dd1ccfe473fb10d001ec4981e5aaededf5576b73388f6efb1bf83` | differs | 26.479 | 2.169 | fix chain (dark outro; 4.28 % px move, max Δ 89) |

† = one of the four cache-boundary mids (§3).

**Summary: 13 of 14 differ; film-s10 is byte-identical.** No frame
is missing — all 14 candidates were obtained and are staged in
`docs/golden/frames-f021991/` under the same file names as the
canonical staging (`s01.png` … `s12.png`).

## 3. The four cache-boundary mids — post-fix status

Frames **560 (s04), 760 (s05b), 969 (s07), 1593 (s11)** sit at
chunk offsets ≥160 in the W77/W87 chunking — past the 160-entry
paint-cache cap. Their status, stated plainly:

- **All four candidate hashes differ from the 381ff47 staging**
  (table above: 36.06 / 45.65 / 43.55 / 21.10 dB old-vs-new).
- **That difference is expected, and the new bytes are the safe
  ones to pin.** The 381ff47 bytes were produced by the binary
  carrying the quantised-key defect: `state_key` rounded
  continuous channels to 2 dp, so a frame re-painted after cache
  eviction could collide with a neighbouring state — W77 measured
  exactly this at frames 1195–1199 (72.34 dB across invocations,
  86 % of differing pixels at |Δ|=1). The old boundary-mid hashes
  were therefore never trustworthy as pins: a re-render under the
  same head could legitimately have produced different bytes.
- At **f021991 the defect is fixed at the root** (`cabcc30`:
  cache keys are exact f32 bit patterns) and the lead's final-head
  determinism probes — independent re-renders of 1195–1204
  (**10/10 byte-identical**, the very boundary range that failed
  before), 795–814 (20/20), and 195–204 (10/10) — are the
  evidence that bytes from this head no longer depend on
  invocation history. Pinning the candidates does not bake in
  the old instability; it retires it.
- **Honest bounds.** The probes cover the 1195–1204 boundary and
  two interior ranges; they are not a re-render of these four
  exact mids in a second chunking. The pack's formal
  `DETERMINISM: PASS` per renderer of record (CI lavapipe leg;
  RADV leg still owed on the user's PC) is tracked elsewhere in
  this wave — this record claims the boundary mids are
  *expected-safe to pin on the evidence above*, not that the
  formal determinism gate has been discharged by this seat.

## 4. The ab1-composer candidate (unchanged by this seat)

The Wave-1 re-pin candidate stands exactly as its packet records
(`docs/golden/repin-packets/ab1-composer.md`): candidate frame
`docs/ab/composer-v2/native.png`, sha256
`a415ead80a1a17d2b190c03c237bd2f8129057a8688ebbfa31cd5d26ffceeee9`,
from CI run 43569670339 at the shade-fix commit `c00897d`,
measuring 23.96 dB vs Chrome (pin: 23.850 dB). This seat did not
re-stage or alter it. Note it predates f021991: it is the
post-shade-fix frame, and re-rendering ab1 at the final head is
expected to move it again only if a later fix touches the ab1
chain (the dome/inheritance fixes are film-path; the exact-key
fix can in principle move any frame whose paint states previously
collided). The orchestrator should treat the packet's candidate
as *the* ab1 act input, or order a fresh ab1 dispatch at the
integration head first and re-measure — the packet's procedure
(step 1–2: measure, name the cause) applies either way.

## 5. Exact pin commands for the orchestrator (two SEPARATE acts)

`make_golden.py --pin` records the hash of **every** reference
file currently on disk — that is why the two acts must not share
a tree state. Do them in this order, as two commits.

### Act 1 — the 14 film first-pins (from the f021991 candidates)

Precondition: the ab1 candidate must **not** be staged at
`docs/ab/composer/native.png` yet (check:
`sha256sum docs/ab/composer/native.png` must print the current
pin `d7bd71a0…`; if it prints `a415ead8…`, Act 2 has leaked into
Act 1 — stop and separate them).

```sh
cd ~/workspace/talerender
python3 tools/golden/make_golden.py        # expect: 4 OK / 0 STALE / 14 MISSING
cp docs/golden/frames-f021991/s*.png docs/golden/frames/
sha256sum docs/golden/frames/s04.png       # spot-check: f135462d330c0e2d1…
python3 tools/golden/make_golden.py --pin
python3 tools/golden/make_golden.py        # expect: 18 items — 4 OK (unchanged pins) + 14 film items now OK
git diff tools/golden/golden-hashes.json   # scope check: EXACTLY 14 lines ADDED (film-s01…film-s12);
                                           # the 4 milestone pins must NOT move
```

What this commits us to: the f021991 bytes become the native
golden references for the film. From this act on, any CI/local
re-render of a mid frame that does not reproduce these bytes
verifies **STALE** and must be explained as a disclosed
re-baseline (NATIVE-GOLDEN.md protocol) — never silently
re-pinned. Commit together: `tools/golden/golden-hashes.json`,
`docs/golden/frames/*.png` (the f021991 bytes replacing the
381ff47 staging), this record, and the `frames-f021991/`
candidate dir (provenance). Suggested message:

> First-pin the 14 film goldens at f021991: deliberate baseline
> from the final-head render (A/B mean 23.02 dB). 13/14 mids
> differ from the 381ff47 staging by named causes (sky dome
> 0f6c033, atmosphere inheritance 1fc6fab, exact paint-cache
> keys cabcc30); film-s10 is byte-identical. The four
> cache-boundary mids (560/760/969/1593) are pinned from the
> fixed binary — the old bytes were struck under the
> quantised-key defect and were never stable. Measured table +
> provenance: docs/integration/GOLDEN-RESTAGE-W03.md. No
> milestone pin moves.

### Act 2 — the ab1-composer re-pin (its own commit)

Follow `docs/golden/repin-packets/ab1-composer.md` steps 1–7
verbatim. In outline:

```sh
python3 tools/golden/make_golden.py        # expect ab1-composer OK, no STALE
cp docs/ab/composer-v2/native.png docs/ab/composer/native.png
sha256sum docs/ab/composer/native.png      # must print a415ead80a1a17d2…
python3 tools/golden/make_golden.py        # ab1-composer now STALE (the honest intermediate state)
python3 tools/golden/make_golden.py --pin
git diff tools/golden/golden-hashes.json   # ONLY the ab1-composer line may change: d7bd71a0… -> a415ead8…
```

Because Act 1 has already pinned the film frames and their files
stay on disk unchanged, Act 2's `--pin` re-records their hashes
identically — the diff scope check above is the guard. What this
commits us to: the post-shade-fix frame becomes the living
reference for the composer chain; the pre-fix pin `d7bd71a0…`
remains in git history and the M5 pack. The packet's suggested
commit message and disclosure text are ready to use as written.

**Explicitly not pinned by either act:** `s28-onoda-t150` (the
packet recommends against re-pinning — historical prototype
artifact), `dof-chart` and `stroke-toon` (no post-fix candidates
staged; both carry the re-pin debt analysis in
NATIVE-GOLDEN.md — a future deliberate act, not this one).

## 6. Frames not obtained

**None.** All 14 f021991 mids were obtained from the on-disk set
and hash-verified through the provenance chain in §1. If the
on-disk set is ever lost, the recovery route is: re-dispatch the
Render workflow at head f021991 (or any head whose film path is
unchanged from it), mode `film` in chunks tiling [0, 1811), and
take `frame_<mid:05d>.png` from the `render-film-<from>-<to>-<run_id>`
artifacts — the candidates' sha256 table in §2 is the acceptance
check for any such recovery (a recovered frame that does not
match its candidate hash is a determinism finding, not a
substitute).

## 7. What this seat did not do

No `--pin` (anywhere), no edits outside
`docs/golden/frames-f021991/` + this file, no commits, no changes
to `src/`, the tooling, the existing staging, or the pins file.
The candidate dir is additive only; deleting it restores the
tree to its pre-seat state exactly.
