<div align="center">

# SENTIL

#### The ESP-IDF component for Probabilistic Signal Temporal Logic

[![ESP-IDF](https://img.shields.io/badge/ESP--IDF-component-blue.svg)](https://docs.espressif.com/projects/esp-idf)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#)

</div>

The SENTIL embedded library packaged as an ESP-IDF component. It carries `Sentil.h` which provides the streaming STL monitor, the multi-formula monitor, the ring buffer, offline robustness over a buffered trace, and the on-board synthesis layer, on an ESP32.

The component's CMake compiles the `Sentil.cpp` wrapper and links a per-chip archive. Add it through the component manager, drop in a release archive, or build the board archive from source as below.

## Install

### Package manager

ESP-IDF's component manager pulls the component from the ESP Component Registry. From your project directory:

```bash
idf.py add-dependency "sedislab/sentil"
```

The manager fetches it into `managed_components/`. You can also add a `sedislab/sentil` entry under `dependencies` in the project's own `idf_component.yml`.

### Prebuilt release

Download [sentil-esp-idf.zip](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-esp-idf.zip) from the GitHub release and unpack it into `components/sentil` in your project. The component looks for the per-chip archive at `prebuilt/<idf_target>/libsentil_embedded.a`, which is where the zip places it; point `SENTIL_ARCHIVE` at a different archive to override that default. The release prebuilds the RISC-V chips `esp32c3` and `esp32c6`; for the Xtensa `esp32` and `esp32s3`, build the archive from source.

### Build from source

Build the archive for your chip from `sentil-embedded/rust`. The RISC-V chips use the stock bare-metal targets:

```bash
git clone https://github.com/sedislab/SENTIL
cd sentil-embedded/rust
cargo build --release --features mcu --target riscv32imc-unknown-none-elf   # esp32c3
cargo build --release --features mcu --target riscv32imac-unknown-none-elf  # esp32c6
```

The Xtensa chips `esp32` and `esp32s3` need the Espressif Rust target rather than a stock one. Install it with `espup` and build with `+esp` against `xtensa-esp32-none-elf` or `xtensa-esp32s3-none-elf`. Copy the result from `rust/target/<triple>/release/libsentil_embedded.a` to `prebuilt/<idf_target>/libsentil_embedded.a`, or set `SENTIL_ARCHIVE` to its path. `extras/cross_compile.md` carries the full recipe.

## A first monitor

Hand the monitor a fixed block of memory once, create it from a formula, then push one sample per step. Time must strictly increase, and the packed values follow the formula's variable order.

```c
#include "Sentil.h"

static uint8_t heap[8192];

void app_main(void) {
    sentil_embedded_init(heap, sizeof(heap));

    sentil_embedded_monitor_t *monitor = NULL;
    if (sentil_embedded_create("historically (x > 0)", &monitor) != SENTIL_EMBEDDED_OK) {
        return;  // sentil_embedded_status_message() turns the code into a string
    }

    double t = 0.0;
    for (;;) {
        double values[1] = { read_sensor() };
        sentil_embedded_robustness_t r;
        if (sentil_embedded_update(monitor, t, values, 1, &r) == SENTIL_EMBEDDED_OK) {
            // r.value is the margin; r.satisfied says whether the property holds
        }
        t += 1.0;
    }
}
```

The archive bundles its own single-core critical-section, so it links against the ESP-IDF runtime without a separate provider; keep SENTIL calls on one core.

See the top-level [`README.md`](../../README.md) for the full API, the heap budget, and which operators suit a board. Dual licensed under MIT or Apache-2.0; SENTIL is the work of Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab at Baylor University.