#!/usr/bin/env python3
"""VRAM ledger registration completeness check (VRAM-IMPL-W84 §6.4).

WHAT THIS IS
    A stdlib-only static checker. The GPU memory ledger in
    src/gpucore.rs (the `gpu-mem:` line) is only honest if EVERY
    texture/buffer allocation in src/ is registered. This check
    enforces the escape-hatch discipline of W84 §5.2: every
    `create_texture` / `create_buffer` occurrence in src/*.rs
    must be either

      (a) inside a CHOKEPOINT body — a helper whose callers
          register per-caller (gpucore::make_target /
          make_target_viewed, composer::make_rt). The ledger
          note for those bytes lives at the call sites, so the
          body itself carries none; or

      (b) within WINDOW lines of a `mem_alloc_*` /
          `mem_note_readback` line (its own registration, in
          either direction — registrations follow their
          creation, except PostChain's class-C retained
          registration, which precedes bloom's make_rt calls
          and is itself inside a chokepoint caller).

    Anything else is a SILENT SITE and fails the check: an
    unregistered allocation would make `accounted_bytes` a
    lower bound that lies about being complete.

USAGE
    check_mem_registration.py [--root DIR] [--self-test]

    --root DIR   repo root to scan (default: the repo this
                 script lives in).
    --self-test  run the synthetic self-test and exit.

EXIT CODES (house convention)
    0  PASS — every site is a chokepoint or registered.
    1  FAIL — at least one silent site (named, with file:line).
    2  input error — bad arguments / unreadable tree.

LIMITS
    Line-based, comment-stripped (// only). It does not parse
    Rust; a creation hidden inside a macro or a string would
    evade it, and a registration mentioned only in a comment
    within the window would satisfy it spuriously. The tree
    has no such shapes today; the check guards the discipline
    as written, not every conceivable evasion.
"""

import os
import re
import sys

CREATE_RE = re.compile(r"\bcreate_(texture|buffer)\b")
REGISTER_RE = re.compile(r"\bmem_(alloc_texture|alloc_buffer|free_texture|free_buffer|note_readback)\b")
FN_RE = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")

# Helpers whose CALLERS register per-caller (W84 §5.2's staged
# decision). A create_* inside one of these bodies is covered
# by construction; the matching registrations sit at the call
# sites and are themselves checked as ordinary lines there.
CHOKEPOINT_FNS = {"make_target", "make_target_viewed", "make_rt"}

WINDOW_AFTER = 12   # registration lines follow their creation
WINDOW_BEFORE = 30  # PostChain's class-C registration precedes
                    # bloom's texture creations (via make_rt
                    # calls — not create_* sites — so this is
                    # headroom, not load-bearing today)


def strip_comment(line):
    """Remove a // comment (naive: this tree's create_* sites
    never sit inside string literals containing //)."""
    idx = line.find("//")
    return line if idx < 0 else line[:idx]


def scan_file(path):
    """Return (sites, registers): sites = [(lineno, kind,
    enclosing_fn)], registers = [lineno]."""
    with open(path, "r", encoding="utf-8") as fh:
        raw_lines = fh.read().splitlines()
    lines = [strip_comment(l) for l in raw_lines]
    sites = []
    registers = []
    enclosing_fn = None
    for i, line in enumerate(lines):
        lineno = i + 1
        m = FN_RE.search(line)
        if m:
            enclosing_fn = m.group(1)
        if REGISTER_RE.search(line):
            registers.append(lineno)
        for cm in CREATE_RE.finditer(line):
            sites.append((lineno, cm.group(1), enclosing_fn))
    return sites, registers


def check_root(root):
    src = os.path.join(root, "src")
    if not os.path.isdir(src):
        print(f"ERROR: no src/ under {root}", file=sys.stderr)
        return 2
    files = sorted(f for f in os.listdir(src) if f.endswith(".rs"))
    if not files:
        print(f"ERROR: no .rs files under {src}", file=sys.stderr)
        return 2
    total = 0
    silent = []
    for name in files:
        sites, registers = scan_file(os.path.join(src, name))
        regset = set(registers)
        for lineno, kind, fn in sites:
            total += 1
            if fn in CHOKEPOINT_FNS:
                continue
            lo = lineno - WINDOW_BEFORE
            hi = lineno + WINDOW_AFTER
            if any(lo <= r <= hi for r in regset):
                continue
            silent.append((name, lineno, kind, fn))
    for name, lineno, kind, fn in silent:
        print(
            f"SILENT SITE: src/{name}:{lineno} create_{kind} "
            f"(in fn {fn or '?'}) has no mem_alloc_*/mem_note_readback "
            f"within [{-WINDOW_BEFORE}, +{WINDOW_AFTER}] lines and is "
            f"not a chokepoint body — register it (VRAM-IMPL-W84 §5.2) "
            f"or route it through a registering helper"
        )
    if silent:
        print(f"MEM-REGISTRATION: FAIL ({len(silent)} silent of {total} sites)")
        return 1
    print(f"MEM-REGISTRATION: PASS ({total} create_* sites, all chokepoint or registered)")
    return 0


def self_test():
    import tempfile

    cases = []

    def write(root, name, text):
        with open(os.path.join(root, "src", name), "w", encoding="utf-8") as fh:
            fh.write(text)

    with tempfile.TemporaryDirectory() as root:
        os.makedirs(os.path.join(root, "src"))
        # 1. A registered site passes.
        write(root, "ok.rs",
              "fn f() {\n"
              "    let t = device.create_texture(&d);\n"
              "    crate::gpucore::mem_alloc_texture(C, bytes);\n"
              "}\n")
        # 2. A chokepoint body passes without its own note.
        write(root, "choke.rs",
              "pub(crate) fn make_target() -> T {\n"
              "    device.create_texture(&d)\n"
              "}\n")
        rc = check_root(root)
        cases.append(("registered + chokepoint pass", rc == 0))
        # 3. A silent site fails.
        write(root, "bad.rs",
              "fn g() {\n"
              "    let b = device.create_buffer(&d);\n"
              "}\n")
        rc = check_root(root)
        cases.append(("silent site fails", rc == 1))
        # 4. A commented-out creation is not a site.
        os.remove(os.path.join(root, "src", "bad.rs"))
        write(root, "comment.rs",
              "fn h() {\n"
              "    // let b = device.create_buffer(&d);\n"
              "}\n")
        rc = check_root(root)
        cases.append(("commented creation ignored", rc == 0))

    failed = [name for name, ok in cases if not ok]
    for name, ok in cases:
        print(f"  self-test: {name}: {'ok' if ok else 'FAILED'}")
    if failed:
        print(f"SELF-TEST: FAIL ({len(failed)} of {len(cases)})")
        return 1
    print(f"SELF-TEST: PASS ({len(cases)} checks)")
    return 0


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg == "--self-test":
            return self_test()
        if arg == "--root" and i + 1 < len(argv):
            root = argv[i + 1]
            i += 2
            continue
        print(f"ERROR: unknown or incomplete argument: {arg}", file=sys.stderr)
        return 2
    return check_root(root)


if __name__ == "__main__":
    sys.exit(main())
