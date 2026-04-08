<div align="center">

# SENTIL

#### Signal Temporal Logic on a Microcontroller

[![Platform](https://img.shields.io/badge/platform-ARM%20|%20RISC--V-blue.svg)](#install)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

The [`sentil`](../sentil-core) engine built `no_std` for a microcontroller board. Each packaging under `packaging/` carries the generic embedded library and a packaging for Arduino, PlatformIO, ESP-IDF, Zephyr, and bare metal.

## Scope

A microcontroller cannot host statistical model checking or the GPU paths, so this target leaves both out. Everything else can run on the chip: the streaming STL monitor, the multi-formula monitor, offline robustness over a buffered trace, open-loop planning by gradient or CMA-ES, the receding-horizon controller, and the safety filter.

## Your first monitor

Include `Sentil.h`, hand the monitor a fixed block of memory once in `setup()`, then update it each loop. The `SentilMonitor` class owns the handle and frees it in its destructor; a monitor in a global needs no cleanup.

```cpp
#include <Sentil.h>

static SentilMonitor monitor;
static uint8_t sentil_heap[4096];

static const double readings[] = {3.0, 1.5, 2.0, -0.5, 4.0};
static unsigned long step = 0;

void setup() {
  Serial.begin(115200);
  sentil_embedded_init(sentil_heap, sizeof(sentil_heap));
  monitor.begin("historically (x > 0)");
}

void loop() {
  double x = readings[step % 5];
  double packed[1] = {x};
  sentil_embedded_robustness_t r;
  if (monitor.update((double)step, packed, 1, r) == SENTIL_EMBEDDED_OK) {
    Serial.print("x=");
    Serial.print(x);
    Serial.print("  robustness=");
    Serial.print(r.value);
    Serial.println(r.satisfied ? "  (holds)" : "  (violated)");
  }
  step++;
  delay(1000);
}
```

Over the serial monitor at 115200 baud that prints:

```bash
x=3.00  robustness=3.00  (holds)
x=1.50  robustness=1.50  (holds)
x=2.00  robustness=1.50  (holds)
x=-0.50  robustness=-0.50  (violated)
x=4.00  robustness=-0.50  (violated)
```

The robustness is the running minimum of `x`, since `historically (x > 0)` asks whether `x` has stayed positive since power-on. The dip to `-0.5` fails the property by `0.5`, and once it has failed it stays failed. A positive value is the margin by which the property still holds; a negative one is how far it has been violated. This is the `BasicMonitor` sketch; `StreamingThreshold` lights the built-in LED when a windowed safety property fails.

## More than one property at a time

The same header gives a multi-formula monitor that folds one sample into a whole set of properties at once, a fixed-size ring buffer with running mean, variance, min, and max for smoothing or thresholding a signal before the monitor sees it, and offline calls that evaluate a formula over a captured trace: its scalar robustness, the per-sample robustness signal, and the intervals where it is violated. A formula bank does the same for several properties over one trace. All of it is declared in `Sentil.h` and exercised in `tests/test_surface.cpp`.

## Planning and control

The synthesis surface plans an input from a model and a spec, runs a controller online, and shields a nominal input. Build a linear model, parse a spec, then let the controller plan each step from the live state.

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
sentil_embedded_controller_control(controller, state, 1, u);
```

For the single integrator `x_{t+1} = x_t + u_t` under a spec asking `x` to stay above zero, the controller emits an input near the upper edge of its bounds to counter a disturbance pulling `x` down, and holds `x` positive step after step. The `Controller` example sketch runs this loop on a board. The step budget is a gradient-step count, not a clock, because a board has none; pick it for the per-step time the chip can spare. Synthesis needs more heap than the bare monitor, so reserve a few more kilobytes; the example reserves eight.

## SENTIL for the smallest boards, and premade specifications

A board short on flash can leave out the formula parser and load a formula compiled on a workstation. Run the bundled tool, paste the byte array it prints into the sketch, and call `beginCompiled` instead of `begin`.

```bash
cargo run --features std --bin sentil-compile-formula -- "historically[0, 8](level < 900)"
```

It prints a `SENTIL_FORMULA` byte array and `SENTIL_FORMULA_LEN`; paste both into the sketch and load them with `beginCompiled(SENTIL_FORMULA, SENTIL_FORMULA_LEN)`. The same tool reaches the premade specifications library, so a board gets a vetted property without the whole library shipping in flash.

```bash
cargo run --features "std specs" --bin sentil-compile-formula -- --list-specs
cargo run --features "std specs" --bin sentil-compile-formula -- --spec controls/overshoot --param max_overshoot=0.2 -o overshoot.bin
```

A spec that resolves to a probabilistic formula is refused, since a board cannot decide one; pick a deterministic variant.

## Errors

Bad input comes back as a status code. A malformed formula gives `SENTIL_EMBEDDED_PARSE`, a packed update shorter than the formula's variables gives `SENTIL_EMBEDDED_PACKED_LENGTH`, and `sentil_embedded_status_message` turns any code into a short string. Exhausting the heap halts the board, so size the region passed to `sentil_embedded_init` for the worst-case window and leave headroom.

## Install

Pick the packaging for your toolchain. Each ships the header, the `Sentil.cpp` wrapper, and a precompiled archive per board, so no Rust toolchain is needed, and each packaging README has the per-OS detail.

### Package manager

- Arduino: search for Sentil in the Library Manager. See [`packaging/arduino/README.md`](packaging/arduino/README.md).
- PlatformIO: add `lib_deps = sentil/Sentil` to `platformio.ini`. See [`packaging/platformio/README.md`](packaging/platformio/README.md).
- ESP-IDF: add it through the component manager. See [`packaging/espidf/README.md`](packaging/espidf/README.md).

### Prebuilt release

Download the archive for your packaging from the latest release and add it the way that ecosystem expects. This is the path for Zephyr and bare metal, which have no registry, and an alternative for the three above.

- Arduino: [sentil-arduino.zip](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-arduino.zip)
- PlatformIO: [sentil-platformio.zip](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-platformio.zip)
- ESP-IDF: [sentil-esp-idf.zip](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-esp-idf.zip)
- Zephyr: [sentil-zephyr.tar.gz](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-zephyr.tar.gz). See [`packaging/zephyr/README.md`](packaging/zephyr/README.md).
- Bare metal: [sentil-bare-metal.tar.gz](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-bare-metal.tar.gz). See [`packaging/bare-metal/README.md`](packaging/bare-metal/README.md).

The prebuilt archives cover the 32-bit ARM and RISC-V cores with a heap: Cortex-M0+ (RP2040, SAMD21), Cortex-M3, Cortex-M4 and M7 (SAMD51, STM32, Teensy), and the RISC-V ESP32 variants (ESP32-C3, ESP32-C6). The original Xtensa ESP32 runs the engine too, but its Rust target needs the Espressif toolchain fork, so it builds from source; `extras/cross_compile.md` has the steps.

### Build from source

Building from source needs a Rust toolchain, and it is how you add a board, rebuild an archive, or run the tests. Clone the repository, then build and test the library:

```bash
git clone https://github.com/sedislab/SENTIL
cd SENTIL
make -C sentil-embedded test        # host oracle and surface tests through the C ABI
make -C sentil-embedded leakcheck    # the same under valgrind, no leaks
```

`extras/cross_compile.md` lists the target triple for each core and where each packaging expects the archive.

### Your board isn't listed

The engine runs on any 32-bit ARM or RISC-V core with a heap and a C toolchain. To bring up a core that is not listed:

1. Find its Rust target triple (for example `thumbv7em-none-eabihf` for Cortex-M4F, `riscv32imac-unknown-none-elf` for a RISC-V core with atomics).
2. Build the archive: `cargo build --release --features mcu --target <triple> --manifest-path sentil-embedded/rust/Cargo.toml`, adding `--no-default-features` on the smallest cores to drop the parser and synthesis.
3. Link the resulting `libsentil_embedded.a` and `Sentil.cpp` into your project, providing a `critical-section` implementation if your target needs one (the ARM and RISC-V single-core ones are bundled).

`extras/cross_compile.md` walks through this with the exact commands, and the bare-metal packaging README covers the few stubs a freestanding target must provide.

## Documentation

The [documentation site](https://sentil.pages.dev) carries the guides, the specification syntax, and the long-form [tutorial](https://sentil.pages.dev/docs/start/tutorial). The full C ABI and the `SentilMonitor` class are documented inline in [`include/Sentil.h`](include/Sentil.h), each packaging has its own README under `packaging/`, and `extras/cross_compile.md` covers bringing up a new core.

## Contributing

The host tests run the monitor and synthesis off the board through the C ABI:

```bash
make -C sentil-embedded test
```

The pull-request flow is in the repository [CONTRIBUTING.md](../CONTRIBUTING.md).

## Citation

If SENTIL is useful in your work, please cite the paper:

```bibtex
@misc{quansah2026sentilruntimeverificationtool,
    title={SENTIL: A Runtime Verification Tool for Probabilistic Temporal Logic},
    author={Paapa Kwesi Quansah and Ernest Bonnah},
    year={2026},
    eprint={2605.21676},
    archivePrefix={arXiv},
    primaryClass={cs.LO},
    url={https://arxiv.org/abs/2605.21676}
}
```

## License

SENTIL is by Paapa Kwesi Quansah and Ernest Bonnah at the SEDIS lab, Baylor University. It is dual licensed under either [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at your option.