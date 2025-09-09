"""Plot the benchmark results.

Reads every record under benchmarks/results, groups by the question a
measurement answers, and writes the figures next to the JSON. Run as
`python plot.py` from anywhere.
"""

import json
import os

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = os.path.normpath(os.path.join(HERE, "..", "results"))

def load():
    records = []
    for name in sorted(os.listdir(RESULTS)):
        if name.endswith(".jsonl"):
            with open(os.path.join(RESULTS, name), encoding="utf-8") as handle:
                records.extend(json.loads(line) for line in handle if line.strip())
    return records

def scalability(records):
    rows = [r for r in records if r.get("benchmark") == "scalability/length"]
    if not rows:
        return
    fig, ax = plt.subplots(figsize=(7, 5))
    series = {}
    for r in rows:
        series.setdefault((r["tool"], r["question"]), []).append(
            (r["size"], r["timing"]["mean_ms"])
        )
    for (tool, question), points in sorted(series.items()):
        points.sort()
        ax.plot(
            [p[0] for p in points],
            [p[1] for p in points],
            marker="o",
            label=f"{tool} ({question})",
        )
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel("trace length (samples)")
    ax.set_ylabel("time per evaluation (ms)")
    ax.set_title("Robustness cost versus trace length")
    ax.grid(True, which="both", linewidth=0.3)
    ax.legend()
    fig.tight_layout()
    out = os.path.join(RESULTS, "scalability.png")
    fig.savefig(out, dpi=150)
    plt.close(fig)
    print("wrote", out)

def deterministic(records):
    rows = [
        r
        for r in records
        if r.get("benchmark") == "deterministic" and r.get("question") == "full_signal"
    ]
    if not rows:
        return
    formulas = sorted({r["formula"] for r in rows})
    tools = sorted({r["tool"] for r in rows})
    fig, ax = plt.subplots(figsize=(9, 5))
    width = 0.8 / max(len(tools), 1)
    for i, tool in enumerate(tools):
        by_formula = {r["formula"]: r["timing"]["mean_ms"] for r in rows if r["tool"] == tool}
        xs = [j + i * width for j in range(len(formulas))]
        ax.bar(xs, [by_formula.get(f, 0.0) for f in formulas], width=width, label=tool)
    ax.set_yscale("log")
    ax.set_xticks([j + width * (len(tools) - 1) / 2 for j in range(len(formulas))])
    ax.set_xticklabels([f"phi{j + 1}" for j in range(len(formulas))])
    ax.set_ylabel("time per evaluation (ms)")
    ax.set_title("Full-signal robustness on the oracle formulas")
    ax.grid(True, axis="y", which="both", linewidth=0.3)
    ax.legend()
    fig.tight_layout()
    out = os.path.join(RESULTS, "deterministic.png")
    fig.savefig(out, dpi=150)
    plt.close(fig)
    print("wrote", out)

THROUGHPUT_MODELS = ("single_step", "always_ten", "eventually_ten")

def smc_throughput(records):
    rows = [r for r in records if "device" in r and r["model"] in THROUGHPUT_MODELS]
    if not rows:
        return
    fig, ax = plt.subplots(figsize=(7, 5))
    series = {}
    for r in rows:
        series.setdefault((r["device"], r["model"]), []).append(
            (r["samples"], r["throughput_per_s"] / 1e6)
        )
    for (device, model), points in sorted(series.items()):
        points.sort()
        ax.plot(
            [p[0] for p in points],
            [p[1] for p in points],
            marker="o",
            label=f"{device}: {model}",
        )
    ax.set_xscale("log")
    ax.set_xlabel("samples")
    ax.set_ylabel("throughput (million realization-steps/s)")
    ax.set_title("Statistical model checking throughput, CPU versus GPU")
    ax.grid(True, which="both", linewidth=0.3)
    ax.legend()
    fig.tight_layout()
    out = os.path.join(RESULTS, "smc_throughput.png")
    fig.savefig(out, dpi=150)
    plt.close(fig)
    print("wrote", out)

def smc_accuracy(records):
    rows = [
        r
        for r in records
        if "device" in r and r.get("ground_truth") is not None and r["samples"] <= 100_000
    ]
    if not rows:
        return
    fig, ax = plt.subplots(figsize=(6, 6))
    ax.plot([0, 1], [0, 1], linewidth=0.8, color="gray")
    ax.scatter([r["ground_truth"] for r in rows], [r["probability"] for r in rows], s=20)
    ax.set_xlabel("known probability")
    ax.set_ylabel("estimated probability")
    ax.set_title("Estimate versus closed-form truth")
    ax.grid(True, linewidth=0.3)
    fig.tight_layout()
    out = os.path.join(RESULTS, "smc_accuracy.png")
    fig.savefig(out, dpi=150)
    plt.close(fig)
    print("wrote", out)

def main():
    records = load()
    if not records:
        print("no results found under", RESULTS)
        return
    scalability(records)
    deterministic(records)
    smc_throughput(records)
    smc_accuracy(records)

if __name__ == "__main__":
    main()