#pragma once

#include <map>
#include <memory>
#include <string>

#include "modules/common_msgs/chassis_msgs/chassis.pb.h"
#include "modules/common_msgs/localization_msgs/localization.pb.h"
#include "modules/common_msgs/perception_msgs/perception_obstacle.pb.h"

#include "sentil/sentil.hpp"

#include "modules/sentil/common/field_extractor.h"
#include "modules/sentil/proto/sentil_config.pb.h"
#include "modules/sentil/proto/sentil_status.pb.h"

namespace apollo {
namespace sentil {

class MonitorEngine {
 public:
  void Build(const SentilConfig& config);

  bool Evaluate(const apollo::perception::PerceptionObstacles& perception,
                const apollo::localization::LocalizationEstimate& localization,
                const apollo::canbus::Chassis& chassis, SentilStatus* status);

 private:
  void FillResult(const Formula& formula, const ::sentil::Robustness& robustness,
                  FormulaResult* result);

  SentilConfig config_;
  FieldExtractor extractor_;
  std::unique_ptr<::sentil::MultiMonitor> monitor_;
  std::map<std::string, double> confidence_;
  double last_stamp_ = -1.0;
};

}  // namespace sentil
}  // namespace apollo