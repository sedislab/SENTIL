#include <sentil/sentil.hpp>

#include <cstdio>
#include <vector>

int main() {
    std::vector<double> times;
    std::vector<double> values;
    for (int i = 0; i < 20; ++i) {
        times.push_back(i);
        values.push_back(0.4 + 0.05 * i);
    }
    sentil::Trace trace(times, "x", values);

    sentil::LiftingRegistry lifting;
    lifting.register_noise("x", sentil::NoiseModel::gaussian(0.0, 0.3));

    sentil::Formula phi = sentil::Formula::parse("P>=0.9 (always (x > 0))");
    sentil::SmcConfig config;
    config.samples = 5000;
    sentil::SmcResult result = phi.check(trace, lifting, config);
    std::printf("probability %.3f, interval [%.3f, %.3f], holds %s\n", result.probability,
                result.interval.lower, result.interval.upper, result.holds ? "true" : "false");
    return 0;
}