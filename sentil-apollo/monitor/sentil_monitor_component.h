#pragma once

#include <memory>

#include "cyber/cyber.h"
#include "cyber/component/component.h"

#include "modules/common_msgs/chassis_msgs/chassis.pb.h"
#include "modules/common_msgs/localization_msgs/localization.pb.h"
#include "modules/common_msgs/perception_msgs/perception_obstacle.pb.h"

#include "modules/sentil/monitor/monitor_engine.h"
#include "modules/sentil/proto/sentil_config.pb.h"
#include "modules/sentil/proto/sentil_status.pb.h"

namespace apollo {
namespace sentil {

class SentilMonitorComponent
    : public apollo::cyber::Component<apollo::perception::PerceptionObstacles,
                                      apollo::localization::LocalizationEstimate,
                                      apollo::canbus::Chassis> {
 public:
  bool Init() override;
  bool Proc(const std::shared_ptr<apollo::perception::PerceptionObstacles>& perception,
            const std::shared_ptr<apollo::localization::LocalizationEstimate>& localization,
            const std::shared_ptr<apollo::canbus::Chassis>& chassis) override;

 private:
  SentilConfig config_;
  MonitorEngine engine_;
  std::shared_ptr<apollo::cyber::Writer<SentilStatus>> status_writer_;
};

CYBER_REGISTER_COMPONENT(SentilMonitorComponent)

}  // namespace sentil
}  // namespace apollo