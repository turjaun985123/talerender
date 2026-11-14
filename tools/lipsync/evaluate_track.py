#!/usr/bin/env python3
"""evaluate_track.py — W59 lipsync adoption evaluation (stdlib only).

Compares, per frame at the film's own frame grid (t = n / fps), the
two candidate drivers of a speaking character's `talk` channel:

  OSC  the free oscillator the film driver uses today, transcribed
       verbatim from src/film.rs:232-238 (HEAD aa92cee):

           pub(crate) fn talk(t: f64, on: bool) -> f64 {
               if on {
                   0.25 + 0.75 * (13.0 * t).sin().abs()
                        * (0.6 + 0.4 * (5.3 * t).sin())
               } else { 0.0 }
           }

       `t` is ABSOLUTE film time (the phase never resets per line
       or shot). The law itself carries no state — it is a pure
       function of (t, on) — but the `on` windows are staging code,
       so this tool evaluates BOTH regimes the code defines:

       REGIME A (narration binding, the adoption counterfactual):
       `on` := the frame lies inside a narration line's word span.
       This is the comparison adoption actually poses — "if a
       mouth followed the narration, oscillator or track?"

       REGIME B (the film as staged): `on` only at the driver's
       two real call sites, both on the Tani character, both
       diegetic speech windows derived from anchors:
         film.rs:1478 (s03): talk(t, lt < w_surr - st0 + 0.35)
           with st0 = 13.061 (shot table film.rs:1993ff:
           s03 starts at lines[2].start - 0.06), w_surr =
           anchors["surrender2"]  ->  on for t in [13.061, 15.37)
         film.rs:1947 (s11): talk(t, t > w_order - 0.1
           && t < w_order + 0.75), w_order = anchors["order10"]
           ->  on for t in (51.68, 52.53)
       Everywhere else the driver never assigns talk; FullState
       defaults it to 0 (toonfull.rs:168).

  TRK  the W23 viseme track (tools/lipsync/build_visemes.py output,
       a W15 clips `talk` track), evaluated with the W15 S4
       semantics (FORMAT-EXT-CLIPS-W15.md): outside the key range
       hold the end value; inside, lerp by the bracketing segment,
       eased by the ENDING key's ease — every key in this track is
       authored "linear", so plain lerp is exact, not an
       approximation of the semantics.

"Phoneme active" is a DATA fact from the track file's own
lipsync.segments table (phoneme segments only; silences are keyed
into the track but are not segments): active(t) := some segment
with start <= t < end. The segment table also gives the viseme.

Painter grounding (src/toonfull.rs:698-700, the branch both call
sites paint through — Tani's mood is Determined at both, not
Shock/Laugh): the open mouth is drawn iff talk > 0.05, with
width 14 + 10*talk and height 6 + 18*talk. Hence:

  painted-open(v)  := v > 0.05
  band-agree       := |osc - trk| <= 0.10   (mouth height differs
                      by <= 1.8 px, width by <= 1.0 px: paints alike)
  class-agree      := painted-open(osc) == painted-open(trk)
  differing frame  := not band-agree
  open-on-silence  := no phoneme active AND osc painted-open
                      (the damning class: mouth moving with no
                      phoneme). Reported for BOTH regimes, and the
                      same test is applied to the track itself
                      ("track leak": W15-linear attack ramps cross
                      silences — see LIPSYNC-EVAL-W59.md).

Precision: Python floats are IEEE f64, matching the Rust law's
arithmetic; math.sin is the platform libm, as is Rust f64::sin on
Linux — any cross-libm residue is ~1 ulp (1e-16), five orders
below the 0.05/0.10 thresholds. The one un-reproduced step is the
`as f32` cast at the call sites (error <= 2^-24 relative); frames
whose either signal sits within 0.001 of the 0.05 painter
threshold are counted and reported as boundary-sensitive instead
of being silently classified. --self-check validates the
oscillator transcription against a hand-computed frame whose
arithmetic is shown in LIPSYNC-EVAL-W59.md S2 (40-digit Decimal
Taylor cross-check: talk(0.5, on) = 0.37726695596532633) and the
track evaluator against a hand-computed bracket lerp
(track(5.85) = 0.24229074889867777).

Usage:
    python3 tools/lipsync/evaluate_track.py [--words PATH]
        [--track PATH] [--film-data PATH] [--fps N]
    python3 tools/lipsync/evaluate_track.py --self-check

Exit codes (repo tools convention): 0 ok, 1 self-check FAIL,
2 input error (one-line `ERROR:` on stderr). The comparison itself
has NO pass/fail bar — no threshold spec exists to gate against;
the numbers are the evidence (see the note).
"""

