#!/usr/bin/env python3
"""Field-by-field comparison for the M6 telemetry agreement proof.

Joins the Chrome-side cutout payloads (the real page via drive.mjs;
the real project modules via the harness) with the native
telemetry payloads and prints the agreement table used in
docs/ab/telemetry/README.md. Usage:
  python3 compare.py <telemetry-dir>
"""
import json
import sys

d = sys.argv[1] if len(sys.argv) > 1 else '.'


def load(name):
    with open(f'{d}/{name}') as f:
        return json.load(f)


def staged(items):
    """The staged onoda: the entry that carries a box."""
    return next(i for i in items if i.get('box'))


chrome_page = staged(load('chrome-cutouts.json'))
chrome_base = staged(load('base-cutouts.json'))
chrome_variant = staged(load('variant-cutouts.json'))
native_hand = load('native-ab1.json')[0]
native_file = load('native-ab1-file.json')[0]
native_variant = load('native-ab1-variant.json')[0]

FIELDS = ['id', 'desc', 'pose', 'overflow', 'box', 'sig']


def row(field, *vals):
    agree = all(v == vals[0] for v in vals)
    print(f"{field:9} {'AGREE' if agree else 'DIFFER':6} " +
          ' | '.join(str(v)[:72] for v in vals))


print('== BASE (S01 @ t=1.50): chrome page | chrome harness | native hand | native file ==')
for f in FIELDS:
    row(f, chrome_page[f], chrome_base[f], native_hand[f], native_file[f])
print()
print('== VARIANT (lean 42, group (1.9,-0.55,0.6)): chrome harness | native ==')
for f in FIELDS:
    row(f, chrome_variant[f], native_variant[f])
