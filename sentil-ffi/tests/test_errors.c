#include "sentil_test.h"

int main(void) {
    CHECK(sentil_formula_parse("always (") == NULL);
    CHECK(sentil_get_last_error_code() == SENTIL_ERR_PARSE);

    char bad_utf8[] = {(char)0xff, (char)0xfe, 0};
    CHECK(sentil_formula_parse(bad_utf8) == NULL);
    CHECK(sentil_get_last_error_code() == SENTIL_ERR_UTF8);

    double bad_times[] = {0.0, 1.0, 0.5};
    CHECK(sentil_trace_create(bad_times, 3) == NULL);
    CHECK(sentil_get_last_error_code() == SENTIL_ERR_TRACE);
    CHECK(strlen(sentil_get_last_error()) > 0);

    double times[] = {0.0, 1.0};
    double values[] = {1.0, 2.0, 3.0};
    sentil_trace_t *tr = sentil_trace_create(times, 2);
    CHECK(sentil_trace_add_signal(tr, "x", values, 3) == SENTIL_ERR_TRACE);
    sentil_trace_destroy(tr);

    CHECK(sentil_noise_gaussian(0.0, -1.0) == NULL);
    CHECK(sentil_get_last_error_code() == SENTIL_ERR_INVALID_NOISE_MODEL);

    CHECK(sentil_ring_buffer_create(0) == NULL);
    CHECK(sentil_get_last_error_code() == SENTIL_ERR_INVALID_CONFIG);

    double ty[] = {0.0, 1.0};
    double y[] = {1.0, 2.0};
    sentil_trace_t *tr2 = sentil_trace_create(ty, 2);
    sentil_trace_add_signal(tr2, "y", y, 2);
    sentil_monitor_t *m = sentil_monitor_parse("always (x > 0)", NULL);
    double rob = 0.0;
    CHECK(sentil_monitor_robustness(m, tr2, &rob) == SENTIL_ERR_UNKNOWN_VARIABLE);
    CHECK(strstr(sentil_get_last_error(), "x") != NULL);
    sentil_monitor_destroy(m);
    sentil_trace_destroy(tr2);

    CHECK(sentil_formula_depth(NULL) == 0);
    CHECK(sentil_get_last_error_code() == SENTIL_ERR_NULL_POINTER);

    return sentil_report("test_errors");
}