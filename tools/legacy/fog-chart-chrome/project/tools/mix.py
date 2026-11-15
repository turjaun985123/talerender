"""Tall Tale soundtrack: narration + soft procedural sound effects + a quiet music bed, all made here with numpy.
No samples, no AI audio, no downloads.

  python3 tools/mix.py [project=.]          → data/mix.wav (48 kHz stereo)

Reads data/cues.json (node tools/capture.mjs cues), data/narration.wav if it exists (voice.py or words.py --audio),
and script.json "music": "bed" (the procedural score, default) · "none" · "audio/your-track.mp3" (your own royalty-free
track, any format ffmpeg reads). The music ducks under the voice.

Sound rules (STYLE.md): every effect is soft and rounded, nothing above ~5 kHz, no ticks / hats / glitches / beeps.
Cue types: see FX table below (21). Each cue also accepts gain / dur / pan.
script.json "loop": true (opt-in) makes the music loop-safe: no terminal fade, seamless join.
Loudness gate: prints `loudness: PASS/FAIL` and exits 0 / 3 (2 = bad input) — tools/measure.py re-checks standalone.
"""
import json, re, subprocess, sys
from pathlib import Path
import numpy as np
from scipy import signal
from scipy.io import wavfile

P = Path(sys.argv[1] if len(sys.argv) > 1 else '.')
SR = 48000
rng = np.random.default_rng(11)
def _load_json(f, hint):
    try:
        return json.loads((P / f).read_text())
    except Exception as e:
        print(f'mix.py: cannot read {f} ({type(e).__name__}: {e}) — {hint}'); sys.exit(2)
cj = _load_json('data/cues.json', 'run `node tools/capture.mjs cues` first')
if 'duration' not in cj or 'cues' not in cj:
    print('mix.py: data/cues.json has no "duration"/"cues" — regenerate with `node tools/capture.mjs cues`'); sys.exit(2)
script = _load_json('script.json', 'mix.py must run in a project folder')
DUR = float(cj['duration'])
N = int(DUR * SR) + SR // 10


def lp(x, f, o=2): b, a = signal.butter(o, min(f, SR / 2 - 100) / (SR / 2), 'low'); return signal.lfilter(b, a, x)
def hp(x, f, o=2): b, a = signal.butter(o, f / (SR / 2), 'high'); return signal.lfilter(b, a, x)
def bp(x, f0, f1, o=2): b, a = signal.butter(o, [f0 / (SR / 2), f1 / (SR / 2)], 'band'); return signal.lfilter(b, a, x)
def db(x): return 10 ** (x / 20)
def tt(d): return np.arange(int(d * SR)) / SR
def env(n, a=0.005, r=None):
    """attack a seconds then exponential release over the rest"""
    e = np.ones(n); na = max(1, int(a * SR)); e[:na] = np.linspace(0, 1, na)
    rest = n - na; e[na:] = np.exp(-np.linspace(0, r or 5, rest)) if rest > 0 else 1; return e
def sweep(f0, f1, d, shape='exp'):
    t = tt(d); f = f0 * (f1 / f0) ** (t / d) if shape == 'exp' else f0 + (f1 - f0) * t / d
    return np.sin(2 * np.pi * np.cumsum(f) / SR)
def noise(d): return rng.standard_normal(int(d * SR))


# ---------- the effects ----------
def whoosh(d=0.38, **_):
    n = noise(d); lo, hi = lp(n, 500), lp(n, 2200)
    k = np.sin(np.pi * np.linspace(0, 1, len(n))); x = (lo * (1 - k) + hi * k) * k ** 1.5
    return x / (np.abs(x).max() + 1e-9) * 0.5
def slide(d=1.0, **_):
    n = lp(noise(d), 700); e = np.sin(np.pi * np.linspace(0, 1, len(n))) ** 2
    return n / (np.abs(n).max() + 1e-9) * e * 0.35 + sweep(260, 120, d) * e * 0.08
def pop(**_): x = sweep(560, 240, 0.11) * env(int(0.11 * SR), 0.002, 6); return lp(x, 3000) * 0.7
def plop(**_): x = sweep(700, 180, 0.14) * env(int(0.14 * SR), 0.002, 5); return lp(x, 2500) * 0.6
def gulp(**_):
    a = sweep(320, 140, 0.16) * env(int(0.16 * SR), 0.01, 4); b = sweep(240, 90, 0.2) * env(int(0.2 * SR), 0.01, 4)
    x = np.concatenate([a, np.zeros(int(0.04 * SR)), b]); return lp(x, 1800) * 0.8
