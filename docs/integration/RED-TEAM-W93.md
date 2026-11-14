# RED-TEAM-W93 — second red team: the wave's closing claims, re-run first-hand

Seat W93, 2026-11-14. Method: nothing taken from the claiming seat's word —
every number below was recomputed in this seat from the primary artefact
(the log file, the MP4, the CI artifact, the git history, the scene data,
the staged PNGs). Verdicts: **REPRODUCED** (claim holds under independent
re-run) / **REFUTED** (claim does not hold as stated) / **UNVERIFIABLE**
(evidence no longer exists to check).

**Score: 5 REPRODUCED · 1 REFUTED · 0 UNVERIFIABLE.** The one refutation is
the FINAL-REPORT commit census — stale by concurrency, internally
inconsistent, and fully decomposed below so the close-out pass (W91) can
close it mechanically.

---

## Claim 1 — Telemetry agreement (W85 vs the lead's runs)

**Claim** (TELEMETRY-W85.md §"Agreement with the lead's own runs"): W85's
cache-24 run ends with the identical verdict line as the lead's
`/tmp/telemetry4.log` and `/tmp/telemetry5.log`, and telemetry5's seven
CAPTION WIDTH FAIL lines match W85's verbatim.

**Verdict: REPRODUCED** — and the evidence is stronger than the claim.

- All three logs exist (`/tmp` was **not** wiped): W85's stdout at
  `~/workspace/.w85-scratch/telemetry-cache24.stdout.log`, the lead's
  `/tmp/telemetry4.log` (08:08 UTC) and `/tmp/telemetry5.log` (08:42 UTC).
- Extracting exactly the verdict line + the 7 failure lines from each file
  and hashing the subsets: **md5 `e1a4a981c7144fe1ce4befed4fff5510` for all
  three**; a direct `diff` of the extracted lines is empty. The lines agree
  character for character — same chunks, widths (1039/1051/1002/892/1015/
  871/1050 px), and times.
- Verdict line in all three, verbatim:
  `FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)`.
- Bonus beyond the claim: `/tmp/telemetry4.log` and `/tmp/telemetry5.log`
  are **byte-identical whole files** (same md5
  `02a6c1e84331f12557519b91a4281331`), so the agreement covers telemetry4's
  failure lines too, not only its verdict.
- The only whole-file difference between W85's stdout log and telemetry5
  is the 12 `film-telemetry: sample N` progress lines, which the lead's log
  carries inline and W85 captured in its separate stderr log
  (`telemetry-cache24.stderr.log`, all 12 lines present). The audit content
  is identical; only the stream routing differs. No discrepancy.

## Claim 2 — MP4 technical evidence (W84 / MP4-EVIDENCE.md)

