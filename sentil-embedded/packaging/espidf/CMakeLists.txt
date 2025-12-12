idf_component_register(
    SRCS "Sentil.cpp"
    INCLUDE_DIRS "include"
)

if(NOT DEFINED SENTIL_ARCHIVE)
    set(SENTIL_ARCHIVE "${CMAKE_CURRENT_LIST_DIR}/prebuilt/${IDF_TARGET}/libsentil_embedded.a")
endif()
target_link_libraries(${COMPONENT_LIB} PUBLIC "${SENTIL_ARCHIVE}")