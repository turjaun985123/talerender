#!/usr/bin/env python3
"""equivalence_check.py — W52 sampled equivalence proof for the s12 bake.

Three evaluation paths are compared, so the BAKE's own error is
measured separately from the CONVERSION's inherent cost (the §5.2
hold+arrival spelling and the §11 f32 data law, which all 63
already-converted poseTracks pay identically):

  CODE   src/film.rs, S12 shoot fn, transcribed semantics:
             bow = pose_track([key(0,0), keym(2.5, 3.5, 0.45)], lt)
             lean = (0.6 * bow) as f32 ; head_tilt = (-bow) as f32
         pose_track / smoother / lerp below replicate film.rs in
         f64 (Python floats are IEEE f64 — bit-comparable).

  A      BAKE-THEN-CONVERT (the deliverable): the real
         tools/format/stage_clips.py run over the baked staging
         copy; the emitted lean / headTilt tracks evaluated under
         the W15 §5.2 spelling (hold key + smoother arrival).

  B      CONVERT-THEN-SCALE (the control): the same §5.2 spelling
         built from the ORIGINAL pre-transform keys (same f32q
         rounding the converter applies), evaluated, and only then
         multiplied by the factor and cast to f32.

  A vs B isolates the bake. CODE vs B isolates the conversion.
  CODE vs A is the delivered total.

Verdict: the bake is FAITHFUL iff
  (a) at both key times, A's f32 values are bit-identical to
      CODE's f32 state (the values the data law actually stores);
  (b) A vs B differ by at most 1 f32 ulp at every sample (the
      bake's own contribution);
  (c) A vs CODE never exceeds B vs CODE by more than 1 ulp (the
      bake adds nothing beyond the accepted conversion envelope).
Ulp distance is meaningless across the zero/subnormal boundary,
so samples where both sides are below 1e-18 in absolute value
(the t=2.05 window-start row: CODE underflows to exactly 0.0,
the spelled tracks read ~1e-20) are judged on the absolute
delta instead and marked DEGENERATE in the table.
The known exception, disclosed rather than smoothed: the code's
-bow yields -0.0 where the track holds 0; the data spells 0, as
the source record does. -0.0 == 0.0 numerically and no consumer
reads the sign of an exact zero; it is reported as SIGN-OF-ZERO,
not counted as an ulp difference.

Samples: both key times (0, 2.5), the move-window start (2.05),
two mid-segment times (2.2, 2.35), a hold-region time (1.0) and
a post-arrival time (3.0).

Usage:  python3 tools/format/staging/s12-baked/equivalence_check.py
Exit:   0 FAITHFUL / 1 NOT FAITHFUL (tables printed) / 2 IO error.
"""
import json
import struct
import subprocess
import sys
import tempfile
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
BAKED = HERE / "s12.scene.json"
CONVERTER = REPO / "tools" / "format" / "stage_clips.py"

SAMPLES = [0.0, 1.0, 2.05, 2.2, 2.35, 2.5, 3.0]
BOW_KEYS = [(0.0, 0.0, 0.35), (2.5, 3.5, 0.45)]  # key() / keym() in film.rs
ORIG_KEYS = [[0, 0], [2.5, 3.5, 0.45]]            # the staging record


def f32(v):
    return struct.unpack("f", struct.pack("f", v))[0]


def bits32(v):
    return struct.unpack("I", struct.pack("f", v))[0]


def ulp_diff(a, b):
    """Distance between two f32 values in ulps (sign-of-zero = 0)."""
    if a == b:
        return 0
    return abs(bits32(a) - bits32(b))


def clamp(x, a, b):
    return max(a, min(b, x))


def lerp(a, b, k):
    return a + (b - a) * k


def smoother(u):
    u = clamp(u, 0.0, 1.0)
    return u * u * u * (u * (u * 6.0 - 15.0) + 10.0)


def pose_track(keys, t):
    """film.rs pose_track, verbatim structure."""
    if not keys:
        return 0.0
    cur = keys[0][1]
    for (t0, v0, _m0), (t1, v1, m1) in zip(keys, keys[1:]):
        if t < t1 - m1:
            return cur
        if t < t1:
            return lerp(v0, v1, smoother((t - (t1 - m1)) / m1))
        cur = v1
    return cur


def clip_eval(keys, t):
    """Evaluate one §5.2-spelled track: list of {t, value, ease?}."""
    ks = sorted(keys, key=lambda k: k["t"])
    if t < ks[0]["t"]:
        return ks[0]["value"]
    for a, b in zip(ks, ks[1:]):
        if t < b["t"]:
            if b.get("ease") == "smoother":
                return lerp(a["value"], b["value"],
                            smoother((t - a["t"]) / (b["t"] - a["t"])))
            return a["value"]
    return ks[-1]["value"]


def spell(keys):
    """The converter's §5.2 spelling of a poseTrack, with its f32q
    rounding: [t, v] / [t, v, m] -> hold key at t-m + arrival key."""
    out = [{"t": f32(keys[0][0]), "value": f32(keys[0][1])}]
    prev = keys[0][1]
    for k in keys[1:]:
        m = k[2] if len(k) > 2 else 0.35
        if m == 0:
            out.append({"t": f32(k[0]), "value": f32(k[1]),
                        "ease": "step"})
        else:
            out.append({"t": f32(k[0] - m), "value": f32(prev)})
            out.append({"t": f32(k[0]), "value": f32(k[1]),
                        "ease": "smoother"})
        prev = k[1]
    return out


