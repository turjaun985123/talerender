# E7-READINESS-W78 — is the RADV determinism leg (E7) actually
# covered by the one-visit PC session, as documented?

**Seat:** TaleRender Wave 2, W78 (Batch 8). **Question:** determinism
is discharged on CI lavapipe (E6) but the contract's second renderer
of record — the user's PC, RADV — still owes its formal pack PASS
(E7). Batch 4's W36 wrote the one-visit PC runbook
(`docs/runbook/PC-EVIDENCE-W36.md`); Batch 7's W64 added a bench
addendum (§5 of `docs/integration/BENCH-FIRST-W64.md`). If the
operator runs those documents exactly as written, does E7 get
discharged — or would the leg be missed, refused, or left
unrecordable?

**Verdict: COVERED, with edge gaps. The leg would NOT be missed.**
W36's §1 *is* the contract's §7 procedure, in the right session
position (first evidence leg, immediately after setup/adapter
proof), with the correct command, fresh-root rule, renderer check,
exit-code discipline, and refusal semantics. The gaps found are at
the edges: what travels home on a FAIL (§2, gap G1), whether the
rest of the visit proceeds after a FAIL (G2), a stale register in
the contract itself (G3), and two free supporting-evidence steps
nobody wrote down (G4, G5). Paste-ready text for each gap is in §3.
Tree state for this audit: HEAD `b2e46d2`; W68 (the PC dry-run of
the VM-runnable legs) was still in flight — nothing here depends
on its outcome, and no claim below rests on it.

---

## 1. What E7 requires — extracted from the contract

Source: `docs/integration/DETERMINISM-CONTRACT-W04.md`. Nothing in
this section is paraphrased into new terms; where the contract is
silent, §2 says ABSENT rather than filling in.

- **R1 — the pack is the only discharging instrument.** "The
  TBD-PROOF item clears when one pack PASS per renderer of record
  is on record" (§4). "Probe overlaps and partial routes (§5, E4)
  support the claim but do not discharge it: only the pack
  exercises the same-dir resume and the cache-cap section" (§4).
- **R2 — content.** The pack (`tools/determinism/determinism_check.sh`)
  renders film frames **[100, 130)** at 1080×1920 (§4):
  run A `--from 100 --to 130`, default cache (the reference bytes);
  run B `--from 100 --to 115` then `--from 115 --to 130` into the
  **same** dir (chunk boundary + resume); run C `--from 100
  --to 130` with `TALERENDER_PAINT_CACHE=24` (cache-cap
  pixel-neutrality). 90 frames in total across the three dirs.
- **R3 — compare method.** sha256 of every frame, A vs B and
  A vs C; the results file carries both
  `section: 30/30 identical (0 differ)` lines (§6).
- **R4 — verdict discipline.** The results file ends with exactly
  one line: `DETERMINISM: PASS` (script exit 0) or
  `DETERMINISM: FAIL (n differ)` (exit 1). A broken run is
  neither: the script **refuses** (exit 2 — stale binary, non-fresh
  output dirs, crashed/short render, missing `renderer:` line).
  "Refusal means *no evidence*, never a soft pass" (§4).
- **R5 — identity of the claim.** The results file records the
  binary's path + sha256 and run A's `renderer:` line (§4). The
  citable record (§6) is the results file's header block, both
  section lines, and the final PASS line, cited for the PC as
  "user's PC, RADV" + the date. "Same data" means the same repo
  commit (§1); "same binary" is identified by sha256 (§1).
- **R6 — renderer identity.** The results file's `renderer (run A):`
  line must name RADV — `device type: IntegratedGpu`,
  `driver: radv`. If it names llvmpipe: pin the RADV ICD
  (`VK_ICD_FILENAMES=$(ls /usr/share/vulkan/icd.d/radeon_icd*.json | head -n 1)`)
  and re-run into a **fresh** output root (§7).
- **R7 — fresh output root.** The pack refuses non-fresh dirs "a
  reused dir could 'pass' by comparing a run against itself" (§4).
