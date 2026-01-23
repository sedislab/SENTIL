if(NOT DEFINED ARA_COM_ROOT)
  message(FATAL_ERROR
    "SENTIL_AP_VENDOR=${SENTIL_AP_VENDOR} needs ARA_COM_ROOT pointing at the vendor ara::com install")
endif()
find_library(ARA_COM_LIBRARY NAMES ara_com PATHS "${ARA_COM_ROOT}/lib" REQUIRED)
add_library(ara_com INTERFACE)
target_include_directories(ara_com INTERFACE "${ARA_COM_ROOT}/include")
target_link_libraries(ara_com INTERFACE "${ARA_COM_LIBRARY}")