# VR-REGISTER-W72 — vr_triage debt-register update pack (post-pin)

**Seat:** Wave 2, Batch 8, W72. **Scope:** the debt register in
`tools/ci/vr_triage.py` still describes the pre-pin repo; this pack
is the updated register in patch-ready form, plus the verification
that it classifies the post-pin steady state correctly. New file
only; the tool is **not** edited here — the lead applies §3 in one
commit (per W37 §6's same-commit rule, and GATE2-W70 §12 item 3d,
which queues exactly this update).

**Where the register lives:** in **code constants** inside
`tools/ci/vr_triage.py` — the module docstring's register
narrative (lines 18–81) and the `REGISTER` / `FILM_ACTION` block
(lines 103–154) — not in a data file the tool reads. The patch is
therefore §3's fenced diffs, not a replacement data file.
`classify()` itself needs **no change**: every new behaviour below
falls out of the existing routing (registered status → KNOWN-DEBT;
registered item in any other non-MATCH status → NEW-SIGNAL;
unregistered pinned item → NEW-SIGNAL; MATCH → OK).

## 1. Why the register is stale (evidence)

The register expects the first dispatch's re-pin debt: 14 film
`UNPINNED`, `ab1-composer` + `s28` DRIFT on pre-`c00897d` pins.
That world ended with the pin acts. Run against the
post-corrective report (run `43625139818`, head `2755ff3`,
artifact fetched by this seat via `gh run download`), the
**current** tool prints:

- Counts: **3 OK · 2 KNOWN-DEBT · 13 NEW-SIGNAL**, exit 1 — the
  13 NEW-SIGNALs are exactly the non-canary film mids reporting
  `NOT-RENDERED`, which the register reads as "registered debt
  item in an unregistered state (expected UNPINNED)".
- Two spurious `Register note:` lines (`ab1-composer` and
  `film-s08b` "debt appears already cleared") — the cleared-debt
  tripwire firing on acts that landed long ago.

W63 §6 recorded the same output and named the cause: the 13 rows
print as NEW-SIGNAL "solely from the pre-pin register". The
renders are not missing: `.github/workflows/visual-regression.yml`
renders a **canary set only, by design** — the ab1 render (checked
against `ab1-composer` *and* `s28-onoda-t150`), `dof-chart`, and
film frame 1231 (`film-s08b`) — so the other 13 pinned mids report
`NOT-RENDERED` on every dispatch, forever, until the render set is
deliberately extended.

## 2. The updated register — every entry, with its justification

Repo facts at HEAD `b2e46d2`: `tools/golden/golden-hashes.json`
pins **all 18 items**; the pack verifies 18 OK / 0 STALE /
0 MISSING (W63, re-confirmed at GATE2-W70).

