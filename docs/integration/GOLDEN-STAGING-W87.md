# Golden staging — the 14 film mid frames (W87)

**Act performed: STAGING ONLY.** All 14 film segment mid frames now
sit at their golden paths in `docs/golden/frames/`, hash-verified
against W77's harvest record before and after the copy. **Nothing
was pinned**: `run_golden.sh` refuses `--pin` by design, no
`make_golden.py --pin` was run by this seat, and
`tools/golden/golden-hashes.json` is byte-unchanged (git shows no
diff). The first pins of these frames remain the human re-baseline
act defined in `docs/golden/NATIVE-GOLDEN.md`.

Seat W87, 2026-11-14. Repo HEAD at staging: `743d248`. Frames
staged from `~/workspace/.w77-scratch/frames` (W77's harvest) with
renderer label **CI lavapipe** — the provenance RESULTS.md records
for the harvest (native, CI lavapipe, rendered at head `381ff47`).

## The tool's state vocabulary (read this before the table)

`make_golden.py` has exactly three states: OK, STALE, MISSING.
There is **no separate "staged" state**. Per the tool's own source,
an on-disk frame with no pin is reported MISSING — *"on disk but
never pinned: not a reference yet"* — and the summary counts do not
move. What staging changes is visible only in the table's
`current` column: before staging it read `-` for all 14 film
items; after staging it carries each frame's sha256 prefix. The
verbatim outputs below are the whole story; anyone reading
"14 MISSING" after this staging should understand it as
*14 staged, 0 pinned*, not *14 absent*.

## Per-mid record

Every sha256 below was verified twice by this seat: the scratch
file against `docs/ab/film/RESULTS.md`'s table **before** staging
(14/14 MATCH), and the staged copy at its golden path **after**
staging (14/14 MATCH, no mismatches).

| item | seg | mid frame | sha256 (staged = RESULTS.md) | hash-verified | cache-boundary caveat |
|---|---|---|---|---|---|
| film-s01 | s01 | 89 | `ebcfeba172be55b8ec8a8a56d2b50bef9a8641543d645fa8fd50106729cbc295` | yes | — |
| film-s02 | s02 | 285 | `9be3879c94db211aaf0d2668ad874dd26e9c804aab7217f0a520334f6fa1b73c` | yes | — |
| film-s03 | s03 | 435 | `61c54a42dbde95c814e80bb0416a63f95503869ca712c2ba9b691db407495857` | yes | — |
| film-s04 | s04 | 560 | `61e2ff32da99814610b695fdccdcdec1d2b1338925e3a594e114c1aaaa85af2f` | yes | **YES — chunk offset 160** |
| film-s05a | s05a | 689 | `bcb161f11cf418814c83e3875c1fb9c5732ed75e0f7c660c88a332c4e12f2bde` | yes | — |
| film-s05b | s05b | 760 | `e932ed856dbac45f299211456dbe4b8c259a1190c56cf84e13099a2a8a1b215c` | yes | **YES — chunk offset 160** |
| film-s06 | s06 | 848 | `d41bd58e3918fe7018d45ce32e1f8ee3c90b749db921a5ea7aba5f3fe6777289` | yes | — |
| film-s07 | s07 | 969 | `9eca38032479cfdba282358d67cb313b68781f68e65edbfff5a45b21fe6122a1` | yes | **YES — chunk offset 169** |
| film-s08 | s08 | 1119 | `7ab95e9d4c94fb0b09e1450214f14649ce551f2eea2dfe8316ba1326f398746a` | yes | — |
| film-s08b | s08b | 1231 | `7764821c30e1094e27626770857e048528fbf4fcc3f8a3c5a512a18379426f5a` | yes | — |
| film-s09 | s09 | 1306 | `ebe45d9c7f322e22a858b9fe11e057bed848afa7d23690a3d578ee95f40290a3` | yes | — |
| film-s10 | s10 | 1440 | `59bdb4f2558fa37f34569482f1c650669b96eb8a8fb225c8d2825386b93ebaad` | yes | — |
| film-s11 | s11 | 1593 | `eb7b23f4a4be2834b842e30924fe29d2928df5022688f3a66fa05fd9a2a84f30` | yes | **YES — chunk offset 193** |
| film-s12 | s12 | 1738 | `1978d6e603bcfc83e0764de509116a1e448332671fa6342d57b640c04a91bfe7` | yes | — |

No mid was missing from the scratch; no CI re-extraction was
needed (the artifacts' ~2026-11-21 expiry was not reached).

## The cache-boundary caveat (applies to 4 of the 14)

W77's chunk-overlap check (RESULTS.md) found frames 1200–1204
byte-identical across the chunk and probe invocations, but frames
**1195–1199 differ at PSNR 72.34 dB**: 1,787 of 2,073,600 px
(0.086%), 86% of them at |Δ| = 1, max |Δ| 28, clustered in the
lower-left foliage. The leading hypothesis is that a paint
re-rendered after the 160-entry FIFO paint-cache cap evicts is not
bit-identical to its first render (the chunk invocation had
rendered 195 frames when it reached 1195; the probe 0–4).

Four staged mids were rendered at chunk-invocation offsets ≥ 160
and so may carry the same ±1-class deviation relative to a fresh
single-pass render: **frame 560 (s04, offset 160), frame 760
(s05b, offset 160), frame 969 (s07, offset 169), frame 1593
(s11, offset 193)**. Their staged bytes are exactly the CI chunk
renders, hashes above — the caveat is about what a *future*
re-render would produce, not about these copies' integrity. It is
immaterial to every A/B number in the proof package (~30 dB below
the references' own JPEG noise), but whoever pins these four
should weigh it first, as RESULTS.md advises.

## Provenance as recorded by the runner

`docs/golden/frames/RENDERER.txt` (written by `run_golden.sh`):

```
renderer: CI lavapipe
frames-dir (as passed): /home/hatch/workspace/.w77-scratch/frames
frames taken from: /home/hatch/workspace/.w77-scratch/frames
staged at (UTC): 2026-11-14T08:36:56Z
repo HEAD: 743d248
staged segment mid frames: (all 14, with the sha256 prefixes
matching the table above)
note: staged frames are NOT pins. A frame becomes a golden
reference only when a human runs make_golden.py --pin
(see NATIVE-GOLDEN.md).
```

## Verify output — before staging (baseline)

```
4 OK / 0 STALE / 14 MISSING of 18 golden items
```
with every film item's `current` column reading `-` (absent from
disk).

## Verify output — after staging (verbatim, from the runner)

`run_golden.sh` exited **0** ("staged 14 of 14 segment mid frames";
"all 14 segment mid frames staged"). The `make_golden.py` table it
printed, verbatim:

```
item            status   pinned            current           path
s28-onoda-t150  OK       ff68fad3bbcc6155  ff68fad3bbcc6155  docs/ab/onoda-s01-t150/native.png
dof-chart       OK       957d5573497e6269  957d5573497e6269  docs/ab/dof-chart/native.png
stroke-toon     OK       9ba9ff890402b6cf  9ba9ff890402b6cf  docs/ab/stroke-battery/native-toon.png
ab1-composer    OK       d7bd71a02a3dfc96  d7bd71a02a3dfc96  docs/ab/composer/native.png
film-s01        MISSING  -                 ebcfeba172be55b8  docs/golden/frames/s01.png
film-s02        MISSING  -                 9be3879c94db211a  docs/golden/frames/s02.png
film-s03        MISSING  -                 61c54a42dbde95c8  docs/golden/frames/s03.png
film-s04        MISSING  -                 61e2ff32da998146  docs/golden/frames/s04.png
film-s05a       MISSING  -                 bcb161f11cf41881  docs/golden/frames/s05a.png
film-s05b       MISSING  -                 e932ed856dbac45f  docs/golden/frames/s05b.png
film-s06        MISSING  -                 d41bd58e3918fe70  docs/golden/frames/s06.png
film-s07        MISSING  -                 9eca38032479cfdb  docs/golden/frames/s07.png
film-s08        MISSING  -                 7ab95e9d4c94fb0b  docs/golden/frames/s08.png
film-s08b       MISSING  -                 7764821c30e1094e  docs/golden/frames/s08b.png
film-s09        MISSING  -                 ebe45d9c7f322e22  docs/golden/frames/s09.png
film-s10        MISSING  -                 59bdb4f2558fa37f  docs/golden/frames/s10.png
film-s11        MISSING  -                 eb7b23f4a4be2834  docs/golden/frames/s11.png
film-s12        MISSING  -                 1978d6e603bcfc83  docs/golden/frames/s12.png

4 OK / 0 STALE / 14 MISSING of 18 golden items
```

So, plainly: **the counts did not change** — 4 OK / 0 STALE /
14 MISSING before and after — because the tool deliberately has
no staged-but-unpinned state distinct from MISSING. What changed:
all 14 film frames are now on disk at their golden paths with
their full hashes recorded in this document and their prefixes in
the tool's `current` column and in `RENDERER.txt`. A human running
`make_golden.py --pin` now would pin exactly these bytes (plus,
per the repin packet's warning, anything else staged on disk at
that moment — see the next section before doing so).

## ab1-composer re-pin candidate — NOT staged by the runner

The candidate was **not** staged, and remains only at
`docs/ab/composer-v2/native.png` (sha256
`a415ead80a1a17d2b190c03c237bd2f8129057a8688ebbfa31cd5d26ffceeee9`,
re-verified by this seat). Reasons:

- `run_golden.sh` stages **film mid frames only**; it has no
  candidate-staging path for milestone items. The repin packet
  (`docs/golden/repin-packets/ab1-composer.md`) says the same:
  "This item is **not** staged by `tools/golden/run_golden.sh`".
- The packet's step 2 (copy the candidate over
  `docs/ab/composer/native.png`) is a step of the **human re-pin
  procedure itself**, not a staging area: the moment it is done,
  the item reads STALE (exit 1) until the human pins. Performing
  it here, detached from the pin act, would leave the pack in a
  failing state and pre-empt the deliberate act the policy
  reserves.
- The golden path still holds the pre-fix pin bytes
  (`d7bd71a0…`), and `ab1-composer` still verifies **OK**, exactly
  as the packet's step 1 expects to find it.

One interaction the human should note before pinning anything:
`--pin` records the hash of **every** reference currently on
disk. With the 14 film frames now staged, a `--pin` run for the
ab1-composer re-baseline would also strike the 14 film first-pins
in the same act. The packet's protocol already warns about this
("anything staged on disk gets pinned too"); the film first-pins
and the ab1 re-pin are separate decisions with separate causes,
so either pin the film frames first in their own commit, or do
the ab1 re-pin knowing it carries them.
