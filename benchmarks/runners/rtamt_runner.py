"""RTAMT runner.

Times RTAMT on the same oracle SENTIL runs and emits the same JSON record, one
per line. The discrete-time offline monitor computes the whole signal, so the
`deterministic` and `scalability` suites are the full-signal track; the `dense`
suite times RTAMT's dense-time offline monitor over the interpolated signal and
reports the monitoring answer, the comparison for the dense chart alongside
Breach. Run as `python rtamt_runner.py <deterministic|scalability|dense>`.
"""

import json
import math
import os
import platform
import statistics
import sys
import time

import rtamt

def signals(n):
    x = [15.0 * math.sin(0.1 * i) for i in range(n)]
    p = [1.0 if (i // 10) % 2 == 0 else -1.0 for i in range(n)]
    q = [1.0] * n
    t = list(range(n))
    return t, x, p, q

CANONICAL = [
    "always[0:10](x < 5)",
    "eventually[0:50](x > 10)",
    "always[0:100](eventually[0:10](p > 0))",
    "(p > 0) implies (eventually[0:20](q > 0))",
    "always[0:200]((p > 0) and (eventually[5:15](q > 0)))",
]
SCALABILITY = "always[0:100](eventually[0:10](x > 5))"

def build(formula):
    spec = rtamt.StlDiscreteTimeOfflineSpecification()
    for name in ("x", "p", "q"):
        spec.declare_var(name, "float")
    spec.declare_var("out", "float")
    spec.spec = f"out = {formula}"
    spec.parse()
    return spec

def evaluate(formula, dataset):
    spec = build(formula)
    return spec.evaluate(dataset)[0][1]

def build_dense(formula):
    spec = rtamt.StlDenseTimeOfflineSpecification()
    spec.declare_var("x", "float")
    spec.declare_var("out", "float")
    spec.spec = f"out = {formula}"
    spec.parse()
    return spec

def evaluate_dense(formula, xs):
    spec = build_dense(formula)
    return spec.evaluate(["x", xs])[0][1]

def hardware():
    cpu = platform.processor() or "unknown"
    try:
        with open("/proc/cpuinfo", encoding="utf-8") as handle:
            for line in handle:
                if line.startswith("model name"):
                    cpu = line.split(":", 1)[1].strip()
                    break
    except OSError:
        pass
    return {"cpu": cpu, "cores": os.cpu_count() or 1}

def timing(run, runs):
    times_ms = []
    for _ in range(runs):
        start = time.perf_counter()
        run()
        times_ms.append((time.perf_counter() - start) * 1e3)
    times_ms.sort()
    last = len(times_ms) - 1
    return {
        "mean_ms": statistics.fmean(times_ms),
        "std_ms": statistics.stdev(times_ms) if len(times_ms) > 1 else 0.0,
        "min_ms": times_ms[0],
        "p50_ms": times_ms[round(last * 0.50)],
        "p99_ms": times_ms[round(last * 0.99)],
    }

def record(benchmark, formula, question, n, robustness, times, runs, peak_rss_bytes=None):
    return {
        "tool": "rtamt",
        "version": getattr(rtamt, "__version__", "0.3.5"),
        "language": "python",
        "benchmark": benchmark,
        "formula": formula.replace(":", ", "),
        "question": question,
        "size": n,
        "robustness": robustness,
        "timing": times,
        "peak_rss_bytes": peak_rss_bytes,
        "runs": runs,
        "hardware": hardware(),
    }

def measure(benchmark, formula, n, runs):
    t, x, p, q = signals(n)
    dataset = {"time": t, "x": x, "p": p, "q": q}
    robustness = evaluate(formula, dataset)
    times = timing(lambda: evaluate(formula, dataset), runs)
    return record(benchmark, formula, "full_signal", n, robustness, times, runs)

def measure_dense(formula, n, runs):
    t, x, _, _ = signals(n)
    xs = [[float(t[i]), x[i]] for i in range(n)]
    robustness = evaluate_dense(formula, xs)
    times = timing(lambda: evaluate_dense(formula, xs), runs)
    return record("scalability/length", formula, "monitoring", n, robustness, times, runs)

def deterministic():
    return [measure("deterministic", f, 2001, 50) for f in CANONICAL]

def scalability():
    out = []
    for n in (1_000, 10_000, 100_000, 1_000_000):
        runs = 30 if n <= 100_000 else 5
        out.append(measure("scalability/length", SCALABILITY, n, runs))
    return out

def dense():
    out = []
    for n in (1_000, 10_000, 100_000, 1_000_000):
        runs = 20 if n <= 100_000 else 3
        out.append(measure_dense(SCALABILITY, n, runs))
    return out

def main():
    suite = sys.argv[1] if len(sys.argv) > 1 else ""
    if suite == "deterministic":
        records = deterministic()
    elif suite == "scalability":
        records = scalability()
    elif suite == "dense":
        records = dense()
    else:
        sys.stderr.write("unknown suite; use `deterministic`, `scalability`, or `dense`\n")
        return 1
    for rec in records:
        print(json.dumps(rec))
    return 0

if __name__ == "__main__":
    sys.exit(main())