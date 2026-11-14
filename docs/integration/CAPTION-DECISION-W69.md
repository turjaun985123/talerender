# Caption decision — keep the 73-chunk re-chunk, or veto back to 66? (W69)

**One decision needed at wave close.** In Wave 1 the film's caption gate
failed on 7 chunks wider than the 850 px authoring bar, and the lead
accepted them with a waiver (they are over the bar in Chrome's original
too — `docs/ab/film/CAPTION-DECISION-W92.md`). Your Wave-2 order
reopened it: the 7 chunks were split (66 → 73 chunks), the driver data
was regenerated, and the gate now reads **`FILM TELEMETRY: CHECK OK`**.
The re-chunk stands unless you veto it. This page is the whole choice.

## 1. What changed — the 7 moments

Each formerly-wide caption now shows as two shorter captions in a row,
inside the same time window. Texts verbatim from the film data
(`film/onoda.json` at HEAD vs `aa10b13`, the pre-re-chunk version):

| Shot | In the film | Was (one caption) | Now (two captions) |
|---|---|---|---|
| s02 | 0:07–0:09 | "was a Japanese soldier" | "was a Japanese" → "soldier" |
| s05a | 0:22–0:23 | "Planes dropped leaflets" | "Planes dropped" → "leaflets" |
| s07 | 0:32–0:33 | "surviving on coconuts" | "surviving" → "on coconuts" |
| s08 | 0:37–0:38 | "named Norio Suzuki" | "named Norio" → "Suzuki" |
| s09 | 0:44–0:45 | "commander ordered it." | "commander" → "ordered it." |
| s10 | 0:46–0:48 | "his old commander," | "his old" → "commander," |
| s11 | 0:52–0:54 | "and Onoda surrendered" | "and Onoda" → "surrendered" |

As a viewer: at these 7 moments the caption changes once mid-sentence
instead of staying up whole. Every piece is ≤ 728 px wide on the strip
(122+ px of headroom under the bar); before, the 7 ran 871–1051 px.

## 2. What did NOT change (verified against the data, this seat)

- **Narration and words.** The caption text stream is identical
  old vs new (each pair rejoins to its old chunk exactly); the
  `lines`, `anchors`, `duration`, `fps`, `frames` data in
  `film/onoda.json` are unchanged between `aa10b13` and HEAD.
- **When captions appear.** Each split pair exactly tiles its parent
  window (piece 1 starts where the old chunk started, piece 2 ends
  where it ended, they meet in the middle — all 7 verified to the
  float). Start/stop moments of every caption window are untouched;
  the only new events are the 7 mid-window changes above.
- **Everything else in the film.** The driver regeneration (`5e90d7e`)
  touched exactly 60 files: `film/onoda.json` + the 59 caption-strip
  PNGs it feeds. No renderer code, no scene files, frame count still
  1,811, audio untouched.
- Chunk count is the only census change: **66 → 73**.

For balance, the waiver's point still stands: the old wide captions
were never clipped — worst on-screen margin was ≈ 53 px. The 850 px
bar is a readability safe-area rule, not a frame-fit limit. Vetoing is
defensible; it is not reverting a defect fix.

## 3. Your two options

**KEEP 73 (do nothing).** The data at HEAD is final. Telemetry stays
`FILM TELEMETRY: CHECK OK` (exit 0, recorded at `5e90d7e`). The 14
golden film pins struck in Act 1 (`916a109`) were struck on this
re-chunked data and stay valid. No further work for anyone.

**VETO → back to 66.** Mechanical consequences, all of them on us:
revert `5e90d7e` (driver data + strips); revert only the caption
paths of `3a36f1f` (scene files, captions manifest, validator chunk
constant 73 → 66 — the rest of that Batch-1 commit is unrelated wave
work and stays); restore the 7 one-piece `caps` lines in the project
script and regenerate; 3 of the 14 golden pins (s07/s08/s11 mids,
struck from post-re-chunk frames) go stale and must be restored from
the pre-re-chunk candidates. Telemetry returns to
`FILM TELEMETRY: CHECK FAILED (… 7 caption failures)`, exit 1, and
the W92 waiver stands again as that verdict's recorded disposition.

## 4. Recommendation (marked as such)

**Keep 73.** The splits land where the sentence naturally breaks —
verified against the narration text: "Planes dropped | leaflets"
splits verb from object (line: "Planes dropped leaflets saying the
war was over"), and "and Onoda | surrendered" splits subject from
verb (line: "He finally gave the order, and Onoda surrendered after
29 years"). Honest caveat: two splits are width-forced mid-phrase —
"named Norio | Suzuki" cuts a name in two, "his old | commander,"
parts adjective from noun — they were the narrowest legal split
points (W92's audit), and on screen each piece still reads as a
normal caption beat. Keeping costs nothing and leaves the gate
green; vetoing spends real work to return to a permanently red gate
over a margin rule, not a visible defect.

---

*Evidence: `film/onoda.json` captions at HEAD (73) vs `git show
aa10b13:film/onoda.json` (66) — texts, windows, and non-caption keys
compared field-by-field this seat; shot ranges from
`scenes/film/film.json` caption tiling at HEAD; `git show --stat
5e90d7e` (60 files); `docs/ab/film/RECHUNK-W01.md` (the splits and
generator audit); `docs/ab/film/CAPTION-DECISION-W92.md` (the waiver
and the width arithmetic). Nothing was modified; this brief is the
only file written.*
