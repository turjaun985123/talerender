#!/usr/bin/env python3
"""Tall Tale — native cues emitter (Chrome-free replacement for
`node tools/capture.mjs cues`).

  python3 tools/cues_native.py [PROJECT_OR_NATIVE_DIR] [-o OUT.json]
                               [--events EVENTS.json] [--strict-events]

Reads a project's BRIDGED native film data — the film manifest
(<native>/film.json, or <native>/scenes/film/film.json in the repo
layout) plus the segment scene files it names — and writes
data/cues.json in the exact contract shape tools/mix.py consumes:

  {"duration": <film duration s>, "cues": [{"t": ..., "type": ...,
   "gain": ..., "dur": ..., "pan": ...}, ...]}

1-space JSON indent, no trailing newline, cues sorted by t — byte-for-
byte the serialisation `capture.mjs cues` produces from OP.cues()
(capture.mjs: `JSON.stringify({duration, cues}, null, 1)`).

WHY THIS EXISTS. In the Chrome flow the cues are extracted by
evaluating OP.cues() in the page, but they are authored DATA, not a
render product: buildShots' cue() helper (template/src/shots.ts)
collects {t, type, gain?, dur?} literals anchored to word times, and
a bridged project's scene files already carry those literals as
shot.cues, transcribed from shots.ts. Page evaluation was an
extraction convenience; this tool is the same extraction over the
scene data. An unbridged project FAILS LOUDLY naming the bridge
step — there is no Chrome fallback.

NORMALISATION (the scene data's two spellings, one canonical out):
  t     <- "t" or "at";  type <- "type" or "name"
  gain / dur / pan pass through verbatim, omitted when absent
  (mix.py defaults: gain 1.0, synth default length, pan 0).
  Provenance keys ("source", anything else) are dropped, counted,
  and reported — the Chrome file never carries them.
  t is rounded to 3 dp, mirroring the cue() helper's +t.toFixed(3);
  gain/dur are NOT rounded (the helper spreads them verbatim).
  Numbers that are integral are emitted without a decimal point,
  mirroring JSON.stringify (JS has no int/float distinction).
  duration <- manifest "durationS", else the manifest's sourceData
  file's "duration", else the last segment's endS. It is the film /
  lines duration (60.357 on Onoda), NEVER the mix.wav duration
  (60.46) — mix.py sizes its canvas from this value.

CLIP EVENTS ARE NOT CUES. Clips/actions emit semantic ClipEvents
({t, cue, gain?}, clips.ts) — 'takeoff', 'land', 'fall-impact',
'trip', 'slip', 'recover', 'lose-balance', 'roll', 'push-up' (plus
'pop'/'inhale', which double as FX names). mix.py SKIPS any cue
whose type is not one of its 21 FX synths, so an event written
straight into cues.json is a beat with no sound. The standard path
is tools/audio_recipes.py build_cues, which compiles events +
a recipe (+ this file as base) into a mix-ready cues.json and
fails loudly on unmapped events. Accordingly this emitter NEVER
merges events into cues.json: events found in scene data
(shot.events / top-level "events", absolute film time) or supplied
via --events are written, in build_cues' input shape, to
clip-events.json beside the output and reported on stdout.
--strict-events makes their presence an error (exit 3) for gates
that want authored-cues-only films to stay that way.

An authored cue whose type is a known event name is routed the
same way (it was never audible via mix.py). An authored cue whose
type is NEITHER an FX name NOR a known event name is a hard error
(exit 2, nothing written) — build_cues applies the same discipline
to base cues, because mix.py would skip it silently.

Exit codes: 0 = written; 2 = bad input / unbridged / contract
violation; 3 = --strict-events tripped (cues.json still written).

Stdlib only. No clock, no RNG: same inputs -> byte-identical output.
"""
import json
import sys
from decimal import Decimal, ROUND_HALF_UP
from pathlib import Path

# mix.py's FX table (21 synths) — the only legal authored cue types.
FX_NAMES = frozenset([
    'ding', 'chirp', 'chug', 'stamp', 'inhale', 'heartbeat', 'whoosh',
    'slide', 'pop', 'plop', 'gulp', 'boing', 'bubbles', 'sizzle',
    'sparkle', 'thud', 'slam', 'sting', 'tock', 'zip', 'alarm',
])
# audio_recipes.py EVENT_VOCABULARY — the clip-event names the
# engine emits today ('pop'/'inhale' also appear in FX_NAMES).
EVENT_NAMES = frozenset([
    'takeoff', 'land', 'fall-impact', 'trip', 'slip', 'recover',
    'lose-balance', 'roll', 'push-up', 'pop', 'inhale',
])
CUE_KEYS = ('gain', 'dur', 'pan')


