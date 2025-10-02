#include "sentil_test.h"

int main(void) {
    sentil_formula_t *g = sentil_formula_parse("always[0,5] (x > 0)");
    CHECK(g != NULL);
    char *j1 = sentil_formula_to_json(g);
    CHECK(j1 != NULL);
    sentil_formula_t *back = sentil_formula_from_json(j1);
    CHECK(back != NULL);
    char *j2 = sentil_formula_to_json(back);
    CHECK(j2 != NULL && strcmp(j1, j2) == 0);
    sentil_free_string(j1);
    sentil_free_string(j2);
    sentil_formula_destroy(g);
    sentil_formula_destroy(back);

    sentil_expr_t *sum = sentil_expr_binary(SENTIL_BIN_ADD, sentil_expr_variable("x"),
                                            sentil_expr_literal(1.0));
    sentil_formula_t *pred = sentil_formula_predicate(sum, SENTIL_CMP_GT, sentil_expr_literal(0.0));
    sentil_formula_t *always = sentil_formula_always(0.0, 3.0, true, pred);
    CHECK(always != NULL);
    CHECK(sentil_formula_has_temporal(always));
    size_t n = 0;
    char **vars = sentil_formula_variables(always, &n);
    CHECK(vars != NULL && n == 1 && strcmp(vars[0], "x") == 0);
    sentil_free_string_array(vars, n);
    sentil_formula_destroy(always);

    sentil_formula_t *inner =
        sentil_formula_predicate(sentil_expr_variable("x"), SENTIL_CMP_GT, sentil_expr_literal(0.0));
    sentil_formula_t *ev = sentil_formula_eventually(0.0, 0.0, false, inner);
    sentil_formula_t *prob = sentil_formula_probabilistic(SENTIL_PROB_GE, 0.9, ev);
    CHECK(prob != NULL);
    sentil_formula_destroy(prob);

    sentil_formula_t *child =
        sentil_formula_predicate(sentil_expr_variable("x"), SENTIL_CMP_GT, sentil_expr_literal(0.0));
    CHECK(sentil_formula_probabilistic(SENTIL_PROB_GE, 1.5, child) == NULL);
    CHECK(sentil_get_last_error_code() == SENTIL_ERR_INVALID_CONFIG);

    sentil_expr_destroy(sentil_expr_literal(3.0));
    return sentil_report("test_formula");
}