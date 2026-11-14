#!/usr/bin/env python3
"""run_tests.py — behavioural tests for tools/film/rerender_plan.py.

Synthetic cases run against the miniature fixture film under
fixtures/rerender/ (3 segments a/b/c, frames [0,60) [60,150)
[150,240), one caption chunk each). The remaining cases are built
programmatically from that base in a temp dir. The final two cases
are the REAL Wave-2 caption re-chunk, reconstructed from git
history (scenes side: 3a36f1f^ -> 3a36f1f; driver side:
5e90d7e^ -> 5e90d7e) and checked against the seven split windows
recorded in docs/ab/film/RECHUNK-W01.md.

Usage: python3 tools/film/fixtures/rerender/run_tests.py
Exit: 0 = all pass (real cases may SKIP without git), 1 = a failure.
"""

import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
TOOL = os.path.join(REPO, "tools", "film", "rerender_plan.py")
MINI = os.path.join(HERE, "mini-old")

FAILURES = []


def plan(old, new, *extra):
    r = subprocess.run([sys.executable, TOOL, old, new, "--json", *extra],
                       capture_output=True, text=True)
    if r.returncode != 0:
        raise AssertionError("planner exited %d: %s" % (r.returncode,
                                                        r.stderr.strip()))
    return json.loads(r.stdout)


def ranges(p):
    return [(r["start"], r["end"]) for r in p["ranges"]]


def check(name, got, want):
    if got == want:
        print("PASS %s" % name)
    else:
        print("FAIL %s\n  got:  %r\n  want: %r" % (name, got, want))
        FAILURES.append(name)


def load(p):
    with open(p) as f:
        return json.load(f)


def save(p, d):
    with open(p, "w") as f:
        json.dump(d, f, indent=1, sort_keys=True)


def variant(tmp, name, mutate):
    dst = os.path.join(tmp, name)
    shutil.copytree(MINI, dst)
    mutate(dst)
    return dst


def seg_edit(root, seg_id, fn):
    p = os.path.join(root, "scenes", "film", "film.json")
    m = load(p)
    for s in m["segments"]:
        if s["id"] == seg_id:
            fn(s)
    save(p, m)


def scene_edit(root, seg_id, fn):
    p = os.path.join(root, "scenes", "film", "%s.scene.json" % seg_id)
    d = load(p)
    fn(d)
    save(p, d)


def main():
    tmp = tempfile.mkdtemp(prefix="rerender-tests-")

    # 1 — identical snapshots: empty plan
    p = plan(MINI, MINI)
    check("identical: no ranges", ranges(p), [])
    check("identical: untouched is the whole film",
          [(u["start"], u["end"]) for u in p["untouched"]], [(0, 240)])

    # 2 — caption text change only (static fixture): exactly the
    # chunk window. Chunk [2.5, 4.0) at 30 fps: frames n with
    # 2.5 <= n/30 < 4.0 -> [75, 120).
    p = plan(MINI, os.path.join(HERE, "mini-new-caption"))
    check("caption text: exactly the chunk window", ranges(p), [(75, 120)])

    # 3 — caption split (the re-chunk shape): one chunk becomes two
    # pieces inside the same outer span, later chunks renumber.
    def split(root):
        def f(d):
            d["captions"] = [
                {"index": 1, "id": "cap_001", "s": 2.5, "e": 3.1,
                 "text": "bravo", "fit": 1.0},
                {"index": 2, "id": "cap_002", "s": 3.1, "e": 4.0,
                 "text": "two", "fit": 1.0}]
        scene_edit(root, "b", f)
        scene_edit(root, "c", lambda d: d["captions"][0].update(
            index=3, id="cap_003"))
        seg_edit(root, "b", lambda s: s.update(
            captionChunks={"first": 1, "last": 2, "count": 2}))
        seg_edit(root, "c", lambda s: s.update(
            captionChunks={"first": 3, "last": 3, "count": 1}))
    p = plan(MINI, variant(tmp, "split", split))
    check("caption split: exactly the outer window", ranges(p), [(75, 120)])

    # 4 — renumber only: indices/ids move, chunk contents identical.
    def renumber(root):
        scene_edit(root, "b", lambda d: d["captions"][0].update(
            index=7, id="cap_007"))
        scene_edit(root, "c", lambda d: d["captions"][0].update(
            index=9, id="cap_009"))
    p = plan(MINI, variant(tmp, "renumber", renumber))
    check("renumber only: no ranges", ranges(p), [])

    # 5 — scene data change (static fixture): exactly segment b.
    p = plan(MINI, os.path.join(HERE, "mini-new-scene"))
    check("scene lights: exactly segment b", ranges(p), [(60, 150)])

    # 6 — timing shift: b stretches 5.0->5.5 s, c shifts with it;
    # the union covers both segments' old+new ranges, a untouched.
    def shift(root):
        def fb(s):
            s.update(endS=5.5, durationS=3.5, endFrame=165, frames=105)
        def fc(s):
            s.update(startS=5.5, endFrame=255, frames=90)
        seg_edit(root, "b", fb)
        seg_edit(root, "c", fc)
        pth = os.path.join(root, "scenes", "film", "film.json")
        m = load(pth)
        m.update(totalFrames=255, durationS=8.5)
        save(pth, m)
    p = plan(MINI, variant(tmp, "shift", shift))
    check("timing shift: union b+c", ranges(p), [(60, 255)])
    check("timing shift: a untouched",
          [(u["start"], u["end"]) for u in p["untouched"]], [(0, 60)])

    # 7 — inheritance: fixture graph b -> c; a dressing change to b
    # expands to c.
    def inherit(root):
        save(os.path.join(root, "scenes", "film", "inheritance.json"),
             {"b": ["c"]})
        scene_edit(root, "b", lambda d: d["lights"]["key"].update(
            intensity=2.5))
    old_i = variant(tmp, "inh-old", lambda r: save(
        os.path.join(r, "scenes", "film", "inheritance.json"),
        {"b": ["c"]}))
    p = plan(old_i, variant(tmp, "inh-new", inherit))
    check("inheritance: b expands to c", ranges(p), [(60, 240)])
    check("inheritance: reason names the mechanism",
          any("inherits dressing from b" in r
              for rr in p["ranges"] for r in rr["reasons"]), True)

    # 8 — stale tail: totalFrames shrinks, segments unchanged.
    def shrink(root):
        pth = os.path.join(root, "scenes", "film", "film.json")
        m = load(pth)
        m.update(totalFrames=210)
        save(pth, m)
    p = plan(MINI, variant(tmp, "shrink", shrink))
    check("stale tail: delete-only range", ranges(p), [(210, 240)])
    check("stale tail: flagged delete_only",
          [r["delete_only"] for r in p["ranges"]], [True])

    # 9 — driver-only snapshot: caption text change in onoda.json.
    def driver_pair(tmp):
        base = {"film": "mini", "project": "/nonexistent",
                "duration": 4.0, "fps": 30, "frames": 120,
                "lines": [{"start": 0.0, "end": 4.0}],
                "anchors": {"a1": 1.0},
                "captions": [{"line": 0, "s": 1.0, "e": 2.0, "text": "x",
                              "width": 100.0, "fit": 1.0,
                              "strip": "cap_000.png"}]}
        dirs = []
        for name, cap_text in (("drv-old", "x"), ("drv-new", "y")):
            d = os.path.join(tmp, name)
            os.makedirs(d)
            data = json.loads(json.dumps(base))
            data["captions"][0]["text"] = cap_text
            save(os.path.join(d, "onoda.json"), data)
            dirs.append(d)
        return dirs
    do_, dn = driver_pair(tmp)
    p = plan(do_, dn)
    check("driver captions: chunk window", ranges(p), [(30, 60)])

    # 10 — driver anchors change: not decomposable -> full film.
    dn2 = os.path.join(tmp, "drv-anchors")
    shutil.copytree(do_, dn2)
    d = load(os.path.join(dn2, "onoda.json"))
    d["anchors"]["a1"] = 1.5
    save(os.path.join(dn2, "onoda.json"), d)
    p = plan(do_, dn2)
    check("driver anchors: full film", ranges(p), [(0, 120)])

    # 11/12 — the real Wave-2 re-chunk, from git history.
    real_cases(tmp)

    print("\n%s" % ("ALL PASS" if not FAILURES else
                    "FAILURES: %s" % ", ".join(FAILURES)))
    return 1 if FAILURES else 0


