/*
 * SENTIL streaming monitor for microcontrollers.
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

/* The outcome of a call. */
typedef enum sentil_embedded_status {
    SENTIL_EMBEDDED_OK = 0,
    SENTIL_EMBEDDED_NULL_POINTER = 1,
    SENTIL_EMBEDDED_PARSE = 2,
    SENTIL_EMBEDDED_UNKNOWN_VARIABLE = 3,
    SENTIL_EMBEDDED_PACKED_LENGTH = 4,
    SENTIL_EMBEDDED_UNSUPPORTED = 5,
    SENTIL_EMBEDDED_DECODE = 6,
    SENTIL_EMBEDDED_INTERNAL = 7,
    SENTIL_EMBEDDED_INVALID_CONFIG = 8
} sentil_embedded_status_t;

/* A streaming monitor. */
typedef struct sentil_embedded_monitor sentil_embedded_monitor_t;

/* The robustness after one sample. */
typedef struct sentil_embedded_robustness {
    bool resolved;
    bool satisfied;
    double value;
    double lower;
    double upper;
} sentil_embedded_robustness_t;

/* Hand the monitor a fixed region of memory to allocate from, before creating any
   monitor. The region must outlive every monitor. */
void sentil_embedded_init(uint8_t *heap, size_t size);

/* Build a monitor from a formula, writing the handle to *out. Needs an archive
   built with the parser. */
sentil_embedded_status_t sentil_embedded_create(const char *formula,
                                                sentil_embedded_monitor_t **out);

/* Build a monitor from a host-compiled formula blob, produced by the
   sentil-compile-formula tool. */
sentil_embedded_status_t sentil_embedded_create_compiled(const uint8_t *bytes, size_t len,
                                                         sentil_embedded_monitor_t **out);

/* Fold one timestamped sample and write the robustness to *out. values holds the
   variables in sentil_embedded_symbol_index order. Times must strictly increase. */
sentil_embedded_status_t sentil_embedded_update(sentil_embedded_monitor_t *monitor, double time,
                                                const double *values, size_t n,
                                                sentil_embedded_robustness_t *out);

/* The number of variables the formula references. */
size_t sentil_embedded_variable_count(const sentil_embedded_monitor_t *monitor);

/* The packed-slice position of a named variable. */
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

/* Present when the library is built with synthesis, which is the default. */

/* A parsed specification for synthesis. */
typedef struct sentil_embedded_formula sentil_embedded_formula_t;
typedef struct sentil_embedded_bounds sentil_embedded_bounds_t;
typedef struct sentil_embedded_model sentil_embedded_model_t;
typedef struct sentil_embedded_controller sentil_embedded_controller_t;
typedef struct sentil_embedded_safety_filter sentil_embedded_safety_filter_t;

/* Parse a spec; needs an archive built with the parser. Free with the destroy
   call, or hand it to a controller, which takes ownership. */
sentil_embedded_status_t sentil_embedded_formula_create(const char *formula,
                                                        sentil_embedded_formula_t **out);
void sentil_embedded_formula_destroy(sentil_embedded_formula_t *formula);

/* Small dense numerics. Matrices are row-major; out buffers hold n doubles. */
sentil_embedded_status_t sentil_embedded_solve_spd(const double *matrix, size_t n,
                                                   const double *rhs, double *out);
sentil_embedded_status_t sentil_embedded_symmetric_eigen(const double *matrix, size_t n,
                                                         double *out_values, double *out_vectors);
/* Minimize 1/2 u'Pu + q'u subject to Gu <= h, P symmetric positive-definite. */
sentil_embedded_status_t sentil_embedded_solve_qp(const double *p, size_t n, const double *q,
                                                  const double *g, const double *h, size_t m,
                                                  double *out);

/* Smooth bounds on the min/max of n values at a temperature. */
double sentil_embedded_soft_min(const double *values, size_t n, double temperature);
double sentil_embedded_soft_max(const double *values, size_t n, double temperature);

/* Box bounds per coordinate. Free with destroy, or hand to a filter. */
sentil_embedded_status_t sentil_embedded_bounds_create(const double *lower, const double *upper,
                                                       size_t n, sentil_embedded_bounds_t **out);
sentil_embedded_status_t sentil_embedded_bounds_unbounded(size_t dimension,
                                                          sentil_embedded_bounds_t **out);
void sentil_embedded_bounds_destroy(sentil_embedded_bounds_t *bounds);

/* Linear model x_{t+1} = A x_t + B u_t. A is row-major n-by-n, B is n-by-b_cols,
   x0 has length n, variables names each state in order to match the spec. */
sentil_embedded_status_t sentil_embedded_linear_model_create(const double *a, size_t n,
                                                             const double *b, size_t b_cols,
                                                             const double *x0,
                                                             const char *const *variables,
                                                             double dt, size_t horizon,
                                                             sentil_embedded_model_t **out);
size_t sentil_embedded_model_input_dimension(const sentil_embedded_model_t *model);
void sentil_embedded_model_destroy(sentil_embedded_model_t *model);

