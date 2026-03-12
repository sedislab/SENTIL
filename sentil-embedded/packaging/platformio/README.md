<div align="center">

# SENTIL

#### The PlatformIO library for probabilistic Signal Temporal Logic

[![PlatformIO](https://img.shields.io/badge/PlatformIO-library-blue.svg)](https://platformio.org)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#)

</div>

The SENTIL embedded library packaged for PlatformIO. It carries the same `Sentil.h` surface as the other embedded packagings: the streaming STL monitor, the multi-formula monitor, the ring buffer, offline robustness over a buffered trace, and the on-board synthesis layer.

Pull the released package in through `lib_deps`, or build the board archive from source as below. The release bundles the header, the `Sentil.cpp` wrapper, and a precompiled archive per core, so no Rust toolchain is needed.

## Add it to a project

Declare the library, then link the core archive for your board with a build flag. This env targets a Raspberry Pi Pico (RP2040) on the earlephilhower core:

```ini
[env:pico]
platform = https://github.com/maxgerhardt/platform-raspberrypi.git
board = pico
framework = arduino
board_build.core = earlephilhower
lib_deps = sentil/Sentil@^0.3.0
build_flags = -L${PROJECT_DIR}/.pio/libdeps/pico/Sentil/src/cortex-m0plus -lsentil_embedded
```

PlatformIO fetches the library, compiles `Sentil.cpp` out of it, and links the archive that `-lsentil_embedded` names from the directory the `-L` flag adds. Two parts of that path change per project. The `pico` segment is the env name, so it tracks whatever you write in `[env:...]`. The `cortex-m0plus` segment is the `src/<mcu>` folder that holds the archive for the board's core.

## Choosing the core folder

The RP2040 is a Cortex-M0+, so its archive lives in `src/cortex-m0plus`. Point `-L` at the folder that matches your board's core:

- `cortex-m0plus` for RP2040 and SAMD21
- `cortex-m4` for SAMD51 and STM32F4, `cortex-m7` for STM32H7 and Teensy 4.x
- `esp32c6` for the ESP32-C6, and the matching chip folder for the other RISC-V ESP32 variants

The library declares the `arduino` and `espidf` frameworks and the `raspberrypi`, `espressif32`, `ststm32`, and `atmelsam` platforms. The same env shape works under `espidf` on an ESP32, with `-L` pointed at that chip's archive folder.

## Build the archive from source

To rebuild an archive, or to add a core the release does not ship, install the Rust toolchain and follow `extras/cross_compile.md` in the repository. It gives the target triple for each core and the `src/<mcu>` folder the archive belongs in. If you keep the archive outside the fetched library, aim the link flag straight at it:

```ini
build_flags = -L path/to/archives -lsentil_embedded
```

For the API, the heap budget, and which operators suit a board, see the top-level `sentil-embedded/README.md`. By Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS lab. Dual licensed under MIT OR Apache-2.0.