| Item | New expected class | Change vs old register | Justification (citation) |
|---|---|---|---|
| `ab1-composer` | **MATCH only** — removed from `REGISTER` | was `{DRIFT, WITHIN-BAND}` KNOWN-DEBT | Act 2 re-pin `166873c`: `d7bd71a0` → `60c32c1a`, a fresh render at the integration head; run `43625139818` renders it byte-identical (MATCH). With the debt cleared it takes the control's treatment: any non-MATCH row falls through to NEW-SIGNAL. Follows W37 §6 verbatim ("after Act 2: `ab1-composer` leaves the register"). |
| `film-s08b` | **MATCH only** — no register entry (`FILM_CANARY_ID`) | was `{UNPINNED}` KNOWN-DEBT | The one film mid the workflow renders (frame 1231). Act 1 `916a109` pinned it; the corrective `2755ff3` moved its pin to `fb4e2ba4…` after the canary caught the staged-bytes error (DRIFT 24.85 dB, run `43624398371`); run `43625139818` renders it byte-identical. Follows W37 §6 verbatim ("after Act 1: `film-s08b` → expect MATCH"). |
| `film-s01…s12` (13 ids, excl. s08b) | **`NOT-RENDERED` = KNOWN-DEBT** ("NOT-RENDERED by design") | was `{UNPINNED}` KNOWN-DEBT | Pinned in Act 1 `916a109`, corrected in `2755ff3` (10 pin lines; s10 the identical-bytes control). The workflow renders none of them, by design (§1), so `NOT-RENDERED` is their permanent healthy state. The entry covers **that one status only** — §4. Diverges from W37 §6's sketch for these ids; justified in §5. |
| `s28-onoda-t150` | **`{DRIFT, WITHIN-BAND}` = KNOWN-DEBT** — the sole expected red | entry kept; action text rewritten | Pin `ff68fad3…` (file `docs/ab/onoda-s01-t150/native.png`) predates `c00897d` and was touched by **neither** act **nor** the corrective (verified in all three diffs, W63 §7). VR supplies the ab1 render as its candidate, so its DRIFT (21.65 dB, runs `43624398371` + `43625139818`, numerically identical) measures current output vs the historical prototype. Clears only with the separate human disposition — unchanged. |
| `stroke-toon` | **`{NOT-RENDERED}` = KNOWN-DEBT** | unchanged | The workflow still renders no battery card set (different output contract, W16 §4). No act has touched this state. |
| `dof-chart` | **MATCH only** — the clean control (`CONTROL_ID`, never in `REGISTER`) | unchanged | Rendered byte-identical on run `43625139818`. Its chart texture never passes through `shade_rgb`; any non-MATCH remains NEW-SIGNAL with the control follow-up. |

Steady state under the new register (the healthy post-pin
dispatch): **3 OK** (ab1-composer, dof-chart, film-s08b — all
MATCH) · **15 KNOWN-DEBT** (s28 DRIFT; stroke-toon + 13 film
NOT-RENDERED) · **0 NEW-SIGNAL** → triage verdict AS EXPECTED,
exit 0 — while the *runner* verdict stays `FAIL (4 pins, 1 drift)`
on s28 alone, exactly the end-state W37 predicted.

## 3. The patch (apply to `tools/ci/vr_triage.py`, one commit)

Line numbers are at HEAD `b2e46d2`. Hunks A and B are real
`diff -u` output generated against the current file; the `+` side
of Hunk B is byte-identical to the block this seat verified in §6.

### Hunk A — docstring register narrative (replaces lines 18–81)

