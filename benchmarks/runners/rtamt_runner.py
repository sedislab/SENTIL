"""RTAMT benchmark runner.

Times RTAMT's discrete-time offline monitor on the same oracle SENTIL runs and
emits the same JSON record, one per line, so the two line up directly. RTAMT
computes the whole robustness signal, so this is the full-signal track, the
like-for-like comparison against SENTIL's robustness_signal.

Run as `python rtamt_runner.py <suite>`, where <suite> is `deterministic` or
`scalability`. RTAMT writes the robustness it computed into each record, so a
run also confirms both tools agree on the value before any timing is read.
"""

import json
import math
import platform
import statistics
import sys
import time

import rtamt

# The oracle, matching benchmarks/src/oracle.rs: x is a scaled sine, p flips
# every ten samples, q is a fixed sign sequence. RTAMT does not depend on q for
# the value-matching formulas, so a constant q keeps the runner free of a
# generator that would have to match Rust's bit for bit.
def signals(n):
    x = [15.0 * math.sin(0.1 * i) for i in range(n)]
    p = [1.0 if (i // 10) % 2 == 0 else -1.0 for i in range(n)]
    q = [1.0] * n
    t = list(range(n))
    return t, x, p, q


# RTAMT writes intervals with a colon; SENTIL writes a comma. Same operator.
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
    # RTAMT consumes the spec on evaluate, so build a fresh one each run.
    spec = build(formula)
    trace = spec.evaluate(dataset)
    return trace[0][1]


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
    import os

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


def deterministic():
    return [measure("deterministic", f, 2001, 50) for f in CANONICAL]


def scalability():
    out = []
    for n in (1_000, 10_000, 100_000, 1_000_000):
        runs = 30 if n <= 100_000 else 5
        out.append(measure("scalability/length", SCALABILITY, n, runs))
    return out


def main():
    suite = sys.argv[1] if len(sys.argv) > 1 else ""
    if suite == "deterministic":
        records = deterministic()
    elif suite == "scalability":
        records = scalability()
    else:
        sys.stderr.write("unknown suite; use `deterministic` or `scalability`\n")
        return 1
    for rec in records:
        print(json.dumps(rec))
    return 0


if __name__ == "__main__":
    sys.exit(main())