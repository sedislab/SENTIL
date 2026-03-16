#!/usr/bin/env bash
# UPPAAL-SMC statistical-model-checking baseline. The interval half-width of 0.004 lands
# near the 10,000 samples the other runs use, though UPPAAL picks its own run count.
# Emits the shared JSON record. The tool is not redistributable, so set VERIFYTA to a
# local binary; without one the run skips rather than fails.
set -uo pipefail
VERIFYTA="${VERIFYTA:-verifyta}"
model="${1:?usage: uppaal_runner.sh <path to a .xml model>}"
query="${model%.xml}.q"

if ! command -v "$VERIFYTA" >/dev/null 2>&1 && [ ! -x "$VERIFYTA" ]; then
  echo "no verifyta at $VERIFYTA; skipping" >&2
  exit 0
fi

name=$(basename "$model" .xml)
start=$(date +%s%N)
if ! out=$("$VERIFYTA" -s -q -E 0.004 -a 0.05 "$model" "$query" 2>&1); then
  echo "verifyta failed on $name: $out" >&2
  exit 1
fi
ms=$(( ($(date +%s%N) - start) / 1000000 ))

runs=$(printf '%s\n' "$out" | sed -n 's/.*(\([0-9][0-9]*\) runs).*/\1/p')
lo=$(printf '%s\n' "$out" | sed -n 's/.*) in \[\([-+0-9.eE]*\),.*/\1/p')
hi=$(printf '%s\n' "$out" | sed -n 's/.*) in \[[-+0-9.eE]*,\([-+0-9.eE]*\)\].*/\1/p')

if [ -z "$runs" ] || [ -z "$lo" ] || [ -z "$hi" ]; then
  echo "verifyta ran but gave no interval for $name" >&2
  exit 1
fi

awk -v lo="$lo" -v hi="$hi" -v n="$runs" -v ms="$ms" 'BEGIN {
  printf "{\"tool\":\"uppaal\",\"version\":\"4.1.19\",\"benchmark\":\"smc/%s\",\"model\":\"barkai_leibler_ctmc\",\"property\":\"Pr[<=20] (<> A >= 100)\",\"probability\":%.6f,\"ci_half_width\":%.6f,\"samples\":%s,\"time_ms\":%.1f}\n", "circadian", (lo+hi)/2, (hi-lo)/2, n, ms
}'