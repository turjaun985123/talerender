# LIPSYNC-W23 — Lip-sync DATA tooling (Phase 4, seat W23)

**Delivered:** `tools/lipsync/build_visemes.py` (new tool, stdlib
only, no downloads, no new deps) + fixtures under
`tools/lipsync/fixtures/` + this note. Nothing in `src/` or any
other tool was touched; nothing committed.

**Scope, honestly:** this is the DATA half W12 specced
(CHARANIM-W12, "What lip-sync would additionally need" items 1–2).
It turns a project's orthographic `words.json` into a **viseme
track in the W15 clips data shape**. No Rust consumes it yet —
exactly like W15's clips themselves, wiring it into the evaluator
is the lead's adoption step (§6 below states what that step is).

---

## 1. What the tool does

```
words.json ──► normalise ──► pronounce ──► time phonemes ──► visemes ──► keys
              (strip punct,   (dictionary /    (weighted split   (phoneme→   (W15 clip,
               digits→words)   number / G2P)    of the word span)  viseme)    talk track)
```

```bash
python3 tools/lipsync/build_visemes.py WORDS.json [-o OUT.json] \
        [--target NAME] [--clip NAME] [--duration SECONDS] \
        [--pronunciation EXTRA.json]     # {word: [ARPAbet...]} merged over the bundled dict
python3 tools/lipsync/build_visemes.py --self-check
```

Exit codes follow the repo tools convention: 0 ok, 1 self-check
FAIL, 2 input error (one-line `ERROR:` on stderr). Output is
deterministic — regenerating the Onoda demo gives a byte-identical
file (sha256 `9f4b8d88…` on both the saved artefact and a fresh
stdout run).

### The output shape — W15 composition

The output is a scene-format-shaped JSON object carrying:

- a top-level `clips` block with ONE clip: one track,
  `target = {node: <--target>, channel: "talk"}`,
  `keys = [{t, value, ease: "linear"}, …]`;
- a minimal character **node stub** with that name
  (`type: "character"`, `state.talk: 0`) instancing the clip
  (`clip` + `clipTime {offset 0, rate 1, loop "clamp"}`) — the stub
  exists so the target resolves under W15's rules; in production
  the lead binds the same track to the real character node of a
  scene file and drops the stub;
- a top-level `lipsync` block (ignored by check_clips) holding the
  provenance an adopter needs: per-segment table
  `{start, end, viseme, talk, phoneme, word, line, source}`, the
  coverage counts, and the G2P / failed word lists.

**Channel naming — no flag needed:** W15's vocabulary already
defines `talk` as a scalar character channel
(FORMAT-EXT-CLIPS-W15 §3; `check_clips.py` CHARACTER_CHANNELS;
`src/toonfull.rs` `FullState.talk`). This tool defines **no** new
channel and uses exactly that one.

**Validation (run, not asserted):**

```
$ python3 tools/format/check_clips.py \
    tools/lipsync/fixtures/expected-lipsync.scene.json \
    tools/lipsync/fixtures/onoda-lipsync.demo.scene.json
BOTH files: [1 structure] PASS · [2 channels] PASS · [3 keys] PASS
            [4 references] PASS · [5 f32] PASS
OVERALL: PASS (5/5 checks passed)   exit 0
```

The §11 f32 law is met by construction: all key times are rounded
to milliseconds (authored-literal precision, ≤ 6 significant digits
for any film under ~1000 s) and all talk values are short decimals
from the table in §2.

Key semantics: a key at every phoneme-segment start; `ease:
"linear"` throughout — a key's ease governs the segment *ending* at
it (W15 §2.3), and lip-sync wants each openness reached **at** its
boundary, not smootherstep's slow attack. Silence (talk 0) is keyed
at a word's end when the gap to the next word is ≥ 0.04 s, at
t = 0, and at t = duration. Duplicate rounded times resolve to the
later segment; consecutive equal values collapse.

---

## 2. The viseme set and the phoneme→viseme mapping

**Provenance, stated plainly:** the *grouping* is the widely used
compact viseme grouping derived from the MPEG-4 Facial Animation
viseme table, in the form popularised by the Preston Blair phoneme
charts — phonemes that look the same on the mouth share a viseme —
collapsed from 14 MPEG-4 visemes + silence to the **15 postures
below**. The *talk amounts* are **this tool's calibration**, not
something copied from either source: `FullState.talk` is a single
openness scalar, and the painter (`src/toonfull.rs`, mouth block;
`st.talk` read at :652) draws the open mouth only when
`talk > 0.05`, with width `14 + 10·talk` and height `6 + 18·talk`.
So a bilabial closure must sit **below** 0.05 to read as closed
lips, and full open vowels sit at 1.0.

