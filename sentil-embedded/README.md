# sentil-embedded

The SENTIL streaming monitor for microcontrollers. SENTIL is a runtime verification engine for Signal Temporal Logic; this library runs its deterministic streaming monitor on a board, on the same compiled core the desktop tools use, so a sketch gets the same robustness numbers a workstation would.

## Scope

A microcontroller cannot host statistical model checking, controller synthesis, or the GPU paths, so this target leaves them out and ships the streaming STL monitor in full. You write a temporal property, feed one sample per loop, and read the quantitative robustness: a positive margin says the property holds and by how much, a negative one says how far it has failed. The per-sample cost is flat and the memory is proportional to the formula's windows, not to the length of the stream, which is the whole point on a device.

## A first monitor

Include `Sentil.h`, hand the monitor a fixed block of memory once in `setup()`, then update it each loop.

```cpp
#include <Sentil.h>

static SentilMonitor monitor;
static uint8_t heap[4096];

void setup() {
  Serial.begin(115200);
  sentil_embedded_init(heap, sizeof(heap));
  monitor.begin("historically (x > 0)");  // has x stayed positive so far
}

void loop() {
  double packed[1] = { (double)analogRead(A0) };
  sentil_embedded_robustness_t r;
  if (monitor.update(millis() / 1000.0, packed, 1, r) == SENTIL_EMBEDDED_OK) {
    Serial.println(r.value);
  }
  delay(100);
}
```

The `examples/` folder carries `BasicMonitor`, which prints the robustness over serial, and `StreamingThreshold`, which lights the built-in LED when a windowed safety property fails.

## Which operators suit a board

A past operator such as `historically` or `once`, and a bounded operator, settle to a verdict from the samples already seen, so they give an answer at every step. A future operator such as `always` or `eventually` needs samples that have not arrived, so online it stays provisional until its window closes and then resolves with that delay. For a real-time alarm, reach for the past-time or bounded forms. An unbounded `eventually` keeps growing its history with no bound, so avoid it on a device.

## Install

Through the Arduino Library Manager, search for Sentil, or add the release zip with Sketch, Include Library, Add .ZIP Library. The library ships a precompiled archive per board architecture, so no Rust toolchain is needed to use it.

The supported cores are the 32-bit ARM and RISC-V families with a heap: Cortex-M0+ (RP2040, SAMD21), Cortex-M4 and M7 (SAMD51, STM32, Teensy), and the ESP32. An 8-bit AVR board does not have the room for the engine.

## Build from source

The library links a `no_std` Rust static library built from `sentil-core`. To rebuild the archives or add a board, install the Rust toolchain and follow `extras/cross_compile.md`, which lists the target for each core and where Arduino expects the archive. The host oracle test, which proves the monitor reproduces the cross-language oracle through the embedded C ABI, runs with `make -C sentil-embedded test`.

## The smallest boards

A board short on flash can leave out the formula parser and load a formula compiled on a workstation. Run the bundled tool, paste the byte array it prints into the sketch, and call `beginCompiled` instead of `begin`.

```
cargo run --features std --bin sentil-compile-formula -- "historically[0, 8](level < 900)"
```

## Errors

Bad input comes back as a status code, never a fault. A malformed formula gives `SENTIL_EMBEDDED_PARSE`, a packed update shorter than the formula's variables gives `SENTIL_EMBEDDED_PACKED_LENGTH`, and `sentil_embedded_status_message` turns any code into a short string. Exhausting the heap halts the board, so size the region passed to `sentil_embedded_init` for the worst-case window and leave headroom.

## Performance

The streaming monitor holds an O(1) amortized per-sample cost with memory proportional to the window. The `examples/Benchmark` sketch reports the microseconds per update on a named board; that number is hardware-bound and recorded in the claims ledger with the board it was measured on.

## Credits and license

SENTIL is the work of Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab at Baylor University. Dual licensed under MIT or Apache-2.0; see the `LICENSE-MIT` and `LICENSE-APACHE` files at the repository root. The full documentation lives at the SENTIL site linked from the project repository.