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
    check("dense within about 12x of discrete", max(mults) <= 13, f"max {max(mults):.1f}x (dense-time interpolation cost)")

def ledger_tables_from_doc():
    tables = {}
    header = None
    with open(os.path.join(ROOT, "docs", "CLAIMS.md"), encoding="utf-8") as handle:
        for line in handle:
            if not line.startswith("|"):
                header = None
                continue
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            if cells[0] == "samples":
                header = tuple(cells[1:])
                tables.setdefault(header, {})
            elif header and cells[0].replace(",", "").isdigit():
                tables[header][int(cells[0].replace(",", ""))] = cells[1:]
    return tables

def printed(cell):
    text = cell.strip()
    text = text[:-2].strip() if text.endswith(" ms") else text[:-1] if text.endswith("x") else None
    if text is None:
        return None
    try:
        value = float(text)
    except ValueError:
        return None
    decimals = len(text.split(".")[1]) if "." in text else 0
    return value, 0.5 * 10 ** -decimals

def ledger_tables():
    doc = ledger_tables_from_doc()
    full = length_sweep(os.path.join(RESULTS, "sentil_scalability.jsonl"), "full_signal")
    rtamt = length_sweep(os.path.join(RESULTS, "rtamt_scalability.jsonl"), "full_signal")
    mon = length_sweep(os.path.join(RESULTS, "sentil_scalability.jsonl"), "monitoring")
    dense = {r["size"]: r["timing"]["mean_ms"] for r in load(os.path.join(RESULTS, "sentil_dense.jsonl"))}

    expected = {
        ("SENTIL", "RTAMT", "speedup"):
            lambda n: [full[n], rtamt[n], rtamt[n] / full[n]],
        ("monitoring", "full signal"):
            lambda n: [mon[n], full[n]],
        ("dense full signal", "discrete full signal"):
            lambda n: [dense[n], full[n]],
    }

    stale = []
    checked = 0
    for header, rows in doc.items():
        build = expected.get(header)
        if build is None:
            continue
        for size, cells in sorted(rows.items()):
            try:
                actuals = build(size)
            except KeyError:
                continue
            parsed = [printed(c) for c in cells]
            if any(p is None for p in parsed):
                continue
            for cell, (value, tolerance), actual in zip(cells, parsed, actuals):
                checked += 1
                if abs(value - actual) > tolerance:
                    stale.append(f"{size}: printed {cell}, artifact {actual:.4g}")
    check(
        "the ledger's tables match the artifacts they cite",
        not stale,
        f"{checked} printed values agree" if not stale else "; ".join(stale),
    )

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

def particle_convergence():
    records = load(os.path.join(RESULTS, "sentil_particles.jsonl"))
    by_event = {}
    for r in records:
        by_event.setdefault(r["event"], {}).setdefault(r["particles"], []).append(r["rel_error"])

    def mean_err(by_count, count):
        errs = by_count[count]
        return sum(errs) / len(errs)

    worst_top = 0.0
    decreases = True
    for by_count in by_event.values():
        top, bottom = max(by_count), min(by_count)
        worst_top = max(worst_top, mean_err(by_count, top))
        decreases = decreases and mean_err(by_count, top) < mean_err(by_count, bottom)
    check("rare-event estimate sharpens with particles", worst_top <= 0.15 and decreases,
          f"error at the largest count is {worst_top:.3f} (target 0.15) and below the smallest-count error")

def glucose():
    report = json.load(open(os.path.join(EXPERIMENTS, "glucose_control", "results", "glucose.json"), encoding="utf-8"))
    missed = report["controllers"]["missed_lunch_bolus"]["specifications"]["euglycemia"]["robustness"]
    tuned = report["controllers"]["tuned"]["specifications"]["euglycemia"]["robustness"]
    # the missed bolus violates euglycemia badly while the tuned controller holds it
    ok = missed < -50.0 < 0 < tuned and abs(missed - (-105.0)) <= 6.0 and abs(tuned - 8.0) <= 2.5
    check("glucose euglycemia separates the controllers", ok, f"missed-bolus {missed}, tuned {tuned}")

def main():
    for stage in (rtamt_speedup, monitoring_flat, dense_multiple, smc_accuracy, synthesis, particle_convergence, circadian, glucose):
        try:
            stage()
        except (FileNotFoundError, KeyError, ValueError, ZeroDivisionError, TypeError) as err:
            check(stage.__name__, False, f"artifact missing or malformed: {err}")
    if failures:
        print(f"\n{len(failures)} claim(s) out of tolerance: {', '.join(failures)}")
        sys.exit(1)
    print("\nall artifact claims within tolerance")

if __name__ == "__main__":
    main()