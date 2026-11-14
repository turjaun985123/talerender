# Visual regression — same-renderer drift vs the native golden pins

Scope: CI lavapipe renders vs CI lavapipe pins ONLY. Same-renderer drift is gated; cross-renderer numbers are reported elsewhere and never gated as equality (docs/golden/NATIVE-GOLDEN.md; determinism contract §2).
Band: PSNR >= 80 dB = WITHIN-BAND; below = DRIFT. Byte-identity (MATCH) is the post-cabcc30 norm; the band exists so a sub-defect flutter is classified, not hidden. See tools/ci/visual_regression.py header for the full justification (defect on record: 72.34 dB).

| pin | status | PSNR (dB) | mean \|d\| | %px > 10 | max d | pinned sha | rendered sha | note |
|---|---|---|---|---|---|---|---|---|---|
| s28-onoda-t150 | DRIFT | 41.27 | 0.51230 | 1.20400 | 87 | ff68fad3bbcc | 3f9a1c77e2b4 | PSNR < band 80 dB — same-renderer drift |
| dof-chart | MATCH | inf | — | — | — | 957d5573497e | 957d5573497e | sha256 == pin (byte-identical) |
| stroke-toon | NOT-RENDERED | — | — | — | — | 9ba9ff890402 | - | pinned, but no rendered frame supplied this run |
| ab1-composer | DRIFT | 38.94 | 0.68410 | 1.93200 | 112 | d7bd71a02a3d | 77c05d19aa31 | PSNR < band 80 dB — same-renderer drift |
| film-s01 | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |
| film-s02 | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |
| film-s03 | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |
| film-s04 | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |
| film-s05a | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |
| film-s05b | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |
| film-s06 | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |
| film-s07 | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |
| film-s08 | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |
| film-s08b | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |
| film-s09 | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |
| film-s10 | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |
| film-s11 | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |
| film-s12 | UNPINNED | — | — | — | — | - | - | no pin in golden-hashes.json — staged candidates are not references; pinning is a human act (NATIVE-GOLDEN.md) |

Counts: 1 MATCH · 2 DRIFT · 14 UNPINNED · 1 NOT-RENDERED (of 18 golden items).

VISUAL REGRESSION: FAIL (3 pins, 2 drift)
