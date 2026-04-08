<div align="center">

# SENTIL

#### The bare-metal C library for Probabilistic Signal Temporal Logic

[![Bare metal](https://img.shields.io/badge/target-bare--metal-blue.svg)](#)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#docs-and-license)

</div>

SENTIL as a bare-metal drop-in, for a project with no operating system. You link the precompiled static archive and the C++ wrapper directly, alongside your own startup code and linker script, and you get the streaming STL monitor, the multi-formula monitor, the ring buffer, offline robustness over a buffered trace, and the on-board synthesis layer with no standard library underneath. You do the same thing for a soft-core CPU on an FPGA.

## Install

### Prebuilt release

Download [sentil-bare-metal.tar.gz](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-bare-metal.tar.gz) from the [GitHub release](https://github.com/sedislab/SENTIL/releases) and unpack it. The tarball holds everything the link needs: `include/Sentil.h`, the `src/Sentil.cpp` wrapper, the `CMakeLists.txt` for the drop-in build, and a precompiled `lib/<mcu>/libsentil_embedded.a` for each core it ships. Those cores are `cortex-m0plus`, `cortex-m3`, `cortex-m4`, `cortex-m7`, `esp32c3`, and `riscv32imac`, where the `cortex-m7` archive is the `cortex-m4` build that runs on both.

### Build from source

To rebuild the archive, or to target a core the release does not ship, install the target triple, then build the `no_std` static library from `sentil-embedded/rust`:

```bash
rustup target add thumbv7em-none-eabihf
cargo build --release --features mcu --target thumbv7em-none-eabihf
# smallest-flash boards, dropping the text parser and synthesis:
cargo build --release --no-default-features --features mcu --target thumbv6m-none-eabi
```

The archive lands at `rust/target/<triple>/release/libsentil_embedded.a`. `extras/cross_compile.md` maps every core to its target triple and its `lib/<mcu>` folder name.

## A first monitor

Hand the monitor a fixed block of memory once at startup, create it from a formula, then push one sample per step. Times must strictly increase.

```c
#include "Sentil.h"

static uint8_t heap[8192];

int main(void) {
    sentil_embedded_init(heap, sizeof(heap));

    sentil_embedded_monitor_t *monitor = NULL;
    if (sentil_embedded_create("historically (x > 0)", &monitor) != SENTIL_EMBEDDED_OK) {
        return 1;
    }

    double t = 0.0;
    for (;;) {
        double values[1] = { read_sensor() };
        sentil_embedded_robustness_t r;
        if (sentil_embedded_update(monitor, t, values, 1, &r) == SENTIL_EMBEDDED_OK) {
            // act on r.satisfied and r.value
        }
        t += 1.0;
    }
}
```

Size the region passed to `sentil_embedded_init` for the worst-case window and leave headroom. A few kilobytes hold a typical monitor, and exhausting the region will stop the board. On a board short on flash mem, drop the parser and load a formula compiled on a workstation, calling `sentil_embedded_create_compiled` in place of `sentil_embedded_create`; the main [README](../../README.md) has that workflow.

## Linking

There are two ways to wire the archive and the `Sentil.cpp` wrapper into a firmware. Both need the archive for the board's core and the `include` directory on the compiler's search path.

### With CMake

Add this directory to your build with `add_subdirectory` and link the `sentil_embedded` target. Point `SENTIL_ARCHIVE` at the archive you unpacked or built for the board's core:

```cmake
set(SENTIL_ARCHIVE ${CMAKE_SOURCE_DIR}/lib/cortex-m4/libsentil_embedded.a)
add_subdirectory(third_party/sentil/packaging/bare-metal sentil)
target_link_libraries(my_firmware PRIVATE sentil_embedded)
```

The subdirectory build compiles `Sentil.cpp`, exposes the `include` directory, and links the archive into the `sentil_embedded` target. It stops with a fatal error when `SENTIL_ARCHIVE` is unset, so there is no silent link against the wrong core.

### Directly with arm-none-eabi-g++

Without CMake the link is the archive, the wrapper, and the include path:

```bash
arm-none-eabi-g++ -mcpu=cortex-m4 main.cpp src/Sentil.cpp lib/cortex-m4/libsentil_embedded.a \
  -I include -o firmware.elf
```

`Sentil.h` declares the C ABI and the C++ `SentilMonitor` class both. A firmware written in C calls the C ABI and links the archive on its own; `Sentil.cpp` only implements the C++ class, so drop it from the command and use `arm-none-eabi-gcc` when the sources are pure C.

## What your project provides

The archive bundles its own single-core critical-section for the allocator, so the symbols it needs from outside are the C runtime ones a board support package already supplies: `memcpy`, `memset`, and the `libm` math functions. With newlib it also references the retarget lock stubs, `__retarget_lock_*` and the `__lock___*_mutex` objects. A single-threaded build stubs those as no-ops, and an RTOS build maps them to its mutexes.

On a multi-core part keep every SENTIL call on one core, since the bundled critical-section guards a single core only. To share the monitor across cores, build the archive without the bundled critical-section and link one from the board HAL instead.

## Docs and license

See the top-level [`sentil-embedded/README.md`](../../README.md) for the full API, the operator guidance, and the heap budget. Dual licensed under MIT OR Apache-2.0. Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS lab.