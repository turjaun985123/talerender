# TELEMETRY-TABLES-W75 — G7: the three review tables for `--film-telemetry`

**Seat:** Wave 2 W75 (Batch 8). **Scope:** design + working prototype for the
G7/R6 remainder recorded OPEN and unqueued in GAPFIX-MATRIX-W49.md:
`--film-telemetry` prints the verdict line (plus the audit's own summary
lines) but no **shot table**, no **full caption table**, and no **cast
list** — the three tables a human reviewer needs to audit a film without
opening the data files by hand.

**Deliverables:** this design, plus the prototype
`tools/film/telemetry_tables.py` (stdlib-only, not wired into the binary).
Its verbatim output on the real film is the worked example in §5.

**Verified at:** HEAD `b2e46d2`, against `film/onoda.json` +
`scenes/film/film.json` + the 14 scene files at that head.

---

## 1. Recommendation: an opt-in `--tables` flag, stdout, after the verdict

Add a `--tables` flag to the `--film-telemetry` invocation. When present,
the binary prints the three tables to **stdout, after the verdict line**.
Default (flag absent) output stays byte-identical to today.

Why this shape, against the alternatives:

- **Verdict-line contract is untouched.** The line `FILM TELEMETRY:
  CHECK OK` / `FILM TELEMETRY: CHECK FAILED (…)` remains exactly where
  and what it is; the skill-side adapter gates on that line (proven
  8/8 cells by W62's re-proof) and the exit-code contract of
  EXIT-CODES-W02 is unchanged. Tables are additive output *below* the
  line the gate reads — nothing that parses today's output breaks,
  because every existing consumer either greps the verdict line or
  checks the exit code.
- **Default-on rejected.** Telemetry output is machine-consumed (adapter
  stub suites, tools-ci steps, the wave's own probes all read it).
  Growing default output by ~120 lines to serve a human reviewer would
  tax every automated reader and invalidate recorded output shapes.
  Human review is the deliberate act; the flag is its switch.
- **Sidecar file rejected.** A `tables.txt` next to the run drifts from
  the run that produced it (the wave has already been bitten by
  stale-file evidence twice — the retracted adapter re-proof, W62/W63).
  Stdout travels with the verdict in the same log, same CI artefact,
  same scrollback. One run, one record.
- **stderr rejected.** In this binary stderr is the progress channel
  (`film-telemetry: sample N …`). Review tables are results, not
  progress; a reviewer archiving stdout should get the verdict and its
  evidence together.
- **Separate mode rejected** (e.g. `--film-tables` as its own mode):
  the tables must describe the *same* data load and the *same* audit
  the verdict summarises. A second mode invites a second load path —
  and with it, the possibility that the tables and the verdict quietly
  describe different data. One invocation, one load, one audit; the
  flag only widens the printing.

## 2. Exit-code interaction: none, by construction

The tables are **informational renderings of data the audit already
loaded**; they introduce no new failure condition and must never change
the exit code (0 OK / 1 FAILED / 3 could-not-run, per EXIT-CODES-W02).
Two consequences the implementation must honour:

- If the audit itself fails to run, there are no tables (exit 3 as
  today) — a tables-only failure mode must not be invented.
- If table *rendering* hits inconsistent data (e.g. a caption whose
  strip is missing), that is already a load error in the audit path
  (`load_assets` fails loudly, film.rs:2156) before any printing. The
  tables inherit the audit's errors; they add none of their own.

The prototype mirrors this discipline in miniature: its exit 1 means
"the *data* failed reconciliation" (a prototype-only concern — it
cross-checks sources the binary trusts singly), never "a table row
looked bad". A flagged caption row (WIDTH/FIT) does **not** move the
prototype's exit code; flags are data, printed in the row.

## 3. Format law

1. **Plain text, fixed-width columns**, one row per record, no colour,
   no box-drawing, no locale-dependent formatting. Numbers right-aligned
   in fixed columns; text last, so long texts never shift other columns.
2. **Stable ordering**: shots in film order, chunks by index, cast by
   first appearance. No sorting by measured values — a diff between two
   runs must read as a *data* change, never as a ranking shuffle.
3. **Stable formatting**: times always `%.2f`, ranges always `a-b`,
   ids always the canonical ones (see §4.3). If a value didn't change,
   its bytes didn't change.
4. **Headers carry the audit bars** (the caption table's header states
   the 850 px / 0.68 bars), so a printed table is self-interpreting in
   a pasted log with no other context.
5. **Notes, not silent choices**: anything the table knows that a row
   can't hold (a caption straddling a cut) is printed as an explicit
   `note:` line under the table — never folded into a number.

## 4. How each table is derived (and its parity delta)

### 4.1 Caption table — width parity is exact, not approximate

The binary's method (film.rs caption-audit block, ~line 3679): for each
caption, load its strip PNG (`film/assets/<captions[i].strip>`) via
`load_png` (image crate → RGBA8); the **rendered width** is the ink
bounding box — columns containing any pixel with **alpha > 32** —
`x1 − x0 + 1` in strip pixels; **raw width** = rendered / `fit`.
`print_caption_audit` (telemetry.rs:1094–1095) fails a chunk, strict
mode, when **rendered_w > 850.0** (WIDTH) or **fit < 0.68** (FIT).

The prototype reproduces this bit-for-bit: PNG is lossless, the strips
are 8-bit RGBA non-interlaced (verified: 1080×300, colour type 6), and
the prototype's stdlib decoder was cross-checked against PIL on all 73
strips — **73/73 identical ink widths** (verification used PIL locally;
the shipped prototype needs nothing beyond the stdlib). The one
approximation in the number is the binary's own declared one — ink bbox
of the pre-rendered strip, "PIL metrics — the declared approximation of
docs/integration/README.md" (film.rs:3542) — and the prototype inherits
it unchanged rather than adding a second one.

Two traps the table avoids, both verified in the data:

- The film data carries a stored `width` field per caption. It is
  **dead provenance** (class F3-a: written by the generator, read by
  nobody) and it is *not* the audit's measure — it drifts from the
  strip measurement by up to **16.11 px** on this film. The table
  prints only the measured value.
- Line assignment is **recomputed** with the binary's rule (last line
  whose start ≤ chunk start + 0.05, film.rs ~line 3695), not read from
  the stored `line` field. On this film the two agree on all 73 chunks
  (the prototype checks and would report a disagreement).

Shot assignment uses film.json's `captionChunks` tiling, verified
against the rule the tiling encodes: the segment containing the chunk's
**start** owns it. (A midpoint rule fails 3 chunks — chunk windows are
word-timed and straddle cuts; see §6.) All 73 chunks satisfy the
start rule.

### 4.2 Shot table — data verbatim, presence cross-verified

Frame ranges, times, durations, set and atmosphere are film.json's
segment fields verbatim (they tile [0, 1811) exactly — checked).
Characters present come from each segment's scene file (`nodes` with
`type: "character"`). Because the scene files are the *staging record*
rather than the driver, presence was cross-verified against the driver
itself: the `char()` calls in every staging function of film.rs match
the scene-file nodes **14/14 shots**, including the two edge cases —
S05A stages nobody in either source (a planes-and-leaflets shot), and
S11 carries two Onoda nodes in both (the young/old instance swap,
shown as `onoda x2`).

