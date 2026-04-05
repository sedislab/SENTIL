#include "sentil/sentil.hpp"

#include <cstdio>
#include <exception>
#include <fstream>
#include <sstream>
#include <string>

#include "google/protobuf/text_format.h"

#include "modules/sentil/common/engine_config.h"
#include "modules/sentil/proto/sentil_control_config.pb.h"
#include "sentil_test.hpp"

#ifndef SENTIL_EXAMPLE_CONFIG
#define SENTIL_EXAMPLE_CONFIG "../examples/synthesize_control.pb.txt"
#endif

using apollo::sentil::SentilControlConfig;

int main() {
  std::ifstream in(SENTIL_EXAMPLE_CONFIG);
  if (!in.good()) {
    std::fprintf(stderr, "cannot open config at %s\n", SENTIL_EXAMPLE_CONFIG);
    return 1;
  }
  std::stringstream buffer;
  buffer << in.rdbuf();
  SentilControlConfig config;
  if (!google::protobuf::TextFormat::ParseFromString(buffer.str(), &config)) {
    std::fprintf(stderr, "cannot parse config at %s\n", SENTIL_EXAMPLE_CONFIG);
    return 1;
  }
  CHECK(config.mode() == apollo::sentil::SYNTHESIZE);

  const std::size_t input_width = config.input_width();
  const std::size_t horizon = config.model().horizon();
  try {
    sentil::SystemModel model = apollo::sentil::model_from_proto(config.model(), input_width);
    sentil::Formula spec = apollo::sentil::formula_from_spec(config.spec());
    sentil::Bounds bounds = apollo::sentil::tile_bounds(config.bounds(), input_width, horizon);

    sentil::SynthesisResult plan = sentil::synthesis::synthesize(model, spec, &bounds);
    CHECK(plan.input.size() == horizon * input_width);
    CHECK(plan.holds);
  } catch (const std::exception& error) {
    std::fprintf(stderr, "planning the example config failed: %s\n", error.what());
    return 1;
  }

  return sentil_report("example_synthesis");
}