- **R8 — default cap by design.** Runs A/B use the driver's
  default paint-cache cap; that is part of what is tested. On the
  PC's ~6.7 GiB RAM: "watch the run, don't pre-shrink the cap"
  (§7). Run C is the tight-cap run.
- **R9 — FAIL protocol.** A FAIL names every differing frame.
  Keep the whole output root — `determinism-results.txt`, `logs/`,
  `A-single/` / `B-chunked/` / `C-cache24/` — until the result is
  recorded; the kept frames plus `tools/ab/` are the diagnostic
  path (§7, §8). "Never answer a FAIL by widening the comparison"
  (§8).
- **R10 — a PASS is per binary + tree.** Any change to painter
  arithmetic, cache keys, grain inputs, scatter seeds, scene/film
  data, or the GPU stack voids the register's PASSes for the
  affected renderer (§8). E7's PASS is therefore cited against the
  provisioned binary's sha256 and the checkout's HEAD.

Checked and *not* requirements, so their absence from the runbook
is not a gap: `TALERENDER_BIN` appears nowhere in the contract or
the pack — the pack takes the binary as a positional path argument.
(`TALERENDER_BIN` is the skill-side adapter/bridge convention —
`tools/bridge/README.md`, and the canonical PC binary path per
W46 — not a determinism input.) Likewise, `tools/ci/byte_probes.py`
(the E4 probe instrument) is **not** part of E7: per R1 its ranges
support, they never discharge. Both points matter for §3: the
paste-ready additions use the pack for the verdict, and byte
probes only where they are honestly labelled as support.

## 2. The audit — requirement by requirement

W36 line quotes are from `docs/runbook/PC-EVIDENCE-W36.md` at HEAD.

| Req | Where | Verdict |
|---|---|---|
| R1 pack-only | §1: "This leg is that run, following DETERMINISM-CONTRACT §7 verbatim." Session map, leg 1: "Determinism E7 (RADV pack) … `determinism-results.txt` — the formal RADV determinism verdict" | **COVERED** |
| R2 window/runs | §1: "renders film frames **[100, 130)** at 1080×1920 four ways — A: one invocation, default cache …; B: [100, 115) then [115, 130) into the **same** directory …; C: … `TALERENDER_PAINT_CACHE=24` … then sha256-compares every frame, A vs B and A vs C. 90 frames rendered in total." + the verbatim pack command `tools/determinism/determinism_check.sh ~/talerender-bin/talerender ~/det-run-onoda` | **COVERED** |
| R3 compare | Same §1 paragraph (sha256, A vs B / A vs C); §5.1 send-back names the results file as carrying "both section lines" | **COVERED** |
| R4 verdict/exits | §1: `echo "pack exit: $?"     # 0 = PASS, 1 = FAIL, 2 = refusal (no verdict)`; "Exit 2 is a **refusal**, not a FAIL: read its `REFUSAL:` line, fix the named cause, re-run into a fresh root. A refusal is *no evidence*, never a soft pass." | **COVERED** |
| R5 identity | §0.1 records `git rev-parse HEAD` ("every result below is cited against it"); §0.2: "the results files record the binary's sha256 either way" + `sha256sum ~/talerender-bin/talerender    # record it`; §5.1 lists `determinism-results.txt` first in the package | **COVERED** |
| R6 RADV identity | §0.4 adapter proof (name/RADV/IntegratedGpu required, "must **not** name llvmpipe", ICD pin recipe); §1: "The results file's `renderer (run A):` line must name RADV (`device type: IntegratedGpu`, `driver: radv`). If it names llvmpipe, pin the ICD (§0.4) and re-run into a **fresh** output root." | **COVERED** |
| R7 fresh root | §1 command block: "Fresh output root — the pack refuses non-fresh dirs (a reused dir could 'pass' by comparing a run against itself)." | **COVERED** |
| R8 default cap | §1: "Runs A/B use the driver's **default paint-cache cap by design** — that is part of what is being tested. **RAM flag:** … watch the run; do **not** pre-shrink the cap — a PASS at a shrunken cap is not the E7 verdict." | **COVERED** |
| R9 FAIL protocol | §1: "A FAIL names every differing frame. Keep the whole `~/det-run-onoda/` tree … until the result is recorded — the kept frames plus `tools/ab/measure_ab.py` are the diagnostic path (contract §8). Never answer a FAIL by widening the comparison." §5.1: logs "required if the verdict is FAIL or REFUSAL; keep the frame dirs … until the result is recorded" | **PARTIAL → G1** |
| R10 per-binary claim | §0.1 + §0.2 pin HEAD and binary sha for the whole visit; §1 inherits both. The contract's §8 voiding rule itself is not restated in W36 — acceptable (it binds future citations, not the visit), noted for completeness | **COVERED** (execution); §8 rule not restated |

