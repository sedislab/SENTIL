"""Mobile-robot navigation monitored by the sentil_ros node, recorded to mp4."""

import argparse
import json
import math
import os

import numpy as np
import pybullet as p
import pybullet_data
import rclpy
from lifecycle_msgs.msg import Transition
from lifecycle_msgs.srv import ChangeState
from nav_msgs.msg import Odometry
from PIL import Image, ImageDraw, ImageFont
from rclpy.node import Node
from sensor_msgs.msg import Range

from sentil_ros.msg import Probability, Robustness

W, H = 880, 600
FENCE = 4.0  # m, matches robot_nav.yaml
OBS_R = 0.3  # m
NEAR_MISS = 0.45  # m
WAYPOINTS = np.array([[-3.6, -1.8], [-2.0, -1.1], [-0.6, -0.3], [0.5, 0.6],
                      [1.5, 0.25], [2.6, 1.0], [3.6, 1.8]])
SPECS = ["geofence", "obstacle_clearance", "speed_limit"]


def font(size):
    for path in ("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
                 "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf"):
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


class Robot(Node):
    def __init__(self):
        super().__init__("nav_robot")
        self.rob = {}
        self.prob = None
        for s in SPECS:
            self.create_subscription(Robustness, "/sentil_monitor/{}/robustness".format(s),
                                     lambda m, s=s: self.rob.__setitem__(s, m.robustness), 10)
        self.create_subscription(Probability, "/sentil_monitor/collision_risk/probability",
                                 lambda m: setattr(self, "prob", m.estimate), 10)
        self.odom_pub = self.create_publisher(Odometry, "/robot/odom", 10)
        self.range_pub = self.create_publisher(Range, "/robot/front_range", 10)

    def transition(self, tid, name):
        cli = self.create_client(ChangeState, "/sentil_monitor/change_state")
        cli.wait_for_service(timeout_sec=15.0)
        req = ChangeState.Request()
        req.transition.id = tid
        fut = cli.call_async(req)
        rclpy.spin_until_future_complete(self, fut, timeout_sec=12.0)
        print(name, "->", "ok" if (fut.result() and fut.result().success) else "FAILED")

    def publish(self, x, y, speed, clearance):
        o = Odometry()
        o.header.stamp = self.get_clock().now().to_msg()
        o.header.frame_id = "odom"
        o.pose.pose.position.x = float(x)
        o.pose.pose.position.y = float(y)
        o.twist.twist.linear.x = float(speed)
        self.odom_pub.publish(o)
        r = Range()
        r.header.stamp = o.header.stamp
        r.header.frame_id = "front_range"
        r.radiation_type = Range.INFRARED
        r.range = float(clearance)
        r.min_range = 0.0
        r.max_range = 10.0
        self.range_pub.publish(r)


def resample(n):
    """Resample the waypoints to n points spaced evenly by arc length."""
    seg = np.linalg.norm(np.diff(WAYPOINTS, axis=0), axis=1)
    cum = np.concatenate([[0], np.cumsum(seg)])
    s = np.linspace(0, cum[-1], n)
    xs = np.interp(s, cum, WAYPOINTS[:, 0])
    ys = np.interp(s, cum, WAYPOINTS[:, 1])
    return np.stack([xs, ys], axis=1)


def place_obstacles(path):
    """Place obstacles offset along the path normal at controlled clearances."""
    obs = []
    for frac, gap, side in [(0.42, NEAR_MISS, 1.0), (0.20, 1.4, -1.0), (0.74, 1.3, 1.0)]:
        i = int(len(path) * frac)
        d = path[min(i + 1, len(path) - 1)] - path[max(i - 1, 0)]
        normal = np.array([-d[1], d[0]]) / (np.linalg.norm(d) + 1e-9)
        obs.append(tuple(path[i] + side * normal * (gap + OBS_R)))
    return obs


def clearance(x, y, obstacles):
    return min(math.hypot(x - ox, y - oy) - OBS_R for ox, oy in obstacles)


def scene(obstacles):
    p.connect(p.DIRECT)
    p.setAdditionalSearchPath(pybullet_data.getDataPath())
    p.setGravity(0, 0, -9.8)
    p.loadURDF("plane.urdf")
    robot = p.loadURDF("husky/husky.urdf", [WAYPOINTS[0][0], WAYPOINTS[0][1], 0.0])
    base_z = p.getBasePositionAndOrientation(robot)[0][2]
    wheels = [j for j in range(p.getNumJoints(robot))
              if b"wheel" in p.getJointInfo(robot, j)[1]]
    pillar = p.createVisualShape(p.GEOM_CYLINDER, radius=OBS_R, length=0.8,
                                 rgbaColor=[0.85, 0.45, 0.1, 1])
    for ox, oy in obstacles:
        p.createMultiBody(baseVisualShapeIndex=pillar, basePosition=[ox, oy, 0.4])
    for axis, sign in ((0, 1), (0, -1), (1, 1), (1, -1)):
        half = [0.02, FENCE, 0.3] if axis == 0 else [FENCE, 0.02, 0.3]
        pos = [sign * FENCE, 0.0, 0.3] if axis == 0 else [0.0, sign * FENCE, 0.3]
        wall = p.createVisualShape(p.GEOM_BOX, halfExtents=half, rgbaColor=[0.3, 0.5, 0.9, 0.22])
        p.createMultiBody(baseVisualShapeIndex=wall, basePosition=pos)
    dot = p.createVisualShape(p.GEOM_SPHERE, radius=0.04, rgbaColor=[0.4, 0.7, 1.0, 0.55])
    for wx, wy in resample(40):
        p.createMultiBody(baseVisualShapeIndex=dot, basePosition=[wx, wy, 0.04])
    return robot, base_z, wheels


