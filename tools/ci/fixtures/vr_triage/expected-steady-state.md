# Visual regression — same-renderer drift vs the native golden pins

Scope: CI lavapipe renders vs CI lavapipe pins ONLY. Same-renderer drift is gated; cross-renderer numbers are reported elsewhere and never gated as equality (docs/golden/NATIVE-GOLDEN.md; determinism contract §2).
Band: PSNR >= 80 dB = WITHIN-BAND; below = DRIFT. Byte-identity (MATCH) is the post-cabcc30 norm; the band exists so a sub-defect flutter is classified, not hidden. See tools/ci/visual_regression.py header for the full justification (defect on record: 72.34 dB).

| pin | status | PSNR (dB) | mean \|d\| | %px > 10 | max d | pinned sha | rendered sha | note |
|---|---|---|---|---|---|---|---|---|---|
| dof-chart | MATCH | inf | — | — | — | 957d5573497e | 957d5573497e | sha256 == pin (byte-identical) |
| stroke-toon | NOT-RENDERED | — | — | — | — | 9ba9ff890402 | - | pinned, but no rendered frame supplied this run |
| ab1-composer | MATCH | inf | — | — | — | 60c32c1af021 | 60c32c1af021 | sha256 == pin (byte-identical) |
| film-s01 | NOT-RENDERED | — | — | — | — | 1608ead9a477 | - | pinned, but no rendered frame supplied this run |
| film-s02 | NOT-RENDERED | — | — | — | — | 9649a9b65fa3 | - | pinned, but no rendered frame supplied this run |
| film-s03 | NOT-RENDERED | — | — | — | — | 0bbdbdd8a939 | - | pinned, but no rendered frame supplied this run |
| film-s04 | NOT-RENDERED | — | — | — | — | f135462d330c | - | pinned, but no rendered frame supplied this run |
| film-s05a | NOT-RENDERED | — | — | — | — | bb0ee0c0d969 | - | pinned, but no rendered frame supplied this run |
| film-s05b | NOT-RENDERED | — | — | — | — | 471366bffdc7 | - | pinned, but no rendered frame supplied this run |
| film-s06 | NOT-RENDERED | — | — | — | — | 7eff42cdf8c8 | - | pinned, but no rendered frame supplied this run |
| film-s07 | NOT-RENDERED | — | — | — | — | 7f8e1555249f | - | pinned, but no rendered frame supplied this run |
| film-s08 | NOT-RENDERED | — | — | — | — | 3e8e703f2560 | - | pinned, but no rendered frame supplied this run |
| film-s08b | MATCH | inf | — | — | — | fb4e2ba44ad8 | fb4e2ba44ad8 | sha256 == pin (byte-identical) |
| film-s09 | NOT-RENDERED | — | — | — | — | 74b6003d0eda | - | pinned, but no rendered frame supplied this run |
| film-s10 | NOT-RENDERED | — | — | — | — | 59bdb4f2558f | - | pinned, but no rendered frame supplied this run |
| film-s11 | NOT-RENDERED | — | — | — | — | 7bed0da5f199 | - | pinned, but no rendered frame supplied this run |
| film-s12 | NOT-RENDERED | — | — | — | — | dfabc09a542d | - | pinned, but no rendered frame supplied this run |

Counts: 3 MATCH · 0 DRIFT · 14 NOT-RENDERED (of 17 golden items).

VISUAL REGRESSION: PASS (3 pins, 0 drift)
