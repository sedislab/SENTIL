---
id: apollo
title: Apollo
sidebar_position: 3
---

# Apollo

`sentil-apollo` integrates SENTIL with Baidu Apollo as a Cyber RT module. A monitor component watches Apollo's channels against Signal Temporal Logic and probabilistic STL specifications and publishes a verdict the planning stack can act on; a control component synthesizes a command from a specification or shields Apollo's nominal command. The compiled SENTIL core ships inside the module, so the build needs no Rust toolchain. The package README is the full reference; this page is the integration overview.

## Installing

Apollo 9.0 and newer build modules with the package method, so drop `sentil-apollo` into the workspace as `modules/sentil` and let `buildtool` build it:

```bash
buildtool build -p sentil
buildtool install sentil
```

The module links the core through a Bazel external repository `@sentil_cpp` that wraps a prebuilt `libsentil.so` and the headers; `bzl/sentil_cpp.BUILD` is its build file. Earlier Apollo releases used a raw `cc_binary` plus an edit to the top-level `BUILD.bazel`, which the package method replaces.

## Configuring the monitor

A config names the formulas and binds each variable to a channel and a field:

```protobuf
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

Each input channel carries its message type, and each field is either a `field_path` or a `builtin`. A field path resolves a nested, optionally indexed path (`pose.position.x`, `perception_obstacle[0].position.x`) against the message descriptor once at startup, so a wrong path fails when the component initializes rather than reading a silent zero on the road. A builtin computes a quantity a raw path cannot, since the nearest obstacle is not the obstacle at index 0: `NEAREST_OBSTACLE_DISTANCE`, `MIN_TTC`, and `FRONT_GAP` reduce the obstacle list.

## The component models

The default monitor is a fused component triggered by perception with localization and chassis as co-channels, so each verdict reflects a consistent message set. A timer variant reads the same channels through pull readers and emits at a fixed rate, for the case where a steady cadence matters more than reacting to each perception frame.

The verdict on `/apollo/sentil/status` carries, per formula, the robustness, a satisfied flag, an OK/WARN/ERROR/FATAL severity, and for a `P~p` formula the running probability with its Wilson interval. Planning subscribes and engages a fallback when `all_satisfied` is false or an interval drops below threshold.

## Synthesis and shielding

The control component turns a specification into a command in one of three modes. SHIELD, the default, projects Apollo's nominal command into the actuator bounds, so SENTIL guards the existing controller rather than replacing it. SYNTHESIZE makes SENTIL the controller, planning over the horizon within a per-tick deadline. ADVISORY synthesizes but publishes only advice. The component runs on a timer and its launch sets `exception_handler: respawn`, so a failure degrades rather than dies.

## Offline analysis

For the heavier algorithms a per-tick budget cannot afford, `sentil_record_analyzer` replays a recorded drive and runs SMC or SPRT over the same config:

```bash
sentil_record_analyzer --config=follow_distance_prstl.pb.txt --record=drive.record
```

GPU rare-event splitting runs over a stochastic model rather than a recorded topic stream, so it lives in the library and the CLI, not as a Cyber RT service.