#!/usr/bin/env bash
# Usage: packaging/check-release.sh <tag>
set -euo pipefail

tag="$1"
version="${tag#v}"
work="$(mktemp -d)"
gh release view "$tag" --json assets --jq '.assets[].name' | grep -vx SHA256SUMS | sort > "$work/assets"
gh release download "$tag" --pattern SHA256SUMS --dir "$work"

awk '{print $2}' "$work/SHA256SUMS" | sort | comm -3 - "$work/assets" |
  sed 's/^\t*/SHA256SUMS and the release assets differ on /' > "$work/problems"
awk -F'"' '/sha256 =/ {h = $2} /url =/ {n = split($2, p, "/"); print h "  " p[n]}' sentil-jl/Artifacts.toml |
  awk 'NR == FNR {sums[$0]; next} !($0 in sums) {print "Artifacts.toml and SHA256SUMS differ on " $2}' "$work/SHA256SUMS" - >> "$work/problems"
names='releases/(latest/)?download/[A-Za-z0-9_./*-]+|(lib)?[Ss]entil[A-Za-z0-9_.*-]*[-_]'"${version//./\\.}"'[A-Za-z0-9_.*-]*\.(tar\.gz|zip|deb|rpm|jar|whl)|[A-Za-z]+\.mltbx'
while read -r name; do
  grep -qx "$(sed 's/\./\\./g; s/\*/.*/g' <<< "$name")" "$work/assets" || echo "a README names $name, which the release does not carry" >> "$work/problems"
done < <(git grep -ohE "$names" -- '*.md' ':!website' | sed 's|.*/||' | sort -u)

cat "$work/problems"
[ ! -s "$work/problems" ]