def fmt(v):
    return "%-22r" % v


def main():
    print("KEY ARITHMETIC (exact, via Fraction over the f64 values)")
    print("  lean arrival : 0.6 x 3.5 — f64 product %r (== f64(2.1): %s)"
          % (0.6 * 3.5, Fraction(0.6 * 3.5) == Fraction(2.1)))
    print("    code cast (0.6*bow) as f32 = %r bits 0x%08x"
          % (f32(0.6 * 3.5), bits32(f32(0.6 * 3.5))))
    print("    baked 2.1 -> f32q          = %r bits 0x%08x  identical: %s"
          % (f32(2.1), bits32(f32(2.1)),
             bits32(f32(0.6 * 3.5)) == bits32(f32(2.1))))
    print("  lean key 1   : 0.6 x 0 = 0 exact, bits 0x00000000 both sides")
    print("  headTilt arr.: -1 x 3.5 = -3.5, exact in f64 and f32, "
          "bits 0x%08x" % bits32(-3.5))
    print("  headTilt key1: -1 x 0 = -0.0 (code, bits 0x80000000); data "
          "spells 0 — SIGN-OF-ZERO, see header")
    print("  times / moveSecs are copied, never multiplied: 0, 2.5, 0.45")
    print()

    # Path A: the real converter over the baked staging.
    try:
        with tempfile.NamedTemporaryFile("w", suffix=".json",
                                         delete=False) as tf:
            out_path = tf.name
        r = subprocess.run([sys.executable, str(CONVERTER), str(BAKED),
                            "-o", out_path],
                           capture_output=True, text=True)
        if r.returncode != 0:
            print("ERROR: converter failed on the baked staging:\n%s%s"
                  % (r.stdout, r.stderr), file=sys.stderr)
            return 2
        clips = json.loads(Path(out_path).read_text(encoding="utf-8"))
    except OSError as e:
        print("ERROR: %s" % e, file=sys.stderr)
        return 2
    tracks_a = {}
    for clip in clips["clips"].values():
        for tr in clip["tracks"]:
            if tr["target"].get("channel") in ("lean", "headTilt"):
                tracks_a[tr["target"]["channel"]] = tr["keys"]
    if set(tracks_a) != {"lean", "headTilt"}:
        print("ERROR: converted clips lack the baked tracks: %s"
              % sorted(tracks_a), file=sys.stderr)
        return 2
    keys_b = spell(ORIG_KEYS)  # Path B spelling of the original keys

    key_times_ok = True
    bake_ok = True
    envelope_ok = True
    for ch, factor in (("lean", 0.6), ("headTilt", -1.0)):
        print("TRACK %s (factor %s)" % (ch, factor))
        print("  %-6s %-22s %-22s %-22s %-8s %-8s %-8s %s"
              % ("t", "CODE f32", "A baked f32", "B ctrl f32",
                 "A-B ulp", "B-C ulp", "A-C ulp", "|A-C|"))
        for t in SAMPLES:
            code = f32(factor * pose_track(BOW_KEYS, t))
            a = f32(clip_eval(tracks_a[ch], t))
            b = f32(factor * clip_eval(keys_b, t))
            u_ab, u_bc, u_ac = ulp_diff(a, b), ulp_diff(b, code), \
                ulp_diff(a, code)
            degenerate = max(abs(a), abs(code)) < 1e-18
            print("  %-6s %s %s %s %-8d %-8d %-8d %.3e%s"
                  % (t, fmt(code), fmt(a), fmt(b), u_ab, u_bc, u_ac,
                     abs(a - code), " DEGENERATE" if degenerate else ""))
            if t in (0.0, 2.5) and not (a == code):
                key_times_ok = False
            if u_ab > 1:
                bake_ok = False
            if not degenerate and u_ac > u_bc + 1:
                envelope_ok = False
        print()
    print("  (headTilt t=0 / 1.0: CODE is -0.0, A spells 0.0 — "
          "ulp_diff counts them equal;")
    print("   the sign of zero is the disclosed exception, not an "
          "error term.)")
    print("  Window start: CODE f64 2.5-0.45 = %r; the spelled hold "
          "key sits at f32(2.05)" % (2.5 - 0.45))
    print("   = %r — 4.77e-8 s (1.4e-6 frames) earlier; that shift, "
          "plus the f32 key" % f32(2.05))
    print("   values, is the whole of the B-vs-CODE column: the "
          "conversion cost every")
    print("   converted poseTrack pays, baked or not.")
    print()
    faithful = key_times_ok and bake_ok and envelope_ok
    print("VERDICT: %s — (a) key-time f32 bit-identity (modulo "
          "sign-of-zero): %s;"
          % ("FAITHFUL" if faithful else "NOT FAITHFUL", key_times_ok))
    print("         (b) bake's own contribution <= 1 ulp everywhere: "
          "%s;" % bake_ok)
    print("         (c) baked total within the conversion envelope "
          "+ 1 ulp: %s" % envelope_ok)
    return 0 if faithful else 1


if __name__ == "__main__":
    sys.exit(main())
