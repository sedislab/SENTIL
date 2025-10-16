#include <sentil/sentil.hpp>

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
#define SENTIL_ORACLE_PATH "../../benchmarks/deterministic/oracle.json"
#endif

static double parse_token(const std::string& token) {
    if (token == "inf") {
        return std::numeric_limits<double>::infinity();
    }
    if (token == "-inf") {
        return -std::numeric_limits<double>::infinity();
    }
    if (token == "nan") {
        return std::numeric_limits<double>::quiet_NaN();
    }
    return std::strtod(token.c_str(), nullptr);
}

static std::vector<double> parse_values(const testjson::Value& array) {
    std::vector<double> out;
    out.reserve(array.size());
    for (std::size_t i = 0; i < array.size(); ++i) {
        out.push_back(parse_token(array[i].text));
    }
    return out;
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
    int reproduced = 0;
    for (std::size_t ci = 0; ci < cases.size(); ++ci) {
        const testjson::Value& test = cases[ci];
        const std::string id = test["id"].text;
        const std::string formula = test["formula"].text;
        const std::size_t length = static_cast<std::size_t>(test["length"].number);

        sentil::Trace trace = sentil::Trace::indexed(length);
        const testjson::Value& signals = test["signals"];
        for (std::size_t si = 0; si < signals.size(); ++si) {
            const testjson::Value& signal = signals[si];
            trace.add_signal(signal["name"].text, parse_values(signal["values"]));
        }

        const std::vector<double> expected = parse_values(test["expected"]);
        sentil::Formula phi = sentil::Formula::parse(formula);
        const std::vector<double> got = phi.robustness_signal(trace);

        CHECK(got.size() == expected.size());
        const std::size_t n = got.size() < expected.size() ? got.size() : expected.size();
        for (std::size_t i = 0; i < n; ++i) {
            if (!sentil_same_bits(got[i], expected[i])) {
                std::fprintf(stderr, "  %s at sample %zu: got %.17g, want %.17g\n", id.c_str(), i,
                             got[i], expected[i]);
            }
            CHECK_BITS(got[i], expected[i]);
        }
        ++reproduced;
    }

    std::printf("oracle: reproduced %d deterministic cases\n", reproduced);
    CHECK(reproduced >= 44);
    return sentil_report("test_oracle");
}