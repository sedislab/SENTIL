# Sentil on bare metal

Download the latest [sentil-bare-metal.tar.gz](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-bare-metal.tar.gz) from the Releases tab, or build the archive from source as below.

For a project with no operating system: link the precompiled archive and the C++ wrapper directly, with your own startup and linker script. This is also the path for a soft-core CPU on an FPGA, since that runs the same instruction set; it is not a hardware monitor synthesized to gates, which is a separate undertaking.

## Build

Cross-compile the archive for your core with the recipe in `../../extras/cross_compile.md`, then add this directory to your CMake build:

```cmake
set(SENTIL_ARCHIVE ${CMAKE_SOURCE_DIR}/lib/cortex-m4/libsentil_embedded.a)
add_subdirectory(third_party/sentil/packaging/bare-metal sentil)
target_link_libraries(my_firmware PRIVATE sentil_embedded)
```

Without CMake, the link is just the archive, the wrapper, and your include path:

```
arm-none-eabi-g++ -mcpu=cortex-m4 main.cpp src/Sentil.cpp lib/cortex-m4/libsentil_embedded.a \
  -I include -o firmware.elf
```

## What your project provides

The archive bundles its own single-core critical-section for the allocator, so the only symbols it needs from outside are the C runtime ones your board support package already supplies: `memcpy`, `memset`, the `libm` math functions, and, with newlib, the retarget lock stubs (`__retarget_lock_*` and the `__lock___*_mutex` objects). A single-threaded build stubs those as no-ops; an RTOS build maps them to its mutexes. Hand the monitor a fixed heap with `sentil_embedded_init` before the first call, as on any target.

On a multi-core part keep every SENTIL call on one core, since the bundled critical-section guards a single core only.