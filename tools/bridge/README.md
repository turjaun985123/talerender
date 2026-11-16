# Bridge packager — `package_bridge.py`

Seat W48, TaleRender 100-agent wave. Stdlib-only Python 3.

The native capture backend (the skill's `tools/capture_native.mjs`) captures
only projects bridged to film data: it looks for a manifest + film data under
`<project>/native/` and **fails closed, naming every missing item**, when the
bridge is absent or broken. This tool assembles that package from the
TaleRender repo's authored data and then verifies its own output against the
adapter's contract — so a package that leaves this tool has already passed
every check the adapter will run before it touches the binary.

## Usage

```bash
python3 tools/bridge/package_bridge.py \
    --repo ~/workspace/talerender \
    --project /path/to/skill-project [--force]
```

- Exit `0` — packaged, all verification checks PASS.
- Exit `1` — packaging or verification failed; the failing check is named.
- Exit `2` — usage error, or refusal: if `<project>/native/` already holds
  managed paths, the tool refuses to overwrite them without `--force`.
  `--force` rebuilds only the managed paths (`film.json`, `MANIFEST-SHA`,
  `scenes/`, `film/`) — an operator-dropped `native/talerender` binary (which
  the adapter also discovers) is never touched.

## Layout produced

Everything lives under `<project>/native/` — exactly the tree the adapter's
discovery walks (`NATIVE_DIR`, default `<project>/native`):

```
<project>/native/
├── film.json                  manifest — the adapter's FIRST discovery path
│                              (format "talerender-film" v1)
├── MANIFEST-SHA               sha256 of every packaged file (see below)
├── scenes/
│   └── film/
│       ├── s01.scene.json     the 14 segment scene files, at exactly the
│       ├── s02.scene.json     native-relative paths the manifest's
│       ├── ...                segments[] entries name
│       └── s12.scene.json
└── film/                      the binary's film data dir (findFilmDir:
    │                          first place it looks for onoda.json)
    ├── onoda.json             duration, lines, anchors, captions[66]
    └── assets/
        ├── plates.json        plate anchor offsets
        ├── paper-map.png      ┐ the 5 plates
        ├── leaflet.png        │
        ├── book-open.png      │
        ├── book-closed.png    │
        ├── coconut.png        ┘
        ├── calendar-1945.png  ┐ the 4 year plates + dark variant
        ├── calendar-1955.png  │
        ├── calendar-1965.png  │
        ├── calendar-1974.png  │
        ├── calendar-dark.png  ┘
        ├── sign.png
        └── cap_000.png ... cap_065.png   the 66 caption strips,
                                          named by onoda.json captions[].strip
```

### Where each piece comes from

| Package path | Repo source | Notes |
|---|---|---|
| `film.json` | `scenes/film/film.json` | Converted, not just copied — see below. |
| `scenes/film/*.scene.json` | `scenes/film/` | Exactly the files the manifest's segments name (14), copied byte-for-byte. |
| `film/onoda.json` | `film/onoda.json` | Byte-for-byte. |
| `film/assets/*` | `film/assets/` | The whole tree (78 files), so the package is complete by construction rather than by enumeration. The binary's `load_assets` reads plates/calendars/sign/strips from here. |
| `MANIFEST-SHA` | generated | One `sha256  <path>` line per packaged file (sha256sum-compatible), paths native-relative, sorted; the file itself is excluded. |

### The manifest conversion

The repo manifest is already adapter-shaped (`format`, `version`, `fps`,
`resolution`, `durationS`, `totalFrames`, and a `segments[]` table whose
ranges tile `[0, totalFrames)`), so everything the adapter reads is carried
**verbatim** — including each segment's `startS`/`endS`, caption-chunk table,
and the `frameRule`/`boundaryRule` provenance text. Only machine-specific
metadata changes:

- `project` and `audio.projectPath`, which in the repo point at the authoring
  VM's absolute paths, are rewritten to the target project passed via
  `--project` (the audio itself stays where the skill keeps it —
  `<project>/data/mix.wav`, muxed unchanged by the finishing step; it is not
  part of the native package).
- A `bridge` block is added recording the source repo, source paths, and
  this tool. The adapter ignores extra fields.

## The verification check list

After assembling, the tool re-reads the package from disk and re-implements,
check for check, the adapter's pre-validation (`findManifest`,
`loadManifest`, `findFilmDir`, `validateFilmData` in
`capture_native.mjs`), printing PASS/FAIL per check:

