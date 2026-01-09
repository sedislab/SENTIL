"""Bring up the SENTIL control node as a managed lifecycle node."""

from launch import LaunchDescription
from launch.actions import DeclareLaunchArgument, EmitEvent, RegisterEventHandler
from launch.conditions import IfCondition
from launch.events import matches_action
from launch.substitutions import LaunchConfiguration, PathJoinSubstitution
from launch_ros.actions import LifecycleNode
from launch_ros.event_handlers import OnStateTransition
from launch_ros.events.lifecycle import ChangeState
from launch_ros.substitutions import FindPackageShare
from lifecycle_msgs.msg import Transition


def generate_launch_description():
    params_file = LaunchConfiguration("params_file")
    autostart = LaunchConfiguration("autostart")

    control = LifecycleNode(
        package="sentil_ros",
        executable="sentil_control",
        name="sentil_control",
        namespace="",
        output="screen",
        parameters=[params_file],
    )

    configure = EmitEvent(
        event=ChangeState(
            lifecycle_node_matcher=matches_action(control),
            transition_id=Transition.TRANSITION_CONFIGURE,
        )
    )

    activate = RegisterEventHandler(
        OnStateTransition(
            target_lifecycle_node=control,
            goal_state="inactive",
            entities=[
                EmitEvent(
                    event=ChangeState(
                        lifecycle_node_matcher=matches_action(control),
                        transition_id=Transition.TRANSITION_ACTIVATE,
                    )
                )
            ],
        ),
        condition=IfCondition(autostart),
    )

    return LaunchDescription(
        [
            DeclareLaunchArgument(
                "params_file",
                default_value=PathJoinSubstitution(
                    [FindPackageShare("sentil_ros"), "config", "control_params.yaml"]
                ),
                description="Path to the controller's parameter file.",
            ),
            DeclareLaunchArgument(
                "autostart",
                default_value="true",
                description="Configure and activate the node on launch.",
            ),
            activate,
            control,
            configure,
        ]
    )