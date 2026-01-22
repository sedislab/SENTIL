# sentil-apollo

Runtime verification and controller synthesis for Baidu Apollo, built on the SENTIL engine and packaged as a Cyber RT module.

It ships two components. The monitor watches Apollo channels against Signal Temporal Logic and probabilistic STL specifications and publishes a verdict per formula. The control component synthesizes a command from a specification or shields Apollo's nominal command so it stays within the specification's bounds. Both read their formulas and their channel-to-variable mapping from a protobuf-text config, and both carry the compiled SENTIL core inside the module, so the build needs no Rust toolchain. This directory installs into an Apollo workspace as `modules/sentil`.

## A first monitor in five minutes, no code

Write the formulas and the field mapping in a config, point the dag at it, and launch. The default `monitor/conf/sentil_monitor.pb.txt` already checks an ego speed limit and a probabilistic follow distance:

```
formulas {
  id: 1
  expression: "always[0, 5.0] (ego_speed < 20.0)"
}
input_channels {
  channel: "/apollo/canbus/chassis"
  message_type: "apollo.canbus.Chassis"
  fields {
    variable: "ego_speed"
    field_path: "speed_mps"
  }
}
output_channel: "/apollo/sentil/status"
```

Launch it and watch the verdict:

```
cyber_launch start modules/sentil/monitor/launch/sentil_monitor.launch
cyber_monitor -c /apollo/sentil/status
```

The status carries, per formula, the robustness, whether it is satisfied, an OK/WARN/ERROR/FATAL severity, and for a `P~p` formula the running probability with its Wilson interval.

## Mapping channels to variables

Each `input_channel` names a channel, its message type, and the fields to read. A field is either a dotted path or a builtin.

A `field_path` resolves a nested, optionally indexed path against the message: `speed_mps`, `pose.position.x`, `perception_obstacle[0].position.x`. The path is resolved against the message descriptor once at startup, so a wrong path fails when the component initializes rather than reading a silent zero at runtime.

A `builtin` computes a quantity a raw path cannot express, because the nearest obstacle is not the obstacle at index 0:

| Builtin | Meaning |
| --- | --- |
| `NEAREST_OBSTACLE_DISTANCE` | the smallest planar range to any obstacle |
| `MIN_TTC` | the smallest time to collision over obstacles closing ahead |
| `FRONT_GAP` | the gap to the nearest obstacle within the ego lane corridor |

The builtins read the standard perception obstacle list and treat each obstacle's position and velocity as ego relative.

## Choosing the component model

The monitor ships in two forms. The default, in `dag/sentil_monitor.dag`, is a fused component triggered by perception with localization and chassis as co-channels, so each verdict reflects a consistent message set. The timer form, in `dag/sentil_timed_monitor.dag`, reads the same channels through pull readers and emits a verdict at a fixed rate; use it when a steady cadence matters more than reacting to each perception frame.

The online monitor evaluates on the samples as they arrive and reads whether a formula is probabilistic from its `P` prefix, so the `algorithm`, `semantics`, `interpolation`, and `backend` fields configure the offline `sentil_record_analyzer`, not the online monitor.

## Synthesis and shielding

The control component, `control/`, turns a specification into a command. Set `mode` in `control/conf/sentil_control.pb.txt`:

- `SHIELD` (default, safest): take Apollo's nominal command off `nominal_channel` and project the throttle, brake, and steering into the configured bounds before they reach the vehicle. SENTIL guards Apollo's controller rather than replacing it.
- `SYNTHESIZE`: SENTIL is the controller. It builds the state from localization and chassis, plans over the horizon within a per-tick deadline, and writes a `ControlCommand`.
- `ADVISORY`: synthesize as above but publish to `/apollo/sentil/control_advice` with no actuation authority, so a synthesized controller can be evaluated against a live drive before it is trusted.

The control component runs on a timer at `control_period_ms` and emits on its deadline even if an input goes quiet; its launch file sets `exception_handler: respawn`, so a failure degrades rather than dies. Pair it with the monitor running the same spec and you have a synthesize, monitor, and re-plan loop in one dag.

## Building and installing

The module links the SENTIL core through a Bazel external repository `@sentil_cpp` that wraps a prebuilt `libsentil.so` and the headers. Build SENTIL once (`cargo build --release -p sentil-ffi` and install the C++ headers), then declare the repository in your Apollo workspace pointing at the install tree, with `bzl/sentil_cpp.BUILD` as its build file. With that in place:

```
buildtool build -p sentil
buildtool install sentil
```

In the dev container the source build is `aem start`, `buildtool build -p sentil`, then `cyber_launch start modules/sentil/monitor/launch/sentil_monitor.launch`.

## Acting on the verdict

Planning subscribes to `/apollo/sentil/status` and engages a fallback when `all_satisfied` is false or a probabilistic interval drops below threshold; `examples/safety_planner_subscriber.cc` is a small standalone version. For the heavier algorithms a per-tick budget cannot afford, `tools/sentil_record_analyzer` replays a recorded drive offline and runs SMC or SPRT over the same config.

## Credits and license

SENTIL is the work of Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab. Dual licensed under MIT or Apache-2.0.