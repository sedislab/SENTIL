/* Offline robustness over a recorded trace, in discrete and dense time. */
#include "sentil.h"
#include <stdio.h>

int main(void) {
    double times[] = {0.0, 1.0, 2.0, 3.0, 4.0};
    double speed[] = {12.0, 9.0, 7.0, 4.0, 6.0};
    sentil_trace_t *trace = sentil_trace_create(times, 5);
    sentil_trace_add_signal(trace, "speed", speed, 5);

    sentil_formula_t *phi = sentil_formula_parse("always (speed > 5)");
    if (phi == NULL) {
        fprintf(stderr, "parse error: %s\n", sentil_get_last_error());
        sentil_trace_destroy(trace);
        return 1;
    }

    double discrete = 0.0;
    double dense = 0.0;
    sentil_formula_robustness(phi, trace, &discrete);
    sentil_formula_robustness_dense(phi, trace, &dense);
    printf("discrete robustness %.3f, dense robustness %.3f\n", discrete, dense);

    sentil_formula_destroy(phi);
    sentil_trace_destroy(trace);
    return 0;
}