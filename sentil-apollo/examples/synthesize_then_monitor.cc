#include <iostream>
#include <string>
#include <vector>

#include "sentil/sentil.hpp"

int main() {
  // A double integrator.
  const std::vector<std::vector<double>> a = {{1.0, 0.1}, {0.0, 1.0}};
  const std::vector<std::vector<double>> b = {{0.005}, {0.1}};
  const std::vector<double> x0 = {0.0, 0.0};
  const std::vector<std::string> variables = {"pos", "vel"};
  sentil::SystemModel model = sentil::SystemModel::linear(a, b, x0, variables, 0.1, 20);

  sentil::Formula spec = sentil::Formula::parse("always[1, 2] (pos > 1.0 and pos < 9.0)");
  sentil::Bounds bounds(std::vector<double>(20, -3.0), std::vector<double>(20, 3.0));
  try {
    sentil::SynthesisResult plan = sentil::synthesis::synthesize(model, spec, &bounds);
    std::cout << "synthesized " << plan.input.size() << " inputs, robustness " << plan.robustness
              << ", holds " << (plan.holds ? "true" : "false") << "\n";
    return plan.holds ? 0 : 1;
  } catch (const std::exception& error) {
    std::cerr << "synthesis failed: " << error.what() << "\n";
    return 1;
  }
}