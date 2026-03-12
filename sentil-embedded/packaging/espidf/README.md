<div align="center">

# SENTIL

#### The ESP-IDF component for Probabilistic Signal Temporal Logic

[![ESP-IDF](https://img.shields.io/badge/ESP--IDF-component-blue.svg)](https://docs.espressif.com/projects/esp-idf)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#)

</div>

Download the latest [sentil-esp-idf.zip](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-esp-idf.zip) from the Releases tab, or build the archive from source as below.

SENTIL as an ESP-IDF component: the streaming STL monitor and the on-board synthesis layer, on an ESP32.

## Add it to a project

Place this directory at `components/sentil` in your project, or add it through the component manager; it carries an `idf_component.yml` manifest. The component's CMake compiles the `Sentil.cpp` wrapper and links a per-chip archive, so no Rust toolchain is needed to use a release.

Build that archive for your target with the recipe in `../../extras/cross_compile.md` and drop it at `prebuilt/<idf_target>/libsentil_embedded.a`, which is where the CMake looks by default, or point `SENTIL_ARCHIVE` at it. The component supports `esp32`, `esp32s3`, `esp32c3`, and `esp32c6`. The Xtensa chips (`esp32`, `esp32s3`) need the Espressif Rust target from the `espup` toolchain fork; the RISC-V chips (`esp32c3`, `esp32c6`) build with the stock `riscv32imc`/`riscv32imac` targets.

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

See the top-level [`README.md`](../../README.md) for the full API and the heap budget. Dual licensed under MIT or Apache-2.0; SENTIL is the work of Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab at Baylor University.