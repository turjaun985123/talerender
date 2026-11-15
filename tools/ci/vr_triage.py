#!/usr/bin/env python3
"""Visual-regression triage — post-pin debt register check.

WHAT THIS IS
    tools/ci/visual_regression.py renders the pinned references
    fresh and fails on same-renderer drift. Since the s28
    retirement (2026-11-15, user ruling) a dispatch is expected
    to PASS outright — no drift debt remains on the register —
    but a red job still needs a second reading: which rows are
    the by-design states we already know about, and which rows —
    if any — are new information?
    This tool is that second reading. It parses a
    visual_regression.py report (the markdown the workflow
    uploads as the `visual-regression-<run_id>` artifact) and
    classifies every non-MATCH row as KNOWN-DEBT or NEW-SIGNAL
    against the register below.

THE REGISTER — the post-pin steady state
    golden-hashes.json pins all 17 items (make_golden verifies
    17 OK / 0 STALE / 0 MISSING). Both pin acts and the Act-1
    corrective have landed: Act 1 916a109 (the 14 film
    first-pins), Act 2 166873c (ab1-composer re-pinned
    d7bd71a0 -> 60c32c1a), corrective 2755ff3 (11 film pins
    moved to the f021991 candidates, after the canary caught
    the first act pinning staged 381ff47-era bytes). The
    workflow renders a canary set only: the ab1 render
    (checked against ab1-composer), dof-chart, and film frame
    1231 (film-s08b). Expected dispatch (post-retirement form
    of the state first measured on run 43625139818 at 2755ff3:
    3 MATCH · 0 DRIFT · 14 NOT-RENDERED):

      ab1-composer    MATCH        re-pinned in Act 2; only
                                   MATCH is acceptable — it is
                                   deliberately absent from
                                   REGISTER, like the control,
                                   so any non-MATCH row is a
                                   NEW-SIGNAL
      film-s08b       MATCH        the one film mid the
                                   workflow renders; same
                                   treatment as ab1-composer
      dof-chart       MATCH        the clean control — its
                                   chart texture never passes
                                   through shade_rgb; anything
                                   but MATCH here is real drift
      (s28-onoda-t150 RETIRED 2026-11-15 — see CLEARING
                      ACTIONS; it no longer appears in reports)
      film-* (13)     NOT-RENDERED pinned in Act 1 but
                                   rendered nowhere — the
                                   workflow's canary set is by
                                   design; the register entry
                                   covers this ONE status only
                                   (see the anti-whitelist rule)
      stroke-toon     NOT-RENDERED the workflow renders no
                                   battery card set (different
                                   output contract, W16 §4)

    THE ANTI-WHITELIST RULE. A register entry names the exact
    statuses it covers; every other non-MATCH status on that
    item is a NEW-SIGNAL ("registered debt item in an
    unregistered state"). The film entries cover NOT-RENDERED
    and nothing else: the day a non-canary film mid actually
    renders and drifts, its DRIFT row is a NEW-SIGNAL, not
    debt. MISSING-RENDER and REFERENCE-MISSING are always
    NEW-SIGNAL (run / repo breakage is never debt), any
    non-MATCH on the dof-chart control is a NEW-SIGNAL, an
    unknown item id is a NEW-SIGNAL, and any registry item
    missing from the report altogether is a NEW-SIGNAL.

CLEARING ACTIONS — the only ways debt leaves the register
    * s28-onoda-t150 — DISPOSITION RECORDED: RETIRED
      2026-11-15 by user ruling (approving the DECISIONS-W94
      recommendation; re-pinning was rejected because VR fed
      the pin the same render as ab1-composer, so it guarded
      nothing new). The pin left golden-hashes.json, the item
      left make_golden.py's ITEMS and this register, and the
      workflow no longer renders it. The historical artefact
      stays in the repo at docs/ab/onoda-s01-t150/native.png;
      it is no longer a regression reference. Recorded per
      docs/golden/NATIVE-GOLDEN.md.
    * stroke-toon NOT-RENDERED clears with a future deliberate
      act: re-baseline the founding pin and add a battery
      render to the workflow (W16 §4).
    * The 13 film NOT-RENDERED entries clear if the workflow's
      render set is deliberately extended to those mids (MATCH
      then expected; the entries leave REGISTER in the same
      commit) — or never: the entries record a permanent
      design fact, not a defect awaiting a fix.
    NOTHING here widens the 80 dB band and NOTHING whitelists
    a pin. The band is policy (changing it is a commit-message
    act in visual_regression.py), and whitelisting is the one
    move every document in this chain forbids. This tool
    classifies; it cannot silence.

REGISTER MAINTENANCE
    The register describes the repo state AFTER the pin acts
    (916a109 / 166873c / 2755ff3). When the state moves again
    — an s28 disposition, a render-set change, a new pin —
    update REGISTER in the same commit. Triage against a stale
    register manufactures NEW-SIGNALs: the pre-pin register
    printed 13 false NEW-SIGNALs (the designed NOT-RENDERED
    film rows) on the healthy post-corrective run 43625139818.

USAGE
    python3 tools/ci/vr_triage.py visual-regression.md
    python3 tools/ci/vr_triage.py - < visual-regression.md
    python3 tools/ci/vr_triage.py --self-test

EXIT CODES (same discipline as the runner)
    0   TRIAGE VERDICT: result is AS EXPECTED
    1   TRIAGE VERDICT: contains NEW SIGNALS: ...
    2   input error — one ERROR line on stderr, NO verdict line
        (an unparseable report is no evidence, never a soft pass)
"""

