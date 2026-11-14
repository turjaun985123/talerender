#!/usr/bin/env python3
"""chunk_caps.py — caption chunker helper for a Tall Tale project (v5, spec: findings/p1-D3.md §3).

  scripts/py tools/chunk_caps.py <project> [--line N] [--write] [--force] [--check]

Proposes authored `caps` for lines that lack them, using the 6-step algorithm documented
in library/story.md §1.7 (atoms -> clause boundaries -> 2-3 word packing -> display-time
merge -> special hook/final lines -> digitised emit). The tool PROPOSES; authored caps
stay the default route (STYLE.md §1) and the tool never runs inside capture/voice/mix.

Default mode is read-only: it prints proposals. --write inserts `caps` into lines that
lack one (surgical text edit: no other field touched, key order and formatting kept).
--force also replaces authored caps (one diff line printed per replaced line).
--check verifies existing caps against the line text (the CAPS-MISMATCH equivalence
shared with script_lint.py) and exits 0/1. Input errors exit 2.
Python 3.10+, standard library only.
"""
import argparse, json, re, sys
from pathlib import Path

ARTICLES = {'a', 'an', 'the', 'this', 'that', 'his', 'her', 'my', 'their', 'its'}
PREPS = {'in', 'on', 'at', 'to', 'of', 'for', 'with', 'from', 'by', 'into', 'onto',
         'over', 'under', 'after', 'before'}
CONJ_CLASS = {'and', 'but', 'so', 'then'}
RELATIVE = {'who', 'which', 'that'}
NEGATIONS = {"didn't", "couldn't", "wasn't", "won't", 'never'}
UNITS = ['zero', 'one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight', 'nine',
         'ten', 'eleven', 'twelve', 'thirteen', 'fourteen', 'fifteen', 'sixteen',
         'seventeen', 'eighteen', 'nineteen']
TENS = ['twenty', 'thirty', 'forty', 'fifty', 'sixty', 'seventy', 'eighty', 'ninety']
NUMBER_WORDS = set(UNITS) | set(TENS) | {'hundred', 'thousand', 'million', 'billion',
                                         'twice', 'double'}
ABSOLUTES = {'never', 'always', 'every', 'nobody', 'nothing', 'none', 'impossible',
             'unstoppable', 'unbreakable', 'still', 'only'}
# Common irregular past-tense forms + function words that block a verb-split signal.
IRREGULAR_VERBS = {'went', 'stood', 'came', 'gave', 'found', 'said', 'left', 'took',
                   'ran', 'sat', 'flew', 'dropped', 'kept', 'fought', 'thought', 'saw',
                   'got', 'made', 'put', 'shook', 'walked', 'lived', 'died', 'fell',
                   'broke', 'caught', 'brought', 'began', 'grew', 'knew', 'spoke',
                   'wore', 'wrote', 'rode', 'sang', 'swam', 'held', 'sent', 'paid',
                   'lost', 'won', 'met', 'led', 'fed', 'shot', 'hit', 'set', 'let'}
NO_SPLIT_AFTER = ARTICLES | PREPS | CONJ_CLASS | NEGATIONS | \
    {'nothing', 'something', 'anything', 'everything', 'very', 'just', 'still'}
# Tuning vocabularies for chunk_line's phrase signals — generic English closed
# classes, calibrated against the delivered episodes' human caps (T-AGREE).
AUXILIARIES = {'had', 'has', 'have', 'was', 'were', 'is', 'are', 'am', 'would',
               'could', 'will', 'shall', 'should', 'may', 'might', 'must', 'did',
               'do', 'does', 'been', 'being'}
SUBJECT_PRONOUNS = {'he', 'she', 'it', 'they', 'we', 'i', 'you'}
DETERMINER_STARTS = {'every', 'each', 'another', 'all', 'both'}
PARTICLES = {'in', 'out', 'off', 'over', 'down'}
UNIT_NOUNS = {'miles', 'mile', 'pounds', 'pound', 'cents', 'cent', 'dollars',
              'dollar', 'years', 'year', 'months', 'month', 'weeks', 'week',
              'days', 'day', 'hours', 'hour', 'minutes', 'minute', 'seconds',
              'second', 'feet', 'foot', 'inches', 'inch', 'times', 'people',
              'men', 'women', 'children'}

