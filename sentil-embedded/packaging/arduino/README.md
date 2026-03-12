<div align="center">

# SENTIL

#### The Arduino library for Probabilistic Signal Temporal Logic

[![Arduino](https://img.shields.io/badge/Arduino-library-blue.svg)](https://www.arduino.cc)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#docs-and-license)

</div>

The SENTIL embedded engine packaged as an Arduino library. It carries `Sentil.h` which gives you the streaming STL monitor, the multi-formula monitor, the ring buffer, offline robustness over a buffered trace, and the on-board synthesis layer. Each release ships a precompiled archive per board architecture, so you install it and start building right away.

## Install

The three paths are the same on Windows, macOS, and Linux; only the menu location moves.

### Package Manager

Open the Arduino IDE, go to Tools, Manage Libraries (the Library Manager icon in the side bar on IDE 2.x), search for Sentil, and click Install. This keeps the library updated alongside the IDE.

### Prebuilt release

Download [`sentil-arduino.zip`](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-arduino.zip) from the Releases tab, then choose Sketch, Include Library, Add .ZIP Library, and pick the file. The bundled sketches then appear under File, Examples, Sentil.

Arduino CLI, on any OS:

```bash
arduino-cli lib install Sentil
# or, from the downloaded zip:
arduino-cli lib install --zip-path sentil-arduino.zip
```

## A first sketch

`BasicMonitor` watches whether a reading has stayed positive since power-on. The past-time `historically` operator settles to a verdict at every step, so each loop produces an answer with no delay.

```cpp
#include <Sentil.h>

static SentilMonitor monitor;
static uint8_t sentil_heap[4096];

static const double readings[] = {3.0, 1.5, 2.0, -0.5, 4.0};
static const size_t reading_count = sizeof(readings) / sizeof(readings[0]);
static unsigned long step = 0;

void setup() {
  Serial.begin(115200);
  while (!Serial) {
  }
  sentil_embedded_init(sentil_heap, sizeof(sentil_heap));
  sentil_embedded_status_t status = monitor.begin("historically (x > 0)");
  if (status != SENTIL_EMBEDDED_OK) {
    Serial.print("could not build the monitor: ");
    Serial.println(sentil_embedded_status_message(status));
    while (true) {
    }
  }
}

void loop() {
  double x = readings[step % reading_count];
  double packed[1] = {x};
  sentil_embedded_robustness_t robustness;
  if (monitor.update((double)step, packed, 1, robustness) == SENTIL_EMBEDDED_OK) {
    Serial.print("x=");
    Serial.print(x);
    Serial.print("  robustness=");
    Serial.print(robustness.value);
    Serial.println(robustness.satisfied ? "  (holds)" : "  (violated)");
  }
  step++;
  delay(1000);
}
```

The pieces, in order. Include `Sentil.h`. Keep the `SentilMonitor` and the byte array it works out of as globals so they outlive `setup`; the 4 KB block here holds a typical monitor with headroom. In `setup`, hand that block to `sentil_embedded_init` once before any monitor exists, then call `monitor.begin` with the formula and stop if the status is not `SENTIL_EMBEDDED_OK`, printing `sentil_embedded_status_message`. In `loop`, pack the reading into a `double[]` in symbol order, call `monitor.update` with a strictly increasing time, and read the verdict back from the `sentil_embedded_robustness_t`: `robustness.value` is the signed margin, and `robustness.satisfied` is true while the property holds.

Open the Serial Monitor at 115200 baud. The reading dips to `-0.5` on the fourth step, and because `historically` keeps the worst value seen so far, the margin drops there and stays down:

```
x=3.00  robustness=3.00  (holds)
x=1.50  robustness=1.50  (holds)
x=2.00  robustness=1.50  (holds)
x=-0.50  robustness=-0.50  (violated)
x=4.00  robustness=-0.50  (violated)
```

## Boards

`library.properties` lists the architectures `samd`, `mbed_rp2040`, `rp2040`, `esp32`, and `stm32`, and the package selects the matching prebuilt archive for the board you compile for. Those archives cover the 32-bit ARM and RISC-V cores with a heap: Cortex-M0+ (RP2040, SAMD21), Cortex-M3, Cortex-M4 and M7 (SAMD51, STM32, Teensy), and the RISC-V ESP32 variants (ESP32-C3 and ESP32-C6). The original Xtensa ESP32 runs the engine too, but its Rust target needs the Espressif toolchain fork, so build its archive from source as below. An 8-bit AVR board such as the classic Uno or Nano has no room for the engine.

If your board's core is not one of these, see "Your board isn't listed" in the top-level `sentil-embedded/README.md`; adding one is a cross-compile and a one-line entry.

## Examples

Four sketches ship under File, Examples, Sentil, each a complete program:

- `BasicMonitor` prints the robustness of a past-time property over serial, the sketch shown above.
- `StreamingThreshold` checks `historically[0, 8](level < 900)` against the A0 input and lights the built-in LED when the recent window crosses the limit, so one stray spike does not trip the alarm.
- `Controller` builds a single-integrator plant and the spec `always (x > 0)`, then plans and applies an input each step with the receding-horizon controller. It reserves 8 KB, since the synthesis layer needs more heap than the bare monitor.
- `Benchmark` times `historically[0, 16](x > 0)` with `micros()` over 2000 updates and reports the mean, minimum, and maximum per-update cost on your board.

## Build the archive from source

To rebuild the shipped archives, or to add a core the package does not cover, install the Rust toolchain and build the static library from `sentil-embedded/rust`:

```
cargo build --release --features mcu --target thumbv7em-none-eabihf
# smallest-flash boards, dropping the text parser and synthesis:
cargo build --release --no-default-features --features mcu --target thumbv6m-none-eabi
```

The archive lands at `rust/target/<triple>/release/libsentil_embedded.a`; copy it to `src/<mcu>/libsentil_embedded.a`, where the Arduino build looks for it. `extras/cross_compile.md` lists the target triple and `src/<mcu>` folder for every core. To confirm a rebuilt archive still reproduces the cross-language robustness oracle, run `make -C sentil-embedded test`.

## Docs and license

See the top-level [`sentil-embedded/README.md`](../../README.md) for the full API, the heap budget, and the operator guidance. Dual licensed under MIT OR Apache-2.0. Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS lab.