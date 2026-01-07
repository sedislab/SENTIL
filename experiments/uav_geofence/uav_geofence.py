"""UAV geofence containment and collision-avoidance case study."""

import json
import os

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

import sentil

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = os.path.join(HERE, "results")

X_MAX, Y_MAX = 200.0, 200.0
Z_FLOOR, Z_CEIL = 10.0, 120.0
SAFE_RADIUS = 10.0  # m
V_MAX = 20.0  # m/s
GPS_SIGMA = 3.0  # m

T_FINAL = 40.0  # s
DT = 0.5  # s

UAV_START = np.array([20.0, 100.0, 50.0])
UAV_GOAL = np.array([180.0, 100.0, 50.0])
INTRUDER_START = np.array([106.0, 180.0, 50.0])
INTRUDER_GOAL = np.array([106.0, 20.0, 50.0])

# route name -> peak altitude of the climb (m)
ROUTES = {"direct": 50.0, "marginal": 60.0, "deconflicted": 70.0}
CLIMB_WIDTH = 25.0  # m


def trajectories(climb_peak):
    t = np.arange(0.0, T_FINAL + DT, DT)
    frac = t / T_FINAL
    uav = UAV_START + np.outer(frac, UAV_GOAL - UAV_START)
    bump = (climb_peak - 50.0) * np.exp(-(((uav[:, 0] - 100.0) / CLIMB_WIDTH) ** 2))
    uav[:, 2] = 50.0 + bump
    intruder = INTRUDER_START + np.outer(frac, INTRUDER_GOAL - INTRUDER_START)
    return t, uav, intruder


def signals(t, uav, intruder):
    sep = np.linalg.norm(uav - intruder, axis=1)
    speed = np.zeros_like(t)
    speed[1:] = np.linalg.norm(np.diff(uav, axis=0), axis=1) / DT
    speed[0] = speed[1]
    return {
        "sep": sep.tolist(),
        "x": uav[:, 0].tolist(),
        "y": uav[:, 1].tolist(),
        "z": uav[:, 2].tolist(),
        "speed": speed.tolist(),
    }


def monitor(t, sig):
    trace = sentil.Trace(t.tolist(), sig)
    deterministic = {
        "geofence": f"always (x > 0 and x < {X_MAX} and y > 0 and y < {Y_MAX} "
                    f"and z > {Z_FLOOR} and z < {Z_CEIL})",
        "separation": f"always (sep > {SAFE_RADIUS})",
        "speed_limit": f"always (speed < {V_MAX})",
    }
    results = {}
    for name, text in deterministic.items():
        phi = sentil.parse(text)
        rob = phi.robustness(trace)
        results[name] = {"formula": text, "robustness": round(rob, 4), "satisfied": rob >= 0.0}

    lifting = sentil.LiftingRegistry()
    lifting.register("sep", sentil.NoiseModel.gaussian(0.0, GPS_SIGMA), sentil.NoiseInteraction.Additive)
    config = sentil.SmcConfig(samples=4000, seed=11)
    prstl = sentil.parse(f"P>=0.95(always (sep > {SAFE_RADIUS}))")
    smc = prstl.check(trace, lifting, config)
    results["separation_under_gps_noise"] = {
        "formula": f"P>=0.95(always (sep > {SAFE_RADIUS}))",
        "probability": round(smc.probability, 4),
        "confidence_interval": [round(smc.interval.lower, 4), round(smc.interval.upper, 4)],
        "holds": smc.holds,
        "gps_sigma": GPS_SIGMA,
    }
    return results


def plot(runs):
    fig, (top, bot) = plt.subplots(2, 1, figsize=(10, 8))
    colors = {"direct": "#e74c3c", "marginal": "#e67e22", "deconflicted": "#27ae60"}
    for name, t, uav, intruder, sig, _ in runs:
        top.plot(uav[:, 0], uav[:, 2], color=colors[name], lw=1.8, label=f"{name} (UAV)")
    top.plot(intruder[:, 0], intruder[:, 2], color="#34495e", lw=1.2, ls="--", label="intruder")
    top.scatter([106], [50], color="#34495e", s=40, zorder=5)
    top.axhline(Z_CEIL, color="#7f8c8d", ls=":", lw=1)
    top.set_xlabel("east (m)")
    top.set_ylabel("altitude (m)")
    top.set_title("Vertical profile across the crossing")
    top.legend(fontsize=8)

    for name, t, uav, intruder, sig, _ in runs:
        bot.plot(t, sig["sep"], color=colors[name], lw=1.8, label=name)
    bot.axhline(SAFE_RADIUS, color="#c0392b", ls="--", lw=1.2, label=f"safe radius ({SAFE_RADIUS} m)")
    bot.set_xlabel("time (s)")
    bot.set_ylabel("separation (m)")
    bot.set_title("Separation from intruder (SENTIL flags any dip below the safe radius)")
    bot.legend(fontsize=8)
    fig.tight_layout()
    path = os.path.join(RESULTS, "uav.png")
    fig.savefig(path, dpi=130)
    return path


def main():
    os.makedirs(RESULTS, exist_ok=True)
    report = {
        "case_study": "uav_geofence_collision_avoidance",
        "area_m": {"x": X_MAX, "y": Y_MAX, "z": [Z_FLOOR, Z_CEIL]},
        "safe_radius_m": SAFE_RADIUS,
        "gps_sigma_m": GPS_SIGMA,
        "routes": {},
    }
    runs = []
    for name, peak in ROUTES.items():
        t, uav, intruder = trajectories(peak)
        sig = signals(t, uav, intruder)
        results = monitor(t, sig)
        report["routes"][name] = {
            "climb_peak_m": peak,
            "closest_approach_m": round(float(min(sig["sep"])), 2),
            "specifications": results,
        }
        runs.append((name, t, uav, intruder, sig, results))
    with open(os.path.join(RESULTS, "uav.json"), "w") as f:
        json.dump(report, f, indent=2)
    plot(runs)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()