| viseme | phonemes (ARPAbet) | talk | reads as |
|---|---|---|---|
| `sil` | — (pause / failed word) | 0.0 | rest, mouth closed |
| `pp` | P B M | 0.02 | bilabial closure (below painter threshold = lips shut) |
| `ff` | F V | 0.30 | labiodental, teeth on lip |
| `th` | TH DH | 0.40 | dental, tongue to teeth |
| `dd` | T D | 0.35 | alveolar stop |
| `nn` | N L | 0.40 | alveolar nasal / lateral |
| `ss` | S Z | 0.22 | fricative, teeth near-closed |
| `kk` | K G NG | 0.50 | velar, back of mouth open |
| `ch` | CH JH SH ZH | 0.55 | postalveolar, lips slightly forward |
| `rr` | R | 0.45 | rhotic |
| `A` | AA AE AH AY AW | 1.0 | open vowels / falling diphthongs |
| `E` | EH EY ER | 0.65 | mid front vowels, rhotic vowel |
| `I` | IH IY Y | 0.50 | close front vowels / glide |
| `O` | AO OW OY | 0.85 | rounded back vowels |
| `U` | UH UW W | 0.60 | close rounded vowels / glide |

HH has no entry: it carries no mouth posture, so its time slice is
absorbed into the following segment at build time.

**Lossiness warning (the reason the segment table is kept):** a
single openness scalar cannot distinguish `pp` from `sil` (both
near-closed) nor lip *rounding* (`O`/`U` vs `A` at similar
openness). The clips track is exactly what today's painter can
express; the `lipsync.segments` table preserves the true viseme
per segment so a future mouth-shape channel (a W15 vocabulary
extension — the lead's call, not this seat's) can consume it
without rebuilding the data.

---

## 3. Pronunciation: dictionary, numbers, G2P

Three sources, in priority order, **counted separately in every
report** — a word never silently changes source:

1. **Dictionary (`dict`)** — a bundled hand-written CMUdict-style
   table (ARPAbet, stress digits omitted) covering the fixture
   vocabulary, the number words, and the common + content words of
   the Onoda narration. `--pronunciation FILE.json` merges a user
   table over it (the extension route for production vocabularies
   / a real CMUdict subset, when someone chooses to vendor one).
2. **Number expansion (`number`)** — digit tokens are expanded to
   English words first, then dictionary-pronounced: cardinals
   generally; four-digit years 1900–1999 as year pairs
   (1944 → *nineteen forty four*, 1974 → *nineteen seventy four*);
   "II" is a dictionary entry pronounced *two*, as spoken.
3. **Rule-based G2P (`g2p`)** — for anything still missing. An
   ordered scanner, fully documented in the tool's docstring:
   silent clusters (kn-/wr-/gn-, -mb), digraphs longest-first
   (tch, dge, tion, sion, ch, sh, th, ph, wh, ck, ng, nk, qu, gh,
   ee/ea, oo, ou/ow, oi/oy, ai/ay, au/aw, ar, or, er/ir/ur),
   magic-e lengthening (word-final and before past-tense -d, via a
   pre-pass), doubled-consonant collapse, context consonants
   (c/g softening, x → K S), short vowels elsewhere, final-e
   silence with syllabic -le. A token yielding no phonemes at all
   is **`failed`**: counted, listed, and emitted as a silence
   segment — never dropped quietly.

Phoneme timing inside a word is a **weighted split** of the word's
[s, e] span (vowels weight 1.5, consonants 1.0): words.json
carries no sub-word timing, so this is a duration model of last
resort and is stated as such.

**Known G2P errors on the real data** (fallback quality, visible
by design): *philippine* → F IH L IH P **AY** N (final -ine
misread as magic-i; real ≈ IY), *onoda* → AA N AA D **AE**
(final vowel too open), *quixote* in the fixture → K W IH K S OW T
(serviceable). Proper names are exactly what a pronouncing
dictionary exists for; the `--pronunciation` override is the fix
path, and the demo below deliberately does **not** use it, so the
fallback's real coverage is what gets measured.

---

## 4. Fixtures + self-check

`tools/lipsync/fixtures/`:

