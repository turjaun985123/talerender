# MINI-DRAFT — synthetic §15-shaped draft for check_foldin.py's self-test

This preamble is outside the paste text and must not be counted.

---

## 15. Mini extensions

Two small extensions ride the existing format version.

### 15.1 The reservation guard (mini)

A loader must hard-error, naming the unsupported extension,
when it encounters a reserved key. Silently dropping a
reserved key is forbidden. Instances never carry a `type` key.

### 15.2 Expansion (mini)

Substitution must be verbatim, with no arithmetic and no
expressions. A cycle is a hard error, and the error prints
the cycle path. Widgets must always be expanded before any
node is walked.

---

Trailing notes outside the paste text. A loader must weep —
this sentence is normative-looking but is NOT in the paste
text and must not enter the census.
