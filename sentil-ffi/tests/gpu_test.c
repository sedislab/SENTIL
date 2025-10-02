#include "sentil_test.h"

int main(void) {
    bool available = sentil_gpu_is_available();
    printf("gpu available: %d\n", (int)available);
    if (!available) {
        printf("gpu_test skipped (no device)\n");
        return 0;
    }

    sentil_sim_expr_t *advance =
        sentil_sim_expr_add(sentil_sim_expr_prev(0), sentil_sim_expr_noise(0));
    sentil_sim_expr_t *inits[] = {sentil_sim_expr_const(0.0)};
    sentil_sim_expr_t *advances[] = {advance};
    const char *vars[] = {"y"};
    sentil_noise_model_t *noises[] = {sentil_noise_gaussian(0.0, 1.0)};
    sentil_sim_model_t *model =
        sentil_sim_model_create(vars, 1, 0.1, 32, inits, 1, advances, 1, noises, 1);
    CHECK(model != NULL);
    sentil_formula_t *phi = sentil_formula_parse("P>=0.99 (always[0,3] (y < 5))");
    CHECK(phi != NULL);
    sentil_rare_event_config_t cfg = sentil_rare_event_config_default();
    sentil_gpu_splitting_estimate_t estimate;
    if (sentil_formula_check_rare_event_gpu(phi, model, &cfg, &estimate) == SENTIL_OK) {
        CHECK(estimate.violation_probability >= 0.0);
    }
    sentil_formula_destroy(phi);
    sentil_sim_model_destroy(model);
    return sentil_report("gpu_test");
}