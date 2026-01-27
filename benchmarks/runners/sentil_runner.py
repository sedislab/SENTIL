import json
import math
import os
import sys
import time

import numpy as np

import sentil

VERSION = "0.3.0"
FORMULA = "always[0, 100](eventually[0, 10](x > 5))"

def summarize(samples):
    samples = sorted(samples)
    n = len(samples)
    mean = sum(samples) / n
    var = sum((s - mean) ** 2 for s in samples) / (n - 1) if n > 1 else 0.0

    def pct(q):
        return samples[round((n - 1) * q)]

    return {
        "mean_ms": mean,
        "std_ms": math.sqrt(var),
        "min_ms": samples[0],
        "p50_ms": pct(0.50),
        "p99_ms": pct(0.99),
    }

def hardware():
    cpu = "unknown"
    try:
        with open("/proc/cpuinfo") as f:
            for line in f:
                if line.startswith("model name"):
                    cpu = line.split(":", 1)[1].strip()
                    break
    except OSError:
        pass
    return {"cpu": cpu, "cores": os.cpu_count() or 1}

def peak_rss_bytes():
    try:
        with open("/proc/self/status") as f:
            for line in f:
                if line.startswith("VmHWM:"):
                    return int(line.split()[1]) * 1024
    except OSError:
        pass
    return None

def emit(benchmark, question, size, robustness, timing, runs):
    record = {
        "tool": "sentil",
        "version": VERSION,
        "language": "python",
        "benchmark": benchmark,
        "formula": FORMULA,
        "question": question,
        "size": size,
        "robustness": robustness,
        "timing": timing,
        "peak_rss_bytes": peak_rss_bytes(),
        "runs": runs,
        "hardware": hardware(),
    }
    print(json.dumps(record))

def oracle_trace(n):
    times = np.arange(n, dtype=np.float64)
    x = 15.0 * np.sin(times * 0.1)
    return sentil.Trace(times, {"x": x})

def scalability():
    for n in (1_000, 10_000, 100_000, 1_000_000, 10_000_000):
        runs = 30 if n <= 100_000 else 5
        trace = oracle_trace(n)
        monitor = sentil.Monitor(FORMULA)

        full = monitor.robustness_signal(trace)[0]
        samples = []
        for _ in range(runs):
            start = time.perf_counter()
            monitor.robustness_signal(trace)
            samples.append((time.perf_counter() - start) * 1e3)
        emit("scalability/length", "full_signal", n, full, summarize(samples), runs)

        mon = monitor.robustness(trace)
        samples = []
        for _ in range(runs):
            start = time.perf_counter()
            monitor.robustness(trace)
            samples.append((time.perf_counter() - start) * 1e3)
        emit("scalability/length", "monitoring", n, mon, summarize(samples), runs)

def streaming():
    monitor = sentil.OnlineMonitor(FORMULA)
    idx = monitor.symbol_index("x")
    n = 1_000_000
    latencies = []
    packed = [0.0]
    last = 0.0
    for i in range(n):
        packed[idx] = 15.0 * math.sin(i * 0.1)
        start = time.perf_counter()
        verdict = monitor.update_packed(float(i), packed)
        latencies.append((time.perf_counter() - start) * 1e3)
        last = verdict.lower
    emit("streaming", "monitoring", n, last, summarize(latencies), n)

def main():
    suite = sys.argv[1] if len(sys.argv) > 1 else ""
    if suite == "scalability":
        scalability()
    elif suite == "streaming":
        streaming()
    else:
        print("unknown suite", file=sys.stderr)
        return 1
    return 0

if __name__ == "__main__":
    sys.exit(main())