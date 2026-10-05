#!/usr/bin/env bash
# Usage: packaging/check-glibc.sh <file-or-dir>...
set -euo pipefail

floor=2.28
status=0
checked=0
while IFS= read -r -d '' f; do
  readelf -h "$f" >/dev/null 2>&1 || continue
  checked=$((checked + 1))
  need="$(readelf -VW "$f" | sed -n 's/.*Name: GLIBC_\([0-9.]*\) .*/\1/p' | sort -uV | tail -1)"
  if [ -n "$need" ] && [ "$(printf '%s\n' "$floor" "$need" | sort -V | tail -1)" != "$floor" ]; then
    echo "$f needs GLIBC_$need, above the $floor floor" >&2
    status=1
  fi
done < <(find "$@" -type f -print0)
[ "$checked" -gt 0 ] || { echo "no ELF file under $*" >&2; exit 1; }
exit "$status"