#!/usr/bin/env python3
"""locomotion_census.py — enumerate every frame the locomotion
adoption (W24's src/locomotion.rs wired into film staging) would
touch, before anyone wires it.

The film's walks are staged by seven hand-rolled (walk_phase,
walk_bob) call sites in src/film.rs. W24's adoption replaces each
pair with one module eval, which changes pixels two ways (W24 §4):
the bob waveform (module drop law vs film's amp*(1-|sin phase|))
and — where an arm is not authored by a beat or a hold — a NEW
counter-phase arm swing. This tool counts, per site and per
character, exactly which frames carry which change, so the A/B
sign-off (docs/integration/LOCOMOTION-AB-W57.md) is decided on
evidence.

Frame classes (driver = the site's progress value — cam_k for s02,
seg() wk for the rest — evaluated at the frame's staged instant
t = n/fps with the film's own formulas, film.rs:121/139/144):

  HEAD-STATIC      driver == 0: phase/bob channels written from the
                   gait pair but phase is not advancing. Legs are
                   Walk in this class except s10 (D12: Stand).
  ADVANCING        0 < driver < 1 and legs == Walk: the character
                   is walking; phase advances frame to frame.
  ADVANCING-STAND  s02 only: 0.97 < k < 1 — phase still advances
                   (travelled is unclamped, D6) but legs already
                   switched to Stand (k > 0.97 rule).
  TAIL-STATIC      driver == 1: legs Stand, phase/bob frozen at
                   their end values for the rest of the segment.

Bob changes on EVERY gait-staged frame (all four classes):
toonfull adds st.bob into the body Y unconditionally
(toonfull.rs:324), and the module's bob value differs from
walk_bob's at every phase, static or not. Arm swing (motion) is
counted on ADVANCING frames only; on static frames adoption
would instead set a constant arm offset — a staging decision,
enumerated in the doc, not counted here.

Inputs: scenes/film/film.json (manifest: segment table + frame
rule), film/onoda.json (anchors for the s10/s11 windows), and the
site table below, transcribed from src/film.rs at aa92cee. The
tool re-verifies the table's stride/bob-amp multisets against
film.rs's actual walk_phase/walk_bob call sites and refuses
(exit 2) if they have drifted.

Usage: python3 tools/film/locomotion_census.py [--repo ROOT]
Exit 0 = census printed; 2 = input/verification error.
"""

import argparse
import json
import math
import re
import sys
from pathlib import Path

# ------------------------------------------------------------------
# film.rs driver formulas, transcribed (film.rs:121/139/144)
# ------------------------------------------------------------------

def clamp(x, lo, hi):
    return lo if x < lo else hi if x > hi else x

def seg(t, a, b):
    """film.rs:121 — LINEAR clamped ramp (not smootherstep)."""
    return clamp((t - a) / (1e-6 if b - a == 0.0 else b - a), 0.0, 1.0)

def smoother(u):
    """film.rs:139 — smootherstep over clamp01."""
    u = clamp(u, 0.0, 1.0)
    return u * u * u * (u * (u * 6.0 - 15.0) + 10.0)

def cam_k(lt, d, hold):
    """film.rs:144."""
    h = clamp(hold, 0.0, 0.45)
    return smoother((lt / d - h) / (1.0 - 2.0 * h))

# ------------------------------------------------------------------
# Site table — transcribed from src/film.rs at aa92cee.
# driver kinds: ("camk", hold) | ("seg_lt", a, b) | ("seg_t", a, b)
# (seg_t endpoints are anchor-derived; resolved in build_sites()).
# legs rules: "stand_if_ge1" (wk >= 1), "stand_if_k97" (k > 0.97),
#             "walk_iff_mid" (0 < wk < 1, s10's D12 rule).
# arms: per arm — ("swing", base) free static pair, swing applies;
#       ("beat", why) poseTrack/hold-authored, never overwritten;
#       ("authored", why) source-authored for the shot (s11, D15).
# ------------------------------------------------------------------