| file | what |
|---|---|
| `words.json` | 10 words, 2 lines: line 0 all-dictionary ("The soldier kept fighting."); line 1 mixes dictionary (*at, the*) with four words deliberately absent from the dictionary — *Zorblax, waved, purple, quixote* — to exercise G2P (incl. the magic-e-before-d path in *waved* → W EY V D) |
| `expected-lipsync.scene.json` | the tool's output for the fixture (`--target Narrator --clip lipsync-fixture`): 44 segments, 52 keys, duration 4.4 — check_clips PASS 5/5 (§1) |

```
$ python3 tools/lipsync/build_visemes.py --self-check
SELF-CHECK: PASS (dict x6, g2p x4, 52 keys, output == expected fixture)
```

The self-check regenerates from `words.json` and asserts: both
pronunciation paths exercised (≥ 5 dict, ≥ 3 G2P, the named G2P
words present), keys strictly increasing with values in [0, 1] and
eases inside the W15 vocabulary, the track target is exactly
`{Narrator, talk}`, and the whole document equals the frozen
expected fixture byte-for-value.

---

## 5. The real run — Onoda `words.json` (read-only)

Input: `~/faceless-shorts/the-soldier-who-didnt-know/data/words.json`
(146 tokens, 12 lines, narration 0.30–58.92 s). Output written
**only** to this repo as a clearly-labelled demo artefact:
`tools/lipsync/fixtures/onoda-lipsync.demo.scene.json`
(`--target narrator --clip lipsync-onoda`). The project directory
was not written to (verified: its `data/` listing is unchanged).

**Coverage (per word token):**

| source | tokens | share |
|---|---|---|
| dictionary | 133 | 91.1 % |
| number expansion | 5 | 3.4 % |
| **dictionary-grade total** | **138** | **94.5 %** |
| G2P fallback | 8 | 5.5 % |
| failed | 0 | 0 % |

The 8 G2P tokens are 5 unique words — all proper names, absent
from the bundled dictionary **deliberately** so this run measures
the fallback on real data: *hiroo* ×1, *onoda* ×4, *norio* ×1,
*suzuki* ×1, *philippine* ×1.

**Output:** 564 viseme segments → **551 keys**, clip duration
58.92 s (= last word end), key gaps 0.011–0.92 s, talk values used
= the 13 non-`sil`-collapsed table values. Segment mix (top):
A 92, nn 85, dd 76, I 63, E 44, ss 41, ff 32, O 26, U 25, rr 24,
pp 21, kk 19, th 9, ch 7. check_clips on the demo: **PASS 5/5**.

---

## 6. The engine-side remainder (for the lead)

1. **Replace the oscillator for speaking characters.**
   `FullState.talk` is today a free oscillator —
   `film.rs:223` `talk(t, on) = 0.25 + 0.75·|sin 13t|·(0.6 +
   0.4·sin 5.3t)` when `on`, else 0 — assigned at the staging call
   sites (`film.rs:1469`, `:1938`). Adoption: when a character
   node instances a lipsync clip, evaluate the clip's `talk`
   track at scene time (W15 §4 semantics: hold outside range,
   linear segments as authored) and assign that instead of
   `talk(t, …)`. The oscillator stays for shots with no clip —
   byte-identical behaviour there.
2. **Paint-cache cost is already paid where it matters.** `talk`
   is part of `state_key` (`film.rs:2147`), so any frame whose
   talk value changes repaints — but the oscillator *already*
   changes talk on essentially every frame of a speaking shot.
   The driven track changes it at most 551 times across the whole
   Onoda film (and holds between keys), so adoption is
   cost-neutral-to-better on speaking shots and free elsewhere.
3. **Speaker attribution is still missing — workaround in force:**
   neither `words.json` nor `lines.json` carries a speaker field
   (W12). This tool attributes **all** words to a single narrator
   target. Note the Onoda narration is voice-over (lines.json
   voice `bm_george`): the demo's target `narrator` is an abstract
   node name, **not** the Onoda character — binding narration
   lip-sync to an on-screen character is a staging decision, not a
   data fact. Because every segment records its source `line`,
   the multi-speaker upgrade is mechanical once staging supplies
   a line→character map: filter segments per line set, emit one
   clip per speaker, retarget.
4. **What this does not give the engine:** sub-word timing
   (weighted split, §3), mouth *shape* (scalar lossiness, §2),
   and co-articulation (each segment holds its own value; the
   linear keys approximate transitions). All three are data-model
   upgrades, not tool bugs — the segment table is the seam each
   would build on.