**Manifest** — discoverable at `film.json` (or the `scenes/film/film.json`
fallback) · parses as JSON · `format == "talerender-film"` · `version == 1` ·
`fps` equals the adapter's FPS (30 by default) · `resolution` width/height
are positive integers · `segments` is a non-empty array · segment frame
ranges tile `[0, totalFrames)` exactly (each `startFrame` continues the
previous `endFrame`, each range non-empty) · every segment's scene file
exists at its native-relative path · `totalFrames == ceil(durationS × fps)`.

**Film data** — a film dir is found (`film/onoda.json`, else `onoda.json`
beside the manifest) · `onoda.json` parses · `duration` is a number > 0 ·
`lines[]` non-empty · `anchors{}` present · `captions[]` non-empty ·
`ceil(duration × fps) == totalFrames` (the bridge data and the manifest
agree) · `assets/plates.json` exists · the 11 unconditional PNGs exist
(5 plates, 4 calendar years + dark, sign) · every caption names a `strip`
file · every named strip exists in `assets/`.

**Integrity (this tool's addition — the adapter does not do it)** —
`MANIFEST-SHA` exists, covers every packaged file with no extras, and every
recorded sha256 matches the bytes on disk. It proves the package on disk is
the package that was assembled, byte for byte.

A package that fails any check exits non-zero with the defect named (the
offending segment, the missing strip, the mismatched hash) — it must not be
used as-is.

## Proof (run 2026-11-14, this VM)

- Fresh package into `/tmp/bridge-proof-project`: **94 files**
  (1 manifest + 14 scenes + `onoda.json` + 78 assets), **21/21 checks PASS**,
  exit 0; `--force` rebuild re-verified 21/21.
- Refusal: re-running without `--force` exits 2 and names the populated
  paths.
- Negative test (copy with `cap_017.png` deleted and one segment's
  `endFrame` shifted): exactly 3 FAILs — tiling (naming the segment),
  the missing strip (by name), and the SHA mismatches — exit 1.
- End-to-end: the real `capture_native.mjs check` run against the package
  (`NATIVE_DIR` pointed at it) accepted the manifest and film data — it
  printed its `backend=native · manifest …` and `film: … 1811 frames`
  header lines, which it only reaches after its own validation passes —
  then failed closed at the telemetry gate because the only binary on
  this VM is the stale 13 Nov repo-root build (`unknown argument:
  --film-telemetry`, the staleness seats W33/W35 already documented).
  That is the adapter's designed behaviour with an old binary, not a
  packaging defect; a live gate pass needs a current CI-artifact binary
  via `TALERENDER_BIN`, which does not exist on this VM.

## Honest scope

**This bridges THIS film's data.** The sources are the one film this repo
holds — the Onoda short: one manifest (`scenes/film/film.json`), its 14
segments, one `film/onoda.json`, one asset tree. The packager derives the
scene-file set from the manifest rather than hardcoding it, so it will
package any repo whose data follows the same layout — but nothing here
*creates* film data for a new project: transcribing a project's shots into
segment scene files, generating `onoda.json` (the repo's
`film/gen_film_data.py` + `film/gen_assets.py` did that, by hand, for this
film), and producing caption strips remain separate, per-film work.
That converter now exists — it is `build_bridge.py` in this directory
(above): an unbridged project is bridged with it and then captured
natively; there is no Chrome capture path (the adapter refuses
unbridged projects with the bridge remedy, and says so itself).
*(Corrected 2026-11-16, remediation R2-01 finding: the previous text
predated the generic bridge and the Chrome replacement.)*
