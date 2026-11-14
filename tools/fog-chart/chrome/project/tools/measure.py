"""Standalone loudness + loop-tail check for a finished mix (D7-2).

  python3 tools/measure.py [project=.] [--json]   → measures data/mix.wav

The gate (the skill's audio semantics; mix.py imports these same definitions,
so the builder and the checker can never drift apart):
  PASS ⟺ integrated loudness in [-14.5, -13.5] LUFS AND true peak <= -1.0 dBTP.
Anything else — off-target, over peak, unmeasurable (silence, ffmpeg missing) —
is FAIL; there is no third state.
Exit codes mirror mix.py: 0 PASS · 3 FAIL · 2 mix.wav missing.

Also reports the loop metric: RMS of the final 0.5 s vs the median 1 s-window
RMS. Mix-level approximation — it assumes the voice is in its outro silence at
the tail, so a loop-safe mix keeps a healthy ratio while a faded ending → ~0.
"""
import json, re, subprocess, sys
from pathlib import Path
import numpy as np

GATE_I = (-14.5, -13.5)
GATE_TP = -1.0


def measure_loudness(path):
    """ffmpeg loudnorm analysis on path -> dict(integrated_lufs, true_peak_dbtp,
    lra, threshold, offset). Raises on any failure (ffmpeg missing, no JSON)."""
    r = subprocess.run(['ffmpeg', '-hide_banner', '-i', str(path), '-af',
                        'loudnorm=I=-14:TP=-1:LRA=11:print_format=json', '-f', 'null', '-'],
                       capture_output=True, text=True)
    text = (r.stderr or '') + (r.stdout or '')
    m = re.search(r'\{\s*"input_i".*?\}', text, re.S)
    if not m: raise ValueError('loudnorm JSON not found in ffmpeg output')
    vals = json.loads(m.group(0))
    return {'integrated_lufs': float(vals['input_i']), 'true_peak_dbtp': float(vals['input_tp']),
            'lra': float(vals['input_lra']), 'threshold': float(vals['input_thresh']),
            'offset': float(vals['target_offset'])}


def gate_verdict(d):
    """(ok, reasons) for a measure_loudness dict, per the gate above.

    Comparisons are made at the pipeline's printed precision (0.1 dB): ffmpeg
    loudnorm's true-peak readout wobbles by ~0.01–0.1 dB between its limiter
    pass and a re-measure (the v4 golden mix's limiter reports output_tp
    -1.00 while the analysis pass reads input_tp -0.99), so a gate stricter
    than the readout would fail mixes the mastering stage itself passed."""
    reasons = []
    i, tp = round(d['integrated_lufs'], 1), round(d['true_peak_dbtp'], 1)
    if not np.isfinite(i): reasons.append('integrated loudness not measurable')
    elif not (GATE_I[0] <= i <= GATE_I[1]): reasons.append(f'integrated {i:.1f} LUFS outside -14 ± 0.5')
    if not np.isfinite(tp): reasons.append('true peak not measurable')
    elif tp > GATE_TP: reasons.append(f'TP {tp:.1f} dBTP > -1.0')
    return (not reasons), reasons


def tail_stats(path):
    """(tail_rms, median_window_rms, ratio): final 0.5 s RMS vs the median of
    the non-overlapping 1 s window RMS values, on the mono sum."""
    from scipy.io import wavfile
    sr, data = wavfile.read(str(path))
    x = data.astype(np.float64)
    if data.dtype == np.int16: x /= 32768.0
    elif data.dtype == np.int32: x /= 2147483648.0
    if x.ndim > 1: x = x.mean(1)
    win = int(sr); nw = len(x) // win
    if nw == 0: return 0.0, 0.0, 0.0
    med = float(np.median([np.sqrt(np.mean(x[k * win:(k + 1) * win] ** 2)) for k in range(nw)]))
    tail = float(np.sqrt(np.mean(x[-int(0.5 * sr):] ** 2)))
    return tail, med, (tail / med if med > 0 else 0.0)


def main(argv):
    as_json = '--json' in argv
    args = [a for a in argv[1:] if not a.startswith('--')]
    P = Path(args[0] if args else '.')
    f = P / 'data/mix.wav'
    if not f.exists():
        print(f'measure.py: no data/mix.wav under {P} — run tools/mix.py first')
        return 2
    d = None
    try:
        d = measure_loudness(f)
        ok, reasons = gate_verdict(d)
    except Exception as e:
        ok, reasons = False, [f'not measurable: {type(e).__name__}: {e}']
    tail, med, ratio = tail_stats(f)
    if as_json:
        print(json.dumps({'integrated_lufs': d['integrated_lufs'] if d else None,
                          'true_peak_dbtp': d['true_peak_dbtp'] if d else None,
                          'lra': d['lra'] if d else None,
                          'tail_ratio': ratio, 'pass': ok, 'reasons': reasons}))
    else:
        if d:
            print(f"integrated: {d['integrated_lufs']:.1f} LUFS · true peak: {d['true_peak_dbtp']:.1f} dBTP · LRA: {d['lra']:.1f} LU")
        print(f'tail: final 0.5 s RMS {tail:.4f} vs median 1 s RMS {med:.4f} → ratio {ratio:.2f} '
              f'(loop metric; mix-level approximation — assumes voice outro silence)')
        if ok:
            print(f"loudness: PASS ({d['integrated_lufs']:.1f} LUFS, TP {d['true_peak_dbtp']:.1f} dBTP)")
        else:
            print(f'loudness: FAIL ({" | ".join(reasons)})')
    return 0 if ok else 3


if __name__ == '__main__':
    sys.exit(main(sys.argv))
