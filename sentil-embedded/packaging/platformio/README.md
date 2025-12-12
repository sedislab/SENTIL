# Sentil for PlatformIO

The SENTIL embedded library packaged for PlatformIO. It carries the same `Sentil.h` surface as the Arduino packaging: the streaming STL monitor and the on-board synthesis layer.

## Add it to a project

The released package bundles the header, the `Sentil.cpp` wrapper, and a precompiled archive per board architecture, so no Rust toolchain is needed. Add it to `platformio.ini`:

```ini
[env:pico]
platform = raspberrypi
board = pico
framework = arduino
lib_deps = sentil/Sentil@^1.0.0
```

PlatformIO compiles `Sentil.cpp` and links the archive matching the board's core. The same `lib_deps` line works under the `espidf` framework on an ESP32.

## Build the archive from source

To rebuild the precompiled archive for a board, or to add one PlatformIO does not ship, install the Rust toolchain and follow `extras/cross_compile.md` in the repository, which lists the target for each core and where the archive goes. Point your build at it with a link flag if you keep it outside the library:

```ini
build_flags = -L path/to/archives -lsentil_embedded
```

## What it runs

The deterministic streaming monitor and the synthesis layer (the numerics, open-loop planning, the receding-horizon controller, and the safety filter). Statistical model checking, the MILP backend, and the GPU paths a board cannot host are left out. See the top-level `README.md` for the API and the heap budget.