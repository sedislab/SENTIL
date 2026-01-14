#include "modules/sentil/monitor/sentil_timed_monitor_component.h"

namespace apollo {
namespace sentil {
namespace {

constexpr char kPerceptionChannel[] = "/apollo/perception/obstacles";
constexpr char kLocalizationChannel[] = "/apollo/localization/pose";
constexpr char kChassisChannel[] = "/apollo/canbus/chassis";

}  // namespace

bool SentilTimedMonitorComponent::Init() {
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
  perception_reader_ =
      node_->CreateReader<apollo::perception::PerceptionObstacles>(kPerceptionChannel);
  localization_reader_ =
      node_->CreateReader<apollo::localization::LocalizationEstimate>(kLocalizationChannel);
  chassis_reader_ = node_->CreateReader<apollo::canbus::Chassis>(kChassisChannel);
  status_writer_ = node_->CreateWriter<SentilStatus>(config_.output_channel());
  AINFO << "sentil monitor: watching " << config_.formulas_size() << " formula(s) on a timer";
  return true;
}

bool SentilTimedMonitorComponent::Proc() {
  perception_reader_->Observe();
  localization_reader_->Observe();
  chassis_reader_->Observe();
  const auto perception = perception_reader_->GetLatestObserved();
  const auto localization = localization_reader_->GetLatestObserved();
  const auto chassis = chassis_reader_->GetLatestObserved();
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