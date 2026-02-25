"""Monitor a recorded CARLA driving trace against an autonomous-driving safety spec."""

import argparse
import json
import math
import os
import time

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

import sentil

HERE = os.path.dirname(os.path.abspath(__file__))

LANE_TOL = 0.3  # m
MIN_CLEARANCE = 5.0  # m
SPEED_LIMIT = 50.0  # km/h
COLLISION_RADIUS = 2.0  # m
PED_SPEED_SIGMA = 0.7  # m/s
LOOKAHEAD = 10.0  # s
PROB_THRESHOLD = 0.99
PED_RANGE = 25.0  # m


def load(path):
    with open(path) as f:
        return json.load(f)


def deterministic(trace, sig):
    specs = {
        "lane_keeping": f"always (lateral_error < {LANE_TOL} and lateral_error > -{LANE_TOL})",
        "obstacle_clearance": f"always (obstacle_distance > {MIN_CLEARANCE})",
        "speed_limit": f"always (speed < {SPEED_LIMIT})",
    }
    out = {}
    for name, text in specs.items():
        phi = sentil.parse(text)
        rob = phi.robustness(trace)
        out[name] = {"formula": text, "robustness": round(rob, 4), "satisfied": rob >= 0.0}
    return out


def streaming_latency(sig, dt):
    """Per-sample cost of the deterministic spec on the streaming monitor."""
    formula = (f"always (lateral_error < {LANE_TOL} and lateral_error > -{LANE_TOL}) and "
               f"always (obstacle_distance > {MIN_CLEARANCE}) and always (speed < {SPEED_LIMIT})")
    mon = sentil.OnlineMonitor(formula)
    order = [mon.symbol_index(v) for v in ("lateral_error", "obstacle_distance", "speed")]
    packed = []
    lat = np.abs(sig["lateral_error"])
    obs = sig["obstacle_distance"]
    spd = sig["speed"]
    for i in range(len(sig["t"])):
        row = [0.0, 0.0, 0.0]
        row[order[0]] = lat[i]
        row[order[1]] = obs[i]
        row[order[2]] = spd[i]
        packed.append(row)
    for i in range(len(packed)):
        mon.update_packed(sig["t"][i], packed[i])
    mon.reset()
    t0 = time.perf_counter()
    for i in range(len(packed)):
        mon.update_packed(sig["t"][i], packed[i])
    elapsed = time.perf_counter() - t0
    n = len(packed)
    return {
        "samples": n,
        "total_ms": round(1e3 * elapsed, 3),
        "mean_per_sample_us": round(1e6 * elapsed / n, 4),
        "sustained_hz": round(n / elapsed, 1),
    }


def collision_free_probability(ego_future, ped_pos0, ped_vel, dt):
    """PrSTL estimate that the ego stays clear of one pedestrian over the lookahead."""
    taus = np.arange(len(ego_future)) * dt
    ped_future = ped_pos0[None, :] + np.outer(taus, ped_vel)
    clearance = np.linalg.norm(ego_future - ped_future, axis=1) - COLLISION_RADIUS
    sigma = max(0.05, PED_SPEED_SIGMA * taus[int(np.argmin(clearance))])
    trace = sentil.Trace(taus.tolist(), {"clearance": clearance.tolist()})
    lifting = sentil.LiftingRegistry()
    lifting.register("clearance", sentil.NoiseModel.gaussian(0.0, sigma),
                     sentil.NoiseInteraction.Additive)
    config = sentil.SmcConfig(samples=2000, seed=5)
    phi = sentil.parse(f"P>={PROB_THRESHOLD}(always (clearance > 0))")
    return phi.check(trace, lifting, config).probability


def probabilistic(sig, dt):
    """Per-frame collision-free probability and the latency of computing it."""
    ego = np.array(sig["ego_xy"], dtype=float)
    horizon = int(round(LOOKAHEAD / dt))
    n = len(sig["t"])

    probs = np.ones(n)
    latencies = []
    if n > 50:
        collision_free_probability(ego[:50], ego[0] + np.array([12.0, 0.0]),
                                   np.array([0.0, 0.5]), dt)
    for i, pr in enumerate(sig["pedestrian_relative"]):
        dx, dy, pvx, pvy = pr
        if dx is None or math.hypot(dx, dy) > PED_RANGE:
            continue
        h = min(horizon, n - 1 - i)
        if h < 5:
            continue
        step = max(1, int(round(0.2 / dt)))
        ego_future = ego[i:i + h + 1:step]
        ped_pos0 = ego[i] + np.array([dx, dy])
        t0 = time.perf_counter()
        probs[i] = collision_free_probability(ego_future, ped_pos0, np.array([pvx, pvy]), dt * step)
        latencies.append(1e3 * (time.perf_counter() - t0))
    lat = np.array(latencies) if latencies else np.array([0.0])
    return probs, {
        "frames_checked": len(latencies),
        "median_ms": round(float(np.median(lat)), 4),
        "p99_ms": round(float(np.percentile(lat, 99)), 4),
        "max_ms": round(float(np.max(lat)), 4),
    }


