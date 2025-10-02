#include "sentil_test.h"

int main(void) {
    size_t n = 0;
    char **available = sentil_spec_registry_available(&n);
    CHECK(available != NULL);

    if (n > 0) {
        sentil_spec_builder_t *builder = sentil_spec_builder_create(available[0]);
        CHECK(builder != NULL);
        char *deterministic = sentil_spec_builder_build_deterministic(builder);
        CHECK(deterministic != NULL && strlen(deterministic) > 0);
        sentil_free_string(deterministic);
        sentil_formula_t *formula = sentil_spec_builder_build_formula(builder);
        CHECK(formula != NULL);
        sentil_formula_destroy(formula);
        char *params = sentil_spec_builder_parameters_json(builder);
        CHECK(params != NULL);
        sentil_free_string(params);
        sentil_spec_builder_destroy(builder);
    }
    sentil_free_string_array(available, n);

    CHECK(sentil_spec_builder_create("no/such/spec") == NULL);
    CHECK(sentil_get_last_error_code() == SENTIL_ERR_INVALID_CONFIG);

    return sentil_report("test_specs");
}