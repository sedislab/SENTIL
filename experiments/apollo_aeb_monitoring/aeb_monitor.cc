#include <cmath>
#include <cstdio>
#include <map>
#include <string>
#include <vector>

#include "sentil/sentil.hpp"

#include "modules/sentil/common/field_extractor.h"
#include "modules/sentil/proto/sentil_config.pb.h"
#include "perception_obstacle.pb.h"

using apollo::perception::PerceptionObstacle;
using apollo::perception::PerceptionObstacles;
using apollo::sentil::FieldExtractor;
using apollo::sentil::FieldMapping;

namespace {

void put_obstacle(PerceptionObstacles* msg, double x, double y, double vx, double vy) {
  PerceptionObstacle* obstacle = msg->add_perception_obstacle();
  obstacle->mutable_position()->set_x(x);
  obstacle->mutable_position()->set_y(y);
  obstacle->mutable_velocity()->set_x(vx);
  obstacle->mutable_velocity()->set_y(vy);
}

}  // namespace

int main() {
  FieldExtractor extractor;
  FieldMapping mapping;
  mapping.set_variable("nearest");
  mapping.set_builtin("NEAREST_OBSTACLE_DISTANCE");
  extractor.add_channel("apollo.perception.PerceptionObstacles", {mapping});

  sentil::Monitor monitor("historically (nearest > 5.0)");

  std::printf("t,nearest,robustness,satisfied\n");
  for (int k = 0; k <= 70; ++k) {
    const double t = k * 0.1;
    const double lead = std::max(2.0, 60.0 - 10.0 * t);
    PerceptionObstacles msg;
    put_obstacle(&msg, lead, 0.0, -10.0, 0.0);
    put_obstacle(&msg, 3.0, 15.0, 0.0, 0.0);

    std::vector<std::string> names;
    std::vector<double> values;
    extractor.extract_into("apollo.perception.PerceptionObstacles", msg, &names, &values);
    std::map<std::string, double> sample;
    for (std::size_t i = 0; i < names.size(); ++i) {
      sample[names[i]] = values[i];
    }
    const sentil::Robustness r = monitor.update(t, sample);
    std::printf("%.1f,%.3f,%.3f,%d\n", t, sample["nearest"], r.value, r.satisfied ? 1 : 0);
  }
  return 0;
}