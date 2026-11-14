# KNOBS-W43-TAIL-W61 — restoration pack for the truncated KNOBS-W43 tail

Seat W61 (replacement run), Wave 2 Batch 7. New file only — no
existing file edited, nothing committed. The lead owns
KNOBS-W43.md; this pack is the reviewable restoration artefact
for its lost tail.

**What happened (W54 §0.3, verified first-hand at this seat's
HEAD):** the committed `docs/integration/KNOBS-W43.md` ends
mid-§4.2. Its last newline-terminated line is 263 (a dangling
bullet marker), followed by the literal text
`...[truncated 3075 chars]` on an unterminated line 264. Its
§4.3 (shadow), §5 (acceptance) and §6 (risks) never reached
the repo.

**Sources — the only ones used, per the seat's brief:**

1. KNOBS-PATCH-W54.md — §0.3 (the truncation disclosure and
   its reconstruction basis) plus the parts of the pack that
   record or implement the lost sections (§4, §5 F1–F8, §6
   A1–A6, §9.2, §9.3, §9.4, §10).
2. The surviving KNOBS-W43.md text's own forward references
   (§3.2 → §5 and → "the resolve-target selection in §4.2";
   §1.1's resolve-seam paragraph; §1.2's lockstep survey).
3. KNOBS-W43's survey data (§§1.1–1.3) as cited in W54.

Nothing is invented. Where a paragraph's substance is W54's
own extension rather than recovered W43 text, the appendix
tags it `[EXTENSION by W54]`. Where the original held content
no source records, the slot is listed in §3 as UNRECOVERABLE
for the lead to re-derive — it is not written into the tail.

---

## 0. Splice instruction for the lead

### 0.1 Exact truncation point

- File: `docs/integration/KNOBS-W43.md`.
- Line 262 is the last complete line. The last intact
  sentence (the §4.2 "Pipelines:" bullet, lines 258–262):

  > "Pipelines: the literal `count: 4` states (ab1.rs:927;
  > film.rs:2606; scenefile.rs:1635, 1679; dof.rs:281) and
  > the `make_basic` scene variants' `4` arguments become
  > `samples`. The overlay variant stays `1`; the
  > `MultisampleState::default()` prepass/shadow pipelines
  > stay untouched (§1.1)."

- Line 263 is a lone `-` (the next §4.2 bullet, cut off after
  its marker). Line 264 is the literal
  `...[truncated 3075 chars]`, with no trailing newline at
  EOF.

### 0.2 Where the tail attaches

1. Delete line 263 (`-`) and line 264 (the marker).
2. Append the fenced text in §1 of this pack verbatim. It
   continues §4.2 directly (the `### 4.2` heading already
   stands in the surviving file — the tail does not repeat
   it), then supplies `### 4.3`, `## 5` and `## 6`.
3. One paragraph needs a decision at splice time: **T2 is
   W54's extension** (the attachment-construction shape),
   tagged as such in the §2 appendix. Splice it only if the
   pack's prescription should be recorded in the spec; the
   pack remains its record either way. W54's own alternative
   also stands: let KNOBS-PATCH-W54 stand as the §4.3+
   record and restore nothing.
4. The tail's file:line cites follow the surviving
   document's own basis (HEAD `f6616aa`, per its header) and
   agree with §1.2's survey cites verbatim — they are not
   re-based to W54's `aa92cee` numbers or to today's HEAD.

### 0.3 Verification step (run at splice time)

The restored document must not contradict any line number
W54 cited from src/. Spot-check these five cites **by
content** (numbers drift as the lead's commits land; content
is the invariant):

| # | W54 cite | content to find |
|---|---|---|
| 1 | film.rs:2545 (§3 table, F3) | `gpucore::make_target(…, Rgba16Float, 4, …, "hdr msaa")` |
| 2 | ab1.rs:652 (A1) | `[1.0 / 2048.0, 2048.0, 0.0, 0.0],` |
| 3 | shader_pbr.wgsl:51 (§9.3) | `// x: shadow texel size (1/2048), y: shadow map size (2048)` |
| 4 | dof.rs:240 (D1) | `gpucore::make_target(…, Rgba16Float, 4, …, "dof-scene-msaa")` |
| 5 | main.rs:174–175 (H1) | `let mut width: u32 = 1080;` / `let mut height: u32 = 1920;` |

This seat ran all five at HEAD `4dd6530`: **#2, #3, #4, #5
exact at the cited lines; #1 content-identical but drifted
to film.rs:2567 (+22)** — the lead's a2ec80c (loud fallbacks
in `load_assets`) and 4dd6530 (§15 guard in
`load_film_data`) both added lines earlier in film.rs after
W54 surveyed it; the twin `shadow_misc` literal drifted
identically (2838 → 2860). No cite is contradicted.

Two further checks:

