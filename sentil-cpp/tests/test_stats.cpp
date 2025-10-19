#include <sentil/sentil.hpp>

#include <cmath>
#include <vector>

#include "sentil_test.hpp"

using namespace sentil;

int main() {
    auto w = stats::wilson_interval(50, 100, 0.95);
    CHECK_CLOSE(w.lower, 0.403831, 1e-5);
    CHECK_CLOSE(w.upper, 0.596169, 1e-5);
    auto cp = stats::clopper_pearson(50, 100, 0.95);
    CHECK_CLOSE(cp.lower, 0.398321, 1e-5);
    CHECK_CLOSE(cp.upper, 0.601679, 1e-5);
    CHECK_CLOSE(stats::z_score(0.95), 1.95996, 1e-5);
    CHECK(stats::chernoff_hoeffding_samples(0.1, 0.05) == 185);

    NoiseModel g = NoiseModel::gaussian(2.0, 0.5);
    CHECK_CLOSE(*g.mean(), 2.0, 1e-12);
    CHECK_CLOSE(*g.variance(), 0.25, 1e-12);
    CHECK(!NoiseModel::cauchy(0, 1).mean().has_value());
    auto residuals = NoiseModel::residuals({10, 20, 30}, {10.5, 19, 31}, NoiseInteraction::Additive);
    CHECK_CLOSE(residuals[0], 0.5, 1e-12);
    CHECK(NoiseModel::fit_gaussian({1, 2, 3, 4, 5}).mean().has_value());
    NoiseModel mix = NoiseModel::mixture({0.5, 0.5}, NoiseModel::gaussian(0, 1),
                                         NoiseModel::gaussian(5, 1));
    CHECK(g.to_json() == NoiseModel::from_json(g.to_json()).to_json());
    (void)mix;

    LiftingRegistry reg;
    reg.register_noise("x", NoiseModel::gaussian(0.0, 1.0));
    CHECK(!reg.empty() && reg.variables() == std::vector<std::string>({"x"}));
    Trace t = Trace::indexed(20);
    t.add_signal("x", std::vector<double>(20, 1.5));
    CHECK(reg.lift(t, 42).signal("x") == reg.lift(t, 42).signal("x"));

    Formula phi = Formula::parse("P>=0.5 (always (x > 0))");
    SmcResult result = phi.check(t, reg, SmcConfig{2000, 0.95, 7});
    CHECK(result.samples == 2000);
    CHECK(result.probability > 0.0 && result.probability < 1.0);
    CHECK(result.interval.lower <= result.probability && result.probability <= result.interval.upper);
    CHECK(phi.check_conservative(t, reg).samples == 10000);
    auto distribution = phi.check_distribution(t, reg, SmcConfig{500, 0.95, 1});
    CHECK(distribution.first.samples == 500 && distribution.second.count > 0);

    SprtResult sprt = phi.check_sequential(t, reg, SprtConfig{0.3, 0.7});
    CHECK(sprt.samples > 0);
    BayesResult bayes = phi.check_bayesian(t, reg, BayesConfig{0.5});
    CHECK(bayes.posterior >= 0.0 && bayes.posterior <= 1.0);

    std::vector<SimExpr> init;
    init.push_back(SimExpr::constant(0.0));
    std::vector<SimExpr> advance;
    advance.push_back(SimExpr::prev(0) * 0.5 + SimExpr::noise(0));
    std::vector<NoiseModel> noise;
    noise.push_back(NoiseModel::gaussian(0.0, 1.0));
    SimModel model({"x"}, 1.0, 20, std::move(init), std::move(advance), std::move(noise));
    CHECK(model.simulate(42).signal("x") == model.simulate(42).signal("x"));
    StochasticSystem system = model.to_stochastic_system();
    Formula rare_spec = Formula::parse("P>=0.001 (always (x < 8))");
    RareEventResult rare = rare_spec.check_rare_event(system, RareEventConfig{2048, 0.0, 1});
    CHECK(rare.simulations > 0);
    CHECK_CLOSE(rare.probability + rare.violation_probability, 1.0, 1e-9);

    return sentil_report("test_stats");
}