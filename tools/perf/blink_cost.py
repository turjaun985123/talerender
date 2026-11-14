#!/usr/bin/env python3
"""blink_cost.py — W33 blink-adoption cost evidence (analytic; no renders).

Quantifies the paint-cache cost of replacing film.rs's binary
`Blinker` with W12's charanim blink channels — SMOOTH
(`blink_amount`) and QUANTIZED (`blink_amount_q`, steps = 8) —
over the real Onoda film staging, exactly as staged today.

Why blink costs paints at all: `state_key` (src/film.rs:2152)
keys the paint cache on the EXACT f32 bit pattern of
`FullState.blink` (post-cabcc30 exact-key law). Every distinct
blink value a character's state takes, paired with an otherwise
repeating state, is one extra painted card (one full
2560x2816 RGBA canvas, CANVAS_BYTES = 28,835,840 B,
src/paintbudget.rs:35). The cache is consulted for every
staged character on every frame (resolve_chars,
src/film.rs:2180+), so presence below means "staged", which
is exactly the population that can cost paints
(resolve_chars is src/film.rs:2182).

Method: frame-accurate simulation. For every frame n of the
film (t = n/fps, the manifest frameRule), for every character
the shot functions in film.rs actually stage in that segment,
evaluate all blink models at t and record the f32 bit pattern
that would enter state_key. Counts are then exact for the
blink component: events (maximal runs of blink > 0 over a
character's presence), frames with blink > 0, distinct nonzero
blink values, and distinct values per event.

Extra-paints estimate (the only modelled step, stated in the
note): a blink event forces (D - 1) extra paints beyond the
binary baseline's 1, where D = distinct blink values sampled
during the event, IF the character's other state components
repeat across the event (held pose / standing listener — the
case the cache exists for). If the rest of the state is unique
every frame anyway (walk, talk, eased tracks, moving camera),
the marginal cost is 0. The reported total is therefore the
r = 1 CEILING; a camera-hold exposure statistic indicates how
much of the blink time even occurs while the camera is static
enough for repetition to be plausible.

Models ported bit-faithfully (f64 arithmetic, same op order;
f32 only where the Rust code casts):
  * splitmix64 / seeded_unit / blink_start / smootherstep /
    blink_amount / blink_amount_q — src/charanim.rs:60-216.
  * Blinker::at — src/film.rs:179-202 (interval table
    [2.7, 3.9, 3.1, 4.4, 2.9, 3.6] rotated by floor(seed*7)%6,
    clock offset seed*1.37, shut 0.12 s).
  * blink_at beats — src/film.rs:205-210 via blink_of
    (src/film.rs:1391-1393): today's value for beat characters
    is max(Blinker, blink_at), still binary.
  * "+beats" variants: max(channel, blink_at) — what adoption
    costs if staging keeps the seven authored word-beat blinks.

Data (read, never written):
  * scenes/film/film.json — segment ids, frame ranges, times.
  * film/onoda.json — fps, duration, frames, word anchors.
  * PRESENCE below — transcribed from the shot functions in
    src/film.rs (line citations in the comment); this is the
    one input that lives in code, not data, because blink
    seeds exist only there. Cross-checked by hand: every
    char() call in s01..s12 has exactly one blink assignment,
    and every blink assignment belongs to a staged character.

Usage: python3 tools/perf/blink_cost.py [--root DIR] [--json PATH]
Exit: 0 on success (self-checks printed), 2 on data mismatch.
"""

import json
import math
import struct
import sys
from pathlib import Path

# ---------------------------------------------------------------------
# f32 helpers (Rust `as f32` cast / f32 arithmetic, emulated exactly)
# ---------------------------------------------------------------------

def f32(x: float) -> float:
    return struct.unpack("<f", struct.pack("<f", x))[0]


def f32_bits(x: float) -> int:
    return struct.unpack("<I", struct.pack("<f", x))[0]


