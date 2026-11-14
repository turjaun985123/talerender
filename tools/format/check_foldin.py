#!/usr/bin/env python3
"""check_foldin.py — §15 fold-in diff-checker.

Purpose: when a paste-ready draft section (by default
docs/integration/FORMAT15-DRAFT-W31.md, the §15 prefabs + clips
text) is folded into a target specification document (by default
docs/scene-format.md), prove that no clause of the draft was
silently dropped or altered in the fold. This tool judges the
PRESENCE of normative content only — never prose quality,
never style, never whether the rule itself is a good rule.

Census (extracted from the draft's paste text — the region
between the first two standalone `---` rules; if a draft has no
such pair, the whole file is the paste text):

  H  every numbered heading in the paste text
     (`## 15. ...`, `### 15.1 ...`, ...; any leading number,
     not just 15).
  N  every normative sentence: a sentence containing at least
     one keyword from the normative keyword set below.
  R  every named rule from the curated NAMED_RULES table in
     this file (the reservation guard, the ease remap,
     wrap-on-instance / duration-on-clip, the colour lane, the
     step side channel, euler-only rotation, verbatim
     substitution, transforms-compose, pid stripping, cycle
     rejection, the R3 grade.time rejection, no shadowing, the
     depth guard, the poseTrack conversion). A named rule is a
     set of phrase groups; a group is satisfied when one of its
     alternative phrases' content tokens all appear in a single
     target sentence.

Normative keyword set (case-insensitive, word-boundaried):
  MUST (`must`), MUST-NOT (`must not` / `may not`),
  NEVER (`never`), ALWAYS (`always`), FORBIDDEN (`forbidden`,
  `forbids`), HARD-ERROR (`hard error`), REQUIRED (`required`).
The strong classes — NEVER, ALWAYS, FORBIDDEN, HARD-ERROR,
MUST-NOT — are the ones whose loss is a weakening: if the best
matching target sentence keeps the content but drops a strong
class the source sentence carried, the clause is reported
ALTERED-SUSPECT, not FOUND.

Matching rule (defined precisely; this is the whole contract):

  * Text is normalised for comparison only: lowercased,
    markdown emphasis/backticks stripped, tokenised to
    [a-z0-9] runs, stopwords removed. Line numbers reported are
    the target's real line numbers (the start line of the
    paragraph/list-item/table-row the sentence came from).
  * Headings: FOUND when the target carries a heading with the
    same number and >= 0.6 title-token containment; ALTERED-
    SUSPECT when the number exists with a divergent title;
    MISSING when no heading carries the number.
  * Normative sentences: containment = |source content tokens
    present in the target sentence| / |source content tokens|,
    scored against every target sentence (a same-section
    candidate wins ties within 0.02 — reorganisation across
    sections is tolerated, and the reported line shows where
    the clause actually landed).
      containment >= 0.75 and no strong class lost -> FOUND
      containment >= 0.75 but a strong class lost  -> ALTERED-SUSPECT
      floor <= containment < 0.75                  -> ALTERED-SUSPECT
      containment < floor                          -> MISSING
    where floor is 0.40, raised to 0.50 for short sentences
    (<= 6 content tokens): a dropped short clause can share a
    stock phrase like "hard error" (2 of 5 tokens = 0.40) with
    a surviving sibling clause, and that coincidence must read
    MISSING, not partially present.
  * Named rules: FOUND when every phrase group is satisfied;
    ALTERED-SUSPECT when at least half the groups are;
    MISSING otherwise. The reported line is the first satisfied
    group's sentence line.

False-negative behaviour (known, accepted, stated):
  * A normative statement carrying NONE of the keywords above
    is not an N-item. It is covered only if it is also a named
    rule (several are: e.g. "Substitution is verbatim — no
    arithmetic" is keyword-less and is census item R8, not an
    N-item). A keyword-less, unnamed normative sentence in a
    future draft would be invisible to this checker.
  * A clause reworded so thoroughly that fewer than 40% of its
    content tokens survive reads MISSING even if a human would
    call it the same rule — the checker is a tripwire for
    review, not a semantic judge.
  * Conversely (false positive direction): a target sentence
    that keeps >= 75% of a clause's tokens and its strong
    keywords reads FOUND even if a qualifier elsewhere in the
    paragraph was cut. Paragraph-level context is not compared.

Exit codes (repo tool convention): 0 = every census item FOUND;
1 = any item ALTERED-SUSPECT or MISSING; 2 = input error (one
`ERROR:` line on stderr, no report).

Usage:
  python3 tools/format/check_foldin.py [TARGET]
      [--draft=DRAFT] [--rules=off]
Defaults resolve against the repo root (two directories up
from this file): draft docs/integration/FORMAT15-DRAFT-W31.md,
target docs/scene-format.md. `--rules=off` skips the named-rule
census (which is specific to the real §15) — used by the
self-test fixtures, whose mini draft carries its own clauses
but not the §15 named rules.
"""

