"""Compare SENTIL and RTAMT as online STL monitors on the CARLA driving signals.

Both run the same past-time STL specifications over the same recorded signals, one
sample at a time, the way an online monitor sees a live stream. RTAMT's online monitor
supports the past-time fragment (historically, once), so the specs are written there;
that is the fragment where a like-for-like online comparison is meaningful. The script
checks the two agree on the robustness, then reports the per-sample latency of each.

Usage: python rtamt_compare.py --capture <capture_dir or results> --out results/rtamt.json
"""

import argparse
import json
import os
import time

import numpy as np

import rtamt
import sentil

# (id, variable, sentil formula, rtamt formula). RTAMT uses <= / >= and ':' intervals.
SPECS = [
    ("speed_limit", "speed",
     "historically[0,10] (speed < 12.0)", "historically[0:10](speed <= 12.0)"),
    ("following_distance", "front_distance",
     "historically[0,10] (front_distance > 6.0)", "historically[0:10](front_distance >= 6.0)"),
    ("pedestrian_clearance", "pedestrian_distance",
     "historically[0,10] (pedestrian_distance > 5.0)", "historically[0:10](pedestrian_distance >= 5.0)"),
]


def load_signals(path):
    f = path if path.endswith(".json") else os.path.join(path, "capture.json")
    data = json.load(open(f))
    if "records" in data:
        recs = data["records"]
        return [r["t"] for r in recs], {k: [r[k] for r in recs]
                                         for k in ("speed", "front_distance", "pedestrian_distance")}
    sig = data["signals"]
    return sig["t"], sig


def run_sentil(times, values, formula, var):
    # Drive on a discrete step clock so the [0,10] window is 10 steps, matching RTAMT's
    # discrete-time [0:10]; otherwise the two monitors compare different window widths.
    mon = sentil.OnlineMonitor(formula)
    lat, rob = [], []
    for i in range(len(times)):
        s = time.perf_counter()
        v = mon.update(float(i), {var: float(values[i])})
        lat.append(time.perf_counter() - s)
        rob.append(v.value)
    return np.array(lat), np.array(rob)


def run_rtamt(times, values, formula, var):
    spec = rtamt.StlDiscreteTimeOnlineSpecification()
    spec.declare_var(var, "float")
    spec.spec = formula
    spec.parse()
    lat, rob = [], []
    for i in range(len(times)):
        s = time.perf_counter()
        r = spec.update(i, [(var, float(values[i]))])
        lat.append(time.perf_counter() - s)
        rob.append(r)
    return np.array(lat), np.array(rob)


def stats(lat):
    us = lat * 1e6
    return {"median_us": round(float(np.median(us)), 3),
            "p99_us": round(float(np.percentile(us, 99)), 3),
            "mean_us": round(float(np.mean(us)), 3),
            "throughput_hz": round(1.0 / float(np.mean(lat)), 1)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--capture", default="results/drive.json")
    ap.add_argument("--out", default="results/rtamt.json")
    args = ap.parse_args()

    times, sig = load_signals(args.capture)
    report = {"samples": len(times), "specs": {}, "tool": "sentil vs rtamt, online past-time STL"}
    for sid, var, sf, rf in SPECS:
        s_lat, s_rob = run_sentil(times, sig[var], sf, var)
        r_lat, r_rob = run_rtamt(times, sig[var], rf, var)
        # The two robustness signals should agree; RTAMT's online monitor resolves on a
        # delay, so compare where both are defined.
        n = min(len(s_rob), len(r_rob))
        finite = np.isfinite(s_rob[:n]) & np.isfinite(r_rob[:n])
        max_diff = float(np.max(np.abs(s_rob[:n][finite] - r_rob[:n][finite]))) if finite.any() else 0.0
        report["specs"][sid] = {
            "sentil_formula": sf,
            "rtamt_formula": rf,
            "robustness_max_abs_diff": round(max_diff, 6),
            "agree": max_diff < 1e-6,
            "sentil": stats(s_lat),
            "rtamt": stats(r_lat),
            "speedup_median": round(float(np.median(r_lat) / np.median(s_lat)), 1),
        }
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    json.dump(report, open(args.out, "w"), indent=2)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()