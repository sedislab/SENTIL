# The C++ test suite. Each test is a standalone executable registered with ctest;
# the top-level CMakeLists rebuilds libsentil first so a stale core never hides a
# regression. The oracle path is passed in so the cross-language oracle loads the
# one shared fixture.

set(SENTIL_TEST_ORACLE "${SENTIL_ROOT}/benchmarks/deterministic/oracle.json"
    CACHE FILEPATH "The shared deterministic oracle the cross-language test loads")

set(SENTIL_CPP_TESTS
    test_oracle
    test_formula
    test_signal
    test_monitor
    test_stats
    test_synthesis
    test_specs
    test_errors)

foreach(test ${SENTIL_CPP_TESTS})
    if(NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/${test}.cpp")
        continue()
    endif()
    add_executable(${test} ${test}.cpp)
    target_link_libraries(${test} PRIVATE Sentil::cpp)
    target_include_directories(${test} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
    target_compile_definitions(${test} PRIVATE SENTIL_ORACLE_PATH="${SENTIL_TEST_ORACLE}")
    target_compile_options(${test} PRIVATE -Wall -Wextra)
    if(TARGET sentil_core)
        add_dependencies(${test} sentil_core)
    endif()
    add_test(NAME ${test} COMMAND ${test})
endforeach()