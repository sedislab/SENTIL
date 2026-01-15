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

  sentil::Formula spec = sentil::Formula::parse("always[0, 20] (pos > 1.0 and pos < 9.0)");
  sentil::Bounds bounds({-3.0}, {3.0});
  sentil::SynthesisResult plan = sentil::synthesis::synthesize(model, spec, &bounds);

  std::cout << "synthesized " << plan.input.size() << " inputs, robustness " << plan.robustness
            << ", holds " << (plan.holds ? "true" : "false") << "\n";

  // The monitor would now watch the deployed controller against the same spec. Here we run
  // the plan back through the model and report the robustness the online monitor would see.
  return plan.holds ? 0 : 1;
}