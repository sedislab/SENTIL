# sentil-embedded

SENTIL on microcontrollers. SENTIL is a runtime verification engine for Signal Temporal Logic; this library runs its deterministic streaming monitor and its synthesis layer on a board, on the same compiled core the desktop tools use, so a sketch gets the same numbers a workstation would. The directory holds the generic embedded library and a packaging for each ecosystem under `packaging/`, starting with Arduino.

## Scope

A microcontroller cannot host statistical model checking or the GPU paths, so this target leaves those out, and the MILP synthesis backend with them since it needs an external solver. Everything else fits: the streaming STL monitor, the numerics, open-loop planning by gradient or CMA-ES, the receding-horizon controller, and the safety filter, all running on the chip with no standard library.

For monitoring you write a temporal property, feed one sample per loop, and read the quantitative robustness: a positive margin says the property holds and by how much, a negative one how far it has failed. The per-sample cost is flat and the memory is proportional to the formula's windows, not to the length of the stream. For control you give the controller a model and a spec, and each step it plans an input from the live state within a fixed step budget, since a board has no wall clock to bound an anytime search.

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

The `packaging/arduino/examples/` folder carries `BasicMonitor`, which prints the robustness over serial, and `StreamingThreshold`, which lights the built-in LED when a windowed safety property fails.

## Which operators suit a board

A past operator such as `historically` or `once`, and a bounded operator, settle to a verdict from the samples already seen, so they give an answer at every step. A future operator such as `always` or `eventually` needs samples that have not arrived, so online it stays provisional until its window closes and then resolves with that delay. For a real-time alarm, reach for the past-time or bounded forms. An unbounded `eventually` keeps growing its history with no bound, so avoid it on a device.

## Planning and control

The synthesis surface plans an input from a model and a spec, runs a controller online, and shields a nominal input. Build a model, parse a spec, then let the controller plan each step from the live state.

```cpp
sentil_embedded_model_t* model;
double a[1] = {1.0}, b[1] = {1.0}, x0[1] = {2.0};
const char* vars[1] = {"x"};
sentil_embedded_linear_model_create(a, 1, b, 1, x0, vars, 1.0, 5, &model);

sentil_embedded_formula_t* spec;
sentil_embedded_formula_create("always (x > 0)", &spec);

sentil_embedded_controller_t* controller;  // takes ownership of the model and spec
sentil_embedded_controller_create(model, spec, 1, 150, nullptr, &controller);

double state[1] = {x}, u[1];
sentil_embedded_controller_control(controller, state, 1, u);  // u[0] is the input to apply
```

The `Controller` example sketch runs this loop on a board. The controller's step budget is a gradient-step count, not a clock, because a board has none; pick it for the per-step time the chip can spare. Synthesis needs more heap than the bare monitor, so reserve a few more kilobytes.

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

The streaming monitor holds an O(1) amortized per-sample cost with memory proportional to the window. The `packaging/arduino/examples/Benchmark` sketch reports the microseconds per update on a named board; that number is hardware-bound and recorded in the claims ledger with the board it was measured on.

## Credits and license

SENTIL is the work of Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab at Baylor University. Dual licensed under MIT or Apache-2.0; see the `LICENSE-MIT` and `LICENSE-APACHE` files at the repository root. The full documentation lives at the SENTIL site linked from the project repository.