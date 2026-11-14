# FORMAT15-CHECK-W51 — the §15 conformance checker (guard + prefabs + clips in one gate)

**Seat:** TaleRender Wave 2, W51 (Batch 6 — adoption preparation).
**Date:** 2026-11-14. **Tree:** HEAD `aa92cee` (Batch 5 closed at
`d3def63`; the lead's `aa92cee` film.rs fix landed while this seat
worked). **Status:** tool + fixtures DONE and proven (§5–§7).
Nothing committed; no existing file edited by this seat (one
hygiene flag, §8).

**Deliverables**

| Piece | Path |
|---|---|
| Unified §15 conformance checker | `tools/format/check_format15.py` |
| Fixtures (1 valid composite, 1 pass-with-note, 9 planted violations) | `tools/format/fixtures/format15/` |
| This note | `docs/integration/FORMAT15-CHECK-W51.md` |

---

## 1. What the tool is, and why it is not a third validator

The §15 draft (`FORMAT15-DRAFT-W31.md`) lands two extensions with
one guard. Until now the only consumers were the two Batch-2
reference validators, each blind to the other's half and both
blind to three things: the guard itself as a check (a file whose
reserved keys sit in **pre-§15 shapes or positions** — the draft's
§15.1 hazard is precisely that such a file is *silently passed*),
the §15.9 placement rulings (no Batch-2 tool looks at them), and
corpus sweeps (§15.10 step 3 wants the validators run over
production data, where "no constructs" must be a pass, not an
input error).

