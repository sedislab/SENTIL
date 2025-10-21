#include <sentil/sentil.hpp>

#include <iostream>

int main() {
    sentil::SystemModel model = sentil::SystemModel::linear({{1.0}}, {{1.0}}, {1.0}, {"x"}, 1.0, 3);
    sentil::Formula spec = sentil::Formula::parse("always (x > 0)");
    sentil::Bounds bounds({-1.0, -1.0, -1.0}, {1.0, 1.0, 1.0});

    sentil::SynthesisResult result = sentil::synthesis::synthesize(model, spec, &bounds);
    std::cout << "input:";
    for (double u : result.input) {
        std::cout << " " << u;
    }
    std::cout << " robustness: " << result.robustness << " holds: " << result.holds << "\n";

    sentil::SafetyFilter shield(sentil::Bounds({-1.0, -1.0, -1.0}, {1.0, 1.0, 1.0}));
    std::cout << "shielded:";
    for (double u : shield.filter({2.0, 0.5, -3.0})) {
        std::cout << " " << u;
    }
    std::cout << "\n";
    return 0;
}