def norm(w):
    """captions.ts normalisation: lowercase, keep only [a-z0-9']."""
    return re.sub(r"[^a-z0-9']", '', str(w).lower())

def is_number_token(raw):
    t = raw.lower().strip('.,!?;:"()[]')
    if any(c.isdigit() for c in t):
        return True
    parts = t.replace('-', ' ').split()
    return bool(parts) and all(p in NUMBER_WORDS for p in parts)

def number_to_words(n):
    """Same routine as script_lint.py (shared equivalence rules, standalone tools)."""
    if '.' in n:
        ip, fp = n.split('.', 1)
        return number_to_words(ip) + ['point'] + [UNITS[int(d)] for d in fp if d.isdigit()]
    v = int(n)
    if v < 20:
        return [UNITS[v]]
    if v < 100:
        t, u = divmod(v, 10)
        return [TENS[t - 2]] + ([UNITS[u]] if u else [])
    if v < 1000:
        h, r = divmod(v, 100)
        return [UNITS[h], 'hundred'] + (number_to_words(str(r)) if r else [])
    if 1100 <= v <= 1999 or 2000 <= v <= 2099:
        hi, lo = divmod(v, 100)
        out = number_to_words(str(hi))
        if lo:
            out += number_to_words(str(lo))
        elif v < 2000:
            out += ['hundred']
        return out
    if v < 10000:
        th, r = divmod(v, 1000)
        return number_to_words(str(th)) + ['thousand'] + (number_to_words(str(r)) if r else [])
    return [str(v)]

def words_to_number(t):
    """Inverse of number_to_words for a single spoken token sequence (digitising)."""
    parts = t.replace('-', ' ').split()
    if not parts or not all(p in NUMBER_WORDS for p in parts):
        return None
    total, cur = 0, 0
    for p in parts:
        if p in UNITS:
            cur += UNITS.index(p)
        elif p in TENS:
            cur += (TENS.index(p) + 2) * 10
        elif p == 'hundred':
            cur = max(cur, 1) * 100
        elif p == 'thousand':
            total += max(cur, 1) * 1000
            cur = 0
        elif p == 'million':
            total += max(cur, 1) * 1000000
            cur = 0
        else:
            return None  # twice/double/billion: leave spelled
    return str(total + cur)

def spoken_stream(text):
    """Same equivalence stream as script_lint.py's CAPS-MISMATCH check."""
    out = []
    for tok in text.split():
        t = tok.lower().strip('.,!?;:"()[]…')
        if not t:
            continue
        if re.fullmatch(r'\d+(\.\d+)?', t):
            out += number_to_words(t)
        elif '-' in t:
            parts = t.split('-')
            if all(p in NUMBER_WORDS or p.isdigit() for p in parts):
                for p in parts:
                    out += number_to_words(p) if p.isdigit() else [p]
            else:
                out.append(re.sub(r"[^a-z0-9']", '', t))
        else:
            out.append(re.sub(r"[^a-z0-9']", '', t))
    return [w for w in out if w]

def chunk_key(chunk):
    """Equivalence key for agreement/--check: spoken stream of a chunk string."""
    return tuple(spoken_stream(chunk))

# ---------------------------------------------------------------- atoms (step 2)
def _bare(tok):
    return norm(tok)

def _is_cap(tok, idx, toks):
    """Capitalised evidence: first letter uppercase, and not merely sentence-initial."""
    s = tok.lstrip('"\'(')
    if not s or not s[0].isupper():
        return False
    if idx == 0:
        return False
    prev = toks[idx - 1]
    if prev.endswith(('.', '!', '?')):
        return False
    return True

