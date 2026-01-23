#include <fstream>
#include <iostream>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

#include "gflags/gflags.h"
#include "google/protobuf/text_format.h"

#include "sentil/sentil.hpp"

#include "modules/sentil/common/engine_config.h"
#include "modules/sentil/proto/sentil_control_config.pb.h"

DEFINE_string(config, "", "the SentilControlConfig text proto file");
DEFINE_string(op, "plan", "the operation: plan, witness, or chance");
DEFINE_double(probability, 0.95, "chance: the target satisfaction probability");
DEFINE_double(confidence, 0.95, "chance: the confidence level of the bound");
DEFINE_double(process_std, 0.0, "chance: the Gaussian process-noise standard deviation");

namespace apollo {
namespace sentil {
namespace {

SentilControlConfig load_config(const std::string& path) {
  std::ifstream in(path);
  if (!in) {
    throw std::runtime_error("could not open config '" + path + "'");
  }
  std::stringstream buffer;
  buffer << in.rdbuf();
  SentilControlConfig config;
  if (!google::protobuf::TextFormat::ParseFromString(buffer.str(), &config)) {
    throw std::runtime_error("could not parse config '" + path + "' as a SentilControlConfig");
  }
  return config;
}

void print_sequence(const std::vector<double>& input) {
  for (std::size_t i = 0; i < input.size(); ++i) {
    std::cout << (i == 0 ? "" : " ") << input[i];
  }
  std::cout << "\n";
}

int run(const SentilControlConfig& config) {
  const std::size_t input_width = config.input_width();
  const std::size_t horizon = config.model().horizon();
  ::sentil::SystemModel model = model_from_proto(config.model(), input_width);
  ::sentil::Formula spec = formula_from_spec(config.spec());
  ::sentil::Bounds bounds = config.has_bounds()
                                ? tile_bounds(config.bounds(), input_width, horizon)
                                : ::sentil::Bounds::unbounded(horizon * input_width);
  if (FLAGS_op == "plan") {
    ::sentil::SynthesisResult result = ::sentil::synthesis::synthesize(model, spec, &bounds);
    std::cout << "robustness " << result.robustness << " holds "
              << (result.holds ? "true" : "false") << "\ninput ";
    print_sequence(result.input);
  } else if (FLAGS_op == "witness") {
    if (!config.has_bounds()) {
      std::cerr << "witness needs bounds to search over\n";
      return 2;
    }
    ::sentil::Witness witness = spec.falsify(model, bounds);
    std::cout << "counterexample robustness " << witness.robustness << "\ninput ";
    print_sequence(witness.input);
  } else if (FLAGS_op == "chance") {
    ::sentil::StochasticSystem system = chance_system_from_model(config.model(), FLAGS_process_std);
    ::sentil::ChanceConstraint constraint(std::move(spec), FLAGS_probability, FLAGS_confidence);
    ::sentil::ChanceReport report = constraint.validate(system);
    std::cout << "estimate " << report.estimate << " lower_bound " << report.lower_bound << " holds "
              << (report.holds ? "true" : "false") << "\n";
  } else {
    std::cerr << "op must be plan, witness, or chance\n";
    return 2;
  }
  return 0;
}

}  // namespace
}  // namespace sentil
}  // namespace apollo

int main(int argc, char** argv) {
  gflags::ParseCommandLineFlags(&argc, &argv, true);
  if (FLAGS_config.empty()) {
    std::cerr << "usage: sentil_synthesizer --config=<file> --op=plan|witness|chance\n";
    return 2;
  }
  try {
    return apollo::sentil::run(apollo::sentil::load_config(FLAGS_config));
  } catch (const std::exception& e) {
    std::cerr << "sentil synthesizer: " << e.what() << "\n";
    return 1;
  }
}