class CueError(ValueError):
    """Input or contract failure. Always loud, always exit 2."""


def _is_num(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def _num(v):
    """JS-number serialisation: integral floats become ints, so
    json.dumps spells 1 as `1`, exactly like JSON.stringify."""
    f = float(v)
    return int(f) if f.is_integer() else f


def _round_t(t):
    """Mirror the cue() helper's +t.toFixed(3): 3 dp, half up."""
    q = Decimal(repr(float(t))).quantize(
        Decimal('0.001'), rounding=ROUND_HALF_UP)
    return _num(float(q))


def _load_json(path, what):
    try:
        return json.loads(Path(path).read_text(encoding='utf-8'))
    except Exception as e:
        raise CueError(f'cannot read {what} {path} '
                       f'({type(e).__name__}: {e})')


# ------------------------------------------------------------ discovery
def find_native(arg):
    """Resolve the input to (native_root, manifest_path).

    Accepts a project dir (native/ inside it), a native dir in
    either layout (film.json at its root, or scenes/film/film.json —
    the two layouts package_bridge.py's findManifest accepts), or a
    manifest file itself. Anything else is an unbridged project.
    """
    p = Path(arg)
    if p.is_file():
        if p.name == 'film.json':
            return p.parent, p
        raise CueError(f'{arg} is a file but not a film manifest '
                       f'(film.json) — cannot locate native film data')
    candidates = []
    for root in (p, p / 'native'):
        candidates.append((root, root / 'film.json'))
        candidates.append((root, root / 'scenes' / 'film' / 'film.json'))
    for root, manifest in candidates:
        if manifest.is_file():
            return root, manifest
    raise CueError(
        f'no native film data found under {p} (looked for film.json, '
        f'native/film.json, scenes/film/film.json, '
        f'native/scenes/film/film.json) — this project is not '
        f'bridged. Bridge it first (the TaleRender bridge builder; '
        f'for packaged film data, tools/bridge/package_bridge.py in '
        f'the talerender repo). There is no Chrome fallback: cues '
        f'are read from native scene data, never from a page.')


def resolve_scene(root, manifest_path, rel):
    """Segment scene paths in the manifest are native-root-relative
    ('scenes/film/s01.scene.json') in both layouts; try the root,
    then the manifest's own dir and its parents, in that order."""
    tried = []
    for base in (root, manifest_path.parent,
                 manifest_path.parent.parent,
                 manifest_path.parent.parent.parent):
        cand = base / rel
        tried.append(cand)
        if cand.is_file():
            return cand
    raise CueError(f'manifest names scene {rel!r} but it exists at '
                   f'none of: ' + ', '.join(str(t) for t in tried))


def scene_files(root, manifest_path, manifest):
    """(segment_id, path) pairs in manifest order; without a
    segments list, every scenes/film/*.scene.json sorted by name."""
    segs = manifest.get('segments')
    if isinstance(segs, list) and segs:
        out = []
        for seg in segs:
            rel = seg.get('scene')
            if not rel:
                raise CueError(f'manifest segment '
                               f'{seg.get("id", "?")!r} has no '
                               f'"scene" path')
            out.append((str(seg.get('id', Path(rel).stem)),
                        resolve_scene(root, manifest_path, rel)))
        return out
    files = sorted((root / 'scenes' / 'film').glob('*.scene.json'))
    if not files:
        raise CueError(f'manifest {manifest_path} has no segments '
                       f'and {root}/scenes/film holds no '
                       f'*.scene.json — nothing to read cues from')
    return [(f.name.split('.')[0], f) for f in files]


def film_duration(root, manifest_path, manifest, files):
    if _is_num(manifest.get('durationS')):
        return _num(manifest['durationS'])
    src = manifest.get('sourceData')
    if isinstance(src, str):
        for cand in (root / src, root / 'film' / Path(src).name,
                     manifest_path.parent / src):
            if cand.is_file():
                doc = _load_json(cand, 'sourceData')
                if _is_num(doc.get('duration')):
                    return _num(doc['duration'])
    ends = [s.get('endS') for s in manifest.get('segments') or []
            if _is_num(s.get('endS'))]
    if ends:
        return _num(max(ends))
    raise CueError(f'no film duration available: manifest '
                   f'{manifest_path} has no "durationS", no readable '
                   f'sourceData duration, and no segment endS')


# ------------------------------------------------------------ extraction
def _split_entry(entry, where, notes, as_event=False):
    """One raw scene-data entry -> (kind, normalised dict).

    kind is 'cue' (an FX cue for cues.json) or 'event' (a clip event
    for the clip-events sidecar). With as_event=True the entry was
    DECLARED as an event (shot.events / an --events file), so even
    an FX name ('pop', 'inhale', or a literal-FX event build_cues
    accepts) stays an event; declared in shot.cues, the same FX
    name is an authored cue. Raises CueError on anything that fits
    neither vocabulary or lacks a usable time/name.
    """
    if not isinstance(entry, dict):
        raise CueError(f'{where}: entry is not an object: {entry!r}')
    t = entry.get('t', entry.get('at'))
    if not _is_num(t):
        raise CueError(f'{where}: entry has no numeric "t"/"at": '
                       f'{entry!r}')
    if 't' in entry and 'at' in entry and \
            float(entry['t']) != float(entry['at']):
        raise CueError(f'{where}: conflicting "t"={entry["t"]} and '
                       f'"at"={entry["at"]} — fix the scene data')
    names = {entry[k] for k in ('type', 'name', 'cue')
             if isinstance(entry.get(k), str)}
    if len(names) > 1:
        raise CueError(f'{where}: conflicting type/name/cue '
                       f'{sorted(names)} — fix the scene data')
    name = names.pop() if names else None
    if not name:
        raise CueError(f'{where}: entry has no "type"/"name"/"cue": '
                       f'{entry!r}')
    spelled = ('at' in entry and 't' not in entry) or \
              ('name' in entry and 'type' not in entry)
    dropped = sorted(set(entry) - {'t', 'at', 'type', 'name', 'cue',
                                   'clock'} - set(CUE_KEYS))
    if not as_event and name in FX_NAMES:
        out = {'t': _round_t(t), 'type': name}
        for k in CUE_KEYS:
            if entry.get(k) is not None:
                if not _is_num(entry[k]):
                    raise CueError(f'{where}: {k!r} is not a '
                                   f'number: {entry!r}')
                out[k] = _num(entry[k])
        if spelled or dropped:
            notes.append(f'{where}: {name} normalised '
                         f'({"at->t" if "at" in entry else ""}'
                         f'{" name->type" if "name" in entry else ""}'
                         f'{", dropped " + ",".join(dropped) if dropped else ""})')
        return 'cue', out
    if name in EVENT_NAMES or (as_event and name in FX_NAMES):
        out = {'t': _round_t(t), 'cue': name}
        for k in ('gain', 'dur'):
            if entry.get(k) is not None:
                if not _is_num(entry[k]):
                    raise CueError(f'{where}: event {k!r} is not '
                                   f'a number: {entry!r}')
                out[k] = _num(entry[k])
        return 'event', out
    raise CueError(
        f'{where}: unknown cue type {name!r} — mix.py would skip '
        f'it silently. Valid FX: {", ".join(sorted(FX_NAMES))}; '
        f'valid clip events: {", ".join(sorted(EVENT_NAMES))}')


def _check_clock(entry, where):
    """Events must already be in absolute film time. A clip-local
    clock cannot be rebased here — the instance offset lives in
    staging, not scene data — so it fails loudly for the bridge."""
    clock = entry.get('clock')
    if clock is not None and str(clock).lower() not in (
            'film', 'film t', 'absolute', 't'):
        raise CueError(
            f'{where}: event clock {clock!r} is not absolute film '
            f'time — rebase it at bridge time (shot start + clip '
            f'instance offset + event t) and write the result as '
            f'plain "t"; cues_native never guesses an offset')


def collect(files, notes):
    """Read every segment scene file -> (cues, events), each sorted
    by t (stable, so equal times keep segment/authoring order)."""
    cues, events = [], []
    for seg_id, path in files:
        doc = _load_json(path, f'scene {seg_id}')
        shot = doc.get('shot') if isinstance(doc.get('shot'), dict) \
            else {}
        raw_cues = shot.get('cues')
        if raw_cues is None:
            raw_cues = doc.get('cues') or []
        elif doc.get('cues'):
            notes.append(f'{seg_id}: both shot.cues and top-level '
                         f'cues present — shot.cues used, top-level '
                         f'ignored')
        raw_events = shot.get('events')
        if raw_events is None:
            raw_events = doc.get('events') or []
        if not isinstance(raw_cues, list) or \
                not isinstance(raw_events, list):
            raise CueError(f'{seg_id} ({path}): shot.cues / events '
                           f'must be arrays')
        for i, e in enumerate(raw_cues):
            kind, out = _split_entry(e, f'{seg_id} cues[{i}]', notes)
            (cues if kind == 'cue' else events).append(out)
            if kind == 'event':
                notes.append(f'{seg_id} cues[{i}]: {out["cue"]!r} is '
                             f'a clip event, not an FX cue — routed '
                             f'to clip-events.json, NOT cues.json')
        for i, e in enumerate(raw_events):
            _check_clock(e, f'{seg_id} events[{i}]')
            kind, out = _split_entry(e, f'{seg_id} events[{i}]',
                                     notes, as_event=True)
            events.append(out)
    cues.sort(key=lambda c: c['t'])
    events.sort(key=lambda c: c['t'])
    return cues, events


def load_events_file(path, notes):
    """--events input, build_cues' shape: [{t, cue, gain?, dur?}]
    or {"events": [...]} / {"cues": [...]}. Absolute film time."""
    doc = _load_json(path, 'events')
    raw = doc
    if isinstance(doc, dict):
        raw = doc.get('events')
        if raw is None:
            raw = doc.get('cues')
    if not isinstance(raw, list):
        raise CueError(f'events file {path}: expected a list or an '
                       f'object with "events" — got '
                       f'{type(raw).__name__}')
    out = []
    for i, e in enumerate(raw):
        _check_clock(e, f'{path} events[{i}]')
        kind, norm = _split_entry(e, f'{path} events[{i}]', notes,
                                  as_event=True)
        out.append(norm)
    out.sort(key=lambda c: c['t'])
    return out


# ------------------------------------------------------------ main
def main(argv):
    opts = argv[1:]
    args = []
    out_path = None
    events_path = None
    strict_events = False
    i = 0
    while i < len(opts):
        if opts[i] == '-o' and i + 1 < len(opts):
            out_path = opts[i + 1]; i += 2
        elif opts[i] == '--events' and i + 1 < len(opts):
            events_path = opts[i + 1]; i += 2
        elif opts[i] == '--strict-events':
            strict_events = True; i += 1
        elif opts[i].startswith('-'):
            print(f'cues_native: unknown option {opts[i]!r} '
                  f'(usage: cues_native.py [DIR] [-o OUT] '
                  f'[--events EVENTS.json] [--strict-events])')
            return 2
        else:
            args.append(opts[i]); i += 1
    if len(args) > 1:
        print(f'cues_native: expected at most one directory '
              f'argument, got {args}')
        return 2
    arg = args[0] if args else '.'
    try:
        root, manifest_path = find_native(arg)
        manifest = _load_json(manifest_path, 'manifest')
        if not isinstance(manifest, dict):
            raise CueError(f'manifest {manifest_path} is not a '
                           f'JSON object')
        files = scene_files(root, manifest_path, manifest)
        duration = film_duration(root, manifest_path, manifest,
                                 files)
        notes = []
        cues, events = collect(files, notes)
        if events_path:
            events += load_events_file(events_path, notes)
            events.sort(key=lambda c: c['t'])
    except CueError as e:
        print(f'cues_native: {e}')
        return 2

    if out_path:
        out = Path(out_path)
    else:
        base = Path(arg)
        project = base if (base / 'native').is_dir() else \
            (root.parent if root.name == 'native' else root)
        out = project / 'data' / 'cues.json'
    out.parent.mkdir(parents=True, exist_ok=True)

    for c in cues:
        if c['t'] < 0 or c['t'] > duration:
            notes.append(f'warning: cue {c["type"]!r} at '
                         f't={c["t"]} lies outside '
                         f'[0, {duration}] — kept verbatim '
                         f'(mix.py drops/clips it there, as the '
                         f'Chrome file would)')
    doc = {'duration': duration, 'cues': cues}
    text = json.dumps(doc, indent=1, ensure_ascii=False)
    existing = out.read_text(encoding='utf-8') if out.is_file() \
        else None
    out.write_text(text, encoding='utf-8')
    print(f'data/cues.json: {len(cues)} cues, {duration:.2f} s')
    print(f'cues_native: wrote {out} '
          f'({len(cues)} cues, duration {duration} s'
          f'{", unchanged content" if existing == text else ""})')
    for n in notes:
        print(f'cues_native: {n}')
    if events:
        side = out.parent / 'clip-events.json'
        side.write_text(
            json.dumps({'duration': duration, 'events': events},
                       indent=1, ensure_ascii=False),
            encoding='utf-8')
        names = ', '.join(sorted({e['cue'] for e in events}))
        print(f'cues_native: {len(events)} clip event(s) '
              f'({names}) written to {side} — NOT in cues.json '
              f'(mix.py skips event names). Compile them with: '
              f'python3 tools/audio_recipes.py build <RECIPE> '
              f'{side.name} --cues {out.name} -o {out.name} '
              f'(run in {out.parent})')
        if strict_events:
            print('cues_native: --strict-events: clip events '
                  'present — failing as requested')
            return 3
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