import argparse
import re
import sys
from pathlib import Path

STATUSES = {"MATCH", "WITHIN-BAND", "DRIFT", "MISSING-RENDER",
            "REFERENCE-MISSING", "UNPINNED", "NOT-RENDERED"}

# ---------------------------------------------------------------- register
# id -> (expected statuses, clearing action). MATCH is never listed:
# a MATCH row needs no triage on any item.
#
# POST-PIN REGISTER (VR-REGISTER-W72). The pre-pin register this
# replaces described the repo BEFORE any pin act: 4 milestone pins
# (two of them stale, pre-c00897d) and 14 unpinned film mids. Both
# acts and the Act-1 corrective have landed:
#   Act 1            916a109  14 film first-pins struck
#   Act 2            166873c  ab1-composer re-pinned
#                             d7bd71a0 -> 60c32c1a (fresh render
#                             at the integration head)
#   Act 1 corrective 2755ff3  11 film pins moved to the f021991
#                             candidates — the VR canary caught
#                             the first act pinning staged
#                             381ff47-era bytes (film-s08b DRIFT
#                             24.85 dB, run 43624398371)
# golden-hashes.json now pins all 17 items; make_golden verifies
# 17 OK / 0 STALE / 0 MISSING (the count was 18 until the s28
# retirement, 2026-11-15 — see the docstring's CLEARING ACTIONS).
#
# The workflow renders a canary set ONLY (.github/workflows/
# visual-regression.yml, by design): the ab1 render — checked
# against ab1-composer — dof-chart, and film frame 1231
# (film-s08b). Every other pinned item reports NOT-RENDERED on
# every dispatch. Steady state, first measured on run
# 43625139818 at 2755ff3 (with s28 then the sole DRIFT; retired
# since): 3 MATCH (ab1-composer, dof-chart, film-s08b — all
# byte-identical) · 0 DRIFT · 14 NOT-RENDERED. Expected classes:
#   ab1-composer    MATCH only — deliberately NOT in REGISTER,
#                   like the dof-chart control: any non-MATCH
#                   row is a NEW-SIGNAL
#   film-s08b       MATCH only — the one film mid rendered;
#                   same treatment as ab1-composer
#   film-* (13)     NOT-RENDERED = expected by design; the
#                   entry covers that ONE status only (see the
#                   anti-whitelist rule in the docstring)
#   stroke-toon     NOT-RENDERED = KNOWN-DEBT (unchanged)
#   dof-chart       MATCH only — the clean control (CONTROL_ID)

FILM_IDS = ["film-s01", "film-s02", "film-s03", "film-s04",
            "film-s05a", "film-s05b", "film-s06", "film-s07",
            "film-s08", "film-s08b", "film-s09", "film-s10",
            "film-s11", "film-s12"]

