#include "modules/sentil/monitor/sentil_monitor_component.h"

namespace apollo {
namespace sentil {

bool SentilMonitorComponent::Init() {
  if (!GetProtoConfig(&config_)) {
    AERROR << "sentil monitor: could not load its configuration";
    return false;
  }
  try {
    engine_.Build(config_);
  } catch (const std::exception& error) {
    AERROR << "sentil monitor: configuration failed: " << error.what();
    return false;
  }
  status_writer_ = node_->CreateWriter<SentilStatus>(config_.output_channel());
  AINFO << "sentil monitor: watching " << config_.formulas_size() << " formula(s)";
  return true;
}

bool SentilMonitorComponent::Proc(
    const std::shared_ptr<apollo::perception::PerceptionObstacles>& perception,
    const std::shared_ptr<apollo::localization::LocalizationEstimate>& localization,
    const std::shared_ptr<apollo::canbus::Chassis>& chassis) {
  if (perception == nullptr || localization == nullptr || chassis == nullptr) {
    return true;
  }
  SentilStatus status;
  try {
    if (engine_.Evaluate(*perception, *localization, *chassis, &status)) {
      status_writer_->Write(status);
    }
  } catch (const std::exception& error) {
    AWARN << "sentil monitor: evaluation failed: " << error.what();
  }
  return true;
}

}  // namespace sentil
}  // namespace apollo