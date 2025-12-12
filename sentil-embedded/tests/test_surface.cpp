#include "Sentil.h"

#include <cstddef>
#include <cstdint>
#include <cstring>

#include "sentil_test.hpp"

static void ring_buffer() {
    sentil_embedded_ring_buffer_t* buf = nullptr;
    CHECK(sentil_embedded_ring_buffer_create(4, &buf) == SENTIL_EMBEDDED_OK);
    for (int i = 0; i < 3; ++i) {
        sentil_embedded_sample_t evicted;
        bool did = false;
        CHECK(sentil_embedded_ring_buffer_push(buf, i, i * 2, &evicted, &did) == SENTIL_EMBEDDED_OK);
        CHECK(!did);
    }
    CHECK(sentil_embedded_ring_buffer_len(buf) == 3);
    CHECK_CLOSE(sentil_embedded_ring_buffer_mean(buf), 2.0, 1e-9);
    sentil_embedded_sample_t s;
    CHECK(sentil_embedded_ring_buffer_get(buf, 0, &s));
    CHECK_CLOSE(s.value, 0.0, 1e-9);
    sentil_embedded_ring_buffer_destroy(buf);
}

static void multi_monitor() {
    sentil_embedded_multi_monitor_t* m = nullptr;
    CHECK(sentil_embedded_multi_create(&m) == SENTIL_EMBEDDED_OK);
    CHECK(sentil_embedded_multi_add(m, "lo", "x > 0") == SENTIL_EMBEDDED_OK);
    CHECK(sentil_embedded_multi_add(m, "hi", "x < 10") == SENTIL_EMBEDDED_OK);

    const char* names[1] = {"x"};
    double values[1] = {3.0};
    CHECK(sentil_embedded_multi_update(m, 0.0, names, values, 1) == SENTIL_EMBEDDED_OK);
    CHECK(sentil_embedded_multi_count(m) == 2);

    sentil_embedded_robustness_t r;
    CHECK(sentil_embedded_multi_result(m, 0, &r) == SENTIL_EMBEDDED_OK);
    CHECK_CLOSE(r.value, 3.0, 1e-9);

    char id[8];
    CHECK(sentil_embedded_multi_id(m, 0, id, sizeof(id)) == 3);
    CHECK(std::strcmp(id, "lo") == 0);
    sentil_embedded_multi_destroy(m);
}

static void formula_introspection() {
    sentil_embedded_formula_t* f = nullptr;
    CHECK(sentil_embedded_formula_create("always[0, 5](speed < 10)", &f) == SENTIL_EMBEDDED_OK);
    CHECK(sentil_embedded_formula_has_temporal(f));
    CHECK(!sentil_embedded_formula_is_probabilistic(f));
    CHECK(sentil_embedded_formula_variable_count(f) == 1);
    char name[16];
    CHECK(sentil_embedded_formula_variable(f, 0, name, sizeof(name)) == 6);
    CHECK(std::strcmp(name, "speed") == 0);
    sentil_embedded_formula_destroy(f);
}

static void offline_trace() {
    double times[4] = {0, 1, 2, 3};
    double values[4] = {1, -2, 3, 0.5};
    sentil_embedded_trace_t* trace = nullptr;
    CHECK(sentil_embedded_trace_create(times, 4, &trace) == SENTIL_EMBEDDED_OK);
    CHECK(sentil_embedded_trace_add_signal(trace, "x", values, 4) == SENTIL_EMBEDDED_OK);

    sentil_embedded_formula_t* f = nullptr;
    CHECK(sentil_embedded_formula_create("x > 0", &f) == SENTIL_EMBEDDED_OK);

    double r = 0;
    CHECK(sentil_embedded_robustness(f, trace, &r) == SENTIL_EMBEDDED_OK);
    CHECK_CLOSE(r, 1.0, 1e-9);

    double starts[4];
    double ends[4];
    std::size_t count = 0;
    CHECK(sentil_embedded_violation_intervals(f, trace, starts, ends, 4, &count) == SENTIL_EMBEDDED_OK);
    CHECK(count == 1);
    CHECK_CLOSE(starts[0], 1.0, 1e-9);
    CHECK_CLOSE(ends[0], 1.0, 1e-9);

    sentil_embedded_formula_destroy(f);
    sentil_embedded_trace_destroy(trace);
}

static void offline_bank() {
    double times[2] = {0, 1};
    double values[2] = {2, 5};
    sentil_embedded_trace_t* trace = nullptr;
    CHECK(sentil_embedded_trace_create(times, 2, &trace) == SENTIL_EMBEDDED_OK);
    CHECK(sentil_embedded_trace_add_signal(trace, "x", values, 2) == SENTIL_EMBEDDED_OK);

    sentil_embedded_bank_t* bank = nullptr;
    CHECK(sentil_embedded_bank_create(&bank) == SENTIL_EMBEDDED_OK);
    CHECK(sentil_embedded_bank_add(bank, "p", "x > 1") == SENTIL_EMBEDDED_OK);
    CHECK(sentil_embedded_bank_robustness(bank, trace) == SENTIL_EMBEDDED_OK);
    CHECK(sentil_embedded_bank_count(bank) == 1);

    double r = 0;
    CHECK(sentil_embedded_bank_result(bank, 0, &r) == SENTIL_EMBEDDED_OK);
    CHECK_CLOSE(r, 1.0, 1e-9);
    sentil_embedded_bank_destroy(bank);
    sentil_embedded_trace_destroy(trace);
}

int main() {
    static std::uint8_t heap[1 << 16];
    sentil_embedded_init(heap, sizeof(heap));

    ring_buffer();
    multi_monitor();
    formula_introspection();
    offline_trace();
    offline_bank();

    return sentil_report("test_surface");
}