def build_atoms(toks):
    """Merge tokens into indivisible atoms. Returns a list of token-index lists."""
    atoms = []
    i, n = 0, len(toks)
    while i < n:
        tok, b = toks[i], _bare(toks[i])
        # number atom: number token (+ 'and a half/quarter') (+ unit noun if <=3 words)
        if is_number_token(tok):
            atom = [i]
            j = i + 1
            if j + 2 <= n - 1 and \
                    [_bare(toks[j]), _bare(toks[j + 1]), _bare(toks[j + 2])] in \
                    (['and', 'a', 'half'], ['and', 'a', 'quarter']):
                atom += [j, j + 1, j + 2]
                j += 3
            if j < n and len(atom) + 1 <= 3 and \
                    _bare(toks[j]).strip('.,!?;:') in UNIT_NOUNS:
                atom.append(j)
                j += 1
            atoms.append(atom)
            i = j
            continue
        # article/determiner atom (+ possessive extension: "her grandma's house.").
        # A possessive determiner right after a conjunction does not merge forward:
        # it pops with the conjunction instead ("And his | home town, ...").
        if b in ARTICLES and i + 1 < n:
            if b in {'his', 'her', 'my', 'their', 'its'} and i > 0 and \
                    _bare(toks[i - 1]) in CONJ_CLASS:
                atoms.append([i])
                i += 1
                continue
            atom = [i, i + 1]
            if _bare(toks[i + 1]).endswith("'s") and i + 2 < n and len(atom) + 1 <= 3:
                atom.append(i + 2)
            atoms.append(atom)
            i += len(atom)
            continue
        # negation + verb
        if b in NEGATIONS and i + 1 < n:
            atoms.append([i, i + 1])
            i += 2
            continue
        # proper-name run (>=2 consecutive capitalised tokens; roman numerals ride
        # along). The run stops before a possessive token — "Lloyd | Olsen's wife"
        # is chunked by the possessive rule below, as the delivered caps do.
        if _is_cap(tok, i, toks) and i + 1 < n and \
                (_is_cap(toks[i + 1], i + 1, toks) or re.fullmatch(r'[IVX]+\.?', toks[i + 1])):
            atom = [i]
            j = i + 1
            while j < n and not _bare(toks[j]).endswith("'s") and \
                    (_is_cap(toks[j], j, toks) or
                     re.fullmatch(r'[IVX]+\.?', toks[j]) or toks[j][0].isdigit()):
                atom.append(j)
                j += 1
            atoms.append(atom)
            i = j
            continue
        # capitalised possessive + noun ("Olsen's wife")
        if b.endswith("'s") and tok.lstrip('"\'(')[:1].isupper() and i + 1 < n:
            atoms.append([i, i + 1])
            i += 2
            continue
        atoms.append([i])
        i += 1
    # preposition pass: a lone preposition merges with the following atom if <=3 words
    # (never across punctuation: a sentence-final "over." does not absorb the next
    # sentence; and a particle preposition after a verb — "tipped in" — stays lone
    # when its object is not determiner-headed, so packing can glue it to the verb)
    out = []
    k = 0
    while k < len(atoms):
        a = atoms[k]
        particle_ctx = len(a) == 1 and _bare(toks[a[0]]) in PARTICLES and out and \
            _verb_form(toks[out[-1][-1]]) and k + 1 < len(atoms) and \
            _bare(toks[atoms[k + 1][0]]) not in ARTICLES
        if len(a) == 1 and _bare(toks[a[0]]) in PREPS and not particle_ctx and \
                not toks[a[0]].endswith(('.', '!', '?', ',', ';', ':')) and \
                k + 1 < len(atoms) and len(a) + len(atoms[k + 1]) <= 3:
            out.append(a + atoms[k + 1])
            k += 2
        else:
            out.append(a)
            k += 1
    return out

# ---------------------------------------------------------------- packing (steps 3-5)
def _verb_form(tok):
    b = _bare(tok)
    return b in IRREGULAR_VERBS or (len(b) > 3 and (b.endswith('ed') or b.endswith('ing')))

