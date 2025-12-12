cmake_minimum_required(VERSION 3.13)

set(SENTIL_ROOT ${CMAKE_CURRENT_LIST_DIR}/../..)

if(NOT DEFINED SENTIL_ARCHIVE)
    message(FATAL_ERROR "set SENTIL_ARCHIVE to the libsentil_embedded.a for your core")
endif()

add_library(sentil_embedded STATIC ${SENTIL_ROOT}/src/Sentil.cpp)
target_include_directories(sentil_embedded PUBLIC ${SENTIL_ROOT}/include)
target_link_libraries(sentil_embedded PUBLIC ${SENTIL_ARCHIVE})