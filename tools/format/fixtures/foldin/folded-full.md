# Mini spec — folded target (all clauses present)

## 1. Skeleton

Some base-format text with no extension content at all.

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

## 16. Later material

Unrelated closing text.
