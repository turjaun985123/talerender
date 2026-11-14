#!/usr/bin/env python3
"""bake_s12.py — regenerate the W52 baked s12 staging copy.

Reads the repo's scenes/film/s12.scene.json, verifies the two
`apply`-carrying poseTrack records (Onoda's `lean` and `headTilt`)
still match the recorded originals EXACTLY (keys + apply text),
multiplies the apply factors through the key values, drops the
`apply` field, records the bake in each record's `law`, and writes
the result to s12.scene.json beside this script.

The factors are the film.rs S12 driver's own (src/film.rs, the S12
shoot function):

    let bow = pose_track(&[key(0.0, 0.0), keym(w_loyal - st0 - 0.1, 3.5, 0.45)], lt);
    onoda.lean      = (0.6 * bow) as f32;
    onoda.head_tilt = (-bow) as f32;

Both are pure scalar multiplies of the evaluated track, and
pose_track's output is always a stored key value or a lerp of two
stored key values, so the factor distributes over the keys exactly
(see docs/integration/S12-BAKE-W52.md for the per-key f32
arithmetic and the sampled equivalence proof).

STALENESS GUARD: if the source records have changed since the bake
was recorded (different keys, different apply text, a third apply
record), this script FAILS and writes nothing — a changed bowA in
the source must be re-baked by a human who updates FACTORS here,
never silently propagated. Key value -0.0 (the -1 x 0 product) is
spelled 0, matching the source record's own spelling; the sign of
an exact zero is the only bit the bake does not reproduce, and no
consumer distinguishes it (S12-BAKE-W52.md, headTilt key 1).

Usage:  python3 tools/format/staging/s12-baked/bake_s12.py
Exit:   0 written / 1 source drifted / 2 usage or IO error.
House style: one-line ERROR: on stderr for exit 2.
"""
import copy
import json
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
SRC = REPO / "scenes" / "film" / "s12.scene.json"
OUT = HERE / "s12.scene.json"

# channel name -> (factor, original apply text, original keys)
FACTORS = {
    "lean": (0.6, "lean = bowA * 0.6", [[0, 0], [2.5, 3.5, 0.45]]),
    "headTilt": (-1.0, "headTilt = -bowA", [[0, 0], [2.5, 3.5, 0.45]]),
}

LAW_SUFFIX = {
    "lean": ("BAKED (W52): the former apply 'lean = bowA * 0.6' is "
             "multiplied through these keys (film.rs S12: "
             "onoda.lean = (0.6 * bow) as f32); pre-bake law: "),
    "headTilt": ("BAKED (W52): the former apply 'headTilt = -bowA' is "
                 "multiplied through these keys (film.rs S12: "
                 "onoda.head_tilt = (-bow) as f32); pre-bake law: "),
}


def f32(v):
    """Nearest f32, widened back to a Python (f64) float."""
    return struct.unpack("f", struct.pack("f", v))[0]


def fail(msg):
    print("BAKE: FAIL — " + msg)
    return 1


def main():
    try:
        doc = json.loads(SRC.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as e:
        print("ERROR: cannot read %s: %s" % (SRC, e), file=sys.stderr)
        return 2

    # Locate the apply records; refuse if the census has changed.
    targets = {}
    for node in doc["nodes"]:
        for ch_name, rec in (node.get("channels") or {}).items():
            if isinstance(rec, dict) and "apply" in rec:
                targets[ch_name] = (node.get("name"), rec)
    if set(targets) != set(FACTORS):
        return fail("apply-record census changed: found %s, expected %s"
                    % (sorted(targets), sorted(FACTORS)))

    baked = copy.deepcopy(doc)
    for node in baked["nodes"]:
        for ch_name, rec in (node.get("channels") or {}).items():
            if ch_name not in FACTORS or "apply" not in rec:
                continue
            factor, apply_text, orig_keys = FACTORS[ch_name]
            if rec["apply"] != apply_text or rec["keys"] != orig_keys:
                return fail("%s record drifted: apply=%r keys=%r "
                            "(expected %r / %r) — re-bake by hand"
                            % (ch_name, rec.get("apply"), rec.get("keys"),
                               apply_text, orig_keys))
            new_keys = []
            for k in rec["keys"]:
                v = factor * k[1]
                if v == 0:
                    # spell -0.0 as 0; keep the source's own token
                    # (int 0) so the baked file diffs minimally
                    v = k[1] if k[1] == 0 else 0.0
                new_keys.append([k[0], v] + list(k[2:]))
            rec["keys"] = new_keys
            del rec["apply"]
            rec["law"] = LAW_SUFFIX[ch_name] + rec["law"]

    # Prove nothing else moved: mask the two records and deep-compare.
    def masked(d):
        d = copy.deepcopy(d)
        for node in d["nodes"]:
            for ch_name in FACTORS:
                if ch_name in (node.get("channels") or {}):
                    node["channels"][ch_name] = None
        return d
    if masked(baked) != masked(doc):
        return fail("internal error: bake touched data outside the "
                    "two apply records")

    text = json.dumps(baked, indent=1, ensure_ascii=False)
    OUT.write_text(text, encoding="utf-8")
    for ch_name, (factor, _, _) in FACTORS.items():
        keys = next(rec["keys"] for node in baked["nodes"]
                    for name, rec in (node.get("channels") or {}).items()
                    if name == ch_name)
        print("baked %-8s x %-4s -> keys %s (f32 arrival: %r)"
              % (ch_name, factor, keys, f32(keys[-1][1])))
    print("BAKE: OK — wrote %s" % OUT)
    return 0


if __name__ == "__main__":
    sys.exit(main())
