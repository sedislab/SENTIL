cmake_minimum_required(VERSION 3.16)
project(SentilCpp
    VERSION 0.3.0
    DESCRIPTION "C++ bindings for SENTIL, runtime verification for STL and PrSTL"
    LANGUAGES CXX)

if(CMAKE_SOURCE_DIR STREQUAL CMAKE_CURRENT_SOURCE_DIR)
    set(_sentil_top ON)
else()
    set(_sentil_top OFF)
endif()

option(SENTIL_CPP_BUILD_TESTS "Build the C++ test suite" ${_sentil_top})
option(SENTIL_CPP_BUILD_EXAMPLES "Build the C++ examples" ${_sentil_top})
option(SENTIL_CPP_BUILD_CORE "Rebuild the Rust core before linking" ${_sentil_top})

get_filename_component(_sentil_root "${CMAKE_CURRENT_SOURCE_DIR}/.." ABSOLUTE)
set(SENTIL_ROOT "${_sentil_root}" CACHE PATH "Root of the SENTIL monorepo")
set(SENTIL_LIB_DIR "${SENTIL_ROOT}/target/release" CACHE PATH "Directory holding libsentil")
set(SENTIL_INCLUDE_DIR "${SENTIL_ROOT}/sentil-ffi/include" CACHE PATH "Directory holding sentil.h")

find_package(Sentil CONFIG QUIET)
if(NOT TARGET Sentil::sentil)
    add_library(Sentil::sentil SHARED IMPORTED)
    set_target_properties(Sentil::sentil PROPERTIES
        IMPORTED_LOCATION
            "${SENTIL_LIB_DIR}/${CMAKE_SHARED_LIBRARY_PREFIX}sentil${CMAKE_SHARED_LIBRARY_SUFFIX}"
        INTERFACE_INCLUDE_DIRECTORIES "${SENTIL_INCLUDE_DIR}")
endif()

find_package(Threads REQUIRED)

add_library(sentil_cpp INTERFACE)
add_library(Sentil::cpp ALIAS sentil_cpp)
target_compile_features(sentil_cpp INTERFACE cxx_std_17)
target_include_directories(sentil_cpp INTERFACE
    "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>"
    "$<INSTALL_INTERFACE:include>")
target_link_libraries(sentil_cpp INTERFACE Sentil::sentil Threads::Threads ${CMAKE_DL_LIBS})
if(UNIX AND NOT APPLE)
    target_link_libraries(sentil_cpp INTERFACE m)
endif()

if(SENTIL_CPP_BUILD_CORE)
    add_custom_target(sentil_core
        COMMAND cargo build --release -p sentil-ffi
        WORKING_DIRECTORY "${SENTIL_ROOT}"
        COMMENT "Building the SENTIL core (libsentil)")
endif()

if(SENTIL_CPP_BUILD_TESTS AND EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/tests/CMakeLists.txt")
    enable_testing()
    add_subdirectory(tests)
endif()
if(SENTIL_CPP_BUILD_EXAMPLES AND EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/examples/CMakeLists.txt")
    add_subdirectory(examples)
endif()

include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

install(DIRECTORY include/sentil DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}")

set(_sentil_cpp_cmakedir "${CMAKE_INSTALL_LIBDIR}/cmake/SentilCpp")
configure_package_config_file(cmake/SentilCppConfig.cmake.in
    "${CMAKE_CURRENT_BINARY_DIR}/SentilCppConfig.cmake"
    INSTALL_DESTINATION "${_sentil_cpp_cmakedir}")
write_basic_package_version_file("${CMAKE_CURRENT_BINARY_DIR}/SentilCppConfigVersion.cmake"
    VERSION ${PROJECT_VERSION} COMPATIBILITY SameMajorVersion)
install(FILES
    "${CMAKE_CURRENT_BINARY_DIR}/SentilCppConfig.cmake"
    "${CMAKE_CURRENT_BINARY_DIR}/SentilCppConfigVersion.cmake"
    DESTINATION "${_sentil_cpp_cmakedir}")