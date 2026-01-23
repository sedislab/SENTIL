#include "sentil/sentil.hpp"

#include "control_app.hpp"
#include "sentil_test.hpp"

int main() {
  const std::vector<std::vector<double>> a = {{1.0, 0.1}, {0.0, 1.0}};
  const std::vector<std::vector<double>> b = {{0.005}, {0.1}};
  sentil_ap::ControlApp app = sentil_ap::ControlApp::synthesize(
      a, b, {5.0, 0.0}, {"pos", "vel"}, 0.1, 5, "always[0, 5] (pos > 1.0 and pos < 9.0)", {-3.0},
      {3.0}, 8000000);

  const std::vector<double> command = app.compute({5.0, 0.0}, {}).command;
  CHECK(command.size() == 1);

  const sentil::SynthesisResult plan = app.plan();
  CHECK(plan.input.size() == 5);
  CHECK(plan.holds);

  const sentil::Witness witness = app.falsify();
  CHECK(witness.input.size() == 5);

  sentil_ap::ControlApp easy = sentil_ap::ControlApp::synthesize(
      a, b, {5.0, 0.0}, {"pos", "vel"}, 0.1, 5, "always[0, 5] (pos > -100.0)", {-3.0}, {3.0},
      8000000);
  const sentil::ChanceReport report = easy.validate_chance(0.9, 0.95, 0.1);
  CHECK(report.holds);
  CHECK(report.estimate > 0.9);

  return sentil_report("synthesis");
}