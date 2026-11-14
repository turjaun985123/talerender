#!/usr/bin/env python3
"""Visual-regression triage — first-dispatch debt register check.

WHAT THIS IS
    tools/ci/visual_regression.py renders the pinned references
    fresh and fails on same-renderer drift. Its FIRST dispatch is
    expected to FAIL — on exactly the known re-pin debt, not on
    new drift (VISUAL-REGRESSION-W16.md §4; the workflow header
    says the same at the point of failure). A red job therefore
    needs a second reading: which rows are the debt we already
    know about, and which rows — if any — are new information?
    This tool is that second reading. It parses a
    visual_regression.py report (the markdown the workflow
    uploads as the `visual-regression-<run_id>` artifact) and
    classifies every non-MATCH row as KNOWN-DEBT or NEW-SIGNAL
    against the register below.

THE REGISTER — a snapshot of the pre-pin repo state
    golden-hashes.json pins exactly 4 items (s28-onoda-t150,
    dof-chart, stroke-toon, ab1-composer); the 14 film mids are
    staged candidates, unpinned. The workflow renders
    ab1-composer, s28-onoda-t150 (both from the one ab1 render),
    dof-chart, and film-s08b. Expected first dispatch:

      ab1-composer    DRIFT        pin predates the shade
                                   root-fix c00897d (EXPECTED
                                   STALE, "certain" —
                                   tools/golden/RUNNER.md)
      s28-onoda-t150  DRIFT        same pre-c00897d debt
      dof-chart       MATCH        the clean control — its chart
                                   texture never passes through
                                   shade_rgb; anything but MATCH
                                   here is real drift
      stroke-toon     NOT-RENDERED the workflow renders no
                                   battery card set (different
                                   output contract, W16 §4)
      film-* (14)     UNPINNED     staged candidates are not
                                   references; reported only

    For the two stale pins the debt is "pin predates c00897d", so
    WITHIN-BAND is the same debt in a milder form (bytes differ
    from a pre-fix pin) and is registered alongside DRIFT. Every
    other non-MATCH row is a NEW-SIGNAL: MISSING-RENDER and
    REFERENCE-MISSING always (run / repo breakage is never debt),
    any non-MATCH on the dof-chart control, any registered item
    in an unregistered state, any unknown item id, and any
    registry item missing from the report altogether.

CLEARING ACTIONS — the only ways debt leaves the register
    * film-* UNPINNED clears with GOLDEN-RESTAGE-W03.md Act 1:
      human first-pins of the 14 film mids from
      docs/golden/frames-f021991/ (one commit; exactly 14 pin
      lines added; the 4 milestone pins unmoved).
    * ab1-composer clears with Act 2: the human re-pin, its own
      commit, diff scoped to the one ab1-composer pin line
      (packet: docs/golden/repin-packets/ab1-composer.md).
    * s28-onoda-t150 is cleared by NEITHER act — the repin packet
      recommends against re-pinning it (historical prototype
      artefact). It clears only with a separate human disposition
      of that pin (a deliberate re-pin anyway, or retiring the
      pin), recorded per docs/golden/NATIVE-GOLDEN.md. Expect the
      job to stay red on s28 alone after Acts 1-2 land; that is
      this debt ageing, not a new regression.
    * stroke-toon NOT-RENDERED clears with a future deliberate
      act: re-baseline the founding pin and add a battery render
      to the workflow (W16 §4).
    NOTHING here widens the 80 dB band and NOTHING whitelists a
    pin. The band is policy (changing it is a commit-message act
    in visual_regression.py), and whitelisting is the one move
    every document in this chain forbids. This tool classifies;
    it cannot silence.

REGISTER MAINTENANCE
    The register describes the repo state BEFORE any pin act.
    When an act lands, update REGISTER in the same commit —
    triage against a stale register manufactures NEW-SIGNALs.
    Post-act expectations: Act 1 -> film-s08b MATCH (it is the
    one film mid this workflow renders), the other film-* rows
    become NOT-RENDERED when rendered nowhere / stay UNPINNED
    only until their pin exists; Act 2 -> ab1-composer MATCH.

USAGE
    python3 tools/ci/vr_triage.py visual-regression.md
    python3 tools/ci/vr_triage.py - < visual-regression.md
    python3 tools/ci/vr_triage.py --self-test

EXIT CODES (same discipline as the runner)
    0   TRIAGE VERDICT: first-dispatch result is AS EXPECTED
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

FILM_IDS = ["film-s01", "film-s02", "film-s03", "film-s04",
            "film-s05a", "film-s05b", "film-s06", "film-s07",
            "film-s08", "film-s08b", "film-s09", "film-s10",
            "film-s11", "film-s12"]

FILM_ACTION = ("Clears with GOLDEN-RESTAGE-W03 Act 1 — human "
               "first-pins of the 14 film mids from "
               "docs/golden/frames-f021991/ (one commit, exactly "
               "14 pin lines added, milestone pins unmoved). "
               "Until pinned, UNPINNED is reported-only by design "
               "and gates nothing.")

REGISTER = {
    "ab1-composer": (
        {"DRIFT", "WITHIN-BAND"},
        "Stale pin: predates the shade root-fix c00897d "
        "(EXPECTED STALE, tools/golden/RUNNER.md). Clears with "
        "GOLDEN-RESTAGE-W03 Act 2 — human re-pin per "
        "docs/golden/repin-packets/ab1-composer.md, its own "
        "commit, diff scoped to the one ab1-composer pin line."),
    "s28-onoda-t150": (
        {"DRIFT", "WITHIN-BAND"},
        "Stale pin: predates c00897d, same debt as ab1-composer — "
        "but cleared by NEITHER W03 act (the repin packet "
        "recommends against re-pinning this historical prototype "
        "artefact). Clears only with a separate human disposition "
        "of the s28 pin (deliberate re-pin or pin retirement, "
        "recorded per NATIVE-GOLDEN.md). Expect this row to keep "
        "failing after Acts 1-2 land."),
    "stroke-toon": (
        {"NOT-RENDERED"},
        "Expected: the workflow renders no battery card set "
        "(different output contract). Clears with a future "
        "deliberate act — re-baseline the founding pin and add a "
        "battery render to the workflow (VISUAL-REGRESSION-W16 "
        "§4)."),
}

for _fid in FILM_IDS:
    REGISTER[_fid] = ({"UNPINNED"}, FILM_ACTION)

# The clean control: only MATCH is expected; it is deliberately
# NOT in REGISTER, so every non-MATCH row lands as NEW-SIGNAL
# with the control-specific follow-up below.
CONTROL_ID = "dof-chart"

# Every id the report must carry (make_golden.py ITEMS order).
EXPECTED_IDS = (["s28-onoda-t150", "dof-chart", "stroke-toon",
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
        "# Visual-regression triage — first dispatch",
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
                     f"first dispatch that passes outright is "
                     f"this note's story, not a silent surprise.")
    lines.append("")
    if signals:
        named = "; ".join(f"{i} ({s})" for i, s in signals)
        lines.append(f"TRIAGE VERDICT: contains NEW SIGNALS: "
                     f"{named}")
    else:
        lines.append("TRIAGE VERDICT: first-dispatch result is "
                     "AS EXPECTED")
    lines.append("")
    return "\n".join(lines), len(signals)


# ---------------------------------------------------------------- self-test

FIXTURE_DIR = Path(__file__).resolve().parent / "fixtures" / \
    "vr_triage"

# (fixture, must_contain_in_verdict, expected_exit)
SELF_TESTS = [
    ("expected-first-dispatch.md", "AS EXPECTED", 0),
    ("planted-new-drift.md", "dof-chart", 1),
    ("planted-missing-render.md", "dof-chart", 1),
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
                    "against the first-dispatch re-pin debt "
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