def round_half_away(x: float) -> float:
    # Rust f32::round: half away from zero. Inputs here are >= 0.
    return math.floor(x + 0.5) if x >= 0.0 else -math.floor(-x + 0.5)


# ---------------------------------------------------------------------
# charanim.rs ports (src/charanim.rs:60-216)
# ---------------------------------------------------------------------

M64 = (1 << 64) - 1
SALT_BLINK_FIRST = 0xB111_0001
SALT_BLINK_GAP = 0xB111_0002
BLINK_DOWN_S = 0.12
BLINK_UP_S = 0.12
BLINK_EVENT_S = BLINK_DOWN_S + BLINK_UP_S  # f64: 0.12 + 0.12, as in Rust


def splitmix64(x: int) -> int:
    x = (x + 0x9E37_79B9_7F4A_7C15) & M64
    x = ((x ^ (x >> 30)) * 0xBF58_476D_1CE4_E5B9) & M64
    x = ((x ^ (x >> 27)) * 0x94D0_49BB_1331_11EB) & M64
    return x ^ (x >> 31)


def seeded_unit(i: int, index: int, salt: int) -> float:
    inner = splitmix64(((index * 0xD1B5_4A32_D192_ED03) & M64) ^ salt)
    mixed = splitmix64(i ^ inner)
    return (mixed >> 11) * (1.0 / 9_007_199_254_740_992.0)


def smootherstep(u: float) -> float:
    u = 0.0 if u < 0.0 else (1.0 if u > 1.0 else u)
    return u * u * u * (u * (u * 6.0 - 15.0) + 10.0)


def blink_start(i: int, n: int) -> float:
    s = 0.6 + 1.4 * seeded_unit(i, 0, SALT_BLINK_FIRST)
    k = 0
    while k < n:
        s += 2.6 + 1.8 * seeded_unit(i, k, SALT_BLINK_GAP)
        k += 1
    return s


def blink_amount_f64(i: int, t: float) -> float:
    """charanim::blink_amount before its `as f32` cast."""
    if not (t >= 0.0):
        return 0.0
    n = 0
    while True:
        s = blink_start(i, n)
        if t < s:
            return 0.0
        if t <= s + BLINK_EVENT_S:
            p = (t - s) / BLINK_EVENT_S
            if p < 0.5:
                return smootherstep(p * 2.0)
            return smootherstep((1.0 - p) * 2.0)
        n += 1
        if n > 1_000_000:
            return 0.0


def blink_amount(i: int, t: float) -> float:
    return f32(blink_amount_f64(i, t))


def blink_amount_q(i: int, t: float, steps: int = 8) -> float:
    """charanim::blink_amount_q: f32 arithmetic throughout."""
    s = float(steps)
    v = blink_amount(i, t)
    q = f32(round_half_away(f32(v * s)) / s)
    return 0.0 if q < 0.0 else (1.0 if q > 1.0 else q)


# ---------------------------------------------------------------------
# film.rs ports (src/film.rs:179-210, 1391-1393)
# ---------------------------------------------------------------------

BLINKER_IV = [2.7, 3.9, 3.1, 4.4, 2.9, 3.6]


def blinker_at(seed: float, t: float) -> float:
    x = t + seed * 1.37
    if not (x >= 0.0):
        return 0.0
    start = ((math.floor(seed * 7.0) % 6) + 6) % 6
    acc = 0.0
    i = start
    while x > acc + BLINKER_IV[i % 6]:
        acc += BLINKER_IV[i % 6]
        i += 1
    return 1.0 if x - acc < 0.12 else 0.0


def blink_at(t: float, beats) -> float:
    return 1.0 if any(0.0 <= t - b < 0.12 for b in beats) else 0.0


