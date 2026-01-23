#include "sentil/sentil.hpp"

#include "monitor_app.hpp"
#include "sentil_test.hpp"

namespace {

double probability_at(double reading) {
  sentil_ap::MonitorApp app;
  app.add_probabilistic("p", "P>=0.5 (x > 0.0)", "x", sentil::NoiseModel::uniform(-0.5, 0.5),
                        sentil::NoiseInteraction::Additive, 0.95, 2000);
  sentil_ap::SignalFrame frame;
  frame.t = 0.0;
  frame.names = {"x"};
  frame.values = {reading};
  return app.on_frame(frame).at("p").probability;
}

}  // namespace

int main() {
  CHECK(probability_at(2.0) > 0.9);

  CHECK(probability_at(-2.0) < 0.1);

  return sentil_report("probabilistic");
}