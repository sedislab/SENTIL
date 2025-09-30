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

/* Per-formula robustness. id is owned by the array. */
typedef struct sentil_bank_result {
    char *id;
    bool ok;
    double value;
    sentil_error_t code;
} sentil_bank_result_t;

/* Free with sentil_free_bank_results. */
sentil_bank_result_t *sentil_formula_bank_robustness(const sentil_formula_bank_t *bank,
                                                     const sentil_trace_t *trace, size_t *out_count);
sentil_bank_result_t *sentil_formula_bank_robustness_dense(const sentil_formula_bank_t *bank,
                                                           const sentil_trace_t *trace,
                                                           size_t *out_count);

void sentil_free_bank_results(sentil_bank_result_t *array, size_t count);

void sentil_formula_bank_destroy(sentil_formula_bank_t *bank);

/* Statistics */

typedef enum sentil_interval_method {
    SENTIL_WILSON = 0,
    SENTIL_CLOPPER_PEARSON = 1,
    SENTIL_JEFFREYS = 2,
    SENTIL_AGRESTI_COULL = 3
} sentil_interval_method_t;

typedef struct sentil_confidence_interval {
    double lower;
    double upper;
    double level;
} sentil_confidence_interval_t;

sentil_confidence_interval_t sentil_wilson_interval(uint64_t successes, uint64_t trials, double level);
sentil_confidence_interval_t sentil_clopper_pearson(uint64_t successes, uint64_t trials, double level);
sentil_confidence_interval_t sentil_jeffreys_interval(uint64_t successes, uint64_t trials,
                                                      double level);
sentil_confidence_interval_t sentil_agresti_coull(uint64_t successes, uint64_t trials, double level);
sentil_confidence_interval_t sentil_interval(sentil_interval_method_t method, uint64_t successes,
                                             uint64_t trials, double level);

/* Two-sided z critical value for a confidence level in (0, 1). */
double sentil_z_score(double level);

/* Sample count for a target error and confidence. */
sentil_error_t sentil_chernoff_hoeffding_samples(double epsilon, double delta, uint64_t *out);
sentil_error_t sentil_wilson_samples(double epsilon, double level, uint64_t *out);

/* Noise models */

typedef enum sentil_noise_interaction {
    SENTIL_NOISE_ADDITIVE = 0,
    SENTIL_NOISE_MULTIPLICATIVE = 1
} sentil_noise_interaction_t;

typedef struct sentil_noise_model sentil_noise_model_t;

/* Free a model with sentil_noise_destroy. */
sentil_noise_model_t *sentil_noise_dirac(double value);
sentil_noise_model_t *sentil_noise_gaussian(double mean, double std_dev);
sentil_noise_model_t *sentil_noise_uniform(double low, double high);
sentil_noise_model_t *sentil_noise_log_normal(double mu, double sigma);
sentil_noise_model_t *sentil_noise_exponential(double lambda);
sentil_noise_model_t *sentil_noise_gamma(double shape, double scale);
sentil_noise_model_t *sentil_noise_beta(double alpha, double beta);
sentil_noise_model_t *sentil_noise_weibull(double shape, double scale);
sentil_noise_model_t *sentil_noise_rayleigh(double scale);
sentil_noise_model_t *sentil_noise_gumbel(double location, double scale);
sentil_noise_model_t *sentil_noise_cauchy(double location, double scale);
sentil_noise_model_t *sentil_noise_student_t(double df, double location, double scale);
sentil_noise_model_t *sentil_noise_truncated_normal(double mean, double std_dev, double lower,
                                                    double upper);
sentil_noise_model_t *sentil_noise_poisson(double lambda);
sentil_noise_model_t *sentil_noise_binomial(uint64_t n, double p);

/* Empirical model resampled from residuals (at least one, all finite). */
sentil_noise_model_t *sentil_noise_bootstrap(const double *residuals, size_t n);

/* Consumes the component handles, except when the call is rejected before reading
   them, where you still own them. */
sentil_noise_model_t *sentil_noise_mixture(const double *weights, sentil_noise_model_t **models,
                                           size_t n);

/* Analytic mean and variance; false when undefined (e.g. Cauchy). */
bool sentil_noise_mean(const sentil_noise_model_t *model, double *out);
bool sentil_noise_variance(const sentil_noise_model_t *model, double *out);

/* Free the result with sentil_free_doubles. */
double *sentil_noise_residuals(const double *ground_truth, size_t n, const double *sensor, size_t m,
                               sentil_noise_interaction_t interaction, size_t *out_len);