```diff
--- a/tools/ci/vr_triage.py (docstring, HEAD b2e46d2)
+++ b/tools/ci/vr_triage.py (docstring, post-pin)
@@ -1,64 +1,88 @@
-THE REGISTER — a snapshot of the pre-pin repo state
-    golden-hashes.json pins exactly 4 items (s28-onoda-t150,
-    dof-chart, stroke-toon, ab1-composer); the 14 film mids are
-    staged candidates, unpinned. The workflow renders
-    ab1-composer, s28-onoda-t150 (both from the one ab1 render),
-    dof-chart, and film-s08b. Expected first dispatch:
+THE REGISTER — the post-pin steady state
+    golden-hashes.json pins all 18 items (make_golden verifies
+    18 OK / 0 STALE / 0 MISSING). Both pin acts and the Act-1
+    corrective have landed: Act 1 916a109 (the 14 film
+    first-pins), Act 2 166873c (ab1-composer re-pinned
+    d7bd71a0 -> 60c32c1a), corrective 2755ff3 (11 film pins
+    moved to the f021991 candidates, after the canary caught
+    the first act pinning staged 381ff47-era bytes). The
+    workflow renders a canary set only: the ab1 render
+    (checked against ab1-composer AND s28-onoda-t150),
+    dof-chart, and film frame 1231 (film-s08b). Expected
+    dispatch (first measured on run 43625139818 at 2755ff3:
+    3 MATCH · 1 DRIFT · 14 NOT-RENDERED):
 
-      ab1-composer    DRIFT        pin predates the shade
-                                   root-fix c00897d (EXPECTED
-                                   STALE, "certain" —
-                                   tools/golden/RUNNER.md)
-      s28-onoda-t150  DRIFT        same pre-c00897d debt
-      dof-chart       MATCH        the clean control — its chart
-                                   texture never passes through
-                                   shade_rgb; anything but MATCH
-                                   here is real drift
+      ab1-composer    MATCH        re-pinned in Act 2; only
+                                   MATCH is acceptable — it is
+                                   deliberately absent from
+                                   REGISTER, like the control,
+                                   so any non-MATCH row is a
+                                   NEW-SIGNAL
+      film-s08b       MATCH        the one film mid the
+                                   workflow renders; same
+                                   treatment as ab1-composer
+      dof-chart       MATCH        the clean control — its
+                                   chart texture never passes
+                                   through shade_rgb; anything
+                                   but MATCH here is real drift
+      s28-onoda-t150  DRIFT        the sole remaining debt:
+                      (or          the pin predates c00897d and
+                      WITHIN-     neither act cleared it (the
+                      BAND)       repin packet recommends
+                                   against re-pinning the
+                                   historical prototype);
+                                   WITHIN-BAND is the same
+                                   debt in a milder form
+      film-* (13)     NOT-RENDERED pinned in Act 1 but
+                                   rendered nowhere — the
+                                   workflow's canary set is by
+                                   design; the register entry
+                                   covers this ONE status only
+                                   (see the anti-whitelist rule)
       stroke-toon     NOT-RENDERED the workflow renders no
                                    battery card set (different
                                    output contract, W16 §4)
-      film-* (14)     UNPINNED     staged candidates are not
-                                   references; reported only
 
-    For the two stale pins the debt is "pin predates c00897d", so
-    WITHIN-BAND is the same debt in a milder form (bytes differ
-    from a pre-fix pin) and is registered alongside DRIFT. Every
-    other non-MATCH row is a NEW-SIGNAL: MISSING-RENDER and
-    REFERENCE-MISSING always (run / repo breakage is never debt),
-    any non-MATCH on the dof-chart control, any registered item
-    in an unregistered state, any unknown item id, and any
-    registry item missing from the report altogether.
+    THE ANTI-WHITELIST RULE. A register entry names the exact
+    statuses it covers; every other non-MATCH status on that
+    item is a NEW-SIGNAL ("registered debt item in an
+    unregistered state"). The film entries cover NOT-RENDERED
+    and nothing else: the day a non-canary film mid actually
+    renders and drifts, its DRIFT row is a NEW-SIGNAL, not
+    debt. MISSING-RENDER and REFERENCE-MISSING are always
+    NEW-SIGNAL (run / repo breakage is never debt), any
+    non-MATCH on the dof-chart control is a NEW-SIGNAL, an
+    unknown item id is a NEW-SIGNAL, and any registry item
+    missing from the report altogether is a NEW-SIGNAL.
 
 CLEARING ACTIONS — the only ways debt leaves the register
-    * film-* UNPINNED clears with GOLDEN-RESTAGE-W03.md Act 1:
-      human first-pins of the 14 film mids from
-      docs/golden/frames-f021991/ (one commit; exactly 14 pin
-      lines added; the 4 milestone pins unmoved).
-    * ab1-composer clears with Act 2: the human re-pin, its own
-      commit, diff scoped to the one ab1-composer pin line
-      (packet: docs/golden/repin-packets/ab1-composer.md).
-    * s28-onoda-t150 is cleared by NEITHER act — the repin packet
-      recommends against re-pinning it (historical prototype
-      artefact). It clears only with a separate human disposition
-      of that pin (a deliberate re-pin anyway, or retiring the
-      pin), recorded per docs/golden/NATIVE-GOLDEN.md. Expect the
-      job to stay red on s28 alone after Acts 1-2 land; that is
-      this debt ageing, not a new regression.
+    * s28-onoda-t150 clears only with a separate human
+      disposition of that pin (a deliberate re-pin anyway —
+      which would make it a byte-duplicate of ab1-composer —
+      or retiring the pin), recorded per
+      docs/golden/NATIVE-GOLDEN.md. Expect the job to stay red
+      on s28 alone until then; that is this debt ageing, not
+      a new regression.
     * stroke-toon NOT-RENDERED clears with a future deliberate
-      act: re-baseline the founding pin and add a battery render
-      to the workflow (W16 §4).
-    NOTHING here widens the 80 dB band and NOTHING whitelists a
-    pin. The band is policy (changing it is a commit-message act
-    in visual_regression.py), and whitelisting is the one move
-    every document in this chain forbids. This tool classifies;
-    it cannot silence.
+      act: re-baseline the founding pin and add a battery
+      render to the workflow (W16 §4).
+    * The 13 film NOT-RENDERED entries clear if the workflow's
+      render set is deliberately extended to those mids (MATCH
+      then expected; the entries leave REGISTER in the same
+      commit) — or never: the entries record a permanent
+      design fact, not a defect awaiting a fix.
+    NOTHING here widens the 80 dB band and NOTHING whitelists
+    a pin. The band is policy (changing it is a commit-message
+    act in visual_regression.py), and whitelisting is the one
+    move every document in this chain forbids. This tool
+    classifies; it cannot silence.
 
 REGISTER MAINTENANCE
-    The register describes the repo state BEFORE any pin act.
-    When an act lands, update REGISTER in the same commit —
-    triage against a stale register manufactures NEW-SIGNALs.
-    Post-act expectations: Act 1 -> film-s08b MATCH (it is the
-    one film mid this workflow renders), the other film-* rows
-    become NOT-RENDERED when rendered nowhere / stay UNPINNED
-    only until their pin exists; Act 2 -> ab1-composer MATCH.
+    The register describes the repo state AFTER the pin acts
+    (916a109 / 166873c / 2755ff3). When the state moves again
+    — an s28 disposition, a render-set change, a new pin —
+    update REGISTER in the same commit. Triage against a stale
+    register manufactures NEW-SIGNALs: the pre-pin register
+    printed 13 false NEW-SIGNALs (the designed NOT-RENDERED
+    film rows) on the healthy post-corrective run 43625139818.
 
```

