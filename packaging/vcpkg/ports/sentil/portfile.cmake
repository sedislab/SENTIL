set(SENTIL_VERSION 1.0.0)
set(SENTIL_RELEASE "https://github.com/sedislab/SENTIL/releases/download/v${SENTIL_VERSION}")

vcpkg_check_linkage(ONLY_DYNAMIC_LIBRARY)

if(VCPKG_TARGET_IS_LINUX)
    if(NOT VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
        message(FATAL_ERROR "sentil ships a prebuilt bundle for linux-x86_64 only.")
    endif()
    set(SENTIL_PLATFORM "linux-x86_64")
    set(SENTIL_SHA512 "SENTIL_SHA512_LINUX")
    set(SENTIL_SHARED "libsentil.so")
    set(SENTIL_STATIC "libsentil.a")
elseif(VCPKG_TARGET_IS_OSX)
    if(VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
        set(SENTIL_PLATFORM "macos-x86_64")
        set(SENTIL_SHA512 "SENTIL_SHA512_MACOS_X86_64")
    elseif(VCPKG_TARGET_ARCHITECTURE STREQUAL "arm64")
        set(SENTIL_PLATFORM "macos-arm64")
        set(SENTIL_SHA512 "SENTIL_SHA512_MACOS_ARM64")
    else()
        message(FATAL_ERROR "sentil ships a prebuilt bundle for macos x86_64 and arm64 only.")
    endif()
    set(SENTIL_SHARED "libsentil.dylib")
    set(SENTIL_STATIC "libsentil.a")
elseif(VCPKG_TARGET_IS_WINDOWS)
    if(NOT VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
        message(FATAL_ERROR "sentil ships a prebuilt bundle for windows-x86_64 only.")
    endif()
    set(SENTIL_PLATFORM "windows-x86_64")
    set(SENTIL_SHA512 "SENTIL_SHA512_WINDOWS")
    set(SENTIL_SHARED "sentil.dll")
    set(SENTIL_IMPORT_LIB "sentil.dll.lib")
else()
    message(FATAL_ERROR "sentil has no prebuilt bundle for this platform.")
endif()

set(SENTIL_BUNDLE "sentil-${SENTIL_VERSION}-${SENTIL_PLATFORM}")

vcpkg_download_distfile(SENTIL_ARCHIVE
    URLS "${SENTIL_RELEASE}/${SENTIL_BUNDLE}.tar.gz"
    FILENAME "${SENTIL_BUNDLE}.tar.gz"
    SHA512 "${SENTIL_SHA512}"
)

vcpkg_extract_source_archive(SENTIL_SRC
    ARCHIVE "${SENTIL_ARCHIVE}"
    SOURCE_BASE "${SENTIL_BUNDLE}"
)

file(INSTALL "${SENTIL_SRC}/include/sentil.h"
    DESTINATION "${CURRENT_PACKAGES_DIR}/include")
file(INSTALL "${SENTIL_SRC}/include/sentil"
    DESTINATION "${CURRENT_PACKAGES_DIR}/include")

if(VCPKG_TARGET_IS_WINDOWS)
    file(INSTALL "${SENTIL_SRC}/lib/${SENTIL_SHARED}"
        DESTINATION "${CURRENT_PACKAGES_DIR}/bin")
    file(INSTALL "${SENTIL_SRC}/lib/${SENTIL_SHARED}"
        DESTINATION "${CURRENT_PACKAGES_DIR}/debug/bin")
    file(INSTALL "${SENTIL_SRC}/lib/${SENTIL_IMPORT_LIB}"
        DESTINATION "${CURRENT_PACKAGES_DIR}/lib")
    file(INSTALL "${SENTIL_SRC}/lib/${SENTIL_IMPORT_LIB}"
        DESTINATION "${CURRENT_PACKAGES_DIR}/debug/lib")
else()
    file(INSTALL "${SENTIL_SRC}/lib/${SENTIL_SHARED}"
        DESTINATION "${CURRENT_PACKAGES_DIR}/lib")
    file(INSTALL "${SENTIL_SRC}/lib/${SENTIL_SHARED}"
        DESTINATION "${CURRENT_PACKAGES_DIR}/debug/lib")
    if(EXISTS "${SENTIL_SRC}/lib/${SENTIL_STATIC}")
        file(INSTALL "${SENTIL_SRC}/lib/${SENTIL_STATIC}"
            DESTINATION "${CURRENT_PACKAGES_DIR}/lib")
        file(INSTALL "${SENTIL_SRC}/lib/${SENTIL_STATIC}"
            DESTINATION "${CURRENT_PACKAGES_DIR}/debug/lib")
    endif()
endif()

if(EXISTS "${SENTIL_SRC}/lib/pkgconfig/sentil.pc")
    file(READ "${SENTIL_SRC}/lib/pkgconfig/sentil.pc" SENTIL_PC)
    string(REGEX REPLACE "prefix=[^\n]*" "prefix=\${pcfiledir}/../.." SENTIL_PC_REL "${SENTIL_PC}")
    file(WRITE "${CURRENT_PACKAGES_DIR}/lib/pkgconfig/sentil.pc" "${SENTIL_PC_REL}")
    file(WRITE "${CURRENT_PACKAGES_DIR}/debug/lib/pkgconfig/sentil.pc" "${SENTIL_PC_REL}")
    vcpkg_fixup_pkgconfig()
endif()

file(WRITE "${CURRENT_PACKAGES_DIR}/share/${PORT}/SentilConfig.cmake"
"set(SENTIL_VERSION ${SENTIL_VERSION})

get_filename_component(SENTIL_PREFIX \"\${CMAKE_CURRENT_LIST_DIR}/../..\" ABSOLUTE)
set(SENTIL_INCLUDE_DIR \"\${SENTIL_PREFIX}/include\")
set(SENTIL_LIBRARY \"\${SENTIL_PREFIX}/lib/\${CMAKE_SHARED_LIBRARY_PREFIX}sentil\${CMAKE_SHARED_LIBRARY_SUFFIX}\")

if(NOT TARGET Sentil::sentil)
    add_library(Sentil::sentil SHARED IMPORTED)
    set_target_properties(Sentil::sentil PROPERTIES
        INTERFACE_INCLUDE_DIRECTORIES \"\${SENTIL_INCLUDE_DIR}\")
    if(WIN32)
        set_target_properties(Sentil::sentil PROPERTIES
            IMPORTED_LOCATION \"\${SENTIL_PREFIX}/bin/sentil.dll\"
            IMPORTED_IMPLIB \"\${SENTIL_PREFIX}/lib/sentil.dll.lib\")
    else()
        set_target_properties(Sentil::sentil PROPERTIES
            IMPORTED_LOCATION \"\${SENTIL_LIBRARY}\")
    endif()
endif()
"
)

file(INSTALL "${CMAKE_CURRENT_LIST_DIR}/usage"
    DESTINATION "${CURRENT_PACKAGES_DIR}/share/${PORT}")

vcpkg_download_distfile(SENTIL_LICENSE_MIT
    URLS "https://raw.githubusercontent.com/sedislab/SENTIL/v${SENTIL_VERSION}/LICENSE-MIT"
    FILENAME "sentil-${SENTIL_VERSION}-LICENSE-MIT"
    SHA512 "SENTIL_SHA512_LICENSE_MIT"
)
vcpkg_download_distfile(SENTIL_LICENSE_APACHE
    URLS "https://raw.githubusercontent.com/sedislab/SENTIL/v${SENTIL_VERSION}/LICENSE-APACHE"
    FILENAME "sentil-${SENTIL_VERSION}-LICENSE-APACHE"
    SHA512 "SENTIL_SHA512_LICENSE_APACHE"
)
vcpkg_install_copyright(FILE_LIST "${SENTIL_LICENSE_MIT}" "${SENTIL_LICENSE_APACHE}")