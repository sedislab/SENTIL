#include <sentil/sentil.hpp>

#include <string>

#include "sentil_test.hpp"

using sentil::Expr;
using sentil::Formula;
using sentil::ProbabilityOp;
using sentil::Trace;

static std::string json_of(const Formula& f) { return f.to_json(); }

int main() {
    Formula phi = Formula::parse("always[0,10](x > 0)");
    CHECK(phi.is_temporal());
    CHECK(phi.depth() >= 2);
    CHECK(phi.variables().size() == 1 && phi.variables()[0] == "x");
    CHECK(Formula::from_json(phi.to_json()).to_json() == phi.to_json());

    CHECK(json_of((Expr::var("x") * 2.0) > 5.0) == json_of(Formula::parse("x * 2 > 5")));
    CHECK(json_of(5.0 < Expr::var("x")) == json_of(Formula::parse("5 < x")));
    CHECK(json_of(sentil::abs(Expr::var("y")) <= 3.0) == json_of(Formula::parse("abs(y) <= 3")));
    CHECK(json_of((Expr::var("a") - Expr::var("b")) != 0.0) ==
          json_of(Formula::parse("a - b != 0")));

    CHECK(json_of(!(Expr::var("x") > 0.0)) == json_of(Formula::parse("not(x > 0)")));
    CHECK(json_of((Expr::var("x") > 0.0) && (Expr::var("y") > 0.0)) ==
          json_of(Formula::parse("(x > 0) and (y > 0)")));
    CHECK(json_of((Expr::var("x") > 0.0) || (Expr::var("y") > 0.0)) ==
          json_of(Formula::parse("(x > 0) or (y > 0)")));
    CHECK(json_of(sentil::implies(Expr::var("x") > 0.0, Expr::var("y") > 0.0)) ==
          json_of(Formula::parse("(x > 0) implies (y > 0)")));
    CHECK(json_of(sentil::next(Expr::var("x") > 0.0)) == json_of(Formula::parse("next(x > 0)")));
    CHECK(json_of(sentil::always(Expr::var("x") > 0.0, 0, 10)) ==
          json_of(Formula::parse("always[0,10](x > 0)")));
    CHECK(json_of(sentil::eventually(Expr::var("x") > 0.0)) ==
          json_of(Formula::parse("eventually(x > 0)")));
    CHECK(json_of(sentil::until(Expr::var("x") > 0.0, Expr::var("y") > 0.0, 0, 2)) ==
          json_of(Formula::parse("(x > 0) until[0,2] (y > 0)")));
    CHECK(json_of((Expr::var("x") > 0.0).always(0, 10)) ==
          json_of(Formula::parse("always[0,10](x > 0)")));
    CHECK(json_of(sentil::probability(sentil::always(Expr::var("x") > 0.0), ProbabilityOp::Ge,
                                      0.95)) == json_of(Formula::parse("P>=0.95(always(x > 0))")));

    Trace t = Trace::indexed(3);
    t.add_signal("x", {3, 1, 2});
    Formula bounded = Formula::parse("always[0,2](x > 0)");
    CHECK_BITS(bounded.robustness(t), 1.0);
    std::vector<double> sig = bounded.robustness_signal(t);
    CHECK(sig.size() == 3);
    CHECK_BITS(sig[0], 1.0);
    CHECK_BITS(sig[2], 2.0);

    Trace dip = Trace::indexed(3);
    dip.add_signal("x", {1, -2, 3});
    CHECK(!Formula::parse("always(x > 0)").violations(dip).empty());

    return sentil_report("test_formula");
}