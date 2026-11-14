#!/usr/bin/env python3
"""script_lint.py — story-layer lint for a Tall Tale project (v5, spec: findings/p1-D3.md §2).

  scripts/py tools/script_lint.py <project> [--target SECONDS] [--lenient-caps]
      [--warn-only] [--json] [--codes] [--verbose]

Reads <project>/script.json (required) and, when BOTH exist, <project>/data/words.json +
data/lines.json (full mode, exact timings). Otherwise script mode: timings are estimated
internally with estimate.py's letter-weight model at the per-voice rate of the shared
VOICE_WPS table (identical numbers to scripts/estimate.py §4.2 — cross-referenced copy,
no shared module, so both tools stay standalone-copyable). Timing-dependent results are
suffixed (EST) in script mode. The tool only reads; it never writes to the project.

Exit codes: 0 = no FAILs (WARNs allowed) · 1 = at least one FAIL · 2 = input error.
Python 3.10+, standard library only.
"""
import argparse, json, re, sys
from pathlib import Path

# --- shared per-voice rate table: words/sec AT SPEED 1.0 (D3 §4.2; identical numbers
# --- must live in scripts/estimate.py — keep the two copies in sync by hand).
VOICE_WPS = {
    'am_michael': 2.67, 'am_puck': 2.75, 'am_fenrir': 2.60, 'am_onyx': 2.55,
    'af_heart': 2.60, 'af_bella': 2.70, 'af_nicole': 2.55,
    'bm_george': 2.30, 'bm_fable': 2.35, 'bf_emma': 2.40,
}
VOICE_WPS_FAMILY = {'am_': 2.67, 'af_': 2.60, 'bm_': 2.32, 'bf_': 2.40}
# Reference word weight for the estimator below — must match scripts/estimate.py's
# REF_W (v5 calibration, Phase 3 seat 3B: (letters + 2.5) of an average word over the
# six delivered scripts, pooled mean 7.05). Speech seconds = Σ(len + 2.5) / (wps ×
# speed × REF_W), with NO per-punctuation pause charges (the measured rate already
# contains them; charging again was the old model's systematic over-read).
REF_W = 7.05

def wps_for(voice):
    if voice in VOICE_WPS:
        return VOICE_WPS[voice]
    return VOICE_WPS_FAMILY.get((voice or '')[:3], 2.60)

# --- vocabularies (D3 §2.3 / §2.5, concrete) ---
UNITS = ['zero', 'one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight', 'nine',
         'ten', 'eleven', 'twelve', 'thirteen', 'fourteen', 'fifteen', 'sixteen',
         'seventeen', 'eighteen', 'nineteen']
TENS = ['twenty', 'thirty', 'forty', 'fifty', 'sixty', 'seventy', 'eighty', 'ninety']
NUMBER_WORDS = set(UNITS) | set(TENS) | {'hundred', 'thousand', 'million', 'billion',
                                         'twice', 'double'}
ABSOLUTES = {'never', 'always', 'every', 'nobody', 'nothing', 'none', 'impossible',
             'unstoppable', 'unbreakable', 'still', 'only'}
STAKE_WORDS = {'died', 'dead', 'danger', 'risk', 'trapped', 'alone', 'banned', 'illegal',
               'cost', 'paid', 'won', 'lost', 'broke', 'fell', 'struck', 'survived'}
EXPLAIN_PHRASES = ['because', 'which is why', "so that's why", 'turned out',
                   'in the end', 'thanks to']
TRAIL_OPEN = {'but', 'until', 'when', 'before', 'because', 'so', 'and', 'then', 'if', 'as'}
TEASE_RE = re.compile(r'what happened next|nobody knew|no one knew|little did|until one day|about to')
STOPWORDS = {'the', 'a', 'an', 'and', 'or', 'but', 'so', 'of', 'to', 'in', 'on', 'at',
             'was', 'were', 'is', 'it', 'he', 'she', 'they', 'his', 'her', 'their',
             'had', 'has', 'with', 'for', 'from', 'that', 'this'}
TONE_MUSIC_OK = {'funny': {'bounce'}, 'amazing': {'tension', 'bounce'},
                 'serious': {'bed', 'tension'}}
TONE_VOICE_OK = {'funny': {'am_michael', 'am_puck', 'af_heart'},
                 'amazing': {'am_michael', 'af_heart', 'bm_fable'},
                 'serious': {'bm_george', 'bm_fable', 'bf_emma'}}