def render(x, y, speed, clr, t, rob, prob):
    view = p.computeViewMatrix([5.4, -6.4, 5.2], [0.1, 0.0, 0.2], [0, 0, 1])
    proj = p.computeProjectionMatrixFOV(50, W / H, 0.1, 30)
    img = p.getCameraImage(W, H, view, proj, renderer=p.ER_TINY_RENDERER)[2]
    rgb = np.reshape(img, (H, W, 4))[:, :, :3].astype(np.uint8)
    im = Image.fromarray(rgb)
    d = ImageDraw.Draw(im, "RGBA")
    d.rectangle([10, 10, 374, 210], fill=(15, 18, 24, 205))
    d.text((22, 16), "SENTIL  online monitor", font=font(20), fill=(235, 235, 235))
    d.text((22, 44), "t = {:4.1f} s   speed {:.2f} m/s   clearance {:.2f} m".format(t, speed, clr),
           font=font(13), fill=(170, 180, 195))
    labels = [("geofence", "inside geofence"), ("obstacle_clearance", "clearance > 0.6 m"),
              ("speed_limit", "speed < 2.0 m/s")]
    y0 = 74
    for key, text in labels:
        v = rob.get(key)
        if v is None:
            color, mark = (120, 120, 120), "--"
        else:
            ok = v >= 0.0
            color, mark = ((60, 200, 90) if ok else (230, 70, 60)), ("OK" if ok else "VIOLATED")
        d.ellipse([24, y0 + 5, 38, y0 + 19], fill=color)
        d.text((48, y0), "{}: {}".format(text, mark), font=font(15), fill=color)
        y0 += 28
    if prob is None:
        color, txt = (120, 120, 120), "P = --"
    else:
        color = (60, 200, 90) if prob >= 0.95 else ((235, 170, 40) if prob >= 0.85 else (230, 70, 60))
        txt = "P[clearance > 0.35] = {:.2f}  (>= 0.95)".format(prob)
    d.ellipse([24, y0 + 5, 38, y0 + 19], fill=color)
    d.text((48, y0), txt, font=font(15), fill=color)
    return np.asarray(im)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="results")
    ap.add_argument("--steps", type=int, default=150)
    ap.add_argument("--dt", type=float, default=0.1)
    args = ap.parse_args()
    os.makedirs(args.out, exist_ok=True)

    rclpy.init()
    robot_node = Robot()
    robot_node.transition(Transition.TRANSITION_CONFIGURE, "configure")
    robot_node.transition(Transition.TRANSITION_ACTIVATE, "activate")

    path = resample(args.steps)
    obstacles = place_obstacles(path)
    body, base_z, wheels = scene(obstacles)

    import imageio.v2 as imageio
    writer = imageio.get_writer(os.path.join(args.out, "nav.mp4"), fps=int(1 / args.dt),
                                quality=8, macro_block_size=8)
    min_clr, viol, rolled = 9.9, 0, 0.0
    breaches = []
    for k in range(args.steps):
        x, y = float(path[k][0]), float(path[k][1])
        if k > 0:
            step = float(np.linalg.norm(path[k] - path[k - 1]))
            heading = math.atan2(path[k][1] - path[k - 1][1], path[k][0] - path[k - 1][0])
        else:
            step, heading = 0.0, math.atan2(path[1][1] - y, path[1][0] - x)
        speed = step / args.dt
        clr = clearance(x, y, obstacles)
        min_clr = min(min_clr, clr)
        if clr < 0.6:
            breaches.append(round(k * args.dt, 2))

        robot_node.publish(x, y, speed, clr)
        for _ in range(10):
            rclpy.spin_once(robot_node, timeout_sec=0.01)

        p.resetBasePositionAndOrientation(body, [x, y, base_z],
                                          p.getQuaternionFromEuler([0, 0, heading]))
        rolled += step / 0.165
        for j in wheels:
            p.resetJointState(body, j, rolled)
        p.stepSimulation()

        if robot_node.rob.get("obstacle_clearance", 1.0) < 0.0:
            viol += 1
        writer.append_data(render(x, y, speed, clr, k * args.dt,
                                  dict(robot_node.rob), robot_node.prob))
    writer.close()

    report = {
        "case_study": "mobile_robot_navigation_monitoring",
        "obstacles": [[round(ox, 2), round(oy, 2)] for ox, oy in obstacles],
        "min_clearance_m": round(min_clr, 3),
        "first_breach_s": breaches[0] if breaches else None,
        "clearance_violation_frames": viol,
        "final_probability": robot_node.prob,
        "geofence_held": bool(robot_node.rob.get("geofence", -1) >= 0.0),
        "speed_limit_held": bool(robot_node.rob.get("speed_limit", -1) >= 0.0),
    }
    json.dump(report, open(os.path.join(args.out, "nav.json"), "w"), indent=2)
    print(json.dumps(report, indent=2))
    p.disconnect()
    robot_node.destroy_node()
    if rclpy.ok():
        rclpy.shutdown()


if __name__ == "__main__":
    main()