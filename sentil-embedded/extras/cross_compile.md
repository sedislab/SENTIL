# Building the static library for a board

The Arduino library ships a precompiled static archive per architecture under
`src/<mcu>/libsentil_embedded.a`, plus `src/Sentil.h` and `src/Sentil.cpp`. The
release workflow builds these archives; this file is the recipe to reproduce them
or to add a board.

## Toolchain

The archive is a `no_std` Rust static library. Install the Rust toolchain and the
targets you need:

```
rustup target add thumbv7em-none-eabihf      # Cortex-M4F / M7 (SAMD51, STM32F4, Teensy)
rustup target add thumbv6m-none-eabi          # Cortex-M0+ (RP2040, SAMD21)
rustup target add riscv32imac-unknown-none-elf # RV32 with atomics (ESP32-C6, some RP2350 builds)
```

ESP32 Xtensa needs the Espressif fork rather than a stock target. Install it with
`espup` and build with `+esp` against `xtensa-esp32-none-elf`; the recipe is
otherwise the same.

## Build

From `sentil-embedded/rust`, build the archive for the board's core, with the
allocator and the parser on:

```
cargo build --release --features mcu --target thumbv7em-none-eabihf
```

The archive lands at `rust/target/<triple>/release/libsentil_embedded.a`. Drop the
parser on the smallest boards to save flash and load a host-compiled formula
instead (see below):

```
cargo build --release --no-default-features --features mcu --target thumbv6m-none-eabi
```

## Placing the archive

Arduino's precompiled-library mechanism looks for the archive in a folder named
for the board's `build.mcu`. Copy the built archive there:

| Rust target | `src/<mcu>/` folder | Boards |
| --- | --- | --- |
| thumbv7em-none-eabihf | `src/cortex-m4/` | SAMD51, STM32F4, Teensy 3.x |
| thumbv7em-none-eabihf | `src/cortex-m7/` | Teensy 4.x, STM32H7 |
| thumbv6m-none-eabi | `src/cortex-m0plus/` | RP2040, SAMD21 |
| riscv32imac-unknown-none-elf | `src/esp32c6/` | ESP32-C6 |
| xtensa-esp32-none-elf | `src/esp32/` | ESP32 |

```
cp rust/target/thumbv7em-none-eabihf/release/libsentil_embedded.a src/cortex-m4/
```

## Two link-time dependencies

The archive expects the board's runtime to provide two symbols, which every HAL
already does:

- A `critical-section` implementation, for the allocator. On Cortex-M the
  `cortex-m` crate's `critical-section-single-core` feature provides it; the
  ESP-IDF and the `riscv` crate provide their own.
- The standard `memcpy`/`memset` and the `libm` math symbols, which the Arduino
  core links.

## Heap budget

The monitor allocates from the fixed region passed to `sentil_embedded_init`. A
formula's state is proportional to its temporal windows, not the trace, so a few
kilobytes hold a typical monitor; the examples reserve 4 KB. A bounded operator is
predictable; an unbounded `eventually` keeps growing its history, so prefer
bounded or past operators on a device. Exhausting the heap halts the board, so
size the region for the worst-case window and leave headroom.

## The smallest boards: a host-compiled formula

A board with little flash can drop the parser. Compile the formula on a
workstation and paste the bytes into the sketch:

```
cargo run --features std --bin sentil-compile-formula -- "historically[0, 8](level < 900)"
```

It prints a `SENTIL_FORMULA` byte array and the packed variable order. Load it with
`beginCompiled(SENTIL_FORMULA, SENTIL_FORMULA_LEN)` instead of `begin`, and build
the archive with `--no-default-features --features mcu`.