# ---------------------------------------------------------------------
# Presence: (display name, blink seed == charanim id, beat anchor|None)
# Transcribed from src/film.rs shot functions; line = the blink
# assignment. Every staged character receives exactly one.
#   s01 :1407 onoda blink_of(3, years0)
#   s02 :1439 onoda(3)  :1445 compA(7)  :1451 compB(11)
#   s03 :1470 taniguchi(9)  :1476 onoda blink_of(3, never2)
#         :1481 compA(7)  :1486 compB(11)
#   s04 :1500 compA(7)  :1506 compB(11)  :1513 onoda blink_of(3, message3)
#   s05a: no char() calls at all (plane + leaflets only) — nobody blinks
#   s05b:1594 onoda(3)
#   s06 :1634 compA(7)  :1637 compB(11)  :1653 onoda(3)
#   s07 :1714 onoda(3)
#   s08 :1746 suzuki blink_of(5, found7)  :1761 onoda(3)
#   s08b:1781 onoda(3)  :1786 suzuki(5)
#   s09 :1813 onoda blink_of(3, commander8)  :1821 suzuki(5)
#   s10 :1854 taniguchi blink_of(9, bookseller9)
#   s11 :1916/:1929 onoda(3) both branches  :1940 taniguchi(9)
#   s12 :1971 onoda blink_of(3, loyal11)  :1977 taniguchi(9)
# ---------------------------------------------------------------------

PRESENCE = {
    "s01": [("onoda", 3, "years0")],
    "s02": [("onoda", 3, None), ("compA", 7, None), ("compB", 11, None)],
    "s03": [("taniguchi", 9, None), ("onoda", 3, "never2"),
            ("compA", 7, None), ("compB", 11, None)],
    "s04": [("compA", 7, None), ("compB", 11, None), ("onoda", 3, "message3")],
    "s05a": [],
    "s05b": [("onoda", 3, None)],
    "s06": [("compA", 7, None), ("compB", 11, None), ("onoda", 3, None)],
    "s07": [("onoda", 3, None)],
    "s08": [("suzuki", 5, "found7"), ("onoda", 3, None)],
    "s08b": [("onoda", 3, None), ("suzuki", 5, None)],
    "s09": [("onoda", 3, "commander8"), ("suzuki", 5, None)],
    "s10": [("taniguchi", 9, "bookseller9")],
    "s11": [("onoda", 3, None), ("taniguchi", 9, None)],
    "s12": [("onoda", 3, "loyal11"), ("taniguchi", 9, None)],
}

# cam_k(lt, d, hold) hold argument per shot function (film.rs):
# the camera move is static for lt/d <= hold and >= 1 - hold.
CAM_HOLD = {
    "s01": 0.1, "s02": 0.03, "s03": 0.1, "s04": 0.1, "s05a": 0.1,
    "s05b": 0.1, "s06": 0.1, "s07": 0.1, "s08": 0.1, "s08b": 0.1,
    "s09": 0.1, "s10": 0.1, "s11": 0.1, "s12": 0.1,
}

CANVAS_BYTES = 2560 * 2816 * 4  # src/paintbudget.rs:35
MODELS = ["today", "smooth", "quant"]
VARIANTS = ["smooth+beats", "quant+beats"]


def model_value(model, cid, seed, t, beats):
    """The f32 blink value that would enter state_key at time t."""
    if model == "today":
        return f32(max(blinker_at(seed, t), blink_at(t, beats)))
    if model == "smooth":
        return blink_amount(cid, t)
    if model == "quant":
        return blink_amount_q(cid, t, 8)
    if model == "smooth+beats":
        return f32(max(blink_amount_f64(cid, t), blink_at(t, beats)))
    if model == "quant+beats":
        return f32(max(blink_amount_q(cid, t, 8), blink_at(t, beats)))
    raise ValueError(model)


def find_runs(samples):
    """samples: [(frame_no, seg_id, bits)] in frame order, present
    frames only. Returns runs of blink > 0 over consecutive frame
    numbers: [{frames: [(frame_no, seg_id, bits)], segs: set}]."""
    runs = []
    cur = None
    prev_frame = None
    for frame_no, seg_id, bits in samples:
        if bits != 0:
            if cur is not None and frame_no == prev_frame + 1:
                cur["frames"].append((frame_no, seg_id, bits))
                cur["segs"].add(seg_id)
            else:
                cur = {"frames": [(frame_no, seg_id, bits)], "segs": {seg_id}}
                runs.append(cur)
        else:
            cur = None
        prev_frame = frame_no
    return runs


