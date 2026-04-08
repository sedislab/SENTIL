<div align="center">

# SENTIL

#### The Baidu Apollo Cyber RT module for Probabilistic Signal Temporal Logic

[![Apollo Cyber RT](https://img.shields.io/badge/Apollo-Cyber%20RT%20module-blue.svg)](https://github.com/ApolloAuto/apollo)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

Apollo Cyber RT bindings for the [`sentil`](../sentil-core) engine, packaged as a module you drop into an Apollo workspace as `modules/sentil`. Two components ship with it. The monitor watches Apollo channels against Signal Temporal Logic and probabilistic STL specifications and publishes a verdict per formula. The control component synthesizes a command from a specification or shields Apollo's nominal command so it stays within the specification's bounds. Both read their formulas and their channel-to-variable mapping from a protobuf-text config. The Apollo build links the compiled core through the `@sentil_cpp` Bazel external repository, so the module build needs no Rust toolchain.

The engine behind it has three capabilities. Deterministic STL monitoring, over a live channel or a recorded drive. Probabilistic monitoring, which fits a noise model to a sensor and estimates satisfaction probability with a confidence bound. And synthesis, from a specification to a control command to an online controller. Apollo reaches all three through the two components and two offline tools.

## Your first monitor

Write the formulas and the field mapping in a config, point the dag at it, and launch. The default `monitor/conf/sentil_monitor.pb.txt` checks an ego speed limit and a probabilistic follow distance; here is the speed-limit half:

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

The status carries, per formula, the robustness, whether it is satisfied, an OK/WARN/ERROR/FATAL severity, and for a `P~p` formula the running probability with its Wilson interval. A negative robustness means the property is violated, and its magnitude is the margin by which it fails. The monitor folds each message as it arrives at O(1) amortized cost per sample, so it holds a real-time loop; the measured latency lives in [`benchmarks/`](../benchmarks) and [`docs/CLAIMS.md`](../docs/CLAIMS.md).

## Mapping channels to variables

Each `input_channel` names a channel, its message type, and the fields to read. A field is either a dotted path or a builtin.

A `field_path` resolves a nested, optionally indexed path against the message: `speed_mps`, `pose.position.x`, `perception_obstacle[0].position.x`. The path is resolved against the message descriptor once at startup, so a wrong path fails when the component initializes rather than reading a silent zero at runtime.

A `builtin` computes a quantity a raw path cannot express, because the nearest obstacle is not the obstacle at index 0:

| Builtin | Meaning |
| --- | --- |
| `NEAREST_OBSTACLE_DISTANCE` | the smallest planar range to any obstacle |
| `MIN_TTC` | the smallest time to collision over obstacles closing ahead |
| `FRONT_GAP` | the gap to the nearest obstacle within the ego lane corridor |

The builtins read the standard perception obstacle list and treat each obstacle's position and velocity as ego relative, so an upstream adapter must transform Apollo's world-frame perception into the ego frame before the monitor reads it.

## Choosing the component model

The monitor ships in two forms. The default, in `dag/sentil_monitor.dag`, is a fused component triggered by perception with localization and chassis as co-channels, so each verdict reflects a consistent message set. The timer form, in `dag/sentil_timed_monitor.dag`, reads the same channels through pull readers and emits a verdict at a fixed rate; reach for it when a steady cadence matters more than reacting to each perception frame.

Both read whether a formula is probabilistic from its leading `P`, so the `algorithm` and `semantics` fields, with the statistical settings, configure the offline `sentil_record_analyzer` rather than the online monitor.

## Synthesis and shielding

The control component turns a specification into a command. Set `mode` in `control/conf/sentil_control.pb.txt`:

- `SHIELD` (default, safest): take Apollo's nominal command off `nominal_channel` and project the throttle, brake, and steering into the configured bounds before they reach the vehicle. SENTIL guards Apollo's controller rather than replacing it.
- `SYNTHESIZE`: SENTIL is the controller. It builds the state from localization and chassis, plans over the horizon within a per-tick deadline, and writes a `ControlCommand`. `examples/synthesize_control.pb.txt` is a runnable one, a double integrator held between 1 and 9 metres once it has accelerated into the band.
- `ADVISORY`: synthesize as above but publish to `/apollo/sentil/control_advice` with no actuation authority, so a synthesized controller can be evaluated against a live drive before it is trusted.

The control component runs on a timer at `control_period_ms` and emits on its deadline even if an input goes quiet; its launch file sets `exception_handler: respawn`, so a failure degrades rather than dies. Pair it with the monitor running the same spec and you have a synthesize, monitor, and re-plan loop in one dag.

For the design-time work the online controller cannot afford per tick, `tools/sentil_synthesizer` reads a control config that carries a `model` and runs the offline side of the subsystem. The default `control/conf/sentil_control.pb.txt` is SHIELD only and has no model to plan against, so point the synthesizer at a SYNTHESIZE config:

```
sentil_synthesizer --config=examples/synthesize_control.pb.txt --op=plan
```

`--op=plan` synthesizes the open-loop input sequence that satisfies the spec over the horizon, `--op=witness` searches for a counterexample input that violates it, and `--op=chance` estimates whether the spec holds with at least a target probability under Gaussian process noise. It needs no Cyber RT runtime, only the config bridge and the engine, so it runs on any host.

## Acting on the verdict

Planning subscribes to `/apollo/sentil/status` and engages a fallback when `all_satisfied` is false or a probabilistic interval drops below threshold; `examples/safety_planner_subscriber.cc` is a small standalone version. For the heavier algorithms a per-tick budget cannot afford, `tools/sentil_record_analyzer` replays a recorded drive offline and runs SMC or SPRT over the same config:

```
sentil_record_analyzer --config=modules/sentil/monitor/conf/sentil_monitor.pb.txt --record=drive.record
```

## Install

Apollo has no package registry, so the module installs by dropping into a workspace and building it with Apollo's `buildtool`. Cyber RT runs on Linux only.

### The Apollo workspace

First supply the core as the `@sentil_cpp` external repository. Build it once and stage an install tree with the C header, the `sentil.hpp` surface, `libsentil.so`, and the deterministic oracle the parity test replays:

```
cargo build --release -p sentil-ffi
install -d /opt/sentil/include/sentil /opt/sentil/lib /opt/sentil/share/sentil
install -m644 sentil-ffi/include/sentil.h /opt/sentil/include/sentil.h
install -m644 sentil-cpp/include/sentil/*.hpp /opt/sentil/include/sentil/
install -m644 target/release/libsentil.so /opt/sentil/lib/libsentil.so
install -m644 benchmarks/deterministic/oracle.json /opt/sentil/share/sentil/oracle.json
```

Declare the repository in your workspace pointing `@sentil_cpp` at that install tree, with `bzl/sentil_cpp.BUILD` as its build file:

```
new_local_repository(
    name = "sentil_cpp",
    path = "/opt/sentil",
    build_file = "//modules/sentil:bzl/sentil_cpp.BUILD",
)
```

Then drop this directory in as `modules/sentil` and build:

```
buildtool build -p sentil
buildtool install sentil
```

In the Apollo dev container the flow is `aem start`, the two `buildtool` commands, then `cyber_launch start modules/sentil/monitor/launch/sentil_monitor.launch`.

### From a GitHub release

To take a tagged release without cloning, download its source archive and the binary bundle for your platform, unpack both, and drop `sentil-apollo` in as `modules/sentil`:

```
curl -L https://github.com/sedislab/SENTIL/archive/refs/tags/v0.3.0.tar.gz -o sentil.tar.gz
curl -L https://github.com/sedislab/SENTIL/releases/download/v0.3.0/sentil-0.3.0-linux-x86_64.tar.gz -o sentil-bin.tar.gz
tar -xzf sentil.tar.gz
tar -xzf sentil-bin.tar.gz
cp -r SENTIL-0.3.0/sentil-apollo modules/sentil
```

The bundle already carries the headers, the shared object, and the oracle in the layout `bzl/sentil_cpp.BUILD` expects, so point `@sentil_cpp` at the unpacked `sentil-0.3.0-linux-x86_64` directory with no further staging and run the same `buildtool build -p sentil` and `buildtool install sentil` as above.

## Documentation

The [documentation site](https://sentil.pages.dev) carries the guides and the specification syntax, and [`docs/apollo.md`](docs/apollo.md) is the integration overview. The `examples/` directory ships runnable configs, including `ego_speed_monitor.pb.txt`, `follow_distance_prstl.pb.txt`, `synthesize_control.pb.txt`, and `cbf_shield_control.pb.txt`, plus two small C++ programs, `safety_planner_subscriber.cc` and `synthesize_then_monitor.cc`.

## Contributing

With the module in `modules/sentil` and `@sentil_cpp` wired up, build and test it in the workspace:

```
buildtool build -p sentil
bazel test //modules/sentil/tests/...
```

The pull-request flow is in the repository [CONTRIBUTING.md](../CONTRIBUTING.md).

## Citation

If SENTIL is useful in your work, please cite the paper:

```bibtex
@misc{quansah2026sentilruntimeverificationtool,
    title={SENTIL: A Runtime Verification Tool for Probabilistic Temporal Logic},
    author={Paapa Kwesi Quansah and Ernest Bonnah},
    year={2026},
    eprint={2605.21676},
    archivePrefix={arXiv},
    primaryClass={cs.LO},
    url={https://arxiv.org/abs/2605.21676}
}
```

## License

SENTIL is the work of Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS lab at Baylor University. It is dual licensed under either [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at your option.