def chunk_line(text, key_word=None, hook_style=False, is_final=False):
    """Chunk one line's text. Returns (caps_string, digitised [(old, new), ...]).
    hook_style (line 1): one word per chunk — the delivered episodes' hook style
    (both delivered episodes chunk line 1 word-by-word; captions.ts' own header
    calls the format "ONE word at a time"). The shipped template's line 1 predates
    the convention, like several other template traits STYLE.md flags."""
    toks = text.split()
    if not toks:
        return '', []
    if hook_style:
        chunks = [[[i]] for i in range(len(toks))]
        return _emit(chunks, toks)
    atoms = build_atoms(toks)
    atom_words = [len(a) for a in atoms]
    total_words = len(toks)
    # clause boundaries (step 3). Two faithful extensions, both visible in every
    # delivered episode's human caps: a sentence-final atom (. ! ?) always closes
    # its chunk, and "or" behaves like the listed clause conjunctions.
    before, after, sent_after = set(), set(), set()
    for k, a in enumerate(atoms):
        last = toks[a[-1]]
        if last.endswith(','):
            after.add(k)
        if last.endswith(('.', '!', '?')) and a[-1] != len(toks) - 1:
            after.add(k)
            sent_after.add(k)
        if k > 0:
            first_bare = _bare(toks[a[0]])
            remaining = total_words - a[0]
            if first_bare in CONJ_CLASS or first_bare == 'or':
                if remaining >= 5:
                    before.add(k)
            elif first_bare in RELATIVE:
                before.add(k)
            elif first_bare == 'to' and _verb_form(toks[atoms[k - 1][-1]]):
                before.add(k)
    # key word (step 5): a standalone atom equal to key_word may pop alone
    key_atoms = set()
    if key_word:
        kw = norm(key_word)
        for k, a in enumerate(atoms):
            if len(a) == 1 and norm(toks[a[0]]) == kw:
                key_atoms.add(k)

    chunks, cur, cur_w = [], [], 0
    def close():
        nonlocal cur, cur_w
        if cur:
            chunks.append(cur)
            cur, cur_w = [], 0
    for k, a in enumerate(atoms):
        aw = atom_words[k]
        if cur and cur_w + aw > 3:
            close()
        if cur and cur_w >= 2:
            first_bare = _bare(toks[a[0]])
            last_cur_bare = _bare(toks[cur[-1][-1]])
            particle = len(a) == 1 and first_bare in PARTICLES and \
                _verb_form(toks[cur[-1][-1]])
            if not particle:
                split = k in before or k in key_atoms
                if first_bare in PREPS or first_bare in ARTICLES or \
                        first_bare in DETERMINER_STARTS or first_bare in NEGATIONS or \
                        is_number_token(toks[a[0]]):
                    split = True
                if any(is_number_token(toks[ti]) for at in cur for ti in at):
                    split = True  # a completed number phrase closes the chunk
                if _is_cap(toks[a[0]], a[0], toks) and \
                        not toks[cur[-1][-1]].endswith(',') and \
                        not (any(_verb_form(toks[at[-1]]) for at in cur) and
                             last_cur_bare.isalpha() and
                             last_cur_bare not in (ARTICLES | PREPS | CONJ_CLASS)):
                    split = True  # name starts a new phrase (not after a comma,
                    # not as a verb's object: "picked young Mike")
                if last_cur_bare not in NO_SPLIT_AFTER and \
                        last_cur_bare not in AUXILIARIES:
                    if first_bare in AUXILIARIES:
                        split = True  # the verb phrase starts at the auxiliary
                    elif _verb_form(toks[a[0]]) and \
                            not toks[a[-1]].endswith(('.', '!', '?')) and \
                            (k + 1 >= len(atoms) or
                             _bare(toks[atoms[k + 1][0]]) not in PREPS):
                        split = True  # main verb starts a new chunk, unless it
                        # ends the sentence or runs into a preposition phrase
                if is_final and k == len(atoms) - 1 and len(a) == 1:
                    split = True  # the punchline's last word pops alone
                if split:
                    close()
        cur.append(a)
        cur_w += aw
        if k in sent_after:
            close()
        elif k in after and cur_w >= 2 and \
                not (k + 1 < len(atoms) and _is_cap(toks[atoms[k + 1][0]], atoms[k + 1][0], toks)):
            close()  # comma closes — unless a capitalised appositive follows
                    # ("home town, Fruita,") and joins this chunk instead
        elif len(cur) == 1 and len(a) >= 2 and _bare(toks[a[0]]) in ARTICLES and \
                '-' in toks[a[-1]]:
            close()  # a hyphenated modifier phrase stands alone ("a five-month-old")
        elif cur_w == 2 and len(cur) == 2 and len(cur[0]) == 1 and len(cur[1]) == 1 and \
                _bare(toks[cur[0][0]]) in CONJ_CLASS and \
                _bare(toks[cur[1][0]]) in {'his', 'her', 'my', 'their', 'its'}:
            close()  # conjunction + possessive pops together ("And his")
        elif cur_w >= 3:
            close()
        elif k in key_atoms:
            close()
    close()
    # post-pass: a chunk never ENDS on a lone conjunction if the next chunk can
    # take it ("examined him and | shrugged:" -> "examined him | and shrugged:")
    for _ in range(10):
        moved = False
        for ci in range(len(chunks) - 1):
            left, right = chunks[ci], chunks[ci + 1]
            if len(left) > 1 and len(left[-1]) == 1 and \
                    _bare(toks[left[-1][0]]) in {'and', 'but', 'or', 'so', 'if', 'then'} and \
                    sum(len(x) for x in right) + 1 <= 3:
                right.insert(0, left.pop())
                moved = True
        if not moved:
            break
    chunks = [c for c in chunks if c]
    return _emit(chunks, toks)