def build_sites(anchors):
    w_flew = anchors["flew9"]    # 49.22, film/onoda.json
    w_order = anchors["order10"]  # 51.78
    s11_end = max(w_order - 50.648 - 0.25, 0.4)  # film.rs:1915, st0 = s11 startS
    return [
        dict(shot="s02", cite="film.rs:1432-1433 (fn s02:1428)", driver=("camk", 0.03),
             dist="k * 6.8 (travelled, unclamped)", stride=0.62, bob_amp=5.0,
             legs_rule="stand_if_k97", branch=None,
             chars=[
                 dict(name="Onoda (SH_ONODA)", dist_factor=None,
                      arm_l=("swing", (10.0, 6.0)), arm_r=("swing", (16.0, -6.0)),
                      base_note="authored pair == GaitParams.arm_base default"),
                 dict(name="Companion A (SH_COMPA)", dist_factor=None,
                      arm_l=("swing", (8.0, 6.0)), arm_r=("swing", (8.0, 6.0)),
                      base_note="arms unset in staging -> FullState::default (8,6)/(8,6); "
                                "module default base is (10,6)/(16,-6) — BASE MISMATCH, doc D2"),
                 dict(name="Companion B (SH_COMPB)", dist_factor=None,
                      arm_l=("swing", (8.0, 6.0)), arm_r=("swing", (8.0, 6.0)),
                      base_note="arms unset in staging -> FullState::default (8,6)/(8,6); "
                                "module default base is (10,6)/(16,-6) — BASE MISMATCH, doc D2"),
             ]),
        dict(shot="s06", cite="film.rs:1631-1641 (mk_walk closure, fn s06:1613)", driver=("seg_lt", 0.1, 3.55),
             dist="wk * 5.96 (COMPA) / wk * 6.1 (COMPB)", stride=0.62, bob_amp=5.0,
             legs_rule="stand_if_ge1", branch=None,
             chars=[
                 dict(name="Companion A (COMPA)", dist_factor=5.96,
                      arm_l=("swing", (8.0, 6.0)), arm_r=("swing", (8.0, 6.0)),
                      base_note="arms unset in mk_walk -> FullState::default (8,6)/(8,6) — BASE MISMATCH, doc D2"),
                 dict(name="Companion B (COMPB)", dist_factor=6.1,
                      arm_l=("swing", (8.0, 6.0)), arm_r=("swing", (8.0, 6.0)),
                      base_note="arms unset in mk_walk -> FullState::default (8,6)/(8,6) — BASE MISMATCH, doc D2"),
             ]),
        dict(shot="s07", cite="film.rs:1707-1713 (fn s07:1691)", driver=("seg_lt", 0.2, 1.85),
             dist="wk * 2.0", stride=0.6, bob_amp=4.0,
             legs_rule="stand_if_ge1", branch=None,
             chars=[
                 dict(name="Onoda (ONODA)", dist_factor=2.0,
                      arm_l=("swing", (12.0, 6.0)),
                      arm_r=("beat", "poseTrack coconut beat (12 -> 136 -> 34, film.rs:1720) + hold_l 'coconut' from t >= 32.35"),
                      base_note="arm_l static (12,6), free"),
             ]),
        dict(shot="s08", cite="film.rs:1743-1749 (fn s08:1737, Suzuki)", driver=("seg_lt", 0.1, 3.3),
             dist="wk_s * 3.28", stride=0.6, bob_amp=5.0,
             legs_rule="stand_if_ge1", branch=None,
             chars=[
                 dict(name="Suzuki (SUZUKI)", dist_factor=3.28,
                      arm_l=("beat", "(38,-52) holding the paper-map plate, hold_l set for the whole shot (film.rs:1757)"),
                      arm_r=("swing", (12.0, 8.0)),
                      base_note="arm_r static (12,8), free"),
             ]),
        dict(shot="s08", cite="film.rs:1760-1766 (fn s08:1737, Onoda)", driver=("seg_lt", 0.35, 1.95),
             dist="wk_o * 2.25", stride=0.6, bob_amp=4.0,
             legs_rule="stand_if_ge1", branch=None,
             chars=[
                 dict(name="Onoda (ONODA)", dist_factor=2.25,
                      arm_l=("swing", (10.0, 6.0)), arm_r=("swing", (16.0, -6.0)),
                      base_note="authored pair == GaitParams.arm_base default"),
             ]),
        dict(shot="s10", cite="film.rs:1848-1853 (fn s10:1845)", driver=("seg_t", w_flew + 0.5, w_flew + 1.7),
             dist="wk * 0.55", stride=0.58, bob_amp=3.0,
             legs_rule="walk_iff_mid", branch=None,
             chars=[
                 dict(name="Tani (BK_TANI)", dist_factor=0.55,
                      arm_l=("swing", (14.0, 8.0)),
                      arm_r=("beat", "poseTrack beat (42 -> 17, film.rs:1854) + book in hold_r for the whole shot"),
                      base_note="arm_l static (14,8), free; NOTE: driver runs on film time t and its "
                                "window (49.72-50.92) outlasts the segment (ends 50.648) — wk never "
                                "reaches 1 inside the shot (max 0.761 at the last frame)"),
             ]),
        dict(shot="s11", cite="film.rs:1914-1920 (fn s11:1897, walk-in branch)", driver=("seg_lt", 0.1, s11_end),
             dist="wk * 2.5", stride=0.6, bob_amp=4.0,
             legs_rule="stand_if_ge1", branch=("t_lt_drop", 53.570),
             chars=[
                 dict(name="Onoda (ONODA, walk-in)", dist_factor=2.5,
                      arm_l=("authored", "D15 source walk-in arms (10,5) — W24: gait swing OFF unless the user re-stages"),
                      arm_r=("authored", "D15 source walk-in arms (16,-6) — W24: gait swing OFF unless the user re-stages"),
                      base_note="no arm swing at this site; bob change only. Site exists only "
                                "while t < drop_t = 53.570 (film.rs:1914); from frame 1608 the "
                                "rifle-drop branch stages ONODA2 with no gait channels at all"),
             ]),
    ]