CODES = [
    ('HOOK-WORDS',   'FAIL', 'line 1 is at most 11 words — the claim completes inside line 1'),
    ('HOOK-FRAGMENT', 'FAIL', 'line 1 must not end open (comma/colon/trailing conjunction)'),
    ('HOOK-MARKER',  'FAIL', 'line 1 needs a payoff marker (number / absolute / Imagine), first one within 6 words'),
    ('HOOK-PAYOFF',  'FAIL/WARN', 'payoff word should end by 3.5 s (WARN to 4.5 s; +0.3 s slack in script mode)'),
    ('HOOK-EXPLAIN', 'FAIL', 'line 1 must not explain itself (because / turned out / in the end / ...)'),
    ('HOOK-SECOND',  'WARN', 'lines 2-3 should not both be background (rubric H3 advisory)'),
    ('CLOSE-PAIR',   'FAIL', 'two consecutive closing lines (the line before the punchline stays open)'),
    ('REHOOK-GAP',   'FAIL/WARN', 'an open (re-hook) line at least every 15 s (WARN at 13-15 s)'),
    ('LOOP-WORDS',   'FAIL/WARN', 'final line loops: 2-3 shared content words with line 1, or a seeded callback word'),
    ('WORDS-TARGET', 'FAIL', 'estimated duration within target ± max(10%, 3 s)'),
    ('DUR-TARGET',   'FAIL', 'actual (full-mode) duration within the same band'),
    ('WORDS-BAND',   'WARN', 'no target declared and estimate outside the 40-60 s band'),
    ('LINE-LONG',    'WARN', 'a line over 5.5 s, or median line over 3.5 s (cadence risk)'),
    ('SRC-MISSING',  'FAIL', 'script.json needs a non-empty "sources" list (hard law 4)'),
    ('SRC-THIN',     'WARN', 'source entry too thin — expected: Publisher, \'Title\' (facts it supports)'),
    ('CAPS-MISSING', 'FAIL', 'every line needs authored "caps" (un-authored lines auto-chunk at ~1 word)'),
    ('CAPS-TYPE',    'FAIL', '"caps" must be a pipe-joined string, never an array/number'),
    ('CAPS-CHUNK',   'FAIL/WARN', 'no chunk over 3 words (4-word proper-name chunk = WARN)'),
    ('CAPS-MISMATCH', 'FAIL', 'caps words must equal the line text words (digit <-> number-word allowed)'),
    ('CAPS-POP',     'FAIL/WARN', 'one-word chunks outside line 1 / final line: FAIL over 25%, WARN over 15%'),
    ('TONE-MUSIC',   'FAIL', 'music must match the declared tone (the silent wrong-tone kill-switch)'),
    ('TONE-VOICE',   'WARN', 'voice outside the tone table row for the declared/inferred tone'),
    ('TONE-MISSING', 'WARN', 'no "tone" field — one is inferred from voice/music and used for tone checks'),
    ('MICRO-BEAT',   'WARN', 'funny/amazing: a line of <=5 words after the midpoint (deadpan micro-beat)'),
]

# ---------------------------------------------------------------- text helpers
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
    """Spoken-word form of a numeric string, for the digit<->word equivalence."""
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

def spoken_stream(text):
    """Word stream of a text/caps string with digits expanded to spoken words and
    hyphenated number compounds split — the CAPS-MISMATCH equivalence (D3 §2.3).
    chunk_caps.py carries the same routine (shared rules, standalone tools)."""
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

def strip_trailing(text):
    s = text.strip()
    while s and s[-1] in '"\')]}”’':
        s = s[:-1].rstrip()
    return s

# ---------------------------------------------------------------- beat classes (D3 §2.5)
def classify(ln):
    beat = ln.get('beat')
    if beat == 'punch':
        return 'PUNCH'
    if beat == 'open' or ln.get('rehook'):
        return 'OPEN'
    if beat == 'close':
        return 'CLOSE'
    text = ln.get('text', '')
    s = strip_trailing(text)
    toks = text.split()
    if s.endswith((',', ';', ':', '…', '...', '—')) or \
            (toks and norm(toks[-1]) in TRAIL_OPEN) or '?' in text or TEASE_RE.search(text.lower()):
        return 'OPEN'
    if (s.endswith('.') or s.endswith('!')) and len(toks) >= 6:
        return 'CLOSE'
    return 'NEUTRAL'

