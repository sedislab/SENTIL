# Regression guard for the benchmark and experiment artifacts

import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, ".."))
RESULTS = os.path.join(ROOT, "benchmarks", "results")
EXPERIMENTS = os.path.join(ROOT, "experiments")

failures = []

def load(path):
    with open(path, encoding="utf-8") as handle:
        return [json.loads(line) for line in handle if line.strip()]

def check(name, ok, detail):
    mark = "ok  " if ok else "FAIL"
    print(f"[{mark}] {name}: {detail}")
    if not ok:
        failures.append(name)

def length_sweep(path, question):
    return {
        r["size"]: r["timing"]["mean_ms"]
        for r in load(path)
        if r.get("benchmark") == "scalability/length" and r.get("question") == question
    }

def rtamt_speedup():
    sentil = length_sweep(os.path.join(RESULTS, "sentil_scalability.jsonl"), "full_signal")
    rtamt = length_sweep(os.path.join(RESULTS, "rtamt_scalability.jsonl"), "full_signal")
    ratios = {s: rtamt[s] / sentil[s] for s in sorted(set(sentil) & set(rtamt))}
    worst = min(ratios.values())
    check("rtamt full-signal speedup", worst >= 100, f"at least {worst:.0f}x across {len(ratios)} sizes (target 100x)")

def monitoring_flat():
    mon = length_sweep(os.path.join(RESULTS, "sentil_scalability.jsonl"), "monitoring")
    times = list(mon.values())
    spread = max(times) / min(times)
    check("monitoring cost flat in length", spread <= 3.0, f"spread {spread:.2f}x over {len(times)} sizes (does not grow with length)")

def dense_multiple():
    dense = {r["size"]: r["timing"]["mean_ms"] for r in load(os.path.join(RESULTS, "sentil_dense.jsonl"))}
    disc = length_sweep(os.path.join(RESULTS, "sentil_scalability.jsonl"), "full_signal")
    mults = [dense[s] / disc[s] for s in sorted(set(dense) & set(disc))]
    check("dense within a single-digit multiple of discrete", max(mults) <= 20, f"max {max(mults):.1f}x (dense-time interpolation cost)")

def smc_accuracy():
    records = load(os.path.join(RESULTS, "sentil_smc_cpu.jsonl")) + load(os.path.join(RESULTS, "sentil_smc_gpu.jsonl"))
    errors = [abs(r["probability"] - r["ground_truth"]) for r in records if r.get("ground_truth") is not None]
    check("smc estimate tracks the known probability", max(errors) <= 0.01, f"max error {max(errors):.4f} over {len(errors)} models (target 0.01)")

def synthesis():
    records = load(os.path.join(RESULTS, "sentil_synth.jsonl"))
    open_loop = all(r["holds"] for r in records if r["mode"] == "open_loop")
    misses = sum(r["deadline_misses"] or 0 for r in records if r["mode"] == "receding_horizon")
    check("synthesis reaches its spec and holds its deadline", open_loop and misses == 0, f"open-loop cases hold, {misses} online deadline misses")

def circadian():
    report = json.load(open(os.path.join(EXPERIMENTS, "circadian_gene_network", "results", "circadian.json"), encoding="utf-8"))
    ok = report["ensemble_oscillation"]["fraction"] == 1.0 and 22 <= report["period_h"] <= 26 and report["deterministic"]["oscillation_peaks"]["robustness"] > 0
    check("circadian network oscillates", ok, f"period {report['period_h']} h, {report['ensemble_oscillation']['holds']}/{report['ensemble_oscillation']['of']} realizations")

def glucose():
    report = json.load(open(os.path.join(EXPERIMENTS, "glucose_control", "results", "glucose.json"), encoding="utf-8"))
    missed = report["controllers"]["missed_lunch_bolus"]["specifications"]["euglycemia"]["robustness"]
    tuned = report["controllers"]["tuned"]["specifications"]["euglycemia"]["robustness"]
    check("glucose euglycemia separates the controllers", missed < 0 < tuned, f"missed-bolus {missed}, tuned {tuned}")

def main():
    for stage in (rtamt_speedup, monitoring_flat, dense_multiple, smc_accuracy, synthesis, circadian, glucose):
        try:
            stage()
        except (FileNotFoundError, KeyError) as err:
            check(stage.__name__, False, f"artifact missing or malformed: {err}")
    if failures:
        print(f"\n{len(failures)} claim(s) out of tolerance: {', '.join(failures)}")
        sys.exit(1)
    print("\nall artifact claims within tolerance")

if __name__ == "__main__":
    main()