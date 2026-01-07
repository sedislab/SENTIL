# Case study: UAV geofence and collision avoidance

A delivery drone has to stay inside its authorized airspace and keep clear of other traffic, and it has to do both from noisy GPS. This case study flies a quadrotor across a geofenced area while an intruder aircraft cuts through its path, and uses SENTIL to check containment, separation, and speed. It also shows the one thing a deterministic monitor cannot do: weigh a safety margin against sensor uncertainty.

## What it does

`uav_geofence.py` flies three routes through the same encounter. The intruder crosses the meeting point offset by 6 m, so the rest of the separation has to come from altitude. The `direct` route holds altitude and collides. The `marginal` route climbs just enough to clear the 10 m separation bound by less than a metre. The `deconflicted` route climbs higher and is comfortably clear. SENTIL checks each route and writes `results/uav.json` and `results/uav.png`.

## The specifications

| Name | Formula | Meaning |
| --- | --- | --- |
| geofence | `always (x > 0 and x < 200 and y > 0 and y < 200 and z > 10 and z < 120)` | stay inside the operating area |
| separation | `always (sep > 10)` | keep clear of the intruder |
| speed_limit | `always (speed < 20)` | hold under the speed limit |
| separation_under_gps_noise | `P>=0.95(always (sep > 10))` | keep separation with 95% probability under GPS error |

## Result

The deterministic checks split the routes as you would expect: the direct route violates separation (robustness about -5.5 m), and the marginal and deconflicted routes both satisfy it (the marginal margin is only +0.7 m). Containment and speed hold throughout.

The probabilistic check is where it gets interesting. The marginal route passes the deterministic separation bound, so a classical monitor reports it safe. SENTIL lifts the separation by the GPS error and finds the probability that separation actually holds is about 0.08, far below the 0.95 the specification demands. A 0.7 m margin against 3 m of position noise is not a safe margin, and SENTIL says so. The deconflicted route clears the same probabilistic check at about 0.998. The plot shows the vertical profiles and the separation over time, with the safe radius marked.

## Run it

```
python experiments/uav_geofence/uav_geofence.py
```

It needs `sentil` (the Python package), NumPy, and Matplotlib. The estimate is seeded, so the numbers reproduce.