def sentence_closing(ln, is_last):
    return bool(re.search(r"[.?!][\"')]*$", ln.get('text', '').strip())) or \
        bool(ln.get('break')) or is_last

# ---------------------------------------------------------------- timing model
def estimate_timeline(S):
    """estimate.py's calibrated letter-weight model (v5, Phase 3 seat 3B): a line's
    speech seconds = Σ(len + 2.5) / (wps × speed × REF_W), word durations pro-rata
    by weight, NO pause charges, per-sentence gap rule. Kept in lock-step with
    scripts/estimate.py — same formula, same constants."""
    speed = float(S.get('speed', 1.0))
    wps = wps_for(S.get('voice'))
    t = float(S.get('lead', 0.4))
    lines, words = [], []
    n = len(S['lines'])
    for i, ln in enumerate(S['lines']):
        toks = ln['text'].split()
        letters = [len(re.sub(r'[^A-Za-z0-9]', '', w)) or 1 for w in toks]
        base = sum(l + 2.5 for l in letters) / (wps * speed * REF_W)
        unit = base / sum(l + 2.5 for l in letters) if toks else 0.0
        start = t
        for j, (w, l) in enumerate(zip(toks, letters)):
            d = (l + 2.5) * unit
            words.append({'w': w, 'line': i, 's': t, 'e': t + d * 0.92})
            t += d
            # NO per-punctuation pause charges — see REF_W note above.
        lines.append({'i': i, 'text': ln['text'], 'start': start, 'end': t})
        if sentence_closing(ln, i == n - 1):
            t += float(ln.get('gap', S.get('gap', 0.30)))
    return lines, words

def expected_seconds(S):
    weight = sum(sum((len(re.sub(r'[^A-Za-z0-9]', '', w)) or 1) + 2.5
                     for w in ln['text'].split())
                 for ln in S['lines'])
    eff = wps_for(S.get('voice')) * float(S.get('speed', 1.0)) * REF_W
    gaps = sum(float(ln.get('gap', S.get('gap', 0.30)))
               for i, ln in enumerate(S['lines']) if sentence_closing(ln, i == len(S['lines']) - 1))
    return weight / eff + gaps + float(S.get('lead', 0.4)) + float(S.get('outro', 0.9))