def run_stats(runs, seg_filter=None):
    """(events, frames>0, sum(D-1), D list) over runs; if seg_filter,
    each run is clipped to that segment's frames first."""
    events = 0
    frames_on = 0
    extra = 0
    ds = []
    for r in runs:
        fr = r["frames"] if seg_filter is None else [f for f in r["frames"] if f[1] == seg_filter]
        if not fr:
            continue
        events += 1
        frames_on += len(fr)
        d = len({bits for _, _, bits in fr})
        ds.append(d)
        extra += d - 1
    return events, frames_on, extra, ds


def port_self_checks():
    out = []
    # charanim.rs test blink_cadence_is_human_plausible: rising
    # crossings of 0.5 for id 3 over 120 s (1 ms-ish sampling)
    # must land in [25, 48].
    events = 0
    prev = blink_amount(3, 0.0)
    t = 0.01
    while t < 120.0:
        b = blink_amount(3, t)
        if prev <= 0.5 < b:
            events += 1
        prev = b
        t += 0.01
    out.append(("charanim id=3 crossings of 0.5 in 120 s", events, 25 <= events <= 48))
    # Schedule shape: first start in [0.6, 2.0), gaps in [2.6, 4.4).
    ok = True
    for cid in (3, 5, 7, 9, 11):
        starts = [blink_start(cid, n) for n in range(12)]
        ok &= 0.6 <= starts[0] < 2.0
        ok &= all(2.6 <= b - a < 4.4 for a, b in zip(starts, starts[1:]))
    out.append(("charanim first starts in [0.6,2.0), gaps in [2.6,4.4)", "ok" if ok else "BAD", ok))
    # Blinker: mean interval is 20.6/6 = 3.4333 s -> ~35 events/120 s
    # for every seed (rotation only changes phase).
    ok = True
    counts = {}
    for seed in (3.0, 5.0, 7.0, 9.0, 11.0):
        c = 0
        prev = blinker_at(seed, 0.0)
        for n in range(1, 3600):
            v = blinker_at(seed, n / 30.0)
            if prev == 0.0 and v == 1.0:
                c += 1
            prev = v
        counts[int(seed)] = c
        ok &= 30 <= c <= 40
    out.append((f"blinker events in 120 s by seed {counts}", "ok" if ok else "BAD", ok))
    return out


