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
    test_errors
    test_parity)

set(SENTIL_CPP_GPU_TESTS test_gpu)

set(_sentil_leak_exes "")
foreach(test ${SENTIL_CPP_TESTS} ${SENTIL_CPP_GPU_TESTS})
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
    if(NOT test IN_LIST SENTIL_CPP_GPU_TESTS)
        list(APPEND _sentil_leak_exes "$<TARGET_FILE:${test}>")
    endif()
endforeach()

find_program(SENTIL_VALGRIND valgrind)
if(SENTIL_VALGRIND AND _sentil_leak_exes)
    set(_leak_commands "")
    foreach(exe ${_sentil_leak_exes})
        list(APPEND _leak_commands COMMAND ${SENTIL_VALGRIND} --leak-check=full
             --errors-for-leak-kinds=definite,indirect --error-exitcode=99 -q ${exe})
    endforeach()
    add_custom_target(leakcheck ${_leak_commands}
        COMMENT "Running the CPU tests under valgrind")
endif()