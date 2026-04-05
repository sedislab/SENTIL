"""The monitor gathers a whole instant before it evaluates."""

import os
import time
import unittest

import launch
import launch_testing
import launch_testing.actions
import pytest
import rclpy
from builtin_interfaces.msg import Time
from geometry_msgs.msg import PointStamped
from launch.events import matches_action
from launch_ros.actions import LifecycleNode
from launch_ros.event_handlers import OnStateTransition
from launch_ros.events.lifecycle import ChangeState
from lifecycle_msgs.msg import State, Transition
from lifecycle_msgs.srv import GetState

from sentil_ros.msg import Robustness

PARAMS = os.path.join(os.path.dirname(__file__), "pairing_params.yaml")

X = [-3.0, 8.0, 8.0, 8.0, 8.0]
Y = [-5.0, -5.0, 9.0, -5.0, -5.0]
PAIRED = -3.0

@pytest.mark.launch_test
def generate_test_description():
    monitor = LifecycleNode(
        package="sentil_ros",
        executable="sentil_monitor",
        name="sentil_monitor",
        namespace="",
        output="screen",
        parameters=[PARAMS],
    )
    configure = launch.actions.EmitEvent(
        event=ChangeState(
            lifecycle_node_matcher=matches_action(monitor),
            transition_id=Transition.TRANSITION_CONFIGURE,
        )
    )
    activate = launch.actions.RegisterEventHandler(
        OnStateTransition(
            target_lifecycle_node=monitor,
            goal_state="inactive",
            entities=[
                launch.actions.EmitEvent(
                    event=ChangeState(
                        lifecycle_node_matcher=matches_action(monitor),
                        transition_id=Transition.TRANSITION_ACTIVATE,
                    )
                )
            ],
        )
    )
    return launch.LaunchDescription(
        [activate, monitor, configure, launch_testing.actions.ReadyToTest()]
    )

class TestPairedInstants(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        rclpy.init()

    @classmethod
    def tearDownClass(cls):
        rclpy.shutdown()

    def setUp(self):
        self.node = rclpy.create_node("sentil_pairing_test")

    def tearDown(self):
        self.node.destroy_node()

    def test_verdict_matches_the_offline_paired_value(self):
        self._wait_until_active()
        x = self.node.create_publisher(PointStamped, "/probe/x", 10)
        y = self.node.create_publisher(PointStamped, "/probe/y", 10)
        seen = []
        self.node.create_subscription(
            Robustness, "/sentil_monitor/nested/robustness", seen.append, 10
        )
        self._wait(
            lambda: x.get_subscription_count() > 0 and y.get_subscription_count() > 0,
            15.0,
            "the monitor never subscribed to /probe/x and /probe/y",
        )

        for k in range(len(X)):
            self._publish(x, k + 1, X[k])
            self._spin(0.1)
            self._publish(y, k + 1, Y[k])
            self._spin(0.3)
        self._wait(
            lambda: len(seen) >= len(X),
            10.0,
            "the monitor published fewer verdicts than the instants it was fed",
        )

        self.assertEqual(len(seen), len(X))
        for k, verdict in enumerate(seen[:-1]):
            self.assertFalse(verdict.is_concrete, "instant {} settled early".format(k))
        self.assertEqual(seen[2].robustness_max, PAIRED)
        settled = seen[-1]
        self.assertTrue(settled.is_concrete, "the last instant never settled")
        self.assertEqual(settled.robustness, PAIRED)
        self.assertEqual(settled.robustness_min, PAIRED)
        self.assertEqual(settled.robustness_max, PAIRED)

    def _publish(self, publisher, stamp, value):
        msg = PointStamped()
        msg.header.stamp = Time(sec=stamp, nanosec=0)
        msg.header.frame_id = "probe"
        msg.point.x = value
        publisher.publish(msg)

    def _spin(self, seconds):
        end = time.time() + seconds
        while time.time() < end:
            rclpy.spin_once(self.node, timeout_sec=0.02)

    def _wait(self, ready, seconds, message):
        deadline = time.time() + seconds
        while time.time() < deadline:
            if ready():
                return
            self._spin(0.1)
        self.fail(message)

    def _wait_until_active(self):
        client = self.node.create_client(GetState, "/sentil_monitor/get_state")
        deadline = time.time() + 30.0
        while time.time() < deadline:
            if client.service_is_ready():
                future = client.call_async(GetState.Request())
                rclpy.spin_until_future_complete(self.node, future, timeout_sec=2.0)
                result = future.result()
                if result and result.current_state.id == State.PRIMARY_STATE_ACTIVE:
                    return
            self._spin(0.2)
        self.fail("the monitor node never became active")