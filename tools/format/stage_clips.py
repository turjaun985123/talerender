#!/usr/bin/env python3
"""stage_clips.py — convert film staging data into W15 clip JSON.

The film's staging lives in two spellings that are NOT the W15 clip
grammar (docs/integration/FORMAT-EXT-CLIPS-W15.md): the scene
files' `channels` audit blocks (poseTrack key triples, step
tables, segLerp records, object-form key tables — transcribed from
the source's time behaviour) and the §14.4 `camera.keys` tables.
This tool converts the key-carrying records into a standalone
clips document ({clips: {...}} plus the node/camera instance
stubs the validator needs to resolve targets), so the lead's
anim/clips adoption is mechanical: run the tool, run
tools/format/check_clips.py over the output, merge the `clips`
block into the scene file.

Accepted input (a "staging fragment"): a JSON object carrying any
of —

  camera   the §14.4 block: {eye, target, fovY, keys: [{t, eye,
           target, fovY?, ease?}]}. Ease is already stored on the
           ARRIVING key in this shape (§14.4 / scenefile.rs:788),
           exactly as in the clip grammar — copied 1:1. The block
           carries no clock field and the film's files spell it
           both ways — shot-local keys (0..duration) in
           s01/s03/s06/s07/s08b/s11/s12, film-time keys
           (shot.at..shot.at+duration) in s02/s04/s05a/s05b/s08/
           s09/s10, corroborated by each file's `time` field.
           Film-time keys are rebased by the segment start and
           reported; a span matching neither window FAILs.
  nodes    a list of nodes (recursively through `children`), each
           {name, type, channels: {<channel>: <record>}}. A full
           scene file is a valid fragment (its other fields are
           ignored); the records accepted per channel are the
           shapes the film's files actually carry:

             poseTrack  {kind: "poseTrack", keys: [[t, v],
                        [t, v, moveSecs], ...]} — also the nested
                        form {poseTrack: {keys: [...]}} (s05b),
                        an `elbowKeys` array beside the keys
                        (s09), and an `elbow` sub-record of the
                        same shape (s11). Converted by the W15
                        §5.2 rule: key [t, v, m] becomes a hold
                        key {t-m, <previous value>} plus the
                        arrival {t, v, ease: "smoother"}; a plain
                        [t, v] key uses the film.rs `key()`
                        default moveSecs 0.35; moveSecs 0 becomes
                        ease "step" on the arrival. On the pair
                        channels armL/armR the track lands on
                        component .0; elbow data lands on .1; a
                        scalar `elbow` is a static component (it
                        already lives in the node's `state`) and is
                        reported, not tracked.
             step       {kind: "step", keys|steps: [[t, v], ...]}
                        on mood / legs / lean / headTurn /
                        headTilt. Arrivals carry ease "step"
                        (mood/legs values are enum-checked).
             stepAt     {kind: "stepAt", clock, at, from, to} on
                        mood — a two-entry step table.
             segLerp    {kind: "segLerp", segment: [a, b], from,
                        to} on position — two keys, arrival ease
                        "linear" (see the ease law below).
             keys       array-form [[t, [x, y, z]], ...] on
                        position with a seg() formula — "linear".
             keys       object-form [{t, value, ease?}, ...] on
                        node `translate` / `euler` — already the
                        clip key shape; a record-level
                        "interpolation": "linear" supplies the
                        ease of keys that carry none. `euler`
                        values are [x, y, z] degree triples and
                        split into rotate.x / rotate.y / rotate.z
                        tracks.
  channels a file-root channels block (dotted names); every real
           one is a formula record and lands in check 4.
  duration the shot duration: `duration`, else `shot.duration`.
  segmentStart  where the shot sits in film time: the fragment's
           `segmentStart`, else `shot.at`, else 0. Records whose
           `clock` is "film t" / "t" are rebased by it (local =
           film - segmentStart); "lt" / absent means already
           local. A film-clock record with no segment start in
           the input FAILs by name — rebasing is never guessed.

The ease law for seg-driven position: film.rs `seg()` is the
clamped LINEAR factor (film.rs:112, used verbatim by the s06/s07/
s08/s11 position drivers) and s11's own audit law says so
("lib.ts seg: clamp-linear, no easing"). One record disagrees —
s07's position law parenthesises "(smootherstep, motion.ts)" —
the conversion follows the code of record (linear) and the note
flags the discrepancy; it is not silently resolved here.

Everything else in the channels blocks is CODE, not key data, and
is accounted — by name — in check 4 under the W15 §5.3 classes
(blink directors/beats, talk() waveforms, camPitchOf, gait
derivations walkPhase/walkBob/distKeys, fire/flame formulas,
position drivers without keys, pose bundles and swaps, prop-hold
swaps, visibility windows, derived channels, law-only records).
A record that carries key data but cannot be mapped — an unknown
channel, an unknown record kind, a malformed key, a poseTrack
whose move window overlaps the previous key, a record carrying an
`apply` transform (its keys are PRE-transform values, e.g. s12's
bow track — converting them verbatim would write wrong channel
values) — FAILs the run by name and no output is written. Nothing
is dropped silently: converted, code-only, or failed.

Every emitted number is rounded to the nearest f32 (the §11 data
law; check_clips.py check 5 rejects computed f64 decimals). The
conversion report states the largest rounding delta applied.

Checks (numbered like tools/film/validate_film.py):

  1. input      the fragment parses; duration and segment start
                resolve to numbers; every channel owner is named.
  2. channels   every key-carrying record maps into the W15
                vocabulary for its target; failures name the
                record, the channel and the offending field.
  3. conversion the clips document was assembled: per-clip track /
                key counts, duration extensions, step-table folds
                and drops, static components, f32 deltas.
  4. code-only  every non-converted record is accounted in a
                named §5.3 class; an unaccounted record FAILs.

Usage: python3 tools/format/stage_clips.py INPUT -o OUTPUT
Exit code: 0 = converted (output written), 1 = any check FAIL
(no output written), 2 = input error (unreadable / invalid JSON /
not an object / no camera keys and no channels — one-line ERROR
on stderr, no report). Output is deterministic: document order
throughout, clips in owner order (camera first), numbers f32-
rounded with the shortest round-trip decimal.
"""

