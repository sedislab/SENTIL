"""Banquo runner.

Times Banquo, the cpslab-asu offline signal temporal logic robustness monitor from
the S-TaLiRo group, on the same oracle SENTIL runs and emits the shared JSON record,
one per line. Banquo scores the whole signal, so this is the full-signal track, the
comparison for the discrete chart alongside RTAMT and MoonLight. It needs Python 3.11
or newer (`pip install pybanquo`). Run as `python3.11 banquo_runner.py scalability`.
"""

import json
import os
import math
import platform
import statistics
import sys
import time

import banquo
from banquo import Predicate, Trace, evaluate
from banquo.operators import Always, Eventually

def signals(n):
    return [15.0 * math.sin(0.1 * i) for i in range(n)]

SCALABILITY = "always[0, 100](eventually[0, 10](x > 5))"

# banquo writes predicates as ax <= b, so -x <= -5 is x >= 5
def build():
    inner = Eventually.with_bounds((0.0, 10.0), Predicate({"x": -1.0}, -5.0))
    return Always.with_bounds((0.0, 100.0), inner)

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

def record(benchmark, question, n, robustness, times, runs, peak_rss_bytes=None):
    return {
        "tool": "banquo",
        "version": getattr(banquo, "__version__", "unknown"),
        "language": "python",
        "benchmark": benchmark,
        "formula": SCALABILITY,
        "question": question,
        "size": n,
        "robustness": robustness,
        "timing": times,
        "peak_rss_bytes": peak_rss_bytes,
        "runs": runs,
        "hardware": hardware(),
    }

def measure(n, runs):
    phi = build()
    trace = Trace({float(i): {"x": v} for i, v in enumerate(signals(n))})
    robustness = float(evaluate(phi, trace))
    times_ms = []
    for _ in range(runs):
        start = time.perf_counter()
        evaluate(phi, trace)
        times_ms.append((time.perf_counter() - start) * 1e3)
    times_ms.sort()
    last = len(times_ms) - 1
    times = {
        "mean_ms": statistics.fmean(times_ms),
        "std_ms": statistics.stdev(times_ms) if len(times_ms) > 1 else 0.0,
        "min_ms": times_ms[0],
        "p50_ms": times_ms[round(last * 0.50)],
        "p99_ms": times_ms[round(last * 0.99)],
    }
    return record("scalability/length", "full_signal", n, robustness, times, runs)

def scalability():
    out = []
    for n in (1_000, 10_000, 100_000, 1_000_000):
        runs = 30 if n <= 100_000 else 5
        out.append(measure(n, runs))
    return out

def main():
    suite = sys.argv[1] if len(sys.argv) > 1 else ""
    if suite != "scalability":
        sys.stderr.write("unknown suite; use `scalability`\n")
        return 1
    for record in scalability():
        print(json.dumps(record))
    return 0

if __name__ == "__main__":
    sys.exit(main())