# The one film mid the workflow renders (frame 1231, the s08b
# mid — "the highest-value film canary per frame rendered",
# visual-regression.yml). It is EXPECTED to render, so — like
# ab1-composer and the dof-chart control — it carries NO
# register entry: only MATCH is acceptable, and any non-MATCH
# row (DRIFT, WITHIN-BAND, or NOT-RENDERED because the render
# set shrank) falls through to a NEW-SIGNAL.
FILM_CANARY_ID = "film-s08b"

FILM_NOT_RENDERED_ACTION = (
    "NOT-RENDERED by design: pinned in GOLDEN-RESTAGE-W03 "
    "Act 1 (916a109; corrective 2755ff3) and covered by the "
    "make_golden pack verification (17 OK / 0 STALE / 0 "
    "MISSING), but the visual-regression workflow renders a "
    "canary set only — the ab1 render (one pin), dof-chart, "
    "and film-s08b (frame 1231) — so this mid renders on no "
    "dispatch. This entry covers NOT-RENDERED ONLY: if the "
    "render set is deliberately extended to this mid, MATCH "
    "is the expectation, and a DRIFT / WITHIN-BAND / UNPINNED "
    "row here is a NEW-SIGNAL, never debt.")

REGISTER = {
    "stroke-toon": (
        {"NOT-RENDERED"},
        "Expected: the workflow renders no battery card set "
        "(different output contract). Clears with a future "
        "deliberate act — re-baseline the founding pin and add a "
        "battery render to the workflow (VISUAL-REGRESSION-W16 "
        "§4)."),
}

for _fid in FILM_IDS:
    if _fid != FILM_CANARY_ID:
        REGISTER[_fid] = ({"NOT-RENDERED"},
                          FILM_NOT_RENDERED_ACTION)

# The clean control: only MATCH is expected; it is deliberately
# NOT in REGISTER, so every non-MATCH row lands as NEW-SIGNAL
# with the control-specific follow-up below.
CONTROL_ID = "dof-chart"

# Every id the report must carry (make_golden.py ITEMS order;
# s28-onoda-t150 retired 2026-11-15 — see CLEARING ACTIONS).
EXPECTED_IDS = (["dof-chart", "stroke-toon",
                 "ab1-composer"] + FILM_IDS)

NEW_SIGNAL_FOLLOWUP = {
    "DRIFT": ("Investigate before anything else: same-renderer "
              "drift on an item carrying no re-pin debt. Bisect "
              "the renderer changes since the pin was struck; do "
              "NOT re-pin to silence it (that would hide a "
              "regression) and do NOT widen the band."),
    "WITHIN-BAND": ("Byte-identity is the post-cabcc30 norm; a "
                    "WITHIN-BAND row here wants its explanation "
                    "found (re-encode artefact vs real sub-band "
                    "change) before the run is accepted."),
    "MISSING-RENDER": ("The render step produced no file — run "
                       "breakage, not drift. Check the workflow's "
                       "render step log and the visual-out "
                       "artifact; fix the run, then re-dispatch."),
    "REFERENCE-MISSING": ("A pinned reference file is gone from "
                          "the repo — repo breakage. Restore the "
                          "file or account for its removal in a "
                          "commit; never re-pin around a missing "
                          "reference."),
    "NOT-RENDERED": ("A pinned item the dispatch was expected to "
                     "render has no render — the workflow's "
                     "render set or the runner invocation changed. "
                     "Reconcile .github/workflows/"
                     "visual-regression.yml deliberately."),
    "UNPINNED": ("A currently pinned item reports UNPINNED — the "
                 "pin state moved outside the recorded acts. "
                 "Reconcile golden-hashes.json history before "
                 "trusting any other row."),
}


class InputError(Exception):
    """Unparseable input. One named line on stderr, exit 2, no
    verdict (house convention, same as the runner)."""


# ---------------------------------------------------------------- parsing

VERDICT_RE = re.compile(
    r"^VISUAL REGRESSION: (PASS|FAIL) \((\d+) pins, (\d+) drift\)\s*$",
    re.M)