sentil_noise_model_t *sentil_noise_fit_gaussian(const double *samples, size_t n);
sentil_noise_model_t *sentil_noise_fit_bootstrap(const double *samples, size_t n);
sentil_noise_model_t *sentil_noise_fit_bootstrap_reservoir(const double *samples, size_t n,
                                                           size_t max_samples);
sentil_noise_model_t *sentil_noise_fit_gaussian_mixture(const double *samples, size_t n,
                                                        size_t components, size_t max_iters);

/* JSON form of a model. Free the string with sentil_free_string. */
char *sentil_noise_to_json(const sentil_noise_model_t *model);
sentil_noise_model_t *sentil_noise_from_json(const char *json);

sentil_noise_model_t *sentil_noise_from_file(const char *path);

void sentil_noise_destroy(sentil_noise_model_t *model);

/* Lifting registry */

typedef struct sentil_lifting_registry sentil_lifting_registry_t;

sentil_lifting_registry_t *sentil_lifting_registry_create(void);

/* Attach a noise model to a signal; the model handle is consumed. */
sentil_error_t sentil_lifting_registry_register(sentil_lifting_registry_t *registry,
                                                const char *variable, sentil_noise_model_t *model,
                                                sentil_noise_interaction_t interaction);

/* Signals with a noise model, sorted. Free with sentil_free_string_array. */
char **sentil_lifting_registry_variables(const sentil_lifting_registry_t *registry,
                                         size_t *out_count);
bool sentil_lifting_registry_is_empty(const sentil_lifting_registry_t *registry);

/* One seeded noisy realization of the trace. */
sentil_trace_t *sentil_lifting_registry_lift(const sentil_lifting_registry_t *registry,
                                             const sentil_trace_t *trace, uint64_t seed);

void sentil_lifting_registry_destroy(sentil_lifting_registry_t *registry);

/* Statistical model checking */

typedef struct sentil_smc_config {
    uint64_t samples;
    double confidence;
    uint64_t seed;
    sentil_interval_method_t interval_method;
} sentil_smc_config_t;

/* The defaults: 10000 samples, 0.95 confidence, seed 42, Wilson interval. */
sentil_smc_config_t sentil_smc_config_default(void);

typedef struct sentil_smc_result {
    double probability;
    sentil_confidence_interval_t interval;
    uint64_t satisfactions;
    uint64_t samples;
    bool holds;
} sentil_smc_result_t;

/* check_conservative always uses the Clopper-Pearson interval. */
sentil_error_t sentil_formula_check(const sentil_formula_t *formula, const sentil_trace_t *trace,
                                    const sentil_lifting_registry_t *lifting,
                                    const sentil_smc_config_t *config, sentil_smc_result_t *out);
sentil_error_t sentil_formula_check_conservative(const sentil_formula_t *formula,
                                                 const sentil_trace_t *trace,
                                                 const sentil_lifting_registry_t *lifting,
                                                 const sentil_smc_config_t *config,
                                                 sentil_smc_result_t *out);

typedef struct sentil_robustness_distribution {
    uint64_t count;
    double mean;
    double variance;
    double std_dev;
    double min;
    double max;
} sentil_robustness_distribution_t;

sentil_error_t sentil_formula_check_distribution(const sentil_formula_t *formula,
                                                 const sentil_trace_t *trace,
                                                 const sentil_lifting_registry_t *lifting,
                                                 const sentil_smc_config_t *config,
                                                 sentil_smc_result_t *out_result,
                                                 sentil_robustness_distribution_t *out_distribution);

/* Uses the monitor's configured SMC settings. */
sentil_error_t sentil_monitor_check(const sentil_monitor_t *monitor, const sentil_trace_t *trace,
                                    const sentil_lifting_registry_t *lifting,
                                    sentil_smc_result_t *out);

/* Sequential testing (SPRT) */

typedef enum sentil_sprt_verdict {
    SENTIL_SPRT_ACCEPT_H0 = 0,
    SENTIL_SPRT_ACCEPT_H1 = 1,
    SENTIL_SPRT_INCONCLUSIVE = 2
} sentil_sprt_verdict_t;

/* Requires 0 < p0 < p1 < 1, both error rates in (0, 1), and max_samples > 0. */
typedef struct sentil_sprt_config {
    double p0;
    double p1;
    double alpha;
    double beta;
    uint64_t max_samples;
    uint64_t seed;
} sentil_sprt_config_t;

typedef struct sentil_sprt_result {
    sentil_sprt_verdict_t verdict;
    uint64_t samples;
    double log_likelihood;
} sentil_sprt_result_t;