import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True

# ---------------------------------------------------------------- text utils

STOPWORDS = {
    "the", "a", "an", "and", "or", "of", "to", "in", "on", "at",
    "by", "for", "with", "without", "from", "into", "as", "is",
    "are", "be", "been", "it", "its", "that", "this", "these",
    "those", "than", "then", "so", "such", "any", "each", "per",
}

KEYWORD_PATTERNS = [
    ("MUST-NOT", re.compile(r"\bmust not\b|\bmay not\b", re.I)),
    ("NEVER", re.compile(r"\bnever\b", re.I)),
    ("ALWAYS", re.compile(r"\balways\b", re.I)),
    ("FORBIDDEN", re.compile(r"\bforbidden\b|\bforbids\b", re.I)),
    ("HARD-ERROR", re.compile(r"\bhard error", re.I)),
    ("REQUIRED", re.compile(r"\brequired\b", re.I)),
    ("MUST", re.compile(r"\bmust\b", re.I)),
]
STRONG = {"MUST-NOT", "NEVER", "ALWAYS", "FORBIDDEN", "HARD-ERROR"}

TOKEN_RE = re.compile(r"[a-z0-9]+")
HEADING_RE = re.compile(r"^(#{2,4})\s+(\d+(?:\.\d+)*)\.?\s+(.*\S)\s*$")
LIST_RE = re.compile(r"^\s*(?:[-*+]|\d+\.)\s+")


def tokens(text):
    return [t for t in TOKEN_RE.findall(text.lower()) if t not in STOPWORDS]


def content_set(text):
    return set(tokens(text))


def keyword_classes(text):
    return {name for name, pat in KEYWORD_PATTERNS if pat.search(text)}


def split_sentences(unit):
    parts = re.split(r"(?<=[.!?])\s+(?=[A-Z0-9`\"“*])", unit.strip())
    return [p.strip() for p in parts if p.strip()]


def extract_units(lines, first_line_no):
    """Yield (text, start_line, section) paragraph-ish units.

    Code fences are skipped. Table rows are their own units.
    A list item starts a new unit; other non-blank lines continue
    the current unit. `section` is the nearest preceding numbered
    heading's number (or None).
    """
    units = []
    section = None
    buf = []
    buf_line = None
    in_fence = False

    def flush():
        nonlocal buf, buf_line
        if buf:
            units.append((" ".join(buf), buf_line, section))
        buf = []
        buf_line = None

    for i, raw in enumerate(lines):
        line_no = first_line_no + i
        line = raw.rstrip("\n")
        if line.strip().startswith("```"):
            flush()
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        m = HEADING_RE.match(line)
        if m:
            flush()
            section = m.group(2)
            continue
        if not line.strip():
            flush()
            continue
        if line.lstrip().startswith("|"):
            flush()
            # A row immediately followed by a |---|---| separator row
            # is a table HEADER, not prose — skip it (its cells are
            # column names, and "value must be" is not a clause).
            nxt = lines[i + 1] if i + 1 < len(lines) else ""
            if nxt.lstrip().startswith("|") and all(
                    re.fullmatch(r":?-{2,}:?", c.strip() or "-")
                    for c in nxt.strip().strip("|").split("|")):
                continue
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            units.append((" ".join(cells), line_no, section))
            continue
        if LIST_RE.match(line) or line.lstrip().startswith(">"):
            flush()
            buf = [LIST_RE.sub("", line).strip()]
            buf_line = line_no
            continue
        if buf:
            buf.append(line.strip())
        else:
            buf = [line.strip()]
            buf_line = line_no
    flush()
    return units


