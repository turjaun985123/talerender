#!/usr/bin/env python3
"""build_visemes.py — lip-sync DATA tooling (W23): words.json -> a
viseme track in the W15 clips data shape (tools/format/check_clips.py
validates the output).

Reads a project `words.json` (the faceless-shorts / faster-Whisper
format: a flat array of {"w", "line", "s", "e"} — orthographic word
timings, no phonemes, no speaker) and emits ONE W15 clip carrying ONE
track:

    target: {node: <target character node name>, channel: "talk"}
    keys:   [{t, value, ease: "linear"}, ...]

`talk` is NOT a new channel: it is the scalar character channel W15's
vocabulary already defines (docs/integration/FORMAT-EXT-CLIPS-W15.md
§3, character nodes; src/toonfull.rs FullState.talk), today driven by
a free oscillator (src/film.rs talk()). This tool builds the DATA
that lets the lead replace that oscillator with a driven channel.
Nothing in src/ consumes the output yet — adoption is the lead's
step, exactly like W15 itself.

Each key's `value` is a talk amount in [0, 1] (mouth openness), so
the track is directly a talk-channel key series. Keys sit at
word/phoneme boundaries. The viseme identity behind each key is
preserved in the output's top-level "lipsync" block (segment table)
for debugging and for a future shape-aware mouth; the clips track
itself carries only what FullState.talk can express today.

Pipeline per word token:
  1. normalise: lowercase, strip surrounding punctuation, fold the
     curly apostrophe; digits are expanded to English number words
     first (see number_to_words — 4-digit years 1900-2099 are read
     as year pairs, "nineteen forty four"; other integers cardinal).
  2. pronunciation: the bundled dictionary below (ARPAbet, stress
     digits omitted), optionally overridden/extended by a user
     --pronunciation JSON file ({"word": ["AH", "N", ...]}). Words
     missing from both go through the documented rule-based G2P
     fallback (g2p() — crude by design, no downloads, no deps).
     A token that yields no phonemes at all is FAILED: it is counted,
     contributes a silence segment, and is listed in the report.
  3. phoneme timing: the word's [s, e] interval is split among its
     phonemes proportionally to weights (vowels 1.5, consonants 1.0
     — a duration model of last resort; words.json carries no
     sub-word timing). HH segments are then dropped and their time
     absorbed by the following segment: HH has no mouth posture.
  4. phoneme -> viseme + talk amount via PHONEME_TO_VISEME (15
     visemes incl. silence; see docs/integration/LIPSYNC-W23.md for
     the table and its provenance). Talk amounts are calibrated to
     the painter: toonfull.rs draws the open mouth only when
     talk > 0.05 (width 14 + 10*talk, height 6 + 18*talk), so the
     bilabial closure viseme sits at 0.02 — deliberately below the
     threshold, i.e. lips closed.
  5. keys: a key at every segment start (ease "linear" — lip-sync
     wants the value reached AT the boundary, not smootherstep's
     slow attack; W15 §2.3: a key's ease governs the segment ending
     at it). Silence (talk 0) is keyed at a word's end when the gap
     to the next word is >= 0.04 s, and at t=0 / t=duration.
     Keys are rounded to milliseconds, de-duplicated by time (the
     later segment wins a shared instant), and consecutive equal
     values collapse. All numbers under "clips" therefore obey the
     §11 f32 data law as check_clips.py enforces it (exact f32 or
     <= 6 significant digits) for any film under ~1000 s.

Speaker attribution: words.json / lines.json carry NO speaker field
(W12). Workaround, stated not hidden: ALL words are attributed to a
single narrator target (--target, default "narrator"). Because every
segment records its source `line`, a multi-speaker pass later is a
filter: build one clip per line set and retarget.

Usage:
  python3 tools/lipsync/build_visemes.py WORDS.json [-o OUT.json]
          [--target NAME] [--clip NAME] [--duration SECONDS]
          [--pronunciation EXTRA.json]
  python3 tools/lipsync/build_visemes.py --self-check

Exit codes (repo tools convention): 0 ok / self-check PASS,
1 self-check FAIL, 2 input error (one-line ERROR: on stderr).
Output is deterministic: same input -> byte-identical JSON
(sorted keys, fixed float formatting via the ms rounding above).
"""

import argparse
import json
import sys
from pathlib import Path

# ------------------------------------------------------------------
# Visemes — 15 incl. silence/rest. Grouping adapted from the
# widely used MPEG-4 facial-animation viseme set as popularised by
# the Preston Blair phoneme charts, collapsed onto the postures a
# single openness scalar (FullState.talk) can distinguish; see
# LIPSYNC-W23.md §2 for the full table + the lossiness warning.
# talk amounts are THIS tool's calibration to the painter
# (open mouth iff talk > 0.05; h = 6 + 18*talk stage units).
# ------------------------------------------------------------------

