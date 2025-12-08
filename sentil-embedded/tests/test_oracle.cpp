#include "Sentil.h"

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <limits>
#include <sstream>
#include <string>
#include <vector>

#include "json.hpp"
#include "sentil_test.hpp"

#ifndef SENTIL_ORACLE_PATH
#define SENTIL_ORACLE_PATH "../benchmarks/deterministic/oracle.json"
#endif

static double parse_token(const std::string& t) {
    if (t == "inf") {
        return std::numeric_limits<double>::infinity();
    }
    if (t == "-inf") {
        return -std::numeric_limits<double>::infinity();
    }
    if (t == "nan") {
        return std::numeric_limits<double>::quiet_NaN();
    }
    return std::strtod(t.c_str(), nullptr);
}

int main() {
    std::ifstream in(SENTIL_ORACLE_PATH);
    if (!in.good()) {
        std::fprintf(stderr, "cannot open oracle at %s\n", SENTIL_ORACLE_PATH);
        return 1;
    }
    std::stringstream buffer;
    buffer << in.rdbuf();
    testjson::Value root = testjson::parse(buffer.str());
    const testjson::Value& cases = root["deterministic"];

    static uint8_t heap[1 << 16];
    sentil_embedded_init(heap, sizeof(heap));

    int exact = 0;
    int streamed = 0;
    for (std::size_t ci = 0; ci < cases.size(); ++ci) {
        const testjson::Value& test = cases[ci];
        const std::string id = test["id"].text;
        const std::string formula = test["formula"].text;
        const std::size_t length = static_cast<std::size_t>(test["length"].number);

        std::vector<double> expected;
        const testjson::Value& exp = test["expected"];
        for (std::size_t i = 0; i < exp.size(); ++i) {
            expected.push_back(parse_token(exp[i].text));
        }

        sentil_embedded_monitor_t* monitor = nullptr;
        sentil_embedded_status_t st = sentil_embedded_create(formula.c_str(), &monitor);
        CHECK(st == SENTIL_EMBEDDED_OK);
        if (st != SENTIL_EMBEDDED_OK) {
            continue;
        }

        const std::size_t nvars = sentil_embedded_variable_count(monitor);
        const testjson::Value& signals = test["signals"];
        std::vector<int> slot(signals.size(), -1);
        std::vector<std::vector<double>> column(signals.size());
        for (std::size_t si = 0; si < signals.size(); ++si) {
            const std::string name = signals[si]["name"].text;
            std::size_t index = 0;
            bool found = false;
            sentil_embedded_symbol_index(monitor, name.c_str(), &index, &found);
            slot[si] = found ? static_cast<int>(index) : -1;
            const testjson::Value& values = signals[si]["values"];
            for (std::size_t k = 0; k < values.size(); ++k) {
                column[si].push_back(parse_token(values[k].text));
            }
        }

        std::vector<double> got;
        std::vector<double> packed(nvars, 0.0);
        bool ran = true;
        bool all_resolved = true;
        for (std::size_t k = 0; k < length; ++k) {
            for (std::size_t si = 0; si < signals.size(); ++si) {
                if (slot[si] >= 0 && k < column[si].size()) {
                    packed[static_cast<std::size_t>(slot[si])] = column[si][k];
                }
            }
            sentil_embedded_robustness_t r;
            sentil_embedded_status_t us =
                sentil_embedded_update(monitor, static_cast<double>(k), packed.data(), nvars, &r);
            if (us != SENTIL_EMBEDDED_OK) {
                ran = false;
                break;
            }
            got.push_back(r.value);
            all_resolved = all_resolved && r.resolved;
        }
        sentil_embedded_destroy(monitor);

        CHECK(ran);
        CHECK(got.size() == length);
        CHECK(expected.size() == length);
        if (ran && all_resolved) {
            for (std::size_t k = 0; k < length && k < expected.size(); ++k) {
                if (!sentil_same_bits(got[k], expected[k])) {
                    std::fprintf(stderr, "  %s at sample %zu: got %.17g, want %.17g\n", id.c_str(), k,
                                 got[k], expected[k]);
                }
                CHECK_BITS(got[k], expected[k]);
            }
            ++exact;
        } else {
            ++streamed;
        }
    }

    std::printf("oracle: %d cases reproduced bit for bit, %d run through the stream\n", exact,
                streamed);
    CHECK(exact >= 24);
    return sentil_report("test_oracle");
}