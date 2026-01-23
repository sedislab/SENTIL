# Emergency-braking monitoring on Apollo

This case study runs the SENTIL Apollo integration's perception path against a forward-collision scenario: it reduces a perception obstacle list to the nearest range each cycle and checks a minimum-clearance safety invariant online. It is the exact field extraction and evaluation the Cyber RT monitor component performs, run here without the Cyber transport so the result reproduces on any host.

## The scenario

An autonomous vehicle drives with a static guardrail off to the side at about 15 meters and a lead vehicle ahead. The lead vehicle is overtaken and closes head on from 60 meters at 10 meters per second, until the two come to rest 2 meters apart. Each cycle, perception reports both obstacles as an `apollo.perception.PerceptionObstacles` message with their positions and velocities.

The safety invariant is a minimum clearance to the nearest obstacle: at every step so far, the nearest obstacle must be beyond 5 meters. In SENTIL syntax that is the past-time invariant `historically (nearest > 5.0)`. The past-time form resolves online over the known history, rather than an unbounded `always` that would wait on a future the monitor cannot see.

## The path under test

`aeb_monitor.cc` builds the real `FieldExtractor` from the integration's `common/`, registers the `NEAREST_OBSTACLE_DISTANCE` builtin on the perception channel, and feeds each frame to a streaming `Monitor`. The builtin reduces the whole obstacle list to the smallest planar range, so the nearest obstacle is found by reduction rather than read from a fixed index. The extractor resolves the obstacle list, position, and velocity by field name through protobuf reflection, which is why [perception_obstacle.proto](perception_obstacle.proto) reproduces Apollo's field names and shape exactly: the extractor cannot tell this message apart from the one the Cyber component receives.

## Results

The full per-cycle trace is in [results/aeb_verdicts.csv](results/aeb_verdicts.csv): the time, the nearest range the extractor computed, the monitor's robustness, and the verdict.

The nearest obstacle was the guardrail at 15.3 meters while the lead vehicle was still far off, and the invariant held with robustness 10.3. As the lead closed past the guardrail it became the nearest obstacle, and the running robustness tracked its range minus 5. At t=5.6 s the lead crossed inside 5 meters, the invariant was breached, and the robustness went to -1.0. It kept falling to -3.0 as the lead came to rest 2 meters away, where the past-time invariant latches: once the clearance has been lost, the history can no longer satisfy it.

| quantity | value |
| --- | --- |
| nearest obstacle while safe | 15.3 m (the guardrail) |
| robustness while the invariant held | 10.3 |
| time the invariant held | 5.6 s |
| robustness at the breach | -1.0 |
| robustness at the closest approach | -3.0 |

A planner subscribing to the monitor's status would brake on the satisfied-to-violated edge at t=5.6 s, well before the 2-meter closest approach.

## Reproducing

The harness compiles the integration's `common/field_extractor.cc` and the driver against the perception proto and the linked core, then runs the scenario:

```
protoc -I=. --cpp_out=. modules/sentil/proto/sentil_config.proto perception_obstacle.proto
c++ -std=c++17 -I. -I<sentil>/include modules/sentil/common/field_extractor.cc aeb_monitor.cc \
  modules/sentil/proto/sentil_config.pb.cc perception_obstacle.pb.cc \
  -L<sentil>/lib -lsentil -lprotobuf -o aeb
./aeb > results/aeb_verdicts.csv
```

The same `NEAREST_OBSTACLE_DISTANCE` builtin, config, and engine run inside the Cyber RT component on a vehicle; see [the Apollo package](../../sentil-apollo/README.md) for the on-platform build and the `MIN_TTC` and `FRONT_GAP` builtins for time-to-collision and in-lane gap requirements.