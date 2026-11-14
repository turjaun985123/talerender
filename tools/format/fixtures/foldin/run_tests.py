#!/usr/bin/env python3
"""run_tests.py — self-test for tools/format/check_foldin.py.

Cases (the mini draft's census is headings + normative
sentences; the named-rule census is §15-specific and is turned
off here with --rules=off):

  1. folded-full.md     every census item FOUND, exit 0
  2. folded-altered.md  the dropped cycle clause is MISSING and
                        the weakened never->usually clause is
                        ALTERED-SUSPECT (strong keyword lost),
                        everything else FOUND, exit 1
  3. missing target     exit 2, one ERROR: line, no report

Prints PASS n/n; exit 0 iff all cases pass.
"""

import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True

HERE = Path(__file__).resolve().parent
TOOL = HERE.parent.parent / "check_foldin.py"
DRAFT = HERE / "mini-draft.md"


def run(target):
    p = subprocess.run(
        [sys.executable, str(TOOL), str(target),
         f"--draft={DRAFT}", "--rules=off"],
        capture_output=True, text=True)
    return p.returncode, p.stdout, p.stderr


def n_line(out, needle):
    for ln in out.splitlines():
        if needle in ln and ("FOUND" in ln or "ALTERED" in ln or "MISSING" in ln):
            return ln
    return ""


def main():
    results = []

    rc, out, _ = run(HERE / "folded-full.md")
    ok = (rc == 0 and "OVERALL: PASS" in out
          and "\n  MISSING" not in out and "\n  ALTERED-SUSPECT" not in out)
    results.append(("folded-full: all clauses FOUND, exit 0", ok))

    rc, out, _ = run(HERE / "folded-altered.md")
    cycle = n_line(out, "cycle is a hard error")
    weakened = n_line(out, "never carry")
    if not weakened:
        weakened = n_line(out, "Instances")
    others_found = ("hard-error, naming" in out and "FOUND" in n_line(out, "hard-error, naming"))
    ok = (rc == 1 and cycle.startswith("  MISSING")
          and "ALTERED-SUSPECT" in weakened
          and "strong keyword lost: NEVER" in weakened
          and others_found)
    results.append(("folded-altered: dropped=MISSING, weakened=ALTERED-SUSPECT, exit 1", ok))

    p = subprocess.run(
        [sys.executable, str(TOOL), str(HERE / "does-not-exist.md"),
         f"--draft={DRAFT}"],
        capture_output=True, text=True)
    ok = (p.returncode == 2 and p.stderr.startswith("ERROR: check_foldin.py:")
          and "OVERALL" not in p.stdout)
    results.append(("missing target: exit 2, ERROR line, no report", ok))

    for name, ok in results:
        print(f"  {'PASS' if ok else 'FAIL'}  {name}")
    n = sum(1 for _, ok in results if ok)
    print(f"SELF-TEST: {'PASS' if n == len(results) else 'FAIL'} {n}/{len(results)}")
    return 0 if n == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