### 4.3 Cast list — shot-granularity frames; beats transcribed and fenced

- **Identity** is the binary's continuity id (film.rs INSTANCE_IDS):
  `onoda, compA, compB, suzuki, tani`. Scene files call the last of
  these `taniguchi` (id and display name); the prototype canonicalises
  via an explicit alias and says so in its header comment. Id-less
  nodes (display names like "Onoda (S08b)") are matched
  case-insensitively after stripping the parenthetical.
- **Frame counts are shot-granularity**: the full frame count of every
  shot the identity appears in. This is the honest data-only measure,
  and it is labelled as such in the table header. **Delta vs the
  binary:** the in-binary implementation sits next to the telemetry
  audit, which samples *painted cutouts* every 0.25 s — it can report
  sampled presence (and, with per-frame sampling, exact counts) that
  the prototype cannot see from data alone (e.g. Taniguchi enters S10
  mid-shot in the staging; the prototype credits him the whole shot).
  The prototype's counts are therefore an upper bound per shot, exact
  in aggregate only where presence is shot-long.
- **Authored blink beats**: the beat *times* are word anchors in the
  film data (`anchors`), but the anchor → (character, shot) binding
  exists only in the driver's staging code. The prototype transcribes
  the seven `blink_of` call sites (each cited to its film.rs line in
  the tool's BEAT_BINDINGS table; call-site seeds match the
  look_registry blink seeds — onoda 3, suzuki 5, tani 9) and then
  *verifies the transcription against the data*: every beat must land
  inside its shot's window and on a character present in that shot.
  All seven pass. Beat-frame counts use the film's own rules — frame
  *n* is staged at t = n/fps (film.json frameRule) and a beat lights
  the blink while 0 ≤ t − beat < 0.12 (film.rs `blink_at`) — totalling
  **25 character-frames**, which reconciles exactly with W58's
  independent enumeration of the beat frames.