/* Plan an input that best satisfies the spec. backend is 0 auto, 1 gradient, 2
   CMA-ES. out_input holds the model's input dimension. The model and spec are
   borrowed. */
sentil_embedded_status_t sentil_embedded_synthesize(const sentil_embedded_model_t *model,
                                                    const sentil_embedded_formula_t *spec,
                                                    const sentil_embedded_bounds_t *bounds,
                                                    int backend, size_t max_iters, double *out_input,
                                                    double *out_robustness, bool *out_holds);

/* Online receding-horizon controller. It consumes the model and spec, even on a
   failure, so do not free them after this call. */
sentil_embedded_status_t sentil_embedded_controller_create(sentil_embedded_model_t *model,
                                                           sentil_embedded_formula_t *spec,
                                                           size_t input_width, size_t max_iters,
                                                           const sentil_embedded_bounds_t *bounds,
                                                           sentil_embedded_controller_t **out);
/* Plan from the current state, writing input_width values to out. */
sentil_embedded_status_t sentil_embedded_controller_control(
    sentil_embedded_controller_t *controller, const double *state, size_t n, double *out);
void sentil_embedded_controller_destroy(sentil_embedded_controller_t *controller);

/* Least-restrictive safety filter that keeps inputs inside the bounds, which it
   consumes. */
sentil_embedded_status_t sentil_embedded_safety_filter_create(sentil_embedded_bounds_t *bounds,
                                                              sentil_embedded_safety_filter_t **out);
/* Input closest to nominal (length n) satisfying each barrier a_i . u >= b_i and
   the bounds. barrier_a is row-major m-by-n, barrier_b has length m; m of 0 is a
   bounds-only clamp. */
sentil_embedded_status_t sentil_embedded_safety_filter_filter(
    const sentil_embedded_safety_filter_t *filter, const double *nominal, size_t n,
    const double *barrier_a, const double *barrier_b, size_t m, double *out);
void sentil_embedded_safety_filter_destroy(sentil_embedded_safety_filter_t *filter);

/* A fixed-size rolling window with O(1) running statistics. */

typedef struct sentil_embedded_ring_buffer sentil_embedded_ring_buffer_t;

/* A timestamped sample. */
typedef struct sentil_embedded_sample {
    double time;
    double value;
} sentil_embedded_sample_t;

sentil_embedded_status_t sentil_embedded_ring_buffer_create(size_t capacity,
                                                            sentil_embedded_ring_buffer_t **out);
/* Push a sample, evicting the oldest when full. */
sentil_embedded_status_t sentil_embedded_ring_buffer_push(sentil_embedded_ring_buffer_t *buffer,
                                                          double time, double value,
                                                          sentil_embedded_sample_t *out_evicted,
                                                          bool *out_did_evict);
size_t sentil_embedded_ring_buffer_len(const sentil_embedded_ring_buffer_t *buffer);
size_t sentil_embedded_ring_buffer_capacity(const sentil_embedded_ring_buffer_t *buffer);
bool sentil_embedded_ring_buffer_is_empty(const sentil_embedded_ring_buffer_t *buffer);
bool sentil_embedded_ring_buffer_is_full(const sentil_embedded_ring_buffer_t *buffer);
/* Running statistics, or NaN when the buffer is empty. */
double sentil_embedded_ring_buffer_mean(const sentil_embedded_ring_buffer_t *buffer);
double sentil_embedded_ring_buffer_variance(const sentil_embedded_ring_buffer_t *buffer);
double sentil_embedded_ring_buffer_std_dev(const sentil_embedded_ring_buffer_t *buffer);
double sentil_embedded_ring_buffer_min(const sentil_embedded_ring_buffer_t *buffer);
double sentil_embedded_ring_buffer_max(const sentil_embedded_ring_buffer_t *buffer);
/* Read a sample to *out; false when the index is out of range or empty. */
bool sentil_embedded_ring_buffer_get(const sentil_embedded_ring_buffer_t *buffer, size_t index,
                                     sentil_embedded_sample_t *out);
bool sentil_embedded_ring_buffer_front(const sentil_embedded_ring_buffer_t *buffer,
                                       sentil_embedded_sample_t *out);
bool sentil_embedded_ring_buffer_back(const sentil_embedded_ring_buffer_t *buffer,
                                      sentil_embedded_sample_t *out);
/* Value recorded at the query time, within a small tolerance, or NaN. */
double sentil_embedded_ring_buffer_at_time(const sentil_embedded_ring_buffer_t *buffer, double time);
bool sentil_embedded_ring_buffer_closest_to_time(const sentil_embedded_ring_buffer_t *buffer,
                                                 double time, sentil_embedded_sample_t *out);
void sentil_embedded_ring_buffer_clear(sentil_embedded_ring_buffer_t *buffer);
void sentil_embedded_ring_buffer_destroy(sentil_embedded_ring_buffer_t *buffer);

#ifdef __cplusplus
} /* extern "C" */

/// A streaming monitor with sketch-friendly lifetime management.
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