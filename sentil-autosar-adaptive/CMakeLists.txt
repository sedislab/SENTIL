cmake_minimum_required(VERSION 3.16)
project(sentil_autosar_adaptive CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()
add_compile_options(-Wall -Wextra)

# Which Adaptive Platform supplies ara::*. The stub ships an open-source ara::com over
# vsomeip so the examples run on a plain Linux box; a vendor AP is a toolchain swap that
# points FindAraCom at the vendor binding, leaving the app source unchanged.
set(SENTIL_AP_VENDOR "stub" CACHE STRING "Adaptive Platform vendor: stub, vector, eb, apex")

set(SENTIL_ROOT "" CACHE PATH "SENTIL install tree (include/ and lib/libsentil)")
add_library(sentil::cpp SHARED IMPORTED)
set_target_properties(sentil::cpp PROPERTIES
  IMPORTED_LOCATION "${SENTIL_ROOT}/lib/libsentil.so"
  INTERFACE_INCLUDE_DIRECTORIES "${SENTIL_ROOT}/include")

if(SENTIL_AP_VENDOR STREQUAL "stub")
  find_package(vsomeip3 REQUIRED)
  add_library(ara_com ara_stub/src/service.cpp)
  target_include_directories(ara_com PUBLIC ara_stub/include)
  target_link_libraries(ara_com PUBLIC vsomeip3)
elseif(NOT SENTIL_AP_VENDOR STREQUAL "none")
  include(cmake/FindAraCom.cmake)
endif()

add_library(sentil_apps
  src/monitor/monitor_app.cpp
  src/control/control_app.cpp)
target_include_directories(sentil_apps PUBLIC
  include src/monitor src/control)
target_link_libraries(sentil_apps PUBLIC sentil::cpp)

add_executable(sentil_monitor src/monitor/main.cpp)
target_link_libraries(sentil_monitor PRIVATE sentil_apps ara_com)

add_executable(sentil_control src/control/main.cpp)
target_link_libraries(sentil_control PRIVATE sentil_apps ara_com)

install(TARGETS sentil_monitor sentil_control RUNTIME DESTINATION bin)
install(FILES "${SENTIL_ROOT}/lib/libsentil.so" DESTINATION lib)
install(DIRECTORY manifest DESTINATION .)

if(SENTIL_AP_VENDOR STREQUAL "stub")
  add_executable(perception_publisher examples/adas_fca_monitor/perception_publisher.cpp)
  target_link_libraries(perception_publisher PRIVATE ara_com)
  target_include_directories(perception_publisher PRIVATE include)
  add_executable(planner_subscriber examples/adas_fca_monitor/planner_subscriber.cpp)
  target_link_libraries(planner_subscriber PRIVATE ara_com)
  target_include_directories(planner_subscriber PRIVATE include)
endif()

enable_testing()

add_executable(parity_test test/parity_test.cpp)
target_include_directories(parity_test PRIVATE test)
target_link_libraries(parity_test PRIVATE sentil_apps)
target_compile_definitions(parity_test PRIVATE
  SENTIL_ORACLE_PATH="${CMAKE_CURRENT_SOURCE_DIR}/../benchmarks/deterministic/oracle.json")
add_test(NAME parity COMMAND parity_test)

add_executable(synthesis_test test/synthesis_test.cpp)
target_include_directories(synthesis_test PRIVATE test)
target_link_libraries(synthesis_test PRIVATE sentil_apps)
add_test(NAME synthesis COMMAND synthesis_test)

add_library(vendor_contract_check OBJECT src/monitor/main.cpp src/control/main.cpp)
target_include_directories(vendor_contract_check PRIVATE
  test/vendor_contract include src/monitor src/control)
target_link_libraries(vendor_contract_check PRIVATE sentil::cpp)

add_test(NAME arxml_validate
  COMMAND ${CMAKE_COMMAND} -E env python3
          "${CMAKE_CURRENT_SOURCE_DIR}/test/arxml_validate.py"
          "${CMAKE_CURRENT_SOURCE_DIR}")

if(SENTIL_AP_VENDOR STREQUAL "stub" AND SENTIL_BUILD_LIFECYCLE_TEST)
  add_executable(lifecycle_test test/lifecycle_test.cpp)
  target_link_libraries(lifecycle_test PRIVATE sentil_apps ara_com)
  add_test(NAME lifecycle COMMAND lifecycle_test)
endif()