# CPS trace corpus

Recorded and high-fidelity traces from the case studies, each paired with the specifications it was checked against and the intervals where a specification is violated. The corpus serves two purposes: a realistic monitoring workload drawn from real systems rather than synthetic signals, and a ground truth the monitor is validated against, since every annotated verdict has to agree with what SENTIL computes.

## The traces

| File | Domain | Source | Contains a violation |
| --- | --- | --- | --- |
| `carla_urban_drive.json` | automotive | CARLA drive on Town10HD | clearance and speed events |
| `glucose_missed_lunch_bolus.json` | medical | artificial pancreas, lunch bolus skipped | euglycemia excursion |
| `glucose_tuned.json` | medical | artificial pancreas, every meal dosed | none |
| `circadian_oscillation.json` | systems biology | circadian gene network | none |

The two glucose traces are the same patient under two controllers, so the corpus carries a matched violated and clean pair, and the circadian trace is a clean case where the property (sustained oscillation) holds.

## Format

Each file is one trace:

```json
{
  "id": "glucose_missed_lunch_bolus",
  "domain": "medical",
  "source": "experiments/glucose_control",
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

An annotation marks a window where the pointwise predicate fails, which is where the violation lies. `expected_satisfied` is the verdict the whole-trace formula should return.

## Regenerating and validating

`build.py` derives the corpus from the case studies: the CARLA signals come from the recorded drive, the glucose traces from rerunning the simulation, and the circadian trace from the ensemble mean. `validate.py` loads every trace, runs each specification through SENTIL, and fails if a verdict disagrees with its annotation.

```
python benchmarks/cps_traces/build.py
python benchmarks/cps_traces/validate.py
```