### Hunk B — the register constants (replaces lines 103–154)

```diff
--- a/tools/ci/vr_triage.py (register block, HEAD b2e46d2)
+++ b/tools/ci/vr_triage.py (register block, post-pin)
@@ -1,36 +1,89 @@
 # ---------------------------------------------------------------- register
 # id -> (expected statuses, clearing action). MATCH is never listed:
 # a MATCH row needs no triage on any item.
+#
+# POST-PIN REGISTER (VR-REGISTER-W72). The pre-pin register this
+# replaces described the repo BEFORE any pin act: 4 milestone pins
+# (two of them stale, pre-c00897d) and 14 unpinned film mids. Both
+# acts and the Act-1 corrective have landed:
+#   Act 1            916a109  14 film first-pins struck
+#   Act 2            166873c  ab1-composer re-pinned
+#                             d7bd71a0 -> 60c32c1a (fresh render
+#                             at the integration head)
+#   Act 1 corrective 2755ff3  11 film pins moved to the f021991
+#                             candidates — the VR canary caught
+#                             the first act pinning staged
+#                             381ff47-era bytes (film-s08b DRIFT
+#                             24.85 dB, run 43624398371)
+# golden-hashes.json now pins all 18 items; make_golden verifies
+# 18 OK / 0 STALE / 0 MISSING (GOLDEN-VERIFY-W63.md).
+#
+# The workflow renders a canary set ONLY (.github/workflows/
+# visual-regression.yml, by design): the ab1 render — checked
+# against ab1-composer AND s28-onoda-t150 — dof-chart, and film
+# frame 1231 (film-s08b). Every other pinned item reports
+# NOT-RENDERED on every dispatch. Steady state, first measured
+# on run 43625139818 at 2755ff3: 3 MATCH (ab1-composer,
+# dof-chart, film-s08b — all byte-identical) · s28 DRIFT ·
+# 14 NOT-RENDERED. Expected classes:
+#   ab1-composer    MATCH only — deliberately NOT in REGISTER,
+#                   like the dof-chart control: any non-MATCH
+#                   row is a NEW-SIGNAL
+#   film-s08b       MATCH only — the one film mid rendered;
+#                   same treatment as ab1-composer
+#   s28-onoda-t150  DRIFT / WITHIN-BAND = KNOWN-DEBT, the sole
+#                   expected red (see its entry)
+#   film-* (13)     NOT-RENDERED = expected by design; the
+#                   entry covers that ONE status only (see the
+#                   anti-whitelist rule in the docstring)
+#   stroke-toon     NOT-RENDERED = KNOWN-DEBT (unchanged)
+#   dof-chart       MATCH only — the clean control (CONTROL_ID)
 
 FILM_IDS = ["film-s01", "film-s02", "film-s03", "film-s04",
             "film-s05a", "film-s05b", "film-s06", "film-s07",
             "film-s08", "film-s08b", "film-s09", "film-s10",
             "film-s11", "film-s12"]
 
-FILM_ACTION = ("Clears with GOLDEN-RESTAGE-W03 Act 1 — human "
-               "first-pins of the 14 film mids from "
-               "docs/golden/frames-f021991/ (one commit, exactly "
-               "14 pin lines added, milestone pins unmoved). "
-               "Until pinned, UNPINNED is reported-only by design "
-               "and gates nothing.")
+# The one film mid the workflow renders (frame 1231, the s08b
+# mid — "the highest-value film canary per frame rendered",
+# visual-regression.yml). It is EXPECTED to render, so — like
+# ab1-composer and the dof-chart control — it carries NO
+# register entry: only MATCH is acceptable, and any non-MATCH
+# row (DRIFT, WITHIN-BAND, or NOT-RENDERED because the render
+# set shrank) falls through to a NEW-SIGNAL.
+FILM_CANARY_ID = "film-s08b"
+
+FILM_NOT_RENDERED_ACTION = (
+    "NOT-RENDERED by design: pinned in GOLDEN-RESTAGE-W03 "
+    "Act 1 (916a109; corrective 2755ff3) and covered by the "
+    "make_golden pack verification (18 OK / 0 STALE / 0 "
+    "MISSING), but the visual-regression workflow renders a "
+    "canary set only — the ab1 render (two pins), dof-chart, "
+    "and film-s08b (frame 1231) — so this mid renders on no "
+    "dispatch. This entry covers NOT-RENDERED ONLY: if the "
+    "render set is deliberately extended to this mid, MATCH "
+    "is the expectation, and a DRIFT / WITHIN-BAND / UNPINNED "
+    "row here is a NEW-SIGNAL, never debt.")
 
 REGISTER = {
-    "ab1-composer": (
-        {"DRIFT", "WITHIN-BAND"},
-        "Stale pin: predates the shade root-fix c00897d "
-        "(EXPECTED STALE, tools/golden/RUNNER.md). Clears with "
-        "GOLDEN-RESTAGE-W03 Act 2 — human re-pin per "
-        "docs/golden/repin-packets/ab1-composer.md, its own "
-        "commit, diff scoped to the one ab1-composer pin line."),
     "s28-onoda-t150": (
         {"DRIFT", "WITHIN-BAND"},
-        "Stale pin: predates c00897d, same debt as ab1-composer — "
-        "but cleared by NEITHER W03 act (the repin packet "
-        "recommends against re-pinning this historical prototype "
-        "artefact). Clears only with a separate human disposition "
-        "of the s28 pin (deliberate re-pin or pin retirement, "
-        "recorded per NATIVE-GOLDEN.md). Expect this row to keep "
-        "failing after Acts 1-2 land."),
+        "The sole remaining re-pin debt. The pin (ff68fad3) "
+        "predates the shade root-fix c00897d; Acts 1 (916a109) "
+        "and 2 (166873c) and the Act-1 corrective (2755ff3) "
+        "all landed WITHOUT touching it — the repin packet "
+        "recommends against re-pinning this historical "
+        "prototype artefact, and VR supplies the ab1 render as "
+        "its candidate, so the DRIFT (21.65 dB on run "
+        "43625139818) measures current output against the "
+        "prototype, not a regression. WITHIN-BAND is the same "
+        "debt in milder form. Clears ONLY with a separate "
+        "human disposition of the s28 pin (deliberate re-pin — "
+        "which would make it a byte-duplicate of ab1-composer "
+        "— or pin retirement), recorded per "
+        "docs/golden/NATIVE-GOLDEN.md. Until then the job "
+        "stays red on s28 alone; that is this debt ageing, "
+        "not a new regression."),
     "stroke-toon": (
         {"NOT-RENDERED"},
         "Expected: the workflow renders no battery card set "
@@ -41,7 +94,9 @@
 }
 
 for _fid in FILM_IDS:
-    REGISTER[_fid] = ({"UNPINNED"}, FILM_ACTION)
+    if _fid != FILM_CANARY_ID:
+        REGISTER[_fid] = ({"NOT-RENDERED"},
+                          FILM_NOT_RENDERED_ACTION)
 
 # The clean control: only MATCH is expected; it is deliberately
 # NOT in REGISTER, so every non-MATCH row lands as NEW-SIGNAL
@@ -51,3 +106,4 @@
 # Every id the report must carry (make_golden.py ITEMS order).
 EXPECTED_IDS = (["s28-onoda-t150", "dof-chart", "stroke-toon",
                  "ab1-composer"] + FILM_IDS)
+
```

