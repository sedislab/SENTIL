/* Probabilistic monitoring: lift a noisy sensor and estimate the satisfaction probability. */
#include "sentil.h"
#include <stdio.h>

int main(void) {
    double times[20];
    double xs[20];
    for (int i = 0; i < 20; ++i) {
        times[i] = i;
        xs[i] = 0.4 + 0.05 * i;
    }
    sentil_trace_t *trace = sentil_trace_create(times, 20);
    sentil_trace_add_signal(trace, "x", xs, 20);

    sentil_lifting_registry_t *lifting = sentil_lifting_registry_create();
    sentil_lifting_registry_register(lifting, "x", sentil_noise_gaussian(0.0, 0.3),
                                     SENTIL_NOISE_ADDITIVE);

    sentil_formula_t *phi = sentil_formula_parse("P>=0.9 (always (x > 0))");
    sentil_smc_config_t config = sentil_smc_config_default();
    config.samples = 5000;
    sentil_smc_result_t result;
    if (sentil_formula_check(phi, trace, lifting, &config, &result) != SENTIL_OK) {
        fprintf(stderr, "check error: %s\n", sentil_get_last_error());
        sentil_formula_destroy(phi);
        sentil_lifting_registry_destroy(lifting);
        sentil_trace_destroy(trace);
        return 1;
    }
    printf("probability %.3f, interval [%.3f, %.3f], holds %s\n", result.probability,
           result.interval.lower, result.interval.upper, result.holds ? "true" : "false");

    sentil_formula_destroy(phi);
    sentil_lifting_registry_destroy(lifting);
    sentil_trace_destroy(trace);
    return 0;
}