#!/usr/bin/env sh
# The figures live in benchmarks/results; they stay gitignored here to avoid a second copy.
set -eu
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
src="$here/../benchmarks/results"
dst="$here/figures"
for f in smc_circadian smc_tandem_queue smc_biodiesel smc_powertrain \
         smc_accuracy smc_throughput rare_event rare_event_3tandem rare_event_gpu; do
  cp "$src/$f.png" "$dst/$f.png"
done