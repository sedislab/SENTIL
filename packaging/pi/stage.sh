#!/usr/bin/env bash
# Usage: packaging/pi/stage.sh <triple> <kind: full|static> [version]
set -euo pipefail

triple="${1:?need a target triple}"
kind="${2:?need a kind: full or static}"
version="${3:-$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)}"

out="target/${triple}/release"
root="sentil-${version}-${triple}"
dest="assets/${root}"
rm -rf "$dest"
mkdir -p "$dest/bin" "$dest/completions"

cp "$out/sentil" "$dest/bin/"

# completions and the man page are architecture-free, so any target's build dir will do.
gen="$(find target -type d -path '*/build/sentil-cli-*/out' 2>/dev/null | head -1)"
if [ -n "$gen" ]; then
  for f in sentil.bash _sentil sentil.fish _sentil.ps1; do
    [ -f "$gen/$f" ] && cp "$gen/$f" "$dest/completions/"
  done
  if [ -f "$gen/sentil.1" ]; then
    mkdir -p "$dest/man"
    cp "$gen/sentil.1" "$dest/man/"
  fi
fi

if [ "$kind" = "full" ]; then
  mkdir -p "$dest/lib/pkgconfig" "$dest/lib/cmake/Sentil" "$dest/include"
  cp "$out/libsentil.so" "$dest/lib/"
  cp "$out/libsentil.a" "$dest/lib/"
  cp sentil-ffi/include/sentil.h "$dest/include/"
  sed 's|@PREFIX@|/usr/local|' sentil-ffi/sentil.pc.in > "$dest/lib/pkgconfig/sentil.pc"
  cp sentil-ffi/cmake/SentilConfig.cmake.in "$dest/lib/cmake/Sentil/SentilConfig.cmake"
fi

cp LICENSE-MIT LICENSE-APACHE "$dest/"
cp packaging/pi/INSTALL.md "$dest/"

tar -C assets -czf "assets/${root}.tar.gz" "$root"
echo "staged assets/${root}.tar.gz"