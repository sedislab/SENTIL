"""Bring up the SENTIL monitor as a managed lifecycle node.

By default it configures and activates on launch, so a single command starts
monitoring; set autostart:=false to drive the lifecycle transitions yourself.
"""

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
    use_sim_time = LaunchConfiguration("use_sim_time")
    autostart = LaunchConfiguration("autostart")

    monitor = LifecycleNode(
        package="sentil_ros",
        executable="sentil_monitor",
        name="sentil_monitor",
        namespace="",
        output="screen",
        parameters=[params_file, {"use_sim_time": use_sim_time}],
    )

    configure = EmitEvent(
        event=ChangeState(
            lifecycle_node_matcher=matches_action(monitor),
            transition_id=Transition.TRANSITION_CONFIGURE,
        )
    )

    activate = RegisterEventHandler(
        OnStateTransition(
            target_lifecycle_node=monitor,
            goal_state="inactive",
            entities=[
                EmitEvent(
                    event=ChangeState(
                        lifecycle_node_matcher=matches_action(monitor),
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
                    [FindPackageShare("sentil_ros"), "config", "example_params.yaml"]
                ),
                description="Path to the monitor's parameter file.",
            ),
            DeclareLaunchArgument(
                "use_sim_time",
                default_value="false",
                description="Use the /clock topic for time, needed for bag replay and simulation.",
            ),
            DeclareLaunchArgument(
                "autostart",
                default_value="true",
                description="Configure and activate the node on launch.",
            ),
            activate,
            monitor,
            configure,
        ]
    )