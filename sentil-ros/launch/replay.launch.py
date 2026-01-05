"""Monitor a recorded bag, with use_sim_time on."""

from launch import LaunchDescription
from launch.actions import DeclareLaunchArgument, IncludeLaunchDescription
from launch.launch_description_sources import PythonLaunchDescriptionSource
from launch.substitutions import LaunchConfiguration, PathJoinSubstitution
from launch_ros.substitutions import FindPackageShare


def generate_launch_description():
    share = FindPackageShare("sentil_ros")
    return LaunchDescription(
        [
            DeclareLaunchArgument(
                "params_file",
                default_value=PathJoinSubstitution([share, "config", "example_params.yaml"]),
                description="Path to the monitor's parameter file.",
            ),
            IncludeLaunchDescription(
                PythonLaunchDescriptionSource(
                    PathJoinSubstitution([share, "launch", "sentil_monitor.launch.py"])
                ),
                launch_arguments={
                    "params_file": LaunchConfiguration("params_file"),
                    "use_sim_time": "true",
                }.items(),
            ),
        ]
    )