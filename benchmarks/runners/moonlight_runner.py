# MoonLight

import json
import math
import os
import platform
import statistics
import sys
import time

from moonlight import ScriptLoader

SCALABILITY = "globally [0, 100] (eventually [0, 10] (x > 5))"
WARMUP = 3

def signal(n):
    times = [float(i) for i in range(n)]
    values = [[15.0 * math.sin(0.1 * i)] for i in range(n)]
    return times, values

def monitor_for(formula):
    script = f"signal {{ real x; }}\ndomain minmax;\nformula phi = {formula};"
    return ScriptLoader.loadFromText(script).getMonitor("phi")

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

def measure(benchmark, formula, moonlight_formula, n, runs):
    times, values = signal(n)
    mon = monitor_for(moonlight_formula)
    for _ in range(WARMUP):
        mon.monitor(times, values)
    robustness = mon.monitor(times, values)[0][1]
    samples_ms = []
    for _ in range(runs):
        start = time.perf_counter()
        mon.monitor(times, values)
        samples_ms.append((time.perf_counter() - start) * 1e3)
    samples_ms.sort()
    last = len(samples_ms) - 1
    return {
        "tool": "moonlight",
        "version": "0.3.1",
        "language": "java",
        "benchmark": benchmark,
        "formula": formula,
        "question": "full_signal",
        "size": n,
        "robustness": robustness,
        "timing": {
            "mean_ms": statistics.fmean(samples_ms),
            "std_ms": statistics.stdev(samples_ms) if len(samples_ms) > 1 else 0.0,
            "min_ms": samples_ms[0],
            "p50_ms": samples_ms[round(last * 0.50)],
            "p99_ms": samples_ms[round(last * 0.99)],
        },
        "peak_rss_bytes": None,
        "runs": runs,
        "hardware": hardware(),
    }

def scalability():
    formula = "always[0, 100](eventually[0, 10](x > 5))"
    out = []
    for n in (1_000, 10_000, 100_000, 1_000_000):
        runs = 30 if n <= 100_000 else 5
        out.append(measure("scalability/length", formula, SCALABILITY, n, runs))
    return out

def main():
    suite = sys.argv[1] if len(sys.argv) > 1 else ""
    if suite == "scalability":
        records = scalability()
    else:
        sys.stderr.write("unknown suite; use `scalability`\n")
        return 1
    for record in records:
        print(json.dumps(record))
    return 0

if __name__ == "__main__":
    sys.exit(main())