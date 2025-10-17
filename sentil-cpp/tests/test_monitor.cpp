#include <sentil/sentil.hpp>

#include <vector>

#include "sentil_test.hpp"

using sentil::Config;
using sentil::Formula;
using sentil::FormulaBank;
using sentil::Monitor;
using sentil::MultiMonitor;
using sentil::OnlineMonitor;
using sentil::TimeMode;
using sentil::Trace;

int main() {
    Monitor m("always[0,2](x > 0)");
    Trace t = Trace::indexed(3);
    t.add_signal("x", {3, 1, 2});
    CHECK_BITS(m.robustness(t), 1.0);
    CHECK(m.robustness_signal(t) == std::vector<double>({1, 1, 2}));
    CHECK(m.config().time() == TimeMode::Discrete);
    CHECK(m.symbol_index("x").value() == 0);
    CHECK(!m.symbol_index("absent").has_value());

    Monitor fold("(x > 0) and (y > 0)");
    CHECK(fold.update(0, {{"x", 4.0}, {"y", 2.0}}).value == 2.0);
    std::vector<double> packed(2);
    packed[*fold.symbol_index("x")] = 4.0;
    packed[*fold.symbol_index("y")] = -1.0;
    CHECK(fold.update_packed(1, packed).value == -1.0);

    Monitor dense(Formula::parse("always(x > 0)"), Config(TimeMode::Dense));
    CHECK(dense.config().time() == TimeMode::Dense);

    OnlineMonitor online("x > 0");
    CHECK(online.variable_count() == 1);
    auto run = online.run(t);
    CHECK(run.size() == 3 && run[0].value == 3 && run[1].value == 1);

    MultiMonitor mm;
    mm.add("pos", "x > 0");
    mm.add("big", Formula::parse("x > 10"));
    CHECK(mm.ids() == std::vector<std::string>({"pos", "big"}));
    auto verdicts = mm.update(0, {{"x", 5.0}});
    CHECK(verdicts.at("pos").satisfied && !verdicts.at("big").satisfied);
    CHECK(mm.remove("big") && mm.size() == 1);

    FormulaBank bank;
    bank.add("a", "x > 0");
    bank.add("b", "always[0,1](x > 0)");
    auto results = bank.robustness(t);
    CHECK_BITS(results.at("a"), 3.0);
    CHECK_BITS(results.at("b"), 1.0);

    return sentil_report("test_monitor");
}