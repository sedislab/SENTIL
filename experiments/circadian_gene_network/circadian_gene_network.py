"""Case study on a mammalian circadian gene-regulatory network."""

import csv
import json
import os

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

import sentil

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = os.path.join(HERE, "results")

HIGH = 3000.0
LOW = 2000.0
PERIOD = 24
HORIZON = 240
MEAS_SIGMA = 250.0

SPECS = {
    "oscillation_peaks": f"always[0,{HORIZON}] (eventually[0,{PERIOD}] (activator > {HIGH}))",
    "oscillation_troughs": f"always[0,{HORIZON}] (eventually[0,{PERIOD}] (activator < {LOW}))",
    "within_capacity": "always (activator < 7000)",
}


def load():
    with open(os.path.join(HERE, "circadian_traces.csv"), encoding="utf-8") as handle:
        rows = list(csv.reader(handle))
    header = rows[0]
    cols = list(zip(*[[float(x) for x in r] for r in rows[1:]]))
    times = list(cols[0])
    mean = list(cols[1])
    traces = [list(cols[i]) for i in range(2, len(header))]
    return times, mean, traces


def robustness_on(times, values):
    trace = sentil.Trace(times, {"activator": values})
    out = {}
    for name, text in SPECS.items():
        rho = sentil.parse(text).robustness(trace)
        out[name] = {"formula": text, "robustness": round(rho, 4), "satisfied": rho > 0}
    return out


def period_of(times, mean):
    peak = max(mean)
    times_at_peaks = [
        times[i]
        for i in range(1, len(mean) - 1)
        if mean[i] > mean[i - 1] and mean[i] >= mean[i + 1] and mean[i] > 0.7 * peak
    ]
    gaps = [b - a for a, b in zip(times_at_peaks, times_at_peaks[1:])]
    return float(np.mean(gaps)) if gaps else float("nan")


def ensemble_probability(times, traces):
    peaks = sentil.parse(SPECS["oscillation_peaks"])
    troughs = sentil.parse(SPECS["oscillation_troughs"])
    holds = 0
    for values in traces:
        trace = sentil.Trace(times, {"activator": values})
        if peaks.robustness(trace) > 0 and troughs.robustness(trace) > 0:
            holds += 1
    return holds, len(traces)


def probabilistic_under_noise(times, mean):
    trace = sentil.Trace(times, {"activator": mean})
    lifting = sentil.LiftingRegistry()
    lifting.register(
        "activator", sentil.NoiseModel.gaussian(0.0, MEAS_SIGMA), sentil.NoiseInteraction.Additive
    )
    config = sentil.SmcConfig(samples=4000, seed=7)
    prstl = sentil.parse(f"P>=0.9(always[0,{HORIZON}] (eventually[0,{PERIOD}] (activator > {HIGH})))")
    result = prstl.check(trace, lifting, config)
    return {
        "formula": f"P>=0.9(always[0,{HORIZON}] (eventually[0,{PERIOD}] (activator > {HIGH})))",
        "probability": round(result.probability, 4),
        "confidence_interval": [round(result.interval.lower, 4), round(result.interval.upper, 4)],
        "holds": result.holds,
        "measurement_sigma": MEAS_SIGMA,
    }


def plot(times, mean, traces):
    fig, ax = plt.subplots(figsize=(11, 5))
    hours = np.array(times)
    for values in traces[:30]:
        ax.plot(hours, values, color="#95a5a6", lw=0.4, alpha=0.4)
    ax.plot(hours, mean, color="#2c3e50", lw=2.0, label="ensemble mean")
    ax.axhline(HIGH, color="#27ae60", ls="--", lw=1, label=f"peak threshold ({HIGH:.0f})")
    ax.axhline(LOW, color="#c0392b", ls="--", lw=1, label=f"trough threshold ({LOW:.0f})")
    ax.set_xlabel("time (hours)")
    ax.set_ylabel("activator protein count")
    ax.set_title("Circadian network: SENTIL verifies the oscillation persists")
    ax.legend(loc="upper right", fontsize=8)
    fig.tight_layout()
    path = os.path.join(RESULTS, "circadian.png")
    fig.savefig(path, dpi=130)
    return path


def main():
    os.makedirs(RESULTS, exist_ok=True)
    times, mean, traces = load()
    deterministic = robustness_on(times, mean)
    holds, total = ensemble_probability(times, traces)
    report = {
        "case_study": "circadian_gene_regulatory_network",
        "model": "Barkai-Leibler activator-repressor circadian oscillator",
        "traces": total,
        "duration_h": times[-1],
        "amplitude": {"min": round(min(mean), 1), "max": round(max(mean), 1)},
        "period_h": round(period_of(times, mean), 2),
        "deterministic": deterministic,
        "ensemble_oscillation": {
            "spec": "peaks and troughs both recur every period",
            "holds": holds,
            "of": total,
            "fraction": round(holds / total, 4),
        },
        "probabilistic_under_noise": probabilistic_under_noise(times, mean),
    }
    with open(os.path.join(RESULTS, "circadian.json"), "w") as f:
        json.dump(report, f, indent=2)
    plot(times, mean, traces)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()