def parse_report(text: str):
    """visual_regression.py markdown -> (runner_verdict, rows).

    Rows are dicts {id, status, psnr, note} in report order.
    Anything structurally off is an InputError: a truncated or
    foreign file must never triage as a quiet pass.
    """
    m = VERDICT_RE.search(text)
    if not m:
        raise InputError("no 'VISUAL REGRESSION: PASS/FAIL "
                         "(n pins, m drift)' verdict line found — "
                         "not a complete visual_regression.py "
                         "report (or the runner refused, exit 2, "
                         "and there is nothing to triage)")
    runner_verdict = m.group(0).strip()

    rows = []
    for line in text.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) < 9 or cells[0] == "pin" or \
                set(cells[0]) <= {"-", " "}:
            continue  # header or separator row
        rows.append({"id": cells[0], "status": cells[1],
                     "psnr": cells[2],
                     "note": "|".join(cells[8:])})
    if not rows:
        raise InputError("no per-pin table rows found — not a "
                         "visual_regression.py report")
    return runner_verdict, rows


# ---------------------------------------------------------------- triage

def classify(row):
    """One report row -> (triage, follow-up).

    triage is OK (MATCH — no action), KNOWN-DEBT (registered,
    with its clearing action) or NEW-SIGNAL (human attention,
    with its investigation step).
    """
    item_id, status = row["id"], row["status"]
    if status == "MATCH":
        return "OK", "none — byte-identical to the pin"
    if status not in STATUSES:
        return "NEW-SIGNAL", (f"unrecognised status '{status}' — "
                              f"the report format moved; update "
                              f"this parser deliberately, in a "
                              f"commit, before trusting the run")
    if status in ("MISSING-RENDER", "REFERENCE-MISSING"):
        return "NEW-SIGNAL", NEW_SIGNAL_FOLLOWUP[status]
    if item_id == CONTROL_ID:
        return "NEW-SIGNAL", (
            "dof-chart is the clean control (its chart texture "
            "never passes through shade_rgb): only MATCH is "
            "expected. " + NEW_SIGNAL_FOLLOWUP[status])
    if item_id in REGISTER:
        expected, action = REGISTER[item_id]
        if status in expected:
            return "KNOWN-DEBT", action
        return "NEW-SIGNAL", (
            f"registered debt item in an unregistered state "
            f"(expected {'/'.join(sorted(expected))}) — the pin "
            f"state or the render set moved since the register "
            f"was written. Reconcile the register first; then: "
            + NEW_SIGNAL_FOLLOWUP[status])
    if item_id in EXPECTED_IDS:
        return "NEW-SIGNAL", NEW_SIGNAL_FOLLOWUP[status]
    return "NEW-SIGNAL", (f"item id not in the golden registry "
                          f"snapshot this tool carries — the "
                          f"registry changed; update the register "
                          f"deliberately, in a commit")


def triage_report(runner_verdict, rows):
    """(runner_verdict, rows) -> (report_text, n_new_signals)."""
    triaged = []
    seen = set()
    for row in rows:
        triage, followup = classify(row)
        triaged.append((row, triage, followup))
        seen.add(row["id"])
    for missing in EXPECTED_IDS:
        if missing not in seen:
            row = {"id": missing, "status": "(absent)",
                   "psnr": "—", "note": ""}
            triaged.append((row, "NEW-SIGNAL",
                            "registry item missing from the "
                            "report — the artifact is truncated "
                            "or the registry moved; a partial "
                            "report triages nothing"))

    n_ok = sum(1 for _, t, _ in triaged if t == "OK")
    n_debt = sum(1 for _, t, _ in triaged if t == "KNOWN-DEBT")
    signals = [(r["id"], r["status"]) for r, t, _ in triaged
               if t == "NEW-SIGNAL"]
    # A registered debt item reporting MATCH is not a signal —
    # but it means the debt state moved, and the register must
    # be told. Informational only; never changes the verdict.
    cleared = [r["id"] for r, _, _ in triaged
               if r["id"] in REGISTER and r["status"] == "MATCH"]

    lines = [
        "# Visual-regression triage — post-pin register",
        "",
        f"Runner verdict: {runner_verdict}",
        "Band: 80 dB — unchanged. Nothing in this triage widens "
        "the band or whitelists a pin; KNOWN-DEBT rows clear only "
        "through the pin acts named in their follow-ups "
        "(docs/integration/VR-DISPATCH-W37.md).",
        "",
        "| pin | runner status | PSNR (dB) | triage | clearing "
        "action / follow-up |",
        "|---|---|---|---|---|",
    ]
    for row, triage, followup in triaged:
        lines.append(f"| {row['id']} | {row['status']} | "
                     f"{row['psnr']} | {triage} | {followup} |")
    lines += ["",
              f"Counts: {n_ok} OK · {n_debt} KNOWN-DEBT · "
              f"{len(signals)} NEW-SIGNAL "
              f"(of {len(triaged)} rows)."]
    for item_id in cleared:
        lines.append(f"Register note: {item_id} reports MATCH — "
                     f"its registered debt appears already "
                     f"cleared. Confirm the clearing act landed "
                     f"and update REGISTER in that commit; a "
                     f"dispatch that passes outright is "
                     f"this note's story, not a silent surprise.")
    lines.append("")
    if signals:
        named = "; ".join(f"{i} ({s})" for i, s in signals)
        lines.append(f"TRIAGE VERDICT: contains NEW SIGNALS: "
                     f"{named}")
    else:
        lines.append("TRIAGE VERDICT: result is "
                     "AS EXPECTED")
    lines.append("")
    return "\n".join(lines), len(signals)


