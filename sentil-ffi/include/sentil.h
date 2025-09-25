/* SENTIL: runtime verification for STL and PrSTL. C ABI. */
#ifndef SENTIL_H
#define SENTIL_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

#define SENTIL_VERSION_MAJOR 1
#define SENTIL_VERSION_MINOR 0
#define SENTIL_VERSION_PATCH 0

typedef enum sentil_error {
    SENTIL_OK = 0,
    SENTIL_ERR_NULL_POINTER = 1,
    SENTIL_ERR_UTF8 = 2,
    SENTIL_ERR_PARSE = 3,
    SENTIL_ERR_UNKNOWN_VARIABLE = 4,
    SENTIL_ERR_EVALUATION = 5,
    SENTIL_ERR_TRACE = 6,
    SENTIL_ERR_NOT_PROBABILISTIC = 7,
    SENTIL_ERR_INVALID_NOISE_MODEL = 8,
    SENTIL_ERR_INVALID_CONFIG = 9,
    SENTIL_ERR_FIT = 10,
    SENTIL_ERR_INGEST = 11,
    SENTIL_ERR_SPLITTING = 12,
    SENTIL_ERR_UNSUPPORTED = 13,
    SENTIL_ERR_TRANSPILATION = 14,
    SENTIL_ERR_GPU = 15,
    SENTIL_ERR_JSON = 16,
    SENTIL_ERR_PANIC = 17
} sentil_error_t;

void sentil_version(uint32_t *major, uint32_t *minor, uint32_t *patch);

/* Last error code on this thread. */
sentil_error_t sentil_get_last_error_code(void);

/* Borrowed message, valid until the next SENTIL call on this thread. */
const char *sentil_get_last_error(void);

/* Copies at most length bytes. Returns the length needed, terminator included. */
size_t sentil_get_last_error_message(char *buffer, size_t length);

/* Every sentil_free_* and sentil_*_destroy takes NULL as a no-op. */
void sentil_free_string(char *string);
void sentil_free_string_array(char **array, size_t count);
void sentil_free_doubles(double *array, size_t count);

/* Formula */

typedef struct sentil_formula sentil_formula_t;

sentil_formula_t *sentil_formula_parse(const char *input);

void sentil_formula_destroy(sentil_formula_t *formula);

/* JSON form of a formula. Free with sentil_free_string. */
char *sentil_formula_to_json(const sentil_formula_t *formula);

sentil_formula_t *sentil_formula_from_json(const char *json);

size_t sentil_formula_depth(const sentil_formula_t *formula);

bool sentil_formula_has_temporal(const sentil_formula_t *formula);

/* Variable names, sorted and unique. Free with sentil_free_string_array. */
char **sentil_formula_variables(const sentil_formula_t *formula, size_t *out_count);

/* Building formulas */

typedef enum sentil_comparison_op {
    SENTIL_CMP_LT = 0,
    SENTIL_CMP_LE = 1,
    SENTIL_CMP_GT = 2,
    SENTIL_CMP_GE = 3,
    SENTIL_CMP_EQ = 4,
    SENTIL_CMP_NE = 5
} sentil_comparison_op_t;

typedef enum sentil_binary_op {
    SENTIL_BIN_ADD = 0,
    SENTIL_BIN_SUB = 1,
    SENTIL_BIN_MUL = 2,
    SENTIL_BIN_DIV = 3,
    SENTIL_BIN_MOD = 4,
    SENTIL_BIN_POW = 5
} sentil_binary_op_t;

typedef enum sentil_probability_op {
    SENTIL_PROB_GE = 0,
    SENTIL_PROB_GT = 1,
    SENTIL_PROB_LE = 2,
    SENTIL_PROB_LT = 3
} sentil_probability_op_t;

typedef struct sentil_expr sentil_expr_t;

/* binary and call consume their operands; free an unused handle with sentil_expr_destroy. */
sentil_expr_t *sentil_expr_variable(const char *name);
sentil_expr_t *sentil_expr_literal(double value);
sentil_expr_t *sentil_expr_binary(sentil_binary_op_t op, sentil_expr_t *left,
                                  sentil_expr_t *right);
sentil_expr_t *sentil_expr_call(const char *name, sentil_expr_t **args, size_t count);
void sentil_expr_destroy(sentil_expr_t *expr);

/* The formula builders below consume the handles passed to them. */
sentil_formula_t *sentil_formula_predicate(sentil_expr_t *lhs, sentil_comparison_op_t op,
                                           sentil_expr_t *rhs);
