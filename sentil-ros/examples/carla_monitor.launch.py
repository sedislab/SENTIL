"""Monitor a CARLA ego vehicle against the safety specifications in
config/carla_verification.yaml.

Run CARLA and the bridge however you like and leave launch_bridge at its default; or
set launch_bridge:=true to start the bridge here, pointing it at host:port.
"""

from launch import LaunchDescription
from launch.actions import DeclareLaunchArgument, IncludeLaunchDescription
from launch.conditions import IfCondition
from launch.launch_description_sources import PythonLaunchDescriptionSource
from launch.substitutions import LaunchConfiguration, PathJoinSubstitution
from launch_ros.actions import Node
from launch_ros.substitutions import FindPackageShare

def generate_launch_description():
    share = FindPackageShare("sentil_ros")
    host = LaunchConfiguration("carla_host")
    port = LaunchConfiguration("carla_port")
    launch_bridge = LaunchConfiguration("launch_bridge")

    bridge = Node(
        package="carla_ros_bridge",
        executable="bridge",
        name="carla_ros_bridge",
        output="screen",
        parameters=[{"host": host, "port": port, "town": LaunchConfiguration("town")}],
        condition=IfCondition(launch_bridge),
    )

    monitor = IncludeLaunchDescription(
        PythonLaunchDescriptionSource(
            PathJoinSubstitution([share, "launch", "sentil_monitor.launch.py"])
        ),
        launch_arguments={
            "params_file": PathJoinSubstitution([share, "config", "carla_verification.yaml"]),
            "use_sim_time": "true",
        }.items(),
    )

    return LaunchDescription(
        [
            DeclareLaunchArgument("carla_host", default_value="localhost",
                                  description="Hostname or IP of the CARLA server."),
            DeclareLaunchArgument("carla_port", default_value="2000",
                                  description="RPC port of the CARLA server."),
            DeclareLaunchArgument("town", default_value="Town03",
                                  description="CARLA map for the bridge to load."),
            DeclareLaunchArgument("launch_bridge", default_value="false",
                                  description="Start the carla_ros_bridge here instead of separately."),
            bridge,
            monitor,
        ]
    )