Notes on Hunk B's shape:

- `FILM_ACTION` is renamed `FILM_NOT_RENDERED_ACTION` (its text is
  now the by-design explanation, not a clearing act). Nothing
  outside the block references the old name.
- `ab1-composer` and `film-s08b` leave `REGISTER` entirely; a new
  `FILM_CANARY_ID = "film-s08b"` constant names why s08b is
  excluded from the film loop. `CONTROL_ID`, `EXPECTED_IDS`,
  `STATUSES`, `NEW_SIGNAL_FOLLOWUP`, and all of `classify()` /
  `triage_report()` logic are untouched.
- The `cleared` informational tripwire still works: a MATCH on
  any of the 13 registered film ids (i.e. the render set was
  extended) prints the `Register note:` line prompting same-commit
  register reconciliation — informational only, verdict unchanged
  (demonstrated in Probe D, §6).

### Hunk C — consistency strings (same commit; cosmetic, no logic)

The tool still calls itself a "first-dispatch" check in five
places. Exact one-line replacements (line numbers at `b2e46d2`):

| Line | Old | New |
|---|---|---|
| 2 | `"""Visual-regression triage — first-dispatch debt register check.` | `"""Visual-regression triage — post-pin debt register check.` |
| 88 | `    0   TRIAGE VERDICT: first-dispatch result is AS EXPECTED` | `    0   TRIAGE VERDICT: result is AS EXPECTED` |
| 304 | `        "# Visual-regression triage — first dispatch",` | `        "# Visual-regression triage — post-pin register",` |
| 328–329 | `f"first dispatch that passes outright is "` | `f"dispatch that passes outright is "` |
| 336–337 | `lines.append("TRIAGE VERDICT: first-dispatch result is "` / `"AS EXPECTED")` | `lines.append("TRIAGE VERDICT: result is "` / `"AS EXPECTED")` |
| 394–395 | `"against the first-dispatch re-pin debt "` / `"register. See the module docstring for the "` | `"against the post-pin re-pin debt "` / `"register. See the module docstring for the "` |