def boing(**_):
    d = 0.42; t = tt(d); f = 200 * (1 + 0.6 * t / d) * (1 + 0.08 * np.sin(2 * np.pi * 18 * t))
    return lp(np.sin(2 * np.pi * np.cumsum(f) / SR) * env(len(t), 0.004, 4), 2000) * 0.55
def bubbles(d=2.0, **_):
    x = np.zeros(int(d * SR))
    for _ in range(int(d * 7)):
        st = rng.integers(0, max(1, len(x) - int(0.08 * SR))); f = rng.uniform(380, 900); b = sweep(f, f * 1.6, 0.06) * env(int(0.06 * SR), 0.003, 5)
        x[st:st + len(b)] += b * rng.uniform(0.2, 0.5)
    return lp(x, 2500) * 0.5
def sizzle(d=1.0, **_):
    n = bp(noise(d), 1800, 4500); crack = (rng.random(len(n)) < 0.002).astype(float); crack = lp(crack, 300) * 40
    e = np.minimum(1, np.linspace(0, 6, len(n))) * np.linspace(1, 0.2, len(n)); return n * (0.4 + crack) * e * 0.12
def sparkle(d=1.2, **_):
    notes = [784, 880, 1047, 1175, 1319, 1568]; x = np.zeros(int(d * SR) + SR // 2)
    for k, f in enumerate(notes):
        st = int(k * d / len(notes) * SR); n = int(0.4 * SR); t = tt(0.4)
        x[st:st + n] += (np.sin(2 * np.pi * f * t) + 0.3 * np.sin(4 * np.pi * f * t)) * env(n, 0.004, 6) * 0.22
    return lp(x, 4500)
def thud(**_):
    d = 0.35; t = tt(d); x = np.sin(2 * np.pi * (70 + 40 * np.exp(-t * 30)) * t) * env(len(t), 0.002, 7)
    return x * 0.9 + lp(noise(d), 220) * env(len(t), 0.001, 12) * 0.3
def slam(**_):
    d = 0.5; t = tt(d); x = np.sin(2 * np.pi * (60 + 60 * np.exp(-t * 25)) * t) * env(len(t), 0.002, 6)
    return np.tanh(1.6 * (x + lp(noise(d), 1200) * env(len(t), 0.001, 10) * 0.6)) * 0.9
def sting(**_):
    d = 0.7; t = tt(d); x = sum(np.sin(2 * np.pi * f * t) for f in (110, 130.81, 164.81, 220 * 1.005))
    return lp(x * env(len(t), 0.004, 5), 2500) * 0.5
def tock(**_): d = 0.09; t = tt(d); return (np.sin(2 * np.pi * 420 * t) + 0.3 * np.sin(2 * np.pi * 840 * t)) * env(len(t), 0.001, 9) * 0.35
def zip_(**_):
    d = 0.32; n = bp(noise(d), 400, 3000); k = np.linspace(0, 1, len(n))
    return (n / (np.abs(n).max() + 1e-9) * 0.35 + sweep(300, 1100, d) * 0.12) * np.sin(np.pi * k) ** 1.2
def alarm(d=2.0, **_):
    x = np.zeros(int(d * SR)); one = np.sin(2 * np.pi * 150 * tt(0.22)) * env(int(0.22 * SR), 0.02, 5)
    for st in np.arange(0, d - 0.25, 0.5): i = int(st * SR); x[i:i + len(one)] += one
    return lp(x, 900) * 0.55

def inhale(d=1.0, **_):
    n = bp(noise(d), 300, 2600); k = np.linspace(0, 1, len(n)); e = np.sin(np.pi * np.minimum(1, k * 1.15)) ** 1.5 * (0.6 + 0.4 * k)
    return n / (np.abs(n).max() + 1e-9) * e * 0.5
def heartbeat(**_):
    one = lambda d, f: np.sin(2 * np.pi * f * tt(d)) * env(int(d * SR), 0.004, 7)
    x = np.concatenate([one(0.16, 58), np.zeros(int(0.09 * SR)), one(0.2, 52) * 0.75]); return lp(x, 300) * 1.0

def ding(**_):
    t = tt(1.4); x = sum(np.sin(2 * np.pi * f * t) * g for f, g in [(1318, 1), (2637, 0.35), (3951, 0.12)]) * env(len(t), 0.002, 4); return x * 0.5
def chirp(d=1.0, **_):
    x = np.zeros(int(d * SR))
    for k, st in enumerate(np.arange(0, d - 0.12, 0.17)):
        i = int(st * SR); c = sweep(3200 + (k % 3) * 300, 4400, 0.08) * env(int(0.08 * SR), 0.004, 6); x[i:i + len(c)] += c[:len(x) - i] * (0.6 + 0.4 * rng.random())
    return x * 0.4
def chug(d=2.0, **_):
    x = np.zeros(int(d * SR)); one = bp(noise(0.14), 200, 1600) * env(int(0.14 * SR), 0.005, 6)
    for k, st in enumerate(np.arange(0, d - 0.15, 0.22)): i = int(st * SR); x[i:i + len(one)] += one * (1 if k % 2 == 0 else 0.6)
    return x * 0.7
def stampit(**_):
    t = tt(0.18); x = np.sin(2 * np.pi * 95 * t) * env(len(t), 0.001, 10) + bp(noise(0.18), 1500, 5000) * env(len(t), 0.001, 14) * 0.6; return x * 0.9

FX = dict(ding=ding, chirp=chirp, chug=chug, stamp=stampit, inhale=inhale, heartbeat=heartbeat, whoosh=whoosh, slide=slide, pop=pop, plop=plop, gulp=gulp, boing=boing, bubbles=bubbles, sizzle=sizzle,
          sparkle=sparkle, thud=thud, slam=slam, sting=sting, tock=tock, zip=zip_, alarm=alarm)
LEVEL = dict(ding=-17, chirp=-19, chug=-20, stamp=-12, inhale=-15, heartbeat=-8, whoosh=-17, slide=-19, pop=-14, plop=-15, gulp=-11, boing=-15, bubbles=-21, sizzle=-18, sparkle=-17,
             thud=-11, slam=-9, sting=-12, tock=-18, zip=-14, alarm=-17)


def _fold(x, n):
    """Wrap-fold a canvas of len(x) >= n down to n samples: the overhang is added
    onto the head, so looping the result continues the track seamlessly (D7-1).
    With len(x) <= n this is a plain slice; addition order matches x[:k] += x[n:]."""
    if len(x) <= n: return x[:n]
    out = np.zeros(n); np.add.at(out, np.arange(len(x)) % n, x); return out


# ---------- the music bed: soft pad + plucked arpeggio, 100 BPM, I–vi–IV–V ----------
def music_bed(loop=False):
    bpm = 100; beat = 60 / bpm; bar = beat * 4
    chords = [[261.6, 329.6, 392.0], [220.0, 261.6, 329.6], [174.6, 220.0, 261.6], [196.0, 246.9, 293.7]]
    NN = N + int(np.ceil(bar * SR)) if loop else N
    x = np.zeros(NN)
    nb = int(np.ceil(DUR / bar)) + (2 if loop else 1)
    for b in range(nb):
        ch = chords[b % 4]; st = int(b * bar * SR); n = int(bar * SR)
        if st >= NN: break
        t = tt(bar); e = np.minimum(1, t / 0.4) * np.minimum(1, (bar - t) / 0.4)
        pad = sum(np.sin(2 * np.pi * f / 2 * t) + 0.25 * np.sin(2 * np.pi * f * t) for f in ch) * e * 0.10
        x[st:st + n] += pad[:max(0, min(n, NN - st))]
        for k in range(8):                                                       # eighth-note pluck arpeggio
            f = ch[[0, 1, 2, 1, 2, 0, 1, 2][k]] * (2 if k % 4 == 2 else 1); ps = st + int(k * beat / 2 * SR); pn = int(0.5 * SR)
            if ps + pn >= NN: continue
            pt = tt(0.5); pl = (np.sin(2 * np.pi * f * pt) + 0.35 * np.sin(4 * np.pi * f * pt)) * env(pn, 0.003, 7) * 0.12
            x[ps:ps + pn] += pl
    x = lp(x, 3200)
    if loop:
        x = _fold(x, N)
        return x / (np.abs(x).max() + 1e-9)
    fade = np.ones(N); fo = int(1.2 * SR); fade[-fo:] = np.linspace(1, 0, fo)
    return x / (np.abs(x).max() + 1e-9) * fade

def music_bounce(loop=False):
    """light comedy: oom-pah bass + staccato plucked melody, 118 BPM, major (script.json "music": "bounce")"""
    bpm = 118; beat = 60 / bpm
    NN = N + int(np.ceil(4 * beat * SR)) if loop else N
    x = np.zeros(NN)
    prog = [(130.8, [261.6, 329.6, 392.0]), (174.6, [261.6, 349.2, 440.0]), (196.0, [246.9, 293.7, 392.0]), (130.8, [261.6, 329.6, 392.0])]
    mel = [0, 2, 1, 2, 0, 1, 2, 1]
    pl = lambda f, d, g: (np.sin(2 * np.pi * f * tt(d)) + 0.4 * np.sin(4 * np.pi * f * tt(d)) + 0.15 * np.sin(6 * np.pi * f * tt(d))) * env(int(d * SR), 0.003, 9) * g
    for k in range(int(DUR / beat) + 1 + (6 if loop else 0)):
        bass, ch = prog[(k // 8) % 4]; i = int(k * beat * SR)
        b = pl(bass if k % 2 == 0 else bass * 1.5, 0.3, 0.35) if k % 2 == 0 else pl(ch[0] / 2 * 1.5, 0.22, 0.18)
        if i + len(b) < NN: x[i:i + len(b)] += b
        if k % 2 == 1:
            c = sum(pl(f, 0.16, 0.07) for f in ch)
            if i + len(c) < NN: x[i:i + len(c)] += c
        f = ch[mel[k % 8]] * 2; m = pl(f, 0.18, 0.09); j = i + int(beat / 2 * SR) if k % 4 == 3 else i
        if j + len(m) < NN and k % 8 not in (3, 7): x[j:j + len(m)] += m
    x = lp(x, 4200)
    if loop:
        x = _fold(x, N)
        return x / (np.abs(x).max() + 1e-9)
    fade = np.ones(N); fo = int(1.2 * SR); fade[-fo:] = np.linspace(1, 0, fo)
    return x / (np.abs(x).max() + 1e-9) * fade

def music_tension(loop=False):
    """low drone + slow minor pulse, for mysterious / tense explainers (script.json "music": "tension")"""
    t = np.arange(N) / SR; x = np.zeros(N)
    for f, g in [(55, 0.5), (82.4, 0.3), (110, 0.2), (130.8, 0.12)]: x += np.sin(2 * np.pi * f * t + np.sin(t * 0.3) * 0.5) * g
    x *= 0.55 + 0.45 * np.sin(2 * np.pi * t / 8) ** 2
    beat = 60 / 84; ping = lambda f: (np.sin(2 * np.pi * f * tt(1.2)) + 0.3 * np.sin(4 * np.pi * f * tt(1.2))) * env(int(1.2 * SR), 0.01, 5)
    seq = [220.0, 261.6, 196.0, 174.6]
    for k in range(int(DUR / (beat * 2))):
        i = int(k * beat * 2 * SR); pg = ping(seq[(k // 2) % 4]) * 0.35
        if i + len(pg) < N: x[i:i + len(pg)] += pg
    x = lp(x, 2200)
    x = x / (np.abs(x).max() + 1e-9)
    if loop:
        # drone never decays, so no wrap-fold: equal-power crossfade of the tail
        # into a saved copy of the head — level-continuous at the join (D7-1).
        h = int(0.5 * SR); head = x[:h].copy(); th = np.linspace(0, np.pi / 2, h)
        x[-h:] = x[-h:] * np.cos(th) + head * np.sin(th)
        return x
    fade = np.ones(N); fo = int(1.5 * SR); fade[-fo:] = np.linspace(1, 0, fo); fi = int(0.8 * SR); fade[:fi] *= np.linspace(0, 1, fi)
    return x * fade

def load_audio(f, max_seconds=None):
    # a12 G5: cap the decode with -t (project duration + a small margin) BEFORE
    # the caller's [:N] slice, so a long user-supplied music track is never
    # decoded in full (>4 GB transient for a 60-minute track) just to keep its
    # first minute. The same cap is safe for the narration, whose length is
    # the project duration by construction.
    if max_seconds is None: max_seconds = DUR + 5
    cmd = ['ffmpeg', '-v', 'error', '-i', str(f)]
    if max_seconds and float(max_seconds) > 0: cmd += ['-t', f'{float(max_seconds):.3f}']
    cmd += ['-f', 'f32le', '-ac', '2', '-ar', str(SR), '-']
    out = subprocess.run(cmd, capture_output=True, check=True).stdout
    a = np.frombuffer(out, dtype=np.float32).reshape(-1, 2).astype(np.float64); return a


def _tile_loop(src, n):
    """Tile a decoded mono track to n samples with 0.25 s equal-power seam
    crossfades, then wrap-fold the final overhang onto the head (D7-1 loop path
    for external music files). Tracks already >= n skip the tiling."""
    if len(src) == 0: return np.zeros(n)
    h = int(0.25 * SR); out = np.array(src, dtype=np.float64)
    if len(out) > h:
        th = np.linspace(0, np.pi / 2, h)
        while len(out) < n:
            out[-h:] = out[-h:] * np.cos(th) + src[:h] * np.sin(th)
            out = np.concatenate([out, src[h:]])
    elif len(out) < n:
        out = np.tile(out, int(np.ceil(n / len(out))) + 1)
    return _fold(out, n)


def _measure_loudness(f):
    """ffmpeg loudnorm analysis on f -> (input_i, input_tp, input_lra,
    input_thresh, target_offset).

    Raises on any failure (missing ffmpeg, no JSON in the output, ...) so the
    caller can skip loudness handling gracefully and keep the file as written.
    """
    r = subprocess.run(['ffmpeg', '-hide_banner', '-i', str(f), '-af', 'loudnorm=I=-14:TP=-1:LRA=11:print_format=json', '-f', 'null', '-'], capture_output=True, text=True)
    text = (r.stderr or '') + (r.stdout or '')
    m = re.search(r'\{\s*"input_i".*?\}', text, re.S)
    if not m: raise ValueError('loudnorm JSON not found in ffmpeg output')
    vals = json.loads(m.group(0))
    return (float(vals['input_i']), float(vals['input_tp']), float(vals['input_lra']),
            float(vals['input_thresh']), float(vals['target_offset']))


# D7-2: the loudness gate (definitions + the one loudnorm call) lives in
# tools/measure.py so mix.py and the standalone checker can never drift apart.
# Pre-v5 projects have no measure.py beside mix.py — ImportError keeps the
# local _measure_loudness above and the local gate constants, unchanged.
GATE_I, GATE_TP = (-14.5, -13.5), -1.0
_shared_gate_verdict = None
try:
    from measure import measure_loudness as _shared_measure_loudness, GATE_I, GATE_TP
    from measure import gate_verdict as _shared_gate_verdict

    def _measure_loudness(f):
        d = _shared_measure_loudness(f)
        return (d['integrated_lufs'], d['true_peak_dbtp'], d['lra'], d['threshold'], d['offset'])
except ImportError:
    pass


def _gate_verdict(i, tp):
    """(ok, reasons) at the gate above. Uses measure.py's copy when importable;
    the fallback mirrors it exactly, including the 0.1 dB printed-precision
    comparison (loudnorm's TP readout wobbles ~0.01–0.1 dB between passes)."""
    if _shared_gate_verdict is not None:
        return _shared_gate_verdict({'integrated_lufs': i, 'true_peak_dbtp': tp})
    reasons = []
    i, tp = round(i, 1), round(tp, 1)
    if not np.isfinite(i): reasons.append('integrated loudness not measurable')
    elif not (GATE_I[0] <= i <= GATE_I[1]): reasons.append(f'integrated {i:.1f} LUFS outside -14 ± 0.5')
    if not np.isfinite(tp): reasons.append('true peak not measurable')
    elif tp > GATE_TP: reasons.append(f'TP {tp:.1f} dBTP > -1.0')
    return (not reasons), reasons


mix = np.zeros((N, 2))
# voice
vf = P / 'data/narration.wav'
voice = None
if vf.exists():
    v = load_audio(vf)[:N]; voice = np.zeros((N, 2)); voice[:len(v)] = v
    rms = np.sqrt(np.mean(voice ** 2) + 1e-12); voice *= db(-19) / rms
    mix += voice
# music — resolution order (D3 §1.5): explicit script.music → tone-table music
# for script.tone → 'bed'. The last step is the v4 default, reached only by
# pre-v5 projects with neither field, so their mixes stay byte-identical; the
# reliance is printed whenever it happens (D0 P-07 defaults honesty).
TONE_MUSIC = {'funny': 'bounce', 'amazing': 'tension', 'serious': 'bed'}
LOOP = bool(script.get('loop', False))
if 'music' in script:
    mus = script['music']
else:
    mus = TONE_MUSIC.get(script.get('tone'), 'bed')
    print(f'music: (missing — tone "{script.get("tone")}" default {mus})' if script.get('tone') in TONE_MUSIC
          else 'music: (missing — default bed)')
if LOOP:
    try:
        _outro = float(script.get('outro', 0.9))
        if _outro > 0.3: print(f'loop: warning — outro {_outro:g} s > 0.3 s; SKILL §3 loop episodes use outro <= 0.3')
    except (TypeError, ValueError):
        pass
if mus and mus != 'none':
    if not isinstance(mus, str) or (mus not in ('bed', 'bounce', 'tension') and not (P / mus).exists()):
        print(f'mix.py: unknown music "{mus}" — use bed, bounce, tension, none, or a file path'); sys.exit(2)
    if mus == 'bed': m = music_bed(loop=LOOP)
    elif mus == 'tension': m = music_tension(loop=LOOP)
    elif mus == 'bounce': m = music_bounce(loop=LOOP)
    elif LOOP:
        a = load_audio(P / mus); m = _tile_loop(a.mean(1), N); m /= np.abs(m).max() + 1e-9
    else:
        a = load_audio(P / mus)[:N]; m = np.zeros(N); m[:len(a)] = a.mean(1); m /= np.abs(m).max() + 1e-9
    m = np.stack([m, m], 1) * db(-24 if voice is not None else -20)
    if voice is not None:                                                        # duck under the voice
        lev = np.abs(voice.mean(1)); lev = signal.lfilter([1 - 0.9995], [1, -0.9995], lev); lev /= lev.max() + 1e-9
        m *= (1 - 0.55 * np.clip(lev * 3, 0, 1))[:, None]
        # D7-2 self-report: how far the ducked music actually sits under the
        # voice (50 ms blocks; voice-active = block RMS above -40 dBFS).
        _blk = int(0.05 * SR); _nb = N // _blk
        _vm = voice.mean(1)[:_nb * _blk].reshape(_nb, _blk); _mm = m.mean(1)[:_nb * _blk].reshape(_nb, _blk)
        _act = np.sqrt((_vm ** 2).mean(1)) > db(-40)
        if _act.any() and (~_act).any():
            _depth = 20 * np.log10((np.sqrt((_mm[~_act] ** 2).mean()) + 1e-12) / (np.sqrt((_mm[_act] ** 2).mean()) + 1e-12))
            print(f'ducking: music {-_depth:.1f} dB under voice (voice-active vs voice-absent RMS; target >= 4 dB)')
    mix += m
# effects
_placed = _over_voice = 0
for c in cj['cues']:
    fn = FX.get(c['type'])
    if not fn: print('unknown cue type, skipped:', c['type']); continue
    x = fn(**({'d': c['dur']} if c.get('dur') else {}))
    x = lp(x, 5000) * db(LEVEL[c['type']]) * c.get('gain', 1.0) / (np.abs(x).max() + 1e-9) * 0.9
    i = int(c['t'] * SR); n = min(len(x), N - i)
    if n > 0 and i >= 0:
        pan = c.get('pan', 0.0); mix[i:i + n, 0] += x[:n] * (1 - max(0, pan)); mix[i:i + n, 1] += x[:n] * (1 + min(0, pan))
        _placed += 1
        # D7-4: a cue counts as "over voice" when the voice stem's RMS inside
        # the cue's window is above -40 dBFS. Informational, not a gate.
        if voice is not None and np.sqrt(np.mean(voice[i:i + n] ** 2) + 1e-12) > db(-40): _over_voice += 1
print(f'cues: {_placed} placed, {_over_voice} over voice')
# master: gentle soft-clip, peak -1 dBFS
mix = np.tanh(mix * 1.2) / 1.2
mix *= db(-1) / (np.abs(mix).max() + 1e-9)
mix_path = P / 'data/mix.wav'
wavfile.write(mix_path, SR, (mix * 32767).astype(np.int16))
# loudness: two-pass master targeting -14 LUFS integrated, TP <= -1 dBTP.
# Pass 0 above pins the SAMPLE peak to -1 dBFS, so the oversampled TRUE peak
# is usually > -1 dBTP and a purely linear gain (v2) can be <= 0 dB, leaving
# the target unreachable. Hence: if the true-peak headroom alone allows the
# required gain, stay purely linear; otherwise gain to the TP ceiling and
# finish with loudnorm's limiter stage (linear=false, two-pass measured
# values), which may compress peaks to reach -14 while respecting TP -1.
# D7-2 verdict: exactly one `loudness: PASS/FAIL` line is printed and the exit
# code follows it (0 PASS, 3 FAIL) — a failure never deletes mix.wav, which
# stays exactly as written for QA (for a limiter failure, as the linear step
# left it). Gate: integrated in [-14.5, -13.5] LUFS and TP <= -1.0 dBTP.
_exit = 3
try:
    _I0, _TP0, _LRA0, _thresh0, _offset0 = _measure_loudness(mix_path)
    if not np.isfinite(_I0) or not np.isfinite(_TP0):
        print('loudness: FAIL (integrated loudness not measurable — silence?)')
    else:
        _target_i, _tp_ceiling = -14.0, -1.0
        _required = _target_i - _I0
        _headroom = _tp_ceiling - _TP0
        _shortfall = _required - _headroom
        _path = 'linear'
        _applied = min(_required, _headroom)
        if abs(_applied) >= 0.01:
            mix = mix * db(_applied)
            wavfile.write(mix_path, SR, (mix * 32767).astype(np.int16))
        if _shortfall > 0.1:
            # Limiter stage: re-measure the CURRENT (headroom-gained) file,
            # then run loudnorm in two-pass form with linear=false.
            _I1, _TP1, _LRA1, _thresh1, _offset1 = _measure_loudness(mix_path)
            if not all(np.isfinite(_v) for _v in (_I1, _TP1, _LRA1, _thresh1, _offset1)):
                raise ValueError('non-finite loudnorm measured values for limiter stage')
            _af = (f'loudnorm=I={_target_i:.1f}:TP={_tp_ceiling:.1f}:LRA=11'
                   f':measured_I={_I1:.2f}:measured_TP={_TP1:.2f}:measured_LRA={_LRA1:.2f}'
                   f':measured_thresh={_thresh1:.2f}:offset={_offset1:.2f}:linear=false')
            _tmp_path = mix_path.with_name('mix_loudnorm_tmp.wav')
            try:
                subprocess.run(['ffmpeg', '-y', '-hide_banner', '-v', 'error', '-i', str(mix_path),
                                '-af', _af, '-c:a', 'pcm_s16le', '-ar', str(SR), '-ac', '2', str(_tmp_path)],
                               capture_output=True, text=True, check=True)
                _tmp_path.replace(mix_path)
            finally:
                if _tmp_path.exists(): _tmp_path.unlink()
            _path = 'linear+limiter'
        _If, _TPf, _LRAf, _threshf, _offsetf = _measure_loudness(mix_path)
        _gain = _If - _I0 if np.isfinite(_If) else _applied
        print(f'loudness: {_I0:.1f} LUFS / TP {_TP0:.1f} -> {_If:.1f} LUFS / TP {_TPf:.1f} '
              f'(target -14 LUFS, TP <= -1 dBTP; path: {_path}; gain {_gain:+.1f} dB)')
        _ok, _reasons = _gate_verdict(_If, _TPf)
        if not _ok:
            print(f'loudness: FAIL ({" | ".join(_reasons)})')
        else:
            print(f'loudness: PASS ({_If:.1f} LUFS, TP {_TPf:.1f} dBTP)'); _exit = 0
except Exception as _e:
    print(f'loudness: FAIL (stage skipped: {type(_e).__name__}: {_e})')
print(f"data/mix.wav: {DUR:.2f} s · voice: {'yes' if voice is not None else 'NO (captions-only)'} · music: {mus} · {len(cj['cues'])} cues")
sys.exit(_exit)
