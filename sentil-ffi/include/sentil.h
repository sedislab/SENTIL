/*
 * SENTIL: runtime verification for Signal Temporal Logic and its probabilistic
 * extension PrSTL. Stable C ABI.
 *
 * Authors: Paapa Kwesi Quansah, Ernest Bonnah, SEDIS Lab, Baylor University.
 * Dual licensed under MIT or Apache-2.0.
 *
 * Every function clears the calling thread's last error on entry. A failed call
 * returns a sentinel (a null handle, a NaN, or a nonzero sentil_error_t) and
 * leaves behind a code and message that sentil_get_last_error_code and
 * sentil_get_last_error_message read back. No call aborts the process.
 */
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

/*
 * Status codes. SENTIL_OK is zero; the rest signal failure, with the detail in
 * the thread's last error message. The integer values are part of the ABI and do
 * not change between releases.
 */
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

/* Writes the library version into the out-parameters. Null pointers are skipped. */
void sentil_version(uint32_t *major, uint32_t *minor, uint32_t *patch);

/* The status code of the most recent failed call on this thread, or SENTIL_OK. */
sentil_error_t sentil_get_last_error_code(void);

/*
 * Copies the thread's last error message into buffer, writing at most length
 * bytes and null terminating when length is nonzero. Returns the length the
 * message needs including the terminator, so a null buffer sizes the allocation.
 */
size_t sentil_get_last_error_message(char *buffer, size_t length);

/*
 * Frees a string returned by this library. Null is a no-op. Do not pass a
 * pointer the library did not return, and do not free the same one twice.
 */
void sentil_free_string(char *string);

/* Formula */

/* An opaque, owned PrSTL syntax tree. Free it with sentil_formula_destroy. */
typedef struct sentil_formula sentil_formula_t;

/*
 * Parses a PrSTL formula. Returns a handle the caller owns, or NULL on a parse
 * error whose message names the line and column. The grammar accepts the word
 * operators (always, eventually, until, since, next, and, or, not, implies) and
 * their aliases (G, F, U, S, X, &&, ||, !, ->), arithmetic predicates, and the
 * probabilistic operator P.
 */
sentil_formula_t *sentil_formula_parse(const char *input);

/* Frees a formula handle. NULL is a no-op. */
void sentil_formula_destroy(sentil_formula_t *formula);

/* The nesting depth: predicates are 1 and each operator adds a level. */
size_t sentil_formula_depth(const sentil_formula_t *formula);

/* Whether the formula contains any temporal operator. */
bool sentil_formula_has_temporal(const sentil_formula_t *formula);

#ifdef __cplusplus
}
#endif

#endif /* SENTIL_H */