def extract_sentences(lines, first_line_no):
    out = []
    for text, line_no, section in extract_units(lines, first_line_no):
        for sent in split_sentences(text):
            out.append({
                "text": sent,
                "tokens": content_set(sent),
                "classes": keyword_classes(sent),
                "line": line_no,
                "section": section,
            })
    return out


def extract_headings(lines, first_line_no):
    out = []
    for i, raw in enumerate(lines):
        m = HEADING_RE.match(raw.rstrip("\n"))
        if m:
            out.append({
                "number": m.group(2),
                "title": m.group(3),
                "tokens": content_set(m.group(3)),
                "line": first_line_no + i,
            })
    return out


def paste_region(lines):
    """Return (paste_lines, first_line_no_1based) for a draft:
    between the first two standalone `---` rules, else whole file."""
    seps = [i for i, ln in enumerate(lines) if ln.strip() == "---"]
    if len(seps) >= 2:
        start = seps[0] + 1
        return lines[start:seps[1]], start + 1
    return lines, 1


# ---------------------------------------------------------------- named rules
# Each rule: (id, name, draft section, [phrase groups]); a group is
# a list of alternative phrases — satisfied when one phrase's
# content tokens are all present in a single target sentence.

NAMED_RULES = [
    ("R-guard", "reservation guard: hard-error naming the extension, no silent drop", "15.1", [
        ["hard error"],
        ["naming the unsupported extension", "names the unsupported extension"],
        ["silently dropping", "silently ignoring", "silently dropped", "silently ignore"],
    ]),
    ("R-guard-film", "guard applies to every node-walking consumer, film.rs included", "15.1", [
        ["every consumer that walks nodes", "applies to every consumer"],
        ["film.rs"],
    ]),
    ("R-noshadow", "prefab scope: no shadowing — a name defined twice is a hard error", "15.2", [
        ["no shadowing"],
        ["defined by two sources", "name defined by two sources"],
        ["hard error"],
    ]),
    ("R-pid", "pid is stripped during expansion and never reaches the renderer", "15.2", [
        ["pid"],
        ["stripped during expansion", "strip every pid"],
        ["never reaches the renderer"],
    ]),
    ("R-subst", "substitution is verbatim — no arithmetic, no interpolation, no expressions", "15.3", [
        ["substitution is verbatim", "replaced by a deep copy"],
        ["no arithmetic"],
        ["no string interpolation", "no expressions"],
    ]),
    ("R-override", "overrides deep-merge by pid; params replace wholesale", "15.4", [
        ["deep-merge", "deep merge", "deep-merged"],
        ["replace wholesale", "replaces wholesale", "not merged"],
    ]),
    ("R-transform", "transforms compose; they never replace", "15.4", [
        ["transforms compose"],
        ["never replace", "they never replace", "never replaced"],
    ]),
    ("R-cycles", "prefab reference cycles are hard errors printing the cycle path", "15.5", [
        ["cycle"],
        ["prints the cycle path", "error prints the cycle"],
        ["hard error"],
    ]),
    ("R-depth", "expansion depth guard (reference validator uses 64)", "15.5", [
        ["depth guard"],
        ["64"],
    ]),
    ("R-gradetime", "post.grade.time is NOT a channel — rejected by name (R3)", "15.7", [
        ["grade.time", "grade time"],
        ["rejected by name", "deliberately not a channel", "is not a channel"],
    ]),
    ("R-posetrack", "poseTrack conversion: hold key + arrival key, evaluates identically", "15.8", [
        ["posetrack"],
        ["two clip keys", "two** clip keys"],
        ["hold key"],
    ]),
    ("R-fire", "fire flicker over the R1 basis 4.6 must not migrate into clips", "15.8", [
        ["fire-flicker", "fire flicker"],
        ["4.6"],
    ]),
    ("R-colour", "colour lane `Color` holds linear-space [f32; 3]; files author #RRGGBB", "15.9", [
        ["Color"],
        ["linear-space [f32; 3]", "linear [f32; 3]", "linear-space"],
        ["#RRGGBB", "rrggbb"],
    ]),
    ("R-step", "step channels ride a side channel; no interpolation", "15.9", [
        ["step"],
        ["side channel"],
        ["no interpolation", "never enter a numeric track", "hold the leaving"],
    ]),
    ("R-ease", "ease remap: format key[i+1].ease -> anim key[i].ease (arriving key)", "15.9", [
        ["remap", "remaps", "remapped"],
        ["key[i+1].ease", "key i+1 ease"],
        ["arriving key", "segment ending at that key", "ending at this key"],
    ]),
    ("R-wrapdur", "duration lives on the clip; wrap lives on the instance", "15.9", [
        ["duration lives on the clip", "duration on the clip"],
        ["wrap lives on the instance", "wrap mode on the instance", "loop becomes"],
    ]),
    ("R-euler", "rotation channels are euler scalars; no quaternion channel", "15.9", [
        ["euler"],
        ["no quaternion channel", "no `rotation` channel", "defines no rotation channel"],
    ]),
    ("R-adopt", "adoption order: guard first, expansion/evaluation next, production data last", "15.10", [
        ["reservation guard first", "guard first"],
        ["production data last"],
    ]),
]


