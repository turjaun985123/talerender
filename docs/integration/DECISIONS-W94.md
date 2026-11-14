# Six decisions for wave close — DECISIONS-W94

**What this is.** Wave 2 ends with six choices that only you can
make. Each one below is self-contained: what it is, the options,
what each option costs or changes, and a working recommendation —
always labelled as a recommendation, never as a fact. You can
answer them in any order, but two ordering notes help:

- Decisions **1 (captions)** and **2 (the s28 pin)** change what
  the release notes for decision **5 (v1.0.1)** can truthfully
  say, so settling 1 and 2 first lets 5 be a clean yes/no.
- Nothing here is urgent in the sense of breakage: the tree is
  green as it stands. Each decision is about what the *final*
  state should be.

Facts in each section are traceable to the pack named at its end.

---

## Decision 1 — Captions: keep the 73-chunk split, or veto back to 66?

### What it is

The film's captions are checked against an 850 px width bar (a
readability safe-area rule — captions wider than that are flagged,
though they were never actually clipped on screen; the worst old
caption still had ≈53 px of margin). In Wave 1, seven captions
were over the bar. The lead accepted them with a written waiver,
and the caption check stayed red. Your Wave-2 order reopened it:
the seven were each split into two shorter captions inside the
same time window (66 → 73 chunks total), and the check now reads
**`FILM TELEMETRY: CHECK OK`**. The split stands unless you veto
it here.

The seven moments — as a viewer, at each of these the caption now
changes once mid-sentence instead of staying up whole:

| Shot | At | Was (one caption) | Now (two captions) |
|---|---|---|---|
| s02 | 0:07–0:09 | "was a Japanese soldier" | "was a Japanese" → "soldier" |
| s05a | 0:22–0:23 | "Planes dropped leaflets" | "Planes dropped" → "leaflets" |
| s07 | 0:32–0:33 | "surviving on coconuts" | "surviving" → "on coconuts" |
| s08 | 0:37–0:38 | "named Norio Suzuki" | "named Norio" → "Suzuki" |
| s09 | 0:44–0:45 | "commander ordered it." | "commander" → "ordered it." |
| s10 | 0:46–0:48 | "his old commander," | "his old" → "commander," |
| s11 | 0:52–0:54 | "and Onoda surrendered" | "and Onoda" → "surrendered" |

What did **not** change: the narration, the words, and every
caption's start/stop moments (each pair exactly tiles its old
window; the two pieces rejoin to the old text word-for-word).
Every caption is now ≤ 728 px wide; before, the seven ran
871–1051 px. The film is still 1,811 frames; the audio is
untouched.

### The options

**Keep 73 (do nothing).** The data as it stands is final, the
caption check stays green, and the golden reference frames
("pins") struck during the wave stay valid. Cost: none.

**Veto → back to 66.** All the work lands on us: revert the
re-split data and caption strips, restore the seven one-piece
caption lines, and regenerate. Three of the 14 golden pins
(s07/s08/s11) were struck from post-split frames and go stale —
they must be restored from the pre-split candidates. The caption
check returns to `CHECK FAILED (… 7 caption failures)` and the
Wave-1 waiver stands again as its recorded disposition — i.e. a
permanently red check, accepted on paper. Vetoing is defensible:
the bar is a readability rule, not a visible defect.

### Recommendation (labelled as such)

