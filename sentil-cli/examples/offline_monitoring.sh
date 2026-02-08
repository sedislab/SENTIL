#!/usr/bin/env bash
# Offline robustness over a recorded trace, in discrete and dense time.
set -uo pipefail
SENTIL="${SENTIL:-sentil}"
trace="${TMPDIR:-/tmp}/sentil_offline_$$.csv"
printf 'time,speed\n0,12\n1,9\n2,7\n3,4\n4,6\n' > "$trace"
trap 'rm -f "$trace"' EXIT

"$SENTIL" check -f 'always (speed > 5)' -t "$trace" --semantics discrete
"$SENTIL" check -f 'always (speed > 5)' -t "$trace" --semantics dense