#pragma once

#include <memory>

#include "cyber/cyber.h"
#include "cyber/component/timer_component.h"

#include "modules/common_msgs/chassis_msgs/chassis.pb.h"
#include "modules/common_msgs/localization_msgs/localization.pb.h"
#include "modules/common_msgs/perception_msgs/perception_obstacle.pb.h"

#include "modules/sentil/monitor/monitor_engine.h"
#include "modules/sentil/proto/sentil_config.pb.h"
#include "modules/sentil/proto/sentil_status.pb.h"

namespace apollo {
namespace sentil {

class SentilTimedMonitorComponent : public apollo::cyber::TimerComponent {
 public:
  bool Init() override;
  bool Proc() override;

 private:
  SentilConfig config_;
  MonitorEngine engine_;
  std::shared_ptr<apollo::cyber::Reader<apollo::perception::PerceptionObstacles>> perception_reader_;
  std::shared_ptr<apollo::cyber::Reader<apollo::localization::LocalizationEstimate>> localization_reader_;
  std::shared_ptr<apollo::cyber::Reader<apollo::canbus::Chassis>> chassis_reader_;
  std::shared_ptr<apollo::cyber::Writer<SentilStatus>> status_writer_;
};

CYBER_REGISTER_COMPONENT(SentilTimedMonitorComponent)

}  // namespace sentil
}  // namespace apollo