cmake_minimum_required(VERSION 3.16)
project(sentil_jni CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

set(SENTIL_LIB_DIR "" CACHE PATH "Directory holding the built libsentil")
set(SENTIL_HEADER_DIR "" CACHE PATH "Directory holding sentil.h")
set(SENTIL_JNI_HEADERS "" CACHE PATH "Directory holding the javac -h headers")
set(SENTIL_RESOURCE_DIR "${CMAKE_CURRENT_BINARY_DIR}" CACHE PATH "Jar resource root")

find_package(JNI REQUIRED)
find_library(SENTIL_CORE NAMES sentil HINTS "${SENTIL_LIB_DIR}" REQUIRED)

string(TOLOWER "${CMAKE_SYSTEM_NAME}" SENTIL_OS)
if(NOT SENTIL_OS STREQUAL "darwin" AND NOT SENTIL_OS STREQUAL "windows")
    set(SENTIL_OS "linux")
endif()
string(TOLOWER "${CMAKE_SYSTEM_PROCESSOR}" SENTIL_ARCH)
if(SENTIL_ARCH MATCHES "amd64|x86_64")
    set(SENTIL_ARCH "x86_64")
elseif(SENTIL_ARCH MATCHES "aarch64|arm64")
    set(SENTIL_ARCH "aarch64")
endif()
set(SENTIL_OUT "${SENTIL_RESOURCE_DIR}/native/${SENTIL_OS}-${SENTIL_ARCH}")
file(MAKE_DIRECTORY "${SENTIL_OUT}")

add_library(sentil_jni SHARED sentil_jni.cpp)
target_include_directories(sentil_jni PRIVATE
    ${JNI_INCLUDE_DIRS}
    "${SENTIL_HEADER_DIR}"
    "${SENTIL_JNI_HEADERS}")
target_link_libraries(sentil_jni PRIVATE ${SENTIL_CORE})
if(CMAKE_COMPILER_IS_GNUCXX OR CMAKE_CXX_COMPILER_ID MATCHES "Clang")
  target_compile_options(sentil_jni PRIVATE -Wall -Wextra)
endif()

if(APPLE)
    set_target_properties(sentil_jni PROPERTIES INSTALL_RPATH "@loader_path" BUILD_WITH_INSTALL_RPATH ON)
else()
    set_target_properties(sentil_jni PROPERTIES INSTALL_RPATH "$ORIGIN" BUILD_WITH_INSTALL_RPATH ON)
endif()
set_target_properties(sentil_jni PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY "${SENTIL_OUT}"
    RUNTIME_OUTPUT_DIRECTORY "${SENTIL_OUT}")

add_custom_command(TARGET sentil_jni POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy "${SENTIL_CORE}" "${SENTIL_OUT}/")