- The tail's §4.3 cites (ab1.rs:809–820, 652; film.rs:2528,
  2531, 2838) equal KNOBS-W43 §1.2's survey cites verbatim —
  the tail introduces no new survey numbers.
- No tail paragraph asserts a decision W54 lists as its own
  (§9.2, items 1–5: last-writer-wins, the uniform tag rule,
  attachment construction, texture-still-created, the dof
  description line) except T2, which is present precisely as
  the tagged extension of §9.2 item 3.

**Honesty note:** the marker claims 3,075 characters were
lost. This restoration is substance-level, assembled from the
sources above — it is not, and does not claim to be, a
recovery of the original bytes or wording.

---

## 1. The proposed restored tail

Block map (appendix keys, in order): **T1** §4.2 resolve
bullet · **T2** §4.2 construction paragraph · **T3** §4.2
risk bullet · **T4** §4.3 threading bullet · **T5** §4.3
uniform bullet · **T6** §5 gate paragraph · **T7** §5
mechanism paragraph · **T8** §6 adapter-risk bullet.

Everything between the fences is the proposed splice text:

```
- Resolve selection: at samples > 1 the passes keep
  `resolve_target: Some(&hdr_view)` exactly as today (§3.2).
  At samples == 1 the pass renders directly into the resolved
  target's view with `resolve_target: None` — wgpu forbids a
  resolve target on a single-sample attachment, so `--aa 0`
  cannot be had by changing counts alone (§1.1). The branch
  is in the attachment, in these two passes; every downstream
  consumer reads the resolved texture and does not branch.
- Construction: build the colour attachment before the pass,
  choosing view and resolve together — at samples > 1 the
  constructed attachment is field-for-field today's (same
  view, `Some` resolve, same ops); at samples == 1 it is the
  resolved view with `None`. The depth attachment is not
  branched: the twin is created at the same `samples`, so it
  matches the colour attachment's count in both arms.
- Risk, accepted: at samples == 1 the MSAA target is still
  created (at count 1) and never bound — 16.6 MB of fp16 at
  1080×1920, created and dropped. Only draft runs pay it;
  making its creation conditional is not prescribed.

### 4.3 `--shadow`

- Thread one `shadow` size from main.rs into `run_ab1` /
  `run_film` — the only two paths that build a shadow map
  (§1.2); `--shadow` is inert in the other modes (§3.3). At
  each site the size replaces the literal 2048 in both target
  creations: ab1's inline 2048×2048 Rgba8Unorm
  (ab1.rs:809–819) and its `composer::make_rt(2048, 2048,
  Depth32Float)` depth (ab1.rs:820); film's
  `gpucore::make_target(2048, 2048, Rgba8Unorm, 1, …)`
  (film.rs:2528) and `make_rt` depth (film.rs:2531).
- The `shadow_misc` uniform moves in lockstep (§1.2):
  `[1.0 / 2048.0, 2048.0, 0.0, 0.0]` becomes
  `[1.0 / size, size, 0.0, 0.0]` at ab1.rs:652 and
  film.rs:2838. The size reaches the shader only through
  that slot, so no WGSL changes; the `shader_pbr.wgsl:51`
  comment is reworded to drop the baked-in numbers (§1.2).
  At 2048 the uniform is bit-identical to today's literal.

## 5. Acceptance

The acceptance gate is §3.2's identity law, executed: for
each knob singly and for all three combined, a run spelled
at explicit defaults (`--res 1 --aa 4 --shadow 2048`) must
produce byte-identical frames to a run with no knob flags,
and print exactly the lines it prints today — no DRAFT tag
at defaults (§3.4).

The film legs run on `tools/ci/byte_probes.py` over the
standing probe ranges (frames 195–204, 795–814,
1195–1204): the no-flag and explicit-default renders must
PASS byte-identical. Complementary smoke: a `--draft` run
must produce frames that differ from the defaults run —
identical output would mean the knobs are decorative — and
must carry the DRAFT tag in its header, progress and
completion lines (§3.4).

## 6. Risks

- `--aa 8` on `Rgba16Float` exceeds some adapters'
  sample-count limits. Where it does, texture/pipeline
  creation fails as a wgpu validation error — a hard,
  visible failure, which is the lawful outcome: the
  no-silent-change law cuts both ways, and 8 is never
  clamped to 4. The §5 gate does not exercise `--aa 8`;
  whether RADV accepts it is recorded at the PC evidence
  session. The same applies to `--shadow 4096` on
  downlevel-class limits — visible failure, no clamp.
```

---

## 2. Appendix — provenance, paragraph by paragraph

