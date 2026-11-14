# Mini spec — folded target (one clause dropped, one normative weakened)

## 1. Skeleton

Some base-format text with no extension content at all.

## 15. Mini extensions

Two small extensions ride the existing format version.

### 15.1 The reservation guard (mini)

A loader must hard-error, naming the unsupported extension,
when it encounters a reserved key. Silently dropping a
reserved key is forbidden. Instances usually carry no `type` key.

### 15.2 Expansion (mini)

Substitution must be verbatim, with no arithmetic and no
expressions. Widgets must always be expanded before any
node is walked.

## 16. Later material

Unrelated closing text.