def main():
    args = sys.argv[1:]
    root = Path(__file__).resolve().parents[2]
    json_path = None
    i = 0
    while i < len(args):
        if args[i] == "--root":
            root = Path(args[i + 1])
            i += 2
        elif args[i] == "--json":
            json_path = Path(args[i + 1])
            i += 2
        else:
            print(f"unknown argument: {args[i]}", file=sys.stderr)
            return 2

    manifest = json.loads((root / "scenes/film/film.json").read_text())
    onoda = json.loads((root / "film/onoda.json").read_text())
    fps = onoda["fps"]
    anchors = onoda["anchors"]
    segments = manifest["segments"]

    # ---- data integrity checks (fail loud, never guess) ----
    problems = []
    if [s["id"] for s in segments] != list(PRESENCE.keys()):
        problems.append("manifest segment ids != PRESENCE table ids/order")
    if sum(s["frames"] for s in segments) != onoda["frames"]:
        problems.append("manifest frame total != onoda.json frames")
    for seg in segments:
        for _, _, beat in PRESENCE[seg["id"]]:
            if beat is not None and not (seg["startS"] <= anchors[beat] < seg["endS"]):
                problems.append(f"beat {beat} outside segment {seg['id']}")
    if problems:
        for p in problems:
            print(f"DATA MISMATCH: {p}", file=sys.stderr)
        return 2

    print("PORT SELF-CHECKS (Python ports vs the Rust sources' own test bands)")
    for label, val, ok in port_self_checks():
        print(f"  [{'PASS' if ok else 'FAIL'}] {label}: {val}")
    print(f"  [PASS] data: 14 segments, {onoda['frames']} frames, fps={fps}, "
          f"all 7 beat anchors inside their segments")
    print()

    # ---- simulation ----
    # Per character (by seed/id): ordered samples per model.
    char_samples = {}   # cid -> model -> [(frame, seg, bits)]
    char_frames = {}    # cid -> present frames
    char_names = {}
    seg_char_frames = {}  # (seg, cid) -> frames
    hold_on = {m: 0 for m in MODELS}    # blink>0 frames in cam-hold windows
    on_frames = {m: 0 for m in MODELS}  # blink>0 frames, all windows
    for seg in segments:
        sid = seg["id"]
        dur = seg["durationS"]
        hold = CAM_HOLD[sid]
        for name, seed, beat in PRESENCE[sid]:
            cid = seed
            char_names[cid] = name
            beats = [anchors[beat]] if beat else []
            for n in range(seg["startFrame"], seg["endFrame"]):
                t = n / fps
                char_frames[cid] = char_frames.get(cid, 0) + 1
                seg_char_frames[(sid, cid)] = seg_char_frames.get((sid, cid), 0) + 1
                lt_frac = (t - seg["startS"]) / dur
                in_hold = lt_frac <= hold or lt_frac >= 1.0 - hold
                for m in MODELS + VARIANTS:
                    v = model_value(m, cid, float(seed), t, beats)
                    char_samples.setdefault(cid, {}).setdefault(m, []).append(
                        (n, sid, f32_bits(v)))
                    if m in MODELS and v > 0.0:
                        on_frames[m] += 1
                        if in_hold:
                            hold_on[m] += 1

    total_lookups = sum(char_frames.values())
    runs = {cid: {m: find_runs(char_samples[cid][m]) for m in MODELS + VARIANTS}
            for cid in char_samples}

    # ---- per segment x character table (main models) ----
    print("PER SEGMENT x CHARACTER  (events / frames blink>0 / distinct nonzero values / "
          "extra paints at r=1)")
    hdr = f"{'seg':<5} {'char':<10} {'fr':>4} | " + " | ".join(
        f"{m:>24}" for m in MODELS)
    print(hdr)
    print("-" * len(hdr))
    seg_rollup = {}
    report = {"segments": {}, "characters": {}, "film": {}}
    for seg in segments:
        sid = seg["id"]
        for name, seed, _ in PRESENCE[sid]:
            cid = seed
            row = f"{sid:<5} {name:<10} {seg_char_frames[(sid, cid)]:>4} | "
            cells = []
            for m in MODELS:
                ev, fon, extra, ds = run_stats(runs[cid][m], seg_filter=sid)
                distinct = len({b for (n, s, b) in char_samples[cid][m]
                                if s == sid and b != 0})
                cells.append(f"{ev:>3} ev {fon:>3} on {distinct:>3} dv {extra:>3} xp")
                r = seg_rollup.setdefault(sid, {}).setdefault(
                    m, {"events": 0, "frames_on": 0, "distinct": 0, "extra": 0})
                r["events"] += ev
                r["frames_on"] += fon
                r["distinct"] += distinct
                r["extra"] += extra
            print(row + " | ".join(cells))
        if not PRESENCE[sid]:
            print(f"{sid:<5} {'(nobody staged)':<10} {seg['frames']:>4} | —")
    print()

    print("PER SEGMENT ROLLUP  (extra paints at r=1, vs today's binary)")
    print(f"{'seg':<5} {'chars':>5} {'char-frames':>11} | " +
          " | ".join(f"{m:>22}" for m in MODELS))
    for seg in segments:
        sid = seg["id"]
        cf = sum(seg_char_frames.get((sid, c), 0) for c in char_names
                 if (sid, c) in seg_char_frames)
        cells = []
        for m in MODELS:
            r = seg_rollup.get(sid, {}).get(m)
            cells.append("—" if r is None else
                         f"{r['events']:>3} ev {r['frames_on']:>4} on {r['extra']:>3} xp")
        print(f"{sid:<5} {len(PRESENCE[sid]):>5} {cf:>11} | " + " | ".join(cells))
        report["segments"][sid] = seg_rollup.get(sid, {})
    print()

    # ---- per character + film totals (all five models) ----
    print("PER CHARACTER, FILM-WIDE  (events counted once per character, "
          "even across a cut)")
    print(f"{'char':<10} {'id':>3} {'screen s':>8} {'frames':>6} | " +
          " | ".join(f"{m:>22}" for m in MODELS + VARIANTS))
    film_tot = {m: {"events": 0, "frames_on": 0, "extra": 0, "distinct": 0,
                    "d_sum": 0, "d_n": 0, "d_max": 0} for m in MODELS + VARIANTS}
    for cid in sorted(char_samples):
        cells = []
        for m in MODELS + VARIANTS:
            ev, fon, extra, ds = run_stats(runs[cid][m])
            distinct = len({b for (_, _, b) in char_samples[cid][m] if b != 0})
            cells.append(f"{ev:>3} ev {fon:>4} on {distinct:>4} dv {extra:>3} xp")
            ft = film_tot[m]
            ft["events"] += ev
            ft["frames_on"] += fon
            ft["extra"] += extra
            ft["distinct"] += distinct
            ft["d_sum"] += sum(ds)
            ft["d_n"] += len(ds)
            ft["d_max"] = max(ft["d_max"], max(ds) if ds else 0)
            report["characters"].setdefault(char_names[cid], {})[m] = {
                "events": ev, "frames_on": fon, "extra_paints_r1": extra,
                "distinct_nonzero_values": distinct}
        print(f"{char_names[cid]:<10} {cid:>3} {char_frames[cid] / fps:>8.2f} "
              f"{char_frames[cid]:>6} | " + " | ".join(cells))
    print()

    print("FILM TOTALS")
    print(f"  cache lookups (character-frames)      : {total_lookups}")
    for m in MODELS + VARIANTS:
        ft = film_tot[m]
        mean_d = ft["d_sum"] / ft["d_n"] if ft["d_n"] else 0.0
        print(f"  {m:<13}: {ft['events']:>3} events, {ft['frames_on']:>4} frames blink>0, "
              f"D mean {mean_d:.2f} max {ft['d_max']}, "
              f"extra paints at r=1 = {ft['extra']:>4} "
              f"({100.0 * ft['extra'] / total_lookups:.2f}% of lookups, "
              f"{ft['extra'] * CANVAS_BYTES / 2**30:.1f} GiB of canvas paints)")
    report["film"] = {m: film_tot[m] for m in MODELS + VARIANTS}
    report["film"]["total_lookups"] = total_lookups
    print()
    print("CAMERA-HOLD EXPOSURE (share of blink>0 frames falling where cam_k "
          "is at an endpoint,")
    print("  i.e. the camera is static. Outside these windows a moving camera "
          "keeps most states")
    print("  unique frame-to-frame, so events there pay ~nothing; inside them "
          "the character must")
    print("  ALSO be self-frozen to pay the full r=1 price — read this as "
          "directional evidence")
    print("  that the real extra sits well below the ceiling, not as a bound)")
    for m in MODELS:
        print(f"  {m:<8}: {hold_on[m]:>4} of {on_frames[m]:>4} blink>0 frames "
              f"({100.0 * hold_on[m] / max(1, on_frames[m]):.1f}%)")

    if json_path:
        json_path.write_text(json.dumps(report, indent=1))
        print(f"\nwrote {json_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
