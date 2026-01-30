/* Synthesize a control-input sequence that satisfies a spec on a linear model. */
#include "sentil.h"
#include <stdio.h>

int main(void) {
    double a[] = {1.0};
    double b[] = {1.0};
    double x0[] = {1.0};
    const char *variables[] = {"x"};
    sentil_system_model_t *model =
        sentil_linear_model_create(a, 1, b, 1, x0, variables, 1, 1.0, 3);
    sentil_formula_t *spec = sentil_formula_parse("always (x > 0)");
    double lower[] = {-1.0, -1.0, -1.0};
    double upper[] = {1.0, 1.0, 1.0};
    sentil_bounds_t *bounds = sentil_bounds_create(lower, upper, 3);

    sentil_synthesis_result_t result;
    if (sentil_synthesize(model, spec, bounds, NULL, 0, SENTIL_BACKEND_GRADIENT, 0, &result)
        != SENTIL_OK) {
        fprintf(stderr, "synthesis error: %s\n", sentil_get_last_error());
        sentil_bounds_destroy(bounds);
        sentil_formula_destroy(spec);
        sentil_system_model_destroy(model);
        return 1;
    }

    printf("input [");
    for (size_t i = 0; i < result.input_len; ++i) {
        printf("%s%.4f", i == 0 ? "" : ", ", result.input[i]);
    }
    printf("], robustness %.4f, holds %s\n", result.robustness, result.holds ? "true" : "false");

    sentil_free_doubles(result.input, result.input_len);
    sentil_bounds_destroy(bounds);
    sentil_formula_destroy(spec);
    sentil_system_model_destroy(model);
    return 0;
}