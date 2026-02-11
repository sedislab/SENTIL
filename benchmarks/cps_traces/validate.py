# Validate the CPS trace corpus against SENTIL

import glob
import json
import os
import sys

import sentil

HERE = os.path.dirname(os.path.abspath(__file__))

def main():
    failures = 0
    checked = 0
    paths = sorted(glob.glob(os.path.join(HERE, "*.json")))
    for path in paths:
        entry = json.load(open(path, encoding="utf-8"))
        trace = sentil.Trace(entry["times"], entry["signals"])
        for spec in entry["specifications"]:
            rho = sentil.parse(spec["formula"]).robustness(trace)
            satisfied = rho >= 0.0
            checked += 1
            if satisfied != spec["expected_satisfied"]:
                print(f"[FAIL] {entry['id']} / {spec['name']}: robustness {rho}, expected {'satisfied' if spec['expected_satisfied'] else 'violated'}")
                failures += 1
    if failures:
        print(f"\n{failures} of {checked} verdicts disagree with the annotations")
        sys.exit(1)
    print(f"all {checked} verdicts across {len(paths)} traces match the annotations")

if __name__ == "__main__":
    main()