`check_format15.py` therefore **delegates**: it imports
`check_prefabs.py` and `check_clips.py` from its own directory and
re-emits their sub-check verdicts and failure lines with
attribution — their fixture semantics are not forked, and §6
proves the delegation reproduces every Batch-2 verdict. Around
the delegation it adds a reserved-key **census** (one walk that
knows whether it is inside the `prefabs` block, the `clips`
block, `nodes`, or an instance's `params`) and four checks of its
own. House style throughout: numbered checks, per-check
PASS/FAIL/SKIPPED with a headline, failure lines in document
order, an `OVERALL:` line per file, exit **0** pass / **1** any
check failed / **2** input error (one-line `ERROR:` on stderr, no
report). Stdlib only.

| # | Check | Source | Runs when |
|---|---|---|---|
| 1 | **guard** — reserved keys in pre-§15 shapes/positions are reported, never silently passed; version rule (a v1 file carrying reserved keys is rejected) | §15.1, §15.2 | always (a construct-free file passes trivially) |
| 2 | **prefabs** — delegated to `check_prefabs.py`, its 6 sub-checks summarised | §15.2–15.5 via W14 | the file carries prefab constructs, or `--library` was given |
| 3 | **clips** — delegated to `check_clips.py`, its 5 sub-checks summarised | §15.6–15.8 via W15 | the file carries a `clips` block or a clip instance |
| 4 | **colour lane** — colour-channel key values authored as `#RRGGBB`; an `[f32; 3]` array or a `{hex, shade}` object on a colour channel fails, citing the conversion law | §15.9 (lane), §15.6/§15.7 (spelling) | a `clips` block exists; SKIPPED if it declares no colour channels |
| 5 | **step channels** — `mood`/`legs` declared only on character nodes, every key `ease: "step"`, every value an enum member | §15.7 table, §15.9 side channel | a `clips` block exists; SKIPPED if it declares no step channels |
| 6 | **placement** — `duration` on the clip only, wrap on the instance only, no poseTrack residue (`move`/`moveSecs`) in clip keys; NOTEs for first-key eases (§15.6) and tracks targeting instance names (§15.5) | §15.9, §15.8, §15.6 | a `clips` block exists |

Channel kinds for checks 4/5 come from `check_clips`' imported
tables (`channel_spec`, `CHARACTER_CHANNELS`) — the vocabulary
has exactly one home.

---

## 2. Clause-by-clause coverage (draft section → check → fixture)

| Draft clause | Rule | Check | Exercised by |
|---|---|---|---|
| §15 preamble | reserved keys: top-level `prefabs`/`clips`, node-level `prefab`, `pid`, instance `clip`/`clipTime` | 1 (census) | all fixtures |
| §15.1 | guard: hard-error, never silently ignore; v1 carrying reserved keys rejected | 1 | `bad-guard-version` (FAIL 1 only) |
| §15.1 | a reserved block in a non-§15 shape is reported | 1 | `bad-guard-shape` (`prefabs` as an array; check 2's parse sub-check names the same defect — overlap O1) |
| §15.1/§15.6 | `clipTime` with no `clip`, `clip` inside a definition, non-string `clip` — reported, not ignored | 1 | `bad-guard-stray-cliptime` (FAIL 1 only; `check_clips` cannot see a lone `clipTime` at all) |
| §15.2 | `pid` exists only inside templates; stripped at expansion | 1 | `bad-guard-stray-pid` (pid on `camera`; FAIL 1 only — no Batch-2 tool scans block positions for `pid`) |
| §15.2 | slots lexically scoped to templates | 1 (+ delegate 2/3) | census; slot grammar itself is W14's fixtures |
| §15.2–15.5 | prefab grammar, params/typing/forwarding, overrides, cycles, expansion + residue | 2 (delegated, 6 sub-checks) | `valid-composite` (PASS); `bad-prefab-closed-key` (FAIL 2 only); W14's 13 fixtures by regression (§6) |
| §15.3 | forwarding type rule (exact, or `color`→`string` widening) | 2 (delegated check 3) | `valid-composite` (post-pair forwards `height`, `capColor`) |
| §15.4 | overrides deep-merge by pid; forbidden keys; transforms compose via wrapper | 2 (delegated checks 4/6) | `valid-composite` (cap roughness override + instance transforms) |
| §15.6 | clip/track/key grammar; `uses` layering; instance `clipTime` defaults; camera clip⊕keys conflict | 3 (delegated, 5 sub-checks) | `valid-composite`; W15's 8 fixtures by regression (§6) |
| §15.6 | first key's `ease` "has no incoming segment and is ignored" | 6 (NOTE, never a failure) | `note-first-key-ease` (PASS + note); also noted on the composite's step tracks |
| §15.7 | channel vocabulary + kinds; `grade.time` rejected by name (R3); unknown channels fail | 3 (delegated check 2) | `valid-composite`; `bad-grade-time-r3` by regression |
| §15.7 | colour values are `"#RRGGBB"` | 3 (delegated check 2) + 4 | `valid-composite` (`lights.key.color`, `fog.color`) |
| §15.7/§15.9 | step channels: ease `step` required, enum values, character targets only, side-channel sampling | 5 (+ delegate 3/3) | `valid-composite` (`mood`, `legs`); `bad-step-channel` (FAIL 5 + 3 — overlap O2) |
| §15.7 | ease names = `Easing::from_name`'s ten spellings; arriving-key placement per §14.4 | 3 (delegated check 3) | `bad-clip-ease` (`bounce`; FAIL 3 only — the same file's prefabs half passes) |
| §15.8 | poseTrack conversion leaves no move field in clip keys | 6 | `bad-placement` (`moveSecs`; FAIL 6 only) |
| §15.9 | colour lane `Color` holds linear `[f32; 3]` **inside the evaluator**; files author hex, loader converts | 4 | `bad-colour-lane` (`[1.0, 0.94, 0.84]` on `lights.key.color`; FAIL 4 + 3 — overlap O3) |
| §15.9 | "Duration lives on the clip; wrap lives on the instance" — never per-track | 6 | `bad-placement` (per-track `duration`; FAIL 6 only) |
| §15.9 | ease stored on the arriving key; loader remaps `key[i+1].ease → key[i].ease` | — (loader-side rule; nothing in a file to check beyond the arriving-key spelling, which is check 3's) | — |
| §15.9 | rotation is euler scalars; no quaternion channel | 3 (delegated check 2: `rotate.x/y/z` exist, `rotation` is an unknown channel) | vocabulary itself |
| §15.10 | adoption order; validators run over production data with exit 0 = pass | 1–3 (census SKIPs) | the §7 sweep: every production file passes trivially |

**Named overlaps** (one defect, two reporters — deliberate; each
reporter cites its own clause, and the fixture files say so in
their `name` fields): **O1** malformed reserved blocks (guard +
`check_prefabs` check 1); **O2** step-ease violations (check 5 +
`check_clips` check 3); **O3** colour value shapes (check 4 +
`check_clips` check 2). A `pid` under `nodes` is likewise caught
twice (guard by position; `check_prefabs` check 6 as expansion
residue) — the stray-pid fixture therefore plants its pid on the
`camera` block, the position only the guard covers.

---

## 3. Disagreements between the draft and the Batch-2 specs

Nothing here was silently resolved: where the texts conflict,
**the draft wins** in this tool (per the seat's brief), and each
item names what the tool does. Verbatim quotes throughout.

**D1 — Both source specs claim §15; the draft merges them.**
W14's adoption list: "(1) fold this spec into
`docs/scene-format.md` as §15, striking nothing above". W15's:
"(a) fold §2–§4 into `docs/scene-format.md` as a new §15 (format
v2 additive field…)". The draft's own flag 1: "**Both source
specs claim §15.** W14 §10 says "fold into §15" for prefabs; W15
§7(a) says "fold §2–§4 as a new §15" for clips. The draft makes
§15 one combined section (guard §15.1, prefabs §15.2–15.5, clips
§15.6–15.8, rulings §15.9, adoption §15.10)". *Tool:* one gate
for the combined §15 — checks 2 and 3 are its two halves.

**D2 — `uses` layering is in the draft but was never ratified.**
W15 §7 flag 3: "`uses` layering is new data surface (§2.4) — it
exists because "clip reference cycles" was a required failure
class… If the evaluator rejects layering, drop `uses` and check
4's cycle/layering half goes with it; instances and the rest of
the model are unaffected." The draft keeps the semantics (§15.6:
"an instance of the clip then applies its own tracks **plus**
the used clips' tracks, transitively") while its flag 2 records
that "WAVE2-RECORD §9 records decisions for the colour/step
lane, the ease bridge, and wrap/duration — but not for W15 §7
flag 3 (keep-or-drop `uses`)". *Tool:* validates `uses` per the
draft (via `check_clips` check 4, exercised by the composite's
`dusk-fall` → `ambience` layering). If the lead drops layering,
check 3's delegation loses that half with the spec — the tool
needs no change of its own.

**D3 — Duration/wrap placement: evaluator shape vs format
ruling.** W15 §7 flag 1: "anim.rs puts `wrap` and an effective
`duration` on each **Track**, while this model puts `duration`
on the **clip** (one bound for all its tracks — the validator's
beyond-duration check needs a single bound) and the wrap mode on
the **instance** (`clipTime.loop`, since two instances of one
clip may legitimately map time differently)." Draft §15.9 rules
it: "**Duration lives on the clip; wrap lives on the instance.**"
and "no format file ever spells them per-track". *Tool:* check 6
is this ruling enforced — it is the one §15.9 item with no
Batch-2 coverage at all (`bad-placement` passes `check_clips`
5/5 and fails only here).

**D4 — The colour lane's name: record vs draft vs code.**
W31 flag 4: "WAVE2-RECORD §9.1 decides `AnimValue::Color`,
linear-space `[f32; 3]` — US spelling, three components… At HEAD
`AnimValue` is a trait, so the record's phrase is lane
shorthand". Draft §15.9: "Decided: the evaluator gains a colour
lane, `Color`, whose values are **linear-space `[f32; 3]`**…
This extension lands with the clips loader adoption (§15.10);
until then no evaluator lane is claimed to exist." Verified at
this seat's HEAD (`aa92cee`): `src/anim.rs` has `pub trait
AnimValue` (line 186) with impls for `f32`, `[f32; 3]`, and
`Quat` only — **no `Color` lane exists yet**, exactly as the
draft states. *Tool:* check 4 speaks of the lane by the draft's
name (`Color`) and claims nothing about code.

**D5 — What a colour value looks like in a file.** The seat
brief's shorthand ("colour-lane values are [f32;3]-shaped
arrays") reads §15.9's lane shape as a file spelling; the draft
itself separates the two. §15.6: "`value` — shaped by the channel
kind (§15.7): a number (scalar), `[x, y, z]` (vec3),
`"#RRGGBB"` (colour), or an enum string (step channels: `mood`,
`legs`)". §15.9: "The loader converts each colour key's authored
`#RRGGBB` to linear by the format's existing hex→linear law
(§15.8) **before** the track carries it, so a `Color` track
never holds an sRGB value". *Tool:* check 4 fails the `[f32; 3]`
array (and the `{hex, shade}` object) as authored values, citing
the conversion law — the array is the lane's *internal* shape,
never a file spelling. This is the draft winning over the
shorthand, recorded here so the lead can overrule if the format
ever grows a literal-lane spelling.

**D6 — A construct-free file: input error or trivial pass.**
`check_clips.py`'s contract (its docstring): exit 2 covers a
file that "carries neither a `clips` block nor any clip
instance". But draft §15.10 step 3: "a production scene or film
file may use prefabs or clips only after steps 1–2 have landed,
and both validators run over it in tools-ci (exit 0 = pass, 1 = a
check failed, 2 = input error — the repo tool convention)".
Running the Batch-2 clips validator over today's (construct-
free) production corpus would exit 2 on every file — the
adoption gate cannot be built on that. *Tool:* the census
decides; checks 2/3 SKIP on construct-free files and the file
passes (exit 0). `check_prefabs.py` already had the matching
posture (a library file's expansion is "SKIPPED (not passed)",
W14 §10); the clips side is the one this tool reconciles.

**D7 — The §14.4 camera ease vocabulary is binary in the
loader.** Draft §15.7's table calls `linear` "the one ease name
v2 already spells (§14.4)", and §15.6's arriving-key placement
is "the same placement as §14.4". Measured loader behaviour
(`src/scenefile.rs:788`): `let linear = k.get("ease")
.and_then(Value::as_str) == Some("linear");` — any other ease
string on a §14.4 camera key is silently treated as
smootherstep; the full `Easing::from_name` vocabulary (verified
at HEAD: the same ten spellings `check_clips` lists) exists only
for clip keys. *Tool:* the vocabulary check applies to clip keys
(via check 3); §14.4 `camera.keys` eases are **not** policed by
this tool — flagging them against the clip vocabulary would
false-fail the base format's own data model.

**D8 — poseTrack conversion is described twice; they agree, and
the residue is checkable.** W15 §5.2: "each poseTrack key
`[t, v, m]` with `m > 0` converts to **two** clip keys: a hold
key `{t: t - m, v: <previous value>}` (unless one already sits
there) and the arrival key `{t: t, v: v, ease: "smoother"}`".
W31 flag 5 records that ANIM-W11 §4.3's evaluator-side encoding
differs in spelling ("the arrival pair plus `Step` on the
preceding key") and that "composed through §15.9's remap they
produce the same evaluator track". *Tool:* only the file-side
half is checkable, and check 6 checks it — a clip key carrying
`move`/`moveSecs` is un-converted poseTrack data and fails.

**D9 — Pids in param-supplied subtrees: carried open by both
texts.** W14 §10: "pids inside param-supplied subtrees are
stripped at expansion but are not addressable by overrides — the
subtrees arrive after the merge step, §5 step 5c/5e ordering".
Draft flag 3(b) carries the same question undecided. *Tool:*
such a pid is a guard **NOTE**, not a failure (§15.5 step 5f
strips it; nothing in either text forbids it) — the one place
this tool reports a §15 construct without a verdict.

**D10 — Guard scope: W14 names two keys, §15.1 names the whole
reserved set.** W14 §7: "encountering `"prefabs"` at the top
level, or `"prefab"` in a node position, is a **hard error**
naming the unsupported extension". Draft §15.1 widens it to
"a top-level `"prefabs"` or `"clips"` block, a `"prefab"` key in
a node position, or a `"clip"` field on any node or block", and
adds: "The guard applies to every consumer that walks nodes,
including `film.rs`'s own node walk". *Tool:* the census covers
the widened set (`prefabs`, `clips`, `prefab`, `clip`,
`clipTime`, `pid`, `$param`).

---

## 4. How the guard sees today's loader (why check 1 exists)

Draft §15.1, measured: "a top-level `"prefabs"` block is never
read — silently ignored, per §1 — while an instance node already
fails loud, because `walk_nodes` requires a `type` on every
node, but with a misleading missing-`type` message that never
mentions prefabs." The guard check is the data-side stand-in for
that named error until §15.10 step 1 lands it in
`scenefile.rs`/`film.rs`: any file whose reserved keys appear in
a shape the extensions cannot honour is named here, at the gate,
instead of being dropped there, at render time.

---

## 5. Self-test — fixtures (verbatim transcript + result table)

Command shape: `python3 tools/format/check_format15.py <fixture>`
(no `--library`; every fixture is self-contained).

The valid composite, verbatim:

```
check_format15.py — tools/format/fixtures/format15/valid-composite.scene.json (§15 census: prefabs block, 4 prefab instance(s), clips block, 6 clip instance(s))

[1 guard] PASS — reserved keys present and in §15 positions (11 instance/pid sites)

[2 prefabs] PASS — via check_prefabs.py: 6/6 sub-checks passed

[3 clips] PASS — via check_clips.py: 5/5 sub-checks passed

[4 colour lane] PASS — 2 colour-channel track(s) authored as hex

[5 step channels] PASS — 2 step-channel track(s) declared legally

[6 placement] PASS — duration on clips, wrap on instances, no residue
    note: clips.look-around.tracks[2].keys[0].ease: the first key's ease ('step') has no incoming segment and is ignored (§15.6) — dead data
    note: clips.look-around.tracks[3].keys[0].ease: the first key's ease ('step') has no incoming segment and is ignored (§15.6) — dead data

OVERALL: PASS (6/6 checks passed)
exit=0
```

(The two notes are the §15.6 first-key rule firing on the step
tracks — W15's own valid fixture spells step keys the same way;
the note is the checker doing its one non-verdict job.)

All eleven fixtures:

| fixture | exit | failing checks | as designed |
|---|---|---|---|
| `valid-composite.scene.json` | 0 | — (2 notes) | PASS 6/6 |
| `note-first-key-ease.scene.json` | 0 | — (1 note) | PASS, note printed |
| `bad-guard-version.scene.json` | 1 | 1 guard | v1 file + clips block |
| `bad-guard-shape.scene.json` | 1 | 1 guard, 2 prefabs (O1) | `prefabs` as an array |
| `bad-guard-stray-pid.scene.json` | 1 | 1 guard | pid on `camera` |
| `bad-guard-stray-cliptime.scene.json` | 1 | 1 guard | `clipTime`, no `clip` |
| `bad-prefab-closed-key.scene.json` | 1 | 2 prefabs | instance key `weight` |
| `bad-clip-ease.scene.json` | 1 | 3 clips | ease `bounce`; prefabs half passes |
| `bad-colour-lane.scene.json` | 1 | 4 colour lane, 3 clips (O3) | `[f32; 3]` colour value |
| `bad-step-channel.scene.json` | 1 | 5 step channels, 3 clips (O2) | mood key eased `smoother` |
| `bad-placement.scene.json` | 1 | 6 placement (2 violations) | per-track `duration` + `moveSecs` |

Input-error contract: `python3 tools/format/check_format15.py
/nonexistent.json` → `ERROR: check_format15.py: cannot read
/nonexistent.json (No such file or directory)`, exit 2.

---

## 6. Regression — the Batch-2 fixture sets through this tool

Delegation must not change a single Batch-2 verdict. Runs mirror
W14 §9's and W15 §6's own invocations:

- Prefabs, `--library scenes/fixtures/prefabs/library.prefabs.json`:
  `valid.scene.json` exit **0**; `invalid-unknown-prefab`,
  `invalid-missing-param`, `invalid-extra-param`,
  `invalid-param-type`, `invalid-override-forbidden`,
  `invalid-override-transform`, `invalid-override-unknown-pid`
  each exit **1**.
- Prefabs, bad libraries as `--library` (W14's prescribed runs):
  `invalid-cycle` exit **1**; `invalid-slot-undeclared` exit **1**.
- Clips, no library: `valid-clips.scene.json` exit **0**;
  `bad-clip-cycle`, `bad-duplicate-key-time`,
  `bad-grade-time-r3`, `bad-key-beyond-duration`,
  `bad-unknown-channel`, `bad-unknown-easing`,
  `bad-unsorted-keys` each exit **1**.

18/18 verdicts identical to running the two validators directly.

*(Process note, for the record: this seat's first regression loop
printed `exit=0` for the whole clips set — the `$(basename …)`
inside its own `echo` reset `$?` before it was read. The tool's
printed `OVERALL: FAIL` lines contradicted the loop, the loop was
re-run with the code captured first, and the table above is from
the corrected run, spot-verified by invoking
`bad-unknown-easing.scene.json` directly: exit 1.)*

---

## 7. Real-data sweep — all of `scenes/` + `film/` (78 JSON files)

Library-less sweep, one invocation per file (so one file's exit 2
cannot mask another's verdict):

- **58 files: OVERALL PASS.** 56 of them report the guard
  headline "no §15 constructs — no reserved keys anywhere" and
  pass on check 1 alone (checks 2–6 SKIPPED).
- **Every production file passes trivially, and no production
  file trips the reservation guard.** The complete non-pass list
  is confined to the Batch-2 extension fixtures and one
  asset-checker fixture (below). In particular
  `scenes/*.scene.json`, `scenes/film/*`, `scenes/sets/*`,
  `film/onoda.json`, and the `fx-*`/`lights`/`assets` fixture
  sets contain **no** `prefabs`, `clips`, `clip`, `clipTime`,
  `pid`, or `$param` keys — the measured state §15.10 assumes
  (production data last; the extensions are not in use yet).
- **20 non-pass, each accounted for:**
  - the 7 `bad-*.scene.json` clips fixtures — FAIL at their
    designed checks, by design (§6);
  - 12 of the 13 files in `scenes/fixtures/prefabs/` — the 9
    invalid cases fail by design (the 2 bad `.prefabs.json`
    libraries fail run positionally, as their definitions are
    themselves invalid), and `valid.scene.json` fails **only**
    because a library-less sweep cannot see `marker-post` /
    `post-pair` ("unknown prefab … (in scope:
    ground-disc-marker)"); with its prescribed `--library` it
    passes, exit 0 (§6). `library.prefabs.json` itself passes
    positionally;
  - `scenes/fixtures/assets/wrong-kind/assets/plates.json` —
    exit **2**: the file is the planted JSON array
    `["not", "an", "object"]` (check_assets' wrong-kind
    fixture), and this tool's inputs are scene/film-data
    *objects*; the exit-2 input-error contract is the correct
    response, stated here so nobody reads it as a §15 finding.

---

## 8. Known limits and hygiene

- **Census position model.** "Node position" for a `prefab` key
  is tracked as *reachable under `nodes`, a template, or an
  instance's `params`* — the census does not model base-format
  sub-positions, so a `prefab` key nested inside a node's
  `material`/`geometry` sub-object is treated as an instance
  position (and validated as one by check 2). §15.2's finer
  exclusion ("`attachments`, `geometry`, `material`, `textures`,
  `lights` are not node positions") is enforced only at the
  block level (`camera`/`lights`/… outside `nodes`). This
  matches `check_prefabs`' own walk, which finds instances
  anywhere under `nodes`; tightening it belongs with the
  loader-side guard, not in a fork here.
- **`--library` files are guarded only through check 2.** The
  census runs on the positional files; a library's own reserved
  keys are validated by `check_prefabs`' library check (its
  templates are legal pid/slot territory by construction).
- **No f32 check of this tool's own.** Clip numbers ride
  `check_clips` check 5; §15.10 leaves the expanded document's
  §11 law to `validate_film.py`, unchanged.
- **First-key-ease notes fire on W15's own spelling** (step keys
  carry `ease: "step"` from key 0). That is §15.6 applied
  literally ("the first key's `ease`… is ignored, exactly as the
  §14.4 loader ignores it"); if the lead prefers step tracks
  exempt, the note — not any verdict — is the only thing that
  changes.
- **Hygiene.** This seat wrote only the three deliverables in
  the table above and committed nothing. One side effect to
  flag for the central commit: importing the sibling validators
  refreshes the *tracked* bytecode artefact
  `tools/format/__pycache__/check_clips.cpython-312.pyc` (the
  tree shows it modified; sibling seats import the same modules,
  so the refresh is not attributable to one seat). The tool now
  sets `sys.dont_write_bytecode` so its own future runs do not
  add to it; no source file was edited.
