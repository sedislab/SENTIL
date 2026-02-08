#!/usr/bin/env bash
# Synthesize a control-input sequence that satisfies a spec on a linear model.
# Set SENTIL to a built binary if `sentil` is not on your PATH.
set -uo pipefail
SENTIL="${SENTIL:-sentil}"
model="${TMPDIR:-/tmp}/sentil_model_$$.json"
printf '%s' '{"a":[[1.0]],"b":[[1.0]],"x0":[0.5],"variables":["x"],"dt":1.0,"horizon":3,"bounds":{"lower":[-1.0],"upper":[1.0]}}' > "$model"
trap 'rm -f "$model"' EXIT

"$SENTIL" synth -f 'always (x > 0)' --model "$model" --method gradient