# STATE-KEY FIX — paint-cache `state_key` sweat/tears defect

**Status: FIXED AND PROVEN, 2026-11-15.**
User order: "Fix the sweat/tears cache defect now" (2026-11-15 16:14 +06).
Defect disclosed in V2-HANDOVER.md ("Discovered defect"), found by the
V2 WP10 seat's `charstate.rs` audit, verified against the code there
and re-verified here field by field.

## The defect

`film.rs::state_key` builds the PaintCache key for a character state.
Painting is a pure function of the state, so the cache is a pure
optimisation ONLY if the key names every field the painter consumes.
The key omitted `FullState.sweat` and `FullState.tears`, both of which
the painter reads — so two states differing only in sweat or only in
tears produced the same key, and the second state silently reused the
first state's paint.

## Field audit (the whole class, not just the instance)

Every `FullState` field, its painter read in `toonfull.rs`
(`paint_toon_full` and its helpers — the only paint path the cache
feeds), and its presence in the pre-fix key:

| FullState field | Painter reads it | In pre-fix key |
|---|---|---|
| mood | yes (face driver) | yes (`{:?}`) |
| look [f32; 2] | yes | yes (exact bits ×2) |
| blink | yes (×2) | yes (exact bits) |
| talk | yes | yes (exact bits) |
| **sweat** | **yes — toonfull.rs:775 `if st.sweat \|\| m == Mood::Worried`** | **NO — defect** |
| **tears** | **yes — toonfull.rs:788 `if st.tears \|\| m == Mood::Sad`** | **NO — defect** |
| arm_l | yes | yes (exact bits ×2) |
| arm_r | yes | yes (exact bits ×2) |
| legs | yes (×5) | yes (tag) |
| phase | yes (×2) | yes (exact bits) |
| lean | yes | yes (exact bits) |
| bob | yes | yes (exact bits) |
| squash | yes | yes (exact bits) |
| head_tilt | yes | yes (exact bits) |
| head_turn | yes | yes (exact bits) |
| cam_pitch | yes (×2) | yes (exact bits) |
| sit_drop | yes | yes (bits widened to i64, −1 = None) |
| hold_l | yes (plate composite) | yes (plate name; name→Plate is fixed per run) |
| hold_r | yes (plate composite) | yes (plate name) |

The remaining painter input, the `FullLook` (14 fields, all read), is
named by the key's look-index prefix; the film look registry is fixed
for a run, so an index names exactly one FullLook (the same soundness
argument `charstate.rs` records). **Conclusion: sweat and tears were
the only omissions. No further omissions exist.**

## The fix (landed alone)

Commit **`b1355b1`** — `src/film.rs` only, nothing else in the commit:

- `state_key` appends `st.sweat, st.tears` to the key. All f32
  channels remain exact bit patterns (the determinism law; no
  quantised keys). Field coverage now matches `charstate.rs`'s
  `CharStateKey` exactly.
- Regression tests (`film.rs::state_key_tests`): state pairs
  differing only in sweat, and only in tears (base state =
  `FullState::default()`, mood Neutral, where both flags are
  paint-relevant), must produce different production keys; identical
  states must share a key.

Test proof, both directions, on CI:

- **Pre-fix failure**: the tests alone were pushed on a throwaway
  branch (deleted after the verdict): CI **43703339048** —
  403 passed, **exactly 2 failed** (`state_key_distinguishes_sweat_only_difference`,
  `state_key_distinguishes_tears_only_difference`).
- **Post-fix pass**: CI **43703592030** at `b1355b1` — Build SUCCESS,
  **Test: 405 passed; 0 failed** (402 baseline + 3 new). Tools CI
  **43703592053** SUCCESS. (The CI run's conclusion reads failure
  only because the "Upload frame artifact" step still fails on the
  GitHub artifact-storage quota — build and test steps are green;
  the same upload-only pattern as the `e3c4a2a` run 43703301512.)

## Byte-probe proof: full-film census — ZERO differing frames

The standing 40 probe frames cannot enumerate a whole film, so a
dispatch-only census workflow (`.github/workflows/census.yml`,
tooling commit `e3c4a2a` — no src changes, so its binary is the
pre-fix renderer) rendered **all 1,811 frames** on the CI lavapipe
pre- and post-fix, printing every frame's sha256 to the job log
(no artifacts, per the quota law):

- Pre-fix (renderer = `a098c33`): census run **43703307203** SUCCESS.
- Post-fix (renderer = `b1355b1`): census run **43710059527** SUCCESS.
- Hash sets (local evidence, `~/workspace/talerender-evidence/`):
  `hashes-prefix-a098c33.txt`, `hashes-postfix-b1355b1.txt` —
  1,811 entries each, frame sets identical.
- **Comparison: 0 of 1,811 frames differ.** The two hash files are
  byte-identical.

Why zero is the correct outcome here (not a proof gap): nothing in
the film path ever sets sweat or tears — no assignment exists in
`src/` outside tests and the scene-file JSON reader; the film's
states carry both flags constant `false`, and its sweat-drop/tear
pixels arrive via the Worried/Sad moods, which were always in the
key. Adding two constant fields to the key cannot repartition the
film's states, so no frame's paint can change. The defect's bite is
on synthetic/future states — exactly what the regression tests
demonstrate: collision on the pre-fix key, distinct keys post-fix.

Telemetry on both census runs: **FILM TELEMETRY: CHECK OK, exit 0**
(pre 43703307203, post 43710059527).

## Golden pins

**No pin action.** Zero differing frames means no pinned frame's
render changed; the pins that verified 17 OK at `a098c33` verify
against byte-identical frames post-fix. The pre-existing film-s03
pin debt (stale for a non-gait reason, predating the blink
adoption) is untouched and unchanged by this fix, as instructed.

## Commit / run index

| Item | Id |
|---|---|
| Tooling (census workflow, no src) | `e3c4a2a` |
| The fix (alone) | `b1355b1` |
| Pre-fix test-failure proof (branch deleted) | CI 43703339048 |
| Post-fix CI (405/405; upload-step quota red only) | CI 43703592030 |
| Post-fix Tools CI | 43703592053 SUCCESS |
| Census pre-fix (`a098c33` renderer) | 43703307203 SUCCESS |
| Census post-fix (`b1355b1` renderer) | 43710059527 SUCCESS |
