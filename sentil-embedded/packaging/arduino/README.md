# Sentil for Arduino

Download the latest [sentil-arduino.zip](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-arduino.zip) from the Releases tab, or build the archive from source as below.

The SENTIL embedded library packaged as an Arduino library. It carries the `Sentil.h` surface: the streaming STL monitor, the multi-formula monitor, the ring buffer, offline robustness over a buffered trace, and the on-board synthesis layer. The package ships a precompiled archive per board architecture, so no Rust toolchain is needed to use it.

## Install

The steps are the same on Windows, macOS, and Linux; only the menu lives in slightly different places.

Through the Library Manager: open the Arduino IDE, go to Tools, Manage Libraries (or the Library Manager icon in the side bar on IDE 2.x), search for Sentil, and install. This is the simplest path and keeps the library updated.

From the release zip: download `sentil-arduino.zip` from the link above, then in the IDE choose Sketch, Include Library, Add .ZIP Library, and pick the file. The examples appear under File, Examples, Sentil.

With Arduino CLI, on any OS:

```
arduino-cli lib install Sentil
# or, from the downloaded zip:
arduino-cli lib install --zip-path sentil-arduino.zip
```

## Boards

The supported cores are the 32-bit ARM and RISC-V families with a heap: Cortex-M0+ (RP2040, SAMD21), Cortex-M3, Cortex-M4 and M7 (SAMD51, STM32, Teensy), and the ESP32 and ESP32-C3. The package selects the matching archive automatically from the board you compile for. An 8-bit AVR board (the classic Uno and Nano) does not have the room for the engine.

If your board's core is not one of these, see "Your board isn't listed" in the top-level `sentil-embedded/README.md`; adding one is a cross-compile and a one-line entry.

## Build the archive from source

To rebuild the precompiled archives, or to add a core the package does not ship, install the Rust toolchain and follow `extras/cross_compile.md` in the repository, which lists the target for each core and where Arduino expects the archive (`src/<mcu>/libsentil_embedded.a`). The host test that proves the monitor reproduces the cross-language oracle runs with `make -C sentil-embedded test`.

## Examples

`BasicMonitor` prints the robustness over serial; `StreamingThreshold` lights the built-in LED when a windowed safety property fails; `Controller` runs the receding-horizon planner; `Benchmark` reports the microseconds per update on your board. See the top-level `README.md` for the API and the heap budget.