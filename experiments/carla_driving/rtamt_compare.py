"""Compare SENTIL and RTAMT on the CARLA driving workload."""

import argparse
import json
import os
import time

import numpy as np

import rtamt
import sentil

HERE = os.path.dirname(os.path.abspath(__file__))

DET = [
    ("speed_limit", "speed", 12.0, "<", "<="),
    ("following_distance", "vehicle_distance", 6.0, ">", ">="),
    ("pedestrian_clearance", "pedestrian_distance", 5.0, ">", ">="),
]


def load_signals(path):
    f = path if path.endswith(".json") else os.path.join(path, "capture.json")
    data = json.load(open(f))
    recs = data["records"] if "records" in data else None
    keys = ("speed", "vehicle_distance", "pedestrian_distance")
    if recs is not None:
        return [r["t"] for r in recs], {k: [r[k] for r in recs] for k in keys}
    sig = data["signals"]
    return sig["t"], {k: sig[k] for k in keys}


def percentiles(lat):
    us = np.array(lat) * 1e6
    return {"median_us": round(float(np.median(us)), 3),
            "p99_us": round(float(np.percentile(us, 99)), 3),
            "mean_us": round(float(np.mean(us)), 3)}


def online_multiformula(times, sig, n):
    """All deterministic formulas, one sample at a time."""
    mon = sentil.MultiMonitor()
    for sid, var, thr, op, _ in DET:
        mon.add(sid, "historically[0,10] (%s %s %.1f)" % (var, op, thr))
    rt = []
    for sid, var, thr, _, rop in DET:
        s = rtamt.StlDiscreteTimeOnlineSpecification()
        s.declare_var(var, "float")
        s.spec = "historically[0:10](%s %s %.1f)" % (var, rop, thr)
        s.parse()
        rt.append((var, s))

    s_lat, r_lat = [], []
    s_last, r_last = {}, {}
    for i in range(n):
        sample = {var: float(sig[var][i]) for _, var, _, _, _ in DET}
        t0 = time.perf_counter()
        s_last = mon.update(float(i), sample)
        s_lat.append(time.perf_counter() - t0)
        t0 = time.perf_counter()
        for var, s in rt:
            r_last[var] = s.update(i, [(var, float(sig[var][i]))])
        r_lat.append(time.perf_counter() - t0)
    return {
        "formulas": len(DET),
        "sentil": percentiles(s_lat),
        "rtamt": percentiles(r_lat),
        "speedup_median": round(float(np.median(r_lat) / np.median(s_lat)), 1),
    }


def offline_future(times, sig, n):
    """The bounded-future specs over the whole signal."""
    out = {}
    for sid, var, thr, op, rop in DET:
        trace = sentil.Trace([float(i) for i in range(n)], {var: [float(x) for x in sig[var][:n]]})
        phi = sentil.parse("always[0,10] (%s %s %.1f)" % (var, op, thr))
        phi.robustness_signal(trace)  # warm up
        t0 = time.perf_counter()
        s_sig = phi.robustness_signal(trace)
        s_t = time.perf_counter() - t0

        spec = rtamt.StlDiscreteTimeOfflineSpecification()
        spec.declare_var(var, "float")
        spec.spec = "always[0:10](%s %s %.1f)" % (var, rop, thr)
        spec.parse()
        data = {"time": list(range(n)), var: [float(x) for x in sig[var][:n]]}
        spec_warm = rtamt.StlDiscreteTimeOfflineSpecification()
        spec_warm.declare_var(var, "float")
        spec_warm.spec = "always[0:10](%s %s %.1f)" % (var, rop, thr)
        spec_warm.parse()
        spec_warm.evaluate({"time": list(range(n)), var: [float(x) for x in sig[var][:n]]})
        t0 = time.perf_counter()
        r_sig = spec.evaluate(data)
        r_t = time.perf_counter() - t0

        m = min(len(s_sig), len(r_sig))
        r_vals = np.array([p[1] for p in r_sig[:m]])
        diff = float(np.max(np.abs(np.array(s_sig[:m]) - r_vals))) if m else 0.0
        out[sid] = {
            "samples": n,
            "agree": diff < 1e-6,
            "robustness_max_abs_diff": round(diff, 6),
            "sentil_ms": round(s_t * 1e3, 3),
            "rtamt_ms": round(r_t * 1e3, 3),
            "speedup": round(r_t / s_t, 1) if s_t > 0 else None,
        }
    return out


def probabilistic(times, sig, n):
    """The probabilistic conjunct by streaming Monte Carlo."""
    lifting = sentil.LiftingRegistry()
    lifting.register("pedestrian_distance", sentil.NoiseModel.gaussian(0.0, 0.6),
                     sentil.NoiseInteraction.Additive)
    config = sentil.SmcConfig(samples=1000, seed=7)
    mon = sentil.MultiMonitor()
    mon.add_probabilistic("collision_risk",
                          "P>=0.95(always[0,10] (pedestrian_distance > 4.0))", lifting, config)
    lat = []
    for i in range(n):
        t0 = time.perf_counter()
        mon.update(float(i), {"pedestrian_distance": float(sig["pedestrian_distance"][i])})
        lat.append(time.perf_counter() - t0)
    p = percentiles(lat)
    p["particles"] = 1000
    p["rtamt"] = "not supported (RTAMT has no probabilistic / Monte Carlo monitoring)"
    return p


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--capture", default=os.path.join(HERE, "results", "drive.json"))
    ap.add_argument("--out", default=os.path.join(HERE, "results", "rtamt.json"))
    args = ap.parse_args()
    out = args.out if os.path.isabs(args.out) else os.path.join(HERE, args.out)
    times, sig = load_signals(args.capture)
    n = len(times)
    report = {
        "samples": n,
        "online_multiformula": online_multiformula(times, sig, n),
        "offline_future_bounded": offline_future(times, sig, n),
        "probabilistic_monte_carlo": probabilistic(times, sig, n),
    }
    os.makedirs(os.path.dirname(out), exist_ok=True)
    json.dump(report, open(out, "w"), indent=2)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()