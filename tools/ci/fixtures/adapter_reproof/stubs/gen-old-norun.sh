#!/bin/sh
# gen-old-norun — pre-aa10b13 generation, could-not-run outcome: NO verdict line,
# reason on stderr, and the old binary exits 0 even here (adapter header contract).
echo 'film telemetry failed: stub: film data unreadable (simulated pre-aa10b13 could-not-run)' >&2
exit 0