**Keep 73.** Most splits land where the sentence naturally
breaks ("Planes dropped | leaflets", "and Onoda |
surrendered"). Honest caveat: two splits are width-forced
mid-phrase — "named Norio | Suzuki" cuts a name in two, and
"his old | commander," parts an adjective from its noun. They
were the narrowest legal split points, and on screen each piece
still reads as a normal caption beat. Keeping costs nothing and
leaves the check green; vetoing spends real work to return to a
permanently red check over a margin rule.

*Source: `docs/integration/CAPTION-DECISION-W69.md` (splits,
windows, and veto mechanics verified field-by-field there);
waiver of record: `docs/ab/film/CAPTION-DECISION-W92.md`.*

---

## Decision 2 — The s28 pin: leave it red, re-pin it, or retire it?

### What it is

The visual-regression suite re-renders a small set of reference
frames and compares them byte-for-byte against pinned "golden"
frames; a pin that no longer matches shows as DRIFT. Every pin
now matches exactly — except one. The pin called
`s28-onoda-t150` points at the very first frame TaleRender ever
produced (the §28 milestone artefact, 2026-11-13: Onoda's first
shot at t = 1.50 s). That pin predates a whole chain of renderer
fixes, so today's output legitimately differs from it (measured
21.65 dB — far outside the suite's near-identical band). The
suite's runner therefore reads `FAIL (4 pins, 1 drift)` on this
one item alone, and has done since the other pins were settled.
The triage tool's register already classifies this exact red as
known debt ("AS EXPECTED"), so it does not mask anything else:
any *new* drift anywhere else still surfaces as a new signal.

### The options

**Leave it red.** Nothing changes. The suite stays red on this
one named item indefinitely; the triage register keeps labelling
it expected debt. Cost: a permanently red suite, and anyone
reading it must know this red is the expected one (the register
says so, in so many words).

**Re-pin it** (point s28 at today's render). The suite goes
green — but the suite feeds s28 the *same render* it checks
against the `ab1-composer` pin, so the new s28 pin would be
byte-for-byte identical to ab1's. Two pins would then guard the
same bytes, and s28 would stop being an independent check of
anything. The re-pin packet recommends against exactly this.

**Retire the item.** Remove the pin. The historical frame stays
in the repo as an artefact; it simply stops being a regression
reference. The suite goes fully green, and nothing of value is
lost — the frame it would guard is already guarded by the ab1
pin.

### Recommendation (labelled as such)

**Do not re-pin** — that part is the source packs' firm
position, not just this brief's. Between the other two, the
working recommendation is **retire the pin**: it guards nothing
the ab1 pin doesn't already guard, and a suite that is red
forever trains everyone to ignore red. If you would rather keep
the first-light artefact under formal watch, leaving it red is
safe — the register names it, and real drift elsewhere still
shows up.

*Sources: `docs/integration/VR-DISPATCH-W37.md` (the s28
warning, §2) and `docs/integration/VR-REGISTER-W72.md` (the
post-pin register: s28 as the sole remaining debt, its DRIFT
measured on run `43625139818`, and the byte-duplicate
consequence of re-pinning).*

---

## Decision 3 — Locomotion: adopt the gait module into the film?

### What it is

The Onoda film's walking was staged by hand. A separate piece of
code — the locomotion module — computes walking from the
physical laws of gait, and its arithmetic is proven by tests
(planted foot stays still on the ground, legs in anti-phase,
arms in counter-phase). Adopting it means re-rendering the
film's walking shots with the module's numbers instead of the
hand-staged ones. One preliminary step needs **no** decision:
merely registering the module in the build (a one-line change
that provably changes zero pixels and puts its gait tests under
CI) is housekeeping, and it proceeds either way. **The decision
here is adoption** — and because adoption is a judgement about
how motion *looks*, it is yours, made from before/after contact
sheets the lead produces first, not from this page.

What would change on screen:

1. **The bob rhythm is currently upside-down; the module sets
   it right.** A walking body is highest as the support leg
   passes underneath and dips lowest when both feet are briefly
   on the ground. The hand-staged bob does the opposite. The
   module also makes the bob a little smaller (about two-thirds
   of today's travel — ~3.3 px instead of 5 px at the largest
   setting). The legs' poses do not change at all — only the
   body's vertical rhythm on top of them.
2. **Walking characters gain arm swing.** Today their free arms
   hang still. The module adds the natural counter-swing — each
   arm opposite to the leg on the same side (16° by default; the
   size is a dial that can be retuned without reopening anything
   else). Arms that are holding something (the coconut, the map,
   the book) and both arms in the s11 walk-in keep their staged
   poses exactly.
3. **Some standing characters shift height slightly.** Because
   the staging writes a bob value even for figures standing at
   the start or end of a walk, swapping the formula re-levels
   them too: the s02 and s06 opening frames lift by ~3 px, most
   standing tails settle by ~1 px, and in **s10 (the bookshop)
   Tani stands ~2 px higher for about 4.4 seconds** before her
   short walk. Not motion — just standing at a slightly
   different height.

Scale, honestly: of the film's 1,811 frames, 471 have someone
walking; 888 frames (about half the film) take the bob change,
standing frames included; 448 frames gain arm swing; and the
other **923 frames must come out byte-for-byte identical** —
the checking protocol requires that, it doesn't hope for it.

### The four sub-decisions

- **D1 — Standing frames: include them, or change only the
  walking itself?** *Recommended: include.* One walking law
  everywhere, with the small re-levels listed above, all shown
  on the comparison sheets. The alternative (walking-only)
  avoids the re-levels but makes a character's height step
  visibly — by up to ~3 px — at the exact moment each walk
  starts and stops.
- **D2 — Four background walkers' resting arm pose: keep
  theirs, or give them the module's default?** Four companion
  walkers (two in s02, two in s06) were never given an arm pose;
  the module's own default is Onoda's pose, which differs, so a
  careless wiring would move their resting arms too.
  *Recommended: keep each walker's current resting pose* — this
  adoption should change how characters walk, not how they are
  posed.
- **D3 — Arm swing only while actually advancing?**
  *Recommended: yes.* Standing characters keep their staged
  arms, and during s02's staged "slide" (feet planted, group
  gliding) the arms stay still rather than swinging over planted
  feet.
- **D4 — s11's arms, and the adoption itself.** The s11 walk-in
  uses arms staged from the source material; the module's author
  ruled they stay as they are unless you ask to re-stage them.
  And accepting adoption makes the new render the film's
  reference of record — the golden pins get re-struck through
  the usual process.

Your verdict options, once you've seen the sheets: **adopt** /
**adopt with the arm-swing size retuned** (name the size) /
**reject** — the film keeps its hand-staged walking and the
module stays available for future work.

### Recommendation (labelled as such)

The pack's recommendations on D1–D3 are above. On the adoption
itself this brief offers no push: the arithmetic is proven, but
no measurement in the pack says the new motion *looks better* —
deliberately, no image-quality score is used as a pass mark,
because whole-body shifts of a few pixels score moderately no
matter how they look. Judge it on the contact sheets (s02 the
march, s06 the companions, s07, s08, s10's stand-and-step, s11
the walk-in), looking for: bodies reading as weight settling,
every arm swinging *against* its same-side leg, no pop where
walking starts, stops, or slides.

*Source: `docs/integration/LOCOMOTION-STEP0-W87.md`, Part 3
(the decision sheet, incl. the census and the D1–D4 texts) and
Part 1 (the zero-pixel registration step); adoption pack behind
it: `docs/integration/LOCOMOTION-AB-W57.md`.*

---

## Decision 4 — Lipsync: bind the phoneme track with Variant A, Variant B, or not at all?

### What it is

When a character speaks on screen, the mouth has to move with
the words. TaleRender has a proper phoneme track for the Onoda
narration — built from the real word timings, 551 keys — but it
is currently bound to **nothing**: nothing in the film uses it.
The only mouth movement in the film today comes from a simple
oscillator (a clock-driven flap) in two short windows where Tani
speaks on screen; she has no speech-timing data, so those windows
stay oscillator-driven under **every** option below. The honest
headline: **this decision changes zero frames of the current
film.** It decides how *future* on-screen speakers — the next
film's characters — get their mouths.

The catch the evaluation found: if the track is bound the plain
way (smooth interpolation between keys), it leaks. The mouth
drifts open across silences (painted open on 229 of 262 in-line
silence frames) and the lip-closures for "p", "b", "m" sounds
hold on only 6 of 36 frames. Two mechanisms fix the leak; they
differ in *where the fix lives*.

### The options

**Variant A — the fix lives in the data (builder-side).** The
track builder marks every segment that *exits a closed mouth*
as "hold the closed posture, then jump at the next sound"
(a step instead of a glide). It's a ~3-line change to the
builder; on this film's track, 77 of 551 keys change their
ease field and nothing else; the file passes the format
validator 5/5 today, unchanged evaluator. Measured result:
closures hold **36/36**, mouth-open leakage outside the spoken
lines drops to **0**, and silence frames painted open fall to
106 of 262 — the remainder are gaps shorter than 0.04 s between
words, where holding the previous posture is what natural
speech does. Every tool that reads these files (the renderer,
the re-render planner, the exporter checks) sees the same
signal, because the file *is* the signal.

**Variant B — the fix lives in the renderer (evaluator-side).**
The builder's files stay plain, and the evaluator special-cases
the speech channel when sampling it. Same pictures as A. Costs:
channel-specific knowledge inside an evaluator that is
deliberately channel-blind; the file the validator checks no
longer describes what actually renders; and every other tool
that evaluates these files must re-implement the same rule or
silently compute a different mouth. Its one advantage —
existing track files stay byte-untouched — protects files whose
whole purpose is to be regenerated by the builder anyway.

**Don't bind at all.** The phoneme track stays unused; future
on-screen speakers keep the oscillator — a mouth flapping on a
clock, open through silences, never closing on "p"/"b"/"m".
No work, and the leak problem never arises because the better
signal is never used.

A sub-option inside A, priced and *not* recommended as the
default: putting a silence key at every word end (call it A2)
closes the last 106 frames too, but creates 85 micro-closures —
51 of them exactly one frame long — i.e. the mouth visibly
slamming shut inside fast speech where no silence exists. It
stays available as a builder switch you can order later.

**How you'd judge it:** a contact-sheet set for narration line
3 (frames 482–632), rendered three ways — oscillator, track
bound plainly, track bound with Variant A — on CI or your PC.
The headline check is the 17-frame silence after "ended."
(frames 570–586): under A the mouth must read shut for all 17
frames. Two 1-frame bridges elsewhere in the line are where
you'd decide whether A2 is ever wanted.

### Recommendation (labelled as such)

**Variant A.** It's the source pack's recommendation, for the
format's own reasons: the artefact that gets validated is the
artefact that renders; the evaluator's plain default survives
intact with zero new machinery; and the price is a 3-line
builder change plus a deterministic regeneration, against
Variant B's permanent split between data and behaviour.

*Source: `docs/integration/LIPSYNC-BIND-W86.md` (both variants
specified and measured; §4 is its recommendation; §5 is the
sign-off sheet spec); evaluation behind it:
`docs/integration/LIPSYNC-EVAL-W59.md`.*

---

## Decision 5 — v1.0.1: give the go?

### What it is

A patch release of the TaleRender program itself. Your PC
currently holds **two different binaries that both call
themselves 1.0.0** (the published release asset, and a second
copy with different bytes), and the published 1.0.0 binary
predates the finished command-line polish — it answers
`--help` with "unknown argument". Release v1.0.1 from the
wave's final tree and there is exactly one current binary,
with the wave's work in it: the full help/version CLI, the
telemetry exit codes, the draft knobs, the format guard, the
generic film-data migration, the blink adoption, and everything
else in the changelog the pack assembles.

What "go" triggers, mechanically: freeze the tree → bump the
one version line → re-prove all six acceptance gates **on that
exact bump commit** (main CI, tools CI, the knobs gate's
`KNOBS GATE: PASS` line, the determinism run, the golden-pack
verify at 18 OK, and telemetry's `FILM TELEMETRY: CHECK OK`)
→ publish **one CI-built binary** as v1.0.1 → clean your PC
down to a single canonical copy at `~/talerender-bin/talerender`
(the path the `TALERENDER_BIN` setting names), every other copy
replaced with the same bytes, old copies kept only as renamed
`.pre-v1.0.1` backups. Two standing rules: the v1.0.0 proof MP4
is **not** re-attached (it predates the caption re-chunk of
Decision 1, so it would misrepresent the release), and v1.0.0
itself stays published and untouched.

### The options

**Go.** The procedure above runs at wave close; the
two-binaries confusion ends; the release notes state the
outcomes of Decisions 1 and 2 truthfully (which is why those
come first in this brief).

**Not yet.** Nothing breaks. The code stays as it is in the
repo, and your PC keeps two divergent "1.0.0" binaries until
some later release. The pack remains ready to execute whenever
you say go.

### Recommendation (labelled as such)

**Go.** The release pack's own summary is that nothing in the
code blocks v1.0.1 — it was "no-go *yet*" only because it was
waiting on this conversation: your explicit go, the wave
freeze, the bump itself, and the six gates re-proven on the
bump commit. The first three are settled by closing the wave;
the fourth is part of the procedure, not a precondition.

*Source: `docs/integration/RELEASE-101B-W89.md`, §7 (the
go/no-go summary) with §5 (pre-release requirements) and §6
(the PC cleanup); base pack:
`docs/integration/RELEASE-101-W46.md`.*

---

## Decision 6 — Skill adoption: make TaleRender an opt-in capture backend in the Tall Tale skill now, or wait?

### What it is

The Tall Tale skill currently captures frames through Chrome.
A work copy of the skill exists with a second capture path
added: TaleRender (native, no browser), selected per run with
`RENDERER=native`. (Name collision, flagged so the record stays
straight: the wiring team's own one-pager for this decision is
*also* called `ADOPTION-DECISION-W94.md` and lives in the work
copy at `~/workspace/talerender-skill-wiring/docs/` — this brief
is `DECISIONS-W94.md` in the TaleRender repo's
`docs/integration/`.)

Adopting means copying the work copy's additions into the
shipped skill — the adapter (`template/tools/capture_native.mjs`)
and its operator doc, the design doc, a native selftest and its
hook, and **one additive paragraph** in SKILL.md that mentions
the native path exists. Nothing is removed or altered:
`capture.mjs` and every existing behaviour are untouched, and
**Chrome stays the default and the renderer of record** —
adoption is not defaulting. A project captures natively only
when the operator explicitly selects it, and only after that
project's staging has been converted ("bridged") into native
film data by a self-verifying tool (21/21 checks; the Onoda
bridge is 94 files). The binary is not shipped with the skill:
before first native use it is downloaded once from a green CI
run to a stable path (`TALERENDER_BIN`), and the binary and the
bridge data must come from compatible points in the repo's
history — a mismatched pair fails closed at the adapter's
telemetry gate (safe, but blocked) rather than rendering wrong.
The adapter never falls back to Chrome silently.

Evidence in hand: the Onoda film was captured natively end to
end and measured against the Chrome frames (60/60 shot pairs;
similarity mean 21.84 dB, min 18.38 dB — reported as a
measurement, not claimed as a match, since the Chrome references
carry a JPEG generation the native frames deliberately don't);
the film finished to an MP4 technical proof at the project's
exact loudness; the scene-file loader lane passes its
acceptance (all 13 texture generators load; the fog chart
matches its analytic expectation to within 1 code value). Two
of the wiring one-pager's scariest open items have since closed
in Wave 2: the caption gate it reported as failing is now green
(that's Decision 1's 73-chunk data), and the golden pins it
reported as unpinned have been struck and verified
(18 OK / 0 stale / 0 missing — Decision 2's s28 is the one
deliberate exception).

The one gap that has **not** closed: **there is no proof on
your PC's graphics hardware yet.** Every native number above
comes from software rendering — CI machines and this VM. The
performance protocol has literally no results in it (its tables
are blank by design): no frame-time numbers on your Radeon, no
determinism pass recorded on that hardware. The protocol's own
validity rule is that a run only counts when its renderer line
names your real GPU driver, and that has never happened yet.

### The options

**Adopt now, as opt-in.** The additions ship in the skill's
next version; native capture becomes available per project,
Chrome untouched as default. What it commits you to: the
one-time binary provisioning, the binary↔data pairing
discipline, bridging each project before it can capture
natively — and living with the RADV gap as *known-open* while
the backend gets exercised on real projects.

**Wait for the PC proof first.** Nothing changes; Chrome
remains the only capture path until the hardware runs are done
(the performance protocol, a determinism pass on your GPU, a
first full film render on it). Cost: the work copy is a
snapshot of today's shipped skill plus the additions — its
drift against the shipped skill is zero *today*, but if the
shipped skill changes first, the additions must be re-checked
against it before adopting later.

### Recommendation (labelled as such)

**Adopt now, as opt-in** — this is this brief's working
recommendation; the wiring one-pager presents the options
without picking one. Reasoning: adoption is reversible and
touches nothing on the default path, and the missing RADV proof
is about performance numbers and a formal determinism record,
not about whether the backend works — the film proof and loader
acceptance are already in hand. Waiting buys certainty about
speed on your hardware; it does not buy correctness, and it
carries the drift risk above. If your first native capture on a
real project is one you'd rather have backed by hardware
numbers, waiting is a coherent choice — that preference is
exactly what this decision is asking.

*Sources: the wiring one-pager,
`~/workspace/talerender-skill-wiring/docs/ADOPTION-DECISION-W94.md`
(§2 what adoption means, §3 evidence, §4 the two options);
Wave-2 updates to its §3 from
`docs/integration/CAPTION-DECISION-W69.md` and
`docs/integration/VR-REGISTER-W72.md`; hardware-proof status:
`docs/runbook/PERF.md` (blank by design) as cited there.*

---

## Answer sheet

Reply with any subset — each decision is independent unless
noted:

1. Captions: **keep 73** / veto to 66
2. s28 pin: leave red / re-pin / **retire**
3. Locomotion: adopt / adopt with arm-swing at ___° / reject —
   plus D1 include-standing (recommended) or walking-only,
   D2 keep companions' pose (recommended) or module default,
   D3 advancing-only swing (recommended) or always
4. Lipsync: **Variant A** / Variant B / don't bind
5. v1.0.1: **go** / not yet — *best answered after 1 and 2*
6. Skill adoption: **adopt now (opt-in)** / wait for PC proof

Bold = the recommendations above.

*Compiled for the Wave-2 close conversation, seat W94
(Batch 10). Nothing in this file changes any code, data, or
pin; it only assembles the choices.*
