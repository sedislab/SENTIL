#include "sentil_test.h"

int main(void) {
    double a[] = {4.0, 1.0, 1.0, 3.0};
    double b[] = {1.0, 2.0};
    double x[2];
    CHECK(sentil_solve_spd(a, 2, b, x) == SENTIL_OK);
    CHECK_CLOSE(a[0] * x[0] + a[1] * x[1], 1.0, 1e-9);
    CHECK_CLOSE(a[2] * x[0] + a[3] * x[1], 2.0, 1e-9);

    double vals[] = {3.0, 1.0, 2.0};
    CHECK(sentil_soft_min(vals, 3, 100.0) < 1.5);

    double times[] = {0.0, 1.0, 2.0};
    double xs[] = {1.0, 2.0, 0.5};
    sentil_trace_t *tr = sentil_trace_create(times, 3);
    sentil_trace_add_signal(tr, "x", xs, 3);
    sentil_formula_t *phi = sentil_formula_parse("always (x > 0)");
    sentil_smooth_config_t sc = sentil_smooth_config_default();
    double sr = 0.0;
    CHECK(sentil_formula_smooth_robustness(phi, tr, &sc, &sr) == SENTIL_OK);
    CHECK(sr > 0.0);

    double am[] = {1.0};
    double bm[] = {1.0};
    double x0[] = {1.0};
    const char *mvars[] = {"x"};
    sentil_system_model_t *model = sentil_linear_model_create(am, 1, bm, 1, x0, mvars, 1, 1.0, 3);
    CHECK(model != NULL);
    CHECK(sentil_system_model_input_dimension(model) == 3);
    double lo[] = {-1.0, -1.0, -1.0};
    double hi[] = {1.0, 1.0, 1.0};
    sentil_bounds_t *bounds = sentil_bounds_create(lo, hi, 3);
    CHECK(bounds != NULL && sentil_bounds_dimension(bounds) == 3);
    sentil_synthesis_result_t result;
    CHECK(sentil_synthesize(model, phi, bounds, NULL, 0, SENTIL_BACKEND_GRADIENT, 0, &result)
          == SENTIL_OK);
    CHECK(result.input_len == 3);
    sentil_free_doubles(result.input, result.input_len);

    sentil_safety_filter_t *filter = sentil_safety_filter_create(bounds);
    CHECK(filter != NULL);
    double nominal[] = {2.0, 0.0, -2.0};
    double clamped[3];
    CHECK(sentil_safety_filter_filter(filter, nominal, 3, NULL, NULL, 0, clamped) == SENTIL_OK);
    CHECK(clamped[0] <= 1.0 + 1e-9 && clamped[2] >= -1.0 - 1e-9);
    sentil_safety_filter_destroy(filter);

    sentil_formula_destroy(phi);
    sentil_system_model_destroy(model);
    sentil_trace_destroy(tr);
    return sentil_report("test_synthesis");
}