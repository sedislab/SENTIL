#!/usr/bin/env bash
# Usage: packaging/cli/stage.sh <triple> <version> <bin-path> [tar.gz|zip]
set -euo pipefail

triple="$1"
version="$2"
bin="$3"
fmt="${4:-tar.gz}"

root="sentil-${version}-${triple}"
rm -rf "$root"
mkdir -p "$root/bin" "$root/completions" "$root/man"

cp "$bin" "$root/bin/"

# completions and the man page are architecture-free, so any target's build dir will do.
outdir="$(find target -type d -path '*/build/sentil-cli-*/out' 2>/dev/null | head -1)"
if [ -n "$outdir" ]; then
  for f in sentil.bash _sentil sentil.fish _sentil.ps1; do
    [ -f "$outdir/$f" ] && cp "$outdir/$f" "$root/completions/"
  done
  [ -f "$outdir/sentil.1" ] && cp "$outdir/sentil.1" "$root/man/"
fi

cp LICENSE-MIT LICENSE-APACHE "$root/" 2>/dev/null || true
cp sentil-cli/README.md "$root/" 2>/dev/null || true

mkdir -p assets
if [ "$fmt" = "zip" ]; then
  (cd "$(dirname "$root")" && zip -r "${OLDPWD}/assets/${root}.zip" "$(basename "$root")" >/dev/null)
  echo "assets/${root}.zip"
else
  tar czf "assets/${root}.tar.gz" "$root"
  echo "assets/${root}.tar.gz"
fi