Also update the WHAT THIS IS paragraph (lines 4–16) minimally:
"Its FIRST dispatch is expected to FAIL — on exactly the known
re-pin debt" → "A dispatch is expected to FAIL — on exactly the
known remaining debt (s28)"; and line 307's banner ("KNOWN-DEBT
rows clear only through the pin acts named in their follow-ups")
may stand as-is — s28's follow-up names its disposition act.

### Required companion change — the self-test fixtures

`tools/ci/fixtures/vr_triage/` encodes the pre-pin report, so the
fixtures must move in the same commit or `--self-test` goes red /
goes muddy:

- `expected-first-dispatch.md` **fails its designed verdict**
  under the new register (its ab1 DRIFT and 14 film UNPINNED rows
  become NEW-SIGNALs; exit 1 ≠ designed 0). Replace it with a
  post-pin steady-state fixture — the shape of run
  `43625139818`'s real report (3 MATCH · s28 DRIFT · 14
  NOT-RENDERED) — renamed `expected-steady-state.md`, and update
  the `SELF_TESTS` tuple (line 349) to
  `("expected-steady-state.md", "AS EXPECTED", 0)`.
- `planted-new-drift.md` and `planted-missing-render.md` still
  trip (exit 1, verdict names `dof-chart`) but no longer isolate
  the planted signal — under the new register their ab1/film rows
  add 15 further NEW-SIGNALs, so a future reader can't tell which
  row was planted. Rebuild both on the steady-state base (same
  single planted change: dof-chart DRIFT / MISSING-RENDER), so
  `dof-chart` is again the **only** NEW-SIGNAL in each verdict.