import argparse
import bisect
import json
import math
import os
import sys

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DEFAULT_WORDS = os.path.expanduser(
    "~/faceless-shorts/the-soldier-who-didnt-know/data/words.json")
DEFAULT_TRACK = os.path.join(
    REPO, "tools", "lipsync", "fixtures", "onoda-lipsync.demo.scene.json")
DEFAULT_FILM = os.path.join(REPO, "film", "onoda.json")

PAINT_THRESHOLD = 0.05   # toonfull.rs:698  `else if t > 0.05`
BAND = 0.10              # |osc-trk| tolerance: <=1.8px height, <=1.0px width
BOUNDARY_EPS = 0.001     # threshold-boundary sensitivity window


def osc(t, on):
    """film.rs:232-238, transcribed. t is absolute film time."""
    if on:
        return 0.25 + 0.75 * abs(math.sin(13.0 * t)) * (0.6 + 0.4 * math.sin(5.3 * t))
    return 0.0


class Track:
    """W15 S4 evaluation of the demo file's single talk track."""

    def __init__(self, doc):
        clip = doc["clips"]["lipsync-onoda"]
        keys = clip["tracks"][0]["keys"]
        self.ts = [k["t"] for k in keys]
        self.vs = [k["value"] for k in keys]
        self.eases = {k.get("ease", "smoother") for k in keys}
        self.segments = doc["lipsync"]["segments"]
        self.seg_starts = [s["start"] for s in self.segments]

    def value(self, t):
        ts, vs = self.ts, self.vs
        if t <= ts[0]:
            return vs[0]
        if t >= ts[-1]:
            return vs[-1]
        i = bisect.bisect_right(ts, t) - 1
        t0, t1, v0, v1 = ts[i], ts[i + 1], vs[i], vs[i + 1]
        if t1 == t0:
            return v1
        return v0 + (v1 - v0) * ((t - t0) / (t1 - t0))

    def segment(self, t):
        i = bisect.bisect_right(self.seg_starts, t) - 1
        if i >= 0:
            s = self.segments[i]
            if s["start"] <= t < s["end"]:
                return s
        return None


def painted_open(v):
    return v > PAINT_THRESHOLD


def line_spans(words):
    """words.json token list -> {line: (first word s, last word e)}."""
    spans = {}
    for tok in words:
        ln = tok["line"]
        if ln in spans:
            s0, e0 = spans[ln]
            spans[ln] = (min(s0, tok["s"]), max(e0, tok["e"]))
        else:
            spans[ln] = (tok["s"], tok["e"])
    return spans


class Stats:
    def __init__(self):
        self.n = 0
        self.band = 0
        self.cls = 0
        self.sum_abs = 0.0
        self.osc_open_silence = 0   # no phoneme, osc painted-open
        self.trk_open_silence = 0   # no phoneme, track painted-open
        self.missed_closure = 0     # phoneme active, trk <= thr, osc open
        self.boundary = 0           # either signal within eps of threshold
        self.hist = [0] * 5         # |d| bins: .05 .10 .25 .50 1.0+
        self.osc_hotter = 0
        self.trk_hotter = 0

    def add(self, o, tr, active):
        self.n += 1
        d = abs(o - tr)
        self.sum_abs += d
        if d <= BAND:
            self.band += 1
        if painted_open(o) == painted_open(tr):
            self.cls += 1
        if not active:
            if painted_open(o):
                self.osc_open_silence += 1
            if painted_open(tr):
                self.trk_open_silence += 1
        else:
            if tr <= PAINT_THRESHOLD and painted_open(o):
                self.missed_closure += 1
        if abs(o - PAINT_THRESHOLD) < BOUNDARY_EPS or abs(tr - PAINT_THRESHOLD) < BOUNDARY_EPS:
            self.boundary += 1
        for i, edge in enumerate((0.05, 0.10, 0.25, 0.50)):
            if d <= edge:
                self.hist[i] += 1
                break
        else:
            self.hist[4] += 1
        if o > tr:
            self.osc_hotter += 1
        elif tr > o:
            self.trk_hotter += 1

    def line(self, label):
        n = self.n or 1
        return (
            f"{label}: frames {self.n} · band-agree {100.0 * self.band / n:.1f}% "
            f"({self.band}) · class-agree {100.0 * self.cls / n:.1f}% ({self.cls}) "
            f"· differing {self.n - self.band} · mean|d| {self.sum_abs / n:.4f} "
            f"· osc-open-on-silence {self.osc_open_silence} "
            f"· trk-open-on-silence {self.trk_open_silence} "
            f"· missed-closures {self.missed_closure}"
        )


