# Live monitoring on Apollo Cyber RT

This case study runs SENTIL as a real Apollo Cyber RT component: the monitor loads in `mainboard`, subscribes to the perception, localization, and chassis channels, and publishes a verdict per formula on `/apollo/sentil/status`. Three separate cyber processes take part, a scenario source, the monitor component, and a verdict reader, so the verdicts cross the real cyber transport rather than a function call.

## The scenario

An ego vehicle drives at a steady 15 meters per second behind a lead vehicle. The lead is overtaken and closes from 12 meters to 3 meters at 1.5 meters per second. Each cycle the scenario source publishes a `Chassis` with the ego speed, a `LocalizationEstimate`, and a `PerceptionObstacles` frame with the lead at its current range, at 10 Hz on the channels the monitor reads.

The monitor watches two properties from [the default config](../../sentil-apollo/monitor/conf/sentil_monitor.pb.txt):

- A deterministic speed limit, `always[0, 5.0] (ego_speed < 20.0)`, reading `Chassis.speed_mps`.
- A probabilistic follow distance, `P>=0.99(always[0, 2.0] (front_gap > 5.0))`, where `front_gap` is the `FRONT_GAP` builtin (the nearest in-lane obstacle ahead) lifted with Gaussian sensor noise and checked by statistical model checking with 1000 samples at 0.99 confidence.

## The deployment

The monitor is the `SentilMonitorComponent`, a message-fused `apollo::cyber::Component` triggered by perception with localization and chassis as co-channels. `mainboard` loads `libsentil_monitor_component.so`, registers the component, runs its `Init` to build the monitor and the field extractor from the config, and creates the subscriber coroutines. The component carries the compiled SENTIL core inside it, so the build needs no Rust toolchain on the vehicle.

[scenario_publisher.cc](scenario_publisher.cc) is the scenario source and [verdict_echo.cc](verdict_echo.cc) is the reader that prints each verdict from the status channel. Both are plain cyber nodes built with the [BUILD](BUILD) file.

## Results

The full per-cycle verdict stream the reader received is in [results/verdicts.csv](results/verdicts.csv): the time, the formula, whether it is satisfied, and the robustness (deterministic) or the probability with its confidence interval (probabilistic). The monitor delivered 71 verdicts over cyber across the run.

The deterministic speed limit is a bounded-future property over a 5 second window, so it stays provisional until 5 seconds of data accumulate, then resolves concrete at robustness 5.0 (the ego at 15 holds 5 under the 20 limit) and stays satisfied.

The probabilistic follow distance tells the more interesting story. Its 2 second look-ahead window keeps it provisional at first, then it resolves to probability 1.0 with the interval `[0.9934, 1.0000]` while the lead is far. As the lead closes, the probability falls: at 4.3 seconds it drops to 0.9830 with the interval `[0.9688, 0.9908]`, crossing below the 0.99 threshold so the verdict turns to violated, then collapses toward zero as the gap closes inside 5 meters. The confidence interval reported alongside each estimate is the Wilson score interval over the 1000 sampled trajectories.

| quantity | value |
| --- | --- |
| verdicts delivered over cyber | 71 |
| speed limit, once resolved | robustness 5.0, satisfied |
| follow distance while safe | P = 1.0000, interval [0.9934, 1.0000] |
| follow distance at the onset | P = 0.9830, interval [0.9688, 0.9908], below the 0.99 threshold |

A downstream planner subscribing to `/apollo/sentil/status` (the [safety planner example](../../sentil-apollo/examples/safety_planner_subscriber.cc)) brakes the moment the probabilistic verdict crosses its threshold.

## Reproducing

Drop this directory into an Apollo workspace next to `modules/sentil`, build the monitor and the two nodes, then run the three processes against the cyber runtime:

```
buildtool build -p sentil
bazel build //modules/sentil/experiments/apollo_cyber_monitoring:all
./bazel-bin/.../verdict_echo &
mainboard -d modules/sentil/monitor/dag/sentil_monitor.dag &
./bazel-bin/.../scenario_publisher
```

The numbers above were captured on a real Apollo Cyber RT runtime in the matched dev container. See [the Apollo package](../../sentil-apollo/README.md) for the on-platform build and the other builtins.