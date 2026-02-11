# CPS trace corpus from the case studies

import csv
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
EXP = os.path.join(ROOT, "experiments")

def violation_intervals(times, values, holds):
    intervals = []
    start = None
    for t, v in zip(times, values):
        if holds(v):
            if start is not None:
                intervals.append([start, prev])
                start = None
        else:
            start = t if start is None else start
        prev = t
    if start is not None:
        intervals.append([start, prev])
    return intervals

def write(entry):
    path = os.path.join(HERE, entry["id"] + ".json")
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(entry, handle, indent=2)
    print("wrote", os.path.basename(path))

def carla():
    drive = json.load(open(os.path.join(EXP, "carla_driving", "results", "drive.json"), encoding="utf-8"))
    sig = drive["signals"]
    times = sig["t"]
    obstacle = [min(v, p) for v, p in zip(sig["vehicle_distance"], sig["pedestrian_distance"])]
    signals = {
        "lateral_error": sig["lateral_error"],
        "obstacle_distance": obstacle,
        "speed": sig["speed"],
    }
    specs = [
        ("lane_keeping", "always (lateral_error < 0.3 and lateral_error > -0.3)",
         "lateral_error", lambda v: -0.3 < v < 0.3, "lane departure beyond 0.3 m"),
        ("obstacle_clearance", "always (obstacle_distance > 5.0)",
         "obstacle_distance", lambda v: v > 5.0, "clearance under 5 m"),
        ("speed_limit", "always (speed < 50.0)",
         "speed", lambda v: v < 50.0, "over the 50 unit urban limit"),
    ]
    return trace_entry("carla_urban_drive", "automotive", "experiments/carla_driving",
                       "A 300 s recorded CARLA drive on Town10HD, checked for lane keeping, clearance, and the speed limit.",
                       drive["dt"], "seconds", times, signals, specs)

def glucose():
    sys.path.insert(0, os.path.join(EXP, "glucose_control"))
    import glucose_control as gc
    out = []
    for cid, (icr, skip) in gc.CONTROLLERS.items():
        times, g = gc.simulate(icr, skip)
        signals = {"glucose": list(g)}
        specs = [
            ("euglycemia", "always (glucose > 70 and glucose < 180)",
             "glucose", lambda v: 70 < v < 180, "outside the 70-180 mg/dL target band"),
            ("no_severe_hypo", "always (glucose > 54)",
             "glucose", lambda v: v > 54, "severe hypoglycemia below 54 mg/dL"),
        ]
        label = "missed lunch bolus" if skip else "every meal dosed"
        out.append(trace_entry(f"glucose_{cid}", "medical", "experiments/glucose_control",
                               f"A 24 h artificial-pancreas run ({label}), checked against clinical safety bands.",
                               1.0, "minutes", list(times), signals, specs))
    return out

def circadian():
    path = os.path.join(EXP, "circadian_gene_network", "circadian_traces.csv")
    rows = list(csv.reader(open(path, encoding="utf-8")))
    cols = list(zip(*[[float(x) for x in r] for r in rows[1:]]))
    times = list(cols[0])
    mean = list(cols[1])
    specs = [
        ("oscillation_peaks", "always[0,240] (eventually[0,24] (activator > 3000))",
         None, None, None),
        ("oscillation_troughs", "always[0,240] (eventually[0,24] (activator < 2000))",
         None, None, None),
    ]
    return trace_entry("circadian_oscillation", "systems_biology", "experiments/circadian_gene_network",
                       "The circadian activator on the ensemble mean, checked for a sustained 24 h rhythm.",
                       1.0, "hours", times, {"activator": mean}, specs)

def trace_entry(cid, domain, source, description, dt, unit_time, times, signals, specs):
    spec_out = []
    annotations = []
    for name, formula, var, holds, severity in specs:
        violated_here = []
        if var is not None:
            violated_here = violation_intervals(times, signals[var], holds)
            for interval in violated_here:
                annotations.append({"spec": name, "interval": interval, "severity": severity})
        spec_out.append({"name": name, "formula": formula, "expected_satisfied": not violated_here})
    return {
        "id": cid,
        "domain": domain,
        "source": source,
        "description": description,
        "dt": dt,
        "unit_time": unit_time,
        "length": len(times),
        "times": times,
        "signals": signals,
        "specifications": spec_out,
        "annotations": annotations,
    }

def main():
    entries = [carla()] + glucose() + [circadian()]
    for entry in entries:
        write(entry)
    print(f"{len(entries)} traces")

if __name__ == "__main__":
    main()