## 5. Worked example — prototype output on the real film, verbatim

Command: `python3 tools/film/telemetry_tables.py` (repo root, HEAD
`b2e46d2`). Exit 0. The block between the markers is byte-identical to
the tool's stdout.

<!-- BEGIN VERBATIM OUTPUT -->
```
SHOT TABLE — 14 shots, 1811 frames @ 30 fps, 60.36 s
shot  frames       time (s)           dur  set  atmosphere characters
S01   0-177        0.00-5.91         5.91  JG   noon       onoda
S02   178-391      5.91-13.06        7.15  SH   noon       onoda, compA, compB
S03   392-478      13.06-15.95       2.89  SH   noon       tani, onoda, compA, compB
S04   479-641      15.95-21.38       5.43  JG   noon       compA, compB, onoda
S05A  642-736      21.38-24.56       3.18  JG   noon       (none)
S05B  737-783      24.56-26.13       1.57  JG   noon       onoda
S06   784-911      26.13-30.39       4.26  JG   fog        compA, compB, onoda
S07   912-1026     30.39-34.21       3.82  JG   golden     onoda
S08   1027-1210    34.21-40.36       6.15  JG   noon       onoda, suzuki
S08B  1211-1251    40.36-41.73       1.37  JG   noon       onoda, suzuki
S09   1252-1360    41.73-45.37       3.63  JG   noon       onoda, suzuki
S10   1361-1519    45.37-50.65       5.28  BK   noon       tani
S11   1520-1665    50.65-55.52       4.87  JG   dusk       onoda x2, tani
S12   1666-1810    55.52-60.36       4.84  JG   dusk       onoda, tani

CAPTION TABLE — 73 chunks (audit bars: rendered > 850 px = WIDTH fail, fit < 0.68 = FIT fail; strict)
idx  line  shot  window (s)         fit  rend    raw  flag      text
0    1     S01   0.27-0.43         1.00   208    208            "This"
1    1     S01   0.43-0.93         1.00   320    320            "soldier"
2    1     S01   0.93-1.35         1.00   208    208            "kept"
3    1     S01   1.35-1.83         1.00   366    366            "fighting"
4    1     S01   1.83-2.15         1.00   279    279            "World"
5    1     S01   2.15-2.77         1.00   193    193            "War"
6    1     S01   2.77-3.13         1.00    60     60            "II"
7    1     S01   3.13-3.65         1.00   144    144            "for"
8    1     S01   3.65-4.17         1.00   127    127            "29"
9    1     S01   4.17-4.67         1.00   265    265            "years"
10   1     S01   4.67-5.13         1.00   224    224            "after"
11   1     S01   5.13-5.29         1.00    72     72            "it"
12   1     S01   5.29-5.94         1.00   318    318            "ended."
13   2     S02   5.94-6.69         1.00   552    552            "Hiroo Onoda"
14   2     S02   6.69-7.87         1.00   713    713            "was a Japanese"
15   2     S02   7.87-8.53         1.00   320    320            "soldier"
16   2     S02   8.53-9.31         1.00   316    316            "sent to"
17   2     S02   9.31-10.39        1.00   811    811            "a Philippine island"
18   2     S02   10.39-13.15       1.00   359    359            "in 1944."
19   3     S03   13.15-13.61       1.00   462    462            "His orders"
20   3     S03   13.61-14.75       1.00   575    575            "were simple:"
21   3     S03   14.75-16.01       1.00   746    746            "never surrender."
22   4     S04   16.01-17.25       1.00   355    355            "In 1945,"
23   4     S04   17.25-19.51       1.00   668    668            "the war ended."
24   4     S04   19.51-19.93       1.00   466    466            "But Onoda"
25   4     S04   19.93-20.47       1.00   430    430            "never got"
26   4     S04   20.47-21.61       1.00   608    608            "the message."
27   5     S05A  21.61-22.09       1.00   695    695            "Planes dropped"
28   5     S05A  22.09-22.61       1.00   346    346            "leaflets"
29   5     S05A  22.61-23.59       1.00   660    660            "saying the war"
30   5     S05A  23.59-24.49       1.00   434    434            "was over."
31   5     S05A  24.49-24.97       1.00   581    581            "He thought it"
32   5     S05B  24.97-25.57       1.00   643    643            "was an enemy"
33   5     S05B  25.57-26.19       1.00   248    248            "trick."
34   6     S06   26.19-26.67       1.00   492    492            "His friends"
35   6     S06   26.67-27.19       1.00   358    358            "gave up"
36   6     S06   27.19-27.79       1.00   343    343            "or died,"
37   6     S06   27.79-28.49       1.00   339    339            "until he"
38   6     S06   28.49-29.35       1.00   485    485            "was alone."
39   7     S06   29.35-29.71       1.00   362    362            "He lived"
40   7     S06   29.71-30.29       1.00   551    551            "in the jungle"
41   7     S06   30.29-31.55       1.00   566    566            "for decades,"
42   7     S07   31.55-32.01       1.00   436    436            "surviving"
43   7     S07   32.01-32.99       1.00   559    559            "on coconuts"
44   7     S07   32.99-34.27       1.00   605    605            "and bananas."
45   8     S08   34.27-35.53       1.00   341    341            "In 1974,"
46   8     S08   35.53-37.13       1.00   728    728            "a young traveler"
47   8     S08   37.13-38.07       1.00   572    572            "named Norio"
48   8     S08   38.07-38.45       1.00   314    314            "Suzuki"
49   8     S08   38.45-39.95       1.00   488    488            "found him."
50   8     S08   39.95-40.83       1.00   844    844            "Onoda still refused"
51   8     S08B  40.83-41.76       1.00   387    387            "to leave."
52   9     S09   41.76-42.17       1.00   332    332            "He said"
53   9     S09   42.17-43.13       1.00   837    837            "he would only stop"
54   9     S09   43.13-44.01       1.00   429    429            "if his own"
55   9     S09   44.01-44.43       1.00   545    545            "commander"
56   9     S09   44.43-45.39       1.00   465    465            "ordered it."
57   10    S10   45.39-46.39       1.00   693    693            "So Japan found"
58   10    S10   46.39-46.93       1.00   299    299            "his old"
59   10    S10   46.93-47.77       1.00   566    566            "commander,"
60   10    S10   47.77-48.93       1.00   776    776            "now a bookseller,"
61   10    S10   48.93-49.63       1.00   571    571            "and flew him"
62   10    S10   49.63-50.69       1.00   583    583            "to the island."
63   11    S11   50.69-51.19       1.00   433    433            "He finally"
64   11    S11   51.19-52.07       1.00   669    669            "gave the order,"
65   11    S11   52.07-52.99       1.00   483    483            "and Onoda"
66   11    S11   52.99-53.57       1.00   559    559            "surrendered"
67   11    S11   53.57-55.55       1.00   654    654            "after 29 years."
68   12    S12   55.55-56.21       1.00   745    745            "He wasn't hiding"
69   12    S12   56.21-57.15       1.00   596    596            "from the war."
70   12    S12   57.15-57.73       1.00   516    516            "He was just"
71   12    S12   57.73-58.35       1.00   405    405            "still loyal"
72   12    S12   58.35-60.36       1.00   210    210            "to it."
note: chunk 12 (S01) straddles the cut — its window runs 0.03 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)
note: chunk 18 (S02) straddles the cut — its window runs 0.09 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)
note: chunk 21 (S03) straddles the cut — its window runs 0.06 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)
note: chunk 26 (S04) straddles the cut — its window runs 0.23 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)
note: chunk 31 (S05A) straddles the cut — its window runs 0.41 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)
note: chunk 33 (S05B) straddles the cut — its window runs 0.06 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)
note: chunk 41 (S06) straddles the cut — its window runs 1.16 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)
note: chunk 44 (S07) straddles the cut — its window runs 0.06 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)
note: chunk 50 (S08) straddles the cut — its window runs 0.47 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)
note: chunk 51 (S08B) straddles the cut — its window runs 0.03 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)
note: chunk 56 (S09) straddles the cut — its window runs 0.03 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)
note: chunk 62 (S10) straddles the cut — its window runs 0.04 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)
note: chunk 67 (S11) straddles the cut — its window runs 0.03 s into the next shot; the driver draws it across the cut (window rule, film.rs eval_frame)

CAST LIST — 5 identities (frame counts are shot-granularity: full frames of every shot the identity appears in)
id        shots  frames  beats  shot list / authored blink beats
onoda        12    1557      5  S01, S02, S03, S04, S05B, S06, S07, S08, S08B, S09, S11, S12
                                beats: years0 @ 4.20 (S01, 4f); never2 @ 14.78 (S03, 3f); message3 @ 20.70 (S04, 4f); commander8 @ 44.04 (S09, 3f); loyal11 @ 58.12 (S12, 4f)
compA         4     592      0  S02, S03, S04, S06
                                beats: -
compB         4     592      0  S02, S03, S04, S06
                                beats: -
tani          4     537      1  S03, S10, S11, S12
                                beats: bookseller9 @ 48.16 (S10, 4f)
suzuki        3     334      1  S08, S08B, S09
                                beats: found7 @ 38.48 (S08, 3f)

RECONCILIATION: 73 chunks (0 flagged), 1811 frames, 14 shots, 7 authored beats bound
RECONCILIATION: all checks passed (tiling == start assignment, stored line == recomputed line, frame totals agree, beats in-window on present characters)
```
<!-- END VERBATIM OUTPUT -->

