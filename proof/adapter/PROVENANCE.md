# Adapter bundle — provenance (CR-B3-02)

The files under `proof/adapter/tools/` are **byte-identical copies** of
the shipped Tall Tale Shorts skill's native adapter, taken from the
skill tree on this machine at staging time (2026-11-16):

| Staged file | Source (skill tree) |
|---|---|
| `tools/capture_native.mjs` | `~/workspace/custom-skills/tall-tale-shorts/template/tools/capture_native.mjs` |
| `tools/CAPTURE-NATIVE.md` | `~/workspace/custom-skills/tall-tale-shorts/template/tools/CAPTURE-NATIVE.md` |
| `tools/castsheet_native.py` | `~/workspace/custom-skills/tall-tale-shorts/template/tools/castsheet_native.py` |
| `tools/cues_native.py` | `~/workspace/custom-skills/tall-tale-shorts/template/tools/cues_native.py` |
| `tools/native_modes/*.mjs` (7 files) | `~/workspace/custom-skills/tall-tale-shorts/template/tools/native_modes/*.mjs` |

Why a copy: the proof workflow runs inside the `talerender` repo,
which does not contain the skill. The skill repo is not yet synced
with the Batch-2 native-first template, so the workflow cannot fetch
the adapter from it; this bundle is the stand-in, and it is staged —
not hand-written — so the proof exercises the adapter exactly as
shipped. When the skill sync lands, this bundle should be replaced by
a checkout of the skill tree (or the copies re-verified against it).

## sha256 (verified identical on both sides at staging)

```
5d7c350024cd94fdbc53216b14c38d83f5c94919e1e8ac08bb2c17fe64e1ed97  capture_native.mjs
246d830c404cc4201bcfb2381d6e867c2db5dc7ad32a577bdb3a7e215879b5a7  CAPTURE-NATIVE.md
1529cdcec5b2667be7a030ab286ca01d9271c56d7d6540c5c81f5a27a878c38c  castsheet_native.py
0d35ff621dc16b94636703b13efdb7995f07adb91c99fdec677e5f75c9ccf56e  cues_native.py
03aef1b1bd8ef89d50c8f6e9407a931c496a79abaab6ecc7f61cb5fdbb237b62  native_modes/castsheet_mode.mjs
5d9d486c4a6800e174d8b42bb4017bea3457d7432b17ee2f64c65dfdb084b42e  native_modes/check_mode.mjs
9070276a493ec54cd9eb7b6a8147f424e39ac33a09ec57ab674f16a0c8dbc7f6  native_modes/era_hint.mjs
25decec8bc2012adb41ba6297fb48852f4d9a431c3de96c97b4bf2203d7f54ed  native_modes/filmdata_validate.mjs
c92f845eeef86ceb7c29bbf7dadaa2a50b4bee35bf1dc3ce850c70bb72885458  native_modes/mode_dispatch.mjs
09b21d47f71cc9d96433b738cac9beac09cc8d639ef04cb23211cb4f14e8c695  native_modes/render_meta.mjs
c44e0ac4645ad699830498e3e52c108498074a1eb8e59ad132abd40eba51eefa  native_modes/sheet_mode.mjs
```

Re-verify after copying into the repo:

```bash
cd proof/adapter/tools && sha256sum -c <<'EOF'
5d7c350024cd94fdbc53216b14c38d83f5c94919e1e8ac08bb2c17fe64e1ed97  capture_native.mjs
246d830c404cc4201bcfb2381d6e867c2db5dc7ad32a577bdb3a7e215879b5a7  CAPTURE-NATIVE.md
1529cdcec5b2667be7a030ab286ca01d9271c56d7d6540c5c81f5a27a878c38c  castsheet_native.py
0d35ff621dc16b94636703b13efdb7995f07adb91c99fdec677e5f75c9ccf56e  cues_native.py
03aef1b1bd8ef89d50c8f6e9407a931c496a79abaab6ecc7f61cb5fdbb237b62  native_modes/castsheet_mode.mjs
5d9d486c4a6800e174d8b42bb4017bea3457d7432b17ee2f64c65dfdb084b42e  native_modes/check_mode.mjs
9070276a493ec54cd9eb7b6a8147f424e39ac33a09ec57ab674f16a0c8dbc7f6  native_modes/era_hint.mjs
25decec8bc2012adb41ba6297fb48852f4d9a431c3de96c97b4bf2203d7f54ed  native_modes/filmdata_validate.mjs
c92f845eeef86ceb7c29bbf7dadaa2a50b4bee35bf1dc3ce850c70bb72885458  native_modes/mode_dispatch.mjs
09b21d47f71cc9d96433b738cac9beac09cc8d639ef04cb23211cb4f14e8c695  native_modes/render_meta.mjs
c44e0ac4645ad699830498e3e52c108498074a1eb8e59ad132abd40eba51eefa  native_modes/sheet_mode.mjs
EOF
```

Adapter era note: this is the Batch-1-wired, Batch-2-shipped adapter
(mode dispatch + check/sheet parity + castsheet native). Its contract
is `tools/CAPTURE-NATIVE.md` in this bundle. The binary it drives in
the proof is built from the same repo checkout the workflow runs in —
binary and bridge data are era-paired by construction.
