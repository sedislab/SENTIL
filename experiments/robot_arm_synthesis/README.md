# Case study: robot-arm control synthesis in ROS

This is the synthesis counterpart to the monitoring case studies. Instead of watching a system and reporting on it, SENTIL is in the loop: the `sentil_ros` control node synthesizes the motion that makes a robot arm satisfy a specification, and actuates it.

## What it does

A KUKA iiwa arm has to move its end effector over a target while never dropping it below the table. The end effector is modeled as a 3D double integrator (position and velocity, acceleration as the input); the control node's receding-horizon mode plans the accelerations online to satisfy

```
eventually[0,30] (over the target in x and y) and always[0,30] (0.25 < z < 1.0)
```

`arm_sim.py` is the plant and the camera. Each step it publishes the end-effector state, the control node returns the synthesized acceleration, the script integrates the end effector forward and moves the arm to the planned pose by inverse kinematics in PyBullet, and it records the scene with a heads-up display of the live verdict. The sim and the node run in one ROS 2 environment, so they discover each other on the loopback, and it needs no GPU.

## Result

The synthesized controller drives the end effector over the target and holds it there, staying above the table throughout. In the committed run it reaches the target at about 3.1 seconds and settles, and the height stays inside the safe band the whole time (the minimum end-effector height is 0.4 m, above the 0.25 m floor). The verdict on the heads-up display flips to reached and stays green. The result is in `results/arm.json` and the video in `results/arm.mp4`.

One detail worth recording, because it is the kind of thing this library exists to get right: a safety constraint written as `always (z > 0.25)` is a trap for a synthesizer, because the robustness it maximizes is `z - 0.25`, which rewards driving the arm upward without bound. Writing the constraint as a band, `0.25 < z < 1.0`, makes the robustness peak in the middle of the band, and the arm settles at a sensible height. The spec above uses the band.

## Run it

Start the control node with the arm configuration, then run the sim:

```
ros2 launch sentil_ros sentil_control.launch.py params_file:=sentil-ros/config/arm_control.yaml
python experiments/robot_arm_synthesis/arm_sim.py --out results
```

It needs `sentil_ros` built, plus PyBullet, NumPy, Pillow, and imageio, all of which sit in one ROS 2 Python environment.