# viseme -> talk amount in [0, 1]
VISEME_TALK = {
    "sil": 0.0,    # silence / rest
    "pp": 0.02,    # bilabial closure — below the painter threshold
    "ff": 0.30,    # labiodental (teeth on lip, slight parting)
    "th": 0.40,    # dental (tongue to teeth)
    "dd": 0.35,    # alveolar stop
    "nn": 0.40,    # alveolar nasal / lateral
    "ss": 0.22,    # alveolar fricative (teeth near-closed)
    "kk": 0.50,    # velar (back of mouth open)
    "ch": 0.55,    # postalveolar (lips slightly rounded/fwd)
    "rr": 0.45,    # rhotic
    "A": 1.0,      # open vowels
    "E": 0.65,     # mid front vowels
    "I": 0.50,     # close front vowels / glide Y
    "O": 0.85,     # rounded back vowels
    "U": 0.60,     # close rounded vowels / glide W
}

# ARPAbet phoneme -> viseme. HH is deliberately absent: it is
# skipped at segment build (no mouth posture of its own).
PHONEME_TO_VISEME = {
    "P": "pp", "B": "pp", "M": "pp",
    "F": "ff", "V": "ff",
    "TH": "th", "DH": "th",
    "T": "dd", "D": "dd",
    "N": "nn", "L": "nn",
    "S": "ss", "Z": "ss",
    "K": "kk", "G": "kk", "NG": "kk",
    "CH": "ch", "JH": "ch", "SH": "ch", "ZH": "ch",
    "R": "rr",
    "AA": "A", "AE": "A", "AH": "A", "AY": "A", "AW": "A",
    "EH": "E", "EY": "E", "ER": "E",
    "IH": "I", "IY": "I", "Y": "I",
    "AO": "O", "OW": "O", "OY": "O",
    "UH": "U", "UW": "U", "W": "U",
}

VOWELS = {"AA", "AE", "AH", "AO", "AW", "AY", "EH", "ER", "EY",
          "IH", "IY", "OW", "OY", "UH", "UW"}

# ------------------------------------------------------------------
# Bundled pronunciation dictionary — ARPAbet, no stress digits.
# Hand-written CMUdict-style entries for (a) the self-check fixture
# vocabulary and (b) the Onoda narration vocabulary (the real-data
# demo in fixtures/), plus the number words. Proper names in the
# Onoda script (Hiroo, Onoda, Norio, Suzuki, Philippine) are
# DELIBERATELY absent so the real run exercises the G2P fallback —
# see the note. Extend/override with --pronunciation.
# ------------------------------------------------------------------

