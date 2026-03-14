set -uo pipefail
MODEST="${MODEST:-modest}"
model="${1:?usage: modest_runner.sh <path to a .modest model>}"

if ! command -v "$MODEST" >/dev/null 2>&1 && [ ! -x "$MODEST" ]; then
  echo "no modest at $MODEST; skipping" >&2
  exit 0
fi

name=$(basename "$model" .modest)
case "$name" in
  circadian)    mdl=barkai_leibler_ctmc; prop='Pmax(<>[T<=20] (a >= 100))' ;;
  tandem_queue) mdl=tandem_queue;        prop='Pmax(<>[T<=50] (q1 == 20 || q2 == 20))' ;;
  biodiesel)    mdl=biodiesel_reactor;   prop='Pmax(<> reached)' ;;
  powertrain)   mdl=powertrain_afr;      prop='Pmax(<> (time >= 50 && !failed))' ;;
  *) echo "no property registered for $name" >&2; exit 2 ;;
esac

if ! out=$("$MODEST" simulate "$model" -N 10000 --max-run-length 0 --seed 20260304 2>&1); then
  echo "modest failed on $name: $out" >&2
  exit 1
fi

p=$(printf '%s\n' "$out" | sed -n 's/.*Estimated probability: *\([-+0-9.eE][-+0-9.eE]*\).*/\1/p')
ci=$(printf '%s\n' "$out" | sed -n 's/.*Interval half-width: *\([-+0-9.eE][-+0-9.eE]*\).*/\1/p')
secs=$(printf '%s\n' "$out" | sed -n 's/.*Simulation time: *\([-+0-9.eE][-+0-9.eE]*\) s.*/\1/p')

if [ -z "$p" ] || [ -z "$ci" ] || [ -z "$secs" ]; then
  echo "modest ran but gave no estimate for $name" >&2
  exit 1
fi

awk -v p="$p" -v ci="$ci" -v s="$secs" -v b="smc/$name" -v m="$mdl" -v f="$prop" 'BEGIN {
  printf "{\"tool\":\"modest\",\"version\":\"3.1.301\",\"benchmark\":\"%s\",\"model\":\"%s\",\"property\":\"%s\",\"probability\":%s,\"ci_half_width\":%s,\"samples\":10000,\"time_ms\":%.1f}\n", b, m, f, p, ci, s*1000.0
}'