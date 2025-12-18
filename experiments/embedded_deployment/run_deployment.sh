#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$here/../.."
cd "$root"

cargo build --release -p sentil-embedded-deployment
exec ./target/release/embedded_deployment \
  --output experiments/embedded_deployment/results/embedded.json \
  "$@"