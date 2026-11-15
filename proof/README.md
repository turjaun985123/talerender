# proof/ — CI render-proof harness (CR-B3-02)

Staged by seat B3-02 (Chrome-replacement wave, Batch 3) for the lead
to commit into `turjaun985123/talerender`. Companion workflow:
`.github/workflows/proof.yml` (staged beside this directory's parent —
see "Layout" below). Full seat report:
`docs/integration/CR-B3-02-CI-PROOF.md` in the talerender repo.

## What it proves

On a clean GitHub-hosted runner (Mesa **lavapipe** — software Vulkan;
correctness only, no speed claim), a bridged project renders
end-to-end through the shipped skill's native adapter with Chrome
made unreachable (packages removed, `CHROME` unset, PATH-first
exec-logging shim on every `chrome*`/`chromium*` name), and the job
log ends with `CHROME-EXEC-LOG: EMPTY`. Adapter legs, in order:
`check` → `frames` → `sheet` → `castsheet look` → `video --draft`
(full range). No artifacts are uploaded (quota); the log carries
exit codes, `CHECK OK`, file sizes + sha256, and an ffprobe summary.

## Layout (as it should land in the repo)

```
.github/workflows/proof.yml     the harness (manual dispatch only)
proof/
  .gitignore                    re-includes fixture PNG inputs against
                                the root *.png rule (see the file)
  README.md                     this file
  adapter/
    PROVENANCE.md               source + sha256 of every adapter file
    tools/                      byte-identical copy of the shipped
                                skill's template/tools adapter subset
  fixture/
    project/                    the bridged proof project (below)
```

## Fixture provenance — READ THIS

**The final staged fixture is the retrofitted Onoda bridge, NOT the
B3-01 proof project.** Seat B3-01 was authoring a new short's bridged
film data in parallel; its directory did not exist when this harness
was built, so per the seat charter the harness was built against the
fallback. The lead then verified a blocker that makes Onoda the only
servable fixture today: **the binary's `--film` driver and
`--film-telemetry` are hardcoded to the Onoda film** (`src/film.rs`
staging indexes `data.anchors` by Onoda's names, the loader requires
`calendarYears`/`calendarDark` from every film, shots are staged in
code). B3-01's bridge-valid new film panics the gate —
`thread 'main' panicked at src/film.rs:3726:55: no entry found for
key` (evidence: `batch3/B3-01/FRICTION.md` F1/F3 + its logs). The
new-film leg of the render proof is **BLOCKED until a generic
scene-sequence film path exists**; do not work around the coupling
(no fake anchors, no invented calendar data in fixtures). Detail:
`docs/integration/CR-B3-02-CI-PROOF.md` §0. The fallback source:

- Source: `~/workspace/chrome-replacement/batch2/B2-08/onoda-copy/`
  — the canonical Onoda project after the CR-B2-08 P1 retrofit
  (re-bridged from the repo's current film data; its own adapter
  check recorded `CHECK OK`, exit 0 — see
  `batch2/B2-08/evidence/adapter-check.log`).
- Copied verbatim: `native/` (manifest `film.json`, 14 segment
  scenes, `film/onoda.json`, 85 asset files), `script.json`,
  `fonts/` (3 files), `data/{cues,lines,words}.json`, `data/words.txt`.
  Key-file sha256 match the source exactly (manifest
  `18ad444c…`, film data `1634013d…`, script `1572c1aa…`).
- Film: "THE SOLDIER WHO DIDN'T KNOW THE WAR WAS OVER" — 60.357 s,
  1811 frames @ 30 fps, 1080×1920, 14 segments, 73 caption chunks.

### Deliberate fixture transformations (both lossless, both verified)

1. **Audio master as FLAC.** `data/mix.wav` (12 MB, sha256
   `7578e5d9…`) is stored as `data/mix.flac` (2.1 MB, FLAC = lossless).
   The workflow decodes it back to `data/mix.wav` before running the
   adapter and asserts the decoded PCM md5 equals the value recorded
   in `data/mix.pcm-md5` (`1d1b69d007d096f92abe3b9f38bb9366`, the md5
   of the original WAV's audio stream). `data/narration.wav` is NOT
   included: it is a stem of the mix and no adapter mode reads it.
2. **No `tools/` in the fixture.** The project's own `tools/` (an
   older adapter copy) is deliberately omitted — the workflow
   installs `proof/adapter/tools/` into the assembled project, so
   the adapter under proof is the shipped skill's, by construction.

### Swapping in the B3-01 project later

*(Only meaningful once the §0 blocker — the Onoda-hardcoded film
driver — is fixed by a generic scene-sequence film path; until then
any non-Onoda fixture panics the gate by design of the current
binary, which is a binary defect, not a harness or fixture defect.)*

Replace the contents of `proof/fixture/project/` with B3-01's
bridged project (same shape: `native/` + `script.json` + `fonts/` +
`data/` with the mix stored per the FLAC convention above, or a
plain `data/mix.wav` — if plain WAV is used, delete `mix.flac` /
`mix.pcm-md5` and drop the decode+verify block from the workflow's
assemble step). **No other workflow edit is needed**: frame times
are derived from the fixture's own `native/film.json` duration,
sheet uses adapter defaults, castsheet uses the catalog `look`
kind, and video is full-range.

## Commit checklist for the lead

```bash
cd ~/workspace/talerender
cp ~/workspace/chrome-replacement/batch3/B3-02/.github/workflows/proof.yml .github/workflows/proof.yml
cp -r ~/workspace/chrome-replacement/batch3/B3-02/proof proof
git add .github/workflows/proof.yml proof
git status --short          # expect ~120 files, INCLUDING the fixture PNGs
git check-ignore proof/fixture/project/native/film/assets/cap_000.png || echo "PNG not ignored — good"
git commit -m "CR-B3-02: CI render-proof harness (Chrome-free native render, lavapipe)"
git push
gh workflow run proof.yml --repo turjaun985123/talerender
gh run watch --repo turjaun985123/talerender
```

The `git check-ignore` line matters: the repo-root `.gitignore`
ignores `*.png` globally, and `proof/.gitignore` re-includes the
fixture's PNG inputs. If a fixture PNG shows as ignored, the bridge
data would be silently incomplete — do not commit until it doesn't.
