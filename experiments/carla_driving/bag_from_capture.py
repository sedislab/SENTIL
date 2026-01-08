"""Turn a CARLA capture into a ROS 2 bag the sentil_ros node can monitor."""

import argparse
import json
import math
import os

import rclpy.serialization as ser
import rosbag2_py
from builtin_interfaces.msg import Time
from nav_msgs.msg import Odometry
from sensor_msgs.msg import Range


def stamp(t):
    s = Time()
    s.sec = int(t)
    s.nanosec = int(round((t - int(t)) * 1e9))
    return s


def topic(name, type_name):
    return rosbag2_py.TopicMetadata(name=name, type=type_name, serialization_format="cdr")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--capture", default="capture")
    ap.add_argument("--bag", default="carla_drive_bag")
    args = ap.parse_args()

    with open(os.path.join(args.capture, "capture.json")) as f:
        cap = json.load(f)
    records = cap["records"]

    writer = rosbag2_py.SequentialWriter()
    writer.open(
        rosbag2_py.StorageOptions(uri=args.bag, storage_id="sqlite3"),
        rosbag2_py.ConverterOptions("", ""),
    )
    writer.create_topic(topic("/carla/hero/odometry", "nav_msgs/msg/Odometry"))
    writer.create_topic(topic("/carla/hero/front_range", "sensor_msgs/msg/Range"))
    writer.create_topic(topic("/carla/hero/pedestrian_range", "sensor_msgs/msg/Range"))

    for r in records:
        t = r["t"]
        st = stamp(t)
        ns = int(round(t * 1e9))

        odom = Odometry()
        odom.header.stamp = st
        odom.header.frame_id = "map"
        odom.child_frame_id = "hero"
        odom.pose.pose.position.x = float(r["x"])
        odom.pose.pose.position.y = float(r["y"])
        odom.pose.pose.orientation.z = math.sin(r["yaw"] / 2.0)
        odom.pose.pose.orientation.w = math.cos(r["yaw"] / 2.0)
        odom.twist.twist.linear.x = float(r["vx"])
        writer.write("/carla/hero/odometry", ser.serialize_message(odom), ns)

        front = Range()
        front.header.stamp = st
        front.header.frame_id = "hero/front_range"
        front.radiation_type = Range.INFRARED
        front.field_of_view = 0.2
        front.min_range = 0.0
        front.max_range = 100.0
        front.range = float(r["front_distance"])
        writer.write("/carla/hero/front_range", ser.serialize_message(front), ns)

        ped = Range()
        ped.header.stamp = st
        ped.header.frame_id = "hero/pedestrian_range"
        ped.radiation_type = Range.INFRARED
        ped.field_of_view = 3.14
        ped.min_range = 0.0
        ped.max_range = 100.0
        ped.range = float(r["pedestrian_distance"])
        writer.write("/carla/hero/pedestrian_range", ser.serialize_message(ped), ns)

    del writer
    print("wrote bag {} with {} frames over {:.1f}s".format(
        args.bag, len(records), records[-1]["t"]))


if __name__ == "__main__":
    main()