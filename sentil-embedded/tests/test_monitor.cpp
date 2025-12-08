#include "Sentil.h"

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <fstream>
#include <iterator>
#include <string>
#include <vector>

#include "sentil_test.hpp"

#ifndef SENTIL_BLOB_PATH
#define SENTIL_BLOB_PATH "rust/target/sentil_test_formula.bin"
#endif

static void canonical_streaming() {
    SentilMonitor monitor;
    CHECK(monitor.begin("always[0, 10] (x > -0.9)") == SENTIL_EMBEDDED_OK);
    std::size_t xi = 0;
    CHECK(monitor.symbolIndex("x", xi));

    bool reported = false;
    for (int t = 0; t < 60; ++t) {
        double packed[1];
        packed[xi] = std::sin(t * 0.3);
        sentil_embedded_robustness_t r;
        CHECK(monitor.update(static_cast<double>(t), packed, 1, r) == SENTIL_EMBEDDED_OK);
        if (r.resolved && !r.satisfied && !reported) {
            std::printf("canonical: first resolved violation at t=%d, robustness=%.6f\n", t, r.value);
            CHECK_CLOSE(r.value, -0.0775, 1e-3);
            reported = true;
        }
    }
    CHECK(reported);
}

static void compiled_matches_parsed() {
    std::ifstream file(SENTIL_BLOB_PATH, std::ios::binary);
    CHECK(file.good());
    std::vector<std::uint8_t> blob((std::istreambuf_iterator<char>(file)),
                                   std::istreambuf_iterator<char>());
    CHECK(!blob.empty());

    SentilMonitor parsed;
    SentilMonitor compiled;
    CHECK(parsed.begin("always[0, 5](x > 0)") == SENTIL_EMBEDDED_OK);
    CHECK(compiled.beginCompiled(blob.data(), blob.size()) == SENTIL_EMBEDDED_OK);

    for (int t = 0; t < 20; ++t) {
        double a[1];
        double b[1];
        a[0] = b[0] = std::sin(t * 0.3);
        sentil_embedded_robustness_t ra;
        sentil_embedded_robustness_t rb;
        parsed.update(static_cast<double>(t), a, 1, ra);
        compiled.update(static_cast<double>(t), b, 1, rb);
        CHECK_BITS(ra.value, rb.value);
        CHECK(ra.resolved == rb.resolved);
    }
}

static void past_operator_and_reset() {
    SentilMonitor monitor;
    CHECK(monitor.begin("historically[0, 5](pressure < 50)") == SENTIL_EMBEDDED_OK);
    std::size_t pi = 0;
    CHECK(monitor.symbolIndex("pressure", pi));

    double packed[1];
    sentil_embedded_robustness_t r;
    packed[pi] = 55.0;
    CHECK(monitor.update(0.0, packed, 1, r) == SENTIL_EMBEDDED_OK);
    CHECK(r.resolved);
    CHECK(!r.satisfied);
    CHECK_CLOSE(r.value, -5.0, 1e-12);

    monitor.reset();
    packed[pi] = 30.0;
    CHECK(monitor.update(0.0, packed, 1, r) == SENTIL_EMBEDDED_OK);
    CHECK(r.resolved);
    CHECK(r.satisfied);
    CHECK_CLOSE(r.value, 20.0, 1e-12);
}

static void error_paths() {
    SentilMonitor monitor;
    CHECK(monitor.begin("always[0,") == SENTIL_EMBEDDED_PARSE);
    CHECK(!monitor.ready());

    CHECK(monitor.begin("x > 0 and y < 1") == SENTIL_EMBEDDED_OK);
    CHECK(monitor.variableCount() == 2);
    double one[1] = {0.0};
    sentil_embedded_robustness_t r;
    CHECK(monitor.update(0.0, one, 1, r) == SENTIL_EMBEDDED_PACKED_LENGTH);

    sentil_embedded_monitor_t* handle = nullptr;
    CHECK(sentil_embedded_create(nullptr, &handle) == SENTIL_EMBEDDED_NULL_POINTER);
    CHECK(handle == nullptr);

    const std::uint8_t junk[4] = {1, 2, 3, 4};
    CHECK(sentil_embedded_create_compiled(junk, 4, &handle) == SENTIL_EMBEDDED_DECODE);
    CHECK(handle == nullptr);

    CHECK(sentil_embedded_update(nullptr, 0.0, one, 1, &r) == SENTIL_EMBEDDED_NULL_POINTER);
    CHECK(std::string(sentil_embedded_status_message(SENTIL_EMBEDDED_PARSE)).size() > 0);
    sentil_embedded_destroy(nullptr);
}

int main() {
    static std::uint8_t heap[1 << 16];
    sentil_embedded_init(heap, sizeof(heap));

    canonical_streaming();
    compiled_matches_parsed();
    past_operator_and_reset();
    error_paths();

    return sentil_report("test_monitor");
}