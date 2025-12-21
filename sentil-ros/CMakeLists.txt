cmake_minimum_required(VERSION 3.16)
project(sentil_ros C CXX)

if(NOT CMAKE_CXX_STANDARD)
  set(CMAKE_CXX_STANDARD 17)
  set(CMAKE_CXX_STANDARD_REQUIRED ON)
endif()

find_package(ament_cmake_ros REQUIRED)
find_package(rosidl_default_generators REQUIRED)
find_package(std_msgs REQUIRED)
find_package(sensor_msgs REQUIRED)

rosidl_generate_interfaces(${PROJECT_NAME}
  msg/Robustness.msg
  msg/Probability.msg
  srv/AnalyzeRareEvent.srv
  srv/AnalyzeSmcGpu.srv
  srv/AnalyzeStatsGpu.srv
  srv/GetSpecInfo.srv
  DEPENDENCIES std_msgs sensor_msgs
)

ament_package()