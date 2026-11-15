# TaleRender V2 — Handover (master prompt §23)

Executed 2026-11-15 by the V2 chief architect / integration
lead + 20 seats in 2 batches of 10 (actual counts: Batch 1
10/10 delivered + integrated; Batch 2 10/10 delivered +
integrated; zero seats rejected). Authority:
docs/integration/V2-MASTER-PROMPT.md; contracts:
docs/integration/V2-CONTRACTS.md. Testing campaigns
(benchmark campaign, stress, fault injection, visual
regression, hardware validation) are DEFERRED to the
validation wave per §21 — statuses below never claim them.

Final head at handover: see git log (this document's
commit). Test suite: **402 passed** (195 pre-V2 → 320
after Batch 1 → 402 final). Every render-path landing
carries the §4 minimal gates: CI build SUCCESS (run
conclusions repeatedly quota-only at the artifact-upload
step — step lists cited per landing), full tests green,
FILM TELEMETRY CHECK OK, byte probes **40/40 identical**
to the d101770 baseline at every gated head.

## Per-WP status

| WP | Status | Evidence / note |
|---|---|---|
| WP1.1 paint-cache LRU | IMPLEMENTED, BUILD VERIFIED | `468b573` per V2-WP1-LRU-SPEC; probe `43693686155` 40/40; golden 17 OK; hits+misses invariant 482=482 |
| WP1.2 dirty-rect uploads | Module IMPLEMENTED, BUILD VERIFIED; activation INCOMPLETE | `dirtyrect.rs` (`a4ab2e4`) is inert policy: `upload_dyn` creates a fresh texture per frame, so partial uploads have nothing to write into. Activation = persistent Dyn textures, a render-path landing of its own |
| WP1.3 memory pressure | IMPLEMENTED, BUILD VERIFIED, partially wired | `mempressure.rs` (`6e5759a`); resolved + reported in the --film branch (`097583f`); budget-feed/queue-cap consumers wire incrementally |
| WP1.4 lifetimes | IMPLEMENTED, BUILD VERIFIED | `lifetimes.rs` (`519c8eb`); diagnostics ahead of consumers by design |
| WP2 telemetry reporting | IMPLEMENTED, BUILD VERIFIED; print wiring INCOMPLETE | `memreport.rs` + gpucore `MemSnapshot` (`9c01eb0`); modes do not yet print the mem-report beside the pinned gpu-mem line |
| WP3 adapter + device loss | IMPLEMENTED, BUILD VERIFIED | Module `2c11ab1`; canonical lines at all 5 sites, policy selection in film + main run, `set_device_lost_callback` live (`ef30684`+fixes); exit-0 observed (probe `43695041527`); recreate-and-resume NOT implemented — documented impossible-honest (STAGE1-SCOPE §6); ab1/scenefile/dof selection still two-step |
| WP4 streaming | Core IMPLEMENTED, BUILD VERIFIED; driver wiring INCOMPLETE | `streamq` (`0a75fa6`), `ffstream` (`1b29db9`), `streammodes` (`86e10e3`); --film resolves + prints the mode and fails loudly on Stream; the streaming session is not wired into run_film |
| WP5 interchange | IMPLEMENTED, BUILD VERIFIED | `projectfile.rs` (`b6c7792`) + `tools/export/export_project.mjs` (`69bce0f`, self-test 15/15). §15 step-2 evaluator + s12 retirement were NOT this wave (C4) |
| WP6 perf lab | IMPLEMENTED, BUILD VERIFIED, tooling-proven on lavapipe | `bench_lab.py` (`cc031d4`, self-test 34/34), `report.py` (`813dff5`, 24/24), timing emission + bench.yml legs (`81cf0f4`); software record docs/runbook/BENCH-SOFTWARE-RECORD.md; GPU rows PENDING-PC |
| WP7 scheduler | IMPLEMENTED, BUILD VERIFIED | `scheduler.rs` (`b1c830b`); planning layer — run_film keeps its loop until the WP4 session wiring consumes plans |
| WP8 asset streaming | IMPLEMENTED, BUILD VERIFIED | `assetstream.rs` (`f8c55cb`); no production caller yet (dedup by path+size, stated in-file) |
| WP9 production integration | IMPLEMENTED (repo-side) | Validator + guide (`3523dc2`); real-project run OK with 1 flag; skill-side changes are PROPOSALS P1–P5, none made |
| WP10 character efficiency | IMPLEMENTED, BUILD VERIFIED (advisory) | `charstate.rs` (`cf0f7f8`); see the discovered defect below |
| WP11 output/encoding | IMPLEMENTED, BUILD VERIFIED | `outputmgr.rs` (`12f5d05`); not wired — film's inline naming is byte-identical by construction + tests |
| WP12 observability | IMPLEMENTED, BUILD VERIFIED | `observability.rs` (`b026a10`) + V2-ERROR-REFERENCE.md; `event()` is used by the device-loss path; wider adoption incremental |
| WP13 config/CLI | IMPLEMENTED, BUILD VERIFIED, wired | `config.rs` (`de91702`) + main wiring (`097583f`); C8's `stream_fps` field DEFERRED (no consumer; contracts §3) |
| WP14 documentation | IMPLEMENTED | `docs/v2/` (`128bb90`) with lead erratum (adapter/charstate landed after the seat's final check) |

## Discovered defect (needs a decision, NOT silently fixed)

`film.rs::state_key` omits `sweat` and `tears`, but the
painter consumes both (toonfull.rs) — states differing
only in sweat/tears collide in PaintCache and reuse the
wrong paint (found by the WP10 seat, verified against
the code). `charstate.rs`'s key includes them. Migrating
the production key will change pixels on exactly the
affected frames (sweat/tears states) — it is a
correctness fix that must land alone under the byte-probe
protocol, with the differing frames quantified and
reported, not folded into any other landing.

## Dependency inventory

wgpu **30.0.2**, pollster **1.0.1**, image **0.25.10**
(exact pins, verified latest on crates.io 2026-11-15);
CI toolchain = latest stable (rustc 1.100.0 at the
upgrade build; 213 packages resolved, no Cargo.lock by
house law). **Zero new dependencies in all of V2.**
Edition stays 2021 (deferred with reason, STAGE1-SCOPE
item 0). The upgrade's pixel-neutrality proof: probes
40/40 vs d101770 at `65e4ea2`, golden 17 OK.

## Known limitations / remainders

1. WP1.2 activation, WP4 driver session wiring, WP2
   mem-report print wiring, WP7/WP8 production callers,
   WP11 wiring — modules landed ahead of consumers,
   each recorded above; none is claimed as wired.
2. WP3: policy selection not threaded into ab1/
   scenefile/dof; device recreate-and-resume not
   supportable on wgpu 30 without new seams.
3. WP5: §15 step-2 evaluator and the s12 retirement
   remain their own programmes (out of this wave per C4).
4. C8 `stream_fps` remainder (contracts §3).
5. film-s03 golden pin remains stale for a pre-existing
   non-gait reason (disclosed since Wave 2; not re-pinned).
6. Pre-existing, untouched: the native Onoda film's
   colour QA question is open with the user (no V2 work
   changed production colour defaults).

## PC-blocked (validation wave — unchanged by V2)

RADV hardware proof + GPU benchmark rows + sysfs VRAM
cross-check (PC-SESSION-W98); fog-chart PSNR leg;
post-adoption skill selftest clean re-run; hardware
device-loss behaviour (CI proves the lavapipe path only).

## Gate evidence index

Upgrade proof probe `43688575920`; Batch-1 gate probe
`43691989034`; Batch-2 integration CI `43700029997`
(Test SUCCESS, quota-only conclusion); LRU probe
`43693686155`; wiring probe `43695041527`; final probe
`43696089022` (at `81cf0f4`, 40/40); bench dispatch
`43696086097` (legs SUCCESS, uploads quota-blocked);
golden verify 17 OK (local-only, by the no-photos rule).