# ---------------------------------------------------------------- self-test

FIXTURE_DIR = Path(__file__).resolve().parent / "fixtures" / \
    "vr_triage"

# (fixture, must_contain_in_verdict, expected_exit)
SELF_TESTS = [
    ("expected-steady-state.md", "AS EXPECTED", 0),
    ("planted-new-drift.md", "dof-chart", 1),
    ("planted-missing-render.md", "dof-chart", 1),
    ("planted-noncanary-drift.md", "film-s03", 1),
]


def run_self_test() -> int:
    failures = 0
    for name, marker, want_exit in SELF_TESTS:
        path = FIXTURE_DIR / name
        print(f"===== self-test: {name} =====")
        try:
            text = path.read_text()
        except OSError as e:
            print(f"ERROR: vr_triage.py: cannot read fixture "
                  f"{path} ({e})", file=sys.stderr)
            return 2
        runner_verdict, rows = parse_report(text)
        report, n_signals = triage_report(runner_verdict, rows)
        print(report)
        got_exit = 1 if n_signals else 0
        verdict_line = [ln for ln in report.splitlines()
                        if ln.startswith("TRIAGE VERDICT:")]
        ok = (got_exit == want_exit and verdict_line and
              marker in verdict_line[0] and
              (want_exit == 0 or "NEW SIGNALS" in
               verdict_line[0]))
        print(f"----- {name}: exit {got_exit} "
              f"(expected {want_exit}), verdict names "
              f"'{marker}': {'PASS' if ok else 'FAIL'} -----\n")
        failures += 0 if ok else 1
    if failures:
        print(f"SELF-TEST: FAIL ({failures} of {len(SELF_TESTS)} "
              f"fixtures mis-triaged)")
        return 1
    print(f"SELF-TEST: PASS ({len(SELF_TESTS)}/{len(SELF_TESTS)} "
          f"fixtures triaged as designed)")
    return 0


# ---------------------------------------------------------------- main

def main() -> int:
    ap = argparse.ArgumentParser(
        description="Triage a visual_regression.py report "
                    "against the post-pin re-pin debt "
                    "register. See the module docstring for the "
                    "register, the clearing actions and the exit "
                    "codes.",
        formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("report", nargs="?", default="-",
                    help="visual-regression.md report file "
                         "(default: '-' = stdin)")
    ap.add_argument("--self-test", action="store_true",
                    help="triage the synthetic fixtures in "
                         "tools/ci/fixtures/vr_triage/ and assert "
                         "the designed verdicts")
    args = ap.parse_args()

    if args.self_test:
        return run_self_test()

    try:
        if args.report == "-":
            text = sys.stdin.read()
        else:
            try:
                text = Path(args.report).read_text()
            except OSError as e:
                raise InputError(f"cannot read report "
                                 f"{args.report} ({e})")
        runner_verdict, rows = parse_report(text)
    except InputError as e:
        print(f"ERROR: vr_triage.py: {e}", file=sys.stderr)
        return 2

    report, n_signals = triage_report(runner_verdict, rows)
    print(report)
    return 1 if n_signals else 0


if __name__ == "__main__":
    sys.exit(main())
