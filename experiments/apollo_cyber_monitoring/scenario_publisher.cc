#include <chrono>
#include <cmath>
#include <memory>
#include <thread>

#include "cyber/cyber.h"

#include "modules/common_msgs/chassis_msgs/chassis.pb.h"
#include "modules/common_msgs/localization_msgs/localization.pb.h"
#include "modules/common_msgs/perception_msgs/perception_obstacle.pb.h"

using apollo::canbus::Chassis;
using apollo::localization::LocalizationEstimate;
using apollo::perception::PerceptionObstacles;

int main(int argc, char** argv) {
  apollo::cyber::Init(argv[0]);
  auto node = apollo::cyber::CreateNode("sentil_scenario_publisher");
  auto chassis_w = node->CreateWriter<Chassis>("/apollo/canbus/chassis");
  auto loc_w = node->CreateWriter<LocalizationEstimate>("/apollo/localization/pose");
  auto perc_w = node->CreateWriter<PerceptionObstacles>("/apollo/perception/obstacles");

  auto chassis0 = std::make_shared<Chassis>();
  chassis0->mutable_header()->set_timestamp_sec(0.0);
  chassis0->set_speed_mps(15.0F);
  chassis_w->Write(chassis0);
  auto loc0 = std::make_shared<LocalizationEstimate>();
  loc0->mutable_header()->set_timestamp_sec(0.0);
  loc0->mutable_pose()->mutable_position()->set_x(0.0);
  loc_w->Write(loc0);
  std::this_thread::sleep_for(std::chrono::milliseconds(400));

  for (int k = 0; k <= 70 && apollo::cyber::OK(); ++k) {
    const double t = k * 0.1;
    const double gap = std::max(3.0, 12.0 - 1.5 * t);

    auto chassis = std::make_shared<Chassis>();
    chassis->mutable_header()->set_timestamp_sec(t);
    chassis->set_speed_mps(15.0F);
    chassis_w->Write(chassis);

    auto loc = std::make_shared<LocalizationEstimate>();
    loc->mutable_header()->set_timestamp_sec(t);
    loc->mutable_pose()->mutable_position()->set_x(0.0);
    loc_w->Write(loc);

    auto perc = std::make_shared<PerceptionObstacles>();
    perc->mutable_header()->set_timestamp_sec(t);
    auto* ob = perc->add_perception_obstacle();
    ob->mutable_position()->set_x(gap);  // ego-relative
    ob->mutable_position()->set_y(0.0);
    ob->mutable_velocity()->set_x(-1.5);
    perc_w->Write(perc);

    std::this_thread::sleep_for(std::chrono::milliseconds(100));
  }
  return 0;
}