**G1 — the FAIL send-back does not include the frames or a hash
manifest.** §5.1's package is `determinism-results.txt` + `logs/*`
(+ frame dirs merely *kept* on the PC). But the recording happens
remotely: W36 §5.2 says "The orchestrator appends the E7 row to
the contract's §5 register … send the results file, that append
is not yours to make." On a FAIL, the diagnosis also happens
remotely — and neither the differing frame PNGs nor a sha256
manifest of the three frame dirs is in the package. "Keep until
recorded" protects against local deletion; it does not get the
evidence off the machine. The contract (R9) requires the dirs be
kept and names `tools/ab/` as the diagnostic path — a manifest +
the differing frames are the minimum that makes that path usable
by anyone not sitting at the PC. Paste-ready fix in §3 (G1).

**G2 — no operative stop/continue rule after a FAIL or REFUSAL.**
The session map's rationale says §1 "gates trust in everything
after it (if this renderer cannot reproduce its own bytes,
timings and goldens measured on it are built on sand)" — but no
section states what the operator actually does next in each
case. The three outcomes differ: a REFUSAL is minutes to fix and
re-run, and proceeding without the leg would silently drop the
visit's head item; a FAIL does *not* invalidate §2's timings
(they claim no determinism) or §4's cross-vendor PSNR bands, but
§3's render leans on exactly the purity §1 tests (resume
skips-by-existence; the corrupt-frame trap in §3.2 is the same
surface). As written, the operator must improvise this call
mid-visit. Paste-ready decision text in §3 (G2).

