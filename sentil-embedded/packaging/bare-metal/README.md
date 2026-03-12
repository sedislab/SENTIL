<div align="center">

# SENTIL

#### The bare-metal C library for Probabilistic Signal Temporal Logic

[![Bare metal](https://img.shields.io/badge/target-bare--metal-blue.svg)](#)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#)

</div>

Download [sentil-bare-metal.tar.gz](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-bare-metal.tar.gz) from the Releases tab, or build the archive from source with the recipe in `../../extras/cross_compile.md`.

For a project with no operating system, link the precompiled archive and the C++ wrapper directly, alongside your own startup code and linker script. A soft-core CPU on an FPGA takes the same path, since it runs the same instruction set; a hardware monitor synthesized to gates is a separate undertaking.

## Build

Cross-compile the archive for your core, then add this directory to your CMake build and link the `sentil_embedded` target:

```cmake
set(SENTIL_ARCHIVE ${CMAKE_SOURCE_DIR}/lib/cortex-m4/libsentil_embedded.a)
add_subdirectory(third_party/sentil/packaging/bare-metal sentil)
target_link_libraries(my_firmware PRIVATE sentil_embedded)
```

CMake stops with an error if `SENTIL_ARCHIVE` is unset, so point it at the `libsentil_embedded.a` you built for the board's core. Without CMake the link is the archive, the wrapper, and your include path:

```
arm-none-eabi-g++ -mcpu=cortex-m4 main.cpp src/Sentil.cpp lib/cortex-m4/libsentil_embedded.a \
  -I include -o firmware.elf
```

`Sentil.h` declares a C ABI as well as the C++ `SentilMonitor` class. A firmware in C calls the C ABI and links the archive on its own; `Sentil.cpp` only implements the C++ class, so leave it out of the command and use `arm-none-eabi-gcc` when the sources are pure C.

## A first monitor

Hand the monitor a fixed block of memory once at startup, create it from a formula, then push one sample per step. Times must strictly increase.

```c
#include "Sentil.h"

static uint8_t heap[8192];

int main(void) {
    sentil_embedded_init(heap, sizeof(heap));

    sentil_embedded_monitor_t *monitor = NULL;
    if (sentil_embedded_create("historically (x > 0)", &monitor) != SENTIL_EMBEDDED_OK) {
        return 1;  // sentil_embedded_status_message() turns a code into a string
    }

    double t = 0.0;
    for (;;) {
        double values[1] = { read_sensor() };
        sentil_embedded_robustness_t r;
        if (sentil_embedded_update(monitor, t, values, 1, &r) == SENTIL_EMBEDDED_OK) {
            // r.value is the margin, r.satisfied whether it holds, r.resolved whether the verdict is settled
        }
        t += 1.0;
    }
}
```

Size the region passed to `sentil_embedded_init` for the worst-case window and leave headroom. A few kilobytes hold a typical monitor, and exhausting the region halts the board. On a board short on flash, drop the parser and load a formula compiled on a workstation, calling `sentil_embedded_create_compiled` in place of `sentil_embedded_create`; the main README has that workflow.

## What your project provides

The archive bundles its own single-core critical-section for the allocator, so the only symbols it needs from outside are the C runtime ones your board support package already supplies: `memcpy`, `memset`, the `libm` math functions, and, with newlib, the retarget lock stubs (`__retarget_lock_*` and the `__lock___*_mutex` objects). A single-threaded build stubs those as no-ops; an RTOS build maps them to its mutexes.

On a multi-core part keep every SENTIL call on one core, since the bundled critical-section guards a single core only.

For the full API and the other packagings, see the [main sentil-embedded README](../../README.md). SENTIL is by Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab, dual licensed under MIT or Apache-2.0.