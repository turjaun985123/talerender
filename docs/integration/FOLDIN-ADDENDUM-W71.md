# FOLDIN-ADDENDUM-W71 — restored §4 for FOLDIN-W65

**Seat:** TaleRender Wave 2, W71 (Batch 8). **Date:** 2026-11-15.
**Status:** addendum only. Nothing committed; no existing file
edited — in particular `docs/integration/FOLDIN-W65.md` is NOT
edited by this seat. The lead splices this file's §4 text into
FOLDIN-W65.md in place of the truncated tail.

**Why this file exists.** Batch 7's gate (GATE2-W70 §4) found
FOLDIN-W65.md truncated at 8,310 bytes / 173 lines: it ends
mid-sentence inside §3's suspect table (the R-gradetime row),
followed by the literal marker `...[truncated 3072 chars]`.
Its §4 — the post-fold acceptance section the introduction
points at ("the run against the current document … passes §4's
bar") — never reached the repo. Same transport-truncation class
as KNOBS-W43.md (3,075 chars there, 3,072 here); second
occurrence in the wave. The fold-in itself was never in doubt:
the gate re-ran the checker three ways and the acceptance
numbers stand (GATE2-W70 §3). What was missing is the note's
own §4 text. This addendum restores it.

**Writing method, stated as required:** this file was written
in **two append chunks** (chunk 1: header + restored §4.1–§4.4;
chunk 2: §4.5 evidence + provenance + close), per the standing
rule the gate proposed for seat notes over ~8 KB after the two
truncations.

**Splice instruction for the lead:** in FOLDIN-W65.md, delete
the truncated tail (from the cut point inside §3's table to
EOF, including the `...[truncated 3072 chars]` marker) and
append the section below, headed `## 4.`, as the note's §4.
The §3 table's lost rows are NOT restored here — they belong
to §3, and the gate's §3 re-run already stands in for them;
this addendum restores §4 only, as commissioned.

---

## 4. Post-fold acceptance — the run, its bar, and how to read it

*(Restored text. Provenance per paragraph is tabulated at the
end of this addendum.)*

### 4.1 The acceptance run

The post-fold acceptance run is a single default invocation,
run from the repo root, no arguments:

```
python3 tools/format/check_foldin.py
```

Defaults resolve against the repo root (two directories up
from the tool): draft
`docs/integration/FORMAT15-DRAFT-W31.md`, target
`docs/scene-format.md`. The census is the full §15 census of
§1: 11 headings + 46 normative sentences + 18 named rules =
75 items. `--rules=off` is for the self-test fixtures only and
is never an acceptance run.

**The bar is one exact line.** The run passes if and only if
its final line reads, verbatim:

```
OVERALL: PASS (75/75 FOUND, 0 ALTERED-SUSPECT, 0 MISSING)
```

Any other OVERALL line — any count below 75 FOUND, any
ALTERED-SUSPECT, any MISSING — is a failed acceptance, however
close it looks. The census counts (75 = 11 + 46 + 18) are part
of the bar: if a future draft legitimately changes the census,
the bar line changes with it and the change is recorded in the
commit that changes the draft — never silently re-baselined
(see §4.4).

### 4.2 Exit-code readings

Per the repo convention recorded in §1:

| Exit | Meaning | Acceptance reading |
|---|---|---|
| 0 | every census item FOUND | PASS — the fold is complete against the current draft |
| 1 | any ALTERED-SUSPECT or MISSING | FAIL — the fold dropped or weakened at least one clause; read the flagged lines (§4.4) |
| 2 | input error (`ERROR:` line on stderr, no report) | NOT A VERDICT — the run did not happen (missing/unreadable draft or target); fix the invocation or the paths and re-run |

An exit-2 run must never be recorded as a fold failure, and a
missing target must never be read as "all clauses MISSING":
the tool prints no report in that case precisely so the two
cannot be confused.

### 4.3 Re-run triggers

Re-run the acceptance run, and record its OVERALL line, on any
commit that touches:

- **§15 of `docs/scene-format.md`** — the folded text itself
  (any edit inside the §15 region, currently lines 559–end of
  §15.10 in the document at the run in §4.5);
- **the draft** — `docs/integration/FORMAT15-DRAFT-W31.md`
  (a draft edit changes the census the fold is judged against);
- **the named-rule table** — `NAMED_RULES` in
  `tools/format/check_foldin.py` (a rule edit changes what
  counts as FOUND for the R census);
- **W51's disagreements** —
  `docs/integration/FORMAT15-CHECK-W51.md` (D1–D10 are folded
  into the census where they bite; a change there can change
  the census without either document moving).

A commit touching none of the four does not need a re-run. A
re-run triggered by a draft/census change must state the old
and new census counts alongside the OVERALL line, so a changed
bar is never mistaken for the old one.

### 4.4 The reading rule

- **FOUND = survived.** An item at FOUND (normative sentences:
  containment ≥ 0.75 with no strong keyword lost; at the
  `4dd6530` fold every normative sentence sits at containment
  1.00) needs no further reading.
- **Read ALTERED-SUSPECT and MISSING lines individually.**
  Each flagged line names the item, its draft section, the
  target line it best matched (or —), and its containment.
  ALTERED-SUSPECT with "strong keyword lost" is a *weakening*
  until a human reads the target sentence and rules otherwise;
  MISSING means the clause is absent *as far as the matching
  rule can tell* — a rewording past the containment floor also
  reads MISSING (§1's false-negative behaviour), so a MISSING
  verdict opens a human check of the target section, it does
  not close one.
- **Never re-baseline by editing the target to please the
  tool.** If a flagged clause was deliberately changed, the
  change is made in the *draft* (or recorded as a superseding
  ruling) and the checker is re-run against the new census —
  the target document is the thing being verified, not the
  thing being tuned. Editing scene-format.md's §15 wording
  merely to raise a containment score invalidates the run.


### 4.5 Evidence — the acceptance result, twice over

**Of record (the gate's run).** GATE2-W70 §3 ran the checker
against the live document folded at `4dd6530`:

```
OVERALL: PASS (75/75 FOUND, 0 ALTERED-SUSPECT, 0 MISSING)
```

— the `4dd6530` fold-in complete against the W31 draft's full
census (11 headings + 46 normative sentences + 18 named
rules). The same gate re-ran the pre-fold snapshot
(`3a3a427:docs/scene-format.md`) and reproduced §3's baseline
exactly: `OVERALL: FAIL (0/75 FOUND, 5 ALTERED-SUSPECT,
70 MISSING)`, and the fixtures: `SELF-TEST: PASS 3/3`.

**This seat's re-run (HEAD `b2e46d2`).** W71 re-ran the
default invocation at the current HEAD — two lead commits past
`4dd6530` (F2 migration phases D and L+T, the latter adding a
film-data appendix to `docs/scene-format.md`). Verdict:
**agreement — 75/75, exit 0.** The interval edits did not touch
any census item; every normative sentence still reads at
containment 1.00. Verbatim output:

```
check_foldin.py — target /home/hatch/workspace/talerender/docs/scene-format.md
  draft /home/hatch/workspace/talerender/docs/integration/FORMAT15-DRAFT-W31.md (paste text lines 36–735)
  census: 11 headings, 46 normative sentences, 18 named rules

[H headings]
  FOUND           §15     Format v2 extensions: prefabs and clips  (target line 559)
  FOUND           §15.1   The reservation guard (fail loud, never silently ignore)  (target line 576)
  FOUND           §15.2   Prefabs: grammar  (target line 608)
  FOUND           §15.3   Prefab params: typing, substitution, and the f32 law  (target line 713)
  FOUND           §15.4   Prefab overrides and transforms  (target line 764)
  FOUND           §15.5   Prefab expansion and cycles (normative)  (target line 841)
  FOUND           §15.6   Clips: the data model  (target line 898)
  FOUND           §15.7   Clips: the channel vocabulary  (target line 994)
  FOUND           §15.8   Clips: evaluation semantics  (target line 1087)
  FOUND           §15.9   Reconciliation rulings  (target line 1158)
  FOUND           §15.10  Adoption order  (target line 1228)

[N normative sentences]
  FOUND           §15.1   (MUST) A v2 loader that does not implement one of these extensions must **hard-error, naming the uns...  (target line 578, containment 1.00)
  FOUND           §15.1   (FORBIDDEN) Silently dropping them is forbidden.  (target line 578, containment 1.00)
  FOUND           §15.1   (NEVER) Measured state of today's loader (`src/scenefile.rs`), so the guard's work is named exactly: ...  (target line 593, containment 1.00)
  FOUND           §15.1   (MUST/MUST-NOT) The guard applies to every consumer that walks nodes, including `film.rs`'s own node walk, wh...  (target line 593, containment 1.00)
  FOUND           §15.2   (NEVER) The block may appear **inline** in a scene file, and/or in a **library file** — a JSON object...  (target line 622, containment 1.00)
  FOUND           §15.2   (HARD-ERROR/MUST) **No shadowing:** a name defined by two sources is a hard error naming both sources — the sam...  (target line 622, containment 1.00)
  FOUND           §15.2   (ALWAYS) A prefab has a **single root node**, so an instance always occupies exactly one node position.  (target line 630, containment 1.00)
  FOUND           §15.2   (REQUIRED) **Param declarations** are a closed key set — `type` (required), `required` (optional boolean...  (target line 637, containment 1.00)
  FOUND           §15.2   (MUST/REQUIRED) Every param is exactly one of: `required: true` (the instance must supply it) or carrying a `...  (target line 649, containment 1.00)
  FOUND           §15.2   (ALWAYS/MUST/REQUIRED) A required param with a default, and a param that is neither, are both definition errors — a ...  (target line 649, containment 1.00)
  FOUND           §15.2   (MUST/MUST-NOT) A `default` must type-check against its own declaration and must be **literal**: it may not c...  (target line 649, containment 1.00)
  FOUND           §15.2   (NEVER) Slots are lexically scoped — a slot inside prefab P's template names a param of P, never of a...  (target line 657, containment 1.00)
  FOUND           §15.2   (MUST-NOT) A slot may not be an object key, may not be the value of `"prefab"` (instantiation targets ar...  (target line 657, containment 1.00)
  FOUND           §15.2   (REQUIRED) The key set is closed: `prefab` (required, literal string), `params`, `overrides` (§15.4), `t...  (target line 686, containment 1.00)
  FOUND           §15.2   (HARD-ERROR) **Any other key is a hard error** — a deliberate departure from §1, mirroring §1's own other ...  (target line 686, containment 1.00)
  FOUND           §15.2   (MUST-NOT) Instances may appear anywhere a node may appear (the top-level `nodes` array, `group` childre...  (target line 686, containment 1.00)
  FOUND           §15.2   (NEVER) A `pid` is not an `id` — the base format's `id` fields name entries in closed, loader-level r...  (target line 702, containment 1.00)
  FOUND           §15.2   (MUST/NEVER) Pids are scoped to their prefab's **direct scope** (the template subtree, not descending into...  (target line 702, containment 1.00)
  FOUND           §15.3   (NEVER) `number` a JSON number (never a boolean) subject to the f32 law below  (target line 720, containment 1.00)
  FOUND           §15.3   (MUST) The forwarded slot's declared type in the outer prefab must match the inner param's declared ...  (target line 754, containment 1.00)
  FOUND           §15.3   (MUST/REQUIRED) A template must be **self-contained**: a nested instance that omits an inner required param i...  (target line 754, containment 1.00)
  FOUND           §15.4   (FORBIDDEN) An override may **add** keys the template node does not carry (adding `roughness` to a materi...  (target line 795, containment 1.00)
  FOUND           §15.4   (MUST-NOT) **What may NOT be overridden** (at any depth inside the partial object):  (target line 807, containment 1.00)
  FOUND           §15.4   (NEVER) `transform` Transforms compose (§15.4 below); they are never replaced.  (target line 818, containment 1.00)
  FOUND           §15.4   (FORBIDDEN/NEVER) **Transforms compose; they never replace.** A template root's own `transform` is part of the ...  (target line 822, containment 1.00)
  FOUND           §15.4   (NEVER) An instance's `name`, when present, is set on the outermost expanded node (the wrapper if the...  (target line 834, containment 1.00)
  FOUND           §15.5   (HARD-ERROR) Duplicate name → hard error.  (target line 848, containment 1.00)
  FOUND           §15.5   (REQUIRED) **Validate every instance** in the scene and in templates: closed key set, known target, requ...  (target line 858, containment 1.00)
  FOUND           §15.5   (FORBIDDEN) **Cycles.** The reference graph of step 3 is **static**: instantiation targets are literal st...  (target line 884, containment 1.00)
  FOUND           §15.5   (HARD-ERROR) A cycle — a prefab whose template transitively instantiates itself — is a hard error, and the...  (target line 884, containment 1.00)
  FOUND           §15.5   (MUST/REQUIRED) A depth guard is still required in implementations (the reference validator uses 64) as belt-...  (target line 884, containment 1.00)
  FOUND           §15.6   (MUST/MUST-NOT) The `uses` graph must be acyclic (self-use counts), every name must resolve, and the layered ...  (target line 958, containment 1.00)
  FOUND           §15.6   (MUST) `clip` — the clip name; must be defined in the same file.  (target line 973, containment 1.00)
  FOUND           §15.6   (MUST) `clipTime` — OPTIONAL; every field defaults: `offset` (default `0.0`, seconds added to the ma...  (target line 976, containment 1.00)
  FOUND           §15.6   (MUST-NOT) **Conflict rule:** `camera` may not carry both `clip` and `keys` (one camera, one time source).  (target line 983, containment 1.00)
  FOUND           §15.6   (REQUIRED) What an instance *means* against static fields: the clip's tracks **replace** the target's st...  (target line 987, containment 1.00)
  FOUND           §15.7   (NEVER) (`i` is the index into the file's `lights.points` array, as written: `points[0].intensity`.) ...  (target line 1020, containment 1.00)
  FOUND           §15.7   (REQUIRED) `step` — hold — discrete channels; **required** on `mood`/`legs` keys  (target line 1085, containment 1.00)
  FOUND           §15.8   (NEVER) **colour:** convert both hex values to linear (the engine's colour pipeline, §4.1/§8), lerp c...  (target line 1109, containment 1.00)
  FOUND           §15.8   (MUST/MUST-NOT) **What must NOT migrate into clips** (the adoption map's negative half; the positive half — `...  (target line 1121, containment 1.00)
  FOUND           §15.9   (MUST/NEVER) **Colour and step channels get their own evaluator lanes.** *Rationale: the evaluator's `Anim...  (target line 1165, containment 1.00)
  FOUND           §15.9   (NEVER) The loader converts each colour key's authored `#RRGGBB` to linear by the format's existing h...  (target line 1165, containment 1.00)
  FOUND           §15.9   (NEVER) Their string values ride a **side channel keyed like the numeric tracks** (same target+channe...  (target line 1165, containment 1.00)
  FOUND           §15.9   (MUST) **Ease is stored on the arriving key; the loader remaps it.** *Rationale: the format's placem...  (target line 1185, containment 1.00)
  FOUND           §15.9   (NEVER) When the loader builds evaluator tracks it remaps **format `key[i+1].ease` → anim `key[i].eas...  (target line 1185, containment 1.00)
  FOUND           §15.10  (HARD-ERROR) **Reservation guard first** (§15.1): the named hard errors on the reserved keys, in `scenefil...  (target line 1232, containment 1.00)

[R named rules]
  FOUND           R-guard      §15.1   reservation guard: hard-error naming the extension, no silent drop  (target line 578, 3/3 phrase groups)
  FOUND           R-guard-film §15.1   guard applies to every node-walking consumer, film.rs included  (target line 593, 2/2 phrase groups)
  FOUND           R-noshadow   §15.2   prefab scope: no shadowing — a name defined twice is a hard error  (target line 622, 3/3 phrase groups)
  FOUND           R-pid        §15.2   pid is stripped during expansion and never reaches the renderer  (target line 702, 3/3 phrase groups)
  FOUND           R-subst      §15.3   substitution is verbatim — no arithmetic, no interpolation, no expressio  (target line 734, 3/3 phrase groups)
  FOUND           R-override   §15.4   overrides deep-merge by pid; params replace wholesale  (target line 766, 2/2 phrase groups)
  FOUND           R-transform  §15.4   transforms compose; they never replace  (target line 818, 2/2 phrase groups)
  FOUND           R-cycles     §15.5   prefab reference cycles are hard errors printing the cycle path  (target line 884, 3/3 phrase groups)
  FOUND           R-depth      §15.5   expansion depth guard (reference validator uses 64)  (target line 884, 2/2 phrase groups)
  FOUND           R-gradetime  §15.7   post.grade.time is NOT a channel — rejected by name (R3)  (target line 1036, 2/2 phrase groups)
  FOUND           R-posetrack  §15.8   poseTrack conversion: hold key + arrival key, evaluates identically  (target line 1121, 3/3 phrase groups)
  FOUND           R-fire       §15.8   fire flicker over the R1 basis 4.6 must not migrate into clips  (target line 1127, 2/2 phrase groups)
  FOUND           R-colour     §15.9   colour lane `Color` holds linear-space [f32; 3]; files author #RRGGBB  (target line 1165, 3/3 phrase groups)
  FOUND           R-step       §15.9   step channels ride a side channel; no interpolation  (target line 1165, 3/3 phrase groups)
  FOUND           R-ease       §15.9   ease remap: format key[i+1].ease -> anim key[i].ease (arriving key)  (target line 945, 3/3 phrase groups)
  FOUND           R-wrapdur    §15.9   duration lives on the clip; wrap lives on the instance  (target line 1200, 2/2 phrase groups)
  FOUND           R-euler      §15.9   rotation channels are euler scalars; no quaternion channel  (target line 1215, 2/2 phrase groups)
  FOUND           R-adopt      §15.10  adoption order: guard first, expansion/evaluation next, production data   (target line 1232, 2/2 phrase groups)

OVERALL: PASS (75/75 FOUND, 0 ALTERED-SUSPECT, 0 MISSING)
```

Run at HEAD `b2e46d2`, exit code 0. (Absolute paths in the
first two lines are this machine's; the run is path-relative
in substance — the same invocation from any checkout of this
HEAD reads the same two files.)

---

## Provenance — where each part of this addendum came from

| Part | Source |
|---|---|
| Header: truncation account (8,310 bytes / 173 lines, marker, mid-§3-table cut) | [FROM GATE2-W70 §4 — the gate found and measured the truncation] |
| Header: "the gate re-ran the checker three ways; acceptance numbers stand" | [FROM GATE2-W70 §3] |
| Header: two-append-chunk writing method + the > ~8 KB standing rule it follows | [FROM GATE2-W70 §4 — the gate proposed the rule; this seat applied it] |
| Splice instruction (delete truncated tail, append as §4; §3 rows not restored) | This seat's mechanics, following [FROM GATE2-W70 §12 item 2 — "commission the restored §4+ … or fold this gate's §3 numbers in by reference"; this addendum does both] |
| §4.1: the single default command, defaults resolution, census 75 = 11 + 46 + 18 | [FROM W65 brief text — the promised §4 spec; command/defaults as documented in FOLDIN-W65 §1 and the tool's own usage text] |
| §4.1: the exact OVERALL bar line | The bar as specified for this restoration; the line itself is the tool's output format, confirmed verbatim by both runs in §4.5 |
| §4.2: exit-code table (0 / 1 / 2, exit 2 is not a verdict) | [FROM W65 brief text — exit semantics as recorded in FOLDIN-W65 §1, restated as acceptance readings] |
| §4.3: re-run triggers (scene-format §15, the draft, NAMED_RULES, W51's disagreements) | [FROM W65 brief text — the promised trigger set; paths spelled out from the repo layout] |
| §4.4: reading rule (FOUND = survived; read flagged lines individually; never re-baseline by editing the target) | [FROM W65 brief text — the promised reading rule; thresholds/containment vocabulary from FOLDIN-W65 §1] |
| §4.5: gate's post-fold OVERALL line + pre-fold baseline + fixtures 3/3 | [FROM GATE2-W70 §3 — the gate supplied §4's missing numbers of record] |
| §4.5: full verbatim run output at HEAD `b2e46d2`, exit 0, agreement verdict | [RE-RUN by this seat at HEAD] |

**Close.** The restored §4 and the gate's §3 agree, and this
seat's independent re-run at `b2e46d2` agrees with both:
**the `4dd6530` §15 fold-in passes its acceptance bar —
75/75 FOUND, 0 ALTERED-SUSPECT, 0 MISSING, exit 0.** No
disagreement to headline. Once the lead splices §4 into
FOLDIN-W65.md, this addendum's remaining job is provenance;
it can be kept alongside the note or dropped at the lead's
discretion.