- Add a fourth fixture, `planted-noncanary-drift.md`: the
  steady-state base with one non-canary film row (e.g. film-s03)
  changed from NOT-RENDERED to DRIFT, expected exit 1 with the
  verdict naming that id. This is the anti-whitelist property of
  §4 pinned as a regression test; add its `SELF_TESTS` tuple
  `("planted-noncanary-drift.md", "film-s03", 1)`.

## 4. The anti-whitelist argument (why this register cannot hide drift)

The standing law for this chain: nothing widens the 80 dB band,
nothing whitelists a pin; the tool classifies, it cannot silence.
The new register honours it structurally, through `classify()`'s
existing routing — no code path was added that can turn a rendered
difference into a quiet row:

1. **An entry covers exactly the statuses it names.** A registered
   item in any other non-MATCH status returns NEW-SIGNAL
   ("registered debt item in an unregistered state"). The 13 film
   entries name `{NOT-RENDERED}` and nothing else.
2. **A non-canary film mid that renders and drifts is a
   NEW-SIGNAL.** Worked example (Probe A, §6): take the healthy
   run-`43625139818` report and change only film-s03's row to
   `DRIFT 41.00` — triage under the new register returns
   `1 NEW-SIGNAL: film-s03 (DRIFT)`, exit 1. Same for
   WITHIN-BAND or UNPINNED on any of the 13.
3. **The rendered canaries carry no cover at all.** ab1-composer,
   film-s08b, and dof-chart are absent from `REGISTER`, so every
   non-MATCH row on them is a NEW-SIGNAL (Probe B: film-s08b
   WITHIN-BAND → NEW-SIGNAL, exit 1). If s08b stops rendering
   (render set shrinks), its NOT-RENDERED row is likewise a
   NEW-SIGNAL via the EXPECTED_IDS follow-up — the register
   cannot be used to quietly shrink the canary set either.
4. **Breakage is never debt.** `MISSING-RENDER` and
   `REFERENCE-MISSING` return NEW-SIGNAL before the register is
   consulted, on every item, registered or not; an item missing
   from the report entirely, or an unknown id, is also a
   NEW-SIGNAL.
5. **s28 stays red-labelled.** Its runner row remains DRIFT and
   fails the job (`FAIL (4 pins, 1 drift)`); triage labels it
   KNOWN-DEBT — a named, owned debt with a named clearing act
   (the human disposition) — never OK, never whitelisted.
   WITHIN-BAND stays in its set for the original reason: against
   a pre-`c00897d` pin, a sub-band byte difference is the same
   debt in milder form, not new drift (Probe C, §6).

## 5. Divergence from W37 §6 — stated, and why

