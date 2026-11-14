# Re-pin packets — index

Evidence packets for the four founding golden items
(`tools/golden/golden-hashes.json`) after the shade root-fix
`c00897d` redefined `shade_rgb` in `src/scene.rs` and declared
a re-baseline. Background: the re-pin debt note in
`docs/golden/NATIVE-GOLDEN.md` and the per-item analysis in
`tools/golden/RUNNER.md` ("Current expectation: the founding
four vs the shade fix").

**The rule: packets are evidence; pinning is a human/lead
act.** A re-pin is deliberate, disclosed and measured
(`docs/golden/NATIVE-GOLDEN.md`, "Re-baselining"), and
`tools/golden/run_golden.sh` refuses `--pin` outright.
Nothing in this directory pins anything, and no packet's
state changes until the human act it describes is performed
and recorded in the packet itself.

| Packet | State |
|---|---|
| [`ab1-composer`](ab1-composer.md) | **CANDIDATE IN HAND** — post-fix candidate (CI run 43569670339 at `c00897d`, sha256 `a415ead8…`) measured closer to the Chrome reference on every metric (23.850 → 23.96 dB) and 41.351 dB from the old pin: a shade correction, not drift. Ready for the human re-pin act; this is the item that actually clears the shade debt. |
| [`s28-onoda-t150`](s28-onoda-t150.md) | **CANDIDATE IDENTIFIED + MEASURED — re-pin NOT recommended under the shade debt.** The pin is the §28 prototype's historical output; the current-chain candidate differs structurally (21.711 dB, max Δ 206), far beyond the shade signature, so replacing it would be a retirement decision, not a re-baseline. Pin keeps verifying OK. |
| [`stroke-toon`](stroke-toon.md) | **PENDING — no post-fix candidate exists.** The move is expected but confined (the `paint_toon` shade regions only). The battery is not a render.yml mode; the packet names the production route: a post-fix CI binary + `talerender --scene stroke-battery`. |
| [`dof-chart`](dof-chart.md) | **PENDING-not-required — no re-pin owed.** Not expected STALE from the shade fix (`chart_pattern` is literal arithmetic, no `shade()` call). A post-fix verification render via render.yml `mode=dof-chart` is expected to match the pin byte-for-byte; any STALE here would be drift, to be investigated — never attributed to `c00897d`. |

The 14 film items have no packets: they have never been
pinned, carry no shade debt, and their first pins will be
struck from post-fix frames.
