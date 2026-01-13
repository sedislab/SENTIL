cmake_minimum_required(VERSION 3.16)
project(sentil_ros C CXX)

if(NOT CMAKE_CXX_STANDARD)
  set(CMAKE_CXX_STANDARD 17)
  set(CMAKE_CXX_STANDARD_REQUIRED ON)
endif()

if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
  set(CMAKE_BUILD_TYPE Release)
endif()
if(CMAKE_COMPILER_IS_GNUCXX OR CMAKE_CXX_COMPILER_ID MATCHES "Clang")
  add_compile_options(-Wall -Wextra -Wpedantic)
endif()

find_package(ament_cmake_ros REQUIRED)
find_package(rosidl_default_generators REQUIRED)
find_package(rclcpp REQUIRED)
find_package(rclcpp_lifecycle REQUIRED)
find_package(rclcpp_components REQUIRED)
find_package(lifecycle_msgs REQUIRED)
find_package(diagnostic_updater REQUIRED)
find_package(diagnostic_msgs REQUIRED)
find_package(rosidl_typesupport_introspection_cpp REQUIRED)
find_package(rcpputils REQUIRED)
find_package(std_msgs REQUIRED)
find_package(sensor_msgs REQUIRED)
find_package(SentilCpp CONFIG REQUIRED)

rosidl_generate_interfaces(${PROJECT_NAME}
  msg/Robustness.msg
  msg/Probability.msg
  msg/Control.msg
  srv/GetSpecInfo.srv
  DEPENDENCIES std_msgs sensor_msgs
)

add_library(sentil_monitor_node SHARED
  src/monitor_node.cpp
  src/field_extractor.cpp)
target_include_directories(sentil_monitor_node PUBLIC
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:include>)
ament_target_dependencies(sentil_monitor_node
  rclcpp rclcpp_lifecycle rclcpp_components lifecycle_msgs
  diagnostic_updater diagnostic_msgs rcpputils
  rosidl_typesupport_introspection_cpp std_msgs sensor_msgs)
target_link_libraries(sentil_monitor_node Sentil::cpp ${CMAKE_DL_LIBS})
rosidl_target_interfaces(sentil_monitor_node ${PROJECT_NAME} "rosidl_typesupport_cpp")
rclcpp_components_register_node(sentil_monitor_node
  PLUGIN "sentil_ros::MonitorNode"
  EXECUTABLE sentil_monitor)

add_library(sentil_control_node SHARED
  src/control_node.cpp)
target_include_directories(sentil_control_node PUBLIC
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:include>)
ament_target_dependencies(sentil_control_node
  rclcpp rclcpp_lifecycle rclcpp_components lifecycle_msgs std_msgs)
target_link_libraries(sentil_control_node Sentil::cpp ${CMAKE_DL_LIBS})
rosidl_target_interfaces(sentil_control_node ${PROJECT_NAME} "rosidl_typesupport_cpp")
rclcpp_components_register_node(sentil_control_node
  PLUGIN "sentil_ros::ControlNode"
  EXECUTABLE sentil_control)

install(TARGETS sentil_monitor_node sentil_control_node
  ARCHIVE DESTINATION lib
  LIBRARY DESTINATION lib
  RUNTIME DESTINATION bin)
install(DIRECTORY include/ DESTINATION include)
install(DIRECTORY launch config examples DESTINATION share/${PROJECT_NAME})

if(BUILD_TESTING)
  find_package(ament_cmake_gtest REQUIRED)
  find_package(geometry_msgs REQUIRED)

  ament_add_gtest(test_field_extractor
    test/test_field_extractor.cpp
    src/field_extractor.cpp)
  target_include_directories(test_field_extractor PRIVATE include)
  ament_target_dependencies(test_field_extractor
    rclcpp rosidl_typesupport_introspection_cpp geometry_msgs sensor_msgs)
endif()

ament_package()