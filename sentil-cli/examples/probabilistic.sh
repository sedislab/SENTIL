#!/usr/bin/env bash
# Probabilistic monitoring. Lift a noisy sensor into an ensemble and estimate the satisfaction probability.
set -uo pipefail
SENTIL="${SENTIL:-sentil}"
trace="${TMPDIR:-/tmp}/sentil_prstl_$$.csv"
printf 'time,x\n' > "$trace"
for i in $(seq 0 19); do
  printf '%d,%s\n' "$i" "$(awk "BEGIN{print 0.4 + 0.05*$i}")" >> "$trace"
done
trap 'rm -f "$trace"' EXIT

"$SENTIL" smc -f 'P>=0.9(always (x > 0))' -t "$trace" --noise 'x=gaussian:0,0.3' --samples 5000