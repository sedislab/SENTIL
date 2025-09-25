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
void sentil_ring_buffer_destroy(sentil_ring_buffer_t *buffer);

#ifdef __cplusplus
}
#endif

#endif /* SENTIL_H */