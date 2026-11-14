#!/usr/bin/env python3
"""blink_census.py — W58 blink-adoption census: the exact frames whose
eye state changes if film.rs adopts charanim's smooth blink with the
authored beats maxed back in.

Companion to tools/perf/blink_cost.py (W33). W33 priced the adoption
(extra paints, r = 1 ceiling); this tool enumerates WHICH frames
change, so the adoption's A/B gate can assert byte-identity exactly
where identity is promised and expect difference exactly where it is
not. Analytic — no renders, stdlib only.

The two models (evaluated per staged character per frame, t = n/fps):

  CURRENT  today's shipped blink: f32(max(Blinker::at(seed, t),
           blink_at(t, beats))) — binary {0, 1}; the only blink the
           film has ever rendered. (--against quant instead compares
           against W12's fallback, blink_amount_q(id, t, 8) with
           beats, i.e. W33's "quant+beats" model.)
  ADOPTED  smooth-with-beats: f32(max(blink_amount_f64(id, t),
           blink_at(t, beats))) — charanim::blink_amount composed
           exactly as film.rs's blink_of composes today, with the
           Blinker swapped for the channel. The patch in
           BLINK-PATCH-W58.md casts the channel to f32 BEFORE the
           max; that is bit-identical to this max-then-cast form
           because blink_at only ever returns 0.0 or 1.0 and the
           channel lies in [0, 1] (max commutes with the monotone
           f32 cast at those endpoints).

A character-frame CHANGES iff the f32 bit pattern entering
state_key (src/film.rs:2161, blink component at :2176) differs
between the models. Classes:

  OPENED    current 0, adopted > 0   — blink where today has none
  REMOVED   current > 0, adopted 0   — today's blink moved away
  RESHAPED  both > 0, bits differ    — e.g. a full 1.0 shut becomes
                                       a partial lid mid-curve

Beat frames (the 7 authored word beats, anchors in film/onoda.json)
are counted separately and MUST show zero changes: blink_at forces
1.0 in both models, so a changed beat frame would mean the patch
broke the beats law. The tool fails (exit 2) if any appears.

Reconciliation with W33 is asserted, not assumed: total cache
lookups must be 3,612; current-model blink>0 frames 159 (today) /
224 (quant+beats); adopted blink>0 frames 279 (smooth+beats) — the
figures in BLINK-DECISION-W33.md. The decomposition identity
  adopted_on = both_on + OPENED,  current_on = both_on + REMOVED,
  changed = OPENED + REMOVED + RESHAPED
is printed so the frame census and W33's event/paint accounting
can be checked against each other. (W33's +219 is a count of
DISTINCT VALUES per event — paints — not frames; the two numbers
measure different things and neither bounds the other directly.)

Models are imported from blink_cost.py (same directory) rather
than re-ported, so the two tools can never drift apart. PRESENCE
below is re-transcribed at HEAD aa92cee (line = the blink
assignment in src/film.rs) and asserted equal to blink_cost's
table on (name, seed, beat) per segment.

Also reported: the standing byte-probe ranges (195-204, 795-814,
1195-1204; DETERMINISM-CONTRACT-W04 §5) intersected with the
changed film-frame set (a film frame changes iff ANY staged
character's blink changes on it), including the blink-clean
sub-ranges the A/B gate may assert byte-identical.

Usage: python3 tools/perf/blink_census.py [--root DIR]
              [--against today|quant] [--json PATH]
Exit: 0 on success (self-checks + reconciliation printed),
      2 on data mismatch / failed check.
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import blink_cost as bc  # ports, model_value, PRESENCE, self-checks

# ---------------------------------------------------------------------
# Presence at HEAD aa92cee: (display name, blink seed == charanim id,
# beat anchor|None); line = the blink assignment in src/film.rs.
#   s01 :1416 onoda blink_of(3, years0)
#   s02 :1448 onoda(3)  :1454 compA(7)  :1460 compB(11)
#   s03 :1479 taniguchi(9)  :1485 onoda blink_of(3, never2)
#         :1490 compA(7)  :1495 compB(11)
#   s04 :1509 compA(7)  :1515 compB(11)  :1522 onoda blink_of(3, message3)
#   s05a: no char() calls (plane + leaflets only) — nobody staged
#   s05b:1603 onoda(3)
#   s06 :1643 compA(7)  :1646 compB(11)  :1662 onoda(3)
#   s07 :1723 onoda(3)
#   s08 :1755 suzuki blink_of(5, found7)  :1770 onoda(3)
#   s08b:1790 onoda(3)  :1795 suzuki(5)
#   s09 :1822 onoda blink_of(3, commander8)  :1830 suzuki(5)
#   s10 :1863 taniguchi blink_of(9, bookseller9)
#   s11 :1925 onoda(3) walk-in branch, :1938 onoda(3) post-drop
#         branch (same seed — presence is continuous)  :1949 taniguchi(9)
#   s12 :1980 onoda blink_of(3, loyal11)  :1986 taniguchi(9)
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

PROBES = [(195, 204), (795, 814), (1195, 1204)]  # W04 contract §5

# W33 reconciliation targets (BLINK-DECISION-W33.md headline table).
W33_LOOKUPS = 3612
W33_ON = {"today": 159, "quant": 224}   # current model, blink>0 frames
W33_ADOPTED_ON = 279                    # smooth+beats, blink>0 frames


def fmt_ranges(frames):
    """[1,2,3,5,7,8] -> '1-3, 5, 7-8' ('' for empty)."""
    if not frames:
        return ""
    out = []
    start = prev = frames[0]
    for n in frames[1:]:
        if n == prev + 1:
            prev = n
            continue
        out.append(f"{start}" if start == prev else f"{start}-{prev}")
        start = prev = n
    out.append(f"{start}" if start == prev else f"{start}-{prev}")
    return ", ".join(out)


def main():
    args = sys.argv[1:]
    root = Path(__file__).resolve().parents[2]
    against = "today"
    json_path = None
    i = 0
    while i < len(args):
        if args[i] == "--root":
            root = Path(args[i + 1])
            i += 2
        elif args[i] == "--against":
            against = args[i + 1]
            i += 2
        elif args[i] == "--json":
            json_path = Path(args[i + 1])
            i += 2
        else:
            print(f"unknown argument: {args[i]}", file=sys.stderr)
            return 2
    if against not in W33_ON:
        print(f"--against must be one of {sorted(W33_ON)}", file=sys.stderr)
        return 2
    cur_model = "today" if against == "today" else "quant+beats"
    adop_model = "smooth+beats"

    manifest = json.loads((root / "scenes/film/film.json").read_text())
    onoda = json.loads((root / "film/onoda.json").read_text())
    fps = onoda["fps"]
    anchors = onoda["anchors"]
    segments = manifest["segments"]

    # ---- integrity (fail loud, never guess) ----
    problems = []
    if [s["id"] for s in segments] != list(PRESENCE.keys()):
        problems.append("manifest segment ids != PRESENCE table ids/order")
    if PRESENCE != bc.PRESENCE:
        problems.append("PRESENCE != blink_cost.PRESENCE (transcription drift)")
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

    print("PORT SELF-CHECKS (shared with blink_cost.py, vs the Rust test bands)")
    ok_all = True
    for label, val, ok in bc.port_self_checks():
        ok_all &= ok
        print(f"  [{'PASS' if ok else 'FAIL'}] {label}: {val}")
    if not ok_all:
        return 2
    print(f"  [PASS] data: 14 segments, {onoda['frames']} frames, fps={fps}, "
          f"all 7 beat anchors inside their segments")
    print(f"  [PASS] presence: this census and blink_cost.py agree on every "
          f"(character, seed, beat)")
    print()
    print(f"CENSUS: current = {cur_model}, adopted = {adop_model} "
          f"(charanim smooth, beats maxed back in)")
    print()

    # ---- simulation ----
    lookups = 0
    cur_on = adop_on = both_on = equal_on = 0
    opened, removed, reshaped = [], [], []       # (frame, seg, char)
    beat_frames = 0
    beat_changed = 0
    seg_report = {}
    changed_film_frames = set()
    for seg in segments:
        sid = seg["id"]
        seg_chars = {}
        seg_union = set()
        for name, seed, beat in PRESENCE[sid]:
            cid = seed
            beats = [anchors[beat]] if beat else []
            ch = {"changed": [], "opened": 0, "removed": 0, "reshaped": 0}
            for n in range(seg["startFrame"], seg["endFrame"]):
                t = n / fps
                lookups += 1
                a = bc.model_value(cur_model, cid, float(seed), t, beats)
                b = bc.model_value(adop_model, cid, float(seed), t, beats)
                a_on, b_on = a > 0.0, b > 0.0
                cur_on += a_on
                adop_on += b_on
                if a_on and b_on:
                    both_on += 1
                if bc.f32_bits(a) == bc.f32_bits(b):
                    if a_on:
                        equal_on += 1
                    continue
                # changed
                ch["changed"].append(n)
                seg_union.add(n)
                changed_film_frames.add(n)
                if not a_on and b_on:
                    ch["opened"] += 1
                    opened.append((n, sid, name))
                elif a_on and not b_on:
                    ch["removed"] += 1
                    removed.append((n, sid, name))
                else:
                    ch["reshaped"] += 1
                    reshaped.append((n, sid, name))
                if beat and bc.blink_at(t, beats) == 1.0:
                    beat_changed += 1
            if beat:
                for n in range(seg["startFrame"], seg["endFrame"]):
                    if bc.blink_at(n / fps, beats) == 1.0:
                        beat_frames += 1
            seg_chars[name] = ch
        seg_report[sid] = {"characters": seg_chars,
                           "film_frames_changed": sorted(seg_union)}

    changed_total = len(opened) + len(removed) + len(reshaped)

    # ---- per-segment report ----
    print("PER SEGMENT x CHARACTER — changed character-frames "
          "(O=opened, R=removed, S=reshaped)")
    for seg in segments:
        sid = seg["id"]
        if not PRESENCE[sid]:
            print(f"{sid:<5} (nobody staged — no blink can change)")
            continue
        for name, seed, beat in PRESENCE[sid]:
            ch = seg_report[sid]["characters"][name]
            tag = f" beat={beat}" if beat else ""
            print(f"{sid:<5} {name:<10} id={seed:<3}{tag:<22} "
                  f"changed {len(ch['changed']):>3} "
                  f"(O {ch['opened']:>3} / R {ch['removed']:>3} / "
                  f"S {ch['reshaped']:>3})")
            print(f"      frames: {fmt_ranges(ch['changed'])}")
        u = seg_report[sid]["film_frames_changed"]
        print(f"  -> {sid} film frames changed (any character): "
              f"{len(u)} of {seg['frames']}: {fmt_ranges(u)}")
    print()

    # ---- reconciliation with W33 ----
    print("RECONCILIATION WITH W33 (BLINK-DECISION-W33.md)")
    rec = [
        ("cache lookups (character-frames)", lookups, W33_LOOKUPS),
        (f"current ({cur_model}) blink>0 frames", cur_on, W33_ON[against]),
        ("adopted (smooth+beats) blink>0 frames", adop_on, W33_ADOPTED_ON),
    ]
    ok_all = True
    for label, got, want in rec:
        ok = got == want
        ok_all &= ok
        print(f"  [{'PASS' if ok else 'FAIL'}] {label}: census {got}, W33 {want}")
    ident_l = adop_on - (both_on + len(opened))
    ident_r = cur_on - (both_on + len(removed))
    ok = ident_l == 0 and ident_r == 0
    ok_all &= ok
    print(f"  [{'PASS' if ok else 'FAIL'}] decomposition identity: "
          f"adopted_on = both_on({both_on}) + opened({len(opened)}); "
          f"current_on = both_on + removed({len(removed)})")
    ok = beat_changed == 0
    ok_all &= ok
    print(f"  [{'PASS' if ok else 'FAIL'}] beats law: {beat_frames} beat "
          f"character-frames (blink_at = 1), {beat_changed} changed — the "
          f"7 authored blinks survive adoption bit-exactly")
    print()
    print("FILM TOTALS")
    print(f"  changed character-frames: {changed_total} of {lookups} lookups "
          f"({100.0 * changed_total / lookups:.2f}%) — "
          f"OPENED {len(opened)}, REMOVED {len(removed)}, "
          f"RESHAPED {len(reshaped)}")
    print(f"  both-on frames: {both_on} (of which bit-equal: {equal_on}, "
          f"reshaped: {len(reshaped)})")
    print(f"  changed film frames (any staged character): "
          f"{len(changed_film_frames)} of {onoda['frames']}")
    print(f"  W33's +219 (smooth+beats, r=1) counts distinct VALUES per "
          f"event (paints);")
    print(f"  this census counts FRAMES. Every extra paint lands on a "
          f"changed frame, but a")
    print(f"  changed frame whose remaining state is unique anyway costs "
          f"no paint — the two")
    print(f"  figures are consistent, not interchangeable.")
    print()

    # ---- probe-range intersection (the A/B gate's raw material) ----
    print("BYTE-PROBE RANGES vs THE CHANGED SET (DETERMINISM-CONTRACT-W04 §5)")
    probe_report = {}
    for lo, hi in PROBES:
        rng = set(range(lo, hi + 1))
        ch = sorted(rng & changed_film_frames)
        clean = sorted(rng - changed_film_frames)
        probe_report[f"{lo}-{hi}"] = {"changed": ch, "clean": clean}
        print(f"  {lo}-{hi}: changed {len(ch)} {fmt_ranges(ch) or '—'}")
        print(f"           blink-clean {len(clean)}: {fmt_ranges(clean) or '—'} "
              f"(byte-identity assertable on these, given the patch "
              f"touches blink only)")
    print()

    if json_path:
        report = {
            "against": cur_model,
            "adopted": adop_model,
            "totals": {
                "lookups": lookups,
                "current_on": cur_on,
                "adopted_on": adop_on,
                "both_on": both_on,
                "equal_on": equal_on,
                "changed_character_frames": changed_total,
                "opened": len(opened),
                "removed": len(removed),
                "reshaped": len(reshaped),
                "changed_film_frames": len(changed_film_frames),
                "beat_frames": beat_frames,
                "beat_changed": beat_changed,
            },
            "segments": {
                sid: {
                    "characters": {
                        name: {
                            "changed": seg_report[sid]["characters"][name]["changed"],
                            "opened": seg_report[sid]["characters"][name]["opened"],
                            "removed": seg_report[sid]["characters"][name]["removed"],
                            "reshaped": seg_report[sid]["characters"][name]["reshaped"],
                        } for name, _, _ in PRESENCE[sid]
                    },
                    "film_frames_changed": seg_report[sid]["film_frames_changed"],
                } for sid in seg_report
            },
            "probes": probe_report,
        }
        json_path.write_text(json.dumps(report, indent=1))
        print(f"wrote {json_path}")

    return 0 if ok_all else 2


if __name__ == "__main__":
    sys.exit(main())