DICTIONARY = {
    # function / common words
    "a": ["AH"], "an": ["AH", "N"], "the": ["DH", "AH"],
    "this": ["DH", "IH", "S"], "it": ["IH", "T"], "in": ["IH", "N"],
    "on": ["AA", "N"], "to": ["T", "UW"], "for": ["F", "AO", "R"],
    "of": ["AH", "V"], "and": ["AH", "N", "D"], "or": ["AO", "R"],
    "but": ["B", "AH", "T"], "if": ["IH", "F"], "so": ["S", "OW"],
    "he": ["HH", "IY"], "his": ["HH", "IH", "Z"],
    "was": ["W", "AH", "Z"], "were": ["W", "ER"],
    "wasn't": ["W", "AH", "Z", "AH", "N", "T"],
    "would": ["W", "UH", "D"], "up": ["AH", "P"],
    "at": ["AE", "T"], "from": ["F", "R", "AH", "M"],
    "after": ["AE", "F", "T", "ER"], "until": ["AH", "N", "T", "IH", "L"],
    "only": ["OW", "N", "L", "IY"], "just": ["JH", "AH", "S", "T"],
    "now": ["N", "AW"], "still": ["S", "T", "IH", "L"],
    "never": ["N", "EH", "V", "ER"], "ever": ["EH", "V", "ER"],
    # Onoda narration vocabulary
    "soldier": ["S", "OW", "L", "JH", "ER"],
    "kept": ["K", "EH", "P", "T"],
    "fighting": ["F", "AY", "T", "IH", "NG"],
    "world": ["W", "ER", "L", "D"], "war": ["W", "AO", "R"],
    "ii": ["T", "UW"],  # "World War II" is spoken "World War Two"
    "years": ["Y", "IH", "R", "Z"], "ended": ["EH", "N", "D", "IH", "D"],
    "japanese": ["JH", "AE", "P", "AH", "N", "IY", "Z"],
    "japan": ["JH", "AH", "P", "AE", "N"],
    "sent": ["S", "EH", "N", "T"], "island": ["AY", "L", "AH", "N", "D"],
    "orders": ["AO", "R", "D", "ER", "Z"], "order": ["AO", "R", "D", "ER"],
    "ordered": ["AO", "R", "D", "ER", "D"],
    "simple": ["S", "IH", "M", "P", "AH", "L"],
    "surrender": ["S", "ER", "EH", "N", "D", "ER"],
    "surrendered": ["S", "ER", "EH", "N", "D", "ER", "D"],
    "got": ["G", "AA", "T"], "message": ["M", "EH", "S", "IH", "JH"],
    "planes": ["P", "L", "EY", "N", "Z"],
    "dropped": ["D", "R", "AA", "P", "T"],
    "leaflets": ["L", "IY", "F", "L", "AH", "T", "S"],
    "saying": ["S", "EY", "IH", "NG"], "over": ["OW", "V", "ER"],
    "thought": ["TH", "AO", "T"], "enemy": ["EH", "N", "AH", "M", "IY"],
    "trick": ["T", "R", "IH", "K"],
    "friends": ["F", "R", "EH", "N", "D", "Z"],
    "gave": ["G", "EY", "V"], "died": ["D", "AY", "D"],
    "alone": ["AH", "L", "OW", "N"],
    "lived": ["L", "IH", "V", "D"],
    "jungle": ["JH", "AH", "NG", "G", "AH", "L"],
    "decades": ["D", "EH", "K", "EY", "D", "Z"],
    "surviving": ["S", "ER", "V", "AY", "V", "IH", "NG"],
    "coconuts": ["K", "OW", "K", "AH", "N", "AH", "T", "S"],
    "bananas": ["B", "AH", "N", "AE", "N", "AH", "Z"],
    "young": ["Y", "AH", "NG"],
    "traveler": ["T", "R", "AE", "V", "AH", "L", "ER"],
    "named": ["N", "EY", "M", "D"], "found": ["F", "AW", "N", "D"],
    "him": ["HH", "IH", "M"], "refused": ["R", "IH", "F", "Y", "UW", "Z", "D"],
    "leave": ["L", "IY", "V"], "said": ["S", "EH", "D"],
    "stop": ["S", "T", "AA", "P"], "own": ["OW", "N"],
    "commander": ["K", "AH", "M", "AE", "N", "D", "ER"],
    "old": ["OW", "L", "D"],
    "bookseller": ["B", "UH", "K", "S", "EH", "L", "ER"],
    "flew": ["F", "L", "UW"],
    "finally": ["F", "AY", "N", "AH", "L", "IY"],
    "hiding": ["HH", "AY", "D", "IH", "NG"],
    "loyal": ["L", "OY", "AH", "L"],
    # number words (used by number expansion; also standalone)
    "zero": ["Z", "IH", "R", "OW"], "one": ["W", "AH", "N"],
    "two": ["T", "UW"], "three": ["TH", "R", "IY"],
    "four": ["F", "AO", "R"], "five": ["F", "AY", "V"],
    "six": ["S", "IH", "K", "S"], "seven": ["S", "EH", "V", "AH", "N"],
    "eight": ["EY", "T"], "nine": ["N", "AY", "N"],
    "ten": ["T", "EH", "N"], "eleven": ["IH", "L", "EH", "V", "AH", "N"],
    "twelve": ["T", "W", "EH", "L", "V"],
    "thirteen": ["TH", "ER", "T", "IY", "N"],
    "fourteen": ["F", "AO", "R", "T", "IY", "N"],
    "fifteen": ["F", "IH", "F", "T", "IY", "N"],
    "sixteen": ["S", "IH", "K", "S", "T", "IY", "N"],
    "seventeen": ["S", "EH", "V", "AH", "N", "T", "IY", "N"],
    "eighteen": ["EY", "T", "IY", "N"],
    "nineteen": ["N", "AY", "N", "T", "IY", "N"],
    "twenty": ["T", "W", "EH", "N", "T", "IY"],
    "thirty": ["TH", "ER", "T", "IY"], "forty": ["F", "AO", "R", "T", "IY"],
    "fifty": ["F", "IH", "F", "T", "IY"], "sixty": ["S", "IH", "K", "S", "T", "IY"],
    "seventy": ["S", "EH", "V", "AH", "N", "T", "IY"],
    "eighty": ["EY", "T", "IY"], "ninety": ["N", "AY", "N", "T", "IY"],
    "hundred": ["HH", "AH", "N", "D", "R", "AH", "D"],
    "thousand": ["TH", "AW", "Z", "AH", "N", "D"],
}

