"""Robot-arm reach-and-stay-safe synthesis, driven by the sentil_ros control node."""

import argparse
import json
import os

import numpy as np
import pybullet as p
import pybullet_data
import rclpy
from lifecycle_msgs.msg import Transition
from lifecycle_msgs.srv import ChangeState
from PIL import Image, ImageDraw, ImageFont
from rclpy.node import Node
from std_msgs.msg import Float64MultiArray

from sentil_ros.msg import Control

W, H = 800, 600
TARGET = np.array([0.4, 0.4, 0.7])
TARGET_HALF = 0.08
FLOOR = 0.25
EE_LINK = 6


def font(size):
    for path in ("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
                 "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf"):
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


class Plant(Node):
    def __init__(self):
        super().__init__("arm_plant")
        self.cmd = None
        self.create_subscription(Control, "/arm/ee_command", self._cmd, 10)
        self.state_pub = self.create_publisher(Float64MultiArray, "/arm/ee_state", 10)

    def _cmd(self, m):
        self.cmd = m

    def transition(self, tid, name):
        cli = self.create_client(ChangeState, "/sentil_control/change_state")
        cli.wait_for_service(timeout_sec=15.0)
        req = ChangeState.Request()
        req.transition.id = tid
        fut = cli.call_async(req)
        rclpy.spin_until_future_complete(self, fut, timeout_sec=12.0)
        ok = bool(fut.result() and fut.result().success)
        print(name, "->", "ok" if ok else "FAILED")
        return ok


def scene():
    p.connect(p.DIRECT)
    p.setAdditionalSearchPath(pybullet_data.getDataPath())
    p.setGravity(0, 0, -9.8)
    p.loadURDF("plane.urdf")
    arm = p.loadURDF("kuka_iiwa/model.urdf", [0, 0, 0], useFixedBase=True)
    slab = p.createVisualShape(p.GEOM_BOX, halfExtents=[0.6, 0.6, 0.002],
                               rgbaColor=[0.8, 0.2, 0.2, 0.35])
    p.createMultiBody(baseVisualShapeIndex=slab, basePosition=[0.3, 0.1, FLOOR])
    tgt = p.createVisualShape(p.GEOM_BOX, halfExtents=[TARGET_HALF] * 3,
                              rgbaColor=[0.2, 0.8, 0.3, 0.4])
    p.createMultiBody(baseVisualShapeIndex=tgt, basePosition=TARGET.tolist())
    return arm


def render(arm, ee, vel, accel, t, reached, safe):
    view = p.computeViewMatrix([1.6, -1.1, 1.25], [0.3, 0.1, 0.5], [0, 0, 1])
    proj = p.computeProjectionMatrixFOV(55, W / H, 0.1, 10)
    img = p.getCameraImage(W, H, view, proj, renderer=p.ER_TINY_RENDERER)[2]
    rgb = np.reshape(img, (H, W, 4))[:, :, :3].astype(np.uint8)
    im = Image.fromarray(rgb)
    d = ImageDraw.Draw(im, "RGBA")
    d.rectangle([10, 10, 360, 168], fill=(15, 18, 24, 205))
    d.text((22, 18), "SENTIL synthesis: robot arm", font=font(20), fill=(235, 235, 235))
    d.text((22, 44), "reach target and stay above the table", font=font(13), fill=(170, 180, 195))
    dist = float(np.linalg.norm(ee[:2] - TARGET[:2]))
    rows = [
        ("t = {:4.1f} s   ee z = {:.2f} m".format(t, ee[2]), (200, 205, 215)),
        ("synth accel  [{:+.2f} {:+.2f} {:+.2f}]".format(*accel), (150, 200, 235)),
        ("reach target: {}  (d={:.2f} m)".format("REACHED" if reached else "...", dist),
         (60, 200, 90) if reached else (200, 205, 215)),
        ("above table (z>0.25): {}".format("OK" if safe else "VIOLATED"),
         (60, 200, 90) if safe else (230, 70, 60)),
    ]
    y = 74
    for text, color in rows:
        d.text((22, y), text, font=font(16), fill=color)
        y += 23
    return np.asarray(im)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="results")
    ap.add_argument("--steps", type=int, default=90)
    ap.add_argument("--dt", type=float, default=0.1)
    args = ap.parse_args()
    os.makedirs(args.out, exist_ok=True)

    rclpy.init()
    plant = Plant()
    plant.transition(Transition.TRANSITION_CONFIGURE, "configure")
    plant.transition(Transition.TRANSITION_ACTIVATE, "activate")

    arm = scene()
    ee = np.array([0.5, -0.3, 0.4])
    vel = np.zeros(3)
    frames, traj = [], []
    reached_at = None
    min_z = float(ee[2])
    import imageio.v2 as imageio
    writer = imageio.get_writer(os.path.join(args.out, "arm.mp4"), fps=int(1 / args.dt),
                                quality=8, macro_block_size=8)
    for k in range(args.steps):
        msg = Float64MultiArray()
        msg.data = [float(ee[0]), float(ee[1]), float(ee[2]),
                    float(vel[0]), float(vel[1]), float(vel[2])]
        plant.state_pub.publish(msg)
        for _ in range(10):
            rclpy.spin_once(plant, timeout_sec=0.01)
        accel = np.array(plant.cmd.input[:3]) if (plant.cmd and len(plant.cmd.input) >= 3) \
            else np.zeros(3)
        ee = ee + vel * args.dt + 0.5 * accel * args.dt * args.dt
        vel = vel + accel * args.dt
        joints = p.calculateInverseKinematics(arm, EE_LINK, ee.tolist())
        for j in range(min(len(joints), p.getNumJoints(arm))):
            p.resetJointState(arm, j, joints[j])
        p.stepSimulation()
        reached = bool(abs(ee[0] - TARGET[0]) < TARGET_HALF and abs(ee[1] - TARGET[1]) < TARGET_HALF)
        safe = bool(ee[2] > FLOOR)
        if reached and reached_at is None:
            reached_at = round(k * args.dt, 2)
        min_z = min(min_z, float(ee[2]))
        frames.append(render(arm, ee, vel, accel, k * args.dt, reached, safe))
        traj.append([round(float(ee[0]), 3), round(float(ee[1]), 3), round(float(ee[2]), 3)])
    for fr in frames:
        writer.append_data(fr)
    writer.close()

    report = {
        "case_study": "robot_arm_reach_and_stay_safe_synthesis",
        "spec": "eventually[0,30] (over target in x,y) and always[0,30] (0.25 < z < 1.0)",
        "target": TARGET.tolist(),
        "reached": reached_at is not None,
        "reached_at_s": reached_at,
        "min_ee_z": round(float(min_z), 3),
        "stayed_above_table": bool(min_z > FLOOR),
        "final_ee": traj[-1],
    }
    json.dump(report, open(os.path.join(args.out, "arm.json"), "w"), indent=2)
    print(json.dumps(report, indent=2))
    p.disconnect()
    plant.destroy_node()
    if rclpy.ok():
        rclpy.shutdown()


if __name__ == "__main__":
    main()