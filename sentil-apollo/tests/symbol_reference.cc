// Names every SENTIL symbol the components call.
#include <sentil/sentil.hpp>

#include <cstdint>
#include <map>
#include <optional>
#include <string>
#include <vector>

namespace {

void reference_monitor_surface() {
  sentil::MultiMonitor monitor;
  sentil::Formula phi = sentil::Formula::parse("x > 0");
  monitor.add("a", "x > 0");
  monitor.add("b", phi);
  sentil::LiftingRegistry lifting;
  lifting.register_noise("x", sentil::NoiseModel::gaussian(0.0, 1.0),
                         sentil::NoiseInteraction::Additive);
  sentil::SmcConfig smc;
  monitor.add_probabilistic("c", phi, lifting, smc);
  std::map<std::string, double> values;
  std::map<std::string, sentil::Robustness> verdicts = monitor.update(0.0, values);
  std::optional<double> probability = monitor.probability("c");
  sentil::ConfidenceInterval interval = sentil::stats::wilson_interval(1, 2, 0.95);
  (void)verdicts;
  (void)probability;
  (void)interval;
}

void reference_noise_families() {
  (void)sentil::NoiseModel::dirac(0.0);
  (void)sentil::NoiseModel::gaussian(0.0, 1.0);
  (void)sentil::NoiseModel::uniform(0.0, 1.0);
  (void)sentil::NoiseModel::log_normal(0.0, 1.0);
  (void)sentil::NoiseModel::exponential(1.0);
  (void)sentil::NoiseModel::gamma(1.0, 1.0);
  (void)sentil::NoiseModel::beta(1.0, 1.0);
  (void)sentil::NoiseModel::truncated_normal(0.0, 1.0, -1.0, 1.0);
}

void reference_control_surface() {
  std::vector<std::vector<double>> a = {{1.0}};
  std::vector<std::vector<double>> b = {{1.0}};
  std::vector<double> x0 = {0.0};
  std::vector<std::string> variables = {"x"};

  sentil::SystemModel model = sentil::SystemModel::linear(a, b, x0, variables, 0.1, 10);
  sentil::Formula spec = sentil::Formula::parse("always (x > 0)");
  sentil::Bounds bounds({-1.0}, {1.0});
  sentil::Controller controller(std::move(model), std::move(spec), 1, 1000000, &bounds);
  std::vector<double> command = controller.control({0.0});

  sentil::SafetyFilter filter(sentil::Bounds({-1.0}, {1.0}));
  std::vector<double> shielded = filter.filter({2.0});

  sentil::SystemModel synth_model = sentil::SystemModel::linear(a, b, x0, variables, 0.1, 10);
  sentil::Formula synth_spec = sentil::Formula::parse("always (x > 0)");
  sentil::Bounds synth_bounds({-1.0}, {1.0});
  sentil::SynthesisResult result = sentil::synthesis::synthesize(synth_model, synth_spec, &synth_bounds);
  (void)command;
  (void)shielded;
  (void)result.input;
  (void)result.robustness;
  (void)result.holds;
}

void reference_specs() {
  sentil::SpecBuilder builder("any");
  sentil::Formula formula = std::move(builder).with_variant("default").build_formula();
  (void)formula;
}

}  // namespace

int main() {
  void (*referenced[])() = {reference_monitor_surface, reference_noise_families,
                            reference_control_surface, reference_specs};
  (void)referenced;
  return 0;
}