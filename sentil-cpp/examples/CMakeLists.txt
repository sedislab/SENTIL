set(SENTIL_CPP_EXAMPLES
    offline_monitoring
    online_streaming
    probabilistic
    synthesis)

foreach(example ${SENTIL_CPP_EXAMPLES})
    add_executable(${example} ${example}.cpp)
    target_link_libraries(${example} PRIVATE Sentil::cpp)
    target_compile_options(${example} PRIVATE -Wall -Wextra)
    if(TARGET sentil_core)
        add_dependencies(${example} sentil_core)
    endif()
endforeach()