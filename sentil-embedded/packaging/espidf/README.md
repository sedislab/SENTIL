# Sentil for ESP-IDF

Download the latest [sentil-esp-idf.zip](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-esp-idf.zip) from the Releases tab, or build the archive from source as below.

SENTIL as an ESP-IDF component: the streaming STL monitor and the on-board synthesis layer, on an ESP32.

## Add it to a project

Place this directory at `components/sentil` in your project, or add it through the component manager. Build the precompiled archive for your chip with the recipe in `../../extras/cross_compile.md` and drop it at `prebuilt/<idf_target>/libsentil_embedded.a`, or set `SENTIL_ARCHIVE` in your build. The Xtensa chips (esp32, esp32s3) need the Espressif Rust target; the RISC-V chips (esp32c3, esp32c6) build with the stock `riscv32imc`/`riscv32imac` targets.

```c
#include "Sentil.h"

void app_main(void) {
    static uint8_t heap[8192];
    sentil_embedded_init(heap, sizeof(heap));
    sentil_embedded_monitor_t *monitor = NULL;
    sentil_embedded_create("historically (x > 0)", &monitor);
    // feed samples from a task loop ...
}
```

The archive bundles its own single-core critical-section, so it links cleanly against the ESP-IDF runtime; keep SENTIL calls on one core. See the top-level `README.md` for the full API.