# ---------------------------------------------------------------- matching

def containment(src_tokens, tgt_tokens):
    if not src_tokens:
        return 1.0
    return len(src_tokens & tgt_tokens) / len(src_tokens)


def check_headings(draft_headings, target_headings):
    by_number = {h["number"]: h for h in target_headings}
    results = []
    for h in draft_headings:
        t = by_number.get(h["number"])
        if t is None:
            results.append(("MISSING", h, None, 0.0))
            continue
        c = containment(h["tokens"], t["tokens"])
        status = "FOUND" if c >= 0.6 else "ALTERED-SUSPECT"
        results.append((status, h, t, c))
    return results


def best_sentence(src, target_sentences):
    best = None
    best_score = -1.0
    for cand in target_sentences:
        score = containment(src["tokens"], cand["tokens"])
        if cand["section"] == src["section"]:
            score += 0.0  # tie-break handled below via adjusted compare
        adj = score + (0.02 if cand["section"] == src["section"] else 0.0)
        if adj > best_score:
            best_score = adj
            best = (cand, score)
    return best  # (sentence, raw containment) or None


def check_sentences(draft_sentences, target_sentences):
    results = []
    seen = set()
    for s in draft_sentences:
        key = " ".join(sorted(s["tokens"])) + "|" + ",".join(sorted(s["classes"]))
        if key in seen or not s["classes"]:
            continue
        seen.add(key)
        match = best_sentence(s, target_sentences)
        if match is None:
            results.append(("MISSING", s, None, 0.0, ""))
            continue
        cand, score = match
        lost = {c for c in (s["classes"] & STRONG) if c not in cand["classes"]}
        floor = 0.50 if len(s["tokens"]) <= 6 else 0.40
        if score >= 0.75 and not lost:
            status = "FOUND"
        elif score >= floor:
            status = "ALTERED-SUSPECT"
        else:
            status = "MISSING"
        note = ""
        if lost and score >= floor:
            note = "strong keyword lost: " + ", ".join(sorted(lost))
        results.append((status, s, cand if status != "MISSING" else None, score, note))
    return results


def check_rules(target_sentences):
    results = []
    for rid, name, section, groups in NAMED_RULES:
        satisfied = 0
        first_line = None
        for group in groups:
            hit_line = None
            for alt in group:
                alt_tokens = content_set(alt)
                if not alt_tokens:
                    continue
                # prefer a same-section sentence; fall back to any
                for cand in target_sentences:
                    if alt_tokens <= cand["tokens"] and cand["section"] == section:
                        hit_line = cand["line"]
                        break
                if hit_line is not None:
                    break
                for cand in target_sentences:
                    if alt_tokens <= cand["tokens"]:
                        hit_line = cand["line"]
                        break
                if hit_line is not None:
                    break
            if hit_line is not None:
                satisfied += 1
                if first_line is None:
                    first_line = hit_line
        if satisfied == len(groups):
            status = "FOUND"
        elif satisfied * 2 >= len(groups):
            status = "ALTERED-SUSPECT"
        else:
            status = "MISSING"
        results.append((status, rid, name, section, satisfied, len(groups), first_line))
    return results


# ---------------------------------------------------------------- main

