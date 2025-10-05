#include "sentil_test.h"

int main(void) {
    double times[] = {0.0, 1.0, 2.0, 3.0};
    double xs[] = {1.0, -2.0, 3.0, -4.0};
    sentil_trace_t *tr = sentil_trace_create(times, 4);
    sentil_trace_add_signal(tr, "x", xs, 4);

    sentil_formula_t *phi = sentil_formula_parse("always (x > 0)");
    double frob = 0.0;
    CHECK(sentil_formula_robustness(phi, tr, &frob) == SENTIL_OK);
    CHECK_CLOSE(frob, -4.0, 1e-9);
    double fdense = 0.0;
    CHECK(sentil_formula_robustness_dense(phi, tr, &fdense) == SENTIL_OK);
    size_t fl = 0;
    double *fsig = sentil_formula_robustness_signal(phi, tr, &fl);
    CHECK(fsig != NULL && fl == 4);
    sentil_free_doubles(fsig, fl);
    double *fdsig = sentil_formula_robustness_dense_signal(phi, tr, &fl);
    CHECK(fdsig != NULL && fl == 4);
    sentil_free_doubles(fdsig, fl);
    size_t fvc = 0;
    sentil_interval_t *fviol = sentil_formula_violations(phi, tr, &fvc);
    CHECK(fviol != NULL);
    sentil_free_intervals(fviol, fvc);
    sentil_formula_destroy(phi);

    sentil_monitor_t *m = sentil_monitor_parse("always (x > 0)", NULL);
    CHECK(m != NULL);
    double rob = 0.0;
    CHECK(sentil_monitor_robustness(m, tr, &rob) == SENTIL_OK);
    CHECK_CLOSE(rob, -4.0, 1e-9);
    size_t n = 0;
    double *sig = sentil_monitor_robustness_signal(m, tr, &n);
    CHECK(sig != NULL && n == 4);
    sentil_free_doubles(sig, n);
    size_t vc = 0;
    sentil_interval_t *viol = sentil_monitor_violations(m, tr, &vc);
    CHECK(viol != NULL);
    sentil_free_intervals(viol, vc);
    sentil_monitor_destroy(m);

    sentil_monitor_config_t *cfg = sentil_monitor_config_create();
    sentil_monitor_config_set_time(cfg, SENTIL_TIME_DENSE);
    CHECK(sentil_monitor_config_time_mode(cfg) == SENTIL_TIME_DENSE);
    sentil_monitor_t *sm = sentil_monitor_parse("x > 0", cfg);
    sentil_monitor_config_destroy(cfg);
    const char *names[] = {"x"};
    double vals[] = {5.0};
    sentil_robustness_t out;
    CHECK(sentil_monitor_update(sm, 0.0, names, vals, 1, &out) == SENTIL_OK);
    CHECK(out.satisfied);
    double packed[] = {-3.0};
    CHECK(sentil_monitor_update_packed(sm, 1.0, packed, 1, &out) == SENTIL_OK);
    CHECK(!out.satisfied);
    size_t idx = 0;
    bool found = false;
    CHECK(sentil_monitor_symbol_index(sm, "x", &idx, &found) == SENTIL_OK && found);
    sentil_monitor_reset(sm);
    sentil_monitor_destroy(sm);

    sentil_stream_monitor_t *str = sentil_stream_monitor_create("eventually (x > 2)");
    CHECK(str != NULL);
    size_t steps = 0;
    sentil_robustness_t *run = sentil_stream_monitor_run(str, tr, &steps);
    CHECK(run != NULL && steps == 4);
    sentil_free_robustness(run, steps);
    sentil_stream_monitor_destroy(str);

    sentil_multi_monitor_t *mm = sentil_multi_monitor_create();
    CHECK(sentil_multi_monitor_add(mm, "safe", "x > 0") == SENTIL_OK);
    CHECK(sentil_multi_monitor_add(mm, "big", "x > 2") == SENTIL_OK);
    CHECK(sentil_multi_monitor_len(mm) == 2);
    size_t rc = 0;
    sentil_named_robustness_t *res = sentil_multi_monitor_update(mm, 0.0, names, vals, 1, &rc);
    CHECK(res != NULL && rc == 2);
    CHECK(strcmp(res[0].id, "safe") == 0);
    sentil_free_named_robustness(res, rc);
    sentil_multi_monitor_destroy(mm);

    sentil_formula_bank_t *bank = sentil_formula_bank_create();
    sentil_formula_bank_add(bank, "a", "always (x > 0)");
    sentil_formula_bank_add(bank, "b", "eventually (x > 2)");
    size_t bc = 0;
    sentil_bank_result_t *br = sentil_formula_bank_robustness(bank, tr, &bc);
    CHECK(br != NULL && bc == 2 && br[0].ok);
    sentil_free_bank_results(br, bc);
    sentil_formula_bank_destroy(bank);

    sentil_trace_destroy(tr);
    return sentil_report("test_monitor");
}