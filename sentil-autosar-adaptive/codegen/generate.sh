#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
MODEL="$HERE/../model"
VENDOR="${SENTIL_AP_VENDOR:-stub}"

if [ "$VENDOR" = "stub" ]; then
  echo "stub: the generic ara::com over vsomeip realizes the interfaces; nothing to generate"
  exit 0
fi

if [ -z "${SENTIL_AP_GENERATOR:-}" ]; then
  echo "set SENTIL_AP_GENERATOR to the vendor ara::com generator for SENTIL_AP_VENDOR=$VENDOR" >&2
  exit 1
fi
"$SENTIL_AP_GENERATOR" --input "$MODEL"/*.arxml --output "$HERE/generated"