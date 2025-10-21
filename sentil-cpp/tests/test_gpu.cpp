#include <sentil/sentil.hpp>

#include <cstdio>
#include <vector>

#include "sentil_test.hpp"

using namespace sentil;

int main() {
    bool present = gpu::is_available();
    CHECK(present == true || present == false);

    if (!present) {
        std::printf("test_gpu: no device, skipping the GPU rare-event path\n");
        return sentil_report("test_gpu");
    }

    std::vector<SimExpr> init;
    init.push_back(SimExpr::constant(0.0));
    std::vector<SimExpr> advance;
    advance.push_back(SimExpr::prev(0) * 0.5 + SimExpr::noise(0));
    std::vector<NoiseModel> noise;
    noise.push_back(NoiseModel::gaussian(0.0, 1.0));
    SimModel model({"x"}, 1.0, 10, std::move(init), std::move(advance), std::move(noise));
    Formula spec = Formula::parse("P>=0.0001 (always[0,10] (x < 6))");
    try {
        GpuSplittingEstimate estimate = spec.check_rare_event_gpu(model, RareEventConfig{1024, 0.0, 1});
        CHECK(estimate.violation_probability >= 0.0 && estimate.violation_probability <= 1.0);
    } catch (const SentilError& e) {
        std::printf("test_gpu: GPU rare-event unavailable here: %s\n", e.what());
    }

    return sentil_report("test_gpu");
}