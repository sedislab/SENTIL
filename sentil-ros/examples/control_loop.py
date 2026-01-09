"""Close the loop around the SENTIL control node on a double integrator.

Run the control node first (it synthesizes control from the spec in control_params.yaml):

    ros2 launch sentil_ros sentil_control.launch.py

Then run this. It plays the role of the plant: it publishes the current state, reads the
control command the node synthesizes, integrates the double integrator forward, and
repeats. The position starts at 0 and the spec asks it to stay in [1, 9]; you should see
the controller drive it into the band and hold it there.
"""

import rclpy
from rclpy.node import Node
from std_msgs.msg import Float64MultiArray

from sentil_ros.msg import Control


class Plant(Node):
    def __init__(self):
        super().__init__("double_integrator_plant")
        self.cmd = None
        self.create_subscription(Control, "/system/command", self._on_cmd, 10)
        self.state_pub = self.create_publisher(Float64MultiArray, "/system/state", 10)

    def _on_cmd(self, msg):
        self.cmd = msg

    def run(self, steps=80, dt=0.1):
        pos, vel = 0.0, 0.0
        for _ in range(steps):
            msg = Float64MultiArray()
            msg.data = [pos, vel]
            self.state_pub.publish(msg)
            for _ in range(8):
                rclpy.spin_once(self, timeout_sec=0.01)
            u = self.cmd.input[0] if (self.cmd and self.cmd.input) else 0.0
            pos += vel * dt + 0.5 * u * dt * dt
            vel += u * dt
            print("pos {:6.3f}  vel {:6.3f}  u {:6.3f}".format(pos, vel, u))


def main():
    rclpy.init()
    plant = Plant()
    try:
        plant.run()
    finally:
        plant.destroy_node()
        if rclpy.ok():
            rclpy.shutdown()


if __name__ == "__main__":
    main()