**G3 — the contract's own register is stale about E6 (context,
not an execution gap).** The contract header still reads
"enforcement job written, not yet run" and its §5 register lists
E6 as "**Pending**: workflow written; awaiting commit + first
dispatch". First-hand check by this seat: `gh run view
43617370962` → **Determinism (dispatch): success @ 3a36f1f** —
E6 is discharged, exactly as W36 §1 states ("determinism dispatch
run `43617370962` at `3a36f1f`, verdict `DETERMINISM: PASS`
(recorded in WAVE2-RECORD.md)"). The runbook is right; the
contract's register hasn't had its §6 append. Risk is mild
confusion only: an operator reading the contract alone would
think both legs are open. The append is the orchestrator's act
(contract §6; W36 §5.2) — flagged here, not fixed here.

**W64 §5 (the bench addendum) — audited separately.** It contains
no determinism procedure and claims none: of its two CI legs it
says the frame sizes are "identical … (size equality only — byte
identity is the determinism workflow's evidence, not this
run's)". That restraint is correct, and nothing in §5 conflicts
with E7 (its legs run inside W36's §2 slot, after §1). Two
missed-but-free support opportunities are the only findings —
G4/G5 below. It also never cross-references W36 §1; a one-line
pointer is included in the G5 paste text.

**Checked and clear.** (a) Binary route: contract §7 step 1 and
W36 §0.2 are the same route (green CI run's `talerender-binary`
artifact → `~/talerender-bin`, `--version` sanity "1.0.0 or
later") — COVERED identically. (b) The pack's stale-binary
refusal is named in both (contract §4; W36 §0.2 "the determinism
pack refuses stale binaries"). (c) No step in W36 or W64 §5 asks
the operator to shrink the cap, reuse an output root, or treat a
probe as a verdict — the three classic ways this leg could have
been silently ruined are all guarded in the text as written.


---

## 3. Paste-ready gap text for the lead

Each block names its target. Nothing here amends the contract;
G4/G5 are explicitly supporting evidence under contract §4
("support the claim but do not discharge it").

### G1 → W36 §5.1, after the `~/det-run-onoda/logs/*` row

> **On a FAIL verdict, add the frame evidence itself** — the
> results file names every differing frame, but the diagnosis
> happens from the sent package, and the frames are the
> diagnosis. Before sending:
>
> ```bash
> cd ~/det-run-onoda
> (cd A-single  && sha256sum frame_*.png) > manifest-A-single.sha256
> (cd B-chunked && sha256sum frame_*.png) > manifest-B-chunked.sha256
> (cd C-cache24 && sha256sum frame_*.png) > manifest-C-cache24.sha256
> ```
>
> Send the three manifests plus, for every frame the results
> file names as differing, that frame's PNG from **each** of
> the three dirs (same frame number, three files). Keep the
> full `~/det-run-onoda/` tree on the PC until the orchestrator
> confirms the E7 row is recorded. On PASS or REFUSAL the
> manifests are optional (the results file is the record);
> the tree is still kept until recording is confirmed.

### G2 → W36 §1, new final subsection "If the verdict is not PASS"

> - **REFUSAL (exit 2): stop the session here.** Fix the named
>   cause and re-run §1 into a fresh output root before any
>   other leg. A refusal is *no evidence*; the remaining legs
>   all cite this leg's binary/adapter proof chain, and the
>   re-run costs minutes — never defer E7 past §3.
> - **FAIL (exit 1): preserve, send, then decide.** Freeze the
>   whole `~/det-run-onoda/` tree (do not re-run into it, do
>   not delete from it), complete §5.1 with the G1 additions,
>   and send `determinism-results.txt` + logs before continuing.
>   §2 (bench) may proceed — its tables claim timings, not
>   determinism — and §4's cross-vendor PSNR bands remain
>   meaningful. **§3 proceeds only with the defect flagged in
>   its send-back note:** the full render's resume/chunking
>   story rests on the same purity the pack just failed, so
>   its frames are evidence *about* the defect, not a
>   production claim. Do not attempt any fix on the PC; a FAIL
>   is a defect report (contract §8), and the kept frame dirs
>   + `tools/ab/measure_ab.py` are its diagnostic path.
> - **PASS:** proceed; §5.2's discharge consequence applies.

### G3 → for the orchestrator (not a runbook edit)

> Contract §5 register: append the already-landed E6 row
> (run `43617370962`, Determinism dispatch, success @ `3a36f1f`,
> verdict `DETERMINISM: PASS` — on record in WAVE2-RECORD.md)
> and update the contract header's "not yet run" status for
> the CI leg. E7's row follows the same §6 form when the PC
> results file arrives.

### G4 → W36 §1, optional support step after the pack (new)

> **Optional — RADV byte-probe support (does NOT discharge
> anything).** The pack's window [100, 130) never touches the
> old 160-entry cache-cap boundary where the quantised-key
> defect measured pre-fix (contract E2). While the proven
> binary is provisioned, the E4 probe ranges
> (`tools/ci/byte_probes.py` defaults: 195–204, 795–814,
> 1195–1204) can be re-rendered twice on RADV and byte-compared
> — supporting evidence on this renderer, in the E4 class:
>
> ```bash
> export TALERENDER_BIN=~/talerender-bin/talerender
> cd ~/workspace/talerender
> for dir in probe-a probe-b; do
>   for r in "195 205" "795 815" "1195 1205"; do
>     set -- $r
>     "$TALERENDER_BIN" --film film --out $dir --from $1 --to $2
>   done
> done
> python3 tools/ci/byte_probes.py probe-a probe-b
> ```
>
> Both dirs fresh, default cache cap, same binary, back to
> back. Expected shape: `BYTE PROBES: PASS (40/40 frames
> identical across 3 range(s))`. Any FAIL here alongside a
> pack PASS is still a headline — send both outputs; per
> contract §4 the pack verdict stands as the E7 record and
> the probe result opens a defect thread, it does not amend
> the verdict either way.

### G5 → W64 §5 addendum / W36 §2 note (one paragraph + one command)

> The §5 RADV bench legs render frames 0..100 twice (default
> cap, then `TALERENDER_PAINT_CACHE=24`) into two fresh dirs —
> the same shape as pack section C over a 100-frame window.
> While both dirs exist, one command turns them into RADV
> cache-cap-neutrality *support* evidence at zero extra
> renders (cross-reference: the E7 verdict itself is W36 §1's
> pack; this is not it):
>
> ```bash
> python3 tools/ci/byte_probes.py --ranges 0-99 \
>     perf-radv/frames-default perf-radv/frames-cache24
> ```
>
> Record the output beside `perf-radv/REPORT.md`. A FAIL here
> with a pack PASS is reported, not reconciled on the spot.

---

## 4. Session-order note — where E7 belongs in the visit

W36's published order — §0 setup → **§1 E7** → §2 bench →
§3 full film → §4 golden verification → §5 send-back — is the
right order for E7, and this audit recommends no change beyond
G2's stop rule:

- **E7 before bench (§2).** Both legs share §0's provisioned
  binary and proven adapter; E7 is the short leg and its verdict
  is the trust anchor the session map promises. The bench
  additionally needs an idle machine — nothing about E7's four
  short renders disturbs that if it runs first and finishes.
- **E7 before the full film (§3), non-negotiably.** §3 is the
  visit's long pole and its resume/chunking claims rest on the
  purity the pack tests. Discovering a determinism FAIL *after*
  §3 would retroactively cloud 1,811 frames of evidence;
  discovering it before costs one short leg.
- **Time implication, stated honestly.** The pack renders
  90 frames in 4 invocations — 1/20th of §3's 1,811 frames at
  the same resolution, with §3 additionally carrying the full
  film's heavier segments. No RADV frame rate exists yet (that
  is the visit's purpose), so no minute figure can be promised
  for either leg. The only measured anchor is CI lavapipe's
  1.85 s/frame over frames 0–100 (W64 — software renderer,
  S01-only window): even if RADV merely matched that software
  pace, the pack would take ~3 minutes against ~56 for the
  film. That is an orientation bound from a different renderer,
  **not** an estimate — W36 §1's "no PC timing exists" stands.
  What can be said without inventing numbers: E7's cost is a
  small fraction of the visit under any plausible RADV speed,
  which is precisely why G2 makes a refusal an immediate
  fix-and-rerun rather than a deferral.
- **The contract's probe-window alternative — quoted, and its
  limit.** The contract offers no shorter E7: §4's discharge
  rule is the pack, full stop. The windowed probes (contract
  §5 E4: 1195–1204 10/10, 795–814 20/20, 195–204 10/10, all CI
  lavapipe) are the only alternative-shaped evidence the
  contract recognises, and it explicitly denies them discharge
  power because they skip the same-dir resume and cache-cap
  sections. On the PC they exist only as G4's optional support.
  Anyone tempted to run "just the probes" on the PC and call
  E7 done would be repeating, on RADV, the exact impersonation
  the contract warns against.

## 5. Bottom line

Run W36 as written and E7 happens: right instrument, right
window, right order, right verdict discipline, right recording
chain. The five additions above close the edges — FAIL evidence
that can actually travel (G1), an operative stop/continue rule
(G2), a stale register line in the contract (G3), and two
zero-or-low-cost RADV support measurements the visit was already
rendering without harvesting (G4, G5). Nothing in W36 or W64 §5
would produce a false E7 PASS; the failure modes that remain are
omissions around a FAIL, not risks to a PASS.

*Evidence basis: DETERMINISM-CONTRACT-W04.md, PC-EVIDENCE-W36.md,
BENCH-FIRST-W64.md §5, GPUCORE-PROBES-W34.md (probe defaults),
tools/determinism/determinism_check.sh header, and `gh run view
43617370962` — all read first-hand by this seat at HEAD `b2e46d2`.
No file was edited; nothing was committed.*
