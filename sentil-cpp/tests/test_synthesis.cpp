#include <sentil/sentil.hpp>

#include <cmath>
#include <vector>

#include "sentil_test.hpp"

using namespace sentil;

int main() {
    std::vector<double> values{1.0, 3.0, 2.0};
    CHECK_CLOSE(synthesis::soft_min(values, 100.0), 1.0, 0.5);
    CHECK_CLOSE(synthesis::soft_max(values, 100.0), 3.0, 0.5);
    Formula phi = Formula::parse("always[0,2](x > 0)");
    Trace t = Trace::indexed(3);
    t.add_signal("x", {3, 1, 2});
    CHECK(std::isfinite(phi.smooth_robustness(t)));

    auto x = synthesis::solve_spd({{2, 0}, {0, 2}}, {4, 6});
    CHECK_CLOSE(x[0], 2.0, 1e-9);
    CHECK_CLOSE(x[1], 3.0, 1e-9);
    auto eig = synthesis::symmetric_eigen({{3, 0}, {0, 5}});
    CHECK(eig.first.size() == 2);
    auto u = synthesis::solve_qp({{1.0}}, {0.0}, {{-1.0}}, {-1.0});
    CHECK_CLOSE(u[0], 1.0, 1e-4);

    Bounds bounds({-1, -1}, {1, 1});
    CHECK(bounds.dimension() == 2);

    SystemModel model = SystemModel::linear({{1.0}}, {{1.0}}, {0.0}, {"x"}, 1.0, 10);
    CHECK(model.input_dimension() == 10);
    Formula spec = Formula::parse("eventually[0,10](x > 5)");
    Bounds input_bounds({-2.0}, {2.0});
    SynthesisResult synthesized =
        synthesis::synthesize(model, spec, &input_bounds, nullptr, Backend::Gradient, 300);
    CHECK(synthesized.input.size() == 10 && synthesized.backend == Backend::Gradient);

    SafetyFilter filter(Bounds({-1, -1}, {1, 1}));
    auto projected = filter.filter({2.0, -3.0});
    CHECK(projected[0] <= 1.0 + 1e-9 && projected[1] >= -1.0 - 1e-9);

    std::vector<SimExpr> init;
    init.push_back(SimExpr::constant(0.0));
    std::vector<SimExpr> advance;
    advance.push_back(SimExpr::prev(0) * 0.5 + SimExpr::noise(0));
    std::vector<NoiseModel> noise;
    noise.push_back(NoiseModel::gaussian(0.0, 0.5));
    SimModel sim({"x"}, 1.0, 15, std::move(init), std::move(advance), std::move(noise));
    StochasticSystem system = sim.to_stochastic_system();
    ChanceConstraint chance(Formula::parse("always (x < 5)"), 0.9, 0.95);
    ChanceReport report = chance.validate(system, 500, 42);
    CHECK(report.samples == 500 && report.estimate >= 0.0 && report.estimate <= 1.0);

    Controller controller(SystemModel::linear({{1.0}}, {{1.0}}, {0.0}, {"x"}, 1.0, 5),
                          Formula::parse("eventually[0,5](x > 3)"), 1, 5'000'000, &input_bounds);
    CHECK(controller.control({0.0}).size() == 1);

    Witness witness =
        Formula::parse("always (x < 2)").find_counterexample(model, input_bounds, 300);
    CHECK(!witness.input.empty() && witness.trace.size() == 11);
    CHECK(witness.robustness < 0.0);

    return sentil_report("test_synthesis");
}