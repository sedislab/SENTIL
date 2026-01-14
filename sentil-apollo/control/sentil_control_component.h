#pragma once

#include <memory>
#include <string>
#include <vector>

#include "cyber/cyber.h"
#include "cyber/component/timer_component.h"

#include "modules/common_msgs/chassis_msgs/chassis.pb.h"
#include "modules/common_msgs/control_msgs/control_cmd.pb.h"
#include "modules/common_msgs/localization_msgs/localization.pb.h"

#include "sentil/sentil.hpp"

#include "modules/sentil/common/field_extractor.h"
#include "modules/sentil/proto/sentil_control_config.pb.h"

namespace apollo {
namespace sentil {

class SentilControlComponent : public apollo::cyber::TimerComponent {
 public:
  bool Init() override;
  bool Proc() override;

 private:
  void ResolveOutputs();

  bool BuildState(std::vector<double>* state);

  void ApplyInputs(const std::vector<double>& input, apollo::control::ControlCommand* command);
  void Emit(const std::vector<double>& input);

  SentilControlConfig config_;
  FieldExtractor extractor_;
  std::vector<std::string> variable_order_;
  std::vector<std::pair<int, const google::protobuf::FieldDescriptor*>> outputs_;

  std::unique_ptr<::sentil::Controller> controller_;
  std::unique_ptr<::sentil::SafetyFilter> shield_;

  std::shared_ptr<apollo::cyber::Reader<apollo::localization::LocalizationEstimate>> localization_reader_;
  std::shared_ptr<apollo::cyber::Reader<apollo::canbus::Chassis>> chassis_reader_;
  std::shared_ptr<apollo::cyber::Reader<apollo::control::ControlCommand>> nominal_reader_;
  std::shared_ptr<apollo::cyber::Writer<apollo::control::ControlCommand>> control_writer_;
};

CYBER_REGISTER_COMPONENT(SentilControlComponent)

}  // namespace sentil
}  // namespace apollo