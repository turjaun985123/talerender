#!/bin/sh
# gen-old-fail — pre-aa10b13 generation, CHECK FAILED outcome: verdict line, STILL exit 0.
printf '%s\n' 'FILM TELEMETRY: CHECK FAILED (0 overflow groups, 0 continuity violations, 7 caption failures)'
exit 0