def fail_input(msg):
    print(f"ERROR: check_foldin.py: {msg}", file=sys.stderr)
    return 2


def main(argv):
    args = [a for a in argv[1:] if not a.startswith("--")]
    opts = {a.split("=")[0]: a.split("=", 1)[1] for a in argv[1:]
            if a.startswith("--") and "=" in a}
    bare_opts = {a for a in argv[1:] if a.startswith("--") and "=" not in a}
    if "-h" in bare_opts or "--help" in bare_opts:
        print(__doc__.split("Usage:")[1].strip().split("\n\n")[0] if "Usage:" in __doc__
              else "usage: check_foldin.py [TARGET] [--draft=DRAFT]")
        print("  exit 0 all FOUND / 1 any ALTERED-SUSPECT or MISSING / 2 input error")
        return 0
    repo = Path(__file__).resolve().parent.parent.parent
    draft_path = Path(opts.get("--draft", repo / "docs/integration/FORMAT15-DRAFT-W31.md"))
    if args:
        target_path = Path(args[0])
    elif "--target" in opts:
        target_path = Path(opts["--target"])
    else:
        target_path = repo / "docs/scene-format.md"
    try:
        draft_lines = draft_path.read_text(encoding="utf-8").splitlines()
    except OSError as e:
        return fail_input(f"cannot read draft {draft_path} ({e.strerror or e})")
    try:
        target_lines = target_path.read_text(encoding="utf-8").splitlines()
    except OSError as e:
        return fail_input(f"cannot read target {target_path} ({e.strerror or e})")

    paste_lines, paste_first = paste_region(draft_lines)
    d_headings = extract_headings(paste_lines, paste_first)
    d_sentences = extract_sentences(paste_lines, paste_first)
    n_normative = len([1 for s in d_sentences if s["classes"]])
    t_headings = extract_headings(target_lines, 1)
    t_sentences = extract_sentences(target_lines, 1)
    if not d_headings:
        return fail_input(f"draft {draft_path} yields no numbered headings — not a draft this tool can census")

    print(f"check_foldin.py — target {target_path}")
    print(f"  draft {draft_path} (paste text lines {paste_first}–{paste_first + len(paste_lines) - 1})")
    rules_on = opts.get("--rules", "on") != "off"
    print(f"  census: {len(d_headings)} headings, {n_normative} normative sentences, "
          f"{len(NAMED_RULES) if rules_on else 0} named rules"
          f"{'' if rules_on else ' (named-rule census off)'}")
    print()

    counts = {"FOUND": 0, "ALTERED-SUSPECT": 0, "MISSING": 0}

    print("[H headings]")
    for status, h, t, c in check_headings(d_headings, t_headings):
        counts[status] += 1
        loc = f"target line {t['line']}" if t else "—"
        print(f"  {status:<15} §{h['number']:<6} {h['title'][:64]}  ({loc})")
    print()

    print("[N normative sentences]")
    sent_results = check_sentences(d_sentences, t_sentences)
    for status, s, cand, score, note in sent_results:
        counts[status] += 1
        loc = f"target line {cand['line']}" if cand else "—"
        kw = "/".join(sorted(s["classes"]))
        txt = s["text"].replace("\n", " ")
        if len(txt) > 96:
            txt = txt[:93] + "..."
        extra = f" [{note}]" if note else ""
        print(f"  {status:<15} §{s['section'] or '?':<6} ({kw}) {txt}  ({loc}, containment {score:.2f}){extra}")
    print()

    if rules_on:
        print("[R named rules]")
        for status, rid, name, section, sat, total, line in check_rules(t_sentences):
            counts[status] += 1
            loc = f"target line {line}" if line else "—"
            print(f"  {status:<15} {rid:<12} §{section:<6} {name[:72]}  ({loc}, {sat}/{total} phrase groups)")
        print()

    total = sum(counts.values())
    verdict = "PASS" if counts["ALTERED-SUSPECT"] == 0 and counts["MISSING"] == 0 else "FAIL"
    print(f"OVERALL: {verdict} ({counts['FOUND']}/{total} FOUND, "
          f"{counts['ALTERED-SUSPECT']} ALTERED-SUSPECT, {counts['MISSING']} MISSING)")
    return 0 if verdict == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
