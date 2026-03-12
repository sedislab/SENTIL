<div align="center">

# SENTIL

#### The PlatformIO library for Probabilistic Signal Temporal Logic

[![PlatformIO](https://img.shields.io/badge/PlatformIO-library-blue.svg)](https://platformio.org)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#docs-and-license)

</div>

The SENTIL embedded engine packaged for PlatformIO. It carries `Sentil.h` which provides the streaming STL monitor, the multi-formula monitor, the ring buffer, offline robustness over a buffered trace, and the on-board synthesis layer for PlatformIO.

## Install

Each package ships the header, the `Sentil.cpp` wrapper, and a precompiled archive per core. Whichever way the `Sentil` folder reaches your project, link its core archive for the board with a build flag; the folder to point at is covered in "Choosing the core folder" below.

### Package manager

Declare the dependency in `platformio.ini` and PlatformIO pulls it from the registry. This env targets a Raspberry Pi Pico (RP2040) on the earlephilhower core:

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

### Prebuilt release

To vendor the library rather than fetch it, download [`sentil-platformio.zip`](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-platformio.zip) from the Releases tab and unzip it into `lib/Sentil` in your project, so the header, the wrapper, and the per-core archives sit under that folder. Drop the `lib_deps` line and point `-L` at the vendored copy:

```ini
build_flags = -L${PROJECT_DIR}/lib/Sentil/src/cortex-m0plus -lsentil_embedded
```

### Build from source

To rebuild an archive, or to add a core the release does not ship, build the static library from `sentil-embedded/rust`:

```bash
cargo build --release --features mcu --target thumbv7em-none-eabihf
# smallest-flash boards, dropping the text parser and synthesis:
cargo build --release --no-default-features --features mcu --target thumbv6m-none-eabi
```

The archive lands at `rust/target/<triple>/release/libsentil_embedded.a`, and `extras/cross_compile.md` lists the target triple and the `src/<mcu>` folder for every core. Copy it into the fetched or vendored library's `src/<mcu>` folder, or keep it anywhere and aim the link flag straight at it:

```ini
build_flags = -L path/to/archives -lsentil_embedded
```

## Choosing the core folder

The RP2040 is a Cortex-M0+, so its archive lives in `src/cortex-m0plus`. Point `-L` at the folder that matches your board's core:

- `cortex-m0plus` for RP2040 and SAMD21
- `cortex-m4` for SAMD51 and STM32F4, `cortex-m7` for STM32H7 and Teensy 4.x
- `esp32c6` for the ESP32-C6, and the matching chip folder for the other RISC-V ESP32 variants

The library declares the `arduino` and `espidf` frameworks and the `raspberrypi`, `espressif32`, `ststm32`, and `atmelsam` platforms. The same env shape works under `espidf` on an ESP32, with `-L` pointed at that chip's archive folder.

## Docs and license

For the API, the heap budget, and which operators suit a board, see the top-level [`sentil-embedded/README.md`](../../README.md). Dual licensed under MIT OR Apache-2.0. Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS lab.