**Claim**: the delivered `~/workspace/your_files/onoda-native-381ff47.mp4`
matches its evidence record on hash, streams, dimensions, frame count,
duration, faststart, and loudness (−14.00 LUFS vs the mix's −13.99).

**Verdict: REPRODUCED on every field** — independent re-probe of the file:

| Field | Claimed | Re-measured (this seat) | |
|---|---|---|---|
| sha256 | `f357a00434e4a8dcc…` | `f357a00434e4a8dcc4b78607a83b8010c159df86bf81b0bcb8f9cf9b00aecaed` | ✓ |
| size | 29,722,802 B | 29,722,802 B | ✓ |
| video | h264 High@L5.0, 1080×1920, yuv420p, 30/1 | identical (level 50, r/avg fps 30/1) | ✓ |
| video bitrate | 3,737,567 b/s | 3,737,567 b/s | ✓ |
| colour tags | space bt709; primaries/trc **unknown** (Finding F1) | `color_space=bt709`, `color_transfer=unknown`, `color_primaries=unknown` | ✓ (F1 is real, as disclosed) |
| header `nb_frames` | 1811 | 1811 | ✓ |
| **counted frames** (`-count_frames`) | 1811 | **1811** | ✓ |
| durations | container 60.457 s; video 60.366667 s | 60.457000 / 60.366667 | ✓ |
| audio | aac LC, 192,073 b/s, 48 kHz stereo | identical; source mix confirmed `pcm_s16le` 48 kHz stereo | ✓ |
| faststart | box order `ftyp, moov, free, mdat` | `['ftyp', 'moov', 'free', 'mdat']` (own box walk) | ✓ |
| loudness MP4 | −14.00 LUFS / −1.58 dBTP / LRA 1.40 | −14.00 / −1.58 / 1.40 (own loudnorm run) | ✓ |
| loudness mix | −13.99 LUFS / −1.65 dBTP / LRA 1.40 | −13.99 / −1.65 / 1.40 (own loudnorm run) | ✓ |

The mux-is-the-mix claim holds to the digit (Δ integrated 0.01 LU).
Nothing in the evidence record overstates the file.

## Claim 3 — fx-textures byte-identity, local vs CI (W86)

**Claim** (LOADER-EVIDENCE-W86.md §1): W86's locally rendered
`fx-textures.png` is byte-identical to the CI dispatch frame from run
43575103004 (head `69952ab`), sha256 `6ef2e39b…`.

**Verdict: REPRODUCED** — verified against a **fresh download made by this
seat**, not W86's copies:

- W86's local render (`~/workspace/.w86-scratch/fx-textures.png`):
  sha256 `6ef2e39b3d2cc3589e6655435bff52f8e5ff638ffc0aedf603f250500577304d`.
- Fresh `gh run download 43575103004` (artifact
  `render-scene-file-0-0-43575103004`, downloaded by this seat to /tmp):
  sha256 `6ef2e39b3d2cc3589e6655435bff52f8e5ff638ffc0aedf603f250500577304d`.
- `cmp` of the fresh CI frame against W86's local render: **byte-identical**
  (62,924 bytes both). Three-way agreement: local render = W86's CI copy =
  this seat's fresh CI download.

## Claim 4 — Commit census (FINAL-REPORT)

**Claim**: FINAL-REPORT's census of commits since base `34d7882`.

**Verdict: REFUTED as a statement about the current tree** — the report's
figures are stale, and the report disagrees with itself. Timing mitigation
below; the decomposition is exact so W91 can close it without re-counting.

**This seat's recount.** Classification rule: a commit is **wave** iff its
subject begins `Wave batch N` (the orchestrator's central batch commits);
everything else is **lead** (the lead/integrator's own subjects — rulings,
film.rs work, loader lane, fog wiring, spec addenda). The rule is not the
discrepancy: applied at the 18-commit mark it reproduces FINAL-REPORT's own
named lead list exactly (the 11 it enumerates in §5.1).

At HEAD `8af40eb` (this seat, 2026-11-14 ~09:17 UTC):

- **Total since `34d7882` (exclusive): 24 — lead 15, wave 9.**

**What the report currently says** (file read at the same moment; its mtime
was seconds old, so W91's completion pass may be in flight concurrently —
it had **not** landed as of this check):

- §5.1: "Lead commits … 11 … Wave batch commits: 7 … Total … 18." — true
  at HEAD `3781eb2` (Batch-7 close); its batch table still lists Batches
  8–10 as `[TO COMPLETE]`.
- §4.5, the [W89 correction]: "21 since base on local main, 20 of them on
  origin/main", origin at `5b0889d` — true when written during Batch 9.

**Delta decomposition** (report → tree): since §5.1's 18, six commits
landed — lead +4 (`69952ab` fog wiring, `5b0889d` spec addendum, `0f6c033`
sky dome, `ede0e92` scene-format/sat doc) and wave +2 (`743d248` Batch 8,
`8af40eb` Batch 9): 11+4 = 15 lead, 7+2 = 9 wave, 24 total. Since §4.5's
21: +3 (`0f6c033`, `ede0e92`, `8af40eb`).

**Push state, current**: remote `main` = `ede0e92` (verified by
`git ls-remote` and the local tracking ref); local HEAD `8af40eb` is ahead
by exactly **1** commit (the Batch-9 wave commit). So the report's
"ahead by exactly that one commit" shape survives, but both of its named
heads (`5b0889d` / `743d248`) are superseded.

Not a fabrication and not a content error — every figure was correct when
written, and the report self-tags the census/push material
`[TO COMPLETE]` for the close-out pass. It is nevertheless the one closing
claim that does not survive contact with the current tree, and it must not
be quoted until W91 lands the recount above.

## Claim 5 — Saturation key census (W81)

**Claim**: zero `saturation` keys remain in the film scene files; all 14
segments carry `sat` in `post.grade`, values in 0.52–1.08.

**Verdict: REPRODUCED exactly.** Independent census (JSON-parsed
`post.grade` plus a raw-text sweep of every file):

- 14/14 segment files carry `sat`; **zero** carry `saturation` in the
  grade block, and the string `"saturation"` appears **nowhere** in any of
  the 14 files (so no stray non-grade occurrence either).
- Values: s01 0.8, s02 0.84, s03 0.84, s04 0.8, s05a 0.8, s05b 0.8,
  s06 0.52, s07 0.85, s08 0.8, s08b 0.8, s09 0.8, s10 0.78, s11 1.08,
  s12 1.08 — min 0.52, max 1.08, exactly the claimed range and the claimed
  per-file values from W71's pre-rename census.

## Claim 6 — Golden staging hashes (W87)

**Claim**: the 14 staged frames in `docs/golden/frames/` hash-match the
table in `docs/ab/film/RESULTS.md`. Sampled per brief: s04, s07, s11
(s04/s11 at chunk offsets ≥160 carry the cache caveat; s07 does not).

**Verdict: REPRODUCED** — all three sampled frames match the table to the
full 64 hex digits:

| Segment | Frame | RESULTS.md table | Staged file (this seat's sha256) |
|---|---|---|---|
| s04 | 560 | `61e2ff32da99814610b695fdccdcdec1d2b1338925e3a594e114c1aaaa85af2f` | `61e2ff32da99814610b695fdccdcdec1d2b1338925e3a594e114c1aaaa85af2f` ✓ |
| s07 | 969 | `9eca38032479cfdba282358d67cb313b68781f68e65edbfff5a45b21fe6122a1` | `9eca38032479cfdba282358d67cb313b68781f68e65edbfff5a45b21fe6122a1` ✓ |
| s11 | 1593 | `eb7b23f4a4be2834b842e30924fe29d2928df5022688f3a66fa05fd9a2a84f30` | `eb7b23f4a4be2834b842e30924fe29d2928df5022688f3a66fa05fd9a2a84f30` ✓ |

Note the caveat survives honestly: hash agreement proves the staged bytes
are the CI chunk renders (what pinning would freeze); it says nothing
about whether a *re-render* of s04/s11 would reproduce them — the
1195–1199 cache-boundary measurement (72.34 dB) stands unrefuted and is
the correct caution to carry into the pinning decision.

---

## Bottom line

The wave's evidence-grade claims — telemetry agreement, MP4 record,
fx byte-identity, saturation census, golden staging — all reproduce
first-hand, two of them (telemetry, fx) more strongly than claimed. The
single failure is bookkeeping, not engineering: FINAL-REPORT's commit
census (18 in §5.1, 21 in §4.5) no longer matches the tree (**24 = 15
lead + 9 wave** at `8af40eb`; origin at `ede0e92`, local ahead by the
Batch-9 commit only). W91's completion pass should adopt the recount and
delta decomposition in Claim 4 verbatim.
