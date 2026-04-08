# Case study: an artificial pancreas

A closed-loop insulin controller is a safety-critical real-time system: too little insulin leaves a patient hyperglycemic, too much drives them into hypoglycemia, and the only glucose signal available is a noisy continuous-glucose-monitor (CGM) reading. This case study shows SENTIL verifying such a controller against clinical safety specifications, both deterministically on the patient's true glucose and probabilistically under the sensor noise.

## What it does

`glucose_control.py` simulates a type-1 patient with the FDA-accepted UVA/Padova model (the meal-simulation model of Dalla Man, Rizza, and Cobelli 2007 with the S2013 risk-based utilization and average-adult parameters) over a 24-hour day with breakfast, lunch, and dinner. The model holds fasting glucose near 120 mg/dL and gives a bolused meal a peak near 180 that recovers. It runs two basal-bolus controllers: one that misses the lunch bolus, a common and dangerous real-world lapse, and one that doses every meal. SENTIL then checks each glucose trace against the specifications below and writes `results/glucose.json` and `results/glucose.png`.

## The specifications

| Name | Formula | Meaning |
| --- | --- | --- |
| euglycemia | `always (glucose > 70 and glucose < 180)` | stay in the target range |
| no_severe_hypo | `always (glucose > 54)` | never reach severe hypoglycemia |
| bounded_hyper | `always (glucose < 250)` | bound hyperglycemia |
| hypo_recovers_30min | `always ((glucose < 70) implies eventually[0,30] (glucose > 70))` | any low recovers within 30 minutes |
| hypo_safety_under_cgm_noise | `P>=0.95(always (glucose > 70))` | avoid hypoglycemia with 95% probability under CGM noise |

The last is probabilistic: it lifts each reading by the CGM's Gaussian error and asks how confident we can be in the safety verdict given that we only ever see noisy sensor values.

## Result

SENTIL flags the missed-bolus controller and clears the tuned one. With the lunch bolus skipped, glucose leaves the target band at minute 809 and does not come back inside it until minute 1424, close to the end of the day, peaking near 285 mg/dL. That puts `euglycemia` at robustness -105.02, with the excursion reported as the interval [809, 1424] minutes, and the peak is high enough to break `bounded_hyper` too, at -35.02. The tuned controller keeps every meal inside the target band (robustness +7.95). Both stay clear of hypoglycemia, and the probabilistic check confirms the hypoglycemia risk is low under sensor noise. The plot shows both days with the SENTIL-flagged excursion shaded.

## Run it

```
python experiments/glucose_control/glucose_control.py
```

It needs `sentil` (the Python package), NumPy, and Matplotlib. The model is deterministic given its seed, so the numbers reproduce.