sentil_formula_t *sentil_formula_not(sentil_formula_t *child);
sentil_formula_t *sentil_formula_and(sentil_formula_t *left, sentil_formula_t *right);
sentil_formula_t *sentil_formula_or(sentil_formula_t *left, sentil_formula_t *right);
sentil_formula_t *sentil_formula_implies(sentil_formula_t *left, sentil_formula_t *right);
sentil_formula_t *sentil_formula_next(sentil_formula_t *child);

/* Temporal builders take [lower, upper]; has_upper = false means unbounded above. */
sentil_formula_t *sentil_formula_always(double lower, double upper, bool has_upper,
                                        sentil_formula_t *child);
sentil_formula_t *sentil_formula_eventually(double lower, double upper, bool has_upper,
                                            sentil_formula_t *child);
sentil_formula_t *sentil_formula_historically(double lower, double upper, bool has_upper,
                                              sentil_formula_t *child);
sentil_formula_t *sentil_formula_once(double lower, double upper, bool has_upper,
                                      sentil_formula_t *child);
sentil_formula_t *sentil_formula_until(double lower, double upper, bool has_upper,
                                       sentil_formula_t *left, sentil_formula_t *right);
sentil_formula_t *sentil_formula_since(double lower, double upper, bool has_upper,
                                       sentil_formula_t *left, sentil_formula_t *right);

/* threshold in [0, 1]. */
sentil_formula_t *sentil_formula_probabilistic(sentil_probability_op_t op, double threshold,
                                               sentil_formula_t *child);

/* Trace */

typedef struct sentil_trace sentil_trace_t;

/* Trace over the given strictly increasing times. */
sentil_trace_t *sentil_trace_create(const double *times, size_t n);

sentil_trace_t *sentil_trace_from_signal(const double *times, size_t n, const char *name,
                                         const double *values, size_t m);

/* Trace with integer times 0, 1, ..., len - 1. */
sentil_trace_t *sentil_trace_indexed(size_t len);

/* Add or replace a named signal; its length must equal the trace length. */
sentil_error_t sentil_trace_add_signal(sentil_trace_t *trace, const char *name,
                                       const double *values, size_t n);

size_t sentil_trace_len(const sentil_trace_t *trace);

bool sentil_trace_is_empty(const sentil_trace_t *trace);

/* Borrowed view of the times, valid until the trace changes or is freed. */
const double *sentil_trace_times(const sentil_trace_t *trace, size_t *out_len);

/* Signal names, sorted. Free with sentil_free_string_array. */
char **sentil_trace_variables(const sentil_trace_t *trace, size_t *out_count);

/* Borrowed view of a named signal, valid until the trace changes or is freed. */
const double *sentil_trace_signal(const sentil_trace_t *trace, const char *name, size_t *out_len);

typedef enum sentil_interpolation {
    SENTIL_INTERP_LINEAR = 0,
    SENTIL_INTERP_HOLD = 1,
    SENTIL_INTERP_CUBIC = 2
} sentil_interpolation_t;

sentil_trace_t *sentil_trace_resample(const sentil_trace_t *trace, const double *times, size_t n,
                                      sentil_interpolation_t interp);

/* Parse a trace from CSV / TSV text: a header row, time column auto-detected. */
sentil_trace_t *sentil_trace_from_csv(const char *text);
sentil_trace_t *sentil_trace_from_tsv(const char *text);

/* Read a trace from a file, dispatching on extension (csv, tsv, parquet, arrow,
   sqlite, mat, and more). */
sentil_trace_t *sentil_trace_from_path(const char *path);

void sentil_trace_destroy(sentil_trace_t *trace);

/* Ring buffer */

/* A timed sample. */
typedef struct sentil_sample {
    bool found;
    double time;
    double value;
} sentil_sample_t;

typedef struct sentil_ring_buffer sentil_ring_buffer_t;

/* Fixed-capacity ring buffer with running statistics. */
sentil_ring_buffer_t *sentil_ring_buffer_create(size_t capacity);

/* On overflow the oldest is evicted into out_evicted, which may be NULL. Times
   must not move backward. */
sentil_error_t sentil_ring_buffer_push(sentil_ring_buffer_t *buffer, double time, double value,
                                       sentil_sample_t *out_evicted);

void sentil_ring_buffer_clear(sentil_ring_buffer_t *buffer);
size_t sentil_ring_buffer_len(const sentil_ring_buffer_t *buffer);
size_t sentil_ring_buffer_capacity(const sentil_ring_buffer_t *buffer);
bool sentil_ring_buffer_is_empty(const sentil_ring_buffer_t *buffer);
bool sentil_ring_buffer_is_full(const sentil_ring_buffer_t *buffer);

/* index 0 is the oldest. */
sentil_sample_t sentil_ring_buffer_front(const sentil_ring_buffer_t *buffer);
sentil_sample_t sentil_ring_buffer_back(const sentil_ring_buffer_t *buffer);
sentil_sample_t sentil_ring_buffer_get(const sentil_ring_buffer_t *buffer, size_t index);

