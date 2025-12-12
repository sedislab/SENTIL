# Sentil for Zephyr

SENTIL as a Zephyr module: the streaming STL monitor and the on-board synthesis layer.

## Add it to a project

Point Zephyr at this directory as an extra module, enable `CONFIG_SENTIL`, build the archive for the board's core with `../../extras/cross_compile.md`, and pass its path as `SENTIL_ARCHIVE`.

```
west build -b nucleo_f401re my_app \
  -- -DZEPHYR_EXTRA_MODULES=$PWD/sentil-embedded/packaging/zephyr \
     -DSENTIL_ARCHIVE=$PWD/lib/cortex-m4/libsentil_embedded.a
```

In `prj.conf`:

```
CONFIG_SENTIL=y
CONFIG_CPP=y
CONFIG_REQUIRES_FULL_LIBC=y
```

```c
#include <Sentil.h>

int main(void) {
    static uint8_t heap[8192];
    sentil_embedded_init(heap, sizeof(heap));
    sentil_embedded_monitor_t *monitor = NULL;
    sentil_embedded_create("historically (x > 0)", &monitor);
    return 0;
}
```

The archive bundles its own single-core critical-section, so it links against the Zephyr kernel without a separate provider; keep SENTIL calls on one core. See the top-level `README.md` for the full API.