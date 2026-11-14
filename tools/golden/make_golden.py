#!/usr/bin/env python3
"""Native golden manifest — assemble, pin, and verify TaleRender's own
reference frames (docs/golden/NATIVE-GOLDEN.md is the policy).

The golden set is NATIVE-only. It exists because contract §5 resolved
(b): native frames stay clean, so native QA references are baselined
natively, once, and never conflated with the skill's Chrome golden
sheet (md5 c535024d...), which stays where it is.

Usage:
    python3 tools/golden/make_golden.py           # verify: OK/STALE/MISSING table
    python3 tools/golden/make_golden.py --json    # verify + print the manifest JSON
    python3 tools/golden/make_golden.py --pin     # RE-BASELINE: record the sha256
                                                  # of every reference currently
                                                  # on disk into golden-hashes.json

--pin is the deliberate re-baseline act described in NATIVE-GOLDEN.md.
It is never run by CI and never implied by a render: a human runs it,
measures the diff first, and commits the hash change with the reason.

Exit code: 0 when every pinned item verifies OK (MISSING items that
have never been pinned are reported but do not fail the run — the
film frames are expected MISSING until the film renders); 1 when any
pinned item is STALE (on disk but not the pinned bytes).
"""

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PIN_FILE = Path(__file__).resolve().parent / "golden-hashes.json"

RENDER = ".github/workflows/render.yml (workflow_dispatch)"

# The film manifest is the source of the segment frame ranges; mid
# frame = (startFrame + endFrame) // 2. Kept in step with
# scenes/film/film.json — if the manifest changes, these change.
FILM_SEGMENTS = [
    ("s01", 0, 178), ("s02", 178, 392), ("s03", 392, 479),
    ("s04", 479, 642), ("s05a", 642, 737), ("s05b", 737, 784),
    ("s06", 784, 912), ("s07", 912, 1027), ("s08", 1027, 1211),
    ("s08b", 1211, 1252), ("s09", 1252, 1361), ("s10", 1361, 1520),
    ("s11", 1520, 1666), ("s12", 1666, 1811),
]


def film_item(seg, start, end):
    mid = (start + end) // 2
    return {
        "id": f"film-{seg}",
        "kind": "film segment mid frame",
        "source": f"scenes/film/{seg}.scene.json via the film driver "
                  f"(segment frames {start}..{end - 1}, mid frame {mid}, "
                  f"t = {mid / 30:.4f} s)",
        "generator": f"{RENDER} mode=film from={mid} to={mid + 1} -> "
                     f"talerender --film film --out render-out/frames "
                     f"--from {mid} --to {mid + 1}; pin "
                     f"render-out/frames/frame_{mid:05d}.png to the path below",
        "path": f"docs/golden/frames/{seg}.png",
    }


ITEMS = [
    {
        "id": "s28-onoda-t150",
        "kind": "first-light A/B frame (§28)",
        "source": "Onoda S01 @ t = 1.50, hand-authored ab1 staging "
                  "(docs/ab/onoda-s01-t150/)",
        "generator": "produced by the §28 prototype run (package scripts in "
                     "docs/ab/onoda-s01-t150/scripts/); the current-chain "
                     "equivalent is " + RENDER + " mode=ab1 -> "
                     "talerender --scene ab1 --out render-out/ab1.png. "
                     "Kept as the historical first-light pin; the M5-chain "
                     "ab1 reference is ab1-composer below.",
        "path": "docs/ab/onoda-s01-t150/native.png",
    },
    {
        "id": "dof-chart",
        "kind": "DoF chart (contract §1, M3 gate)",
        "source": "src/dof.rs chart scene: 10 textured panels 2-30 m + "
                  "bleed panel, focus 10 m, aperture 0.0015 / maxblur 0.006",
        "generator": RENDER + " mode=dof-chart -> "
                     "talerender --scene dof-chart --out render-out/dof-chart.png",
        "path": "docs/ab/dof-chart/native.png",
    },
    {
        "id": "stroke-toon",
        "kind": "stroke-battery character card (contract §2, M4 gate)",
        "source": "docs/ab/stroke-battery/spec.json through the CPU "
                  "painter in src/scene.rs (no GPU involved)",
        "generator": "talerender --scene stroke-battery --out <dir> "
                     "(CI binary; battery mode of src/main.rs)",
        "path": "docs/ab/stroke-battery/native-toon.png",
    },
    {
        "id": "ab1-composer",
        "kind": "ab1 at the full M5 composer chain",
        "source": "ab1 scene through Render -> Bokeh -> Bloom -> Output "
                  "-> GRADE, HalfFloat MSAA-4x (docs/ab/composer/)",
        "generator": RENDER + " mode=ab1 -> "
                     "talerender --scene ab1 --out render-out/ab1.png",
        "path": "docs/ab/composer/native.png",
    },
] + [film_item(*s) for s in FILM_SEGMENTS]


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def load_pins():
    if PIN_FILE.exists():
        return json.loads(PIN_FILE.read_text())
    return {}


def main():
    pin_mode = "--pin" in sys.argv
    pins = load_pins()

    if pin_mode:
        new_pins = dict(pins)
        for item in ITEMS:
            p = ROOT / item["path"]
            if p.exists():
                new_pins[item["id"]] = sha256_of(p)
        PIN_FILE.write_text(json.dumps(new_pins, indent=2) + "\n")
        print(f"--pin: recorded {len(new_pins)} hashes in {PIN_FILE.name} "
              f"(re-baseline act — commit it with the measured diff and reason)")
        pins = new_pins

    rows, counts = [], {"OK": 0, "STALE": 0, "MISSING": 0}
    manifest = []
    for item in ITEMS:
        p = ROOT / item["path"]
        current = sha256_of(p) if p.exists() else None
        recorded = pins.get(item["id"])
        if current is None:
            status = "MISSING"
        elif recorded is None:
            status = "MISSING"  # on disk but never pinned: not a reference yet
        elif current == recorded:
            status = "OK"
        else:
            status = "STALE"
        counts[status] += 1
        rows.append((item["id"], status, item["path"],
                     (recorded or "-")[:16], (current or "-")[:16]))
        manifest.append({**item, "sha256_pinned": recorded,
                         "sha256_current": current, "status": status})

    w = max(len(r[0]) for r in rows)
    print(f"{'item':<{w}}  {'status':<7}  {'pinned':<16}  {'current':<16}  path")
    for rid, status, path, pin_h, cur_h in rows:
        print(f"{rid:<{w}}  {status:<7}  {pin_h:<16}  {cur_h:<16}  {path}")
    print(f"\n{counts['OK']} OK / {counts['STALE']} STALE / "
          f"{counts['MISSING']} MISSING of {len(ITEMS)} golden items")

    if "--json" in sys.argv:
        print(json.dumps(manifest, indent=2))

    return 1 if counts["STALE"] else 0


if __name__ == "__main__":
    sys.exit(main())