sentil_sample_t sentil_ring_buffer_pop_front(sentil_ring_buffer_t *buffer);
sentil_sample_t sentil_ring_buffer_pop_back(sentil_ring_buffer_t *buffer);

sentil_sample_t sentil_ring_buffer_closest_to_time(const sentil_ring_buffer_t *buffer, double time);

/* mean, min, and max need one sample; variance and std_dev need two. */
bool sentil_ring_buffer_mean(const sentil_ring_buffer_t *buffer, double *out);
bool sentil_ring_buffer_variance(const sentil_ring_buffer_t *buffer, double *out);
bool sentil_ring_buffer_std_dev(const sentil_ring_buffer_t *buffer, double *out);
bool sentil_ring_buffer_min(const sentil_ring_buffer_t *buffer, double *out);
bool sentil_ring_buffer_max(const sentil_ring_buffer_t *buffer, double *out);

void sentil_ring_buffer_recompute_statistics(sentil_ring_buffer_t *buffer);

/* Value recorded at the query time, within a small tolerance. */
bool sentil_ring_buffer_at_time(const sentil_ring_buffer_t *buffer, double time, double *out);

/* False when empty. */
bool sentil_ring_buffer_time_range(const sentil_ring_buffer_t *buffer, double *out_start,
                                   double *out_end);

/* Samples with time in [start, end]. Free with sentil_free_samples. */
sentil_sample_t *sentil_ring_buffer_between(const sentil_ring_buffer_t *buffer, double start,
                                            double end, size_t *out_count);

void sentil_free_samples(sentil_sample_t *samples, size_t count);
void sentil_ring_buffer_destroy(sentil_ring_buffer_t *buffer);

/* Monitor configuration */

typedef enum sentil_time_mode {
    SENTIL_TIME_DISCRETE = 0,
    SENTIL_TIME_DENSE = 1
} sentil_time_mode_t;

typedef struct sentil_monitor_config sentil_monitor_config_t;

/* Default configuration: discrete time. */
sentil_monitor_config_t *sentil_monitor_config_create(void);

sentil_error_t sentil_monitor_config_set_time(sentil_monitor_config_t *config,
                                              sentil_time_mode_t mode);
sentil_time_mode_t sentil_monitor_config_time_mode(const sentil_monitor_config_t *config);

void sentil_monitor_config_destroy(sentil_monitor_config_t *config);

/* Monitor */

typedef struct sentil_monitor sentil_monitor_t;

/* Consumes the formula, even on a NULL return. config may be NULL for the default. */
sentil_monitor_t *sentil_monitor_create(sentil_formula_t *formula,
                                        const sentil_monitor_config_t *config);

/* config may be NULL for the default. */
sentil_monitor_t *sentil_monitor_parse(const char *formula,
                                       const sentil_monitor_config_t *config);

/* An owned copy of the monitored formula. Free with sentil_formula_destroy. */
sentil_formula_t *sentil_monitor_formula(const sentil_monitor_t *monitor);

/* An owned copy of the config. Free with sentil_monitor_config_destroy. */
sentil_monitor_config_t *sentil_monitor_config(const sentil_monitor_t *monitor);

/* A time span [start, end] where a property does not hold. */
typedef struct sentil_interval {
    double start;
    double end;
} sentil_interval_t;

/* A robustness verdict. */
typedef struct sentil_robustness {
    bool resolved;
    bool satisfied;
    double value;
    double lower;
    double upper;
} sentil_robustness_t;

sentil_error_t sentil_monitor_update(sentil_monitor_t *monitor, double time,
                                     const char *const *names, const double *values, size_t n,
                                     sentil_robustness_t *out);

/* Values in sentil_monitor_symbol_index order. */
sentil_error_t sentil_monitor_update_packed(sentil_monitor_t *monitor, double time,
                                            const double *values, size_t n,
                                            sentil_robustness_t *out);

/* Robustness of the trace, honoring the config's time mode. */
sentil_error_t sentil_monitor_robustness(const sentil_monitor_t *monitor,
                                         const sentil_trace_t *trace, double *out);

/* Robustness at every sample. Free with sentil_free_doubles. */
double *sentil_monitor_robustness_signal(const sentil_monitor_t *monitor,
                                         const sentil_trace_t *trace, size_t *out_len);

/* Spans where robustness is negative. Free with sentil_free_intervals. */
sentil_interval_t *sentil_monitor_violations(const sentil_monitor_t *monitor,
                                             const sentil_trace_t *trace, size_t *out_count);