def _emit(chunks, toks):
    """Step 6: words exactly as in text; number words digitised where clean
    (values >= 10 only — the template precedent: 73, 48, 53, 50; "one" stays "one")."""
    digitised = []
    out_chunks = []
    for ch in chunks:
        words = []
        for ai in ch:
            for ti in ai:
                tok = toks[ti]
                m = re.match(r'^(.*?)([.,!?;:]*)$', tok)
                core, punct = m.group(1), m.group(2)
                low = core.lower()
                if low in NUMBER_WORDS or ('-' in low and
                        all(p in NUMBER_WORDS for p in low.replace('-', ' ').split())):
                    num = words_to_number(low)
                    if num is not None and int(num) >= 10:
                        digitised.append((core, num))
                        words.append(num + punct)
                        continue
                words.append(tok)
        out_chunks.append(' '.join(words))
    return ' | '.join(out_chunks), digitised

def key_word_for(script, idx):
    """Step 5's key word: line 1 -> hookWord or lint's last marker; final line ->
    a content word shared with / seeded by line 1 (the loop word)."""
    lines = script['lines']
    toks = lines[idx]['text'].split()
    if idx == 0:
        if lines[0].get('hookWord'):
            return lines[0]['hookWord']
        last = None
        for j, raw in enumerate(toks):
            if is_number_token(raw) or norm(raw) in ABSOLUTES or \
                    (norm(raw) == 'no' and j + 1 < len(toks) and norm(toks[j + 1]) == 'one'):
                last = raw
        return last
    if idx == len(lines) - 1:
        stop = {'the', 'a', 'an', 'and', 'or', 'but', 'so', 'of', 'to', 'in', 'on',
                'at', 'was', 'were', 'is', 'it', 'he', 'she', 'they', 'his', 'her',
                'their', 'had', 'has', 'with', 'for', 'from', 'that', 'this'}
        first = {norm(w) for w in lines[0]['text'].split()}
        counts, seeded = {}, set()
        for li, ln in enumerate(lines):
            for w in ln['text'].split():
                counts[norm(w)] = counts.get(norm(w), 0) + 1
                if 1 <= li <= 4:
                    seeded.add(norm(w))
        for raw in toks:
            nw = norm(raw)
            if nw and nw not in stop and (nw in first or counts.get(nw, 0) >= 3 or
                                          (counts.get(nw, 0) >= 2 and nw in seeded)):
                return raw
    return None

