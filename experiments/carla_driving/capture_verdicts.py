"""Record the sentil_ros node's verdicts while a bag replays, for the video overlay."""

import argparse
import json

import rclpy
from rclpy.node import Node

from sentil_ros.msg import Probability, Robustness


class VerdictLogger(Node):
    def __init__(self, node_name, ids, out):
        super().__init__("verdict_logger")
        self.out = out
        self.records = []
        for fid in ids:
            self.create_subscription(
                Robustness, "/{}/{}/robustness".format(node_name, fid),
                self._robustness_cb(fid), 50)
            self.create_subscription(
                Probability, "/{}/{}/probability".format(node_name, fid),
                self._probability_cb(fid), 50)
        self.create_timer(1.0, self._flush)

    @staticmethod
    def _t(stamp):
        return stamp.sec + stamp.nanosec * 1e-9

    def _robustness_cb(self, fid):
        def cb(msg):
            self.records.append({"t": self._t(msg.header.stamp), "id": fid, "kind": "robustness",
                                 "value": msg.robustness, "concrete": bool(msg.is_concrete)})
        return cb

    def _probability_cb(self, fid):
        def cb(msg):
            self.records.append({"t": self._t(msg.header.stamp), "id": fid, "kind": "probability",
                                 "estimate": msg.estimate, "samples": int(msg.samples),
                                 "concrete": bool(msg.is_concrete)})
        return cb

    def _flush(self):
        with open(self.out, "w") as f:
            json.dump(self.records, f)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--node", default="sentil_monitor")
    ap.add_argument("--ids", nargs="+",
                    default=["speed_limit", "following_distance", "collision_risk"])
    ap.add_argument("--out", default="verdicts.json")
    args = ap.parse_args()

    rclpy.init()
    logger = VerdictLogger(args.node, args.ids, args.out)
    try:
        rclpy.spin(logger)
    except KeyboardInterrupt:
        pass
    finally:
        logger._flush()
        logger.destroy_node()
        if rclpy.ok():
            rclpy.shutdown()


if __name__ == "__main__":
    main()