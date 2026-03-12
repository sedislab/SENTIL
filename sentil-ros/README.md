<div align="center">

# SENTIL

#### The ROS 2 nodes for Probabilistic Signal Temporal Logic

[![ROS 2](https://img.shields.io/badge/ROS%202-Humble%20%7C%20Jazzy%20%7C%20Rolling-blue.svg)](https://docs.ros.org)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

ROS 2 nodes for the [`sentil`](../sentil-core) engine. You write a small YAML file that names each formula and binds its variables to topics and message fields; the node subscribes to those topics, evaluates the formulas as messages arrive, and publishes a verdict per formula. Messages of any type resolve at runtime, so you watch topics you already have without touching them.

Two managed lifecycle nodes ship in the package. `sentil_monitor` watches topics against a specification and reports how close the system is to breaking it. `sentil_control` runs the synthesis subsystem the other way around, turning a specification into a control input and actuating it on a topic. Both are composable components, so you can load them into the same process as the nodes they watch. The package builds against Humble, Jazzy, and Rolling.

## Your first monitor

A monitor is a YAML file and a launch command. Name the formulas, bind each variable to a topic and a field, and pick deterministic or probabilistic checking.

```yaml
# speed.yaml
sentil_monitor:
  ros__parameters:
    formulas: ["speed_limit"]
    formulas.speed_limit:
      formula: "always[0,10] (speed < 30.0)"
      verification:
        method: "robustness"
      signal_names: ["speed"]
      variables:
        speed:
          topic: "/vehicle/odom"
          field: "twist.twist.linear.x"
```

```bash
ros2 launch sentil_ros sentil_monitor.launch.py params_file:=speed.yaml
```

The node subscribes to `/vehicle/odom`, pulls `twist.twist.linear.x` out of each message, and publishes a `sentil_ros/msg/Robustness` on `~/speed_limit/robustness`.

```bash
ros2 topic echo /sentil_monitor/speed_limit/robustness
```

The `robustness` field is the signed margin. It stays positive while every reading in the trailing ten-second window sits under 30 m/s, and a reading of 32 makes that sample's margin `-2.0` and pulls the window negative by the size of the overshoot. `is_concrete` says whether the value is final or still an interval bounded by `robustness_min` and `robustness_max` while the window fills.

## Running it as a lifecycle node

`sentil_monitor` is a managed lifecycle node, so it configures, activates, deactivates, and cleans up under supervision, and it publishes only while active. The launch file configures and activates it for you. Pass `autostart:=false` to drive the transitions yourself:

```bash
ros2 launch sentil_ros sentil_monitor.launch.py params_file:=speed.yaml autostart:=false
ros2 lifecycle set /sentil_monitor configure
ros2 lifecycle set /sentil_monitor activate
```

Each subscription matches its publisher's QoS, so a best-effort sensor stream or a latched topic is not silently dropped. The node also publishes a `diagnostic_msgs/DiagnosticStatus` per formula on `/diagnostics`, so a monitor shows up in `rqt_robot_monitor` and the diagnostic aggregator. Satisfied is `OK`, violated is `ERROR`, an undecided probabilistic verdict is `WARN`, and a formula still waiting on data is `STALE`.

To replay a recorded bag against the same config, launch with the clock wired up and play the bag:

```bash
ros2 launch sentil_ros replay.launch.py params_file:=speed.yaml
ros2 bag play --clock your_bag
```

## Probabilistic monitoring

A `P~p` operator asks whether a formula holds with at least (or at most) probability `p`. Give the variable a noise model and set the method to `smc`; the node lifts each reading into an ensemble, evaluates the formula across it, and publishes the running probability with a Wilson interval.

```yaml
# gap.yaml
sentil_monitor:
  ros__parameters:
    formulas: ["following_distance"]
    formulas.following_distance:
      formula: "P>=0.95(always[0,10] (gap > 5.0))"
      verification:
        method: "smc"
      signal_names: ["gap"]
      variables:
        gap:
          topic: "/perception/lead_vehicle"
          field: "range"
          noise:
            type: "gaussian"
            mean: 0.0
            std_dev: 0.2
      config:
        particles: 1000
        confidence: 0.95
```

This publishes a `sentil_ros/msg/Probability` on `~/following_distance/probability` next to the robustness topic: the `estimate`, how many of the `samples` trajectories currently satisfy the formula, and `ci_lower` and `ci_upper` at `ci_confidence`.

The noise family is set by `noise.type`, one of `gaussian` (`mean`, `std_dev`), `uniform` (`low`, `high`), `log_normal` (`mu`, `sigma`), `exponential` (`rate`), `gamma` (`shape`, `scale`), `beta` (`alpha`, `beta`), or `none`, each reading its own parameters under `noise`.

## The specification library

The premade specifications are part of the engine, so you can name one in a config instead of writing a formula: set `spec:` to a library name, with an optional `variant:` and `spec_params`, in place of `formula:`. To inspect one at runtime, call the `~/get_spec_info` service (`sentil_ros/srv/GetSpecInfo`), which returns a spec's resolved deterministic and probabilistic formulas, its parameters as JSON, and its variants:

```bash
ros2 service call /sentil_monitor/get_spec_info sentil_ros/srv/GetSpecInfo "{spec_name: 'controls/overshoot'}"
```

Browse the library under [`specifications/`](../specifications).

## CARLA example

`examples/carla_monitor.launch.py` checks a CARLA ego vehicle against `config/carla_verification.yaml`, which binds four formulas to the standard `carla_ros_bridge` topics: a speed limit, a following distance, pedestrian clearance, and a probabilistic collision-risk bound. Start CARLA and the bridge however you run them, then:

```bash
ros2 launch sentil_ros carla_monitor.launch.py
```

Or have the launch file start the bridge for you, aimed at the server:

```bash
ros2 launch sentil_ros carla_monitor.launch.py launch_bridge:=true carla_host:=192.168.1.50 carla_port:=2000
```

## Synthesizing control

`sentil_control` runs the synthesis subsystem as a node, turning a specification into a control input on a topic. It is the same lifecycle component shape as the monitor, and the `mode` parameter picks what it does:

- `receding_horizon`: an online controller that plans over a short horizon each step and emits the first input within a hard deadline.
- `open_loop`: offline trajectory synthesis, an input sequence that satisfies the spec, stepped out in real time.
- `safety_filter`: a control-barrier shield that takes a nominal command and returns the closest input that keeps the bounds and barriers.
- `witness`: counterexample search, an input sequence that violates the spec, published for replay.
- `chance`: a chance-constraint check that estimates whether the spec holds with at least the target probability under Gaussian process noise, reported once with the estimate and its lower bound.

The linear state-space model, the spec, the input bounds, and the mode all come from the config; `config/control_params.yaml` is a double-integrator example and `config/arm_control.yaml` drives a robot-arm end effector. The spec sits under a `spec` namespace here: a raw formula under `spec.formula`, or a library spec under `spec.name` with an optional `spec.variant`. The state arrives on a `std_msgs/Float64MultiArray` and the command goes out as a `sentil_ros/msg/Control`, with a plain `Float64MultiArray` alongside. `open_loop` and `witness` synthesize a whole sequence and carry a real `robustness` and `holds`; the online `receding_horizon` and `safety_filter` modes return an input without a verdict, reporting `robustness` as NaN and `feasible` true.

```bash
ros2 launch sentil_ros sentil_control.launch.py
```

`examples/control_loop.py` closes the loop on a double integrator: it publishes the state, applies each command, and drives the position into the band the spec asks for.

## Performance

The streaming and online numbers, measured against other tools, are in [`benchmarks/`](../benchmarks), and every reproduced figure is in [`docs/CLAIMS.md`](../docs/CLAIMS.md).

## Install

### From your distribution

Install the released package through apt (or your distribution's package manager):

```bash
curl -sLf 'https://dl.cloudsmith.io/public/sedislab/sentil/cfg/setup/bash.deb.sh' | sudo bash
sudo apt install ros-$ROS_DISTRO-sentil-ros
```

Substitute `<distro_name>` for your distribution (`jazzy`, `rolling`, `humble`).

### From source

The node links the SENTIL C++ package (`SentilCpp`), which sits on top of the compiled core (`libsentil`). Install both onto a prefix through the [C++ package](../sentil-cpp); a distribution package, the prebuilt bundle, or a source build each leave a prefix with the CMake config and the library. Point colcon at that prefix, and keep `libsentil` on the loader path:

```bash
cd ~/ros2_ws
export SENTIL_PREFIX=/path/to/sentil-prefix
colcon build --packages-select sentil_ros --cmake-args -DCMAKE_PREFIX_PATH="$SENTIL_PREFIX"
export LD_LIBRARY_PATH="$SENTIL_PREFIX/lib:$LD_LIBRARY_PATH"
source install/setup.bash
```

### From a release archive

The same build without the clone, on any machine with ROS 2. Download the source archive for the tag, extract it into your workspace `src/`, and build against the same `SENTIL_PREFIX`.

On Linux or macOS:

```bash
cd ~/ros2_ws/src
curl -L https://github.com/sedislab/SENTIL/archive/refs/tags/v0.3.0.tar.gz | tar xz
cd ~/ros2_ws
colcon build --packages-select sentil_ros --cmake-args -DCMAKE_PREFIX_PATH="$SENTIL_PREFIX"
source install/setup.bash
```

On Windows, from a command prompt with ROS 2 sourced:

```bash
cd %USERPROFILE%\ros2_ws\src
curl -L -o sentil.zip https://github.com/sedislab/SENTIL/archive/refs/tags/v0.3.0.zip
tar xf sentil.zip
cd %USERPROFILE%\ros2_ws
colcon build --packages-select sentil_ros --cmake-args -DCMAKE_PREFIX_PATH=%SENTIL_PREFIX%
call install\setup.bat
```

## Contributing

With `SentilCpp` on `SENTIL_PREFIX` (see the build above), build and test this package on its own:

```bash
colcon build --packages-select sentil_ros --cmake-args -DCMAKE_PREFIX_PATH="$SENTIL_PREFIX"
colcon test --packages-select sentil_ros --return-code-on-test-failure
```

The pull-request flow is in the repository [CONTRIBUTING.md](../CONTRIBUTING.md).

## Documentation

The [documentation site](https://sentil.pages.dev) carries the ROS guide, the formula syntax, and the long-form [tutorial](https://sentil.pages.dev/docs/tutorial). The `config/` and `examples/` directories ship the configs and launch files shown here, plus the mobile-robot monitor in `config/robot_nav.yaml` and the robot-arm controller in `config/arm_control.yaml`.

## Citation

If SENTIL is useful in your work, please cite the paper:

```bibtex
@misc{quansah2026sentilruntimeverificationtool,
    title={SENTIL: A Runtime Verification Tool for Probabilistic Signal Temporal Logic},
    author={Paapa Kwesi Quansah and Ernest Bonnah},
    year={2026},
    eprint={2605.21676},
    archivePrefix={arXiv},
    primaryClass={cs.LO},
    url={https://arxiv.org/abs/2605.21676}
}
```

## License

SENTIL is by Paapa Kwesi Quansah and Ernest Bonnah at the SEDIS lab, Baylor University. It is dual licensed under either [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at your option.