W37 §6 prescribed, for after Act 1: "`film-s08b` → expect MATCH;
the other film ids leave the register as debt (once pinned, a
non-MATCH on them is a NEW-SIGNAL)". This pack follows that
prescription for film-s08b and ab1-composer exactly, but **not**
for the other 13 ids, for a reason W37's sketch did not reckon
with: the runner reports **all 18 registry items on every
dispatch**, and the 13 non-canary mids' row is `NOT-RENDERED` —
itself a non-MATCH status. If the 13 ids left `REGISTER`
entirely, `classify()` would route those permanent NOT-RENDERED
rows through the `EXPECTED_IDS` branch to NEW-SIGNAL ("a pinned
item the dispatch was expected to render has no render") on every
healthy run, forever — 13 false signals per dispatch, the precise
failure this update exists to end (and the tool would be crying
wolf when a real signal arrives).

The NOT-RENDERED-only entry preserves the substance of W37's
rule — *once pinned, a non-MATCH on them is a NEW-SIGNAL* — for
every outcome that involves a render: DRIFT, WITHIN-BAND, and
UNPINNED on any of the 13 are NEW-SIGNALs under this register
(§4.2, Probe A). The only state reclassified is the one that
carries no render at all, which the workflow produces by design.
If the lead prefers W37's letter over this reasoning, the
alternative is a `classify()` change (an explicit
NOT-RENDERED-by-design branch) — a code change this pack
deliberately avoids, since the register-only form achieves the
same labels with the tool's logic untouched.

## 6. Verification (this seat's own runs)

Report of record: run **`43625139818`** (Visual Regression
(dispatch), head `2755ff3`, conclusion failure), artifact
`visual-regression-43625139818` fetched via `gh run download`;
its report reads `VISUAL REGRESSION: FAIL (4 pins, 1 drift)`,
counts 3 MATCH · 1 DRIFT · 14 NOT-RENDERED.

| Run | Tool | Result |
|---|---|---|
| Real report | current `vr_triage.py` (old register) | 3 OK · 2 KNOWN-DEBT · **13 NEW-SIGNAL** (the 13 film NOT-RENDERED rows), exit **1** — stale labels reproduced; plus the 2 spurious "debt appears cleared" notes (§1) |
| Real report | scratch copy with Hunk B applied (docstring/strings untouched — they do not affect `classify()`) | **3 OK · 15 KNOWN-DEBT · 0 NEW-SIGNAL**, verdict AS EXPECTED, exit **0**; no register notes |
| Probe A — real report with film-s03 → `DRIFT 41.00` | scratch copy | 3 OK · 14 KNOWN-DEBT · **1 NEW-SIGNAL: film-s03 (DRIFT)**, exit **1** — anti-whitelist property (§4.2) |
| Probe B — real report with film-s08b → `WITHIN-BAND` | scratch copy | **1 NEW-SIGNAL: film-s08b (WITHIN-BAND)**, exit **1** — the canary carries no cover (§4.3) |
| Probe C — real report with s28 → `WITHIN-BAND` | scratch copy | 3 OK · 15 KNOWN-DEBT · 0 NEW-SIGNAL, exit **0** — same debt, milder form (§4.5) |
| Probe D — real report with film-s03 → `MATCH` | scratch copy | 4 OK · 14 KNOWN-DEBT · 0 NEW-SIGNAL, exit **0**, plus the informational `Register note:` for film-s03 prompting register reconciliation — a render-set extension surfaces in triage without failing it (§3) |

The scratch copy lived outside the repo (`/tmp/w72/newtool/`);
the spliced block parses clean and is byte-identical to Hunk B's
`+` side, so the verified behaviour is the patched behaviour for
all register-driven paths.

## 7. Application notes for the lead

1. Apply Hunks A + B + C and the fixture replacements (§3) in
   **one commit** — W37 §6's rule: the register update rides with
   the state it describes; a split landing leaves either the tool
   or its self-test describing a world that doesn't exist.
2. After landing, re-dispatch visual-regression once and triage
   the fresh artifact: expected output is the §2 steady state
   (3 OK · 15 KNOWN-DEBT · 0 NEW-SIGNAL, AS EXPECTED) over a
   runner verdict of `FAIL (4 pins, 1 drift)` — s28 still red,
   by design, until its human disposition.
3. Do not edit the register mid-incident to quiet a future red
   run (W37 §5's standing warning); a NEW-SIGNAL under this
   register is either a real drift, a breakage, or a state change
   that deserves its own deliberate commit.
