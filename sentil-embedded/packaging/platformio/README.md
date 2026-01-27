# Sentil for PlatformIO

Download the latest [sentil-platformio.zip](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-platformio.zip) from the Releases tab, or build the archive from source as below.

The SENTIL embedded library packaged for PlatformIO. It carries the same `Sentil.h` surface as the Arduino packaging: the streaming STL monitor and the on-board synthesis layer.

## Add it to a project

The released package bundles the header, the `Sentil.cpp` wrapper, and a precompiled archive per board architecture, so no Rust toolchain is needed. Add the library and link the archive for your board's core:

```ini
[env:pico]
platform = https://github.com/maxgerhardt/platform-raspberrypi.git
board = pico
framework = arduino
board_build.core = earlephilhower
lib_deps = sentil/Sentil@^0.3.0
build_flags = -L${PROJECT_DIR}/.pio/libdeps/pico/Sentil/src/cortex-m0plus -lsentil_embedded
```

PlatformIO compiles `Sentil.cpp` from the library and links the archive named on the `build_flags` line, where the path is the `src/<mcu>` folder for the board's core. The same shape works under the `espidf` framework on an ESP32, with the chip's archive folder.

## Build the archive from source

To rebuild the precompiled archive for a board, or to add one PlatformIO does not ship, install the Rust toolchain and follow `extras/cross_compile.md` in the repository, which lists the target for each core and where the archive goes. Point your build at it with a link flag if you keep it outside the library:

```ini
build_flags = -L path/to/archives -lsentil_embedded
```

## What it runs

The deterministic streaming monitor and the synthesis layer (the numerics, open-loop planning, the receding-horizon controller, and the safety filter). Statistical model checking, the MILP backend, and the GPU paths a board cannot host are left out. See the top-level `README.md` for the API and the heap budget.