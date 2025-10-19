#include <sentil/sentil.hpp>

#include <string>

#include "sentil_test.hpp"

using namespace sentil;

template <typename Fn>
static bool throws_parse(Fn fn) {
    try {
        fn();
    } catch (const ParseError&) {
        return true;
    } catch (...) {
        return false;
    }
    return false;
}

template <typename Fn>
static bool throws_semantic(Fn fn) {
    try {
        fn();
    } catch (const SemanticError&) {
        return true;
    } catch (...) {
        return false;
    }
    return false;
}

template <typename Fn>
static bool throws_sentil(Fn fn) {
    try {
        fn();
    } catch (const SentilError&) {
        return true;
    } catch (...) {
        return false;
    }
    return false;
}

int main() {
    CHECK(throws_parse([] { Formula::parse("always[0,"); }));
    try {
        Formula::parse("x >");
    } catch (const ParseError& e) {
        CHECK(std::string(e.what()).size() > 0);
        CHECK(e.code() == SENTIL_ERR_PARSE);
    }

    Trace t = Trace::indexed(2);
    t.add_signal("x", {1, 2});
    CHECK(throws_semantic([&] { Formula::parse("y > 0").robustness(t); }));

    LiftingRegistry reg;
    reg.register_noise("x", NoiseModel::gaussian(0, 1));
    CHECK(throws_semantic([&] { Formula::parse("x > 0").check(t, reg); }));

    CHECK(throws_sentil([] { NoiseModel::gaussian(0.0, -1.0); }));
    CHECK(throws_sentil([] { Bounds({0, 0}, {1}); }));
    CHECK(throws_sentil([] { SpecBuilder("not-a-real-spec"); }));

    CHECK(throws_sentil([] { Formula::parse("@@@"); }));

    FormulaBank bank;
    bank.add("good", "x > 0");
    bank.add("bad", "missing > 0");
    bool named = false;
    try {
        bank.robustness(t);
    } catch (const SentilError& e) {
        named = std::string(e.what()).find("'bad'") != std::string::npos;
    }
    CHECK(named);

    return sentil_report("test_errors");
}