# ---------------------------------------------------------------- display merge (step 4)
def apply_display_merge(script, proposals, words, lines_meta):
    """Merge non-final chunks displaying < 0.45 s with the next chunk when the merge
    stays <= 3 words; otherwise keep and warn. Returns (proposals, notes)."""
    notes = []
    if not words:
        return proposals, notes
    by_line = {}
    for w in words:
        by_line.setdefault(w.get('line'), []).append(w)
    line_end = {}
    if lines_meta:
        for l in lines_meta.get('lines', []):
            line_end[l.get('i')] = float(l.get('end', 0))
    for idx, caps in list(proposals.items()):
        if idx == 0:
            continue  # the hook line is exempt: its one-word pops are the point
                      # (both delivered episodes keep sub-0.45 s hook chunks)
        ws = by_line.get(idx)
        if not ws or len(ws) != len(script['lines'][idx]['text'].split()):
            continue
        chunks = [c.strip() for c in caps.split('|') if c.strip()]
        starts, pos = [], 0
        ok = True
        for c in chunks:
            cw = len(c.split())
            if pos >= len(ws):
                ok = False
                break
            starts.append(float(ws[pos]['s']))
            pos += cw
        if not ok or pos != len(ws):
            continue
        end_last = line_end.get(idx, float(ws[-1]['e']) + 0.25)
        changed = True
        while changed:
            changed = False
            bounds = starts + [end_last]
            for k in range(len(chunks) - 1):
                disp = bounds[k + 1] - bounds[k]
                if disp < 0.45:
                    if idx == len(script['lines']) - 1 and k + 1 == len(chunks) - 1 \
                            and len(chunks[k + 1].split()) == 1:
                        continue  # the punchline's final-word pop is deliberate
                    if len(chunks[k].split()) + len(chunks[k + 1].split()) <= 3:
                        chunks[k] = chunks[k] + ' ' + chunks[k + 1]
                        del chunks[k + 1]
                        del starts[k + 1]
                        changed = True
                        break
                    notes.append(f'line {idx + 1}: ⚠ fast chunk "{chunks[k]}" '
                                 f'({disp:.2f} s) — left unmerged (would exceed 3 words)')
        proposals[idx] = ' | '.join(chunks)
    return proposals, notes

def propose(script, words=None, lines_meta=None):
    """Returns ({line_idx: caps}, notes, digitised_notes)."""
    proposals, notes, digi = {}, [], []
    for i, ln in enumerate(script['lines']):
        if isinstance(ln.get('caps'), str) and ln['caps'].strip():
            continue
        caps, d = chunk_line(ln['text'], key_word_for(script, i),
                             hook_style=(i == 0),
                             is_final=(i == len(script['lines']) - 1))
        proposals[i] = caps
        for old, new in d:
            digi.append(f'line {i + 1}: digitised "{old}" -> "{new}"')
    if words:
        proposals, mnotes = apply_display_merge(script, proposals, words, lines_meta)
        notes += mnotes
    else:
        notes.append('timings: estimated display merge skipped (no data/words.json)')
    return proposals, notes, digi

# ---------------------------------------------------------------- surgical --write
def _line_object_spans(raw):
    """Spans (start, end) of each object in the top-level "lines" array, or None."""
    m = re.search(r'"lines"\s*:\s*\[', raw)
    if not m:
        return None
    i = m.end()
    spans, depth, in_str, esc = [], 0, False, False
    start = None
    while i < len(raw):
        c = raw[i]
        if in_str:
            if esc:
                esc = False
            elif c == '\\':
                esc = True
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == '{':
            if depth == 0:
                start = i
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0 and start is not None:
                spans.append((start, i + 1))
                start = None
        elif c == ']' and depth == 0:
            break
        i += 1
    return spans

def write_caps(path, script, new_caps):
    """Insert/replace caps for {line_idx: caps} via surgical text edits.
    Returns a list of human-readable change lines. Raises ValueError on failure."""
    raw = path.read_text()
    spans = _line_object_spans(raw)
    if spans is None or len(spans) != len(script['lines']):
        raise ValueError('could not locate the "lines" objects for a surgical edit — '
                         'file left untouched')
    edits, changes = [], []
    for idx, caps in sorted(new_caps.items()):
        s, e = spans[idx]
        obj = raw[s:e]
        encoded = json.dumps(caps, ensure_ascii=False)
        m = re.search(r'"caps"\s*:\s*"(?:\\.|[^"\\])*"', obj)
        if m:
            old = json.loads(m.group(0).split(':', 1)[1].strip())
            if old != caps:
                edits.append((s + m.start(), s + m.end(), f'"caps": {encoded}'))
                changes.append(f'line {idx + 1}: caps replaced: "{old}" -> "{caps}"')
        else:
            edits.append((e - 1, e - 1, f', "caps": {encoded}'))
            changes.append(f'line {idx + 1}: caps added: "{caps}"')
    for s, e, rep in sorted(edits, reverse=True):
        raw = raw[:s] + rep + raw[e:]
    json.loads(raw)  # never write a file we cannot re-parse
    path.write_text(raw)
    return changes