/* Index in packed-update order; out_found is false if the formula does not use it. */
sentil_error_t sentil_monitor_symbol_index(sentil_monitor_t *monitor, const char *name,
                                           size_t *out_index, bool *out_found);

void sentil_monitor_reset(sentil_monitor_t *monitor);

sentil_interval_t *sentil_violation_intervals(const double *times, size_t n, const double *signal,
                                              size_t m, size_t *out_count);

void sentil_free_intervals(sentil_interval_t *intervals, size_t count);
void sentil_monitor_destroy(sentil_monitor_t *monitor);

/* Streaming monitor */

typedef struct sentil_stream_monitor sentil_stream_monitor_t;

sentil_stream_monitor_t *sentil_stream_monitor_create(const char *formula);
sentil_stream_monitor_t *sentil_stream_monitor_from_formula(const sentil_formula_t *formula);
size_t sentil_stream_monitor_variable_count(const sentil_stream_monitor_t *monitor);

/* Index of a variable in packed-update order; false if the formula does not use it. */
bool sentil_stream_monitor_symbol_index(const sentil_stream_monitor_t *monitor, const char *name,
                                        size_t *out_index);

sentil_error_t sentil_stream_monitor_update(sentil_stream_monitor_t *monitor, double time,
                                            const char *const *names, const double *values,
                                            size_t n, sentil_robustness_t *out);
sentil_error_t sentil_stream_monitor_update_packed(sentil_stream_monitor_t *monitor, double time,
                                                   const double *values, size_t n,
                                                   sentil_robustness_t *out);

/* Per-step robustness. Free with sentil_free_robustness. */
sentil_robustness_t *sentil_stream_monitor_run(sentil_stream_monitor_t *monitor,
                                               const sentil_trace_t *trace, size_t *out_count);

void sentil_free_robustness(sentil_robustness_t *array, size_t count);

void sentil_stream_monitor_reset(sentil_stream_monitor_t *monitor);
void sentil_stream_monitor_destroy(sentil_stream_monitor_t *monitor);

/* Multi-formula monitor */

typedef struct sentil_multi_monitor sentil_multi_monitor_t;

sentil_multi_monitor_t *sentil_multi_monitor_create(void);

/* Add a formula under an id, from a string or a borrowed formula handle. */
sentil_error_t sentil_multi_monitor_add(sentil_multi_monitor_t *monitor, const char *id,
                                        const char *formula);
sentil_error_t sentil_multi_monitor_add_formula(sentil_multi_monitor_t *monitor, const char *id,
                                                const sentil_formula_t *formula);

/* Removes the first formula with the id. */
bool sentil_multi_monitor_remove(sentil_multi_monitor_t *monitor, const char *id);

void sentil_multi_monitor_reset(sentil_multi_monitor_t *monitor);
size_t sentil_multi_monitor_len(const sentil_multi_monitor_t *monitor);
bool sentil_multi_monitor_is_empty(const sentil_multi_monitor_t *monitor);

/* Ids in insertion order. Free with sentil_free_string_array. */
char **sentil_multi_monitor_ids(const sentil_multi_monitor_t *monitor, size_t *out_count);

/* A formula id paired with its verdict. id is owned by the result array. */
typedef struct sentil_named_robustness {
    char *id;
    sentil_robustness_t robustness;
} sentil_named_robustness_t;

/* Per-id verdicts in insertion order. Free with sentil_free_named_robustness. */
sentil_named_robustness_t *sentil_multi_monitor_update(sentil_multi_monitor_t *monitor, double time,
                                                       const char *const *names,
                                                       const double *values, size_t n,
                                                       size_t *out_count);

void sentil_free_named_robustness(sentil_named_robustness_t *array, size_t count);

void sentil_multi_monitor_destroy(sentil_multi_monitor_t *monitor);

/* Formula bank */

typedef struct sentil_formula_bank sentil_formula_bank_t;

sentil_formula_bank_t *sentil_formula_bank_create(void);

/* Add a formula under an id, from a string or a borrowed formula handle. */
sentil_error_t sentil_formula_bank_add(sentil_formula_bank_t *bank, const char *id,
                                       const char *formula);
sentil_error_t sentil_formula_bank_add_formula(sentil_formula_bank_t *bank, const char *id,
                                               const sentil_formula_t *formula);

/* Ids in insertion order. Free with sentil_free_string_array. */
char **sentil_formula_bank_ids(const sentil_formula_bank_t *bank, size_t *out_count);
size_t sentil_formula_bank_len(const sentil_formula_bank_t *bank);
bool sentil_formula_bank_is_empty(const sentil_formula_bank_t *bank);

void sentil_formula_bank_destroy(sentil_formula_bank_t *bank);

#ifdef __cplusplus
}
#endif

#endif /* SENTIL_H */