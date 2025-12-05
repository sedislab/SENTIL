/*
 * SENTIL streaming monitor for microcontrollers.
 *
 * This header is the surface an Arduino sketch includes. It declares the small C
 * ABI of the prebuilt static library and a thin C++ class over it. The library
 * runs the deterministic STL streaming monitor and nothing else: a board cannot
 * host statistical model checking, synthesis, or a GPU, so those are absent here.
 *
 * Dual licensed under MIT or Apache-2.0; see the LICENSE files.
 */
#ifndef SENTIL_EMBEDDED_H
#define SENTIL_EMBEDDED_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The outcome of a call. Zero is success; the rest name one failure each. */
typedef enum sentil_embedded_status {
    SENTIL_EMBEDDED_OK = 0,
    SENTIL_EMBEDDED_NULL_POINTER = 1,
    SENTIL_EMBEDDED_PARSE = 2,
    SENTIL_EMBEDDED_UNKNOWN_VARIABLE = 3,
    SENTIL_EMBEDDED_PACKED_LENGTH = 4,
    SENTIL_EMBEDDED_UNSUPPORTED = 5,
    SENTIL_EMBEDDED_DECODE = 6,
    SENTIL_EMBEDDED_INTERNAL = 7
} sentil_embedded_status_t;

/* A streaming monitor. Opaque: create one, feed it samples, then destroy it. */
typedef struct sentil_embedded_monitor sentil_embedded_monitor_t;

/* The robustness after one sample. value is the margin (the interval midpoint
   while a temporal window is still open), satisfied is value >= 0, and resolved
   says whether the verdict has settled or still depends on later samples. */
typedef struct sentil_embedded_robustness {
    bool resolved;
    bool satisfied;
    double value;
    double lower;
    double upper;
} sentil_embedded_robustness_t;

/* Hand the monitor a fixed region of memory to allocate from. Call once from
   setup() with a static buffer, before creating any monitor; the region must
   outlive every monitor. A host build has a system allocator and ignores this. */
void sentil_embedded_init(uint8_t *heap, size_t size);

/* Build a monitor from a formula, writing the handle to *out. Returns OK and an
   owned handle on success, or a failure code with *out set to NULL. The formula
   path needs an archive built with the parser; the smallest boards drop it and
   use sentil_embedded_create_compiled instead. */
sentil_embedded_status_t sentil_embedded_create(const char *formula,
                                                sentil_embedded_monitor_t **out);

/* Build a monitor from a host-compiled formula blob, produced by the
   sentil-compile-formula tool. Available in every archive. */
sentil_embedded_status_t sentil_embedded_create_compiled(const uint8_t *bytes, size_t len,
                                                         sentil_embedded_monitor_t **out);

/* Fold one timestamped sample and write the robustness to *out. values holds the
   variables in sentil_embedded_symbol_index order, with n entries. Times must
   strictly increase across calls. */
sentil_embedded_status_t sentil_embedded_update(sentil_embedded_monitor_t *monitor, double time,
                                                const double *values, size_t n,
                                                sentil_embedded_robustness_t *out);

/* The number of variables the formula references; the length values must reach.
   Zero for a NULL handle. */
size_t sentil_embedded_variable_count(const sentil_embedded_monitor_t *monitor);

/* The packed-slice position of a named variable. Writes the index and true when
   the formula uses it, or false when it does not. */
sentil_embedded_status_t sentil_embedded_symbol_index(const sentil_embedded_monitor_t *monitor,
                                                      const char *name, size_t *out_index,
                                                      bool *out_found);

/* Clear all state so the monitor can run a fresh stream. NULL is a no-op. */
void sentil_embedded_reset(sentil_embedded_monitor_t *monitor);

/* Free a monitor from a create call. NULL is a no-op. */
void sentil_embedded_destroy(sentil_embedded_monitor_t *monitor);

/* A short static message for a status code. Never free it. */
const char *sentil_embedded_status_message(int status);

/* Write the library version. NULL out-pointers are skipped. */
void sentil_embedded_version(uint32_t *major, uint32_t *minor, uint32_t *patch);

#ifdef __cplusplus
} /* extern "C" */

/// A streaming monitor with sketch-friendly lifetime management.
///
/// Default construct one, call `begin` with a formula in `setup()`, then call
/// `update` once per `loop()` with the current readings and act on the
/// robustness. The handle is freed in the destructor, so a monitor that lives in
/// a global or static needs no cleanup. The class does not use exceptions, since
/// Arduino builds disable them; every call returns a status instead.
class SentilMonitor {
public:
    SentilMonitor();
    ~SentilMonitor();

    SentilMonitor(const SentilMonitor &) = delete;
    SentilMonitor &operator=(const SentilMonitor &) = delete;

    /// Builds the monitor from a formula string, replacing any current one.
    sentil_embedded_status_t begin(const char *formula);

    /// Builds the monitor from a host-compiled formula blob.
    sentil_embedded_status_t beginCompiled(const uint8_t *bytes, size_t len);

    /// Whether a formula is loaded and ready to monitor.
    bool ready() const;

    /// Folds one sample and writes the robustness to `out`.
    sentil_embedded_status_t update(double time, const double *values, size_t n,
                                    sentil_embedded_robustness_t &out);

    /// The number of variables the formula references.
    size_t variableCount() const;

    /// The packed position of a variable, written to `index` when it is found.
    bool symbolIndex(const char *name, size_t &index) const;

    /// Clears state so the monitor can run a fresh stream.
    void reset();

private:
    sentil_embedded_monitor_t *handle_;
};

#endif /* __cplusplus */
#endif /* SENTIL_EMBEDDED_H */