# ------------------------------------------------------------------
# classification
# ------------------------------------------------------------------

def driver_value(site, lt, t, d):
    kind = site["driver"][0]
    if kind == "camk":
        return cam_k(lt, d, site["driver"][1])
    if kind == "seg_lt":
        return seg(lt, site["driver"][1], site["driver"][2])
    return seg(t, site["driver"][1], site["driver"][2])  # seg_t

def legs_walk(site, drv):
    rule = site["legs_rule"]
    if rule == "stand_if_ge1":
        return drv < 1.0
    if rule == "stand_if_k97":
        return drv <= 0.97
    return 0.0 < drv < 1.0  # walk_iff_mid

def classify(site, drv):
    if drv == 0.0:
        return "HEAD-STATIC"
    if drv == 1.0:
        return "TAIL-STATIC"
    return "ADVANCING" if legs_walk(site, drv) else "ADVANCING-STAND"

def merge_ranges(frames):
    if not frames:
        return []
    out = []
    a = b = frames[0]
    for n in frames[1:]:
        if n == b + 1:
            b = n
        else:
            out.append((a, b))
            a = b = n
    out.append((a, b))
    return out

def fmt_ranges(ranges):
    return ", ".join(f"{a}-{b}" if a != b else f"{a}" for a, b in ranges) or "-"

def swing_arms(ch):
    out = []
    for side in ("arm_l", "arm_r"):
        if ch[side][0] == "swing":
            out.append("L" if side == "arm_l" else "R")
    return out