def find_critical(sig, probs):
    """The frame where deterministic clearance holds but the probability does not."""
    best = None
    for i in range(len(probs)):
        clear = sig["obstacle_distance"][i] > MIN_CLEARANCE
        if clear and probs[i] < PROB_THRESHOLD:
            if best is None or probs[i] < probs[best]:
                best = i
    if best is None:
        return None
    return {
        "frame": best,
        "time_s": sig["t"][best],
        "obstacle_distance_m": sig["obstacle_distance"][best],
        "pedestrian_distance_m": sig["pedestrian_distance"][best],
        "collision_free_probability": round(float(probs[best]), 4),
        "deterministic_clearance_holds": True,
    }


def plot(sig, probs, critical, out):
    t = np.array(sig["t"])
    fig, (a, b, c) = plt.subplots(3, 1, figsize=(11, 8), sharex=True)
    a.plot(t, sig["lateral_error"], color="#2c3e50", lw=1)
    a.axhline(LANE_TOL, color="#c0392b", ls="--", lw=1)
    a.axhline(-LANE_TOL, color="#c0392b", ls="--", lw=1)
    a.set_ylabel("lateral error (m)")
    a.set_title("Lane keeping")
    b.plot(t, sig["obstacle_distance"], color="#2980b9", lw=1, label="nearest vehicle")
    b.plot(t, sig["pedestrian_distance"], color="#16a085", lw=1, label="nearest pedestrian")
    b.axhline(MIN_CLEARANCE, color="#c0392b", ls="--", lw=1, label="min clearance")
    b.set_ylabel("distance (m)")
    b.set_ylim(0, 50)
    b.set_title("Clearance")
    b.legend(fontsize=8)
    c.plot(t, probs, color="#8e44ad", lw=1)
    c.axhline(PROB_THRESHOLD, color="#c0392b", ls="--", lw=1, label=f"threshold {PROB_THRESHOLD}")
    c.set_ylabel("P(collision-free)")
    c.set_xlabel("time (s)")
    c.set_ylim(0, 1.02)
    c.set_title("Probabilistic collision-free verdict over a 10 s lookahead")
    if critical:
        c.scatter([critical["time_s"]], [critical["collision_free_probability"]],
                  color="#c0392b", zorder=5, s=40)
    c.legend(fontsize=8)
    fig.tight_layout()
    path = os.path.join(os.path.dirname(out), "drive.png")
    fig.savefig(path, dpi=130)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--trace", default=os.path.join(HERE, "results", "drive.json"))
    ap.add_argument("--out", default=os.path.join(HERE, "results", "verdicts.json"))
    args = ap.parse_args()
    out = args.out if os.path.isabs(args.out) else os.path.join(HERE, args.out)

    data = load(args.trace)
    sig = data["signals"]
    dt = data["dt"]
    sig["lateral_error"] = np.array(sig["lateral_error"], dtype=float)
    veh = np.array(sig["vehicle_distance"], dtype=float)
    ped = np.array(sig["pedestrian_distance"], dtype=float)
    sig["obstacle_distance"] = np.minimum(veh, ped).tolist()
    sig["speed"] = np.array(sig["speed"], dtype=float)

    trace = sentil.Trace(sig["t"], {
        "lateral_error": np.abs(sig["lateral_error"]).tolist(),
        "obstacle_distance": sig["obstacle_distance"],
        "speed": sig["speed"].tolist(),
    })
    det = deterministic(trace, sig)
    stream = streaming_latency(sig, dt)
    probs, prob_lat = probabilistic(sig, dt)
    critical = find_critical(sig, probs)

    report = {
        "case_study": "carla_autonomous_driving",
        "trace": {"source": data.get("source"), "town": data.get("town"),
                  "frames": data["frames"], "dt": dt,
                  "duration_s": round(data["frames"] * dt, 1),
                  "recorded_collisions": data.get("collision_total", 0)},
        "deterministic": det,
        "probabilistic": {
            "formula": "P>=0.99(always[0,10] (no collision under pedestrian uncertainty))",
            "min_collision_free_probability": round(float(np.min(probs)), 4),
            "threshold": PROB_THRESHOLD,
            "critical_frame": critical,
        },
        "latency": {
            "deterministic_streaming": stream,
            "probabilistic_per_frame_ms": prob_lat,
        },
    }
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w") as f:
        json.dump(report, f, indent=2)
    plot(sig, probs, critical, out)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()