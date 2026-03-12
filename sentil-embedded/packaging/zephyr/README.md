<div align="center">

# SENTIL

#### The Zephyr module for probabilistic Signal Temporal Logic

[![Zephyr](https://img.shields.io/badge/Zephyr-module-blue.svg)](https://www.zephyrproject.org)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#)

</div>

SENTIL builds as a Zephyr module that carries the streaming STL monitor and the on-board synthesis layer. The Rust engine ships as a precompiled static archive, one per core, and a small C++ wrapper compiles into your application from the module.

Download the latest [sentil-zephyr.tar.gz](https://github.com/sedislab/SENTIL/releases/latest/download/sentil-zephyr.tar.gz) from the Releases tab, or point Zephyr at this directory in the source tree and build the archive yourself as below.

## Add it to a project

Register this directory as an extra module with `ZEPHYR_EXTRA_MODULES`, turn on `CONFIG_SENTIL`, and pass the path to the archive for your board's core as `SENTIL_ARCHIVE`. Build that archive with the recipe in `../../extras/cross_compile.md`; a Cortex-M4 board such as the Nucleo F401RE uses the `cortex-m4` target.

```
west build -b nucleo_f401re my_app \
  -- -DZEPHYR_EXTRA_MODULES=$PWD/sentil-embedded/packaging/zephyr \
     -DSENTIL_ARCHIVE=$PWD/lib/cortex-m4/libsentil_embedded.a
```

The module is registered through `zephyr/module.yml`, and its Kconfig symbol is `SENTIL`. Enabling `CONFIG_SENTIL` adds the include path, compiles the C++ wrapper into your image, and links the archive; leaving it off drops SENTIL from the build.

`prj.conf` needs the C++ toolchain and the full C library, since the wrapper is C++ and the engine calls into libm:

```
CONFIG_SENTIL=y
CONFIG_CPP=y
CONFIG_REQUIRES_FULL_LIBC=y
```

A monitor over `historically (x > 0)` reads:

```c
#include <Sentil.h>

int main(void) {
    static uint8_t heap[8192];
    sentil_embedded_init(heap, sizeof(heap));

    sentil_embedded_monitor_t *monitor = NULL;
    if (sentil_embedded_create("historically (x > 0)", &monitor) != SENTIL_EMBEDDED_OK) {
        return 0;
    }

    double t = 0.0;
    for (;;) {
        double values[1] = { read_sensor() };
        sentil_embedded_robustness_t r;
        if (sentil_embedded_update(monitor, t, values, 1, &r) == SENTIL_EMBEDDED_OK) {
            // act on r.satisfied and r.value
        }
        t += 1.0;
    }
}
```

Call `sentil_embedded_init` once before you create a monitor, and give the heap room for the formula's windows plus headroom. Times passed to `sentil_embedded_update` must strictly increase, and the packed `values` array carries one entry per variable in symbol-index order. The archive bundles its own single-core critical-section, so it links against the Zephyr kernel without a separate provider; keep every SENTIL call on one core.

For the full C ABI and the C++ `SentilMonitor` class, see the [main sentil-embedded README](../../README.md). Dual licensed under MIT OR Apache-2.0. Built by Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS lab.