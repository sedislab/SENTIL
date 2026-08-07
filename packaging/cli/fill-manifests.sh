#!/usr/bin/env bash
# Usage: packaging/cli/fill-manifests.sh <version> <assets-dir> <out-dir>
set -euo pipefail

version="$1"
assets="$2"
out="$3"
mkdir -p "$out"

sha() {
  local file
  file="$(find "$assets" -type f -name "$1" -print -quit)"
  if [ -z "$file" ]; then
    echo "missing archive: $1" >&2
    exit 1
  fi
  sha256sum "$file" | awk '{print $1}'
}

darwin_arm="$(sha "sentil-${version}-aarch64-apple-darwin.tar.gz")"
darwin_x86="$(sha "sentil-${version}-x86_64-apple-darwin.tar.gz")"
linux_x86="$(sha "sentil-${version}-x86_64-unknown-linux-gnu.tar.gz")"
win_x86="$(sha "sentil-${version}-x86_64-pc-windows-msvc.zip")"

bump_urls='s|download/v[0-9.]*/sentil-[0-9.]*-|download/v'"${version}"'/sentil-'"${version}"'-|g'

sed -e "s/^  version .*/  version \"${version}\"/" \
    -e "$bump_urls" \
    -e "s/REPLACE_WITH_AARCH64_DARWIN_SHA256/${darwin_arm}/" \
    -e "s/REPLACE_WITH_X86_64_DARWIN_SHA256/${darwin_x86}/" \
    -e "s/REPLACE_WITH_X86_64_LINUX_SHA256/${linux_x86}/" \
    packaging/cli/homebrew/sentil.rb > "$out/sentil.rb"

sed -e "s/\"version\": \"[0-9.]*\"/\"version\": \"${version}\"/" \
    -e "$bump_urls" \
    -e "s/REPLACE_WITH_WINDOWS_X86_64_SHA256/${win_x86}/" \
    -e "s/sentil-[0-9.]*-x86_64-pc-windows-msvc/sentil-${version}-x86_64-pc-windows-msvc/g" \
    packaging/cli/scoop/sentil.json > "$out/sentil.json"

for manifest in SEDIS.SENTIL.yaml SEDIS.SENTIL.installer.yaml SEDIS.SENTIL.locale.en-US.yaml; do
  sed -e "s/PackageVersion: [0-9.]*/PackageVersion: ${version}/" \
      -e "$bump_urls" \
      -e "s/REPLACE_WITH_WINDOWS_X86_64_SHA256/${win_x86}/" \
      -e "s/sentil-[0-9.]*-x86_64-pc-windows-msvc/sentil-${version}-x86_64-pc-windows-msvc/g" \
      "packaging/cli/winget/${manifest}" > "$out/${manifest}"
done

for manifest in "$out"/SEDIS.SENTIL*.yaml; do
  [ -s "$manifest" ] && [ "$(tail -c1 "$manifest" | wc -l)" -eq 0 ] && printf '\n' >> "$manifest"
done

if grep -rq "REPLACE_WITH" "$out"; then
  echo "a checksum placeholder was left unfilled" >&2
  exit 1
fi
echo "filled manifests for ${version} in ${out}"