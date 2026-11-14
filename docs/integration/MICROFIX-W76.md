# MICROFIX-W76 — micro-fix pack verification: finish.sh AAC rate + `--help` text

**Seat:** Wave 2, Batch 8, W76. **Verified at HEAD `b2e46d2`** (git log
checked once at seat start; `b2e46d2` is the tip). New file only;
nothing else touched, nothing committed.

**Method note (evidence provenance):** the binary at
`~/talerender-bin/talerender` is the stale pre-F1 build (19,548,480 B;
its `--help` still shows the older `--film` paragraph "DIR holds
film.json + its scene files (format v2, §14)"), so it is **not**
evidence for HEAD. All `--help` verdicts below rest on the `HELP`
const in `src/main.rs` at `b2e46d2` (const at main.rs:41), read in
full, with each named path/claim checked against the code.

The two items originate in GAPFIX-MATRIX-W49.md §D3/§D4:

> D3: "**AAC bitrate deviation in the deliverable.** `film/finish.sh`
> encodes `-b:a 192k`; the skill master's setting per the analysis
> (§2.1/G16) is AAC 256k." — verdict there: "OPEN (micro) — owner: lead."

> D4: "**`--help` still over-promises scene resolution.** Post-F1 help
> says film "segment scenes resolve from scenes/film/" — but
> `src/film.rs` contains no `scenes/film` reference at all" — verdict
> there: "OPEN (micro) — owner: lead."

---

## Item 1 — `film/finish.sh` encodes AAC at 192k; skill master is 256k

### Current state (at `b2e46d2`)

The finishing script is `film/finish.sh` (repo root `film/`, not
`tools/film/`). Its complete audio path is one line, **finish.sh:31**:

```
  -c:a aac -b:a 192k \
```

inside the single ffmpeg invocation (finish.sh:24–32). The script's
own header states its contract (finish.sh:6–8):

```
# data/mix.wav, muxed unchanged (AAC encode only). Settings are the
# pipeline's own finishing settings: x264 slow CRF 18, BT.709 tags,
```

`git log --follow -- film/finish.sh` shows exactly two commits
(`fd12cab`, `6ca97fc` — the latter the BT.709 VUI fix); the 192k rate
has been there since the script was created and has never been
revisited.

### The skill master setting (read-only reference, quoted)

`~/workspace/custom-skills/tall-tale-shorts/template/tools/capture.mjs:419–420`:

```
// I4 (r09 §9 + a12 G3): master = preset slow (CRF stays 18), x264 -threads 4, AAC 256k, explicit
// -ar 48000, BT.709 colour tags; draft = veryfast CRF 23 (was ultrafast/28), AAC stays 192k.
```

and the master encode args, capture.mjs:425:

```
: ['-c:v', 'libx264', '-preset', 'slow', '-crf', '18', '-threads', '4', '-pix_fmt', 'yuv420p', '-color_primaries', 'bt709', '-color_trc', 'bt709', '-colorspace', 'bt709', '-color_range', 'tv', '-x264-params', 'colorprim=bt709:transfer=bt709:colormatrix=bt709', '-c:a', 'aac', '-b:a', '256k', '-ar', '48000'];
```

Corroborated by the skill README
(`~/workspace/custom-skills/tall-tale-shorts/README.md:228`):

```
(preset `slow` at CRF 18, AAC 256k at 48 kHz, BT.709 colour tags, x264 `-threads 4`)
```

Note the asymmetry the quotes expose: in the skill, **192k is the
draft rate** ("AAC stays 192k" on the draft line, capture.mjs:424)
and **256k is the master rate**. finish.sh pairs the master's video
settings (slow / CRF 18 / BT.709 VUI triple — matching the master
line) with the draft's audio rate. It is a mixed preset, not a
coherent draft.

### Verdict: **STILL-OPEN**

Nothing since W49 has touched the line; the F2 migration (`2c41313`)
changed the film driver's data loading, not the finishing script.

### Loudness / master consequence (answered from the finishing flow)

**The AAC bitrate does not touch the −14 LUFS figure.** Three
independent grounds, all from the record:

1. **The script has no gain stage.** finish.sh:24–32 was read in
   full: the only audio operation in the entire finishing flow is
   `-c:a aac -b:a 192k`. There is no `loudnorm`, no `volume`, no
   `-af` — nothing that can apply gain or normalization. The header
   says the same in words: "muxed unchanged (AAC encode only)".
2. **Bitrate is a coding-fidelity parameter, not a level.** It
   selects how much lossy-compression detail the AAC encoder keeps;
   it applies no gain. Integrated loudness is a level measurement,
   so a 192k↔256k change cannot move it (beyond encoder rounding,
   quantified next).
3. **It was measured, on the mux, at 192k.** docs/ab/film/MP4-EVIDENCE.md:104–110:

   > "## Loudness (measured on the mux, not the mix)
   > The mix is the project master at **−14.0 LUFS integrated**
   > (film.json audio block, muxed unchanged). The MP4's audio is an
   > AAC encode of that master with no gain stage, so the mux must
   > measure the same within encode tolerance."

   and its table (MP4-EVIDENCE.md:118–120):

   > | integrated loudness | **−13.99 LUFS** | **−14.00 LUFS** | Δ 0.01 LU — match (tolerance ≈ 0.3 LU) ✓ |
   > | true peak | **−1.65 dBTP** | **−1.58 dBTP** | +0.07 dB AAC overshoot; no clipping ✓ |

So the delivered proof MP4's −14.00 LUFS was measured **after** the
192k encode and matches the WAV master to 0.01 LU. Raising the rate
to 256k buys coding transparency and true-peak headroom behaviour
(the +0.07 dB encode overshoot is the only level-domain effect
observed), **not** loudness compliance — the master is already
compliant either way. The fix matters because finish.sh claims "the
pipeline's own finishing settings" while silently deviating from
them in exactly one parameter (the failure mode W49 named: silent
drift in a "settings verbatim" claim, not the 64k itself).

### Patch-ready correction

One line, finish.sh:31:

```
-  -c:a aac -b:a 192k \
+  -c:a aac -b:a 256k \
```

(W49's alternative — keep 192k and record the deviation as
deliberate in MP4-EVIDENCE — remains available to the lead, but the
one-line alignment is recommended: the script's header asserts the
pipeline's settings, and at 256k that assertion becomes true.)

**Adjacent observation (not part of D3, flagged for the same
pass):** the skill master also pins `-ar 48000` explicitly
(capture.mjs:425); finish.sh passes no `-ar`, so the output sample
rate simply follows `mix.wav`'s. Whether that ever differs was not
verified here (mix.wav's rate not probed); the lead may rule it in
or out while the file is open.

---

## Item 2 — `--help` claims segment scenes "resolve from scenes/film/"

### Current state (at `b2e46d2`)

`src/main.rs`, const HELP, the `--film` entry (the false clause is
on main.rs:67):

```
    --film <DIR>            Render a whole film: DIR holds the
                            film data (film.json — the generic
                            entry; onoda.json accepted for this
                            film) + an assets/ dir; segment scenes
                            resolve from scenes/film/. Frames are
                            written to --out as frame_%05d.png;
                            existing frames are skipped (resume).
```

### The code's actual behaviour

- `grep -rn "scenes/film" src/` returns exactly four hits: the HELP
  line itself (main.rs:67), two **comments** in src/scene.rs
  (2142, 2170) describing scene files as data, and a **test fixture
  string** in src/errors.rs (307–310). No code path reads it.
- `src/film.rs` contains **no** `scenes/film` reference and no
  `"scenes…"` path literal at all. What `run_film` reads from DIR
  is the film data (film.rs:73–76: `film.json` if it exists, else
  `onoda.json`) and `DIR/assets` (film.rs:2103:
  `let adir = dir.join("assets");`). The segment shots are staged
  and evaluated by the driver in code.
- The actual readers of `scenes/film/` are tools, not the binary:
  `tools/film/validate_film.py`, `tools/film/rerender_plan.py`,
  `tools/film/locomotion_census.py`, `tools/golden/make_golden.py`
  (the manifest/validator convention, exactly as W49 said).
- The F2 migration (`20a81a7` + `2c41313`) changed what the driver
  demands **inside** DIR (plates manifest v1, calendar/sign declared
  in film data); it did not change scene resolution. The sentence
  was false before F2 and is false after it.

### Verdict: **STILL-OPEN**

### Patch-ready correction

Replace the `--film` entry in const HELP (src/main.rs) with:

```
    --film <DIR>            Render a whole film: DIR holds the
                            film data (film.json — the generic
                            entry; onoda.json accepted for this
                            film) + an assets/ dir; the segment
                            shots are staged by the driver in
                            code (scenes/film/ is the tools'
                            manifest copy — --film never reads
                            it). Frames are written to --out as
                            frame_%05d.png; existing frames are
                            skipped (resume).
```

(The module doc comment at main.rs:8–10 — "whole films (scene
format v2 + the film driver)" — makes no path claim and needs no
change; main.rs:11's "keep the two in step" stays satisfied.)

---

## Sweep — every path/filename `--help` names, checked against the code

HELP const read in full (src/main.rs:41–128). Every path or
filename it names:

| # | HELP names | Where | Code behaviour (verified) | Verdict |
|---|---|---|---|---|
| 1 | `docs/scene-format.md` ("see docs/scene-format.md") | main.rs:64 | File exists at `docs/scene-format.md` in the repo | ACCURATE |
| 2 | `film.json` — "the generic entry" | main.rs:66 | film.rs:73–74: `if dir.join("film.json").exists()` → preferred | ACCURATE (F1 landing, `aadc193`) |
| 3 | `onoda.json` — "accepted for this film" | main.rs:67 | film.rs:76: fallback `dir.join("onoda.json")` | ACCURATE |
| 4 | `assets/` dir in DIR | main.rs:67 | film.rs:2103: `dir.join("assets")` | ACCURATE |
| 5 | `scenes/film/` — "segment scenes resolve from" | main.rs:67 | No src reader exists (Item 2); tools-only manifest convention | **STALE — Item 2** |
| 6 | `frame_%05d.png` output naming | main.rs:69 | film.rs:3501: `format!("{out_dir}/frame_{n:05}.png")`; consumer side agrees — finish.sh:25 reads `frame_%05d.png` | ACCURATE |
| 7 | `frame.png` — default `--out` | main.rs:48, 87 | main.rs:176: `let mut out = String::from("frame.png");` | ACCURATE |
| 8 | "the §28 strip under docs/ab/" — default `--caption` | main.rs:88–92 | main.rs:178: default `docs/ab/onoda-s01-t150/assets/caption.png`; the file exists in the repo; help's own caveat ("repo-relative path, so run from the repo root") is honest | ACCURATE |

Behaviour claims adjacent to the paths, also verified (same read):

| Claim | Code | Verdict |
|---|---|---|
| `--scene`: `m1 \| ab1 \| dof-chart \| stroke-battery`, default m1 | main.rs:177 default `"m1"`; dispatch main.rs:296–311 matches exactly those four, unknown name → CLI error | ACCURATE |
| `--scene-file` "Wins over --scene when both are given" | Dispatch order main.rs:289–294: scene_file branch precedes the scene match (note: `--film` is dispatched before both; HELP claims no order for that pair) | ACCURATE |
| `--post-stage`: `lighting \| bloom \| grade`, default grade | main.rs parse default `"grade"`; dispatch main.rs:298–307 maps exactly the three, else CLI error | ACCURATE |
| `--telemetry <MODE>`: `ab1 \| ab1-file \| ab1-variant` | telemetry.rs:1384–1395 matches exactly | ACCURATE |
| Verdict lines `"FILM TELEMETRY: CHECK OK" / "CHECK FAILED (...)"` | film.rs:3716 and 3719 print exactly these formats | ACCURATE |
| `--film-telemetry` exits: 1 = CHECK FAILED, 3 = could not run | main.rs:272–277: `Ok(false)` → exit 1, `Err` → exit 3, `Ok(true)` → 0 | ACCURATE |
| `--width` 1080 / `--height` 1920 defaults | main.rs parse defaults `width: u32 = 1080`, `height: u32 = 1920` | ACCURATE |
| `--from` default 0; `--to` default 0 = through the end | main.rs: `film_from: u32 = 0`, `film_to: u32 = 0`; dispatch comment main.rs:283–285 states the same | ACCURATE |
| `--captions` / `--cutouts-out` are telemetry inputs | Parsed main.rs:228–232; passed to `telemetry::run` main.rs:259–263 | ACCURATE |
| `TALERENDER_PAINT_CACHE` default 160 | paintbudget.rs:38: `DEFAULT_ENTRY_CAP: usize = 160` | ACCURATE |
| `TALERENDER_PAINT_BUDGET_MB`: "derived — the entry cap's bytes, at most 2048 MiB"; "0 disables" | paintbudget.rs:68–72: `default_budget_bytes` = entry_cap × canvas bytes, `.min(BUDGET_CEILING_BYTES)`; paintbudget.rs:77: `"0" -> 0, byte enforcement DISABLED`; `admit()` treats 0 as disabled | ACCURATE |
| `--film` resume: "existing frames are skipped (resume)" | film.rs:3502–3503 skips when the PNG exists; film.rs:3530 summary prints "(resume)" | ACCURATE |
| No `--res`/knobs flag documented | None exists in src at this HEAD (no `Knobs`, `effective_size`, or `"--res"` anywhere in `src/`) — HELP omits nothing that exists | ACCURATE (no gap) |

### Third stale instance found — the EXIT CODES table (same HELP text, numeric class)

Not a path/filename, but the same defect class (help text
describing behaviour the code does not have), found by the sweep
and recorded so the fixes pass can take it in the same edit:

HELP's EXIT CODES section (const HELP, src/main.rs) lists only
0/1/2/3 and describes 3 solely as:

```
    3   --film-telemetry could not run (film data unreadable;
        the reason is printed on stderr, no verdict line).
```

But main.rs:159 routes every non-telemetry mode through
`exit_on_err`, which exits with `TaleError::exit_code()`
(src/errors.rs:181–188):

```
            TaleError::DataLoad { .. } => 2,
            | TaleError::Render { .. } => 3,
            TaleError::ImageIo { .. } => 4,
```

(errors.rs:319–330 unit-tests exactly this mapping.) So in the
binary as built: **exit 4 exists (image I/O) and appears nowhere in
`--help`**, and **exit 3 also fires for GPU/render failures in any
mode**, not only for telemetry-could-not-run. Exit 2 likewise also
covers data-load failures, which HELP's "Command-line error" row
does not mention. The telemetry-specific readings (1 and 3) are
correct as far as they go — the table is incomplete, not wrong
about telemetry.

Patch-ready replacement for the EXIT CODES section:

```
EXIT CODES:
    0   Success.
    1   --film-telemetry QA verdict: CHECK FAILED (the printed
        line carries the counts and stays authoritative).
    2   Command-line error (unknown flag, bad or missing value);
        also film or scene data that fails to load.
    3   --film-telemetry could not run (film data unreadable;
        the reason is printed on stderr, no verdict line);
        also GPU initialisation or render failure in any mode.
    4   Image I/O failure (reading or writing a PNG).
```

---

## Summary for the fixes pass

| Item | Verdict | Fix |
|---|---|---|
| D3 — finish.sh AAC 192k vs master 256k | **STILL-OPEN** | finish.sh:31 `192k` → `256k` (one line). No loudness consequence: no gain stage exists in the flow; −14.00 LUFS was measured on the finished MP4 at 192k and matched the WAV to 0.01 LU |
| D4 — `--help` "segment scenes resolve from scenes/film/" | **STILL-OPEN** | Replacement `--film` entry above (driver stages shots in code; scenes/film/ is tools-only) |
| Sweep — 8 named paths in HELP | 7 ACCURATE, 1 STALE (D4 itself) | — |
| Sweep — adjacent behaviour claims | All ACCURATE | — |
| Third instance — EXIT CODES table omits 4, narrows 3 and 2 | **STALE (incomplete)** | Replacement EXIT CODES section above |