def self_check(track):
    ok = True
    v = osc(0.5, True)
    want = 0.37726695596532633  # hand chain, LIPSYNC-EVAL-W59.md S2
    good = abs(v - want) < 1e-9
    ok &= good
    print(f"[self-check] osc(0.5, on) = {v:.14f} vs hand {want} -> {'OK' if good else 'FAIL'}")
    v2 = track.value(5.85)
    want2 = 0.24229074889867777  # keys (5.74,0.0)->(5.967,0.5), u=0.484581...
    good2 = abs(v2 - want2) < 1e-6
    ok &= good2
    print(f"[self-check] track(5.85) = {v2:.14f} vs hand {want2} -> {'OK' if good2 else 'FAIL'}")
    good3 = osc(0.5, False) == 0.0 and track.value(-1.0) == track.vs[0] \
        and track.value(1e9) == track.vs[-1]
    ok &= good3
    print(f"[self-check] off-state + hold-outside-range -> {'OK' if good3 else 'FAIL'}")
    seg = track.segment(0.31)
    good4 = seg is not None and seg["viseme"] == "th" and seg["word"] == "This"
    ok &= good4
    print(f"[self-check] segment(0.31) = {seg and seg['viseme']}/{seg and seg['word']} -> {'OK' if good4 else 'FAIL'}")
    print(f"SELF-CHECK: {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


def main(argv=None):
    ap = argparse.ArgumentParser(description="W59 lipsync adoption evaluation (see module docstring)")
    ap.add_argument("--words", default=DEFAULT_WORDS)
    ap.add_argument("--track", default=DEFAULT_TRACK)
    ap.add_argument("--film-data", default=DEFAULT_FILM)
    ap.add_argument("--fps", type=int, default=None)
    ap.add_argument("--self-check", action="store_true")
    args = ap.parse_args(argv)

    try:
        with open(args.words) as f:
            words = json.load(f)
        with open(args.track) as f:
            track_doc = json.load(f)
        with open(args.film_data) as f:
            film = json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        print(f"ERROR: cannot read inputs: {e}", file=sys.stderr)
        return 2

    track = Track(track_doc)
    if args.self_check:
        return self_check(track)

    fps = args.fps or film.get("fps", 30)
    n_frames = film.get("frames")
    duration = film.get("duration")
    if not n_frames or not duration:
        print("ERROR: film data lacks frames/duration", file=sys.stderr)
        return 2
    spans = line_spans(words)
    anchors = film["anchors"]
    flines = film["lines"]

    # --- REGIME B windows, derived from the source expressions ---
    s03_start = flines[2]["start"] - 0.06          # shot table: l(2)
    s04_start = flines[3]["start"] - 0.06          # l(3): s03 is gone after this
    w1 = (s03_start, anchors["surrender2"] + 0.35)  # film.rs:1478
    w2 = (anchors["order10"] - 0.1, anchors["order10"] + 0.75)  # film.rs:1947
    assert abs(s03_start - 13.061) < 1e-9, s03_start
    assert w1[1] < s04_start, "s03 window must end inside its shot"

    def osc_b(t):
        on = (w1[0] <= t < w1[1]) or (w2[0] < t < w2[1])
        return osc(t, on)

    print(f"[1 inputs] words={args.words} ({len(words)} tokens, {len(spans)} lines) · "
          f"track keys={len(track.ts)} eases={sorted(track.eases)} segments={len(track.segments)} · "
          f"film frames={n_frames} fps={fps} duration={duration}")
    print(f"    regime-B windows from source: W1 s03 [{w1[0]:.3f}, {w1[1]:.3f}) · "
          f"W2 s11 ({w2[0]:.3f}, {w2[1]:.3f}) · painter threshold {PAINT_THRESHOLD} · band {BAND}")

    # --- per-frame evaluation -------------------------------------
    per_line_a = {ln: Stats() for ln in spans}
    total_a = Stats()
    per_viseme = {}
    regime_b = Stats()
    window_stats = {"W1": Stats(), "W2": Stats()}
    line_b = {2: Stats(), 10: Stats()}
    track_leak_outside_lines = 0
    frames_outside_lines = 0
    osc_b_open_frames = 0

    for n in range(n_frames):
        t = n / fps
        tr = track.value(t)
        seg = track.segment(t)
        active = seg is not None
        vis = seg["viseme"] if active else "sil"
        ln = None
        for k, (s0, e0) in spans.items():
            if s0 <= t < e0:
                ln = k
                break
        # REGIME A: oscillator bound to the narration
        if ln is not None:
            o = osc(t, True)
            per_line_a[ln].add(o, tr, active)
            total_a.add(o, tr, active)
            st = per_viseme.setdefault(vis, Stats())
            st.add(o, tr, active)
        else:
            frames_outside_lines += 1
            if not active and painted_open(tr):
                track_leak_outside_lines += 1
        # REGIME B: the film as staged
        ob = osc_b(t)
        if painted_open(ob):
            osc_b_open_frames += 1
        regime_b.add(ob, tr, active)
        if w1[0] <= t < w1[1]:
            window_stats["W1"].add(ob, tr, active)
        if w2[0] < t < w2[1]:
            window_stats["W2"].add(ob, tr, active)
        if ln in line_b:
            line_b[ln].add(ob, tr, active)

    print("\n[2 regime A — oscillator bound to narration (on = inside line span), per line]")
    densest, densest_share = None, -1.0
    for ln in sorted(spans):
        st = per_line_a[ln]
        print("   ", st.line(f"line {ln:2d} [{spans[ln][0]:.3f}, {spans[ln][1]:.3f})"))
        share = (st.n - st.band) / (st.n or 1)
        if share > densest_share:
            densest, densest_share = ln, share
    print("   ", total_a.line("ALL LINES"))
    print(f"    track leak outside lines: {track_leak_outside_lines} of {frames_outside_lines} "
          f"out-of-line frames painted-open with no phoneme active")

    print("\n[3 regime A — difference distribution over in-line frames]")
    h = total_a.hist
    print(f"    |d| bins: <=0.05 {h[0]} · (0.05,0.10] {h[1]} · (0.10,0.25] {h[2]} · "
          f"(0.25,0.50] {h[3]} · >0.50 {h[4]} · osc hotter {total_a.osc_hotter} · "
          f"track hotter {total_a.trk_hotter} · boundary-sensitive {total_a.boundary}")
    print("    by active viseme (osc bound to narration):")
    for vis in sorted(per_viseme):
        st = per_viseme[vis]
        n = st.n or 1
        print(f"      {vis:>3}: frames {st.n:4d} · band-agree {100.0 * st.band / n:5.1f}% · "
              f"mean|d| {st.sum_abs / n:.4f} · missed-closures {st.missed_closure} · "
              f"osc-open-on-silence {st.osc_open_silence}")

    print("\n[4 regime B — the film as staged (oscillator only in its two diegetic windows)]")
    print("   ", window_stats["W1"].line(f"W1 s03 Tani explains [{w1[0]:.3f}, {w1[1]:.3f})"))
    print("   ", window_stats["W2"].line(f"W2 s11 Tani orders ({w2[0]:.3f}, {w2[1]:.3f})"))
    print(f"    oscillator painted-open frames film-wide: {osc_b_open_frames} of {n_frames}; "
          f"outside the windows the driver never assigns talk (state default 0)")
    print("   ", line_b[2].line("line  2 under regime B (window W1 covers its start)"))
    print("   ", line_b[10].line("line 10 under regime B (window W2 sits inside it)"))
    print("   ", regime_b.line("FILM-WIDE (regime B osc vs narrator track — different speakers; "
                              "reported for completeness, NOT an adoption metric)"))

    print(f"\n[5 densest-difference line for the A/B sheet gate] line {densest} "
          f"({100.0 * densest_share:.1f}% of its frames differ under regime A)")
    print("EVALUATION COMPLETE — no pass/fail bar exists for this comparison; "
          "the numbers above are the evidence (LIPSYNC-EVAL-W59.md).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
