# CPS trace corpus

This directory contains recorded traces from real-world case studies and it is used as a realistic monitoring benchmark drawn from real systems rather than synthetic ones.

## The traces

| File | Domain | Source | Contains a violation |
| --- | --- | --- | --- |
| `carla_urban_drive.json` | automotive | CARLA drive on Town10HD | lane and clearance events |
| `glucose_missed_lunch_bolus.json` | medical | artificial pancreas, lunch bolus skipped | euglycemia excursion |
| `glucose_tuned.json` | medical | artificial pancreas, every meal dosed | none |
| `circadian_oscillation.json` | systems biology | circadian gene network | none |

The two glucose traces are the same patient under two controllers, so the corpus has both a clean and a violataed trace.

## Format

Each file is one trace:

```json
{
  "id": "glucose_missed_lunch_bolus",
  "domain": "medical",
  "source": "experiments/glucose_control",
  "description": "A 24 h artificial-pancreas run (missed lunch bolus), checked against clinical safety bands.",
  "dt": 1.0,
  "unit_time": "minutes",
  "length": 1441,
  "times": [0.0, 1.0, ...],
  "signals": { "glucose": [120.0, ...] },
  "specifications": [
    { "name": "euglycemia", "formula": "always (glucose > 70 and glucose < 180)", "expected_satisfied": false },
    { "name": "no_severe_hypo", "formula": "always (glucose > 54)", "expected_satisfied": true }
  ],
  "annotations": [
    { "spec": "euglycemia", "interval": [809.0, 1424.0], "severity": "outside the 70-180 mg/dL target band" }
  ]
}
```

An annotation marks a window where the predicate fails and the violation occurs. `expected_satisfied` is the boolean verdict the whole-trace formula should return.

## Regenerating and validating

`build.py` regenerates the corpus from the case studies and `validate.py` loads and runs each trace through SENTIL, and fails if a verdict disagrees with its annotation.

```
python benchmarks/cps_traces/build.py
python benchmarks/cps_traces/validate.py
```