#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
BUILD="${BUILD:-$HERE/../../build}"
export VSOMEIP_CONFIGURATION="$HERE/vsomeip/demo.json"

cleanup() { kill $(jobs -p) 2>/dev/null || true; }
trap cleanup EXIT

VSOMEIP_APPLICATION_NAME=sentil_monitor "$BUILD/sentil_monitor" &
sleep 1
VSOMEIP_APPLICATION_NAME=planner_subscriber "$BUILD/planner_subscriber" &
sleep 1
VSOMEIP_APPLICATION_NAME=perception_publisher "$BUILD/perception_publisher" &
wait