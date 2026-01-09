# Case study: autonomous driving in CARLA

An autonomous vehicle has to keep its lane, keep clear of traffic, and not hit anyone, all from a moving platform in a world full of other agents whose next move it cannot know for certain. This case study runs SENTIL as the runtime monitor on a vehicle driving through the CARLA simulator. It comes in two parts: an offline study that measures how fast the engine is and shows where its probabilistic layer beats a deterministic check, and an online run where the real `sentil_ros` node monitors a recorded drive in real time and the verdicts are drawn over the camera as a video.

Recording from CARLA needs a GPU; everything downstream runs on plain CPU and reproduces from the committed artifacts.

## Part 1: offline engine study

`record_drive.py` drives an ego vehicle under the CARLA Traffic Manager and records the monitored signals each frame: lateral lane error, distance to the nearest vehicle and pedestrian, speed, the ego pose, and the nearest pedestrian's relative motion. `monitor_drive.py` reads that trace (`results/drive.json`) and checks it with SENTIL through the Python API, writing `results/verdicts.json` and `results/drive.png`.

The specification:

```
always (|lateral_error| < 0.3)           lane keeping, within 0.3 m
always (obstacle_distance > 5.0)         clearance, 5 m from the nearest agent
always (speed < 50)                      urban speed limit, km/h
P>=0.99(always[0,10] (no collision))     collision-free over the next 10 s under
                                         uncertainty about where pedestrians go
```

Latency is what makes online monitoring viable. The three deterministic conjuncts run together on the streaming monitor at about 0.54 microseconds per sample, a sustained 1.8 million samples per second, far beyond the few hundred hertz a control loop publishes at. The probabilistic conjunct, the expensive one, runs at a median of about 0.65 ms per frame and a 99th percentile of about 0.89 ms, inside the 2 ms closed-loop deadline. For comparison, the STORM paper reports an RTAMT-based monitor at about 47 ms per frame on the same workload, which misses the deadline by more than twenty times. These figures are measured on the machine that runs the monitor, not the GPU node that runs CARLA.

The probabilistic check earns its place at the pedestrian encounter near t = 146 s. There the deterministic clearance still holds, 5.7 m to the nearest agent, so a classical monitor reports no problem. SENTIL puts the collision-free probability over the next ten seconds at essentially zero, because the pedestrian's predicted path, under its uncertainty, runs into the car's own recorded path inside the lookahead. The probabilistic verdict sat near 1.0 the whole time except at the few real encounters, where it dropped sharply, which is exactly the behavior you want from a risk monitor.

`rtamt_compare.py` runs SENTIL and RTAMT, a widely used STL monitor, head to head on this workload, in full, and writes the numbers to `results/rtamt.json`. A single simple formula understates the gap, so it measures three things. Online, with all three deterministic formulas at once: RTAMT has no multi-formula monitor and runs one per formula, while SENTIL folds them into a single streaming update, so SENTIL is about twenty times faster per frame (about 1.9 microseconds against about 40). Offline, on the actual bounded-future specs the case study uses (`always[0,10] ...`) over the whole signal: the two agree on the robustness exactly, and SENTIL is roughly eighty to a hundred and fifty times faster depending on the signal. And the probabilistic conjunct, `P>=0.95(always[0,10](...))` by sequential Monte Carlo, is something RTAMT cannot do at all: it has no probabilistic monitoring, so for the spec the vehicle actually runs there is no RTAMT verdict, while SENTIL produces one at about a tenth of a millisecond per frame for a thousand particles.

## Part 2: online monitoring through sentil_ros

This part runs the actual `sentil_ros` node, the same managed lifecycle node any ROS 2 user would deploy, against a recorded drive, and produces the video in `results/carla_drive.mp4`.

`record_carla_ros.py` drives a busy scene, Town10HD with 40 vehicles and over 80 pedestrians and an assertive ego that runs lights and follows closely, and saves the signals and the forward camera. `bag_from_capture.py` turns that into a ROS 2 bag with carla_ros_bridge-style topics: `/carla/hero/odometry` for speed, and `sensor_msgs/Range` on `/carla/hero/front_range` and `/carla/hero/pedestrian_range`. Replaying the bag through the node at the recording rate is genuine online monitoring; the node sees each message as it arrives, with no view of the future. The configuration is `sentil-ros/config/carla_verification.yaml`:

```
speed_limit:           always[0,10] (speed < 12)
following_distance:    always[0,10] (front_distance > 6)
pedestrian_clearance:  always[0,10] (pedestrian_distance > 5)
collision_risk:        P>=0.95(always[0,10] (pedestrian_distance > 4))   under sensor noise
```

The node publishes a robustness verdict per formula and, for the probabilistic one, its running probability. On this drive it flags what the assertive ego actually does: the following distance is violated in more than half the frames because the ego tailgates, in two long stretches from about 17 s to 56 s and from 66 s to 76 s, and the pedestrian clearance goes negative at the close pass in the final two seconds. The speed limit holds the whole way, but not by much; the fastest the ego gets is 11.55 m/s against the 12 m/s limit, and the margin is under 1 m/s for over half the frames. The probabilistic collision-risk estimate reads zero until its ten-second window fills, then sits at 1.0 and falls through the 0.95 threshold at that same pedestrian encounter, bottoming out near 0.04. The video overlays all of this, the node's real output, frame by frame.

The probability row is drawn amber while the window is still filling, from the `is_concrete` flag the node now publishes alongside the estimate. The committed timeline and video were captured before that flag existed, so they paint the opening ten seconds as a violation the monitor had not yet decided; a fresh capture reads it correctly.

## Run it

Offline study, from the committed trace, no GPU:

```
python experiments/carla_driving/monitor_drive.py --trace experiments/carla_driving/results/drive.json
```

Online run, on a machine with ROS 2 and the `sentil_ros` package built, replaying the committed bag through the node:

```
ros2 launch sentil_ros replay.launch.py params_file:=sentil-ros/config/carla_verification.yaml
ros2 bag play experiments/carla_driving/carla_drive_bag --clock
```

`capture_verdicts.py` logs the node's verdicts while the bag plays, and `render_video.py` draws them over the camera frames to produce the mp4. Recording a fresh drive needs the `carla` client (Python 3.7); the monitor and the rest run under any recent Python.