sentil_error_t sentil_formula_check_sequential(const sentil_formula_t *formula,
                                               const sentil_trace_t *trace,
                                               const sentil_lifting_registry_t *lifting,
                                               const sentil_sprt_config_t *config,
                                               sentil_sprt_result_t *out);
sentil_error_t sentil_monitor_check_sequential(const sentil_monitor_t *monitor,
                                               const sentil_trace_t *trace,
                                               const sentil_lifting_registry_t *lifting,
                                               const sentil_sprt_config_t *config,
                                               sentil_sprt_result_t *out);

/* Bayesian sequential testing */

typedef enum sentil_bayes_verdict {
    SENTIL_BAYES_HOLDS = 0,
    SENTIL_BAYES_FAILS = 1,
    SENTIL_BAYES_INCONCLUSIVE = 2
} sentil_bayes_verdict_t;

/* Requires threshold in (0, 1), bayes_factor > 1, and max_samples > 0. */
typedef struct sentil_bayes_config {
    double threshold;
    double bayes_factor;
    uint64_t max_samples;
    uint64_t seed;
} sentil_bayes_config_t;

typedef struct sentil_bayes_result {
    sentil_bayes_verdict_t verdict;
    uint64_t samples;
    double posterior;
} sentil_bayes_result_t;

sentil_error_t sentil_formula_check_bayesian(const sentil_formula_t *formula,
                                             const sentil_trace_t *trace,
                                             const sentil_lifting_registry_t *lifting,
                                             const sentil_bayes_config_t *config,
                                             sentil_bayes_result_t *out);

/* draw returns the next sample given userdata. */
typedef bool (*sentil_bernoulli_fn)(void *userdata);
sentil_error_t sentil_sequential_test(const sentil_sprt_config_t *config, sentil_bernoulli_fn draw,
                                      void *userdata, sentil_sprt_result_t *out);
sentil_error_t sentil_bayes_sequential_test(const sentil_bayes_config_t *config,
                                            sentil_bernoulli_fn draw, void *userdata,
                                            sentil_bayes_result_t *out);

/* Simulation model (data form) */

typedef struct sentil_sim_expr sentil_sim_expr_t;

/* The binary builders and call consume their operands. prev reads the previous
   step's value of a variable; noise draws from a noise source. */
sentil_sim_expr_t *sentil_sim_expr_prev(size_t variable);
sentil_sim_expr_t *sentil_sim_expr_time(void);
sentil_sim_expr_t *sentil_sim_expr_const(double value);
sentil_sim_expr_t *sentil_sim_expr_noise(size_t source);
sentil_sim_expr_t *sentil_sim_expr_add(sentil_sim_expr_t *left, sentil_sim_expr_t *right);
sentil_sim_expr_t *sentil_sim_expr_sub(sentil_sim_expr_t *left, sentil_sim_expr_t *right);
sentil_sim_expr_t *sentil_sim_expr_mul(sentil_sim_expr_t *left, sentil_sim_expr_t *right);
sentil_sim_expr_t *sentil_sim_expr_div(sentil_sim_expr_t *left, sentil_sim_expr_t *right);
sentil_sim_expr_t *sentil_sim_expr_call(const char *name, sentil_sim_expr_t **args, size_t count);
void sentil_sim_expr_destroy(sentil_sim_expr_t *expr);

typedef struct sentil_sim_model sentil_sim_model_t;

/* Build a declarative stochastic model: one init and one advance expression per
   variable, drawing from the given noise sources. The init, advance, and noise
   handles are all consumed. NULL on error. */
sentil_sim_model_t *sentil_sim_model_create(const char *const *variables, size_t n_vars, double dt,
                                            size_t horizon, sentil_sim_expr_t **init, size_t n_init,
                                            sentil_sim_expr_t **advance, size_t n_advance,
                                            sentil_noise_model_t **noise, size_t n_noise);

typedef struct sentil_stochastic_system sentil_stochastic_system_t;

sentil_trace_t *sentil_sim_model_simulate(const sentil_sim_model_t *model, uint64_t seed);
char **sentil_sim_model_variables(const sentil_sim_model_t *model, size_t *out_count);
double sentil_sim_model_dt(const sentil_sim_model_t *model);
size_t sentil_sim_model_horizon(const sentil_sim_model_t *model);

sentil_stochastic_system_t *sentil_sim_model_to_stochastic_system(const sentil_sim_model_t *model);

void sentil_sim_model_destroy(sentil_sim_model_t *model);

/* The callbacks must be thread-safe. */
typedef struct sentil_system_callbacks {
    void *userdata;
    void (*init)(void *userdata, uint64_t seed, double *out_state, size_t n);
    void (*step)(void *userdata, const double *prev, size_t n, double time, uint64_t seed,
                 double *out_state);
} sentil_system_callbacks_t;