# The seven split windows of the re-chunk (docs/ab/film/RECHUNK-W01.md
# §2), mapped with the driver's arithmetic s <= n/30 < e:
#   6.69-8.53 -> [201,256)    21.61-22.61 -> [649,679)
#   31.55-32.99 -> [947,990)  37.13-38.45 -> [1114,1154)
#   44.01-45.395 -> [1321,1362)  46.39-47.77 -> [1392,1434)
#   52.07-53.57 -> [1563,1608)
RECHUNK_WINDOWS = [(201, 256), (649, 679), (947, 990), (1114, 1154),
                   (1321, 1362), (1392, 1434), (1563, 1608)]


def git_snap(tmp, name, ref, paths):
    try:
        raw = subprocess.run(["git", "-C", REPO, "archive", ref, "--"] +
                             paths, capture_output=True, check=True).stdout
    except Exception:  # noqa: BLE001 - no git / no such ref: skip
        return None
    dst = os.path.join(tmp, name)
    os.makedirs(dst)
    with tarfile.open(fileobj=io.BytesIO(raw)) as tf:
        tf.extractall(dst)
    return dst


def real_cases(tmp):
    old = git_snap(tmp, "real-scenes-old", "3a36f1f^",
                   ["scenes", "film", "assets"])
    new = git_snap(tmp, "real-scenes-new", "3a36f1f",
                   ["scenes", "film", "assets"])
    if old is None or new is None:
        print("SKIP real scenes-side re-chunk (git history unavailable)")
    else:
        p = plan(old, new)
        check("REAL scenes re-chunk 3a36f1f: exactly the 7 windows",
              ranges(p), RECHUNK_WINDOWS)
        # the record's cross-check: exactly 10 of the 60 one-second
        # A/B sample stills (frame = 30t + 15) fall inside the plan
        hits = [f for f in (30 * k + 15 for k in range(60))
                if any(s <= f < e for s, e in ranges(p))]
        check("REAL scenes re-chunk: 10 sampled stills affected",
              len(hits), 10)
    old = git_snap(tmp, "real-drv-old", "5e90d7e^", ["film"])
    new = git_snap(tmp, "real-drv-new", "5e90d7e", ["film"])
    if old is None or new is None:
        print("SKIP real driver-side re-chunk (git history unavailable)")
    else:
        p = plan(old, new)
        check("REAL driver re-chunk 5e90d7e: exactly the 7 windows",
              ranges(p), RECHUNK_WINDOWS)


if __name__ == "__main__":
    sys.exit(main())
