# sentil_ros

Monitor ROS 2 topic streams against Signal Temporal Logic and probabilistic STL specifications, online and in real time. You write a small YAML configuration that names each formula and binds its variables to topics and message fields; the node subscribes to those topics, evaluates the formulas as messages arrive, and publishes a verdict per formula along with a standard diagnostic. It is built on the SENTIL engine and works with messages of any type, resolved at runtime, so you monitor your existing topics without changing them.

This package targets ROS 2 Humble.

## Why it is built the way it is

The monitor is a managed lifecycle node, so it can be configured, paused, resumed, and torn down under supervision, and it only publishes while active. It is a composable component, so you can load it into the same process as the nodes it watches for zero-copy, in-process delivery. It subscribes generically and matches each subscription's QoS to the publisher, so a best-effort sensor stream or a latched topic is never silently dropped, the most common way a topic monitor goes quietly blind.

## Install

From a release, through rosdep and your distribution's package manager once it is published to rosdistro:

```
sudo apt install ros-humble-sentil-ros
```

From source, in a colcon workspace with the SENTIL C++ package (`SentilCpp`) available:

```
cd ~/ros2_ws
colcon build --packages-select sentil_ros
source install/setup.bash
```

## Run it

```
ros2 launch sentil_ros sentil_monitor.launch.py params_file:=/path/to/your.yaml
```

That configures and activates the node on launch. Pass `autostart:=false` to drive the lifecycle yourself with `ros2 lifecycle set /sentil_monitor configure` then `activate`. To monitor a recorded bag, use `replay.launch.py` and play the bag with `--clock`:

```
ros2 launch sentil_ros replay.launch.py params_file:=/path/to/your.yaml
ros2 bag play --clock your_bag
```

## Configuration

A formula is configured under `formulas.<id>`. `config/example_params.yaml` is a complete, commented starting point. The fields:

| Parameter | Meaning |
| --- | --- |
| `formulas` | the list of formula ids to monitor |
| `formulas.<id>.formula` | the STL or PrSTL formula, e.g. `always[0,10] (speed < 30)` or `P>=0.95(always[0,10] (gap > 5))` |
| `formulas.<id>.spec` | a premade specification name, used instead of a raw formula |
| `formulas.<id>.variant` | a variant of the named spec |
| `formulas.<id>.verification.method` | `robustness` (deterministic), `smc`, `sprt`, or `automatic` |
| `formulas.<id>.signal_names` | the variable names the formula reads |
| `formulas.<id>.variables.<v>.topic` | the topic carrying variable `v` |
| `formulas.<id>.variables.<v>.field` | a dotted, optionally indexed path to the scalar, e.g. `twist.twist.linear.x` or `ranges[0]` |
| `formulas.<id>.variables.<v>.type` | the message type `pkg/msg/Name`, if it cannot be inferred from the topic |
| `formulas.<id>.variables.<v>.noise.type` | a noise family (`gaussian`, `uniform`, ...) for a probabilistic formula |
| `formulas.<id>.config.particles` | the ensemble size for a probabilistic estimate |
| `formulas.<id>.config.confidence` | the confidence level |

## Topics and diagnostics

Per formula `<id>`, the node publishes:

- `~/<id>/robustness` (`sentil_ros/msg/Robustness`): the signed robustness, whether the verdict is concrete, and the interval bounds.

It also publishes a `diagnostic_msgs/DiagnosticStatus` per formula on `/diagnostics`, so the monitor shows up in `rqt_robot_monitor` and the diagnostic aggregator: satisfied is OK, violated is ERROR, an undecided probabilistic verdict is WARN, and no data yet is STALE.

## CARLA example

`examples/carla_monitor.launch.py` monitors a CARLA ego vehicle against `config/carla_verification.yaml`, which binds to the standard `carla_ros_bridge` topics. Because the monitor consumes only those ROS topics, it works the same whether CARLA runs natively, in Docker, or in an Apptainer image, on the same machine or a remote one. Run CARLA and the bridge however you like, then:

```
ros2 launch sentil_ros carla_monitor.launch.py
```

Or have the launch start the bridge for you, aimed at the server:

```
ros2 launch sentil_ros carla_monitor.launch.py launch_bridge:=true carla_host:=192.168.1.50 carla_port:=2000
```

## Credits and license

SENTIL is the work of Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab. Dual licensed under MIT or Apache-2.0.