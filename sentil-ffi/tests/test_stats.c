#include "sentil_test.h"

int main(void) {
    sentil_confidence_interval_t w = sentil_wilson_interval(50, 100, 0.95);
    CHECK_CLOSE(w.lower, 0.403831, 1e-5);
    CHECK_CLOSE(w.upper, 0.596169, 1e-5);
    sentil_confidence_interval_t cp = sentil_clopper_pearson(50, 100, 0.95);
    CHECK_CLOSE(cp.lower, 0.398321, 1e-5);
    CHECK_CLOSE(cp.upper, 0.601679, 1e-5);
    CHECK_CLOSE(sentil_z_score(0.95), 1.95996, 1e-5);
    uint64_t samples = 0;
    CHECK(sentil_chernoff_hoeffding_samples(0.1, 0.05, &samples) == SENTIL_OK && samples == 185);

    sentil_noise_model_t *g = sentil_noise_gaussian(2.0, 0.5);
    double mean = 0.0, var = 0.0;
    CHECK(sentil_noise_mean(g, &mean) && fabs(mean - 2.0) < 1e-12);
    CHECK(sentil_noise_variance(g, &var) && fabs(var - 0.25) < 1e-12);
    sentil_noise_model_t *cauchy = sentil_noise_cauchy(0.0, 1.0);
    CHECK(!sentil_noise_mean(cauchy, &mean));
    sentil_noise_destroy(cauchy);
    char *nj = sentil_noise_to_json(g);
    CHECK(nj != NULL);
    sentil_noise_model_t *g2 = sentil_noise_from_json(nj);
    CHECK(g2 != NULL);
    sentil_free_string(nj);
    sentil_noise_destroy(g2);
    sentil_noise_destroy(g);

    sentil_noise_model_t *comps[] = {sentil_noise_gaussian(0.0, 1.0), sentil_noise_uniform(-1.0, 1.0)};
    double weights[] = {0.6, 0.4};
    sentil_noise_model_t *mix = sentil_noise_mixture(weights, comps, 2);
    CHECK(mix != NULL);

    double times[] = {0.0, 1.0, 2.0, 3.0};
    double xs[] = {0.5, 0.4, 0.6, 0.55};
    sentil_trace_t *tr = sentil_trace_create(times, 4);
    sentil_trace_add_signal(tr, "x", xs, 4);
    sentil_lifting_registry_t *reg = sentil_lifting_registry_create();
    CHECK(sentil_lifting_registry_register(reg, "x", mix, SENTIL_NOISE_ADDITIVE) == SENTIL_OK);
    sentil_formula_t *phi = sentil_formula_parse("P>=0.5 (always (x > 0))");
    sentil_smc_config_t scfg = sentil_smc_config_default();
    CHECK(scfg.samples == 10000);
    sentil_smc_result_t res;
    CHECK(sentil_formula_check(phi, tr, reg, &scfg, &res) == SENTIL_OK);
    CHECK(res.probability >= 0.0 && res.probability <= 1.0);

    sentil_formula_t *det = sentil_formula_parse("always (x > 0)");
    CHECK(sentil_formula_check(det, tr, reg, &scfg, &res) != SENTIL_OK);
    CHECK(sentil_get_last_error_code() == SENTIL_ERR_NOT_PROBABILISTIC);
    sentil_formula_destroy(det);
    sentil_formula_destroy(phi);
    sentil_lifting_registry_destroy(reg);
    sentil_trace_destroy(tr);

    sentil_sim_expr_t *advance = sentil_sim_expr_add(sentil_sim_expr_prev(0), sentil_sim_expr_noise(0));
    sentil_sim_expr_t *inits[] = {sentil_sim_expr_const(0.0)};
    sentil_sim_expr_t *advances[] = {advance};
    const char *vars[] = {"y"};
    sentil_noise_model_t *noises[] = {sentil_noise_gaussian(0.0, 1.0)};
    sentil_sim_model_t *sm = sentil_sim_model_create(vars, 1, 0.1, 20, inits, 1, advances, 1, noises, 1);
    CHECK(sm != NULL);
    sentil_trace_t *walk = sentil_sim_model_simulate(sm, 7);
    CHECK(walk != NULL && sentil_trace_len(walk) == 21);
    sentil_trace_destroy(walk);
    sentil_stochastic_system_t *sys = sentil_sim_model_to_stochastic_system(sm);
    CHECK(sys != NULL);
    sentil_stochastic_system_destroy(sys);
    sentil_sim_model_destroy(sm);

    return sentil_report("test_stats");
}