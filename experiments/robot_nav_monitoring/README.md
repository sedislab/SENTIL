# Case study: mobile-robot navigation monitoring in ROS

This is the monitoring counterpart to the robot-arm synthesis study. A wheeled robot drives a route through a field of obstacles inside a geofence, and the `sentil_ros` monitor node watches it online: containment, obstacle clearance, the speed limit, and a probabilistic collision risk under range-sensor noise. It is the same node used for CARLA, pointed at a different robot's topics.

## What it does

The robot publishes odometry on `/robot/odom` and a forward range on `/robot/front_range`. The monitor node checks four specifications from `sentil-ros/config/robot_nav.yaml`:

```
geofence            -4 < x < 4 and -4 < y < 4
obstacle_clearance  clearance > 0.6
speed_limit         speed < 2.0
collision_risk      P>=0.95(clearance > 0.35)   under range noise, std 0.3
```

The checks are instantaneous, so each verdict tracks the robot's current state. `nav_sim.py` is the robot and the camera. Each step it drives the robot one point along a scripted route in PyBullet, publishes the odometry and range, reads the node's verdicts back, and records the scene with a heads-up display, to mp4. The route, the node, and this script run in one ROS 2 environment and discover each other on the loopback. It needs no GPU.

## Result

The route threads between the obstacles but skirts one close, by design, so the clearance falls to 0.45 m at the near miss and the deterministic `obstacle_clearance` verdict flips to violated for that stretch, then recovers. The probabilistic `collision_risk` is the interesting one: under range noise its satisfaction probability falls below the 0.95 threshold while the nominal clearance still reads above the deterministic 0.6 m line, so it raises the alarm before the deterministic check does. That earlier warning is the case the probabilistic monitor exists for. The robot stays inside the geofence and under the speed limit throughout, so those two verdicts hold the whole run. The numbers are in `results/nav.json` and the video in `results/nav.mp4`.

The obstacles are placed relative to the route, not by hand, so the closest approach is exact and the run reproduces: the skirted obstacle sits one near-miss distance off the nearest point of the path.

## Run it

Start the monitor node with the navigation configuration, then run the sim:

```
ros2 run sentil_ros sentil_monitor --ros-args --params-file sentil-ros/config/robot_nav.yaml -p autostart:=false
python experiments/robot_nav_monitoring/nav_sim.py --out results
```

It needs `sentil_ros` built, plus PyBullet, NumPy, Pillow, and imageio, all of which sit in one ROS 2 Python environment.