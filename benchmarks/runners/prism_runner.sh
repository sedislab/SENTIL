#!/usr/bin/env bash
# PRISM statistical-model-checking baseline on the Barkai-Leibler circadian CTMC, the
# same model and property sentil_ctmc_runner checks, at the same 10,000 samples. Emits
# the shared JSON record. Set PRISM to the prism launcher; pass the path to circadian.nm.
set -uo pipefail
PRISM="${PRISM:-prism}"
model="${1:?usage: prism_runner.sh <path to circadian.nm>}"

out=$("$PRISM" "$model" -pf 'P=? [ F<=20 (a>=100) ]' -sim -simmethod ci -simsamples 10000 -simpathlen 20000 2>&1)
p=$(printf '%s\n' "$out" | sed -n 's/^Result: \([0-9.][0-9.]*\).*/\1/p')
ci=$(printf '%s\n' "$out" | sed -n 's/.*(+\/- \([0-9.][0-9.]*\).*/\1/p')
secs=$(printf '%s\n' "$out" | sed -n 's/.*iterations in \([0-9.][0-9.]*\) seconds.*/\1/p')

awk -v p="$p" -v ci="$ci" -v s="$secs" 'BEGIN {
  printf "{\"tool\":\"prism\",\"version\":\"4.9\",\"benchmark\":\"smc/circadian\",\"model\":\"barkai_leibler_ctmc\",\"property\":\"F<=20 (a>=100)\",\"probability\":%s,\"ci_half_width\":%s,\"samples\":10000,\"time_ms\":%.1f}\n", p, ci, s*1000.0
}'