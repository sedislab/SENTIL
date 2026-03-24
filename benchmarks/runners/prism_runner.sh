set -uo pipefail
PRISM="${PRISM:-prism}"
model="${1:?usage: prism_runner.sh <path to a .nm model>}"

if ! command -v "$PRISM" >/dev/null 2>&1 && [ ! -x "$PRISM" ]; then
  echo "no prism at $PRISM; skipping" >&2
  exit 0
fi

name=$(basename "$model" .nm)
case "$name" in
  circadian)    mdl=barkai_leibler_ctmc; prop='F<=20 (a>=100)';        pf='P=? [ F<=20 (a>=100) ]' ;;
  tandem)       mdl=tandem_queue;        prop='F<=50 (q1=20 | q2=20)'; pf='P=? [ F<=50 (q1=20 | q2=20) ]' ;;
  *) echo "no property registered for $name" >&2; exit 2 ;;
esac

out=$("$PRISM" "$model" -pf "$pf" -sim -simmethod ci -simsamples 10000 -simpathlen 20000 2>&1)
p=$(printf '%s\n' "$out" | sed -n 's/^Result: \([0-9.][0-9.]*\).*/\1/p')
ci=$(printf '%s\n' "$out" | sed -n 's/.*(+\/- \([0-9.][0-9.]*\).*/\1/p')
secs=$(printf '%s\n' "$out" | sed -n 's/.*iterations in \([0-9.][0-9.]*\) seconds.*/\1/p')

if [ -z "$p" ] || [ -z "$ci" ] || [ -z "$secs" ]; then
  echo "prism ran but gave no estimate for $(basename "$model")" >&2
  exit 1
fi

b=$name; [ "$name" = tandem ] && b=tandem_queue
awk -v p="$p" -v ci="$ci" -v s="$secs" -v b="$b" -v m="$mdl" -v f="$prop" 'BEGIN {
  printf "{\"tool\":\"prism\",\"version\":\"4.9\",\"benchmark\":\"smc/%s\",\"model\":\"%s\",\"property\":\"%s\",\"probability\":%s,\"ci_half_width\":%s,\"samples\":10000,\"time_ms\":%.1f}\n", b, m, f, p, ci, s*1000.0
}'