# ---------------------------------------------------------------- main lint
def run_lint(S, mode, tl_lines, tl_words, actual_dur, target_override, lenient_caps):
    R = []  # result dicts: code, level, line (1-based|None), msg
    def add(code, level, line, msg):
        R.append({'code': code, 'level': level, 'line': line, 'msg': msg})
    est = ' (EST)' if mode == 'script' else ''
    lines = S['lines']
    n = len(lines)
    texts = [ln.get('text', '') for ln in lines]
    toks = [t.split() for t in texts]
    norms = [[norm(w) for w in t] for t in toks]

    # ---- tone resolution (declared wins; else inference per D3 §1.5/§2.3) ----
    declared_tone = S.get('tone') if S.get('tone') in TONE_MUSIC_OK else None
    voice = S.get('voice')
    music = S.get('music')
    if declared_tone:
        tone, tone_src = declared_tone, 'declared'
    else:
        eff_music = music or 'bed'  # legacy consumer default (mix.py) — pre-v5 path
        if eff_music == 'bounce':
            tone = 'funny'
        elif eff_music in ('bed', 'tension') and voice in ('bm_george', 'bm_fable'):
            tone = 'serious'
        elif eff_music == 'tension':
            tone = 'amazing'
        else:
            tone = 'funny'
        tone_src = 'inferred'

    # ---- HOOK ----
    wc1 = len(toks[0])
    if wc1 > 11:
        extra = '; rubric score-1 band (>14 words)' if wc1 > 14 else ''
        add('HOOK-WORDS', 'FAIL', 1,
            f'{wc1} words (max 11) — the claim must complete inside line 1{extra}')
    s1 = strip_trailing(texts[0])
    if s1.endswith((',', ';', ':')) or (toks[0] and norm(toks[0][-1]) in TRAIL_OPEN):
        add('HOOK-FRAGMENT', 'FAIL', 1,
            f'line 1 ends open ("{s1[-12:]}") — the claim completes in line 2; '
            'end the claim inside line 1')
    markers = []  # (position 1-based, raw token)
    for j, raw in enumerate(toks[0]):
        nw = norms[0][j]
        if is_number_token(raw) or nw in ABSOLUTES or \
                (nw == 'no' and j + 1 < wc1 and norms[0][j + 1] == 'one') or \
                (j == 0 and nw == 'imagine'):
            markers.append((j + 1, raw))
    payoff_idx = None
    if not markers:
        add('HOOK-MARKER', 'FAIL', 1,
            'no payoff marker in line 1 (number, absolute, or line-initial "Imagine") — rubric H2')
    else:
        if markers[0][0] > 6:
            add('HOOK-MARKER', 'FAIL', 1,
                f'first marker "{markers[0][1]}" is at word {markers[0][0]} '
                '(must be within the first 6 words) — rubric H2')
        payoff_idx = markers[-1][0] - 1
    hook_word = lines[0].get('hookWord')
    if hook_word:
        hw = norm(hook_word)
        hits = [j for j, w in enumerate(norms[0]) if w == hw]
        if hits:
            payoff_idx = hits[-1]
    if payoff_idx is not None:
        wlist = [w for w in tl_words if w.get('line') == 0]
        if payoff_idx < len(wlist):
            e = float(wlist[payoff_idx]['e'])
            warn_t, fail_t = (3.8, 4.8) if mode == 'script' else (3.5, 4.5)
            if e > fail_t:
                add('HOOK-PAYOFF', 'FAIL', 1,
                    f'payoff word "{toks[0][payoff_idx]}" ends at {e:.2f} s '
                    f'(pass needs <= 3.5 s){est}')
            elif e > warn_t:
                add('HOOK-PAYOFF', 'WARN', 1,
                    f'payoff word "{toks[0][payoff_idx]}" ends at {e:.2f} s '
                    f'(pass needs <= 3.5 s){est}')
    joined1 = ' '.join(norms[0])
    for ph in EXPLAIN_PHRASES:
        pat = ph.split()
        if any(norms[0][k:k + len(pat)] == pat for k in range(len(norms[0]) - len(pat) + 1)):
            add('HOOK-EXPLAIN', 'FAIL', 1,
                f'line 1 contains "{ph}" — the claim explains itself (rubric H2); '
                'state the fact, let the video explain it')
            break
    def is_background(i):
        if i >= n:
            return False
        if '?' in texts[i]:
            return False
        for j, raw in enumerate(toks[i]):
            if is_number_token(raw) or norms[i][j] in ABSOLUTES or norms[i][j] in STAKE_WORDS:
                return False
        return True
    if n >= 3 and is_background(1) and is_background(2):
        add('HOOK-SECOND', 'WARN', None,
            'lines 2-3 carry no second surprise/stake/proof — rubric H3 '
            '(advisory: H3\'s final call is the scorer\'s, not the lint\'s)')

    # ---- BEAT SHAPE ----
    classes = [classify(ln) for ln in lines]
    for i in range(n - 1):
        if i == n - 2:
            continue  # the (last-1, last) pair is exempt: the punchline must close
        if classes[i] in ('CLOSE', 'PUNCH') and classes[i + 1] in ('CLOSE', 'PUNCH'):
            if (len(toks[i]) <= 5 and classes[i] == 'NEUTRAL') or \
                    (len(toks[i + 1]) <= 5 and classes[i + 1] == 'NEUTRAL'):
                continue  # a NEUTRAL micro line is a pause, not a closer
            add('CLOSE-PAIR', 'FAIL', i + 1,
                f'lines {i + 1} and {i + 2} both close — the line before the punchline '
                'must stay open (end it on a turn, or annotate "beat": "open")')
    starts = [float(l['start']) for l in tl_lines]
    end_all = float(tl_lines[-1]['end'])
    open_after_first = [i for i in range(1, n) if classes[i] == 'OPEN']
    if n > 1 and not open_after_first:
        add('REHOOK-GAP', 'FAIL', None,
            'no open (re-hook) lines after line 1 — mark re-hooks with '
            '"beat": "open" or "rehook": true (a re-hook every 10-15 s)')
    else:
        anchors = sorted({0.0} | {starts[i] for i in range(n) if classes[i] == 'OPEN'} | {end_all})
        for a, b in zip(anchors, anchors[1:]):
            gap = b - a
            if gap > 15:
                add('REHOOK-GAP', 'FAIL', None,
                    f'no re-hook for {gap:.1f} s between {a:.1f} s and {b:.1f} s '
                    f'(max 15 s){est}')
            elif gap > 13:
                add('REHOOK-GAP', 'WARN', None,
                    f're-hook gap {gap:.1f} s between {a:.1f} s and {b:.1f} s '
                    f'(13-15 s is the warn band){est}')
    # LOOP-WORDS
    def content(i):
        return [w for w in norms[i] if w and w not in STOPWORDS]
    final_content = set(content(n - 1))
    line1_content = set(content(0))
    shared = final_content & line1_content
    loop_ok, loop_how = False, ''
    if len(shared) >= 2:
        loop_ok, loop_how = True, f'shares {len(shared)} content words with line 1: ' + \
            ', '.join(sorted(shared))
    else:
        all_counts = {}
        for i in range(n):
            for w in content(i):
                all_counts[w] = all_counts.get(w, 0) + 1
        for w in sorted(final_content):
            seeded_early = any(w in content(i) for i in range(1, min(5, n - 1)))
            if all_counts.get(w, 0) >= 3 or (seeded_early and all_counts.get(w, 0) >= 2):
                loop_ok, loop_how = True, \
                    f'callback word "{w}" (seeded early, {all_counts[w]} occurrences)'
                break
    punch_bad = [i for i, ln in enumerate(lines) if ln.get('beat') == 'punch' and i < n - 1]
    for i in punch_bad:
        add('LOOP-WORDS', 'FAIL', i + 1,
            f'line {i + 1} is annotated "beat": "punch" but is not the final line — '
            'the punchline must close the script')
    if not loop_ok:
        lvl = 'WARN' if tone == 'serious' else 'FAIL'
        add('LOOP-WORDS', lvl, n,
            'final line shares no loop with line 1: reuse 2-3 exact content words from '
            'line 1, or seed a callback word at least twice early and return to it '
            '(serious tone: WARN only — the flip matters more than the words)')

    # ---- DURATION ----
    expected = expected_seconds(S)
    target = target_override if target_override is not None else S.get('target')
    if target is not None:
        target = float(target)
        tol = max(0.10 * target, 3.0)
        if abs(expected - target) > tol:
            eff = wps_for(S.get('voice')) * float(S.get('speed', 1.0))
            dw = int(round((target - expected) * eff))
            add('WORDS-TARGET', 'FAIL', None,
                f'expected {expected:.1f} s vs target {target:g} s (±{tol:.1f} s){est} — '
                f'{"add" if dw > 0 else "cut"} ~{abs(dw)} words at this voice/speed')
        if mode == 'full' and actual_dur is not None and abs(actual_dur - target) > tol:
            add('DUR-TARGET', 'FAIL', None,
                f'actual duration {actual_dur:.2f} s vs target {target:g} s (±{tol:.1f} s) — '
                'the words are right but the take is off; re-voice rather than rewrite')
    else:
        if expected < 40 or expected > 60:
            add('WORDS-BAND', 'WARN', None,
                f'expected {expected:.1f} s is outside the 40-60 s band and no target is '
                f'declared (add "target": SECONDS to script.json){est}')
    durs = [float(l['end']) - float(l['start']) for l in tl_lines]
    for i, d in enumerate(durs):
        if d > 5.5:
            add('LINE-LONG', 'WARN', i + 1,
                f'line {i + 1} runs {d:.1f} s (> 5.5 s){est} — split it; line cadence is cut cadence')
    if durs:
        med = sorted(durs)[len(durs) // 2]
        if med > 3.5:
            add('LINE-LONG', 'WARN', None,
                f'median line duration {med:.1f} s (> 3.5 s){est} — pacing risk (rubric P1/P3)')

    # ---- SOURCES ----
    src = S.get('sources')
    if not isinstance(src, list) or not src:
        add('SRC-MISSING', 'FAIL', None,
            'script.json has no non-empty "sources" list — hard law 4: facts are '
            'verified and listed')
    else:
        for k, entry in enumerate(src):
            if not isinstance(entry, str) or len(entry) < 25 or ',' not in entry:
                add('SRC-THIN', 'WARN', None,
                    f'source {k + 1} is too thin — expected shape: '
                    'Publisher, \'Title\' (the facts this source supports)')

    # ---- CAPTIONS ----
    missing, type_bad = [], []
    all_chunks = []  # (line_idx, chunk_text)
    for i, ln in enumerate(lines):
        if 'caps' not in ln or ln.get('caps') in ('', None) or \
                (isinstance(ln.get('caps'), str) and not ln['caps'].strip()):
            missing.append(i + 1)
        elif not isinstance(ln['caps'], str):
            type_bad.append(i)
        else:
            for c in ln['caps'].split('|'):
                c = c.strip()
                if c:
                    all_chunks.append((i, c))
    for i in type_bad:
        add('CAPS-TYPE', 'FAIL', i + 1,
            f'line {i + 1}: "caps" must be a pipe-joined string, e.g. '
            '"caps": "first chunk | second chunk" — an array crashes capture check')
    if missing:
        add('CAPS-MISSING', 'WARN' if lenient_caps else 'FAIL', None,
            'no authored "caps" on lines ' + ', '.join(str(m) for m in missing) +
            ' — un-authored lines fall back to ~1-word auto-chunks (rubric C1)')
    for i, c in all_chunks:
        cw = c.split()
        if len(cw) > 3:
            cap_run = any(cw[k][0].isupper() and cw[k + 1][0].isupper()
                          for k in range(len(cw) - 1))
            if len(cw) == 4 and cap_run:
                add('CAPS-CHUNK', 'WARN', i + 1,
                    f'line {i + 1} chunk "{c}" is 4 words — allowed only as an '
                    'unsplittable proper name; pass level still wants it split')
            else:
                add('CAPS-CHUNK', 'FAIL', i + 1,
                    f'line {i + 1} chunk "{c}" is {len(cw)} words (max 3) — split it')
    for i, ln in enumerate(lines):
        if isinstance(ln.get('caps'), str) and ln['caps'].strip():
            caps_stream = spoken_stream(' '.join(
                c.strip() for c in ln['caps'].split('|') if c.strip()))
            text_stream = spoken_stream(texts[i])
            if caps_stream != text_stream:
                k = next((x for x in range(min(len(caps_stream), len(text_stream)))
                          if caps_stream[x] != text_stream[x]),
                         min(len(caps_stream), len(text_stream)))
                a = caps_stream[k] if k < len(caps_stream) else '<end>'
                b = text_stream[k] if k < len(text_stream) else '<end>'
                add('CAPS-MISMATCH', 'FAIL', i + 1,
                    f'line {i + 1}: caps diverge from text at word {k + 1} '
                    f'(caps "{a}" vs text "{b}") — captions.ts anchors each chunk by '
                    'its first word and silently misanchors on divergence')
    if all_chunks:
        one_out = sum(1 for i, c in all_chunks
                      if len(c.split()) == 1 and (i + 1) not in (1, n))
        pct = 100.0 * one_out / len(all_chunks)
        if pct > 25:
            add('CAPS-POP', 'FAIL', None,
                f'{one_out} of {len(all_chunks)} chunks are one-word outside line 1 / '
                f'the final line ({pct:.0f}% > 25%) — one-word pops are for the hook, '
                'the punchline and emphasis only')
        elif pct > 15:
            add('CAPS-POP', 'WARN', None,
                f'{one_out} of {len(all_chunks)} chunks are one-word outside line 1 / '
                f'the final line ({pct:.0f}% > 15%) — rubric C1 pass is <= 15%')

    # ---- TONE WIRING ----
    if tone_src == 'inferred':
        bad = f' (the value "{S.get("tone")}" is not a tone)' if 'tone' in S else ''
        add('TONE-MISSING', 'WARN', None,
            f'no "tone" in script.json{bad} — inferred {tone} from voice/music; '
            'tone-dependent checks ran against the inference')
    if declared_tone and music is not None and music not in TONE_MUSIC_OK[declared_tone]:
        add('TONE-MUSIC', 'FAIL', None,
            f'tone "{declared_tone}" cannot use music "{music}" (requires '
            f'{" or ".join(sorted(TONE_MUSIC_OK[declared_tone]))}) — this is the silent '
            'wrong-tone chain from the audits, terminated here')
    if voice and voice not in TONE_VOICE_OK[tone]:
        add('TONE-VOICE', 'WARN', None,
            f'voice "{voice}" is outside the {tone} row of the tone table '
            '(library/story.md §1.5) — casting has exceptions, music register does not')
    if tone in ('funny', 'amazing') and n > 2:
        mid = end_all / 2.0
        if not any(len(toks[i]) <= 5 for i in range(n) if starts[i] >= mid):
            add('MICRO-BEAT', 'WARN', None,
                'no line of <=5 words after the midpoint — no deadpan micro-beat '
                'candidate (one short flat line after the biggest reveal)')
    return R, {'tone': tone, 'toneSource': tone_src, 'target': target,
               'expected': expected, 'actual': actual_dur if mode == 'full' else None}

def main(argv=None):
    ap = argparse.ArgumentParser(description='Story-layer lint for a Tall Tale project.')
    ap.add_argument('project', nargs='?', help='project directory containing script.json')
    ap.add_argument('--target', type=float, default=None)
    ap.add_argument('--lenient-caps', action='store_true')
    ap.add_argument('--warn-only', action='store_true')
    ap.add_argument('--json', action='store_true')
    ap.add_argument('--codes', action='store_true')
    ap.add_argument('--verbose', action='store_true')
    a = ap.parse_args(argv)
    if a.codes:
        for code, lvl, desc in CODES:
            print(f'{code:15s} {lvl:9s} {desc}')
        return 0
    if not a.project:
        print('ERROR: a project directory is required (or use --codes)', file=sys.stderr)
        return 2
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
    # timing source: full mode iff BOTH data files exist; a present-but-malformed
    # data file is an input error (exit 2), never a silent fallback.
    wp, lp = P / 'data' / 'words.json', P / 'data' / 'lines.json'
    mode, tl_lines, tl_words, actual = 'script', None, None, None
    parsed = {}
    for key, path in (('words', wp), ('lines', lp)):
        if path.exists():
            try:
                parsed[key] = json.loads(path.read_text())
            except Exception as e:
                print(f'ERROR: {path} present but malformed: {e}', file=sys.stderr)
                return 2
    if 'words' in parsed and 'lines' in parsed:
        try:
            tl_words = parsed['words']
            tl_lines = parsed['lines']['lines']
            actual = float(parsed['lines']['duration'])
            if len(tl_lines) != len(S['lines']):
                raise ValueError('lines.json line count != script.json')
            mode = 'full'
        except Exception as e:
            print(f'ERROR: data/words.json + data/lines.json unusable: {e}', file=sys.stderr)
            return 2
    if mode == 'script':
        tl_lines, tl_words = estimate_timeline(S)
    try:
        R, meta = run_lint(S, mode, tl_lines, tl_words, actual, a.target, a.lenient_caps)
    except (KeyError, TypeError, ValueError) as e:
        print(f'ERROR: script.json shape not understood: {e!r}', file=sys.stderr)
        return 2
    fails = sum(1 for r in R if r['level'] == 'FAIL')
    warns = sum(1 for r in R if r['level'] == 'WARN')
    verdict = 'FAIL' if fails else 'PASS'
    if a.json:
        print(json.dumps({
            'verdict': verdict, 'mode': mode, 'tone': meta['tone'],
            'toneSource': meta['toneSource'],
            'target': meta['target'],
            'expectedSeconds': round(meta['expected'], 2),
            'actualSeconds': (round(meta['actual'], 2) if meta['actual'] is not None else None),
            'checks': R, 'counts': {'fail': fails, 'warn': warns},
        }, indent=1))
    else:
        order = {c: k for k, (c, _, _) in enumerate(CODES)}
        shown = sorted(R, key=lambda r: order.get(r['code'], 99))
        if a.verbose:
            bad_codes = {r['code'] for r in R}
            for code, _, _ in CODES:
                if code not in bad_codes:
                    print(f'PASS {code}')
        for r in shown:
            loc = f' line {r["line"]}' if r['line'] else ''
            print(f'{r["level"]} {r["code"]}{loc}: {r["msg"]}')
        summ = f'SCRIPT LINT: {fails} FAIL, {warns} WARN · mode {mode} · ' \
               f'est. {meta["expected"]:.1f} s'
        if meta['target'] is not None:
            summ += f' / target {meta["target"]:g} s'
        if mode == 'full' and meta['actual'] is not None:
            summ += f' · actual {meta["actual"]:.2f} s'
        summ += f' · verdict {verdict}'
        print(summ)
    if a.warn_only:
        return 0
    return 1 if fails else 0

if __name__ == '__main__':
    sys.exit(main())
