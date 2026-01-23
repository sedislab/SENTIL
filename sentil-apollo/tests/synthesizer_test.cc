#include "sentil/sentil.hpp"

#include "modules/sentil/common/engine_config.h"
#include "modules/sentil/proto/sentil_control_config.pb.h"
#include "sentil_test.hpp"

using apollo::sentil::SentilControlConfig;

int main() {
  SentilControlConfig config;
  auto* model_proto = config.mutable_model();
  for (double v : {1.0, 0.1, 0.0, 1.0}) {
    model_proto->add_a(v);
  }
  for (double v : {0.005, 0.1}) {
    model_proto->add_b(v);
  }
  model_proto->add_x0(5.0);
  model_proto->add_x0(0.0);
  model_proto->add_variables("pos");
  model_proto->add_variables("vel");
  model_proto->set_dt(0.1);
  model_proto->set_horizon(5);
  config.mutable_spec()->set_expression("always[0, 5] (pos > 1.0 and pos < 9.0)");
  config.set_input_width(1);
  config.mutable_bounds()->add_lower(-3.0);
  config.mutable_bounds()->add_upper(3.0);

  const std::size_t input_width = config.input_width();
  const std::size_t horizon = config.model().horizon();
  sentil::SystemModel model = apollo::sentil::model_from_proto(config.model(), input_width);
  sentil::Formula spec = apollo::sentil::formula_from_spec(config.spec());
  sentil::Bounds bounds = apollo::sentil::tile_bounds(config.bounds(), input_width, horizon);

  sentil::SynthesisResult plan = sentil::synthesis::synthesize(model, spec, &bounds);
  CHECK(plan.input.size() == 5);
  CHECK(plan.holds);

  sentil::Witness witness = spec.falsify(model, bounds);
  CHECK(witness.input.size() == 5);

  sentil::StochasticSystem system = apollo::sentil::chance_system_from_model(config.model(), 0.1);
  sentil::ChanceConstraint constraint(
      sentil::Formula::parse(config.spec().expression()), 0.5, 0.95);
  sentil::ChanceReport report = constraint.validate(system);
  CHECK(report.estimate >= 0.0);
  CHECK(report.estimate <= 1.0);
  CHECK(report.samples > 0);

  return sentil_report("synthesizer");
}