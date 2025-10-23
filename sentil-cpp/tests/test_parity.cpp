#include <sentil/sentil.hpp>

#include <cmath>
#include <random>
#include <stdexcept>
#include <string>
#include <vector>

#include "sentil_test.hpp"

using namespace sentil;

static std::string json_of(const Formula& f) { return f.to_json(); }

static std::vector<std::vector<double>> integrate(const std::vector<double>& initial,
                                                  const std::vector<double>& input) {
    std::vector<double> trajectory{initial[0]};
    for (double u : input) {
        trajectory.push_back(trajectory.back() + u);
    }
    return {trajectory};
}

int main() {
    CHECK(json_of(sqrt(Expr::var("x")) > 2.0) == json_of(Formula::parse("sqrt(x) > 2")));
    CHECK(json_of(min(Expr::var("x"), 0.0) < 1.0) == json_of(Formula::parse("min(x, 0) < 1")));
    CHECK(json_of(max(Expr::var("x"), Expr::var("y")) > 0.0) ==
          json_of(Formula::parse("max(x, y) > 0")));
    CHECK(json_of(pow(Expr::var("x"), 2.0) > 4.0) == json_of(Formula::parse("x ^ 2 > 4")));
    CHECK(json_of((Expr::var("x") % 3.0) == 0.0) == json_of(Formula::parse("x % 3 == 0")));

    LiftingRegistry reg;
    reg.register_noise("x", NoiseModel::gaussian(0.0, 0.3));
    Formula prob = Formula::parse("P>=0.8 (always (x > 0))");
    OnlineMonitor online = OnlineMonitor::with_lifting(prob, reg);
    online.update(0, {{"x", 2.0}});
    MultiMonitor multi;
    multi.add("det", "x > 0");
    multi.add_probabilistic("prob", prob, reg);
    CHECK(multi.update(0, {{"x", 2.0}}).count("prob") == 1);

    Bounds box({-1, -1}, {1, 1});
    CHECK(box.clamp({2.0, -3.0}) == std::vector<double>({1.0, -1.0}));
    CHECK(!violation_intervals({0, 1, 2}, {1.0, -1.0, 2.0}).empty());
    CHECK(SpecBuilder::available().size() > 0);
    std::optional<SpecSmcSettings> settings = SpecBuilder(SpecBuilder::available().front()).smc_settings();
    CHECK(!settings.has_value() || (settings->confidence > 0.0 && settings->confidence < 1.0));

    Formula phi = Formula::parse("always[0,2](x > 0)");
    Trace t = Trace::indexed(3);
    t.add_signal("x", {3, 1, 2});
    std::pair<double, std::vector<std::vector<double>>> grad = phi.smooth_value_and_gradient(t);
    CHECK(grad.second.size() == 1 && grad.second[0].size() == 3);

    Bounds line({-10.0}, {10.0});
    std::pair<std::vector<double>, double> peak =
        synthesis::cma_es([](const std::vector<double>& x) { return -(x[0] - 3.0) * (x[0] - 3.0); },
                          {0.0}, &line);
    CHECK(std::fabs(peak.first[0] - 3.0) < 0.2);
    bool objective_threw = false;
    try {
        synthesis::cma_es([](const std::vector<double>&) -> double {
            throw std::runtime_error("boom");
        }, {0.0}, &line);
    } catch (const std::runtime_error&) {
        objective_threw = true;
    }
    CHECK(objective_threw);

    std::mt19937 rng(7);
    std::bernoulli_distribution coin(0.7);
    CHECK(stats::sequential_test(SprtConfig{0.4, 0.8}, [&] { return coin(rng); }).samples > 0);
    std::vector<Trace> traces;
    Trace a = Trace::indexed(3);
    a.add_signal("x", {2.0, 3.0, 2.5});
    traces.push_back(std::move(a));
    double mined = mine_tightest_parameter(
        [](double p) { return Formula::parse("always (x > " + std::to_string(p) + ")"); }, traces,
        0.0, 5.0);
    CHECK(mined > 1.9 && mined < 2.1);

    struct Walk {
        double x;
        int step;
    };
    AmsSimulator<Walk> walk;
    walk.initial_state = [](std::uint64_t) { return Walk{0.0, 0}; };
    walk.step = [](const Walk& w, std::uint64_t seed) {
        std::mt19937_64 r(seed);
        std::normal_distribution<double> n(0.0, 1.0);
        return Walk{w.x + n(r), w.step + 1};
    };
    walk.is_terminal = [](const Walk& w, bool& rare) {
        rare = w.x >= 6.0;
        return w.step >= 30 || w.x >= 6.0;
    };
    walk.score = [](const Walk& w) { return w.x; };
    CHECK(adaptive_multilevel_splitting<Walk>(walk, 1024, 6.0, 30, 42).simulations > 0);

    StochasticSystem system = StochasticSystem::custom(
        {"x"}, 1.0, 20,
        [](std::uint64_t seed) {
            std::mt19937_64 r(seed);
            std::normal_distribution<double> n(0, 1);
            return std::vector<double>{n(r)};
        },
        [](const std::vector<double>& prev, double, std::uint64_t seed) {
            std::mt19937_64 r(seed);
            std::normal_distribution<double> n(0, 0.5);
            return std::vector<double>{0.5 * prev[0] + n(r)};
        });
    CHECK(system.simulate(42).size() == 21);

    SystemModel model = SystemModel::custom({"x"}, 1.0, 3, {1.0}, 3, integrate);
    Bounds limits({-1, -1, -1}, {1, 1, 1});
    CHECK(synthesis::synthesize(model, Formula::parse("always (x > 0)"), &limits).input.size() == 3);
    Controller controller(SystemModel::custom({"x"}, 1.0, 3, {0.0}, 3, integrate),
                          Formula::parse("eventually[0,3](x > 1)"), 3, 5'000'000, &limits);
    CHECK(controller.control({0.0}).size() == 3);
    bool rollout_threw = false;
    try {
        SystemModel bad = SystemModel::custom(
            {"x"}, 1.0, 3, {0.0}, 3,
            [](const std::vector<double>&,
               const std::vector<double>&) -> std::vector<std::vector<double>> {
                throw std::runtime_error("rollout boom");
            });
        synthesis::synthesize(bad, Formula::parse("always (x > 0)"), &limits);
    } catch (const std::runtime_error&) {
        rollout_threw = true;
    }
    CHECK(rollout_threw);

    return sentil_report("test_parity");
}