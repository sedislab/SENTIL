#include "sentil_test.h"

int main(void) {
    double times[] = {0.0, 1.0, 2.0, 3.0};
    double xs[] = {1.0, -2.0, 3.0, -4.0};
    sentil_trace_t *t = sentil_trace_create(times, 4);
    CHECK(t != NULL);
    CHECK(sentil_trace_add_signal(t, "x", xs, 4) == SENTIL_OK);
    CHECK(sentil_trace_len(t) == 4);

    size_t n = 0;
    const double *view = sentil_trace_times(t, &n);
    CHECK(view != NULL && n == 4);
    CHECK_CLOSE(view[2], 2.0, 1e-12);
    const double *sig = sentil_trace_signal(t, "x", &n);
    CHECK(sig != NULL && n == 4);
    CHECK_CLOSE(sig[3], -4.0, 1e-12);
    CHECK(sentil_trace_signal(t, "missing", &n) == NULL);
    CHECK(sentil_get_last_error_code() == SENTIL_ERR_UNKNOWN_VARIABLE);

    size_t vc = 0;
    char **vars = sentil_trace_variables(t, &vc);
    CHECK(vc == 1);
    sentil_free_string_array(vars, vc);

    double grid[] = {0.5, 1.5, 2.5};
    sentil_trace_t *rs = sentil_trace_resample(t, grid, 3, SENTIL_INTERP_LINEAR);
    CHECK(rs != NULL && sentil_trace_len(rs) == 3);
    sentil_trace_destroy(rs);
    sentil_trace_destroy(t);

    sentil_trace_t *csv = sentil_trace_from_csv("time,x\n0,1\n1,2\n2,3\n");
    CHECK(csv != NULL && sentil_trace_len(csv) == 3);
    sentil_trace_destroy(csv);

    sentil_ring_buffer_t *rb = sentil_ring_buffer_create(3);
    CHECK(rb != NULL);
    sentil_sample_t evicted;
    for (int i = 0; i < 5; i++) {
        CHECK(sentil_ring_buffer_push(rb, (double)i, (double)(i * i), &evicted) == SENTIL_OK);
    }
    CHECK(sentil_ring_buffer_is_full(rb));
    CHECK(sentil_ring_buffer_len(rb) == 3);
    sentil_sample_t front = sentil_ring_buffer_front(rb);
    CHECK(front.found);
    CHECK_CLOSE(front.time, 2.0, 1e-12);
    double mean = 0.0;
    CHECK(sentil_ring_buffer_mean(rb, &mean));
    CHECK_CLOSE(mean, (4.0 + 9.0 + 16.0) / 3.0, 1e-9);

    size_t bc = 0;
    sentil_sample_t *win = sentil_ring_buffer_between(rb, 2.0, 4.0, &bc);
    CHECK(win != NULL && bc == 3);
    sentil_free_samples(win, bc);
    double at = 0.0;
    CHECK(sentil_ring_buffer_at_time(rb, 3.0, &at));
    CHECK_CLOSE(at, 9.0, 1e-12);
    sentil_ring_buffer_destroy(rb);

    return sentil_report("test_signal");
}