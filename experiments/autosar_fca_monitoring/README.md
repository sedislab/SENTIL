# Forward-collision monitoring on the Adaptive Platform

This case study runs SENTIL as a real AUTOSAR Adaptive service that watches a forward-collision requirement and streams verdicts to a planner over SOME/IP. It is the autosar-adaptive integration end to end: three processes, the production transport, and a safety property checked online while the scenario plays out.

## The scenario

An ego vehicle approaches a slower lead vehicle. Perception reports the front gap each cycle, starting at 30 meters and closing half a meter every 100 ms as the lead vehicle is overtaken. The requirement is a minimum following distance: the gap must stay above 5 meters. In SENTIL syntax that is `always (front_gap > 5.0)`, whose quantitative robustness is `front_gap - 5`, positive while the requirement holds and negative once it is breached.

## The deployment

Three Adaptive Platform applications run as separate processes and find each other through service discovery, exactly as they would on a vehicle:

- `perception_publisher` offers `SignalInterface` (service 0x6000) and pushes a `SignalFrame` with the current `front_gap` each cycle.
- `sentil_monitor` consumes `SignalInterface`, evaluates the spec on every frame, and offers `VerdictInterface` (service 0x6001): a latched `Verdict` field and a `Violation` event.
- `planner_subscriber` consumes `VerdictInterface`, prints each verdict, and engages a fallback when the `Violation` event fires.

The transport is open-source vsomeip, so the verdicts cross a real SOME/IP bus between processes rather than a function call inside one. The monitor process hosts the routing manager.

## Results

The full verdict stream the planner received is in [results/verdicts.txt](results/verdicts.txt), and the per-tick robustness in [results/verdicts.csv](results/verdicts.csv).

The monitor delivered 600 verdicts over the bus across the 60-second scenario. The requirement held for the first 5 seconds while the gap shrank from 30 meters toward 5. At t=5.1 s the gap crossed below 5 meters, robustness went negative at -0.5, the `Violation` event fired, and the planner logged `VIOLATION at t=5.1, engaging fallback`. The robustness kept falling as the gap closed, reaching -4.0 once the gap bottomed out at 1 meter, and stayed there for the rest of the run.

| quantity | value |
| --- | --- |
| verdicts delivered over SOME/IP | 600 |
| time the requirement held | 5.1 s |
| robustness at the breach | -0.5 |
| robustness at the closest approach | -4.0 |
| violation events | 1 |

The single violation event, rather than one per tick after the breach, is the monitor firing on the satisfied-to-unsatisfied transition of a concrete verdict, so a planner reacts once to the onset rather than being flooded.

## Reproducing

The case study uses the package's own forward-collision example binaries. Build the package against a SENTIL install tree and run the capture:

```
cmake -S sentil-autosar-adaptive -B build -DSENTIL_ROOT=<install> -DCMAKE_PREFIX_PATH=<vsomeip>
cmake --build build
VSOMEIP_CONFIGURATION=sentil-autosar-adaptive/examples/adas_fca_monitor/vsomeip/demo.json \
  ./build/sentil_monitor &
./build/planner_subscriber &
./build/perception_publisher
```

The numbers above were captured on a Linux x86-64 node with the gpu-free core and vsomeip 3. See [the example](../../sentil-autosar-adaptive/examples/adas_fca_monitor) for the three programs and [the package guide](../../sentil-autosar-adaptive/README.md) for vendor Adaptive Platform builds.