sentil_stochastic_system_t *sentil_stochastic_system_create(const char *const *variables,
                                                            size_t n_vars, double dt, size_t horizon,
                                                            sentil_system_callbacks_t callbacks);

sentil_trace_t *sentil_stochastic_system_simulate(const sentil_stochastic_system_t *system,
                                                  uint64_t seed);
char **sentil_stochastic_system_variables(const sentil_stochastic_system_t *system,
                                          size_t *out_count);
double sentil_stochastic_system_dt(const sentil_stochastic_system_t *system);
size_t sentil_stochastic_system_horizon(const sentil_stochastic_system_t *system);
void sentil_stochastic_system_destroy(sentil_stochastic_system_t *system);

/* Rare-event splitting */

typedef struct sentil_rare_event_config {
    size_t particles;
    double margin;
    uint64_t seed;
} sentil_rare_event_config_t;

/* The defaults: 4096 particles, margin 0, seed 42. */
sentil_rare_event_config_t sentil_rare_event_config_default(void);

typedef struct sentil_rare_event_result {
    double probability;
    double violation_probability;
    bool holds;
    uint64_t simulations;
} sentil_rare_event_result_t;

/* Adaptive multilevel splitting over a stochastic system. */
sentil_error_t sentil_formula_check_rare_event(const sentil_formula_t *formula,
                                               const sentil_stochastic_system_t *system,
                                               const sentil_rare_event_config_t *config,
                                               sentil_rare_event_result_t *out);
sentil_error_t sentil_monitor_check_rare(const sentil_monitor_t *monitor,
                                         const sentil_stochastic_system_t *system,
                                         sentil_rare_event_result_t *out);

/* is_terminal returns whether the run ended and sets out_in_rare_event. The
   callbacks must be thread-safe. */
typedef struct sentil_ams_interface {
    size_t state_size;
    void *userdata;
    void (*initial_state)(void *userdata, uint64_t seed, void *out_state);
    void (*step)(void *userdata, const void *state, uint64_t seed, void *out_state);
    bool (*is_terminal)(void *userdata, const void *state, bool *out_in_rare_event);
    double (*score)(void *userdata, const void *state);
} sentil_ams_interface_t;

typedef struct sentil_rare_event_estimate {
    double probability;
    uint64_t simulations;
} sentil_rare_event_estimate_t;

sentil_error_t sentil_adaptive_multilevel_splitting(sentil_ams_interface_t simulator,
                                                    size_t particles, double target_score,
                                                    uint64_t max_steps, uint64_t seed,
                                                    sentil_rare_event_estimate_t *out);

/* Synthesis numerics */

/* Matrices are row-major and output buffers are caller-allocated. eigen fills n
   eigenvalues and an n-by-n matrix whose row j is the eigenvector for eigenvalue j. */

/* Minimize 1/2 u'Pu + q'u subject to Gu <= h, P symmetric positive-definite, G is
   m-by-n. */
sentil_error_t sentil_solve_qp(const double *p, size_t n, const double *q, const double *g,
                               size_t m, const double *h, size_t max_iters, double *out);

/* Solve Ax = b for symmetric positive-definite A (n-by-n). */
sentil_error_t sentil_solve_spd(const double *matrix, size_t n, const double *rhs, double *out);

sentil_error_t sentil_symmetric_eigen(const double *matrix, size_t n, double *out_values,
                                      double *out_vectors);

/* Smooth robustness */

typedef enum sentil_soft_kind {
    SENTIL_SOFT_LOG_SUM_EXP = 0,
    SENTIL_SOFT_ARITHMETIC_GEOMETRIC_MEAN = 1
} sentil_soft_kind_t;

/* temperature must be finite and positive; the arithmetic-geometric-mean kind ignores it. */
typedef struct sentil_smooth_config {
    double temperature;
    sentil_soft_kind_t kind;
} sentil_smooth_config_t;

/* The defaults: temperature 10, log-sum-exp. */
sentil_smooth_config_t sentil_smooth_config_default(void);

double sentil_soft_min(const double *values, size_t n, double temperature);
double sentil_soft_max(const double *values, size_t n, double temperature);

/* Differentiable surrogate for robustness. */
sentil_error_t sentil_formula_smooth_robustness(const sentil_formula_t *formula,
                                                const sentil_trace_t *trace,
                                                const sentil_smooth_config_t *config, double *out);

#ifdef __cplusplus
}
#endif

#endif /* SENTIL_H */