import argparse
import json
import re
import struct
import sys

# ------------------------------------------------------------------
# W15 channel vocabulary — copied from tools/format/check_clips.py
# (the validator the output must pass): kinds are "scalar",
# "vec3", "step". Only the tables this converter can target are
# needed; a channel outside them is an unknown channel here too.
# ------------------------------------------------------------------

MOODS = ["neutral", "smug", "happy", "laugh", "shock", "worried",
         "angry", "sad", "sly", "proud", "tired", "dizzy",
         "determined", "embarrassed", "awe", "suspicious", "relieved"]
LEG_STATES = ["stand", "walk", "run", "sit", "jump"]

CHARACTER_SCALARS = ["position", "armL.0", "armL.1", "armR.0",
                     "armR.1", "look.0", "look.1", "blink", "talk",
                     "phase", "lean", "bob", "squash", "headTilt",
                     "headTurn", "camPitch", "sitOn", "height"]
CHARACTER_STEPS = {"mood": MOODS, "legs": LEG_STATES}
# position is vec3; the rest of CHARACTER_SCALARS are scalars.
NODE_CHANNELS = {"translate": "vec3", "scale": "vec3",
                 "rotate.x": "scalar", "rotate.y": "scalar",
                 "rotate.z": "scalar"}
PAIR_CHANNELS = ("armL", "armR")  # state pairs [shoulder, elbow]
DEFAULT_MOVE_SECS = 0.35          # film.rs key() default (film.rs:141)
RESERVED_TARGETS = ("camera", "lights", "post", "fog")


class InputError(Exception):
    """Bad input file — the run cannot start (exit 2, no report)."""