# ---------------------------------------------------------------- main
def main(argv=None):
    ap = argparse.ArgumentParser(description='Caption chunker helper (proposes authored caps).')
    ap.add_argument('project')
    ap.add_argument('--line', type=int, default=None, help='only this line (1-based)')
    ap.add_argument('--write', action='store_true')
    ap.add_argument('--force', action='store_true')
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args(argv)
    P = Path(a.project)
    sp = P / 'script.json'
    if not sp.exists():
        print(f'ERROR: {sp} not found', file=sys.stderr)
        return 2
    try:
        S = json.loads(sp.read_text())
    except Exception as e:
        print(f'ERROR: script.json unparseable: {e}', file=sys.stderr)
        return 2
    if not isinstance(S, dict) or not S.get('lines'):
        print('ERROR: script.json has zero lines', file=sys.stderr)
        return 2
    lines = S['lines']

    if a.check:
        bad = 0
        for i, ln in enumerate(lines):
            if not (isinstance(ln.get('caps'), str) and ln['caps'].strip()):
                print(f'line {i + 1}: no caps (skipped)')
                continue
            caps_stream = spoken_stream(' '.join(
                c.strip() for c in ln['caps'].split('|') if c.strip()))
            text_stream = spoken_stream(ln['text'])
            if caps_stream == text_stream:
                print(f'line {i + 1}: caps OK')
            else:
                bad += 1
                k = next((x for x in range(min(len(caps_stream), len(text_stream)))
                          if caps_stream[x] != text_stream[x]),
                         min(len(caps_stream), len(text_stream)))
                x = caps_stream[k] if k < len(caps_stream) else '<end>'
                y = text_stream[k] if k < len(text_stream) else '<end>'
                print(f'line {i + 1}: CAPS-MISMATCH at word {k + 1} (caps "{x}" vs text "{y}")')
        print(f'CAPS CHECK: {"FAIL" if bad else "PASS"} ({bad} mismatched line(s))')
        return 1 if bad else 0

    words, lines_meta = None, None
    wp, lp = P / 'data' / 'words.json', P / 'data' / 'lines.json'
    if wp.exists():
        try:
            words = json.loads(wp.read_text())
            if lp.exists():
                lines_meta = json.loads(lp.read_text())
        except Exception:
            print('timings: words.json present but unusable — display merge skipped')
            words = None
    proposals, notes, digi = propose(S, words, lines_meta)
    if a.force:
        for i, ln in enumerate(lines):
            if isinstance(ln.get('caps'), str) and ln['caps'].strip():
                caps, _ = chunk_line(ln['text'], key_word_for(S, i),
                                     hook_style=(i == 0),
                                     is_final=(i == len(lines) - 1))
                proposals[i] = caps
    todo = {i: c for i, c in proposals.items()
            if a.line is None or i == a.line - 1}
    if a.write:
        try:
            changes = write_caps(sp, S, todo)
        except ValueError as e:
            print(f'ERROR: {e}', file=sys.stderr)
            return 2
        for ch in changes:
            print(ch)
        if not changes:
            print('nothing to write (all lines already have caps)')
    else:
        for i, ln in enumerate(lines):
            if a.line is not None and i != a.line - 1:
                continue
            authored = isinstance(ln.get('caps'), str) and ln['caps'].strip()
            if authored and not a.force:
                print(f'line {i + 1}: (authored, skipped)')
            elif i in proposals:
                print(f'line {i + 1}: "{proposals[i]}"')
    for nt in notes:
        print(nt)
    for d in digi:
        print(d)
    return 0

if __name__ == '__main__':
    sys.exit(main())
