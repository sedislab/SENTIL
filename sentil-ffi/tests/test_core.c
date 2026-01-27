#include "sentil_test.h"
#include <stddef.h>

int main(void) {
    uint32_t mj = 0, mn = 0, pa = 0;
    sentil_version(&mj, &mn, &pa);
    CHECK(mj == 0 && mn == 3 && pa == 0);
    sentil_version(NULL, NULL, NULL);

    sentil_formula_t *f = sentil_formula_parse("always[0,5] (x > 0)");
    CHECK(f != NULL);
    CHECK(sentil_formula_has_temporal(f));
    CHECK(sentil_formula_depth(f) >= 2);
    sentil_formula_destroy(f);

    sentil_formula_t *bad = sentil_formula_parse("always[0,5] (x > )");
    CHECK(bad == NULL);
    CHECK(sentil_get_last_error_code() == SENTIL_ERR_PARSE);
    CHECK(strlen(sentil_get_last_error()) > 0);
    CHECK(strstr(sentil_get_last_error(), "column") != NULL);

    CHECK(sentil_formula_parse(NULL) == NULL);
    CHECK(sentil_get_last_error_code() == SENTIL_ERR_NULL_POINTER);

    sentil_formula_t *ok = sentil_formula_parse("x > 0");
    CHECK(ok != NULL);
    CHECK(sentil_get_last_error_code() == SENTIL_OK);
    sentil_formula_destroy(ok);

    sentil_formula_parse("bad (");
    size_t need = sentil_get_last_error_message(NULL, 0);
    CHECK(need > 0);
    char buf[256];
    size_t wrote = sentil_get_last_error_message(buf, sizeof buf);
    CHECK(wrote == need);
    CHECK(strlen(buf) == need - 1);

    sentil_formula_destroy(NULL);
    sentil_free_string(NULL);
    sentil_free_string_array(NULL, 0);

    return sentil_report("test_core");
}