| ID | Tag | Basis |
|---|---|---|
| T1 | [INFERRED from forward-reference — lowest confidence] | The rule is assembled from W43's own surviving words: §3.2's "the resolve-target selection in §4.2 must yield `Some(...)` at samples > 1 exactly as today" and §1.1's resolve-seam paragraph (sites film.rs:3173–3178, ab1.rs:1414–1419; "the pass must render directly into the resolved target's view with `resolve_target: None` when samples == 1"; downstream works on the resolved texture). That §4.2 held this bullet is proven by the §3.2 reference; its original wording is not (see U1). |
| T2 | [EXTENSION by W54] | KNOBS-PATCH-W54 §4, named as an extension in its §0.3 ("the attachment-construction shape in §4") and reaffirmed in its §9.2 item 3. Not recovered W43 text — see splice instruction §0.2 item 3. |
| T3 | [RECONSTRUCTED from W54 §4 + §9.2] | W54 §4: "Risk (named in W43, restated)" — the created-and-dropped MSAA texture at samples == 1, 16.6 MB fp16 at 1080×1920; §9.2 item 4 records the still-created decision. The fact and figure are W43's per W54's attribution; the not-prescribed stance is the pack's. Original placement (§4.2 vs §6) is unrecorded — see U5. |
| T4 | [RECONSTRUCTED from W54 §5 F3 + §6 A2] | The threading and per-site replacements are implemented in W54's film/ab1 hunks; the decision substance (two shadow paths only, size in both target creations) is W43's surviving §1.2 survey, and inertness elsewhere is §3.3. Cites are §1.2's, verbatim. |
| T5 | [RECONSTRUCTED from W54 §5 F5 + §6 A1 + §9.3] | Lockstep is §1.2's own word ("a matching uniform slot that must move in lockstep"); the slot's new form is W54 F5/A1; the bit-identity at 2048 is W54's power-of-two argument (F5); the comment rewording is asked for in §1.2 itself, with W54 §9.3 supplying the replacement text (not quoted in the tail). |
| T6 | [RECONSTRUCTED from W54 §10] | W54 §10 implements "W43 §5's acceptance"; its leg A is exactly this gate ("the per-knob gate as well as the combined one"), including the no-DRAFT-at-defaults condition. §5's existence and subject are independently proven by the surviving §3.2 forward reference (§5 = "the acceptance gate"). |
| T7 | [RECONSTRUCTED from W54 §0.3 + §10] | §0.3 names the two §5 elements recovered from W43's seat report — "defaults gating via byte_probes' ranges, the --draft differ-smoke" — and §10 gives the ranges (195–204, 795–814, 1195–1204, `byte_probes.DEFAULT_RANGES`) and the differ smoke (leg B). The DRAFT-tag lines are §3.4's surviving list (header, progress, RunLog completion). |
| T8 | [RECONSTRUCTED from W54 §9.4] | §9.4 is headed "(from W43's report, restated)": MSAA 8× on Rgba16Float vs adapter limits, hard visible failure, never clamp 8 → 4, gate does not exercise it, RADV recorded at the PC session; `--shadow 4096` on downlevel limits likewise. |

---

## 3. UNRECOVERABLE register — proven to have existed, not sourced

| ID | Slot | What proves it existed | Why it stays unwritten |
|---|---|---|---|
| U1 | §4.2's resolve bullet in its original wording | §3.2's reference to "the resolve-target selection in §4.2"; §1.1's seam paragraph | T1 reconstructs the rule from §1.1/§3.2's own words; the bullet's original phrasing and any further prescription detail are in no source. |
| U2 | Any §4.2 bullets beyond the Pipelines / resolve / risk items | The dangling `-` at line 263 proves at least one further bullet began where the file was cut | Only the resolve topic is recoverable (via §3.2). Any other bullet's subject is unknown. |
| U3 | §4.3's original prose, and any §4.3 decision not carried into W54's hunks | W54 §0.3 names "§4.3 (shadow)" as lost | T4/T5 restore only what §1.2's survey + W54's hunks jointly record. |
| U4 | §5 content beyond T6/T7 — incl. the gate's leg structure as originally specified, and any still-mode probe sets | §3.2's "(§5)" forward reference; W54 §0.3's two recovered elements | W54 §10's third leg (CLI validation) implements §2's surviving rules but is not attributed to W43's report in §0.3, and no in-scope source records still-mode (ab1/dof/scene-file) acceptance probes — neither confirmed nor denied as original §5 content. The lead re-derives if §5 needs them. |
| U5 | §6's contents beyond T8 — and T3's original placement | W54 §0.3 names "§6 (risks)" as lost; §9.4 attributes only the adapter-risk item to W43's report | No source enumerates §6's items. Whether the samples==1 texture risk (T3) stood in §4.2 or §6 is likewise unrecorded. |
| U6 | The original tail as text (wording, order, length) | The marker's own claim: 3,075 chars | The restoration is substance-level from secondary sources. Byte recovery is impossible from the repo; only W43's original seat report, if retained outside the repo, could supply it. |