def is_number(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def is_vec3(v):
    return (isinstance(v, list) and len(v) == 3
            and all(is_number(x) for x in v))


def f32q(v):
    """Round a float to the nearest f32 (ints pass through). The
    result is the f64 widening of an exact f32, which check_clips
    check 5 passes outright and json prints shortest-round-trip."""
    if isinstance(v, int):
        return v
    return struct.unpack("f", struct.pack("f", float(v)))[0]


def qnum(v, track_delta):
    """f32-round a number, recording the delta applied."""
    out = f32q(v)
    if isinstance(v, float):
        d = abs(float(out) - v)
        if d > track_delta[0]:
            track_delta[0] = d
    return out


def qvec(v, track_delta):
    return [qnum(x, track_delta) for x in v]


def slug(name):
    s = re.sub(r"[^a-z0-9]+", "-", str(name).lower()).strip("-")
    return s or "node"


# ------------------------------------------------------------------
# Conversion context: shared state for one input file.
# ------------------------------------------------------------------

class Ctx:
    def __init__(self, prefix, duration, seg_start, seg_known):
        self.prefix = prefix            # clip-name prefix (file stem)
        self.duration = duration        # declared shot duration (raw)
        self.seg_start = seg_start      # film-time segment start
        self.seg_known = seg_known      # a segment start was supplied
        self.delta = [0.0]              # largest f32 rounding applied
        self.fail2 = []                 # check 2 failures
        self.fail4 = []                 # check 4 failures
        self.info3 = []                 # check 3 informational lines
        self.code_only = []             # (class, owner, channel) triples
        self.clips = {}                 # clip name -> clip dict
        self.clip_order = []            # clip names in owner order
        self.used_names = set()
        self.n_tracks = 0
        self.n_keys = 0

    def clip_name(self, owner_slug):
        base = "%s-%s" % (self.prefix, owner_slug)
        name, n = base, 2
        while name in self.used_names:
            name = "%s-%d" % (base, n)
            n += 1
        self.used_names.add(name)
        return name

    def local_time(self, rec, t, path):
        """Rebase a record time to clip-local by its clock field."""
        clock = rec.get("clock")
        if clock is None or clock == "lt":
            return t
        if clock in ("film t", "t"):
            if not self.seg_known:
                self.fail2.append(
                    "%s: record clock is %r (film time) but the input "
                    "supplies no segment start (segmentStart / "
                    "shot.at) — refusing to guess the rebase" % (path, clock))
                return None
            lt = t - self.seg_start
            # the segment start is an f64 literal, the record times
            # are f32 transcriptions — their difference jitters by
            # ~1e-6 around a true zero (f32 ulp at film times)
            if abs(lt) < 1e-5:
                lt = 0.0
            return lt
        self.fail2.append("%s: unknown clock %r — expected 'lt' or "
                          "'film t'" % (path, clock))
        return None

    def add_track(self, clip, node, channel, keys):
        clip["tracks"].append({"target": {"node": node,
                                          "channel": channel},
                               "keys": keys})
        self.n_tracks += 1
        self.n_keys += len(keys)

    def new_clip(self, owner_slug):
        name = self.clip_name(owner_slug)
        clip = {"duration": self.duration, "tracks": []}
        self.clips[name] = clip
        self.clip_order.append(name)
        return name, clip


def finish_keys(keys, path, ctx):
    """Quantise times/values, enforce strictly-increasing times.
    `keys` is a list of dicts with numeric t and value already
    validated for shape. Equal-time keys with equal values merge;
    equal times with different values FAIL by name."""
    out = []
    for k in keys:
        k = dict(k)
        k["t"] = qnum(k["t"], ctx.delta)
        v = k["value"]
        if isinstance(v, list):
            k["value"] = qvec(v, ctx.delta)
        elif is_number(v):
            k["value"] = qnum(v, ctx.delta)
        if out and float(k["t"]) == float(out[-1]["t"]):
            if k["value"] == out[-1]["value"]:
                # same instant, same value: the later ease wins if
                # it carries one (it describes the same boundary)
                if "ease" in k:
                    out[-1]["ease"] = k["ease"]
                ctx.info3.append(
                    "%s: merged a duplicate key at t=%r (same value)"
                    % (path, k["t"]))
                continue
            ctx.fail2.append(
                "%s: two keys at t=%r with different values (%r vs "
                "%r) after f32 rounding — ambiguous"
                % (path, k["t"], out[-1]["value"], k["value"]))
            return None
        if out and float(k["t"]) < float(out[-1]["t"]):
            ctx.fail2.append(
                "%s: key time %r follows %r — times must be "
                "increasing" % (path, k["t"], out[-1]["t"]))
            return None
        out.append(k)
    return out


def parse_array_keys(raw, path, ctx, value_kind):
    """Validate a [[t, v], [t, v, m], ...] table. Returns a list of
    (t, v, m-or-None) or None after recording a named failure.
    value_kind: 'scalar' | 'vec3'."""
    if not isinstance(raw, list) or not raw:
        ctx.fail2.append("%s: key table is missing, not an array, "
                         "or empty" % path)
        return None
    out = []
    for i, k in enumerate(raw):
        kp = "%s.keys[%d]" % (path, i)
        if not isinstance(k, list) or len(k) not in (2, 3):
            ctx.fail2.append(
                "%s: key must be [t, value] or [t, value, moveSecs] "
                "— got %r" % (kp, k if not isinstance(k, list) else
                              "array of %d" % len(k)))
            return None
        t, v = k[0], k[1]
        m = k[2] if len(k) == 3 else None
        if not is_number(t):
            ctx.fail2.append("%s: key time %r is not a number" % (kp, t))
            return None
        if value_kind == "scalar" and not is_number(v):
            ctx.fail2.append("%s: key value %r is not a number"
                             % (kp, v))
            return None
        if value_kind == "vec3" and not is_vec3(v):
            ctx.fail2.append("%s: key value %r is not a [x, y, z] "
                             "triple of numbers" % (kp, v))
            return None
        if m is not None and not is_number(m):
            ctx.fail2.append("%s: moveSecs %r is not a number" % (kp, m))
            return None
        if m is not None and m < 0:
            ctx.fail2.append("%s: moveSecs %r is negative" % (kp, m))
            return None
        if out and t <= out[-1][0]:
            ctx.fail2.append(
                "%s: key time %r does not follow %r — the table "
                "must be in increasing time order" % (kp, t, out[-1][0]))
            return None
        out.append((float(t), v, None if m is None else float(m)))
    return out


def convert_posetrack(raw_keys, path, ctx):
    """W15 §5.2: poseTrack [t, v, m] -> hold key + smoother arrival.
    Returns clip keys or None (failure recorded)."""
    parsed = parse_array_keys(raw_keys, path, ctx, "scalar")
    if parsed is None:
        return None
    keys = [{"t": parsed[0][0], "value": parsed[0][1]}]
    prev_v = parsed[0][1]
    for (t, v, m) in parsed[1:]:
        move = DEFAULT_MOVE_SECS if m is None else m
        if move == 0:
            keys.append({"t": t, "value": v, "ease": "step"})
        else:
            hold_t = t - move
            last_t = keys[-1]["t"]
            if hold_t < last_t and abs(hold_t - last_t) > 1e-9:
                ctx.fail2.append(
                    "%s: key at t=%r moves over %rs, so its hold "
                    "would start at %r — before the previous key at "
                    "%r; pose_track() never holds the previous value "
                    "here and the clip grammar cannot express the "
                    "overlap" % (path, t, move, hold_t, last_t))
                return None
            if hold_t > last_t:
                keys.append({"t": hold_t, "value": prev_v})
            keys.append({"t": t, "value": v, "ease": "smoother"})
        prev_v = v
    return finish_keys(keys, path, ctx)


def convert_step_table(entries, path, ctx, kind):
    """Step table [[t, v], ...] (already clock-rebased) -> keys
    with step arrivals. Pre-window entries fold into a t=0 key;
    post-duration entries are dropped — both reported, neither
    silent. kind: 'scalar' | ('step', enum-list)."""
    keys = []
    folded = dropped = 0
    for (t, v) in entries:
        if t < 0:
            # value already in force when the shot opens
            keys = [{"t": 0.0, "value": v, "ease": "step"}]
            folded += 1
        elif t > ctx.duration:
            dropped += 1
        else:
            keys.append({"t": t, "value": v, "ease": "step"})
    if folded:
        ctx.info3.append("%s: %d step(s) before the shot window "
                         "folded into the t=0 key" % (path, folded))
    if dropped:
        ctx.info3.append("%s: %d step(s) after the shot duration "
                         "dropped (never in force in-shot)"
                         % (path, dropped))
    if not keys:
        ctx.fail2.append("%s: every step falls outside the shot "
                         "window — nothing to convert" % path)
        return None
    # every key keeps ease "step", the first included: the clip
    # validator requires it on step channels (check_clips check 3)
    return finish_keys(keys, path, ctx)


def parse_step_entries(raw, path, ctx, channel, enum):
    """Validate a step table's values against its channel kind."""
    parsed = parse_array_keys(raw, path, ctx,
                              "scalar" if enum is None else "step")
    if parsed is None:
        return None
    out = []
    for i, (t, v, m) in enumerate(parsed):
        if m is not None:
            ctx.fail2.append("%s.keys[%d]: step keys take no "
                             "moveSecs" % (path, i))
            return None
        if enum is not None and v not in enum:
            ctx.fail2.append(
                "%s.keys[%d]: value %r is not a valid '%s' — one "
                "of: %s" % (path, i, v, channel, ", ".join(enum)))
            return None
        out.append((t, v))
    return out


# ------------------------------------------------------------------
# Code-only classes (W15 §5.3): a non-converted record must land in
# exactly one of these, named, or check 4 fails. Classes are
# matched by (channel, record shape) in classify_code_only().
# ------------------------------------------------------------------

CLS_BLINK = "blink director / beats (procedural, §5.3)"
CLS_TALK = "talk() waveform (closed form, §5.3)"
CLS_CAMPITCH = "camPitchOf (camera-derived, §5.3)"
CLS_GAIT = "gait derivation — walkPhase/walkBob/driver (§5.3)"
CLS_FIRE = "fire/flame formula (procedural, §5.3)"
CLS_POSE = "pose bundle / stage swap (§5.3)"
CLS_HOLD = "prop-hold swap (attachment state, not a channel)"
CLS_VISIBLE = "visibility window (structural, §5.3)"
CLS_DERIVED = "derived channel (function of another channel, §5.3)"
CLS_LAWONLY = "law-only record (window formula, no key data)"
CLS_FORMULA = "formula record (closed form, §5.3)"


def classify_code_only(channel, rec):
    """Return the §5.3 class for a record that carries no
    convertible key data, or None if it is unaccounted."""
    kind = rec.get("kind")
    name = channel.split(".")[-1]
    if name == "blink" or kind in ("blinkDirector", "blinkOf+blinkAt",
                                   "blinkDirector+blinkAt"):
        return CLS_BLINK
    if kind == "talk" or name == "talk":
        return CLS_TALK
    if kind == "camPitchOf" or name == "camPitch":
        return CLS_CAMPITCH
    if kind in ("walkPhase", "walkBob") or name in ("phase", "bob",
                                                   "walk"):
        return CLS_GAIT
    if name in ("fireLight", "flame", "flame2") or \
            channel.startswith(("fireLight", "flame")):
        return CLS_FIRE
    if kind == "pose" or name == "pose":
        return CLS_POSE
    if name in ("holdL", "holdR"):
        return CLS_HOLD
    if name == "visible":
        return CLS_VISIBLE
    if kind == "derived":
        return CLS_DERIVED
    if name == "sit":
        return CLS_POSE
    if "formula" in rec:
        # a position driver with no keys (s02's camK march) is a
        # gait/driver formula; everything else is a closed form
        return CLS_GAIT if name == "position" else CLS_FORMULA
    if "law" in rec or "driver" in rec:
        return CLS_LAWONLY
    return None


def carries_key_data(rec):
    if not isinstance(rec, dict):
        return False
    if isinstance(rec.get("keys"), list):
        return True
    if isinstance(rec.get("steps"), list):
        return True
    pt = rec.get("poseTrack")
    if isinstance(pt, dict) and isinstance(pt.get("keys"), list):
        return True
    if rec.get("kind") == "poseTrack":
        return True  # keys missing -> a named failure, not code
    if rec.get("kind") == "segLerp":
        return True
    if rec.get("kind") == "stepAt":
        return True
    return False


# ------------------------------------------------------------------
# Per-record converters. Each returns True if it produced track(s).
# ------------------------------------------------------------------

def character_channel_kind(node_type, channel):
    """('scalar'|'vec3'|('step', enum)|None) for a channels-record
    name on a node, in the terms the records use (armR, not
    armR.0)."""
    if node_type != "character":
        return NODE_CHANNELS.get(channel)
    if channel == "position":
        return "vec3"
    if channel in PAIR_CHANNELS or channel in CHARACTER_SCALARS:
        return "scalar"
    if channel in CHARACTER_STEPS:
        return ("step", CHARACTER_STEPS[channel])
    return None


def require_local(rec, path, ctx, what):
    """poseTrack / object-form / array-form key tables are
    shot-local by construction in every record the film carries;
    a film-clock spelling of one is not silently converted."""
    clock = rec.get("clock")
    if clock is not None and clock != "lt":
        ctx.fail2.append(
            "%s: %s record is clocked %r — this shape is "
            "shot-local ('lt') in the film data; rebase it in the "
            "staging data first" % (path, what, clock))
        return False
    return True


def convert_record(ctx, clip, node_name, node_type, channel, rec, path):
    """Convert one key-carrying record into track(s) on `clip`."""
    kind = rec.get("kind")
    is_char = node_type == "character"

    # --- poseTrack family -------------------------------------------
    pt = rec.get("poseTrack") if isinstance(rec.get("poseTrack"),
                                            dict) else None
    if kind == "poseTrack" or pt is not None:
        src = rec if kind == "poseTrack" else pt
        if not require_local(src, path, ctx, "poseTrack"):
            return False
        chan_kind = character_channel_kind(node_type, channel)
        if chan_kind is None:
            ctx.fail2.append(
                "%s: unknown channel '%s' for %s '%s' — poseTrack "
                "data has no W15 channel to land on"
                % (path, channel,
                   "character node" if is_char else "node", node_name))
            return False
        if chan_kind != "scalar":
            ctx.fail2.append(
                "%s: poseTrack values are scalars but channel "
                "'%s' is %s — ambiguous, not converted"
                % (path, channel,
                   "a step channel" if isinstance(chan_kind, tuple)
                   else "vec3"))
            return False
        base = channel + ".0" if channel in PAIR_CHANNELS else channel
        keys = convert_posetrack(src.get("keys"), path, ctx)
        if keys is None:
            return False
        ctx.add_track(clip, node_name, base, keys)
        # elbow: sub-record / elbowKeys -> component .1
        elbow_rec = rec.get("elbow") if isinstance(rec.get("elbow"),
                                                   dict) else None
        if isinstance(rec.get("elbowKeys"), list):
            ekeys = convert_posetrack(rec["elbowKeys"],
                                      path + ".elbowKeys", ctx)
            if ekeys is None:
                return False
            ctx.add_track(clip, node_name, channel + ".1", ekeys)
        elif elbow_rec is not None:
            ekeys = convert_posetrack(elbow_rec.get("keys"),
                                      path + ".elbow", ctx)
            if ekeys is None:
                return False
            ctx.add_track(clip, node_name, channel + ".1", ekeys)
        elif channel in PAIR_CHANNELS:
            if is_number(rec.get("elbow")):
                ctx.info3.append(
                    "%s: %s.1 is the static value %r (record field "
                    "'elbow') — already in the node's state, no "
                    "track emitted" % (path, channel, rec["elbow"]))
            else:
                ctx.info3.append(
                    "%s: %s.1 has no key data in the record — the "
                    "static component stays in the node's state"
                    % (path, channel))
        return True

    # --- step tables --------------------------------------------------
    if kind == "step" and ("keys" in rec or "steps" in rec):
        raw = rec.get("keys", rec.get("steps"))
        chan_kind = character_channel_kind(node_type, channel)
        if chan_kind is None:
            ctx.fail2.append(
                "%s: unknown channel '%s' for %s '%s' — step data "
                "has no W15 channel to land on"
                % (path, channel,
                   "character node" if is_char else "node", node_name))
            return False
        enum = chan_kind[1] if isinstance(chan_kind, tuple) else None
        if isinstance(chan_kind, tuple) or chan_kind == "scalar":
            entries = parse_step_entries(raw, path, ctx, channel, enum)
            if entries is None:
                return False
            rebased = []
            for (t, v) in entries:
                lt = ctx.local_time(rec, t, path)
                if lt is None:
                    return False
                rebased.append((lt, v))
            keys = convert_step_table(rebased, path, ctx, chan_kind)
            if keys is None:
                return False
            ctx.add_track(clip, node_name, channel, keys)
            return True
        ctx.fail2.append("%s: step table on %s channel '%s' — no "
                         "conversion rule" % (path, chan_kind, channel))
        return False

    # --- stepAt (mood) -------------------------------------------------
    if kind == "stepAt":
        if channel != "mood":
            ctx.fail2.append("%s: stepAt record on channel '%s' — "
                             "the only stepAt channel in the film "
                             "data is 'mood'" % (path, channel))
            return False
        frm, to, at = rec.get("from"), rec.get("to"), rec.get("at")
        for field, val in (("from", frm), ("to", to)):
            if val not in MOODS:
                ctx.fail2.append("%s.%s: %r is not a valid 'mood' — "
                                 "one of: %s"
                                 % (path, field, val, ", ".join(MOODS)))
                return False
        if not is_number(at):
            ctx.fail2.append("%s.at: %r is not a number" % (path, at))
            return False
        lt = ctx.local_time(rec, float(at), path)
        if lt is None:
            return False
        if lt < 0:
            # the step fired before the shot opened: `to` is the
            # value in force for the whole shot
            keys = [{"t": 0.0, "value": to, "ease": "step"}]
            ctx.info3.append("%s: stepAt at %r is before the shot "
                             "window — folded to a constant '%s' "
                             "track" % (path, lt, to))
        elif lt > ctx.duration:
            keys = [{"t": 0.0, "value": frm, "ease": "step"}]
            ctx.info3.append("%s: stepAt at %r is after the shot "
                             "duration — the step is never in force "
                             "in-shot; constant '%s' track"
                             % (path, lt, frm))
        elif lt == 0:
            keys = [{"t": 0.0, "value": to, "ease": "step"}]
        else:
            keys = [{"t": 0.0, "value": frm, "ease": "step"},
                    {"t": lt, "value": to, "ease": "step"}]
        keys = finish_keys(keys, path, ctx)
        if keys is None:
            return False
        ctx.add_track(clip, node_name, "mood", keys)
        return True

    # --- segLerp position ----------------------------------------------
    if kind == "segLerp":
        if channel != "position":
            ctx.fail2.append("%s: segLerp record on channel '%s' — "
                             "segLerp is a position shape"
                             % (path, channel))
            return False
        seg = rec.get("segment")
        if not (isinstance(seg, list) and len(seg) == 2
                and all(is_number(x) for x in seg)):
            ctx.fail2.append("%s.segment: %r is not a [a, b] pair "
                             "of numbers" % (path, seg))
            return False
        frm, to = rec.get("from"), rec.get("to")
        if not is_vec3(frm) or not is_vec3(to):
            ctx.fail2.append("%s: segLerp from/to must be [x, y, z] "
                             "triples (got %r / %r)" % (path, frm, to))
            return False
        a = ctx.local_time(rec, float(seg[0]), path)
        b = ctx.local_time(rec, float(seg[1]), path)
        if a is None or b is None:
            return False
        if b <= a:
            ctx.fail2.append("%s.segment: end %r does not follow "
                             "start %r after rebasing" % (path, b, a))
            return False
        keys = finish_keys([{"t": a, "value": frm},
                            {"t": b, "value": to, "ease": "linear"}],
                           path, ctx)
        if keys is None:
            return False
        ctx.add_track(clip, node_name, "position", keys)
        return True

    # --- key tables without a kind --------------------------------------
    if isinstance(rec.get("keys"), list):
        raw = rec["keys"]
        if raw and isinstance(raw[0], dict):
            return convert_object_keys(ctx, clip, node_name, node_type,
                                       channel, rec, path)
        if raw and isinstance(raw[0], list):
            if not require_local(rec, path, ctx, "position key "
                                 "table"):
                return False
            if channel != "position":
                ctx.fail2.append(
                    "%s: array-form key table on channel '%s' — "
                    "the only array-form table shape is position "
                    "(poseTrack/step tables carry a kind)"
                    % (path, channel))
                return False
            evidence = json.dumps(rec)
            if "seg(" not in evidence:
                ctx.fail2.append(
                    "%s: position key table carries no seg() law — "
                    "its interpolation cannot be determined and is "
                    "not guessed" % path)
                return False
            parsed = parse_array_keys(raw, path, ctx, "vec3")
            if parsed is None:
                return False
            keys = []
            for i, (t, v, m) in enumerate(parsed):
                if m is not None:
                    ctx.fail2.append("%s.keys[%d]: position keys "
                                     "take no moveSecs" % (path, i))
                    return False
                k = {"t": t, "value": v}
                if i > 0:
                    k["ease"] = "linear"
                keys.append(k)
            keys = finish_keys(keys, path, ctx)
            if keys is None:
                return False
            ctx.add_track(clip, node_name, "position", keys)
            return True
        ctx.fail2.append("%s: key table is empty or its first key "
                         "is neither an object nor an array" % path)
        return False

    ctx.fail2.append("%s: record kind %r on channel '%s' carries "
                     "no conversion rule" % (path, kind, channel))
    return False


def convert_object_keys(ctx, clip, node_name, node_type, channel,
                        rec, path):
    """Object-form keys [{t, value, ease?}] on translate / euler —
    already the clip key shape (W15 §5.1)."""
    if node_type == "character" or channel not in ("translate",
                                                   "euler"):
        ctx.fail2.append(
            "%s: object-form keys on channel '%s' of %s '%s' — the "
            "object key shape exists in the film data only on node "
            "translate/euler channels"
            % (path, channel,
               "character node" if node_type == "character" else "node",
               node_name))
        return False
    if not require_local(rec, path, ctx, "object-form key table"):
        return False
    interp = rec.get("interpolation")
    if interp is not None and interp != "linear":
        ctx.fail2.append("%s.interpolation: %r — the only record "
                         "interpolation in the film data is "
                         "'linear'" % (path, interp))
        return False
    keys = []
    for i, k in enumerate(rec["keys"]):
        kp = "%s.keys[%d]" % (path, i)
        if not isinstance(k, dict) or not is_number(k.get("t")) \
                or "value" not in k:
            ctx.fail2.append("%s: key must be an object with a "
                             "numeric 't' and a 'value'" % kp)
            return False
        if not is_vec3(k["value"]):
            ctx.fail2.append("%s.value: %r is not a [x, y, z] "
                             "triple of numbers" % (kp, k["value"]))
            return False
        if "ease" in k and not isinstance(k["ease"], str):
            ctx.fail2.append("%s.ease: not a string" % kp)
            return False
        nk = {"t": float(k["t"]), "value": list(k["value"])}
        if "ease" in k:
            nk["ease"] = k["ease"]
        elif interp == "linear" and keys:
            nk["ease"] = "linear"
        keys.append(nk)
    if keys != sorted(keys, key=lambda k: k["t"]):
        ctx.fail2.append("%s: object-form keys are not in "
                         "increasing time order" % path)
        return False
    if channel == "translate":
        keys = finish_keys(keys, path, ctx)
        if keys is None:
            return False
        ctx.add_track(clip, node_name, "translate", keys)
        return True
    # euler: one vec3 table -> three scalar rotation tracks
    for axis, comp in (("rotate.x", 0), ("rotate.y", 1),
                       ("rotate.z", 2)):
        akeys = []
        for k in keys:
            ak = {"t": k["t"], "value": k["value"][comp]}
            if "ease" in k:
                ak["ease"] = k["ease"]
            akeys.append(ak)
        akeys = finish_keys(akeys, path + "." + axis, ctx)
        if akeys is None:
            return False
        ctx.add_track(clip, node_name, axis, akeys)
    return True


# ------------------------------------------------------------------
# Owner processing.
# ------------------------------------------------------------------

def process_channels(ctx, owner_label, node_name, node_type,
                     channels, get_clip):
    """Convert or account every record in one channels block.
    get_clip() lazily creates the owner's clip (only if a track
    lands). Root blocks pass node_name=None: their dotted channel
    names resolve against the reserved targets."""
    for channel, rec in channels.items():
        path = "%s.channels.%s" % (owner_label, channel)
        if not isinstance(rec, dict):
            ctx.fail2.append("%s: channel record is not an object"
                             % path)
            continue
        if "apply" in rec and carries_key_data(rec):
            ctx.fail2.append(
                "%s: record carries an 'apply' transform (%r) — "
                "its keys are pre-transform values, not channel "
                "values; converting them verbatim would write "
                "wrong data. Apply the transform in the staging "
                "data or extend this tool deliberately"
                % (path, rec["apply"]))
            continue
        if carries_key_data(rec):
            if node_name is None:
                prefix = channel.split(".")[0]
                if prefix not in RESERVED_TARGETS:
                    ctx.fail2.append(
                        "%s: root channel '%s' carries key data "
                        "but its target '%s' is not a reserved "
                        "target (%s)"
                        % (path, channel, prefix,
                           ", ".join(RESERVED_TARGETS)))
                    continue
                # no root key data exists in the film today; the
                # record shapes are node shapes, so fail honestly
                ctx.fail2.append(
                    "%s: root channel '%s' carries key data for "
                    "reserved target '%s' — no conversion rule "
                    "(root records in the film data are formulas)"
                    % (path, channel, prefix))
                continue
            clip = get_clip()
            convert_record(ctx, clip, node_name, node_type, channel,
                           rec, path)
        else:
            cls = classify_code_only(channel, rec)
            if cls is None:
                ctx.fail4.append(
                    "%s: record (kind %r, fields %s) is neither "
                    "convertible key data nor a recognised §5.3 "
                    "code-only shape — unaccounted"
                    % (path, rec.get("kind"),
                       ", ".join(sorted(rec.keys()))))
            else:
                ctx.code_only.append((cls, owner_label, channel))


def convert_camera(ctx, cam):
    """§14.4 camera.keys -> eye/target/fovY tracks on a camera
    clip. Ease is already on the arriving key in both grammars —
    copied verbatim (the anim.rs easing-to-next flip is a loader
    concern, W11 §4.1, not a data conversion)."""
    keys = cam.get("keys")
    if not isinstance(keys, list) or not keys:
        return None
    path = "camera.keys"
    base_fov = cam.get("fovY")
    parsed = []
    for i, k in enumerate(keys):
        kp = "%s[%d]" % (path, i)
        if not isinstance(k, dict) or not is_number(k.get("t")):
            ctx.fail2.append("%s: key must be an object with a "
                             "numeric 't'" % kp)
            return None
        for field in ("eye", "target"):
            if not is_vec3(k.get(field)):
                ctx.fail2.append("%s.%s: %r is not a [x, y, z] "
                                 "triple (eval_camera_keys "
                                 "requires it per key)"
                                 % (kp, field, k.get(field)))
                return None
        if "ease" in k and not isinstance(k["ease"], str):
            ctx.fail2.append("%s.ease: not a string" % kp)
            return None
        if "fovY" in k and not is_number(k["fovY"]):
            ctx.fail2.append("%s.fovY: not a number" % kp)
            return None
        parsed.append(k)
    times = [float(k["t"]) for k in parsed]
    if times != sorted(times) or len(set(times)) != len(times):
        ctx.fail2.append("%s: key times are not strictly "
                         "increasing" % path)
        return None
    # The camera block carries no clock field and the film's files
    # spell it BOTH ways: s01/s03/s06/s07/s08b/s11/s12 keys are
    # shot-local (0..duration), s02/s04/s05a/s05b/s08/s09/s10 keys
    # are film-time (shot.at..shot.at+duration — corroborated by
    # each file's own `time` field living in the same clock).
    # Clips are shot-local, so film-time keys are rebased by the
    # segment start; a range matching neither window FAILs.
    offset = 0.0
    if abs(times[0]) <= 1e-6 and times[-1] <= ctx.duration + 1e-3:
        offset = 0.0
    elif ctx.seg_known and abs(times[0] - ctx.seg_start) <= 1e-4 \
            and times[-1] <= ctx.seg_start + ctx.duration + 1e-3:
        offset = ctx.seg_start
        ctx.info3.append(
            "camera.keys: keys are film-clocked [%r..%r] — "
            "rebased by the segment start %r to shot-local"
            % (times[0], times[-1], ctx.seg_start))
    else:
        ctx.fail2.append(
            "%s: key times span [%r..%r] — neither shot-local "
            "(0..%r) nor film time (%r..%r); the camera clock "
            "cannot be determined and is not guessed"
            % (path, times[0], times[-1], ctx.duration,
               ctx.seg_start, ctx.seg_start + ctx.duration))
        return None
    if offset:
        def rebase(t):
            lt = t - offset
            return 0.0 if abs(lt) < 1e-5 else lt  # f32 jitter, ditto
        parsed = [dict(k, t=rebase(float(k["t"]))) for k in parsed]
    name, clip = ctx.new_clip("camera")
    for field, channel in (("eye", "eye"), ("target", "target")):
        tkeys = []
        for k in parsed:
            nk = {"t": float(k["t"]), "value": list(k[field])}
            if "ease" in k:
                nk["ease"] = k["ease"]
            tkeys.append(nk)
        tkeys = finish_keys(tkeys, path + "." + field, ctx)
        if tkeys is None:
            return None
        ctx.add_track(clip, "camera", channel, tkeys)
    if any("fovY" in k for k in parsed):
        if not is_number(base_fov) and \
                any("fovY" not in k for k in parsed):
            ctx.fail2.append(
                "%s: some keys omit fovY and the camera block "
                "carries no base fovY to fall back to "
                "(eval_camera_keys falls back to the base)"
                % path)
            return None
        tkeys = []
        for k in parsed:
            nk = {"t": float(k["t"]),
                  "value": k.get("fovY", base_fov)}
            if "ease" in k:
                nk["ease"] = k["ease"]
            tkeys.append(nk)
        tkeys = finish_keys(tkeys, path + ".fovY", ctx)
        if tkeys is None:
            return None
        ctx.add_track(clip, "camera", "fovY", tkeys)
    return name


def walk_nodes(nodes, out):
    """(node-dict) list in document order, recursively."""
    for n in nodes or []:
        if isinstance(n, dict):
            out.append(n)
            walk_nodes(n.get("children"), out)
    return out


def convert_file(path):
    try:
        with open(path) as fh:
            doc = json.load(fh)
    except OSError as e:
        raise InputError("cannot read %s (%s)" % (path, e.strerror or e))
    except ValueError as e:
        raise InputError("invalid JSON in %s (%s)" % (path, e))
    if not isinstance(doc, dict):
        raise InputError("%s: top level is not a JSON object" % path)

    stem = path.rsplit("/", 1)[-1]
    for suffix in (".scene.json", ".fragment.json", ".json"):
        if stem.endswith(suffix):
            stem = stem[: -len(suffix)]
            break
    prefix = slug(stem)

    shot = doc.get("shot") if isinstance(doc.get("shot"), dict) else {}
    duration = doc.get("duration", shot.get("duration"))
    seg_start = doc.get("segmentStart", shot.get("at"))
    seg_known = seg_start is not None
    if not is_number(duration):
        raise InputError("%s: no numeric duration (duration / "
                         "shot.duration) — clip durations cannot "
                         "be determined" % path)
    if seg_start is None:
        seg_start = 0.0
    if not is_number(seg_start):
        raise InputError("%s: segment start (segmentStart / "
                         "shot.at) is not a number" % path)

    cam = doc.get("camera") if isinstance(doc.get("camera"),
                                          dict) else {}
    nodes = walk_nodes(doc.get("nodes"), [])
    has_channels = any(isinstance(n.get("channels"), dict)
                       for n in nodes) \
        or isinstance(doc.get("channels"), dict)
    if not cam.get("keys") and not has_channels:
        raise InputError("%s: no camera keys and no channels "
                         "blocks — nothing to stage" % path)

    ctx = Ctx(prefix, float(duration), float(seg_start), seg_known)
    in_fail = []

    # ---- 1 input -------------------------------------------------
    for i, n in enumerate(nodes):
        if isinstance(n.get("channels"), dict) and \
                not isinstance(n.get("name"), str):
            in_fail.append("nodes: the node at document position "
                           "%d carries channels but has no string "
                           "'name' — its tracks would be "
                           "unaddressable" % i)
            n["_unnamed"] = True
    ctx.in_fail = in_fail

    # ---- camera --------------------------------------------------
    cam_clip = convert_camera(ctx, cam) if cam.get("keys") else None

    # ---- node + root channels -------------------------------------
    instances = {}   # node name -> (type, clip name)
    for n in nodes:
        channels = n.get("channels")
        if not isinstance(channels, dict) or n.get("_unnamed"):
            continue
        holder = {}

        def get_clip(n=n, holder=holder):
            if "clip" not in holder:
                name, clip = ctx.new_clip(slug(n["name"]))
                holder["clip"] = clip
                holder["name"] = name
                instances[n["name"]] = (n.get("type"), name)
            return holder["clip"]

        process_channels(ctx, "nodes[%s]" % n["name"], n["name"],
                         n.get("type"), channels, get_clip)
    if isinstance(doc.get("channels"), dict):
        process_channels(ctx, "<root>", None, None, doc["channels"],
                         lambda: None)

    # ---- prune clips left empty by a failed record (the run ------
    # ---- fails on check 2 regardless; this keeps the report -----
    # ---- free of knock-on failures) ------------------------------
    for name in list(ctx.clip_order):
        if not ctx.clips[name]["tracks"]:
            del ctx.clips[name]
            ctx.clip_order.remove(name)
            for nm, (ty, cn) in list(instances.items()):
                if cn == name:
                    del instances[nm]

    # ---- durations: extend past the last key where the data ------
    # ---- runs past the declared shot duration (reported) ---------
    for name in ctx.clip_order:
        clip = ctx.clips[name]
        last = 0.0
        for tr in clip["tracks"]:
            for k in tr["keys"]:
                last = max(last, float(k["t"]))
        dur = qnum(ctx.duration, ctx.delta)
        if float(last) > float(dur):
            why = ("f32 transcription jitter at the segment "
                   "boundary" if float(last) - float(dur) <= 1e-5
                   else "converted keys run past the declared "
                        "shot duration (the move is unfinished "
                        "at the cut, as staged)")
            ctx.info3.append(
                "clips.%s.duration: extended %r -> %r — %s"
                % (name, dur, last, why))
            dur = last
        clip["duration"] = dur

    # ---- assemble --------------------------------------------------
    out = {"generatedBy": "tools/format/stage_clips.py",
           "source": path,
           "clips": {}}
    for name in ctx.clip_order:
        out["clips"][name] = ctx.clips[name]
    stubs = []
    seen_stub = set()
    for n in nodes:
        nm = n.get("name")
        if nm in instances and nm not in seen_stub:
            seen_stub.add(nm)
            stub = {"name": nm}
            if isinstance(n.get("type"), str):
                stub["type"] = n["type"]
            stub["clip"] = instances[nm][1]
            stubs.append(stub)
    if stubs:
        out["nodes"] = stubs
    if cam_clip:
        out["camera"] = {"clip": cam_clip}
    return ctx, out


def report(ctx, path, wrote):
    print("stage_clips.py — %s (%d clips, %d tracks, %d keys "
          "staged; %d records code-only)"
          % (path, len(ctx.clip_order), ctx.n_tracks, ctx.n_keys,
             len(ctx.code_only)))
    results = []

    f1 = list(getattr(ctx, "in_fail", []))
    results.append(("1 input", f1))

    results.append(("2 channels", ctx.fail2))

    f3 = ["clips.%s: clip has no tracks" % name
          for name in ctx.clip_order if not ctx.clips[name]["tracks"]]
    results.append(("3 conversion", f3))

    results.append(("4 code-only", ctx.fail4))

    all_ok = True
    for name, failures in results:
        ok = not failures
        all_ok = all_ok and ok
        print("\n[%s] %s — %s"
              % (name, "PASS" if ok else "FAIL",
                 "no failures" if ok else "%d failure(s)"
                 % len(failures)))
        for line in failures:
            print("    " + line)
    if ctx.info3:
        print("\n    conversion notes:")
        for line in ctx.info3:
            print("      " + line)
    if ctx.code_only:
        print("\n    code-only records (W15 §5.3 accounting):")
        by_class = {}
        for cls, owner, channel in ctx.code_only:
            by_class.setdefault(cls, []).append("%s.%s"
                                                 % (owner, channel))
        for cls in sorted(by_class):
            items = by_class[cls]
            print("      %s — %d: %s"
                  % (cls, len(items), ", ".join(items)))
    if ctx.delta[0]:
        print("\n    largest f32 rounding applied to an emitted "
              "number: %.3g" % ctx.delta[0])
    print("\nOVERALL: %s (%d/%d checks passed)%s"
          % ("PASS" if all_ok else "FAIL",
             sum(1 for _, f in results if not f), len(results),
             "" if wrote else " — no output written"))
    return 0 if all_ok else 1


def main():
    ap = argparse.ArgumentParser(
        description="Convert film staging data (channels blocks + "
                    "camera keys) into a W15 clips document.")
    ap.add_argument("input", help="staging fragment / scene JSON")
    ap.add_argument("-o", "--output", required=True,
                    help="where to write the clips document")
    args = ap.parse_args()
    ctx, out = convert_file(args.input)
    ok = not ctx.fail2 and not ctx.fail4 \
        and not getattr(ctx, "in_fail", [])
    wrote = False
    if ok:
        with open(args.output, "w") as fh:
            json.dump(out, fh, indent=2)
            fh.write("\n")
        wrote = True
    return report(ctx, args.input, wrote)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except InputError as e:
        print("ERROR: %s" % e, file=sys.stderr)
        sys.exit(2)