Reading notes for the reviewer: chunk 50 ("Onoda still refused") is the
film's widest at **844 px — 6 px under the bar** (the same ~844 px the
Batch-1 gate probed as a watch item; the re-chunk left it legal but
tight). Max rendered 844 and min fit 1.00 across the table are exactly
consistent with the binary's standing `FILM TELEMETRY: CHECK OK`.

## 6. What the tables already surface (findings, not adjustments)

1. **Chunk 41 straddles the S06→S07 cut by 1.16 s.** Thirteen chunks
   straddle their cut (printed as `note:` lines) — mechanically
   expected, since cuts sit at line-start − 0.06 s while chunk windows
   run to the next word — but twelve overrun by ≤ 0.47 s. Chunk 41
   ("for decades,", 30.29–31.55) overruns by 1.16 s because the
   S06/S07 boundary (30.39) is an authored shot-block boundary, not
   the cut rule (film.json boundaryRule). The caption therefore plays
   across a set/atmosphere change (fog → golden) for over a second.
   Whether that is an editorial defect is a human call; until now it
   was invisible without hand-diffing two data files.
2. **S05A has no cast** and S10 is a **Taniguchi solo** — both now
   one-glance facts instead of scene-file archaeology.
3. **The width margin of the whole film is 6 px** (chunk 50). Any
   future strip regeneration that widens ink by > 6 px anywhere near
   that chunk returns the film to CHECK FAILED; the table makes the
   margin a number a reviewer can watch.

## 7. Implementation notes for the lead

- Hook: in `run_film_telemetry`, after the verdict line is pushed to
  `out`, append the three tables when the flag is set; parse the flag
  in main beside the existing telemetry dispatch. No new loads: shots
  and captions are already in hand; cast presence can be accumulated
  from the audit's own samples (finer than the prototype's — see
  §4.3), beats from `data.anchors` + the same call-site knowledge the
  staging already encodes.
- The tables must print **after** the verdict line, never before:
  consumers that read a bounded prefix of the output (the adapter's
  verdict scan) keep today's behaviour even with the flag on.
- After landing, re-run W62's adapter grid once with `--tables` on as
  a belt-and-braces proof that additive output does not disturb the
  gate (expected: 8/8 unchanged).
- Prototype limits, for the record: the stdlib PNG decoder covers
  8-bit non-interlaced PNGs (all strip/plate formats in the repo) and
  raises on anything else rather than mismeasuring; the prototype's
  reconciliation exit code is a development aid and has no counterpart
  in the proposed binary behaviour (§2).