_ONES = ["zero", "one", "two", "three", "four", "five", "six", "seven",
         "eight", "nine", "ten", "eleven", "twelve", "thirteen",
         "fourteen", "fifteen", "sixteen", "seventeen", "eighteen",
         "nineteen"]
_TENS = ["", "", "twenty", "thirty", "forty", "fifty", "sixty",
         "seventy", "eighty", "ninety"]


def _cardinal_words(n):
    """English cardinal word list for 0 <= n <= 9999 (no 'and')."""
    if n < 20:
        return [_ONES[n]]
    if n < 100:
        w = [_TENS[n // 10]]
        if n % 10:
            w.append(_ONES[n % 10])
        return w
    if n < 1000:
        w = [_ONES[n // 100], "hundred"]
        if n % 100:
            w += _cardinal_words(n % 100)
        return w
    w = _cardinal_words(n // 1000) + ["thousand"]
    if n % 1000:
        w += _cardinal_words(n % 1000)
    return w


def number_to_words(token):
    """Digit token -> spoken word list. 1900-2099 four-digit numbers
    are read as year pairs (the narration case: 1944 -> 'nineteen
    forty four', 1974, 1945); 2000-2009 as 'two thousand N'; all
    other integers cardinal. Returns None if not a plain integer."""
    if not token.isdigit():
        return None
    n = int(token)
    if len(token) == 4 and 1900 <= n <= 1999:
        hi, lo = divmod(n, 100)
        if lo == 0:
            return _cardinal_words(hi) + ["hundred"]
        if lo < 10:
            return _cardinal_words(hi) + [_ONES[lo]]
        return _cardinal_words(hi) + _cardinal_words(lo)
    if n > 9999:
        return None  # out of scope for the expander -> caller fails it
    return _cardinal_words(n)


# ------------------------------------------------------------------
# Rule-based G2P fallback (for words missing from the dictionary).
# Ordered, documented rules — a scanner over the lowercased word:
#
#   0. apostrophes are dropped ("wasn't" handled by the dictionary,
#      but e.g. "don't" -> "dont").
#   1. silent clusters first at the cursor: kn- -> N, wr- -> R,
#      gn- -> N (word-initial); -mb word-final -> M.
#   2. multi-letter patterns, longest first at the cursor:
#      tch -> CH, dge -> JH, tion -> SH AH N, sion -> ZH AH N,
#      ch -> CH, sh -> SH, th -> TH, ph -> F, wh -> W, ck -> K,
#      ng -> NG, nk -> NG K, qu -> K W, gh -> (silent),
#      ee/ea -> IY, oo -> UW, ou/ow -> AW, oi/oy -> OY,
#      ai/ay -> AY, au/aw -> AO, ar -> AA R, er/ir/ur -> ER
#      (before a consonant or word-finally), or -> AO R.
#   3. magic-e: a vowel followed by one consonant + word-final 'e'
#      is long (a->EY, e->IY, i->AY, o->OW, u->Y UW) and the final
#      'e' is silent; a word-final 'e' after any other pattern is
#      silent too (except after another vowel sound, e.g. -le keeps
#      a syllabic AH L: purple -> ... AH L).
#   4. doubled consonants collapse to one (bb dd ff gg ll mm nn pp
#      rr ss tt zz), processed by the single-letter rules.
#      (Magic-e itself is applied by the g2p_word pre-pass before
#      this scanner runs — see g2p_word.)
#   5. single letters: a AE, e EH, i IH, o AA, u AH, y -> Y initially
#      / IY word-finally after a consonant / IH elsewhere;
#      c -> S before e/i/y else K; g -> JH before e/i/y else G;
#      x -> K S; the rest map to their ARPAbet namesake.
#      s -> S (plural voicing is NOT modelled — the dictionary
#      carries the common words where it matters).
#   6. anything left that is not a-z yields no phoneme.
#
# This is a fallback, not a pronouncing dictionary: expect errors
# on names and irregular words. Its failures are VISIBLE (the
# report counts G2P words and lists them), never silent.
# ------------------------------------------------------------------

_SINGLE = {
    "b": ["B"], "d": ["D"], "f": ["F"], "h": ["HH"], "j": ["JH"],
    "k": ["K"], "l": ["L"], "m": ["M"], "n": ["N"], "p": ["P"],
    "r": ["R"], "s": ["S"], "t": ["T"], "v": ["V"], "w": ["W"],
    "z": ["Z"],
}
_LONG_VOWEL = {"a": ["EY"], "e": ["IY"], "i": ["AY"], "o": ["OW"],
               "u": ["Y", "UW"]}
_SHORT_VOWEL = {"a": ["AE"], "e": ["EH"], "i": ["IH"], "o": ["AA"],
                "u": ["AH"]}


def g2p(word):
    """Crude rule-based grapheme->phoneme for one normalised word.
    Returns a list of ARPAbet phonemes (possibly empty)."""
    w = word.replace("'", "")
    out = []
    i, n = 0, len(w)

    def peek(s):
        return w.startswith(s, i)

    while i < n:
        c = w[i]
        rest = w[i:]
        # 1. silent initial clusters
        if i == 0 and (peek("kn") or peek("gn")):
            i += 1
            continue
        if i == 0 and peek("wr"):
            i += 1
            continue
        # 2. multi-letter patterns, longest first
        if peek("tch"):
            out.append("CH"); i += 3; continue
        if peek("dge"):
            out.append("JH"); i += 3; continue
        if peek("tion"):
            out += ["SH", "AH", "N"]; i += 4; continue
        if peek("sion"):
            out += ["ZH", "AH", "N"]; i += 4; continue
        if peek("ch"):
            out.append("CH"); i += 2; continue
        if peek("sh"):
            out.append("SH"); i += 2; continue
        if peek("th"):
            out.append("TH"); i += 2; continue
        if peek("ph"):
            out.append("F"); i += 2; continue
        if peek("wh"):
            out.append("W"); i += 2; continue
        if peek("ck"):
            out.append("K"); i += 2; continue
        if peek("ng"):
            out.append("NG"); i += 2; continue
        if peek("nk"):
            out += ["NG", "K"]; i += 2; continue
        if peek("qu"):
            out += ["K", "W"]; i += 2; continue
        if peek("gh"):
            i += 2; continue
        if peek("mb") and i + 2 == n:
            out.append("M"); i += 2; continue
        if peek("ee") or peek("ea"):
            out.append("IY"); i += 2; continue
        if peek("oo"):
            out.append("UW"); i += 2; continue
        if peek("ou") or peek("ow"):
            out.append("AW"); i += 2; continue
        if peek("oi") or peek("oy"):
            out.append("OY"); i += 2; continue
        if peek("ai") or peek("ay"):
            out.append("AY"); i += 2; continue
        if peek("au") or peek("aw"):
            out.append("AO"); i += 2; continue
        if peek("ar"):
            out += ["AA", "R"]; i += 2; continue
        if peek("or"):
            out += ["AO", "R"]; i += 2; continue
        if (peek("er") or peek("ir") or peek("ur")) and \
                (i + 2 == n or w[i + 2] not in "aeiou"):
            out.append("ER"); i += 2; continue
        # 4. doubled consonants collapse: drop one twin here; the
        #    remaining letter is processed by the normal rules on
        #    the next pass (so 'cc' before 'e' still softens, etc.)
        if c not in "aeiouhwy" and i + 1 < n and w[i + 1] == c:
            i += 1
            continue
        # 3. vowels (magic-e is handled by the g2p_word pre-pass;
        #    a vowel here is short unless a digraph above claimed it)
        if c in "aeiou":
            out += _SHORT_VOWEL[c]
            i += 1
            continue
        if c == "y":
            if i == 0:
                out.append("Y")
            elif i == n - 1:
                out.append("IY")
            else:
                out.append("IH")
            i += 1
            continue
        if c == "e" and i == n - 1:
            i += 1  # final silent e (non magic-e contexts)
            continue
        if c == "c":
            nxt = w[i + 1] if i + 1 < n else ""
            out.append("S" if nxt in "eiy" else "K")
            i += 1
            continue
        if c == "g":
            nxt = w[i + 1] if i + 1 < n else ""
            out.append("JH" if nxt in "eiy" else "G")
            i += 1
            continue
        if c == "x":
            out += ["K", "S"]; i += 1; continue
        if c in _SINGLE:
            out += _SINGLE[c]
            i += 1
            continue
        i += 1  # 6. unknown char: no phoneme
    return out


def _magic_e(stem, vi):
    """stem has the silent e removed; stem[vi] is the vowel to
    lengthen. head/tail go through the plain scanner."""
    head = g2p(stem[:vi]) if vi > 0 else []
    tail = g2p(stem[vi + 1:]) if vi + 1 < len(stem) else []
    return head + _LONG_VOWEL[stem[vi]] + tail


def g2p_word(word):
    """G2P with the magic-e pre-pass: a vowel followed by one
    consonant + a silent 'e' is long. Two silent-e positions are
    recognised: word-final ('wave' -> 'wav' + EY) and before the
    past-tense '-d' ('waved' -> 'wavd' with a long a). Implemented
    as a wrapper so the scanner in g2p() stays a simple
    left-to-right pass."""
    w = word.replace("'", "")
    if len(w) >= 3 and w.endswith("e") and w[-2] not in "aeiou" \
            and w[-3] in "aeiou" and not w.endswith(("ee", "oe", "ue")):
        stem = w[:-1]  # drop silent e
        return _magic_e(stem, len(stem) - 2)
    if len(w) >= 4 and w.endswith("ed") and w[-3] not in "aeiou" \
            and w[-4] in "aeiou":
        stem = w[:-2] + "d"  # drop the silent e of '-ed'
        return _magic_e(stem, len(stem) - 3)
    if w.endswith("e") and len(w) >= 2 and w[-2] not in "aeiou":
        # plain final silent e (e.g. 'purple' -> 'purpl' keeps the
        # syllabic l: append AH before final L if stem ends in 'l'
        # after a consonant)
        stem = w[:-1]
        res = g2p(stem)
        if stem.endswith("l") and len(stem) >= 2 \
                and stem[-2] not in "aeiou":
            res = res[:-1] + ["AH", "L"] if res and res[-1] == "L" \
                else res + ["AH", "L"]
        return res
    return g2p(w)


# ------------------------------------------------------------------
# Build
# ------------------------------------------------------------------

def r3(x):
    """Round to milliseconds — the §11-safe authored precision."""
    return round(float(x) + 1e-9, 3)


def normalise(raw):
    """words.json token -> lookup form: lowercase, strip surrounding
    punctuation (inner apostrophes kept), fold curly apostrophe."""
    w = raw.strip().lower().replace("’", "'")
    return w.strip(".,;:!?\"“”()[]")


def pronounce(norm, extra_dict):
    """-> (phonemes, source) where source is 'dict' | 'number' |
    'g2p' | 'failed'."""
    num_words = number_to_words(norm)
    if num_words is not None:
        phones = []
        for nw in num_words:
            p = extra_dict.get(nw) or DICTIONARY.get(nw)
            if p is None:
                p = g2p_word(nw)
            phones += p
        return phones, "number"
    p = extra_dict.get(norm) or DICTIONARY.get(norm)
    if p is not None:
        return list(p), "dict"
    phones = g2p_word(norm)
    if phones:
        return phones, "g2p"
    return [], "failed"


def build(words, target="narrator", clip_name="lipsync",
          duration=None, extra_dict=None):
    """words: [{w, line, s, e}] -> (doc, report dict)."""
    extra_dict = extra_dict or {}
    segments = []  # {start,end,viseme,talk,word,phoneme,line,source}
    counts = {"dict": 0, "number": 0, "g2p": 0, "failed": 0}
    failed_words, g2p_words = [], []

    for wd in words:
        raw = str(wd["w"])
        norm = normalise(raw)
        s, e = float(wd["s"]), float(wd["e"])
        line = wd.get("line")
        if e <= s or not norm:
            counts["failed"] += 1
            failed_words.append(raw)
            segments.append({"start": r3(s), "end": r3(e),
                             "viseme": "sil", "talk": 0.0,
                             "word": raw, "phoneme": None,
                             "line": line, "source": "failed"})
            continue
        subwords = number_to_words(norm)
        if subwords is not None:
            # split the interval equally across the number's words
            per = (e - s) / len(subwords)
            phones, source = [], "number"
            for k, nw in enumerate(subwords):
                p = extra_dict.get(nw) or DICTIONARY.get(nw) \
                    or g2p_word(nw)
                phones += [(ph, s + k * per, s + (k + 1) * per)
                           for ph in p]
            counts["number"] += 1
            word_segments = _phone_segments(phones, raw, line, source)
        else:
            phones, source = pronounce(norm, extra_dict)
            counts[source] += 1
            if source == "g2p":
                g2p_words.append(norm)
            if source == "failed":
                failed_words.append(raw)
                word_segments = [{"start": r3(s), "end": r3(e),
                                  "viseme": "sil", "talk": 0.0,
                                  "word": raw, "phoneme": None,
                                  "line": line, "source": "failed"}]
            else:
                timed = _split_word(phones, s, e)
                word_segments = _phone_segments(
                    timed, raw, line, source)
        segments.extend(word_segments)

    segments.sort(key=lambda sg: (sg["start"], sg["end"]))
    dur = r3(duration if duration is not None
             else max((float(w["e"]) for w in words), default=0.0))
    keys = _keys_from_segments(segments, dur)

    doc = {
        "format": "talerender-scene",
        "version": 2,
        "name": "lipsync — %s (built by tools/lipsync/build_visemes.py)"
                % clip_name,
        "nodes": [{
            "name": target,
            "type": "character",
            "clip": clip_name,
            "clipTime": {"offset": 0.0, "rate": 1.0, "loop": "clamp"},
            "state": {"talk": 0},
        }],
        "clips": {
            clip_name: {
                "duration": dur,
                "tracks": [{
                    "target": {"node": target, "channel": "talk"},
                    "keys": keys,
                }],
            }
        },
        "lipsync": {
            "tool": "tools/lipsync/build_visemes.py (W23)",
            "speakerWorkaround": "single narrator — words.json and "
                                 "lines.json carry no speaker field "
                                 "(W12); every word is attributed to "
                                 "the one --target node",
            "target": target,
            "clipName": clip_name,
            "counts": counts,
            "g2pWords": sorted(set(g2p_words)),
            "failedWords": failed_words,
            "segments": segments,
        },
    }
    report = {"counts": counts, "keys": len(keys), "duration": dur,
              "segments": len(segments),
              "g2pWords": sorted(set(g2p_words)),
              "failedWords": failed_words}
    return doc, report


def _split_word(phones, s, e):
    """Weighted split of [s, e] over phonemes (vowels 1.5, else 1)."""
    weights = [1.5 if p in VOWELS else 1.0 for p in phones]
    total = sum(weights) or 1.0
    out, t = [], s
    for p, wgt in zip(phones, weights):
        nxt = t + (e - s) * wgt / total
        out.append((p, t, nxt))
        t = nxt
    if out:
        out[-1] = (out[-1][0], out[-1][1], e)
    return out


def _phone_segments(timed, raw, line, source):
    """timed: [(phoneme, s, e)] -> viseme segments, HH dropped with
    its time absorbed by the next segment (or the previous if last)."""
    segs = []
    pending_start = None
    for ph, s, e in timed:
        if ph == "HH":
            if pending_start is None:
                pending_start = s
            continue
        start = pending_start if pending_start is not None else s
        pending_start = None
        vis = PHONEME_TO_VISEME[ph]
        segs.append({"start": r3(start), "end": r3(e), "viseme": vis,
                     "talk": VISEME_TALK[vis], "word": raw,
                     "phoneme": ph, "line": line, "source": source})
    if pending_start is not None and segs:
        segs[-1]["end"] = r3(timed[-1][2])
    return segs


# Variant A closed-exit threshold (LIPSYNC-BIND-W86 §1.1; user
# ruling 2026-11-15, DECISIONS-W94 decision 4): a key whose
# predecessor key is closed (value <= CLOSED_EXIT_MAX) is emitted
# with ease "step", so the §15 evaluator holds the closed posture
# across the silence/closure and jumps at the next phoneme. The
# threshold is the painter's own open threshold.
CLOSED_EXIT_MAX = 0.05  # the painter's open threshold,
                        # src/toonfull.rs:694


def _keys_from_segments(segments, dur):
    by_t = {}

    def put(t, v):
        by_t[r3(t)] = v

    put(0.0, 0.0)
    prev_end = 0.0
    for sg in segments:
        if sg["start"] - prev_end >= 0.04:
            put(prev_end, 0.0)  # silence across a real gap
        put(sg["start"], sg["talk"])
        prev_end = max(prev_end, sg["end"])
    put(dur, 0.0)
    keys, last_v = [], None
    for t in sorted(by_t):
        if t < 0 or t > dur:
            continue
        v = by_t[t]
        if v == last_v:
            continue  # collapse consecutive equal values
        # Closed-exit rule (Variant A): the predecessor relation is
        # read after the collapse, and the collapse cannot break it
        # (a dropped key has the same value as its kept predecessor,
        # so a collapsed-away closed key leaves an equally closed
        # predecessor). No key times or values change — ease only.
        keys.append({"t": t, "value": v,
                     "ease": "step" if (last_v is not None
                                        and last_v <= CLOSED_EXIT_MAX)
                             else "linear"})
        last_v = v
    if keys and keys[-1]["value"] != 0.0:
        keys.append({"t": dur, "value": 0.0,
                     "ease": "step" if keys[-1]["value"]
                                        <= CLOSED_EXIT_MAX
                             else "linear"})
    return keys


# ------------------------------------------------------------------
# Self-check: dictionary path, G2P path, failure honesty, shape.
# ------------------------------------------------------------------

def self_check():
    here = Path(__file__).resolve().parent
    fix = here / "fixtures"
    words = json.loads((fix / "words.json").read_text())
    expected = json.loads((fix / "expected-lipsync.scene.json")
                          .read_text())
    doc, report = build(words, target="Narrator",
                        clip_name="lipsync-fixture")
    failures = []

    def check(cond, msg):
        if not cond:
            failures.append(msg)

    check(report["counts"].get("dict", 0) >= 5,
          "dictionary path not exercised: counts=%r" % report["counts"])
    check(report["counts"].get("g2p", 0) >= 3,
          "G2P path not exercised: counts=%r" % report["counts"])
    check("zorblax" in report["g2pWords"]
          and "quixote" in report["g2pWords"],
          "fixture G2P words missing: %r" % report["g2pWords"])
    keys = doc["clips"]["lipsync-fixture"]["tracks"][0]["keys"]
    times = [k["t"] for k in keys]
    check(times == sorted(times) and len(set(times)) == len(times),
          "keys not strictly increasing")
    check(all(0.0 <= k["value"] <= 1.0 for k in keys),
          "talk value outside [0, 1]")
    check(all(k["ease"] in ("linear", "smoother", "smootherstep",
                            "step", "easeOutCubic", "eo",
                            "easeInOutCubic", "eio", "easeOutBack",
                            "eob") for k in keys),
          "ease outside the W15 vocabulary")
    check(doc["clips"]["lipsync-fixture"]["tracks"][0]["target"]
          == {"node": "Narrator", "channel": "talk"},
          "track target is not the W15 talk channel")
    check(doc == expected,
          "output differs from fixtures/expected-lipsync.scene.json "
          "(regenerate deliberately and re-inspect; do not bless "
          "blindly)")
    if failures:
        print("SELF-CHECK: FAIL")
        for f in failures:
            print("  - %s" % f)
        return 1
    print("SELF-CHECK: PASS (dict x%d, g2p x%d, %d keys, "
          "output == expected fixture)"
          % (report["counts"]["dict"], report["counts"]["g2p"],
             report["keys"]))
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(
        description="words.json -> W15 clips talk track (visemes)")
    ap.add_argument("words", nargs="?", help="project words.json")
    ap.add_argument("-o", "--out", help="output scene JSON path")
    ap.add_argument("--target", default="narrator",
                    help="character node name the talk track targets "
                         "(default: narrator — the single-narrator "
                         "workaround; see the module docstring)")
    ap.add_argument("--clip", default="lipsync", help="clip name")
    ap.add_argument("--duration", type=float, default=None,
                    help="clip duration (default: last word end)")
    ap.add_argument("--pronunciation", default=None,
                    help="JSON {word: [phonemes]} merged over the "
                         "bundled dictionary")
    ap.add_argument("--self-check", action="store_true",
                    help="run the fixture self-check and exit")
    args = ap.parse_args(argv)

    if args.self_check:
        return self_check()
    if not args.words:
        print("ERROR: no words.json given (or use --self-check)",
              file=sys.stderr)
        return 2
    try:
        words = json.loads(Path(args.words).read_text())
    except (OSError, ValueError) as e:
        print("ERROR: cannot read words.json (%s)" % e, file=sys.stderr)
        return 2
    if not isinstance(words, list) or not words or not all(
            isinstance(w, dict) and "w" in w and "s" in w and "e" in w
            for w in words):
        print("ERROR: words.json is not a [{w, line, s, e}] array",
              file=sys.stderr)
        return 2
    extra = {}
    if args.pronunciation:
        try:
            extra = json.loads(Path(args.pronunciation).read_text())
        except (OSError, ValueError) as e:
            print("ERROR: cannot read pronunciation file (%s)" % e,
                  file=sys.stderr)
            return 2
    doc, report = build(words, target=args.target, clip_name=args.clip,
                        duration=args.duration, extra_dict=extra)
    text = json.dumps(doc, indent=1, sort_keys=True) + "\n"
    if args.out:
        Path(args.out).write_text(text)
    else:
        sys.stdout.write(text)
    c = report["counts"]
    total = sum(c.values())
    print("words: %d (dict %d, number %d, g2p %d, failed %d) · "
          "segments: %d · keys: %d · duration: %s"
          % (total, c["dict"], c["number"], c["g2p"], c["failed"],
             report["segments"], report["keys"], report["duration"]),
          file=sys.stderr)
    if c["g2p"]:
        print("g2p words: %s" % ", ".join(report["g2pWords"]),
              file=sys.stderr)
    if c["failed"]:
        print("FAILED words (silence emitted): %s"
              % ", ".join(report["failedWords"]), file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