# ------------------------------------------------------------------
# main
# ------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(description="Locomotion adoption census (see module docstring).")
    ap.add_argument("--repo", default=".", help="repo root (default: cwd)")
    args = ap.parse_args()
    repo = Path(args.repo)

    try:
        manifest = json.loads((repo / "scenes/film/film.json").read_text())
        film_data = json.loads((repo / "film/onoda.json").read_text())
        film_rs = (repo / "src/film.rs").read_text()
    except (OSError, json.JSONDecodeError) as e:
        print(f"ERROR: cannot load inputs: {e}", file=sys.stderr)
        return 2

    fps = manifest["fps"]
    total = manifest["totalFrames"]
    segments = {s["id"]: s for s in manifest["segments"]}

    # --- verify the manifest tiles [0, total) per its own frameRule
    expect = 0
    for s in manifest["segments"]:
        if s["startFrame"] != expect or s["endFrame"] - s["startFrame"] != s["frames"]:
            print(f"ERROR: manifest tiling broken at {s['id']}", file=sys.stderr)
            return 2
        if s["startFrame"] != math.ceil(s["startS"] * fps - 1e-9):
            print(f"ERROR: manifest startFrame != ceil(startS*fps) at {s['id']}", file=sys.stderr)
            return 2
        expect = s["endFrame"]
    if expect != total:
        print("ERROR: manifest segments do not tile [0, totalFrames)", file=sys.stderr)
        return 2

    # --- verify the site table against film.rs's real call sites
    sites = build_sites(film_data["anchors"])
    strides = sorted(float(m) for m in re.findall(r"walk_phase\([^,]+,\s*([0-9.]+)\)", film_rs))
    amps = sorted(float(m) for m in re.findall(r"walk_bob\(\w+,\s*([0-9.]+)\)", film_rs))
    if strides != sorted(s["stride"] for s in sites) or amps != sorted(s["bob_amp"] for s in sites):
        print("ERROR: site table drifted from film.rs call sites "
              f"(film.rs strides {strides} / amps {amps})", file=sys.stderr)
        return 2

    print("LOCOMOTION ADOPTION CENSUS — Onoda film, film lane (src/film.rs staging)")
    print("sources: scenes/film/film.json (segment table + frame rule) · film/onoda.json (anchors) ·")
    print("         src/film.rs site table transcribed at aa92cee · module src/locomotion.rs (W24)")
    print(f"VERIFY: manifest tiles [0, {total}) exactly — OK · site table vs film.rs "
          f"walk_phase/walk_bob call sites — OK (7 sites, strides {strides}, bob amps {amps})")
    print("classes: HEAD-STATIC driver=0 · ADVANCING 0<driver<1, legs Walk · ADVANCING-STAND (s02) "
          "phase advancing, legs Stand · TAIL-STATIC driver=1, legs Stand")
    print("changes: BOB = bob waveform replaced (every gait-staged frame) · SWING = counter-phase "
          "arm swing added (ADVANCING frames, free arms only)")
    print()

    shot_adv = {}        # shot -> set of ADVANCING frames (any char)
    shot_gait = {}       # shot -> set of all gait-staged frames
    shot_swing = {}      # shot -> set of ADVANCING frames with >= 1 swinging char
    shot_advstand = {}   # shot -> set of ADVANCING-STAND frames with no char ADVANCING
    char_adv_total = 0
    char_swing_total = 0

    order = []
    for site in sites:
        if site["shot"] not in order:
            order.append(site["shot"])
    by_shot = {sid: [s for s in sites if s["shot"] == sid] for sid in order}

    for sid in order:
        segm = segments[sid]
        d = segm["endS"] - segm["startS"]
        print(f"SHOT {sid} — segment frames {segm['startFrame']}-{segm['endFrame'] - 1} "
              f"({segm['frames']}), t {segm['startS']:.3f}-{segm['endS']:.3f}")
        adv_union, gait_union, swing_union, advstand_cand = set(), set(), set(), set()
        for site in by_shot[sid]:
            def num(v):
                return repr(round(v, 6))
            if site["driver"][0] == "camk":
                drv_desc = f"cam_k(lt, d, {num(site['driver'][1])})"
            elif site["driver"][0] == "seg_lt":
                drv_desc = f"seg(lt, {num(site['driver'][1])}, {num(site['driver'][2])})"
            else:
                drv_desc = f"seg(t, {num(site['driver'][1])}, {num(site['driver'][2])})  [film time]"
            print(f"  SITE {site['cite']}")
            print(f"    driver {drv_desc} · dist {site['dist']} · stride {site['stride']} · bobAmp {site['bob_amp']}")
            for ch in site["chars"]:
                classes = {}
                staged = 0
                for n in range(segm["startFrame"], segm["endFrame"]):
                    t = n / fps
                    lt = t - segm["startS"]
                    if site["branch"] and t >= site["branch"][1]:
                        continue  # s11 rifle-drop branch: no gait channels
                    drv = driver_value(site, lt, t, d)
                    classes.setdefault(classify(site, drv), []).append(n)
                    staged += 1
                adv = classes.get("ADVANCING", [])
                sw = swing_arms(ch)
                print(f"    {ch['name']}: gait-staged {staged} frames")
                for cls in ("HEAD-STATIC", "ADVANCING", "ADVANCING-STAND", "TAIL-STATIC"):
                    if cls in classes:
                        fr = classes[cls]
                        print(f"      {cls:<16} {fmt_ranges(merge_ranges(fr))} ({len(fr)})")
                chg = [f"BOB on all {staged} gait-staged frames"]
                if sw and adv:
                    chg.append(f"SWING ({'+'.join(sw)}) on {len(adv)} advancing frames")
                    char_swing_total += len(adv)
                elif sw:
                    chg.append(f"SWING ({'+'.join(sw)}) armed but no advancing frames")
                else:
                    why = ch["arm_l"][1] if ch["arm_l"][0] != "swing" else ch["arm_r"][1]
                    chg.append(f"no arm swing — {why}")
                print(f"      changes: {'; '.join(chg)}")
                print(f"      arms: {ch['base_note']}")
                char_adv_total += len(adv)
                adv_union |= set(adv)
                advstand_cand |= set(classes.get("ADVANCING-STAND", []))
                gait_union |= set(range(segm["startFrame"], segm["endFrame"])) if not site["branch"] \
                    else set(n for n in range(segm["startFrame"], segm["endFrame"]) if n / fps < site["branch"][1])
                if sw:
                    swing_union |= set(adv)
        advstand_union = advstand_cand - adv_union
        shot_adv[sid] = adv_union
        shot_gait[sid] = gait_union
        shot_swing[sid] = swing_union
        shot_advstand[sid] = advstand_union
        static_n = len(gait_union) - len(adv_union) - len(advstand_union)
        print(f"  SHOT TOTAL: walking (ADVANCING, any character) {len(adv_union)} frames "
              f"{fmt_ranges(merge_ranges(sorted(adv_union)))} · gait-staged {len(gait_union)} · "
              f"static-gait (bob-only) {static_n}"
              + (f" · advancing-stand (bob-only) {len(advstand_union)}" if advstand_union else ""))
        print()

    adv_all = set().union(*shot_adv.values())
    gait_all = set().union(*shot_gait.values())
    swing_all = set().union(*shot_swing.values())
    advstand_all = set().union(*shot_advstand.values())
    print("FILM TOTALS (of 1811 frames)")
    print(f"  walking frames (ADVANCING, >= 1 character): {len(adv_all)} "
          f"({100.0 * len(adv_all) / total:.1f}%)")
    print(f"  character-frames advancing: {char_adv_total}")
    print(f"  frames with arm-swing motion added (ADVANCING, >= 1 free arm): {len(swing_all)} "
          f"({100.0 * len(swing_all) / total:.1f}%) · character-frames swinging: {char_swing_total}")
    print(f"  gait-staged frames (bob value changes, incl. static): {len(gait_all)} "
          f"({100.0 * len(gait_all) / total:.1f}%) · of those: advancing-stand "
          f"{len(advstand_all)} (s02 slide tail) · static-gait (driver at an endpoint) "
          f"{len(gait_all) - len(adv_all) - len(advstand_all)}")
    print(f"  shots with walking: {', '.join(order)} · shots untouched by adoption: "
          f"{', '.join(s['id'] for s in manifest['segments'] if s['id'] not in order)}")

    # --- scene-file lane cross-check (instant states, not ranges)
    print()
    print("CROSS-CHECK — scene-file lane (scenes/film/*.scene.json carry an instant state, not a range):")
    for s in manifest["segments"]:
        p = repo / s["scene"]
        try:
            data = json.loads(p.read_text())
        except (OSError, json.JSONDecodeError):
            continue
        walks = 0
        stack = [data]
        while stack:
            node = stack.pop()
            if isinstance(node, dict):
                if node.get("legs") == "walk":
                    walks += 1
                stack.extend(node.values())
            elif isinstance(node, list):
                stack.extend(node)
        if walks or s["id"] in order:
            print(f"  {s['id']}: scene time {data.get('time')} · states with legs=walk: {walks} · "
                  f"film-lane walking frames: {len(shot_adv.get(s['id'], set()))}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
