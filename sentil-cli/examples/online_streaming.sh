#!/usr/bin/env bash
# Online streaming: feed one JSON sample per line into the monitor, which emits a verdict per sample.
set -uo pipefail
SENTIL="${SENTIL:-sentil}"

printf '{"time":0,"x":5}\n{"time":1,"x":-2}\n{"time":2,"x":3}\n' \
  | "$SENTIL" monitor -f 'always[0, 1] (x > 0)' -o ndjson