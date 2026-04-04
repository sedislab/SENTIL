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
set(SENTIL_VERSION "0.3.0" CACHE STRING "SENTIL release to fetch when no local library is found")
set(SENTIL_LIB_DIR "" CACHE PATH "Directory holding libsentil; skips every other step when set")
option(SENTIL_NO_DOWNLOAD "Never fetch a prebuilt libsentil from the release page" OFF)

if(SENTIL_LIB_DIR)
  find_library(SENTIL_LIBRARY NAMES sentil libsentil HINTS "${SENTIL_LIB_DIR}" NO_DEFAULT_PATH)
endif()

if(NOT SENTIL_LIBRARY)
  find_library(SENTIL_LIBRARY NAMES sentil libsentil)
endif()

if(NOT SENTIL_LIBRARY)
  find_library(SENTIL_LIBRARY NAMES sentil libsentil
    HINTS "${CMAKE_CURRENT_SOURCE_DIR}/../target/release" NO_DEFAULT_PATH)
endif()

if(NOT SENTIL_LIBRARY AND NOT SENTIL_NO_DOWNLOAD)
  if(CMAKE_SYSTEM_NAME STREQUAL "Linux" AND CMAKE_SYSTEM_PROCESSOR MATCHES "x86_64|amd64")
    set(_sentil_asset "linux-x86_64")
  elseif(CMAKE_SYSTEM_NAME STREQUAL "Linux" AND CMAKE_SYSTEM_PROCESSOR MATCHES "aarch64|arm64")
    set(_sentil_asset "aarch64-unknown-linux-gnu")
  elseif(APPLE AND CMAKE_SYSTEM_PROCESSOR MATCHES "arm64")
    set(_sentil_asset "macos-arm64")
  elseif(APPLE)
    set(_sentil_asset "macos-x86_64")
  endif()

  if(_sentil_asset)
    set(_sentil_root "sentil-${SENTIL_VERSION}-${_sentil_asset}")
    set(_sentil_url "https://github.com/sedislab/SENTIL/releases/download/v${SENTIL_VERSION}/${_sentil_root}.tar.gz")
    message(STATUS "libsentil not found locally; fetching ${_sentil_url}")

    set(_sentil_sums "https://github.com/sedislab/SENTIL/releases/download/v${SENTIL_VERSION}/SHA256SUMS")
    file(DOWNLOAD "${_sentil_sums}" "${CMAKE_CURRENT_BINARY_DIR}/SHA256SUMS" STATUS _sentil_sums_dl)
    list(GET _sentil_sums_dl 0 _sentil_sums_code)
    set(_sentil_expected "")
    if(_sentil_sums_code EQUAL 0)
      file(STRINGS "${CMAKE_CURRENT_BINARY_DIR}/SHA256SUMS" _sentil_sum_lines
           REGEX "  ${_sentil_root}\\.tar\\.gz$")
      if(_sentil_sum_lines)
        list(GET _sentil_sum_lines 0 _sentil_sum_line)
        string(REGEX MATCH "^[0-9a-fA-F]+" _sentil_expected "${_sentil_sum_line}")
      endif()
    endif()
    if(NOT _sentil_expected)
      message(FATAL_ERROR
        "could not read a checksum for ${_sentil_root}.tar.gz from ${_sentil_sums}.\n"
        "Build the library instead:  cargo build --release -p sentil-ffi\n"
        "or configure with -DSENTIL_NO_DOWNLOAD=ON and point -DSENTIL_LIB_DIR at it.")
    endif()

    file(DOWNLOAD "${_sentil_url}" "${CMAKE_CURRENT_BINARY_DIR}/sentil.tar.gz"
         EXPECTED_HASH "SHA256=${_sentil_expected}"
         STATUS _sentil_dl SHOW_PROGRESS)
    list(GET _sentil_dl 0 _sentil_dl_code)
    if(_sentil_dl_code EQUAL 0)
      file(ARCHIVE_EXTRACT INPUT "${CMAKE_CURRENT_BINARY_DIR}/sentil.tar.gz"
           DESTINATION "${CMAKE_CURRENT_BINARY_DIR}/sentil")
      find_library(SENTIL_LIBRARY NAMES sentil libsentil
        HINTS "${CMAKE_CURRENT_BINARY_DIR}/sentil/${_sentil_root}/lib" NO_DEFAULT_PATH)
    else()
      list(GET _sentil_dl 1 _sentil_dl_msg)
      message(STATUS "could not fetch libsentil: ${_sentil_dl_msg}")
    endif()
  endif()
endif()

if(NOT SENTIL_LIBRARY)
  message(FATAL_ERROR
    "libsentil was not found and could not be fetched.\n"
    "Pick one:\n"
    "  install the package:  sudo apt install ./libsentil-dev_${SENTIL_VERSION}_amd64.deb\n"
    "  build it yourself:    cargo build --release -p sentil-ffi\n"
    "  point at a copy:      colcon build --cmake-args -DSENTIL_LIB_DIR=/path/to/lib")
endif()
message(STATUS "using libsentil at ${SENTIL_LIBRARY}")

add_library(Sentil::cpp INTERFACE IMPORTED)
target_link_libraries(Sentil::cpp INTERFACE "${SENTIL_LIBRARY}" ${CMAKE_DL_LIBS})

rosidl_generate_interfaces(${PROJECT_NAME}
  msg/Robustness.msg
  msg/Probability.msg
  msg/Control.msg
  srv/GetSpecInfo.srv
  DEPENDENCIES std_msgs
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
  rosidl_typesupport_introspection_cpp std_msgs)
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